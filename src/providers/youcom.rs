//! You.com / ydc-index search + livecrawl extract.

use async_trait::async_trait;
use serde_json::Value;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::http::{HttpClient, pick_str};
use crate::types::{ExtractedDoc, Freshness, ProviderId, ProviderSearchRequest, SearchPage};

use super::{Provider, hit_from_value};

pub struct YouCom {
    http: HttpClient,
    key: Option<String>,
    base: String,
}

impl YouCom {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            key: config.keys.youcom.clone(),
            base: config.endpoints.youcom.clone(),
        }
    }
}

#[async_trait]
impl Provider for YouCom {
    fn id(&self) -> ProviderId {
        ProviderId::Youcom
    }
    fn is_configured(&self) -> bool {
        self.key.is_some()
    }
    fn skip_reason(&self) -> Option<String> {
        self.key
            .is_none()
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
        let key = self.key.as_deref().ok_or_else(|| Error::NotConfigured {
            provider: "youcom".into(),
            reason: "YOU_API_KEY not set".into(),
        })?;
        let freshness = request.freshness.map(|f| match f {
            Freshness::Day => "day",
            Freshness::Week => "week",
            Freshness::Month => "month",
            Freshness::Year => "year",
        });
        let offset = request
            .cursor
            .and_then(|c| c.parse::<u32>().ok())
            .unwrap_or(0);
        let mut builder = self
            .http
            .get(&format!("{}/v1/search", self.base))
            .header("X-API-Key", key)
            .query(&[
                ("query", request.query),
                ("count", &request.page_size.min(100).to_string()),
                ("offset", &offset.to_string()),
                ("country", request.country),
                ("language", request.language),
            ]);
        if let Some(f) = freshness {
            builder = builder.query(&[("freshness", f)]);
        }
        let (_, value) = self.http.send_json("youcom", builder).await?;
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
        let hits = rows
            .iter()
            .filter_map(|v| {
                hit_from_value(
                    ProviderId::Youcom,
                    v,
                    &["url"],
                    &["title"],
                    &["description", "snippets", "snippet"],
                    &["published_date", "date"],
                    &[],
                )
            })
            .collect::<Vec<_>>();
        let next = if hits.len() as u32 >= request.page_size.min(100) && offset < 9 {
            Some((offset + 1).to_string())
        } else {
            None
        };
        Ok(SearchPage {
            hits,
            next_cursor: next,
            answer: pick_str(&value, &["answer"]),
        })
    }

    async fn extract(&self, urls: &[String]) -> Result<Vec<ExtractedDoc>> {
        let key = self.key.as_deref().ok_or_else(|| Error::NotConfigured {
            provider: "youcom".into(),
            reason: "YOU_API_KEY not set".into(),
        })?;
        let mut docs = Vec::new();
        for url in urls {
            let (_, value) = self
                .http
                .send_json(
                    "youcom",
                    self.http
                        .get(&format!("{}/v1/search", self.base))
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
