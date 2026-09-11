//! Parallel fan-out, budgets, cache, pagination, and merge.

use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::future::join_all;
use tracing::{info, warn};

use crate::cache::SearchCache;
use crate::config::Config;
use crate::error::Error;
use crate::ground::ground_one;
use crate::health::HealthBoard;
use crate::http::HttpClient;
use crate::intent::{select_providers, sort_ladder};
use crate::merge::{apply_freshness, rrf_merge};
use crate::providers::{Provider, Registry, page_size_for};
use crate::ssrf::assert_public_http_url;
use crate::types::{
    ExtractRequest, ExtractResponse, ExtractedDoc, ProviderFailure, ProviderHealth, ProviderId,
    ProviderSearchRequest, ProviderSkip, ResearchResponse, RunMeta, SearchHit, SearchMode,
    SearchRequest, SearchResponse,
};

pub struct AppState {
    pub config: Config,
    pub http: HttpClient,
    pub registry: Registry,
    pub cache: SearchCache,
    pub health: HealthBoard,
}

impl AppState {
    /// Build state from an explicit config.
    pub fn new(config: Config) -> crate::error::Result<Arc<Self>> {
        config.validate_accounts()?;
        config.validate_gateways()?;
        let http = HttpClient::new(&config)?;
        let registry = Registry::new(&config, http.clone());
        Ok(Arc::new(Self {
            config,
            http,
            registry,
            cache: SearchCache::new(),
            health: HealthBoard::new(),
        }))
    }

    /// Build state from environment.
    pub fn from_env() -> crate::error::Result<Arc<Self>> {
        Self::new(Config::from_env())
    }

    /// Non-secret live health for every registered provider.
    pub fn search_health(&self) -> Vec<ProviderHealth> {
        self.registry
            .infos()
            .into_iter()
            .map(|info| {
                let id = ProviderId::parse(&info.id).ok();
                let snap = id.map(|id| self.health.snapshot(id)).unwrap_or_default();
                snap.into_health(
                    &info.id,
                    info.configured,
                    info.requires_key,
                    info.estimated_search_usd,
                    info.notes,
                )
            })
            .collect()
    }
}

struct Fanout {
    lists: Vec<Vec<SearchHit>>,
    successful: Vec<String>,
    failed: Vec<ProviderFailure>,
    timed_out: Vec<String>,
    answers: Vec<String>,
}

impl Fanout {
    fn empty() -> Self {
        Self {
            lists: Vec::new(),
            successful: Vec::new(),
            failed: Vec::new(),
            timed_out: Vec::new(),
            answers: Vec::new(),
        }
    }

    fn absorb(&mut self, other: Self) {
        self.lists.extend(other.lists);
        self.successful.extend(other.successful);
        self.failed.extend(other.failed);
        self.timed_out.extend(other.timed_out);
        self.answers.extend(other.answers);
    }
}

