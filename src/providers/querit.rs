//! Querit multilingual search + contents.

use async_trait::async_trait;
use serde_json::json;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, result_array};
use crate::types::{ExtractedDoc, ProviderId, ProviderSearchRequest, SearchPage};

use super::{Provider, freshness_token, hit_from_value};

pub struct Querit {
    http: HttpClient,
    keys: Vec<String>,
    base: String,
}

impl Querit {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            keys: config.keys.querit.clone(),
            base: config.endpoints.querit.clone(),
        }
    }
}

#[async_trait]
impl Provider for Querit {
    fn id(&self) -> ProviderId {
        ProviderId::Querit
    }
    fn is_configured(&self) -> bool {
        !self.keys.is_empty()
    }
    fn skip_reason(&self) -> Option<String> {
        self.keys
            .is_empty()
            .then(|| "QUERIT_API_KEY not set".into())
    }
    fn supports_extract(&self) -> bool {
        true
    }
    fn estimated_search_usd(&self) -> f64 {
        0.004
    }
    fn notes(&self) -> &'static str {
        "Multilingual search. POST /v1/search and /v1/contents."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        crate::try_keys!(&self.keys, "querit", "QUERIT_API_KEY not set", |key| {
            self.search_with(key, request).await
        })
    }

    async fn extract(&self, urls: &[String]) -> Result<Vec<ExtractedDoc>> {
        crate::try_keys!(&self.keys, "querit", "QUERIT_API_KEY not set", |key| {
            self.extract_with(key, urls).await
        })
    }
}

impl Querit {
    async fn search_with(
        &self,
        key: &str,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
        let mut body = json!({
            "query": request.query,
            "count": request.page_size,
            "filters": {
                "countries": [request.country],
                "languages": [request.language],
            }
        });
        if let Some(token) = freshness_token(request.freshness) {
            body["filters"]["timeRange"] = json!({ "date": token });
        }
        let (_, value) = self
            .http
            .send_json(
                "querit",
                self.http
                    .post(&format!("{}/v1/search", self.base))
                    .bearer_auth(key)
                    .json(&body),
            )
            .await?;
        let rows = value
            .pointer("/results/result")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_else(|| result_array(&value));
        let hits = rows
            .iter()
            .filter_map(|v| {
                hit_from_value(
                    ProviderId::Querit,
                    v,
                    &["url"],
                    &["title"],
                    &["snippet", "content"],
                    &["page_age", "published_at"],
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

    async fn extract_with(&self, key: &str, urls: &[String]) -> Result<Vec<ExtractedDoc>> {
        let (_, value) = self
            .http
            .send_json(
                "querit",
                self.http
                    .post(&format!("{}/v1/contents", self.base))
                    .bearer_auth(key)
                    .json(&json!({ "urls": urls, "format": "markdown" })),
            )
            .await?;
        Ok(super::tavily::extract_docs(ProviderId::Querit, &value))
    }
}
