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
pub mod omniroute;
pub mod parallel;
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
use crate::http::{HttpClient, pick_str, result_array};
use crate::types::{
    ExtractedDoc, ProviderId, ProviderInfo, ProviderSearchRequest, SearchHit, SearchPage,
    SearchType,
};
use serde_json::Value;

#[async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> ProviderId;
    fn is_configured(&self) -> bool;
    fn skip_reason(&self) -> Option<String> {
        None
    }
    fn supports_search(&self) -> bool {
        true
    }
    fn supports_extract(&self) -> bool {
        false
    }
    fn estimated_search_usd(&self) -> f64 {
        0.005
    }
    fn max_page_size(&self) -> u32 {
        20
    }
    fn notes(&self) -> &'static str {
        ""
    }
    fn requires_key(&self) -> bool {
        true
    }
    /// Whether this provider pool exposes the named account (for pin-by-account).
    fn known_account(&self, _name: &str) -> bool {
        false
    }
    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage>;
    async fn extract(
        &self,
        urls: &[String],
        _account: Option<&str>,
    ) -> Result<Vec<ExtractedDoc>> {
        let _ = urls;
        Ok(Vec::new())
    }

    /// Firecrawl-style site crawl. Default: unsupported.
    async fn crawl(
        &self,
        _url: &str,
        _limit: u32,
        _timeout_secs: u64,
        _account: Option<&str>,
    ) -> Result<serde_json::Value> {
        Err(crate::error::Error::Invalid(format!(
            "{} does not support crawl",
            self.id()
        )))
    }

    /// Firecrawl-style site map. Default: unsupported.
    async fn map_urls(
        &self,
        _url: &str,
        _search: Option<&str>,
        _limit: Option<u32>,
        _account: Option<&str>,
    ) -> Result<serde_json::Value> {
        Err(crate::error::Error::Invalid(format!(
            "{} does not support map",
            self.id()
        )))
    }

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

pub struct Registry {
    providers: Vec<Arc<dyn Provider>>,
    pub account_health: Arc<crate::accounts::AccountHealthBoard>,
}

impl Registry {
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
            providers.push(Arc::new(mcp_backend::McpBackends::new(
                config,
                http.clone(),
            )));
        }
        let wrapped = crate::accounts::wrap_providers(providers, config, http)
            .expect("account configuration must be validated before constructing Registry");
        Self {
            providers: wrapped.providers,
            account_health: wrapped.health,
        }
    }

    pub fn get(&self, id: ProviderId) -> Option<Arc<dyn Provider>> {
        self.providers.iter().find(|p| p.id() == id).cloned()
    }

    pub fn configured_search(&self) -> Vec<Arc<dyn Provider>> {
        self.providers
            .iter()
            .filter(|p| p.is_configured() && p.supports_search())
            .cloned()
            .collect()
    }

    pub fn configured_extract(&self) -> Vec<Arc<dyn Provider>> {
        self.providers
            .iter()
            .filter(|p| p.is_configured() && p.supports_extract())
            .cloned()
            .collect()
    }

    pub fn infos(&self) -> Vec<ProviderInfo> {
        self.providers.iter().map(|p| p.info()).collect()
    }
}

/// HTTP + key ring + overridable base. Shared by keyed vendor adapters.
pub(crate) struct Keyed {
    pub http: HttpClient,
    pub keys: Vec<String>,
    pub base: String,
}

impl Keyed {
    pub fn new(http: HttpClient, keys: Vec<String>, base: impl Into<String>) -> Self {
        Self {
            http,
            keys,
            base: base.into(),
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base.trim_end_matches('/'))
    }

    pub fn configured(&self) -> bool {
        !self.keys.is_empty()
    }

    pub fn skip(&self, env: &str) -> Option<String> {
        self.keys.is_empty().then(|| format!("{env} not set"))
    }
}

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

