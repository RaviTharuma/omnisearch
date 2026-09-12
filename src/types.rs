//! Shared request/response types and provider identifiers.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Hard safety bound on unique merged results. Not a product default cap.
pub const SAFETY_BOUND: usize = 10_000;

pub const DEFAULT_RRF_K: f64 = 60.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderId {
    Tavily,
    Exa,
    Firecrawl,
    Linkup,
    Brave,
    Kagi,
    Github,
    X,
    Reddit,
    Discord,
    Youtube,
    Instagram,
    Facebook,
    Youcom,
    Parallel,
    Querit,
    Tinyfish,
    Keenable,
    Perplexity,
    Wikipedia,
    Scholar,
    Mastodon,
    Bluesky,
    McpBackend,
    Omniroute,
}

impl ProviderId {
    /// First-class providers that always join default parallel fan-out when keyed.
    pub const fn must_have() -> &'static [Self] {
        &[Self::Brave, Self::Github]
    }

    pub const fn all() -> &'static [Self] {
        &[
            Self::Tavily,
            Self::Exa,
            Self::Firecrawl,
            Self::Linkup,
            Self::Brave,
            Self::Kagi,
            Self::Youcom,
            Self::Parallel,
            Self::Querit,
            Self::Tinyfish,
            Self::Keenable,
            Self::Perplexity,
            Self::Github,
            Self::Reddit,
            Self::X,
            Self::Discord,
            Self::Youtube,
            Self::Instagram,
            Self::Facebook,
            Self::Wikipedia,
            Self::Scholar,
            Self::Mastodon,
            Self::Bluesky,
            Self::McpBackend,
            Self::Omniroute,
        ]
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tavily => "tavily",
            Self::Exa => "exa",
            Self::Firecrawl => "firecrawl",
            Self::Linkup => "linkup",
            Self::Brave => "brave",
            Self::Kagi => "kagi",
            Self::Github => "github",
            Self::X => "x",
            Self::Reddit => "reddit",
            Self::Discord => "discord",
            Self::Youtube => "youtube",
            Self::Instagram => "instagram",
            Self::Facebook => "facebook",
            Self::Youcom => "youcom",
            Self::Parallel => "parallel",
            Self::Querit => "querit",
            Self::Tinyfish => "tinyfish",
            Self::Keenable => "keenable",
            Self::Perplexity => "perplexity",
            Self::Wikipedia => "wikipedia",
            Self::Scholar => "scholar",
            Self::Mastodon => "mastodon",
            Self::Bluesky => "bluesky",
            Self::McpBackend => "mcp_backend",
            Self::Omniroute => "omniroute",
        }
    }

    pub fn parse(raw: &str) -> crate::error::Result<Self> {
        let key = raw.trim().to_ascii_lowercase();
        let id = match key.as_str() {
            "tavily" => Self::Tavily,
            "exa" => Self::Exa,
            "firecrawl" => Self::Firecrawl,
            "linkup" => Self::Linkup,
            "brave" => Self::Brave,
            "kagi" => Self::Kagi,
            "github" | "gh" => Self::Github,
            "x" | "xsearch" | "twitter" => Self::X,
            "reddit" => Self::Reddit,
            "discord" => Self::Discord,
            "youtube" | "yt" => Self::Youtube,
            "instagram" | "ig" => Self::Instagram,
            "facebook" | "fb" | "meta" => Self::Facebook,
            "youcom" | "you" | "you.com" => Self::Youcom,
            "parallel" | "parallel.ai" => Self::Parallel,
            "querit" => Self::Querit,
            "tinyfish" => Self::Tinyfish,
            "keenable" => Self::Keenable,
            "perplexity" | "pplx" | "sonar" => Self::Perplexity,
            "wikipedia" | "wiki" => Self::Wikipedia,
            "scholar" | "semantic_scholar" | "semanticscholar" => Self::Scholar,
            "mastodon" => Self::Mastodon,
            "bluesky" | "bsky" => Self::Bluesky,
            "omniroute" => Self::Omniroute,
            "mcp_backend" | "mcp" | "backend" => Self::McpBackend,
            other => {
                return Err(crate::error::Error::Invalid(format!(
                    "unknown provider '{other}'"
                )));
            }
        };
        Ok(id)
    }
}

