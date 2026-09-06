//! MCP tool surface.

use std::sync::Arc;

use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{Implementation, ServerCapabilities, ServerInfo};
use rmcp::{ErrorData, ServerHandler, schemars, tool, tool_handler, tool_router};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Error;
use crate::orchestrator::{AppState, direct_fetch, extract, research, search};
use crate::types::{ExtractRequest, Freshness, ProviderId, SearchMode, SearchRequest, SearchType};

#[derive(Clone)]
pub struct OmniServer {
    pub state: Arc<AppState>,
}

impl OmniServer {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    fn parse_providers(raw: &Option<Vec<String>>) -> Result<Option<Vec<ProviderId>>, ErrorData> {
        match raw {
            None => Ok(None),
            Some(items) => items
                .iter()
                .map(|s| ProviderId::parse(s).map_err(|e| e.to_mcp()))
                .collect::<Result<Vec<_>, _>>()
                .map(Some),
        }
    }

    fn search_request(
        params: SearchParams,
        default_mode: SearchMode,
    ) -> Result<SearchRequest, ErrorData> {
        if params.query.trim().is_empty() {
            return Err(Error::Invalid("query is required".into()).to_mcp());
        }
        let mode = match params.mode.as_deref() {
            Some("auto") => SearchMode::Auto,
            Some("ladder") | Some("free_first") | Some("free-first") => SearchMode::Ladder,
            Some("all") => SearchMode::All,
            None => default_mode,
            _ => SearchMode::All,
        };
        let search_type = match params.search_type.as_deref() {
            Some(raw) => SearchType::parse(raw).map_err(|e| e.to_mcp())?,
            None => SearchType::Web,
        };
        let freshness = params
            .freshness
            .as_deref()
            .map(Freshness::parse)
            .transpose()
            .map_err(|e| e.to_mcp())?;
        Ok(SearchRequest {
            query: params.query,
            providers: Self::parse_providers(&params.providers)?,
            limit: params.limit,
            unlimited: params.unlimited.unwrap_or(false),
            mode,
            search_type,
            freshness,
            country: params.country,
            language: params.language,
            max_providers: params.max_providers.map(|n| n as usize),
            timeout_seconds: params.timeout_seconds,
            budget_usd: params.budget_usd,
            no_cache: params.no_cache.unwrap_or(false),
            cache_partial: params.cache_partial.unwrap_or(false),
            include_quality_report: params.quality_report.unwrap_or(false),
            ground_top: params.ground_top,
            evidence_min: params.evidence_min,
        })
    }
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct SearchParams {
    pub query: String,
    /// Restrict to these providers. Omit to fan out to every configured provider.
    pub providers: Option<Vec<String>>,
    /// Soft per-provider fetch hint. Does not cap the merged aggregate.
    pub limit: Option<u32>,
    /// Page until providers exhaust (still bounded by the safety bound).
    pub unlimited: Option<bool>,
    /// `all` (default parallel), `auto` (intent + health), or `ladder` (free-first).
    pub mode: Option<String>,
    /// `web`, `news`, `code`, `social`, `scholarly`, `video`, `users`.
    pub search_type: Option<String>,
    /// `day`, `week`, `month`, `year`.
    pub freshness: Option<String>,
    pub country: Option<String>,
    pub language: Option<String>,
    pub max_providers: Option<u32>,
    pub timeout_seconds: Option<u64>,
    pub budget_usd: Option<f64>,
    pub no_cache: Option<bool>,
    /// Cache even if some selected providers failed.
    pub cache_partial: Option<bool>,
    pub quality_report: Option<bool>,
    /// Fetch and rewrite snippets for the top N hits (SSRF-safe).
    pub ground_top: Option<u32>,
    /// Ladder stop: unique hits needed before skipping remaining paid providers.
    pub evidence_min: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct QueryParams {
    pub query: String,
    pub limit: Option<u32>,
    pub unlimited: Option<bool>,
    pub freshness: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct ExtractParams {
    pub urls: Vec<String>,
    pub providers: Option<Vec<String>>,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct ResearchParams {
    pub query: String,
    pub providers: Option<Vec<String>>,
    pub extract_top: Option<u32>,
    pub budget_seconds: Option<u64>,
    pub freshness: Option<String>,
    pub search_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct UrlParams {
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct CrawlParams {
    pub url: String,
    pub limit: Option<u32>,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct MapParams {
    pub url: String,
    pub search: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct BenchParams {
    pub query: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct GithubParams {
    pub query: String,
    /// `repo` (default), `code`, or `users`.
    pub kind: Option<String>,
    pub limit: Option<u32>,
}

#[tool_router]
impl OmniServer {
    /// Fan out to every configured provider in parallel, then RRF-merge and dedupe.
    #[tool(
        name = "search",
        description = "Unified parallel search across all configured providers. Default fans out to everyone, then RRF-merges and dedupes by URL/title. Optional providers[] filter. limit is a soft per-provider hint; omit it or set unlimited=true to page until exhaustion (safety bound 10k unique)."
    )]
    pub async fn search_tool(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<Json<Value>, ErrorData> {
        let req = Self::search_request(params, self.state.config.default_mode)?;
        let out = search(&self.state, req).await;
        to_json(&out)
    }

    /// Tavily-only search.
    #[tool(description = "Search with Tavily only.")]
    pub async fn tavily_search(
        &self,
        Parameters(params): Parameters<QueryParams>,
    ) -> Result<Json<Value>, ErrorData> {
        self.one(ProviderId::Tavily, params).await
    }

    /// Exa-only search.
    #[tool(description = "Neural/keyword search with Exa only.")]
    pub async fn exa_search(
        &self,
        Parameters(params): Parameters<QueryParams>,
    ) -> Result<Json<Value>, ErrorData> {
        self.one(ProviderId::Exa, params).await
    }

    /// Linkup-only search.
    #[tool(description = "Search with Linkup only.")]
    pub async fn linkup_search(
        &self,
        Parameters(params): Parameters<QueryParams>,
    ) -> Result<Json<Value>, ErrorData> {
        self.one(ProviderId::Linkup, params).await
    }

    /// X/Twitter search.
    #[tool(description = "Search X.com via X API v2 or xAI x_search.")]
    pub async fn x_search(
        &self,
        Parameters(params): Parameters<QueryParams>,
    ) -> Result<Json<Value>, ErrorData> {
        self.one(ProviderId::X, params).await
    }

    /// Reddit search.
    #[tool(description = "Search Reddit via search.json or OAuth.")]
    pub async fn reddit_search(
        &self,
        Parameters(params): Parameters<QueryParams>,
    ) -> Result<Json<Value>, ErrorData> {
        self.one(ProviderId::Reddit, params).await
    }

    /// YouTube search.
    #[tool(description = "Search YouTube with Data API v3.")]
    pub async fn youtube_search(
        &self,
        Parameters(params): Parameters<QueryParams>,
    ) -> Result<Json<Value>, ErrorData> {
        self.one(ProviderId::Youtube, params).await
    }

    /// Instagram hashtag search.
    #[tool(description = "Instagram Graph API hashtag search (not free-text).")]
    pub async fn instagram_search(
        &self,
        Parameters(params): Parameters<QueryParams>,
    ) -> Result<Json<Value>, ErrorData> {
        self.one(ProviderId::Instagram, params).await
    }

    /// Facebook page search.
    #[tool(
        description = "Facebook Pages Search. Public post keyword search is not offered by Meta."
    )]
    pub async fn facebook_search(
        &self,
        Parameters(params): Parameters<QueryParams>,
    ) -> Result<Json<Value>, ErrorData> {
        self.one(ProviderId::Facebook, params).await
    }

    /// Brave search.
    #[tool(
        description = "Search with Brave Search (BRAVE_API_KEY). Also included in default parallel fan-out."
    )]
    pub async fn brave_search(
        &self,
        Parameters(params): Parameters<QueryParams>,
    ) -> Result<Json<Value>, ErrorData> {
        self.one(ProviderId::Brave, params).await
    }

    /// Kagi search.
    #[tool(description = "Search with Kagi.")]
    pub async fn kagi_search(
        &self,
        Parameters(params): Parameters<QueryParams>,
    ) -> Result<Json<Value>, ErrorData> {
        self.one(ProviderId::Kagi, params).await
    }

    /// GitHub search.
    #[tool(
        description = "Search GitHub repositories (kind=repo, default), code (kind=code), or users (kind=users). Uses GITHUB_TOKEN or GITHUB_API_KEY."
    )]
    pub async fn github_search(
        &self,
        Parameters(params): Parameters<GithubParams>,
    ) -> Result<Json<Value>, ErrorData> {
        let mut req = SearchRequest::new(params.query);
        req.providers = Some(vec![ProviderId::Github]);
        req.limit = params.limit;
        req.unlimited = params.limit.is_none();
        req.search_type = crate::providers::github::parse_kind(params.kind.as_deref());
        to_json(&search(&self.state, req).await)
    }

    /// Answer-oriented search using vendors that return answers.
    #[tool(
        description = "Answer-oriented search. Fans out and returns vendor answers plus merged sources."
    )]
    pub async fn ai_search(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<Json<Value>, ErrorData> {
        let mut req = Self::search_request(params, self.state.config.default_mode)?;
        if req.providers.is_none() {
            req.providers = Some(vec![
                ProviderId::Tavily,
                ProviderId::Kagi,
                ProviderId::Youcom,
                ProviderId::Exa,
                ProviderId::Perplexity,
            ]);
        }
        to_json(&search(&self.state, req).await)
    }

    /// Extract URLs through a vendor cascade, then SSRF-safe direct fetch.
    #[tool(
        description = "Extract page content. Cascade: configured extract vendors, then SSRF-safe direct fetch."
    )]
    pub async fn extract_tool(
        &self,
        Parameters(params): Parameters<ExtractParams>,
    ) -> Result<Json<Value>, ErrorData> {
        if params.urls.is_empty() {
            return Err(Error::Invalid("urls is required".into()).to_mcp());
        }
        let mut out = extract(
            &self.state,
            ExtractRequest {
                urls: params.urls.clone(),
                providers: Self::parse_providers(&params.providers)?,
                timeout_seconds: params.timeout_seconds,
            },
        )
        .await;
        if out.documents.is_empty() {
            for url in &params.urls {
                match direct_fetch(&self.state, url).await {
                    Ok(doc) => out.documents.push(doc),
                    Err(err) => out.meta.failed.push(crate::types::ProviderFailure {
                        provider: "direct_fetch".into(),
                        error: err.to_string(),
                    }),
                }
            }
        }
        to_json(&out)
    }

    /// Alias for extract.
    #[tool(description = "Extract web pages (alias of extract).")]
    pub async fn web_extract(
        &self,
        Parameters(params): Parameters<ExtractParams>,
    ) -> Result<Json<Value>, ErrorData> {
        self.extract_tool(Parameters(params)).await
    }

    /// Research mode: search then extract top URLs under a time budget.
    #[tool(
        description = "Research mode: parallel search then extract the top URLs under a time budget."
    )]
    pub async fn research_tool(
        &self,
        Parameters(params): Parameters<ResearchParams>,
    ) -> Result<Json<Value>, ErrorData> {
        let mut req = SearchRequest::new(params.query);
        req.providers = Self::parse_providers(&params.providers)?;
        req.freshness = params
            .freshness
            .as_deref()
            .map(Freshness::parse)
            .transpose()
            .map_err(|e| e.to_mcp())?;
        req.search_type = match params.search_type.as_deref() {
            Some(raw) => SearchType::parse(raw).map_err(|e| e.to_mcp())?,
            None => SearchType::Web,
        };
        let out = research(
            &self.state,
            req,
            params.extract_top.unwrap_or(5),
            params.budget_seconds.unwrap_or(45),
        )
        .await;
        to_json(&out)
    }

    /// Firecrawl scrape.
    #[tool(description = "Scrape one URL with Firecrawl.")]
    pub async fn firecrawl_scrape(
        &self,
        Parameters(params): Parameters<UrlParams>,
    ) -> Result<Json<Value>, ErrorData> {
        let provider = self
            .state
            .registry
            .get(ProviderId::Firecrawl)
            .ok_or_else(|| Error::provider("firecrawl", "missing").to_mcp())?;
        let docs = provider
            .extract(&[params.url])
            .await
            .map_err(|e| e.to_mcp())?;
        to_json(&docs)
    }

    /// Firecrawl crawl.
    #[tool(description = "Crawl a site with Firecrawl and poll until done.")]
    pub async fn firecrawl_crawl(
        &self,
        Parameters(params): Parameters<CrawlParams>,
    ) -> Result<Json<Value>, ErrorData> {
        let fc = crate::providers::firecrawl::Firecrawl::new(
            &self.state.config,
            self.state.http.clone(),
        );
        let value = fc
            .crawl(
                &params.url,
                params.limit.unwrap_or(25),
                params.timeout_seconds.unwrap_or(90),
            )
            .await
            .map_err(|e| e.to_mcp())?;
        Ok(Json(value))
    }

    /// Firecrawl map.
    #[tool(description = "Map URLs on a site with Firecrawl.")]
    pub async fn firecrawl_map(
        &self,
        Parameters(params): Parameters<MapParams>,
    ) -> Result<Json<Value>, ErrorData> {
        let fc = crate::providers::firecrawl::Firecrawl::new(
            &self.state.config,
            self.state.http.clone(),
        );
        let value = fc
            .map(&params.url, params.search.as_deref(), params.limit)
            .await
            .map_err(|e| e.to_mcp())?;
        Ok(Json(value))
    }

    /// Non-secret provider metadata.
    #[tool(description = "List providers with configured/capability flags. Never returns secrets.")]
    pub async fn get_provider_info(&self) -> Result<Json<Value>, ErrorData> {
        to_json(&self.state.registry.infos())
    }

    /// Live provider health: configured, cooldown, latency, errors, requires_key.
    #[tool(
        description = "Live provider health: configured, cooldown, recent latency/errors, requires_key. Never returns secrets."
    )]
    pub async fn search_health(&self) -> Result<Json<Value>, ErrorData> {
        to_json(&self.state.search_health())
    }

    /// Quality diagnostics for a fresh search.
    #[tool(description = "Run a search and return the quality_report plus meta.")]
    pub async fn quality_report(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<Json<Value>, ErrorData> {
        let mut req = Self::search_request(params, self.state.config.default_mode)?;
        req.include_quality_report = true;
        let out = search(&self.state, req).await;
        to_json(&serde_json::json!({
            "meta": out.meta,
            "quality_report": out.quality_report,
            "unique_count": out.unique_count
        }))
    }

    /// Latency bench against configured providers.
    #[tool(description = "Benchmark configured providers with a short query.")]
    pub async fn provider_bench(
        &self,
        Parameters(params): Parameters<BenchParams>,
    ) -> Result<Json<Value>, ErrorData> {
        let query = params.query.unwrap_or_else(|| "omnisearch rust mcp".into());
        let report = crate::bench::run_bench(&self.state, &query).await;
        to_json(&report)
    }
}

