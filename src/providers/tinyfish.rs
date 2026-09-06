//! TinyFish source-only search.

use async_trait::async_trait;
use serde_json::json;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, result_array};
use crate::types::{ProviderId, ProviderSearchRequest, SearchPage};

use super::{Provider, hit_from_value};

pub struct TinyFish {
    http: HttpClient,
    keys: Vec<String>,
    base: String,
}

impl TinyFish {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            keys: config.keys.tinyfish.clone(),
            base: config.endpoints.tinyfish.clone(),
        }
    }
}

#[async_trait]
impl Provider for TinyFish {
    fn id(&self) -> ProviderId {
        ProviderId::Tinyfish
    }
    fn is_configured(&self) -> bool {
        !self.keys.is_empty()
    }
    fn skip_reason(&self) -> Option<String> {
        self.keys
            .is_empty()
            .then(|| "TINYFISH_API_KEY not set".into())
    }
    fn supports_extract(&self) -> bool {
        false
    }
    fn estimated_search_usd(&self) -> f64 {
        0.006
    }
    fn notes(&self) -> &'static str {
        "Source-only: returns URLs/snippets, no extract. POST /v1/search."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        crate::try_keys!(&self.keys, "tinyfish", "TINYFISH_API_KEY not set", |key| {
            self.search_with(key, request).await
        },)
    }
}

impl TinyFish {
    async fn search_with(
        &self,
        key: &str,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
        let (_, value) = self
            .http
            .send_json(
                "tinyfish",
                self.http
                    .post(&format!("{}/v1/search", self.base))
                    .bearer_auth(key)
                    .json(&json!({
                        "query": request.query,
                        "limit": request.page_size
                    })),
            )
            .await?;
        let hits = result_array(&value)
            .iter()
            .filter_map(|v| {
                hit_from_value(
                    ProviderId::Tinyfish,
                    v,
                    &["url", "source", "link"],
                    &["title", "name"],
                    &["snippet", "description"],
                    &["published_at", "date"],
                    &[],
                )
            })
            .collect();
        Ok(SearchPage {
            hits,
            next_cursor: None,
            answer: None,
        })
    }
}
