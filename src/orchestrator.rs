//! Parallel fan-out, budgets, cache, pagination, and merge.

use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::future::join_all;
use tracing::{info, warn};

use crate::cache::SearchCache;
use crate::config::Config;
use crate::error::Error;
use crate::health::HealthBoard;
use crate::http::HttpClient;
use crate::intent::select_providers;
use crate::merge::{apply_freshness, rrf_merge};
use crate::providers::{Provider, Registry, page_size_for};
use crate::ssrf::assert_public_http_url;
use crate::types::{
    ExtractRequest, ExtractResponse, ExtractedDoc, ProviderFailure, ProviderId,
    ProviderSearchRequest, ProviderSkip, ResearchResponse, RunMeta, SearchRequest, SearchResponse,
};

/// Shared process state.
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
}

/// Run a unified search.
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

    // Cost gates: skip expensive providers unless explicitly requested.
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

    if let Some(max) = request.max_providers {
        selected.truncate(max);
    }

    let mut estimated_cost = 0.0;
    if let Some(budget) = request.budget_usd {
        let mut kept = Vec::new();
        for id in selected {
            let cost = state
                .registry
                .get(id)
                .map(|p| p.estimated_search_usd())
                .unwrap_or(0.0);
            if estimated_cost + cost > budget && !kept.is_empty() {
                skipped.push(ProviderSkip {
                    provider: id.as_str().into(),
                    reason: format!("budget_usd {budget} exhausted"),
                });
                continue;
            }
            estimated_cost += cost;
            kept.push(id);
        }
        selected = kept;
    } else {
        estimated_cost = selected
            .iter()
            .filter_map(|id| state.registry.get(*id).map(|p| p.estimated_search_usd()))
            .sum();
    }

    let timeout = Duration::from_secs(
        request
            .timeout_seconds
            .unwrap_or(state.config.default_timeout_secs),
    );

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
        async move {
            let Some(provider) = provider else {
                return (id, Err(Error::provider(id.as_str(), "missing provider")));
            };
            let run = collect_provider(
                provider,
                &req,
                &country,
                &language,
                state.config.max_pages_per_provider,
                state.config.safety_bound,
            );
            match tokio::time::timeout(timeout, run).await {
                Ok(result) => (id, result),
                Err(_) => (
                    id,
                    Err(Error::Timeout {
                        provider: id.as_str().into(),
                        seconds: timeout.as_secs(),
                    }),
                ),
            }
        }
    });

    let outcomes = join_all(futs).await;
    let mut lists = Vec::new();
    let mut successful = Vec::new();
    let mut failed = Vec::new();
    let mut timed_out = Vec::new();
    let mut answers = Vec::new();

    for (id, outcome) in outcomes {
        match outcome {
            Ok((hits, answer)) => {
                state.health.mark_success(id);
                if let Some(answer) = answer {
                    answers.push(answer);
                }
                successful.push(id.as_str().to_string());
                lists.push(hits);
            }
            Err(err) => {
                state.health.mark_failure(
                    id,
                    &err,
                    Duration::from_secs(state.config.cooldown_secs),
                );
                if matches!(err, Error::Timeout { .. }) {
                    timed_out.push(id.as_str().to_string());
                } else {
                    failed.push(ProviderFailure {
                        provider: id.as_str().into(),
                        error: err.to_string(),
                    });
                }
            }
        }
    }

    let merged = rrf_merge(lists, state.config.rrf_k, state.config.max_per_domain);
    let mut hits = merged.hits;
    if let Some(fresh) = request.freshness {
        hits = apply_freshness(hits, &fresh.since_rfc3339());
    }

    let mut truncated = false;
    if hits.len() > state.config.safety_bound {
        hits.truncate(state.config.safety_bound);
        truncated = true;
    }

    let unique_count = hits.len() as u32;
    let mut response = SearchResponse {
        query: request.query.clone(),
        unique_count,
        answer: answers.into_iter().next(),
        results: hits,
        quality_report: request.include_quality_report.then_some(merged.quality),
        delivery: None,
        meta: RunMeta {
            selected: selected.iter().map(|id| id.as_str().to_string()).collect(),
            successful,
            failed,
            timed_out,
            skipped,
            cache_hit: false,
            truncated,
            safety_bound: state.config.safety_bound as u32,
            rrf_k: state.config.rrf_k,
            estimated_cost_usd: estimated_cost,
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
    };
    maybe_spill_to_file(&state.config, &mut response);
    if !request.no_cache {
        state.cache.put(cache_key, response.clone());
    }
    info!(
        query = %request.query,
        providers = response.meta.successful.len(),
        results = response.unique_count,
        "search complete"
    );
    response
}

async fn collect_provider(
    provider: Arc<dyn Provider>,
    request: &SearchRequest,
    country: &str,
    language: &str,
    max_pages: usize,
    safety: usize,
) -> crate::error::Result<(Vec<crate::types::SearchHit>, Option<String>)> {
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

/// Tiered extract cascade with SSRF guards.
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
        match tokio::time::timeout(timeout, provider.extract(&urls)).await {
            Ok(Ok(docs)) if !docs.is_empty() => {
                state.health.mark_success(id);
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
                );
                meta.failed.push(ProviderFailure {
                    provider: id.as_str().into(),
                    error: err.to_string(),
                });
            }
            Err(_) => meta.timed_out.push(id.as_str().into()),
        }
    }
    meta.elapsed_ms = started.elapsed().as_millis() as u64;
    ExtractResponse { documents, meta }
}

/// Fan-out search then extract top URLs under a remaining time budget.
pub async fn research(
    state: &AppState,
    mut request: SearchRequest,
    extract_top: u32,
    budget_seconds: u64,
) -> ResearchResponse {
    request.include_quality_report = true;
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

/// Direct fetch fallback used by web_extract when vendors fail.
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
