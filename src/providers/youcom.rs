//! You.com / ydc-index search + livecrawl extract.

use async_trait::async_trait;
use serde_json::Value;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, pick_str};
use crate::types::{ExtractedDoc, Freshness, ProviderId, ProviderSearchRequest, SearchPage};

use super::{Keyed, Provider, map_rows, offset};

pub struct YouCom {
    inner: Keyed,
}

impl YouCom {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            inner: Keyed::new(http, config.keys.youcom.clone(), &config.endpoints.youcom),
        }
    }
}

#[async_trait]
impl Provider for YouCom {
    fn id(&self) -> ProviderId {
        ProviderId::Youcom
    }
    fn is_configured(&self) -> bool {
        self.inner.configured()
    }
    fn skip_reason(&self) -> Option<String> {
        self.inner
            .keys
            .is_empty()
            .then(|| "YOU_API_KEY or YDC_API_KEY not set".into())
    }
    fn supports_extract(&self) -> bool {
        true
    }
    fn estimated_search_usd(&self) -> f64 {
        0.005
    }
    fn max_page_size(&self) -> u32 {
        100
    }
    fn notes(&self) -> &'static str {
        "GET /v1/search on ydc-index.io. Extract uses livecrawl=web."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        crate::try_keys!(&self.inner.keys, "youcom", "YOU_API_KEY not set", |key| {
            self.search_with(key, request).await
        })
    }

    async fn extract(&self, urls: &[String], _account: Option<&str>) -> Result<Vec<ExtractedDoc>> {
        crate::try_keys!(&self.inner.keys, "youcom", "YOU_API_KEY not set", |key| {
            self.extract_with(key, urls).await
        })
    }
}

impl YouCom {
    async fn search_with(
        &self,
        key: &str,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
        let freshness = request.freshness.map(|f| match f {
            Freshness::Day => "day",
            Freshness::Week => "week",
            Freshness::Month => "month",
            Freshness::Year => "year",
        });
        let start = offset(request.cursor);
        let page_size = request.page_size.min(100);
        let mut builder = self
            .inner
            .http
            .get(&self.inner.url("/v1/search"))
            .header("X-API-Key", key)
            .query(&[
                ("query", request.query),
                ("count", &page_size.to_string()),
                ("offset", &start.to_string()),
                ("country", request.country),
                ("language", request.language),
            ]);
        if let Some(f) = freshness {
            builder = builder.query(&[("freshness", f)]);
        }
        let value = self.inner.http.json("youcom", builder).await?;
        let mut rows = Vec::new();
        if let Some(web) = value.pointer("/results/web").and_then(Value::as_array) {
            rows.extend(web.iter().cloned());
        }
        if let Some(news) = value.pointer("/results/news").and_then(Value::as_array) {
            rows.extend(news.iter().cloned());
        }
        if rows.is_empty() {
            rows = crate::http::result_array(&value);
        }
        let hits = map_rows(
            ProviderId::Youcom,
            &rows,
            &["url"],
            &["title"],
            &["description", "snippets", "snippet"],
            &["published_date", "date"],
            &[],
        );
        let next = (hits.len() as u32 >= page_size && start < 9).then(|| (start + 1).to_string());
        Ok(SearchPage {
            hits,
            next_cursor: next,
            answer: pick_str(&value, &["answer"]),
        })
    }

    async fn extract_with(&self, key: &str, urls: &[String]) -> Result<Vec<ExtractedDoc>> {
        let mut docs = Vec::new();
        for url in urls {
            let value = self
                .inner
                .http
                .json(
                    "youcom",
                    self.inner
                        .http
                        .get(&self.inner.url("/v1/search"))
                        .header("X-API-Key", key)
                        .query(&[
                            ("query", url.as_str()),
                            ("count", "1"),
                            ("livecrawl", "web"),
                            ("livecrawl_formats", "markdown"),
                        ]),
                )
                .await?;
            let content = value
                .pointer("/results/web/0/contents/markdown")
                .and_then(Value::as_str)
                .or_else(|| {
                    value
                        .pointer("/results/web/0/description")
                        .and_then(Value::as_str)
                })
                .unwrap_or_default()
                .to_string();
            docs.push(ExtractedDoc {
                url: url.clone(),
                title: value
                    .pointer("/results/web/0/title")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                content,
                provider: "youcom".into(),
            });
        }
        Ok(docs)
    }
}
