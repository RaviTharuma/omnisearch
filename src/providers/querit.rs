//! Querit multilingual search + contents.

use async_trait::async_trait;
use serde_json::json;

use crate::config::Config;
use crate::error::Result;
use crate::http::HttpClient;
use crate::types::{ExtractedDoc, ProviderId, ProviderSearchRequest, SearchPage};

use super::{Keyed, Provider, extract_docs, freshness_token, map_hits, map_rows, page};

pub struct Querit {
    inner: Keyed,
}

impl Querit {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            inner: Keyed::new(http, config.keys.querit.clone(), &config.endpoints.querit),
        }
    }
}

#[async_trait]
impl Provider for Querit {
    fn id(&self) -> ProviderId {
        ProviderId::Querit
    }
    fn is_configured(&self) -> bool {
        self.inner.configured()
    }
    fn skip_reason(&self) -> Option<String> {
        self.inner.skip("QUERIT_API_KEY")
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
        crate::try_keys!(
            &self.inner.keys,
            "querit",
            "QUERIT_API_KEY not set",
            |key| { self.search_with(key, request).await }
        )
    }

    async fn extract(
        &self,
        urls: &[String],
        _account: Option<&str>,
    ) -> Result<Vec<ExtractedDoc>> {
        crate::try_keys!(
            &self.inner.keys,
            "querit",
            "QUERIT_API_KEY not set",
            |key| { self.extract_with(key, urls).await }
        )
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
        let value = self
            .inner
            .http
            .json(
                "querit",
                self.inner
                    .http
                    .post(&self.inner.url("/v1/search"))
                    .bearer_auth(key)
                    .json(&body),
            )
            .await?;
        let hits = value
            .pointer("/results/result")
            .and_then(|v| v.as_array())
            .map(|rows| {
                map_rows(
                    ProviderId::Querit,
                    rows,
                    &["url"],
                    &["title"],
                    &["snippet", "content"],
                    &["page_age", "published_at"],
                    &[],
                )
            })
            .unwrap_or_else(|| {
                map_hits(
                    ProviderId::Querit,
                    &value,
                    &["url"],
                    &["title"],
                    &["snippet", "content"],
                    &["page_age", "published_at"],
                    &[],
                )
            });
        Ok(page(hits))
    }

    async fn extract_with(&self, key: &str, urls: &[String]) -> Result<Vec<ExtractedDoc>> {
        let value = self
            .inner
            .http
            .json(
                "querit",
                self.inner
                    .http
                    .post(&self.inner.url("/v1/contents"))
                    .bearer_auth(key)
                    .json(&json!({ "urls": urls, "format": "markdown" })),
            )
            .await?;
        Ok(extract_docs(ProviderId::Querit, &value))
    }
}
