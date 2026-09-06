//! Linkup agentic search.

use async_trait::async_trait;
use serde_json::json;

use crate::config::Config;
use crate::error::Result;
use crate::http::HttpClient;
use crate::types::{ProviderId, ProviderSearchRequest, SearchPage};

use super::{Keyed, Provider, map_hits, page};

pub struct Linkup {
    inner: Keyed,
}

impl Linkup {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            inner: Keyed::new(http, config.keys.linkup.clone(), &config.endpoints.linkup),
        }
    }
}

#[async_trait]
impl Provider for Linkup {
    fn id(&self) -> ProviderId {
        ProviderId::Linkup
    }
    fn is_configured(&self) -> bool {
        self.inner.configured()
    }
    fn skip_reason(&self) -> Option<String> {
        self.inner.skip("LINKUP_API_KEY")
    }
    fn estimated_search_usd(&self) -> f64 {
        0.007
    }
    fn notes(&self) -> &'static str {
        "POST /v1/search with outputType=searchResults."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        crate::try_keys!(
            &self.inner.keys,
            "linkup",
            "LINKUP_API_KEY not set",
            |key| { self.search_with(key, request).await }
        )
    }
}

impl Linkup {
    async fn search_with(
        &self,
        key: &str,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
        let value = self
            .inner
            .http
            .json(
                "linkup",
                self.inner
                    .http
                    .post(&self.inner.url("/v1/search"))
                    .bearer_auth(key)
                    .json(&json!({
                        "q": request.query,
                        "depth": "standard",
                        "outputType": "searchResults",
                    })),
            )
            .await?;
        Ok(page(map_hits(
            ProviderId::Linkup,
            &value,
            &["url"],
            &["name", "title"],
            &["content", "snippet"],
            &[],
            &[],
        )))
    }
}
