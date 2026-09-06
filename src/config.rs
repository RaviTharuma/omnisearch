//! Environment-only configuration.

use std::env;

use crate::keys::{GITHUB_KEY_NAMES, env_key_ring, env_key_rings};
use crate::types::{DEFAULT_RRF_K, SAFETY_BOUND, SearchMode};

/// Process configuration loaded from the environment.
#[derive(Debug, Clone)]
pub struct Config {
    pub user_agent: String,
    pub country: String,
    pub language: String,
    pub rrf_k: f64,
    pub safety_bound: usize,
    pub max_pages_per_provider: usize,
    pub default_timeout_secs: u64,
    pub cache_ttl_secs: u64,
    pub cooldown_secs: u64,
    pub auto_allow_usd: f64,
    pub max_per_domain: usize,
    pub inline_max_bytes: usize,
    pub cache_partial: bool,
    pub default_mode: SearchMode,
    pub ground_top: usize,
    pub evidence_min: u32,
    pub http_bind: String,
    pub auth_tokens: Vec<String>,
    pub http_rpm: u32,
    pub request_timeout_secs: u64,
    pub keys: ProviderKeys,
    pub endpoints: Endpoints,
    pub mcp_backends: Vec<McpBackend>,
    pub keenable_public: bool,
    pub keenable_title: String,
}

/// Downstream MCP search backend.
#[derive(Debug, Clone)]
pub struct McpBackend {
    pub name: String,
    pub url: String,
    pub token: Option<String>,
}

/// Secret material. Never serialize into tool output.
#[derive(Debug, Clone, Default)]
pub struct ProviderKeys {
    pub tavily: Vec<String>,
    pub exa: Vec<String>,
    pub firecrawl: Vec<String>,
    pub linkup: Vec<String>,
    pub brave: Vec<String>,
    pub kagi: Vec<String>,
    pub github: Vec<String>,
    pub x_bearer: Vec<String>,
    pub xai: Vec<String>,
    pub reddit_client_id: Option<String>,
    pub reddit_client_secret: Option<String>,
    pub youtube: Vec<String>,
    pub instagram_token: Vec<String>,
    pub instagram_user_id: Option<String>,
    pub facebook_token: Vec<String>,
    pub youcom: Vec<String>,
    pub parallel: Vec<String>,
    pub querit: Vec<String>,
    pub tinyfish: Vec<String>,
    pub keenable: Vec<String>,
    pub perplexity: Vec<String>,
    pub mastodon_token: Option<String>,
    pub mastodon_instance: Option<String>,
    pub bluesky_handle: Option<String>,
    pub bluesky_app_password: Option<String>,
}

/// Overridable API bases (tests + self-host).
#[derive(Debug, Clone)]
pub struct Endpoints {
    pub tavily: String,
    pub exa: String,
    pub firecrawl: String,
    pub linkup: String,
    pub brave: String,
    pub kagi: String,
    pub github: String,
    pub x: String,
    pub xai: String,
    pub reddit: String,
    pub reddit_oauth: String,
    pub youtube: String,
    pub meta_graph: String,
    pub youcom: String,
    pub parallel: String,
    pub querit: String,
    pub tinyfish: String,
    pub keenable: String,
    pub perplexity: String,
    pub wikipedia: String,
    pub scholar: String,
    pub bluesky: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            tavily: "https://api.tavily.com".into(),
            exa: "https://api.exa.ai".into(),
            firecrawl: "https://api.firecrawl.dev".into(),
            linkup: "https://api.linkup.so".into(),
            brave: "https://api.search.brave.com".into(),
            kagi: "https://kagi.com".into(),
            github: "https://api.github.com".into(),
            x: "https://api.x.com".into(),
            xai: "https://api.x.ai".into(),
            reddit: "https://www.reddit.com".into(),
            reddit_oauth: "https://oauth.reddit.com".into(),
            youtube: "https://www.googleapis.com".into(),
            meta_graph: "https://graph.facebook.com/v22.0".into(),
            youcom: "https://ydc-index.io".into(),
            parallel: "https://api.parallel.ai".into(),
            querit: "https://api.querit.ai".into(),
            tinyfish: "https://api.tinyfish.dev".into(),
            keenable: "https://api.keenable.ai".into(),
            perplexity: "https://api.perplexity.ai".into(),
            wikipedia: "https://en.wikipedia.org".into(),
            scholar: "https://api.semanticscholar.org".into(),
            bluesky: "https://public.api.bsky.app".into(),
        }
    }
}