pub(crate) fn map_hits(
    id: ProviderId,
    value: &Value,
    url_keys: &[&str],
    title_keys: &[&str],
    snippet_keys: &[&str],
    date_keys: &[&str],
    score_keys: &[&str],
) -> Vec<SearchHit> {
    map_rows(
        id,
        &result_array(value),
        url_keys,
        title_keys,
        snippet_keys,
        date_keys,
        score_keys,
    )
}

pub(crate) fn map_rows(
    id: ProviderId,
    rows: &[Value],
    url_keys: &[&str],
    title_keys: &[&str],
    snippet_keys: &[&str],
    date_keys: &[&str],
    score_keys: &[&str],
) -> Vec<SearchHit> {
    rows.iter()
        .filter_map(|v| {
            hit_from_value(
                id,
                v,
                url_keys,
                title_keys,
                snippet_keys,
                date_keys,
                score_keys,
            )
        })
        .collect()
}

pub(crate) fn page(hits: Vec<SearchHit>) -> SearchPage {
    SearchPage {
        hits,
        next_cursor: None,
        answer: None,
    }
}

pub(crate) fn page_answer(hits: Vec<SearchHit>, answer: Option<String>) -> SearchPage {
    SearchPage {
        hits,
        next_cursor: None,
        answer,
    }
}

pub(crate) fn page_next(hits: Vec<SearchHit>, page_size: u32, next: impl ToString) -> SearchPage {
    let next_cursor = (hits.len() as u32 >= page_size).then(|| next.to_string());
    SearchPage {
        hits,
        next_cursor,
        answer: None,
    }
}

pub(crate) fn offset(cursor: Option<&str>) -> u32 {
    cursor.and_then(|value| value.parse().ok()).unwrap_or(0)
}

pub(crate) fn page_num(cursor: Option<&str>) -> u32 {
    cursor
        .and_then(|value| value.parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(1)
}

pub(crate) fn extract_docs(id: ProviderId, value: &Value) -> Vec<ExtractedDoc> {
    result_array(value)
        .into_iter()
        .filter_map(|v| {
            let url = pick_str(&v, &["url"])?;
            Some(ExtractedDoc {
                url,
                title: pick_str(&v, &["title"]),
                content: pick_str(&v, &["raw_content", "content", "text", "markdown"])
                    .unwrap_or_default(),
                provider: id.as_str().to_string(),
            })
        })
        .collect()
}

pub fn page_size_for(max: u32, request: &crate::types::SearchRequest) -> u32 {
    match request.limit {
        Some(limit) if !request.wants_unlimited() => max.min(limit.max(1)),
        _ => max,
    }
}

pub fn freshness_token(f: Option<crate::types::Freshness>) -> Option<&'static str> {
    f.map(|v| match v {
        crate::types::Freshness::Day => "day",
        crate::types::Freshness::Week => "week",
        crate::types::Freshness::Month => "month",
        crate::types::Freshness::Year => "year",
    })
}

pub fn is_news(search_type: SearchType) -> bool {
    matches!(search_type, SearchType::News)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn map_hits_and_extract_docs() {
        let value = json!({
            "results": [{
                "url": "https://a.test",
                "title": "A",
                "content": "hello",
                "score": 0.4
            }]
        });
        let hits = map_hits(
            ProviderId::Tavily,
            &value,
            &["url"],
            &["title"],
            &["content"],
            &[],
            &["score"],
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].score, Some(0.4));
        let docs = extract_docs(ProviderId::Tavily, &value);
        assert_eq!(docs[0].content, "hello");
    }

    #[test]
    fn offset_and_page_cursors() {
        assert_eq!(offset(None), 0);
        assert_eq!(offset(Some("20")), 20);
        assert_eq!(page_num(None), 1);
        assert_eq!(page_num(Some("0")), 1);
        assert_eq!(page_num(Some("3")), 3);
        let p = page_next(
            vec![SearchHit::new(ProviderId::Brave, "t", "https://a", "")],
            1,
            "2",
        );
        assert_eq!(p.next_cursor.as_deref(), Some("2"));
    }

    #[test]
    fn page_without_next_when_short() {
        let p = page_next(vec![], 5, "2");
        assert!(p.next_cursor.is_none());
    }
}
