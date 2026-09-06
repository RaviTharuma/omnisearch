//! Perplexity Search API.

use async_trait::async_trait;
use serde_json::json;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, pick_str, result_array};
use crate::types::{ProviderId, ProviderSearchRequest, SearchPage};

use super::{Provider, hit_from_value};

pub struct Perplexity {
    http: HttpClient,
    keys: Vec<String>,
    base: String,
}

impl Perplexity {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            keys: config.keys.perplexity.clone(),
            base: config.endpoints.perplexity.clone(),
        }
    }
}

#[async_trait]
impl Provider for Perplexity {
    fn id(&self) -> ProviderId {
        ProviderId::Perplexity
    }
    fn is_configured(&self) -> bool {
        !self.keys.is_empty()
    }
    fn skip_reason(&self) -> Option<String> {
        self.keys
            .is_empty()
            .then(|| "PERPLEXITY_API_KEY not set".into())
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
            &self.keys,
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
        let body = json!({
            "query": request.query,
            "max_results": request.page_size.min(20),
        });
        let (_, value) = self
            .http
            .send_json(
                "perplexity",
                self.http
                    .post(&format!("{}/search", self.base))
                    .bearer_auth(key)
                    .json(&body),
            )
            .await?;
        let hits = result_array(&value)
            .iter()
            .filter_map(|v| {
                hit_from_value(
                    ProviderId::Perplexity,
                    v,
                    &["url", "link"],
                    &["title"],
                    &["snippet", "text", "content"],
                    &["date", "published_at"],
                    &["score"],
                )
            })
            .collect();
        Ok(SearchPage {
            hits,
            next_cursor: None,
            answer: pick_str(&value, &["answer"]),
        })
    }
}
