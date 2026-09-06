//! TinyFish source-only search.

use async_trait::async_trait;
use serde_json::json;

use crate::config::Config;
use crate::error::Result;
use crate::http::HttpClient;
use crate::types::{ProviderId, ProviderSearchRequest, SearchPage};

use super::{Keyed, Provider, map_hits, page};

pub struct TinyFish {
    inner: Keyed,
}

impl TinyFish {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            inner: Keyed::new(
                http,
                config.keys.tinyfish.clone(),
                &config.endpoints.tinyfish,
            ),
        }
    }
}

#[async_trait]
impl Provider for TinyFish {
    fn id(&self) -> ProviderId {
        ProviderId::Tinyfish
    }
    fn is_configured(&self) -> bool {
        self.inner.configured()
    }
    fn skip_reason(&self) -> Option<String> {
        self.inner.skip("TINYFISH_API_KEY")
    }
    fn estimated_search_usd(&self) -> f64 {
        0.006
    }
    fn notes(&self) -> &'static str {
        "Source-only: returns URLs/snippets, no extract. POST /v1/search."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        crate::try_keys!(
            &self.inner.keys,
            "tinyfish",
            "TINYFISH_API_KEY not set",
            |key| self.search_with(key, request).await,
        )
    }
}

impl TinyFish {
    async fn search_with(
        &self,
        key: &str,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
        let value = self
            .inner
            .http
            .json(
                "tinyfish",
                self.inner
                    .http
                    .post(&self.inner.url("/v1/search"))
                    .bearer_auth(key)
                    .json(&json!({
                        "query": request.query,
                        "limit": request.page_size
                    })),
            )
            .await?;
        Ok(page(map_hits(
            ProviderId::Tinyfish,
            &value,
            &["url", "source", "link"],
            &["title", "name"],
            &["snippet", "description"],
            &["published_at", "date"],
            &[],
        )))
    }
}
