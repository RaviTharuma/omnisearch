//! Perplexity Search API.

use async_trait::async_trait;
use serde_json::json;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, pick_str};
use crate::types::{ProviderId, ProviderSearchRequest, SearchPage};

use super::{Keyed, Provider, map_hits, page_answer};

pub struct Perplexity {
    inner: Keyed,
}

impl Perplexity {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            inner: Keyed::new(
                http,
                config.keys.perplexity.clone(),
                &config.endpoints.perplexity,
            ),
        }
    }
}

#[async_trait]
impl Provider for Perplexity {
    fn id(&self) -> ProviderId {
        ProviderId::Perplexity
    }
    fn is_configured(&self) -> bool {
        self.inner.configured()
    }
    fn skip_reason(&self) -> Option<String> {
        self.inner.skip("PERPLEXITY_API_KEY")
    }
    fn estimated_search_usd(&self) -> f64 {
        0.005
    }
    fn max_page_size(&self) -> u32 {
        20
    }
    fn notes(&self) -> &'static str {
        "POST /search. Optional official MCP remote: https://mcp.perplexity.ai/mcp"
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        crate::try_keys!(
            &self.inner.keys,
            "perplexity",
            "PERPLEXITY_API_KEY not set",
            |key| self.search_with(key, request).await,
        )
    }
}

impl Perplexity {
    async fn search_with(
        &self,
        key: &str,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
        let value = self
            .inner
            .http
            .json(
                "perplexity",
                self.inner
                    .http
                    .post(&self.inner.url("/search"))
                    .bearer_auth(key)
                    .json(&json!({
                        "query": request.query,
                        "max_results": request.page_size.min(20),
                    })),
            )
            .await?;
        Ok(page_answer(
            map_hits(
                ProviderId::Perplexity,
                &value,
                &["url", "link"],
                &["title"],
                &["snippet", "text", "content"],
                &["date", "published_at"],
                &["score"],
            ),
            pick_str(&value, &["answer"]),
        ))
    }
}
