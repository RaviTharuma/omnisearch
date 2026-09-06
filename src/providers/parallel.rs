//! Parallel.ai search + extract.

use async_trait::async_trait;
use serde_json::json;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::http::{HttpClient, result_array};
use crate::types::{ExtractedDoc, ProviderId, ProviderSearchRequest, SearchPage};

use super::{Provider, hit_from_value};

pub struct Parallel {
    http: HttpClient,
    key: Option<String>,
    base: String,
}

impl Parallel {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            key: config.keys.parallel.clone(),
            base: config.endpoints.parallel.clone(),
        }
    }
}

#[async_trait]
impl Provider for Parallel {
    fn id(&self) -> ProviderId {
        ProviderId::Parallel
    }
    fn is_configured(&self) -> bool {
        self.key.is_some()
    }
    fn skip_reason(&self) -> Option<String> {
        self.key
            .is_none()
            .then(|| "PARALLEL_API_KEY not set".into())
    }
    fn supports_extract(&self) -> bool {
        true
    }
    fn estimated_search_usd(&self) -> f64 {
        0.01
    }
    fn notes(&self) -> &'static str {
        "POST /v1/search and /v1/extract. x-api-key header."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        let key = self.key.as_deref().ok_or_else(|| Error::NotConfigured {
            provider: "parallel".into(),
            reason: "PARALLEL_API_KEY not set".into(),
        })?;
        let (_, value) = self
            .http
            .send_json(
                "parallel",
                self.http
                    .post(&format!("{}/v1/search", self.base))
                    .header("x-api-key", key)
                    .json(&json!({
                        "objective": request.query,
                        "search_queries": [request.query],
                        "max_results": request.page_size,
                    })),
            )
            .await?;
        let hits = result_array(&value)
            .iter()
            .filter_map(|v| {
                let excerpts = v
                    .get("excerpts")
                    .and_then(|e| e.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();
                let mut hit = hit_from_value(
                    ProviderId::Parallel,
                    v,
                    &["url"],
                    &["title"],
                    &["snippet"],
                    &["publish_date", "published_at"],
                    &[],
                )?;
                if hit.snippet.is_empty() {
                    hit.snippet = excerpts;
                }
                Some(hit)
            })
            .collect();
        Ok(SearchPage {
            hits,
            next_cursor: None,
            answer: None,
        })
    }

    async fn extract(&self, urls: &[String]) -> Result<Vec<ExtractedDoc>> {
        let key = self.key.as_deref().ok_or_else(|| Error::NotConfigured {
            provider: "parallel".into(),
            reason: "PARALLEL_API_KEY not set".into(),
        })?;
        let (_, value) = self
            .http
            .send_json(
                "parallel",
                self.http
                    .post(&format!("{}/v1/extract", self.base))
                    .header("x-api-key", key)
                    .json(&json!({ "urls": urls })),
            )
            .await?;
        Ok(super::tavily::extract_docs(ProviderId::Parallel, &value))
    }
}
