//! Keenable search + extract, with optional public tier.

use async_trait::async_trait;
use serde_json::json;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::http::HttpClient;
use crate::types::{ExtractedDoc, ProviderId, ProviderSearchRequest, SearchPage};

use super::{Keyed, Provider, extract_docs, map_hits, page};

pub struct Keenable {
    inner: Keyed,
    public: bool,
    title: String,
}

impl Keenable {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            inner: Keyed::new(
                http,
                config.keys.keenable.clone(),
                &config.endpoints.keenable,
            ),
            public: config.keenable_public,
            title: config.keenable_title.clone(),
        }
    }
}

#[async_trait]
impl Provider for Keenable {
    fn id(&self) -> ProviderId {
        ProviderId::Keenable
    }
    fn is_configured(&self) -> bool {
        self.inner.configured() || self.public
    }
    fn skip_reason(&self) -> Option<String> {
        if self.is_configured() {
            None
        } else {
            Some("KEENABLE_API_KEY not set (or set KEENABLE_PUBLIC=true)".into())
        }
    }
    fn supports_extract(&self) -> bool {
        self.inner.configured()
    }
    fn estimated_search_usd(&self) -> f64 {
        if self.inner.keys.is_empty() {
            0.0
        } else {
            0.003
        }
    }
    fn requires_key(&self) -> bool {
        !self.public
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
        let value = if self.inner.keys.is_empty() {
            self.inner
                .http
                .json(
                    "keenable",
                    self.inner
                        .http
                        .post(&self.inner.url("/v1/search/public"))
                        .header("X-Keenable-Title", &self.title)
                        .json(&body),
                )
                .await?
        } else {
            crate::try_keys!(
                &self.inner.keys,
                "keenable",
                "KEENABLE_API_KEY not set",
                |key| {
                    self.inner
                        .http
                        .json(
                            "keenable",
                            self.inner
                                .http
                                .post(&self.inner.url("/v1/search"))
                                .header("X-API-Key", key)
                                .json(&body),
                        )
                        .await
                }
            )?
        };
        Ok(page(map_hits(
            ProviderId::Keenable,
            &value,
            &["url"],
            &["title"],
            &["snippet", "description", "excerpt"],
            &["published_at"],
            &[],
        )))
    }

    async fn extract(&self, urls: &[String], _account: Option<&str>) -> Result<Vec<ExtractedDoc>> {
        crate::try_keys!(
            &self.inner.keys,
            "keenable",
            "extract requires KEENABLE_API_KEY",
            |key| {
                let value = self
                    .inner
                    .http
                    .json(
                        "keenable",
                        self.inner
                            .http
                            .post(&self.inner.url("/v1/extract"))
                            .header("X-API-Key", key)
                            .json(&json!({ "urls": urls })),
                    )
                    .await?;
                Ok(extract_docs(ProviderId::Keenable, &value))
            }
        )
    }
}
