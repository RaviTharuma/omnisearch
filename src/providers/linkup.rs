//! Linkup agentic search.

use async_trait::async_trait;
use serde_json::json;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, result_array};
use crate::types::{ProviderId, ProviderSearchRequest, SearchPage};

use super::{Provider, hit_from_value};

pub struct Linkup {
    http: HttpClient,
    keys: Vec<String>,
    base: String,
}

impl Linkup {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            keys: config.keys.linkup.clone(),
            base: config.endpoints.linkup.clone(),
        }
    }
}

#[async_trait]
impl Provider for Linkup {
    fn id(&self) -> ProviderId {
        ProviderId::Linkup
    }
    fn is_configured(&self) -> bool {
        !self.keys.is_empty()
    }
    fn skip_reason(&self) -> Option<String> {
        self.keys
            .is_empty()
            .then(|| "LINKUP_API_KEY not set".into())
    }
    fn estimated_search_usd(&self) -> f64 {
        0.007
    }
    fn notes(&self) -> &'static str {
        "POST /v1/search with outputType=searchResults."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        crate::try_keys!(&self.keys, "linkup", "LINKUP_API_KEY not set", |key| self
            .search_with(key, request)
            .await,)
    }
}

impl Linkup {
    async fn search_with(
        &self,
        key: &str,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
        let (_, value) = self
            .http
            .send_json(
                "linkup",
                self.http
                    .post(&format!("{}/v1/search", self.base))
                    .bearer_auth(key)
                    .json(&json!({
                        "q": request.query,
                        "depth": "standard",
                        "outputType": "searchResults",
                    })),
            )
            .await?;
        let hits = result_array(&value)
            .iter()
            .filter_map(|v| {
                hit_from_value(
                    ProviderId::Linkup,
                    v,
                    &["url"],
                    &["name", "title"],
                    &["content", "snippet"],
                    &[],
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
