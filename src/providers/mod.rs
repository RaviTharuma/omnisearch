//! Search/extract provider implementations.

mod bluesky;
mod brave;
mod exa;
mod facebook;
pub mod firecrawl;
pub mod github;
mod instagram;
mod kagi;
mod keenable;
mod linkup;
mod mastodon;
mod mcp_backend;
mod parallel;
mod perplexity;
mod querit;
mod reddit;
mod scholar;
mod tavily;
mod tinyfish;
mod wikipedia;
mod xsearch;
mod youcom;
mod youtube;

use std::sync::Arc;

use async_trait::async_trait;

use crate::config::Config;
use crate::error::Result;
use crate::http::HttpClient;
use crate::types::{
    ExtractedDoc, ProviderId, ProviderInfo, ProviderSearchRequest, SearchPage, SearchType,
};

/// A search and optional extract backend.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Stable id.
    fn id(&self) -> ProviderId;
    /// Whether secrets/config are present.
    fn is_configured(&self) -> bool;
    /// Human reason when not configured.
    fn skip_reason(&self) -> Option<String>;
    /// Whether this backend can search.
    fn supports_search(&self) -> bool {
        true
    }
    /// Whether this backend can extract URLs.
    fn supports_extract(&self) -> bool {
        false
    }
    /// Approximate USD per search call.
    fn estimated_search_usd(&self) -> f64 {
        0.005
    }
    /// Provider-native page size ceiling.
    fn max_page_size(&self) -> u32 {
        20
    }
    /// Extra documentation for get_provider_info.
    fn notes(&self) -> &'static str {
        ""
    }
    /// Whether a secret is required for this backend to run.
    fn requires_key(&self) -> bool {
        true
    }
    /// Execute one result page.
    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage>;
    /// Extract documents when supported.
    async fn extract(&self, _urls: &[String]) -> Result<Vec<ExtractedDoc>> {
        Ok(Vec::new())
    }

    /// Non-secret metadata.
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: self.id().as_str().to_string(),
            configured: self.is_configured(),
            search: self.supports_search(),
            extract: self.supports_extract(),
            estimated_search_usd: self.estimated_search_usd(),
            requires_key: self.requires_key(),
            notes: self.notes().to_string(),
        }
    }
}

/// All providers owned by the process.
pub struct Registry {
    providers: Vec<Arc<dyn Provider>>,
}

impl Registry {
    /// Construct every provider from config.
    pub fn new(config: &Config, http: HttpClient) -> Self {
        let mut providers: Vec<Arc<dyn Provider>> = vec![
            Arc::new(tavily::Tavily::new(config, http.clone())),
            Arc::new(exa::Exa::new(config, http.clone())),
            Arc::new(firecrawl::Firecrawl::new(config, http.clone())),
            Arc::new(linkup::Linkup::new(config, http.clone())),
            Arc::new(brave::Brave::new(config, http.clone())),
            Arc::new(kagi::Kagi::new(config, http.clone())),
            Arc::new(youcom::YouCom::new(config, http.clone())),
            Arc::new(parallel::Parallel::new(config, http.clone())),
            Arc::new(querit::Querit::new(config, http.clone())),
            Arc::new(tinyfish::TinyFish::new(config, http.clone())),
            Arc::new(keenable::Keenable::new(config, http.clone())),
            Arc::new(perplexity::Perplexity::new(config, http.clone())),
            Arc::new(github::Github::new(config, http.clone())),
            Arc::new(reddit::Reddit::new(config, http.clone())),
            Arc::new(xsearch::XSearch::new(config, http.clone())),
            Arc::new(youtube::Youtube::new(config, http.clone())),
            Arc::new(instagram::Instagram::new(config, http.clone())),
            Arc::new(facebook::Facebook::new(config, http.clone())),
            Arc::new(wikipedia::Wikipedia::new(config, http.clone())),
            Arc::new(scholar::Scholar::new(config, http.clone())),
            Arc::new(mastodon::Mastodon::new(config, http.clone())),
            Arc::new(bluesky::Bluesky::new(config, http.clone())),
        ];
        if !config.mcp_backends.is_empty() {
            providers.push(Arc::new(mcp_backend::McpBackends::new(config, http)));
        }
        Self { providers }
    }

    /// Borrow a provider by id.
    pub fn get(&self, id: ProviderId) -> Option<Arc<dyn Provider>> {
        self.providers.iter().find(|p| p.id() == id).cloned()
    }

    /// Configured search providers.
    pub fn configured_search(&self) -> Vec<Arc<dyn Provider>> {
        self.providers
            .iter()
            .filter(|p| p.is_configured() && p.supports_search())
            .cloned()
            .collect()
    }

    /// Configured extract providers.
    pub fn configured_extract(&self) -> Vec<Arc<dyn Provider>> {
        self.providers
            .iter()
            .filter(|p| p.is_configured() && p.supports_extract())
            .cloned()
            .collect()
    }

    /// Metadata for every provider.
    pub fn infos(&self) -> Vec<ProviderInfo> {
        self.providers.iter().map(|p| p.info()).collect()
    }
}

/// Map a JSON object onto a unified hit.
pub(crate) fn hit_from_value(
    id: ProviderId,
    value: &serde_json::Value,
    url_keys: &[&str],
    title_keys: &[&str],
    snippet_keys: &[&str],
    date_keys: &[&str],
    score_keys: &[&str],
) -> Option<crate::types::SearchHit> {
    let url = crate::http::pick_str(value, url_keys)?;
    let mut hit = crate::types::SearchHit::new(
        id,
        crate::http::pick_str(value, title_keys).unwrap_or_else(|| url.clone()),
        url,
        crate::http::pick_str(value, snippet_keys).unwrap_or_default(),
    );
    hit.published_at = crate::http::pick_str(value, date_keys);
    hit.score = crate::http::pick_f64(value, score_keys);
    Some(hit)
}

/// Preferred page size given a soft limit.
pub fn page_size_for(max: u32, request: &crate::types::SearchRequest) -> u32 {
    match request.limit {
        Some(limit) if !request.wants_unlimited() => max.min(limit.max(1)),
        _ => max,
    }
}

/// Freshness query token used by several web APIs.
pub fn freshness_token(f: Option<crate::types::Freshness>) -> Option<&'static str> {
    f.map(|v| match v {
        crate::types::Freshness::Day => "day",
        crate::types::Freshness::Week => "week",
        crate::types::Freshness::Month => "month",
        crate::types::Freshness::Year => "year",
    })
}

/// News vs web hint.
pub fn is_news(search_type: SearchType) -> bool {
    matches!(search_type, SearchType::News)
}
