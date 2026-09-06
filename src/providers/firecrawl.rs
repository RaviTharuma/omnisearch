//! Firecrawl search, scrape, crawl, and map.

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::config::Config;
use crate::error::{Error, Result};
use crate::http::{HttpClient, pick_str, result_array};
use crate::types::{ExtractedDoc, ProviderId, ProviderSearchRequest, SearchHit, SearchPage};

use super::{Provider, hit_from_value};

pub struct Firecrawl {
    http: HttpClient,
    keys: Vec<String>,
    base: String,
}

impl Firecrawl {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            keys: config.keys.firecrawl.clone(),
            base: config.endpoints.firecrawl.clone(),
        }
    }

    fn keys(&self) -> &[String] {
        &self.keys
    }

    /// Scrape a single URL to markdown.
    pub async fn scrape(&self, url: &str) -> Result<ExtractedDoc> {
        crate::try_keys!(
            self.keys(),
            "firecrawl",
            "FIRECRAWL_API_KEY not set",
            |key| self.scrape_with(key, url).await,
        )
    }

    async fn scrape_with(&self, key: &str, url: &str) -> Result<ExtractedDoc> {
        let (_, value) = self
            .http
            .send_json(
                "firecrawl",
                self.http
                    .post(&format!("{}/v2/scrape", self.base))
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
        let (_, started) = self
            .http
            .send_json(
                "firecrawl",
                self.http
                    .post(&format!("{}/v2/crawl", self.base))
                    .bearer_auth(key)
                    .json(&json!({ "url": url, "limit": limit })),
            )
            .await?;
        let id = pick_str(&started, &["id", "jobId"])
            .ok_or_else(|| Error::provider("firecrawl", "crawl response missing id"))?;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
        loop {
            let (_, status) = self
                .http
                .send_json(
                    "firecrawl",
                    self.http
                        .get(&format!("{}/v2/crawl/{id}", self.base))
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

    /// Map site URLs.
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
        let (_, value) = self
            .http
            .send_json(
                "firecrawl",
                self.http
                    .post(&format!("{}/v2/map", self.base))
                    .bearer_auth(key)
                    .json(&body),
            )
            .await?;
        Ok(value)
    }
}

#[async_trait]
impl Provider for Firecrawl {
    fn id(&self) -> ProviderId {
        ProviderId::Firecrawl
    }
    fn is_configured(&self) -> bool {
        !self.keys.is_empty()
    }
    fn skip_reason(&self) -> Option<String> {
        self.keys
            .is_empty()
            .then(|| "FIRECRAWL_API_KEY not set".into())
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
            &self.keys,
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
        let (_, value) = self
            .http
            .send_json(
                "firecrawl",
                self.http
                    .post(&format!("{}/v2/search", self.base))
                    .bearer_auth(key)
                    .json(&json!({
                        "query": request.query,
                        "limit": request.page_size.min(100)
                    })),
            )
            .await?;
        let mut hits: Vec<SearchHit> = result_array(&value)
            .iter()
            .filter_map(|v| {
                hit_from_value(
                    ProviderId::Firecrawl,
                    v,
                    &["url"],
                    &["title"],
                    &["description", "markdown", "snippet"],
                    &["publishedTime", "published_at"],
                    &[],
                )
            })
            .collect();
        if hits.is_empty()
            && let Some(web) = value.pointer("/data/web").and_then(Value::as_array)
        {
            hits = web
                .iter()
                .filter_map(|v| {
                    hit_from_value(
                        ProviderId::Firecrawl,
                        v,
                        &["url"],
                        &["title"],
                        &["description", "snippet"],
                        &[],
                        &[],
                    )
                })
                .collect();
        }
        Ok(SearchPage {
            hits,
            next_cursor: None,
            answer: None,
        })
    }
}
