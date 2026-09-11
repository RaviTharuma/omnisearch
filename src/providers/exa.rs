//! Exa neural / keyword search and contents extract.

use async_trait::async_trait;
use serde_json::json;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, result_array};
use crate::types::{ExtractedDoc, ProviderId, ProviderSearchRequest, SearchPage, SearchType};

use super::{Keyed, Provider, extract_docs, hit_from_value, page};

pub struct Exa {
    inner: Keyed,
}

impl Exa {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            inner: Keyed::new(http, config.keys.exa.clone(), &config.endpoints.exa),
        }
    }
}

#[async_trait]
impl Provider for Exa {
    fn id(&self) -> ProviderId {
        ProviderId::Exa
    }
    fn is_configured(&self) -> bool {
        self.inner.configured()
    }
    fn skip_reason(&self) -> Option<String> {
        self.inner.skip("EXA_API_KEY")
    }
    fn supports_extract(&self) -> bool {
        true
    }
    fn estimated_search_usd(&self) -> f64 {
        0.006
    }
    fn max_page_size(&self) -> u32 {
        100
    }
    fn notes(&self) -> &'static str {
        "Neural / keyword search plus /contents extract. Dual-key: EXA_API_KEY_2."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        crate::try_keys!(&self.inner.keys, "exa", "EXA_API_KEY not set", |key| {
            self.search_with(key, request).await
        })
    }

    async fn extract(
        &self,
        urls: &[String],
        _account: Option<&str>,
    ) -> Result<Vec<ExtractedDoc>> {
        crate::try_keys!(&self.inner.keys, "exa", "EXA_API_KEY not set", |key| {
            self.extract_with(key, urls).await
        })
    }
}

impl Exa {
    async fn search_with(
        &self,
        key: &str,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
        let kind = if matches!(request.search_type, SearchType::News) {
            "neural"
        } else {
            "auto"
        };
        let mut body = json!({
            "query": request.query,
            "numResults": request.page_size.min(100),
            "type": kind,
            "contents": { "text": true },
        });
        if let Some(fresh) = request.freshness {
            body["startPublishedDate"] = json!(fresh.since_rfc3339());
        }
        let value = self
            .inner
            .http
            .json(
                "exa",
                self.inner
                    .http
                    .post(&self.inner.url("/search"))
                    .header("x-api-key", key)
                    .json(&body),
            )
            .await?;
        let hits = result_array(&value)
            .iter()
            .filter_map(|v| {
                let text = v
                    .pointer("/contents/text")
                    .and_then(|x| x.as_str())
                    .or_else(|| v.get("text").and_then(|x| x.as_str()))
                    .unwrap_or("");
                let mut hit = hit_from_value(
                    ProviderId::Exa,
                    v,
                    &["url", "id"],
                    &["title"],
                    &["text", "snippet"],
                    &["publishedDate", "published_at"],
                    &["score"],
                )?;
                if hit.snippet.is_empty() {
                    hit.snippet = text.chars().take(400).collect();
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
                "exa",
                self.inner
                    .http
                    .post(&self.inner.url("/contents"))
                    .header("x-api-key", key)
                    .json(&json!({ "urls": urls, "text": true })),
            )
            .await?;
        Ok(extract_docs(ProviderId::Exa, &value))
    }
}
