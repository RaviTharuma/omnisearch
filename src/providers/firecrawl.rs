//! Firecrawl search, scrape, crawl, and map.

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::config::Config;
use crate::error::{Error, Result};
use crate::http::{HttpClient, pick_str};
use crate::types::{ExtractedDoc, ProviderId, ProviderSearchRequest, SearchPage};

use super::{Keyed, Provider, map_hits, map_rows, page};

pub struct Firecrawl {
    inner: Keyed,
}

impl Firecrawl {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            inner: Keyed::new(
                http,
                config.keys.firecrawl.clone(),
                &config.endpoints.firecrawl,
            ),
        }
    }

    fn keys(&self) -> &[String] {
        &self.inner.keys
    }

    pub async fn scrape(&self, url: &str) -> Result<ExtractedDoc> {
        crate::try_keys!(
            self.keys(),
            "firecrawl",
            "FIRECRAWL_API_KEY not set",
            |key| self.scrape_with(key, url).await,
        )
    }

    async fn scrape_with(&self, key: &str, url: &str) -> Result<ExtractedDoc> {
        let value = self
            .inner
            .http
            .json(
                "firecrawl",
                self.inner
                    .http
                    .post(&self.inner.url("/v2/scrape"))
                    .bearer_auth(key)
                    .json(&json!({ "url": url, "formats": ["markdown"] })),
            )
            .await?;
        let data = value.get("data").cloned().unwrap_or(value);
        Ok(ExtractedDoc {
            url: url.to_string(),
            title: pick_str(&data, &["title"]).or_else(|| {
                data.pointer("/metadata/title")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            }),
            content: pick_str(&data, &["markdown", "content", "rawHtml"]).unwrap_or_default(),
            provider: "firecrawl".into(),
        })
    }

    /// Start a crawl and poll until completion or timeout.
    pub async fn crawl(&self, url: &str, limit: u32, timeout_secs: u64) -> Result<Value> {
        crate::try_keys!(
            self.keys(),
            "firecrawl",
            "FIRECRAWL_API_KEY not set",
            |key| self.crawl_with(key, url, limit, timeout_secs).await,
        )
    }

    async fn crawl_with(
        &self,
        key: &str,
        url: &str,
        limit: u32,
        timeout_secs: u64,
    ) -> Result<Value> {
        let started = self
            .inner
            .http
            .json(
                "firecrawl",
                self.inner
                    .http
                    .post(&self.inner.url("/v2/crawl"))
                    .bearer_auth(key)
                    .json(&json!({ "url": url, "limit": limit })),
            )
            .await?;
        let id = pick_str(&started, &["id", "jobId"])
            .ok_or_else(|| Error::provider("firecrawl", "crawl response missing id"))?;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
        loop {
            let status = self
                .inner
                .http
                .json(
                    "firecrawl",
                    self.inner
                        .http
                        .get(&self.inner.url(&format!("/v2/crawl/{id}")))
                        .bearer_auth(key),
                )
                .await?;
            let state = pick_str(&status, &["status"]).unwrap_or_default();
            if state == "completed" || state == "failed" {
                return Ok(status);
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(Error::Timeout {
                    provider: "firecrawl".into(),
                    seconds: timeout_secs,
                });
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    pub async fn map(&self, url: &str, search: Option<&str>, limit: Option<u32>) -> Result<Value> {
        crate::try_keys!(
            self.keys(),
            "firecrawl",
            "FIRECRAWL_API_KEY not set",
            |key| self.map_with(key, url, search, limit).await,
        )
    }

    async fn map_with(
        &self,
        key: &str,
        url: &str,
        search: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Value> {
        let mut body = json!({ "url": url });
        if let Some(q) = search {
            body["search"] = json!(q);
        }
        if let Some(n) = limit {
            body["limit"] = json!(n);
        }
        self.inner
            .http
            .json(
                "firecrawl",
                self.inner
                    .http
                    .post(&self.inner.url("/v2/map"))
                    .bearer_auth(key)
                    .json(&body),
            )
            .await
    }
}

#[async_trait]
impl Provider for Firecrawl {
    fn id(&self) -> ProviderId {
        ProviderId::Firecrawl
    }
    fn is_configured(&self) -> bool {
        self.inner.configured()
    }
    fn skip_reason(&self) -> Option<String> {
        self.inner.skip("FIRECRAWL_API_KEY")
    }
    fn supports_extract(&self) -> bool {
        true
    }
    fn estimated_search_usd(&self) -> f64 {
        0.01
    }
    fn max_page_size(&self) -> u32 {
        50
    }
    fn notes(&self) -> &'static str {
        "Search plus scrape/crawl/map. Crawl is async and polled."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        crate::try_keys!(
            &self.inner.keys,
            "firecrawl",
            "FIRECRAWL_API_KEY not set",
            |key| self.search_with(key, request).await,
        )
    }

    async fn extract(&self, urls: &[String]) -> Result<Vec<ExtractedDoc>> {
        let mut out = Vec::new();
        for url in urls {
            out.push(self.scrape(url).await?);
        }
        Ok(out)
    }
}

impl Firecrawl {
    async fn search_with(
        &self,
        key: &str,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
        let value = self
            .inner
            .http
            .json(
                "firecrawl",
                self.inner
                    .http
                    .post(&self.inner.url("/v2/search"))
                    .bearer_auth(key)
                    .json(&json!({
                        "query": request.query,
                        "limit": request.page_size.min(100)
                    })),
            )
            .await?;
        let mut hits = map_hits(
            ProviderId::Firecrawl,
            &value,
            &["url"],
            &["title"],
            &["description", "markdown", "snippet"],
            &["publishedTime", "published_at"],
            &[],
        );
        if hits.is_empty()
            && let Some(web) = value.pointer("/data/web").and_then(Value::as_array)
        {
            hits = map_rows(
                ProviderId::Firecrawl,
                web,
                &["url"],
                &["title"],
                &["description", "snippet"],
                &[],
                &[],
            );
        }
        Ok(page(hits))
    }
}