impl Config {
    /// Load configuration from process environment.
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();
        let endpoints = Endpoints {
            tavily: env_or("TAVILY_BASE_URL", "https://api.tavily.com"),
            exa: env_or("EXA_BASE_URL", "https://api.exa.ai"),
            firecrawl: env_or("FIRECRAWL_BASE_URL", "https://api.firecrawl.dev"),
            linkup: env_or("LINKUP_BASE_URL", "https://api.linkup.so"),
            brave: env_or("BRAVE_BASE_URL", "https://api.search.brave.com"),
            kagi: env_or("KAGI_BASE_URL", "https://kagi.com"),
            github: env_or("GITHUB_API_URL", "https://api.github.com"),
            x: env_or("X_API_BASE_URL", "https://api.x.com"),
            xai: env_or("XAI_BASE_URL", "https://api.x.ai"),
            reddit: env_or("REDDIT_BASE_URL", "https://www.reddit.com"),
            reddit_oauth: env_or("REDDIT_OAUTH_BASE_URL", "https://oauth.reddit.com"),
            youtube: env_or("YOUTUBE_BASE_URL", "https://www.googleapis.com"),
            meta_graph: env_or("META_GRAPH_BASE_URL", "https://graph.facebook.com/v22.0"),
            youcom: env_or("YOU_BASE_URL", "https://ydc-index.io"),
            parallel: env_or("PARALLEL_BASE_URL", "https://api.parallel.ai"),
            querit: env_or("QUERIT_BASE_URL", "https://api.querit.ai"),
            tinyfish: env_or("TINYFISH_BASE_URL", "https://api.tinyfish.dev"),
            keenable: env_or("KEENABLE_BASE_URL", "https://api.keenable.ai"),
            perplexity: env_or("PERPLEXITY_BASE_URL", "https://api.perplexity.ai"),
            wikipedia: env_or("WIKIPEDIA_BASE_URL", "https://en.wikipedia.org"),
            scholar: env_or(
                "SEMANTIC_SCHOLAR_BASE_URL",
                "https://api.semanticscholar.org",
            ),
            bluesky: env_or("BLUESKY_BASE_URL", "https://public.api.bsky.app"),
        };

        Self {
            user_agent: env_or(
                "OMNISEARCH_USER_AGENT",
                "omnisearch/0.1.0 (+https://github.com/RaviTharuma/omnisearch)",
            ),
            country: env_or("OMNISEARCH_COUNTRY", "CH"),
            language: env_or("OMNISEARCH_LANGUAGE", "de"),
            rrf_k: env_f64("OMNISEARCH_RRF_K", DEFAULT_RRF_K),
            safety_bound: env_usize("OMNISEARCH_SAFETY_BOUND", SAFETY_BOUND),
            max_pages_per_provider: env_usize("OMNISEARCH_MAX_PAGES_PER_PROVIDER", 250),
            default_timeout_secs: env_u64("OMNISEARCH_TIMEOUT_SECONDS", 30),
            cache_ttl_secs: env_u64("OMNISEARCH_CACHE_TTL_SECS", 120),
            cooldown_secs: env_u64("OMNISEARCH_COOLDOWN_SECS", 60),
            auto_allow_usd: env_f64("OMNISEARCH_AUTO_ALLOW_USD", 0.05),
            max_per_domain: env_usize("OMNISEARCH_MAX_PER_DOMAIN", 12),
            inline_max_bytes: env_usize("OMNISEARCH_INLINE_MAX_BYTES", 120_000),
            cache_partial: env_bool("OMNISEARCH_CACHE_PARTIAL"),
            default_mode: parse_mode(&env_or("OMNISEARCH_ROUTE", "all")),
            ground_top: env_usize("OMNISEARCH_GROUND_TOP", 0),
            evidence_min: env_u64("OMNISEARCH_EVIDENCE_MIN", 8) as u32,
            http_bind: env_or("OMNISEARCH_HTTP_BIND", "127.0.0.1:48731"),
            auth_tokens: split_csv(env::var("AUTH_TOKENS").ok()),
            http_rpm: env_u64("OMNISEARCH_HTTP_RPM", 120) as u32,
            request_timeout_secs: env_u64("OMNISEARCH_HTTP_TIMEOUT_SECS", 45),
            keys: ProviderKeys {
                tavily: env_key_ring("TAVILY_API_KEY"),
                exa: env_key_ring("EXA_API_KEY"),
                firecrawl: env_key_ring("FIRECRAWL_API_KEY"),
                linkup: env_key_ring("LINKUP_API_KEY"),
                brave: env_key_ring("BRAVE_API_KEY"),
                kagi: env_key_ring("KAGI_API_KEY"),
                github: env_key_rings(GITHUB_KEY_NAMES),
                x_bearer: env_key_rings(&["X_BEARER_TOKEN", "TWITTER_BEARER_TOKEN"]),
                xai: env_key_ring("XAI_API_KEY"),
                reddit_client_id: env::var("REDDIT_CLIENT_ID").ok().filter(|s| !s.is_empty()),
                reddit_client_secret: env::var("REDDIT_CLIENT_SECRET")
                    .ok()
                    .filter(|s| !s.is_empty()),
                youtube: env_key_rings(&["YOUTUBE_API_KEY", "GOOGLE_API_KEY"]),
                instagram_token: env_key_rings(&["INSTAGRAM_ACCESS_TOKEN", "META_ACCESS_TOKEN"]),
                instagram_user_id: env::var("INSTAGRAM_BUSINESS_ACCOUNT_ID")
                    .ok()
                    .filter(|s| !s.is_empty()),
                facebook_token: env_key_rings(&["FACEBOOK_ACCESS_TOKEN", "META_ACCESS_TOKEN"]),
                youcom: env_key_rings(&["YOU_API_KEY", "YDC_API_KEY"]),
                parallel: env_key_ring("PARALLEL_API_KEY"),
                querit: env_key_ring("QUERIT_API_KEY"),
                tinyfish: env_key_ring("TINYFISH_API_KEY"),
                keenable: env_key_ring("KEENABLE_API_KEY"),
                perplexity: env_key_rings(&["PERPLEXITY_API_KEY", "PPLX_API_KEY"]),
                mastodon_token: env::var("MASTODON_ACCESS_TOKEN")
                    .ok()
                    .filter(|s| !s.is_empty()),
                mastodon_instance: env::var("MASTODON_INSTANCE").ok().filter(|s| !s.is_empty()),
                bluesky_handle: env::var("BLUESKY_HANDLE").ok().filter(|s| !s.is_empty()),
                bluesky_app_password: env::var("BLUESKY_APP_PASSWORD")
                    .ok()
                    .filter(|s| !s.is_empty()),
            },
            endpoints,
            mcp_backends: parse_backends(env::var("OMNISEARCH_MCP_BACKENDS").ok()),
            keenable_public: env_bool("KEENABLE_PUBLIC"),
            keenable_title: env_or("KEENABLE_TITLE", "omnisearch"),
        }
    }
}