pub async fn search(state: &AppState, request: SearchRequest) -> SearchResponse {
    let started = Instant::now();
    let cache_key = SearchCache::key(&[
        &request.query,
        &format!("{:?}", request.providers),
        &format!("{:?}", request.mode),
        &format!("{:?}", request.search_type),
        &format!("{:?}", request.freshness),
        &format!("{:?}", request.limit),
        &request.unlimited.to_string(),
        &format!("{:?}", request.ground_top),
        &format!("{:?}", request.account),
        &format!("{:?}", request.depth),
    ]);
    if !request.no_cache
        && let Some(mut cached) = state
            .cache
            .get(&cache_key, Duration::from_secs(state.config.cache_ttl_secs))
    {
        cached.meta.cache_hit = true;
        return cached;
    }

    let configured: Vec<ProviderId> = state
        .registry
        .configured_search()
        .iter()
        .map(|p| p.id())
        .collect();

    let mut skipped = Vec::new();
    for info in state.registry.infos() {
        if !info.configured {
            skipped.push(ProviderSkip {
                provider: info.id,
                reason: "not configured".into(),
            });
        }
    }

    let mut selected = select_providers(
        &request,
        &configured,
        &state.health,
        request.providers.is_none(),
    );

    if request.mode == SearchMode::Ladder {
        sort_ladder(&mut selected, |id| provider_cost(state, id));
    }

    if request.providers.is_none() {
        selected.retain(|id| {
            let Some(p) = state.registry.get(*id) else {
                return false;
            };
            if p.estimated_search_usd() > state.config.auto_allow_usd {
                skipped.push(ProviderSkip {
                    provider: id.as_str().into(),
                    reason: format!(
                        "auto_allow cost gate ${:.4} > ${:.4}",
                        p.estimated_search_usd(),
                        state.config.auto_allow_usd
                    ),
                });
                return false;
            }
            true
        });
    }

    let mut budget_stopped = false;
    let mut max_providers_stopped = false;
    if let Some(max) = request.max_providers
        && selected.len() > max
    {
        for id in selected.iter().skip(max) {
            skipped.push(ProviderSkip {
                provider: id.as_str().into(),
                reason: format!("max_providers {max}"),
            });
        }
        selected.truncate(max);
        max_providers_stopped = true;
    }

    let mut estimated_cost = 0.0;
    if let Some(budget) = request.budget_usd {
        let mut kept = Vec::new();
        for id in selected {
            let cost = provider_cost(state, id);
            if estimated_cost + cost > budget && !kept.is_empty() {
                skipped.push(ProviderSkip {
                    provider: id.as_str().into(),
                    reason: format!("budget_usd {budget} exhausted"),
                });
                budget_stopped = true;
                continue;
            }
            estimated_cost += cost;
            kept.push(id);
        }
        selected = kept;
    } else {
        estimated_cost = selected.iter().map(|id| provider_cost(state, *id)).sum();
    }

    let timeout = Duration::from_secs(
        request
            .timeout_seconds
            .unwrap_or(state.config.default_timeout_secs),
    );
    let evidence_min = request.evidence_min.unwrap_or(state.config.evidence_min) as usize;

    let (mut fanout, evidence_stopped) = if request.mode == SearchMode::Ladder
        && request.providers.is_none()
        && selected.iter().any(|id| provider_cost(state, *id) == 0.0)
        && selected.iter().any(|id| provider_cost(state, *id) > 0.0)
    {
        let free: Vec<_> = selected
            .iter()
            .copied()
            .filter(|id| provider_cost(state, *id) == 0.0)
            .collect();
        let paid: Vec<_> = selected
            .iter()
            .copied()
            .filter(|id| provider_cost(state, *id) > 0.0)
            .collect();
        let mut acc = run_fanout(state, &free, &request, timeout).await;
        let preview = rrf_merge(
            acc.lists.clone(),
            state.config.rrf_k,
            state.config.max_per_domain,
        );
        if preview.hits.len() >= evidence_min {
            for id in &paid {
                skipped.push(ProviderSkip {
                    provider: id.as_str().into(),
                    reason: format!("evidence_min {evidence_min} met by free providers"),
                });
            }
            (acc, true)
        } else {
            acc.absorb(run_fanout(state, &paid, &request, timeout).await);
            (acc, false)
        }
    } else {
        (run_fanout(state, &selected, &request, timeout).await, false)
    };

    let merged = rrf_merge(
        fanout.lists,
        state.config.rrf_k,
        state.config.max_per_domain,
    );
    let mut hits = merged.hits;
    if let Some(fresh) = request.freshness {
        hits = apply_freshness(hits, &fresh.since_rfc3339());
    }

    let ground_n = request.ground_top.unwrap_or(state.config.ground_top as u32);
    if ground_n > 0 {
        ground_hits(state, &request.query, &mut hits, ground_n as usize, timeout).await;
    }

    let mut truncated = false;
    if hits.len() > state.config.safety_bound {
        hits.truncate(state.config.safety_bound);
        truncated = true;
    }

    let cost_usd = fanout
        .successful
        .iter()
        .filter_map(|name| ProviderId::parse(name).ok())
        .map(|id| provider_cost(state, id))
        .sum();

    let complete = fanout.failed.is_empty() && fanout.timed_out.is_empty() && !selected.is_empty();
    let stop_reason = if truncated {
        "safety_bound"
    } else if evidence_stopped {
        "evidence"
    } else if budget_stopped {
        "budget_usd"
    } else if max_providers_stopped {
        "max_providers"
    } else if !fanout.timed_out.is_empty() {
        "timeout"
    } else if !fanout.failed.is_empty() {
        "partial_provider_failure"
    } else {
        "complete"
    };

    let unique_count = hits.len() as u32;
    let provider_used = fanout.successful.clone();
    let mut response = SearchResponse {
        query: request.query.clone(),
        unique_count,
        answer: fanout.answers.into_iter().next(),
        results: hits,
        quality_report: request.include_quality_report.then_some(merged.quality),
        delivery: None,
        meta: RunMeta {
            selected: selected.iter().map(|id| id.as_str().to_string()).collect(),
            successful: std::mem::take(&mut fanout.successful),
            failed: fanout.failed,
            timed_out: fanout.timed_out,
            skipped,
            cache_hit: false,
            truncated,
            safety_bound: state.config.safety_bound as u32,
            rrf_k: state.config.rrf_k,
            estimated_cost_usd: estimated_cost,
            cost_usd,
            provider_used,
            stop_reason: Some(stop_reason.into()),
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
    };
    maybe_spill_to_file(&state.config, &mut response);

    let allow_partial = request.cache_partial || state.config.cache_partial;
    if !request.no_cache && (complete || allow_partial) {
        state.cache.put(cache_key, response.clone());
    }
    info!(
        query = %request.query,
        providers = response.meta.successful.len(),
        results = response.unique_count,
        stop = stop_reason,
        "search complete"
    );
    response
}

fn provider_cost(state: &AppState, id: ProviderId) -> f64 {
    state
        .registry
        .get(id)
        .map(|p| p.estimated_search_usd())
        .unwrap_or(0.0)
}

async fn run_fanout(
    state: &AppState,
    selected: &[ProviderId],
    request: &SearchRequest,
    timeout: Duration,
) -> Fanout {
    if selected.is_empty() {
        return Fanout::empty();
    }
    let futs = selected.iter().copied().map(|id| {
        let provider = state.registry.get(id);
        let req = request.clone();
        let country = request
            .country
            .clone()
            .unwrap_or_else(|| state.config.country.clone());
        let language = request
            .language
            .clone()
            .unwrap_or_else(|| state.config.language.clone());
        let max_pages = state.config.max_pages_per_provider;
        let safety = state.config.safety_bound;
        async move {
            let started = Instant::now();
            let Some(provider) = provider else {
                return (
                    id,
                    Err(Error::provider(id.as_str(), "missing provider")),
                    started.elapsed(),
                );
            };
            let run = collect_provider(provider, &req, &country, &language, max_pages, safety);
            let outcome = match tokio::time::timeout(timeout, run).await {
                Ok(result) => result,
                Err(_) => Err(Error::Timeout {
                    provider: id.as_str().into(),
                    seconds: timeout.as_secs(),
                }),
            };
            (id, outcome, started.elapsed())
        }
    });

    let outcomes = join_all(futs).await;
    let mut out = Fanout::empty();
    for (id, outcome, latency) in outcomes {
        match outcome {
            Ok((hits, answer)) => {
                state.health.mark_success(id, latency);
                if let Some(answer) = answer {
                    out.answers.push(answer);
                }
                out.successful.push(id.as_str().to_string());
                out.lists.push(hits);
            }
            Err(err) => {
                state.health.mark_failure(
                    id,
                    &err,
                    Duration::from_secs(state.config.cooldown_secs),
                    latency,
                );
                if matches!(err, Error::Timeout { .. }) {
                    out.timed_out.push(id.as_str().to_string());
                } else {
                    out.failed.push(ProviderFailure {
                        provider: id.as_str().into(),
                        error: err.to_string(),
                    });
                }
            }
        }
    }
    out
}

async fn collect_provider(
    provider: Arc<dyn Provider>,
    request: &SearchRequest,
    country: &str,
    language: &str,
    max_pages: usize,
    safety: usize,
) -> crate::error::Result<(Vec<SearchHit>, Option<String>)> {
    if let Some(name) = request.account.as_deref()
        && !provider.known_account(name)
    {
        return Err(Error::Invalid(format!(
            "unknown account '{name}' for provider {}",
            provider.id()
        )));
    }
    let mut hits = Vec::new();
    let mut cursor: Option<String> = None;
    let mut answer = None;
    let page_size = page_size_for(provider.max_page_size(), request);
    for _ in 0..max_pages {
        if hits.len() >= safety {
            break;
        }
        if !request.wants_unlimited()
            && let Some(limit) = request.limit
            && hits.len() >= limit as usize
        {
            break;
        }
        let page_req = ProviderSearchRequest {
            query: &request.query,
            cursor: cursor.as_deref(),
            page_size,
            search_type: request.search_type,
            freshness: request.freshness,
            country,
            language,
            account: request.account.as_deref(),
            depth: request.depth.as_deref(),
        };
        let page = provider.search(&page_req).await?;
        if answer.is_none() {
            answer = page.answer;
        }
        if page.hits.is_empty() {
            break;
        }
        hits.extend(page.hits);
        match page.next_cursor {
            Some(next) if Some(&next) != cursor.as_ref() => cursor = Some(next),
            _ => break,
        }
    }
    Ok((hits, answer))
}

async fn ground_hits(
    state: &AppState,
    query: &str,
    hits: &mut [SearchHit],
    top_n: usize,
    timeout: Duration,
) {
    let n = top_n.min(hits.len());
    let futs = hits.iter().take(n).map(|hit| {
        let http = state.http.clone();
        let query = query.to_string();
        let hit = hit.clone();
        async move {
            match tokio::time::timeout(timeout, ground_one(&http, &query, &hit)).await {
                Ok(Some(snippet)) => Some(snippet),
                _ => None,
            }
        }
    });
    let framed = join_all(futs).await;
    for (hit, snippet) in hits.iter_mut().zip(framed) {
        if let Some(snippet) = snippet {
            hit.snippet = snippet;
            hit.snippet_grounded = true;
        }
    }
}

pub async fn extract(state: &AppState, request: ExtractRequest) -> ExtractResponse {
    let started = Instant::now();
    let mut meta = RunMeta {
        safety_bound: state.config.safety_bound as u32,
        rrf_k: state.config.rrf_k,
        ..RunMeta::default()
    };
    let mut urls = Vec::new();
    for raw in &request.urls {
        match assert_public_http_url(raw) {
            Ok(url) => urls.push(url.to_string()),
            Err(err) => meta.failed.push(ProviderFailure {
                provider: "ssrf".into(),
                error: err.to_string(),
            }),
        }
    }

    let providers: Vec<Arc<dyn Provider>> = if let Some(filter) = &request.providers {
        filter
            .iter()
            .filter_map(|id| state.registry.get(*id))
            .collect()
    } else {
        state.registry.configured_extract()
    };
    meta.selected = providers
        .iter()
        .map(|p| p.id().as_str().to_string())
        .collect();

    let timeout = Duration::from_secs(
        request
            .timeout_seconds
            .unwrap_or(state.config.default_timeout_secs),
    );
    let mut documents = Vec::new();
    for provider in providers {
        if urls.is_empty() {
            break;
        }
        let id = provider.id();
        if let Some(name) = request.account.as_deref()
            && !provider.known_account(name)
        {
            meta.failed.push(ProviderFailure {
                provider: id.as_str().into(),
                error: format!("unknown account '{name}' for provider {id}"),
            });
            continue;
        }
        let call_started = Instant::now();
        match tokio::time::timeout(
            timeout,
            provider.extract(&urls, request.account.as_deref()),
        )
        .await
        {
            Ok(Ok(docs)) if !docs.is_empty() => {
                state.health.mark_success(id, call_started.elapsed());
                meta.successful.push(id.as_str().into());
                let got: std::collections::HashSet<_> =
                    docs.iter().map(|d| d.url.clone()).collect();
                documents.extend(docs);
                urls.retain(|u| !got.contains(u));
            }
            Ok(Ok(_)) => {
                meta.skipped.push(ProviderSkip {
                    provider: id.as_str().into(),
                    reason: "empty extract".into(),
                });
            }
            Ok(Err(err)) => {
                warn!(provider = %id, error = %err, "extract failed");
                state.health.mark_failure(
                    id,
                    &err,
                    Duration::from_secs(state.config.cooldown_secs),
                    call_started.elapsed(),
                );
                meta.failed.push(ProviderFailure {
                    provider: id.as_str().into(),
                    error: err.to_string(),
                });
            }
            Err(_) => meta.timed_out.push(id.as_str().into()),
        }
    }
    meta.provider_used = meta.successful.clone();
    meta.cost_usd = meta
        .successful
        .iter()
        .filter_map(|name| ProviderId::parse(name).ok())
        .map(|id| provider_cost(state, id))
        .sum();
    meta.estimated_cost_usd = meta.cost_usd;
    meta.stop_reason = Some(if urls.is_empty() && meta.failed.is_empty() {
        "complete".into()
    } else if !meta.timed_out.is_empty() {
        "timeout".into()
    } else if !meta.failed.is_empty() {
        "partial_provider_failure".into()
    } else {
        "complete".into()
    });
    meta.elapsed_ms = started.elapsed().as_millis() as u64;
    ExtractResponse { documents, meta }
}

pub async fn research(
    state: &AppState,
    mut request: SearchRequest,
    extract_top: u32,
    budget_seconds: u64,
) -> ResearchResponse {
    request.include_quality_report = true;
    let search_account = request.account.clone();
    let started = Instant::now();
    let search = search(state, request).await;
    let spent = started.elapsed().as_secs();
    let remaining = budget_seconds.saturating_sub(spent).max(1);
    let urls: Vec<String> = search
        .results
        .iter()
        .map(|h| h.url.clone())
        .take(extract_top.max(1) as usize)
        .collect();
    let extracts = if urls.is_empty() {
        Vec::new()
    } else {
        extract(
            state,
            ExtractRequest {
                urls,
                providers: None,
                timeout_seconds: Some(remaining),
                account: search_account,
            },
        )
        .await
        .documents
    };
    ResearchResponse {
        search,
        extracts,
        extract_budget_seconds: remaining,
    }
}

pub async fn direct_fetch(state: &AppState, url: &str) -> crate::error::Result<ExtractedDoc> {
    let safe = assert_public_http_url(url)?;
    let response = state.http.get(safe.as_str()).send().await?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(Error::provider("direct_fetch", format!("HTTP {status}")));
    }
    Ok(ExtractedDoc {
        url: safe.to_string(),
        title: None,
        content: body.chars().take(20_000).collect(),
        provider: "direct_fetch".into(),
    })
}

fn maybe_spill_to_file(config: &Config, response: &mut SearchResponse) {
    let Ok(json) = serde_json::to_vec(response) else {
        return;
    };
    if json.len() <= config.inline_max_bytes {
        return;
    }
    let path = std::env::temp_dir().join(format!(
        "omnisearch-{}-{}.json",
        std::process::id(),
        response.meta.elapsed_ms
    ));
    if std::fs::write(&path, &json).is_ok() {
        let preview = response.results.len().min(8);
        response.results.truncate(preview);
        response.delivery = Some(crate::types::Delivery {
            mode: "file".into(),
            path: Some(path.display().to_string()),
            bytes: json.len() as u64,
        });
    }
}