impl std::fmt::Display for ProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    /// Fan out to every configured (and allowed) provider. Default.
    #[default]
    All,
    /// Query-intent plus health-based subset.
    Auto,
    /// Free providers first, then paid by rising cost. May stop on evidence.
    Ladder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum SearchType {
    #[default]
    Web,
    News,
    Code,
    Social,
    Scholarly,
    Video,
    /// GitHub user/org search (`/search/users`).
    Users,
}

impl SearchType {
    pub fn parse(raw: &str) -> crate::error::Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "web" | "general" | "repo" | "repos" | "repositories" => Ok(Self::Web),
            "news" => Ok(Self::News),
            "code" => Ok(Self::Code),
            "social" => Ok(Self::Social),
            "scholarly" | "scholar" | "academic" => Ok(Self::Scholarly),
            "video" => Ok(Self::Video),
            "users" | "user" | "people" => Ok(Self::Users),
            other => Err(crate::error::Error::Invalid(format!(
                "unknown search_type '{other}'"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    Day,
    Week,
    Month,
    Year,
}

impl Freshness {
    /// Parse a freshness token.
    pub fn parse(raw: &str) -> crate::error::Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "day" | "d" | "pd" => Ok(Self::Day),
            "week" | "w" | "pw" => Ok(Self::Week),
            "month" | "m" | "pm" => Ok(Self::Month),
            "year" | "y" | "py" => Ok(Self::Year),
            other => Err(crate::error::Error::Invalid(format!(
                "unknown freshness '{other}'"
            ))),
        }
    }

    /// Approximate lower bound timestamp (RFC3339) relative to now.
    pub fn since_rfc3339(self) -> String {
        let days = match self {
            Self::Day => 1,
            Self::Week => 7,
            Self::Month => 31,
            Self::Year => 365,
        };
        (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SearchHit {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    /// Providers that contributed this URL after RRF merge.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    /// Blended confidence after RRF (recency + multi-source trust).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// True when the snippet was rewritten from a fetched page.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub snippet_grounded: bool,
}

impl SearchHit {
    pub fn new(
        provider: ProviderId,
        title: impl Into<String>,
        url: impl Into<String>,
        snippet: impl Into<String>,
    ) -> Self {
        let provider = provider.as_str().to_string();
        Self {
            title: title.into(),
            url: url.into(),
            snippet: snippet.into(),
            sources: vec![provider.clone()],
            provider,
            score: None,
            published_at: None,
            confidence: None,
            snippet_grounded: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ExtractedDoc {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub content: String,
    pub provider: String,
}

#[derive(Debug, Clone, Default)]
pub struct SearchPage {
    pub hits: Vec<SearchHit>,
    pub next_cursor: Option<String>,
    pub answer: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SearchRequest {
    pub query: String,
    pub providers: Option<Vec<ProviderId>>,
    pub limit: Option<u32>,
    pub unlimited: bool,
    pub mode: SearchMode,
    pub search_type: SearchType,
    pub freshness: Option<Freshness>,
    pub country: Option<String>,
    pub language: Option<String>,
    pub max_providers: Option<usize>,
    pub timeout_seconds: Option<u64>,
    pub budget_usd: Option<f64>,
    pub no_cache: bool,
    /// Cache this response even if some selected providers failed.
    pub cache_partial: bool,
    pub include_quality_report: bool,
    /// Fetch and rewrite snippets for the top N hits (SSRF-safe).
    pub ground_top: Option<u32>,
    /// Ladder/evidence stop: enough unique hits to skip remaining paid providers.
    pub evidence_min: Option<u32>,
    /// Pin to a named `OMNISEARCH_ACCOUNTS` entry. Omit for round-robin/failover.
    pub account: Option<String>,
    /// Provider-specific search depth when supported (e.g. Linkup: fast/standard/deep).
    pub depth: Option<String>,
}

impl SearchRequest {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            providers: None,
            limit: None,
            unlimited: true,
            mode: SearchMode::All,
            search_type: SearchType::Web,
            freshness: None,
            country: None,
            language: None,
            max_providers: None,
            timeout_seconds: None,
            budget_usd: None,
            no_cache: false,
            cache_partial: false,
            include_quality_report: false,
            ground_top: None,
            evidence_min: None,
            account: None,
            depth: None,
        }
    }

    /// True when callers asked to page until exhaustion.
    pub fn wants_unlimited(&self) -> bool {
        self.unlimited || self.limit.is_none()
    }
}

#[derive(Debug, Clone)]
pub struct ProviderSearchRequest<'a> {
    pub query: &'a str,
    pub cursor: Option<&'a str>,
    pub page_size: u32,
    pub search_type: SearchType,
    pub freshness: Option<Freshness>,
    pub country: &'a str,
    pub language: &'a str,
    /// Pin to a named account when the provider uses `OMNISEARCH_ACCOUNTS`.
    pub account: Option<&'a str>,
    /// Optional depth hint for providers that expose it (Linkup).
    pub depth: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default, PartialEq)]
pub struct RunMeta {
    pub selected: Vec<String>,
    pub successful: Vec<String>,
    pub failed: Vec<ProviderFailure>,
    pub timed_out: Vec<String>,
    pub skipped: Vec<ProviderSkip>,
    pub cache_hit: bool,
    pub truncated: bool,
    pub safety_bound: u32,
    pub rrf_k: f64,
    pub estimated_cost_usd: f64,
    /// Estimated USD of providers that actually succeeded.
    pub cost_usd: f64,
    /// Providers that returned hits or an empty-but-successful page.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provider_used: Vec<String>,
    /// Why the run stopped: complete, partial_provider_failure, timeout, budget_usd, max_providers, safety_bound, evidence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ProviderFailure {
    pub provider: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ProviderSkip {
    pub provider: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct QualityReport {
    pub unique_results: u32,
    pub unique_domains: u32,
    pub providers_contributing: u32,
    pub with_published_at: u32,
    pub spam_dropped: u32,
    pub top_domains: Vec<DomainCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct DomainCount {
    pub domain: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchHit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    pub unique_count: u32,
    pub meta: RunMeta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_report: Option<QualityReport>,
    /// When the payload is large, results may be stored on disk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery: Option<Delivery>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Delivery {
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderInfo {
    pub id: String,
    pub configured: bool,
    pub search: bool,
    pub extract: bool,
    pub estimated_search_usd: f64,
    pub requires_key: bool,
    pub notes: String,
}

/// Live health snapshot for one provider (no secrets).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderHealth {
    pub id: String,
    pub configured: bool,
    pub requires_key: bool,
    pub cooling: bool,
    pub cooldown_remaining_secs: u64,
    pub success: u64,
    pub failure: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub estimated_search_usd: f64,
    pub notes: String,
}

#[derive(Debug, Clone)]
pub struct ExtractRequest {
    pub urls: Vec<String>,
    pub providers: Option<Vec<ProviderId>>,
    pub timeout_seconds: Option<u64>,
    /// Pin to a named `OMNISEARCH_ACCOUNTS` entry. Omit for round-robin/failover.
    pub account: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractResponse {
    pub documents: Vec<ExtractedDoc>,
    pub meta: RunMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResearchResponse {
    pub search: SearchResponse,
    pub extracts: Vec<ExtractedDoc>,
    pub extract_budget_seconds: u64,
}
