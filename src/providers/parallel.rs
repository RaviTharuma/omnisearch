//! Parallel.ai search + extract.

use async_trait::async_trait;
use serde_json::json;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, result_array};
use crate::types::{ExtractedDoc, ProviderId, ProviderSearchRequest, SearchPage};

use super::{Keyed, Provider, extract_docs, hit_from_value, page};

pub struct Parallel {
    inner: Keyed,
}

impl Parallel {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            inner: Keyed::new(
                http,
                config.keys.parallel.clone(),
                &config.endpoints.parallel,
            ),
        }
    }
}

#[async_trait]
impl Provider for Parallel {
    fn id(&self) -> ProviderId {
        ProviderId::Parallel
    }
    fn is_configured(&self) -> bool {
        self.inner.configured()
    }
    fn skip_reason(&self) -> Option<String> {
        self.inner.skip("PARALLEL_API_KEY")
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
        crate::try_keys!(
            &self.inner.keys,
            "parallel",
            "PARALLEL_API_KEY not set",
            |key| self.search_with(key, request).await,
        )
    }

    async fn extract(&self, urls: &[String], _account: Option<&str>) -> Result<Vec<ExtractedDoc>> {
        crate::try_keys!(
            &self.inner.keys,
            "parallel",
            "PARALLEL_API_KEY not set",
            |key| self.extract_with(key, urls).await,
        )
    }
}

impl Parallel {
    async fn search_with(
        &self,
        key: &str,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
        let value = self
            .inner
            .http
            .json(
                "parallel",
                self.inner
                    .http
                    .post(&self.inner.url("/v1/search"))
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
        Ok(page(hits))
    }

    async fn extract_with(&self, key: &str, urls: &[String]) -> Result<Vec<ExtractedDoc>> {
        let value = self
            .inner
            .http
            .json(
                "parallel",
                self.inner
                    .http
                    .post(&self.inner.url("/v1/extract"))
                    .header("x-api-key", key)
                    .json(&json!({ "urls": urls })),
            )
            .await?;
        Ok(extract_docs(ProviderId::Parallel, &value))
    }
}