impl OmniServer {
    async fn one(&self, id: ProviderId, params: QueryParams) -> Result<Json<Value>, ErrorData> {
        let mut req = SearchRequest::new(params.query);
        req.providers = Some(vec![id]);
        req.limit = params.limit;
        req.unlimited = params.unlimited.unwrap_or(params.limit.is_none());
        req.freshness = params
            .freshness
            .as_deref()
            .map(Freshness::parse)
            .transpose()
            .map_err(|e| e.to_mcp())?;
        to_json(&search(&self.state, req).await)
    }
}

fn to_json<T: Serialize>(value: &T) -> Result<Json<Value>, ErrorData> {
    serde_json::to_value(value)
        .map(Json)
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))
}

#[tool_handler(
    name = "omnisearch",
    version = "0.1.0",
    instructions = "Unified multi-provider search MCP. Call search for parallel fan-out, then RRF merge. Use research for search+extract. Use search_health and get_provider_info for non-secret status."
)]
impl ServerHandler for OmniServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "omnisearch",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Parallel multi-provider search MCP. Default search fans out to every configured engine (Brave and GitHub are first-class when keyed), then RRF-merges with URL/title dedupe and sources[] provenance. limit is a soft hint; omit it for unlimited pagination up to the 10k safety bound. Budgets cap spend/providers/time, not result count. github_search supports repo, code, and users. Social: YouTube (Data API), Instagram (hashtag Graph API), Facebook (pages/search only), X, Reddit, Mastodon, Bluesky.",
            )
    }
}