/// Environment string or default.
fn env_or(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn env_u64(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn env_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn env_f64(name: &str, default: f64) -> f64 {
    env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn env_bool(name: &str) -> bool {
    matches!(
        env::var(name).ok().as_deref(),
        Some("1" | "true" | "TRUE" | "yes" | "YES")
    )
}

fn split_csv(raw: Option<String>) -> Vec<String> {
    raw.unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// Official remote MCP endpoints (tokens come from the matching env key).
pub const OFFICIAL_MCP_REMOTES: &[(&str, &str, &str)] = &[
    ("tavily", "https://mcp.tavily.com/mcp", "TAVILY_API_KEY"),
    ("exa", "https://mcp.exa.ai/mcp", "EXA_API_KEY"),
    (
        "firecrawl",
        "https://mcp.firecrawl.dev/mcp",
        "FIRECRAWL_API_KEY",
    ),
    ("linkup", "https://mcp.linkup.so/mcp", "LINKUP_API_KEY"),
    ("kagi", "https://kagi.com/api/mcp", "KAGI_API_KEY"),
    (
        "perplexity",
        "https://mcp.perplexity.ai/mcp",
        "PERPLEXITY_API_KEY",
    ),
];

fn parse_mode(raw: &str) -> SearchMode {
    match raw.trim().to_ascii_lowercase().as_str() {
        "auto" => SearchMode::Auto,
        "ladder" | "free_first" | "free-first" => SearchMode::Ladder,
        _ => SearchMode::All,
    }
}

/// Parse `name|url|token,name2|url2` plus the `official` preset token.
fn parse_backends(raw: Option<String>) -> Vec<McpBackend> {
    let mut backends = Vec::new();
    for item in raw.unwrap_or_default().split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        if item.eq_ignore_ascii_case("official") {
            backends.extend(official_backends());
            continue;
        }
        let mut parts = item.split('|');
        let Some(name) = parts.next().map(str::trim).filter(|s| !s.is_empty()) else {
            continue;
        };
        let Some(url) = parts.next().map(str::trim).filter(|s| !s.is_empty()) else {
            continue;
        };
        let token = parts
            .next()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);
        backends.push(McpBackend {
            name: name.to_string(),
            url: url.to_string(),
            token,
        });
    }
    backends
}

fn official_backends() -> Vec<McpBackend> {
    OFFICIAL_MCP_REMOTES
        .iter()
        .filter_map(|(name, url, env_name)| {
            let token = env_key_ring(env_name).into_iter().next()?;
            Some(McpBackend {
                name: (*name).to_string(),
                url: (*url).to_string(),
                token: Some(token),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{OFFICIAL_MCP_REMOTES, parse_backends};

    #[test]
    fn parses_backend_specs() {
        let backends = parse_backends(Some(
            "alpha|https://a.example/mcp|tok,beta|https://b.example/mcp".into(),
        ));
        assert_eq!(backends.len(), 2);
        assert_eq!(backends[0].name, "alpha");
        assert_eq!(backends[0].token.as_deref(), Some("tok"));
        assert!(backends[1].token.is_none());
    }

    #[test]
    fn official_preset_is_documented() {
        assert_eq!(OFFICIAL_MCP_REMOTES.len(), 6);
        assert!(
            OFFICIAL_MCP_REMOTES
                .iter()
                .any(|(name, _, _)| *name == "perplexity")
        );
    }
}
