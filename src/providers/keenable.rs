//! Keenable search + extract, with optional public tier.

use async_trait::async_trait;
use serde_json::json;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::http::{HttpClient, result_array};
use crate::types::{ExtractedDoc, ProviderId, ProviderSearchRequest, SearchPage};

use super::{Provider, hit_from_value};

pub struct Keenable {
    http: HttpClient,
    key: Option<String>,
    public: bool,
    title: String,
    base: String,
}

impl Keenable {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            key: config.keys.keenable.clone(),
            public: config.keenable_public,
            title: config.keenable_title.clone(),
            base: config.endpoints.keenable.clone(),
        }
    }
}

#[async_trait]
impl Provider for Keenable {
    fn id(&self) -> ProviderId {
        ProviderId::Keenable
    }
    fn is_configured(&self) -> bool {
        self.key.is_some() || self.public
    }
    fn skip_reason(&self) -> Option<String> {
        if self.is_configured() {
            None
        } else {
            Some("KEENABLE_API_KEY not set (or set KEENABLE_PUBLIC=true)".into())
        }
    }
    fn supports_extract(&self) -> bool {
        self.key.is_some()
    }
    fn estimated_search_usd(&self) -> f64 {
        if self.key.is_some() { 0.003 } else { 0.0 }
    }
    fn max_page_size(&self) -> u32 {
        50
    }
    fn notes(&self) -> &'static str {
        "POST /v1/search with X-API-Key. Public tier: KEENABLE_PUBLIC=true hits /v1/search/public."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        if !self.is_configured() {
            return Err(Error::NotConfigured {
                provider: "keenable".into(),
                reason: "KEENABLE_API_KEY not set".into(),
            });
        }
        let mut body = json!({
            "query": request.query,
            "max_results": request.page_size.min(50),
        });
        if let Some(fresh) = request.freshness {
            body["published_after"] = json!(fresh.since_rfc3339());
        }
        let builder = if let Some(key) = &self.key {
            self.http
                .post(&format!("{}/v1/search", self.base))
                .header("X-API-Key", key)
                .json(&body)
        } else {
            self.http
                .post(&format!("{}/v1/search/public", self.base))
                .header("X-Keenable-Title", &self.title)
                .json(&body)
        };
        let (_, value) = self.http.send_json("keenable", builder).await?;
        let hits = result_array(&value)
            .iter()
            .filter_map(|v| {
                hit_from_value(
                    ProviderId::Keenable,
                    v,
                    &["url"],
                    &["title"],
                    &["snippet", "description", "excerpt"],
                    &["published_at"],
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

    async fn extract(&self, urls: &[String]) -> Result<Vec<ExtractedDoc>> {
        let key = self.key.as_deref().ok_or_else(|| Error::NotConfigured {
            provider: "keenable".into(),
            reason: "extract requires KEENABLE_API_KEY".into(),
        })?;
        let (_, value) = self
            .http
            .send_json(
                "keenable",
                self.http
                    .post(&format!("{}/v1/extract", self.base))
                    .header("X-API-Key", key)
                    .json(&json!({ "urls": urls })),
            )
            .await?;
        Ok(super::tavily::extract_docs(ProviderId::Keenable, &value))
    }
}
