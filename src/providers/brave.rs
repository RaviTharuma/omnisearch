//! Brave Search API.

use async_trait::async_trait;

use crate::config::Config;
use crate::error::Result;
use crate::http::HttpClient;
use crate::types::{Freshness, ProviderId, ProviderSearchRequest, SearchPage, SearchType};

use super::{Provider, hit_from_value};

pub struct Brave {
    http: HttpClient,
    key: Option<String>,
    base: String,
}

impl Brave {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            key: config.keys.brave.clone(),
            base: config.endpoints.brave.clone(),
        }
    }
}

#[async_trait]
impl Provider for Brave {
    fn id(&self) -> ProviderId {
        ProviderId::Brave
    }
    fn is_configured(&self) -> bool {
        self.key.is_some()
    }
    fn skip_reason(&self) -> Option<String> {
        self.key.is_none().then(|| "BRAVE_API_KEY not set".into())
    }
    fn estimated_search_usd(&self) -> f64 {
        0.003
    }
    fn max_page_size(&self) -> u32 {
        20
    }
    fn notes(&self) -> &'static str {
        "Web and news endpoints. Supports country, search_lang, freshness."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        let key = self
            .key
            .as_deref()
            .ok_or_else(|| crate::error::Error::NotConfigured {
                provider: "brave".into(),
                reason: "BRAVE_API_KEY not set".into(),
            })?;
        let path = if matches!(request.search_type, SearchType::News) {
            "/res/v1/news/search"
        } else {
            "/res/v1/web/search"
        };
        let freshness = request.freshness.map(|f| match f {
            Freshness::Day => "pd",
            Freshness::Week => "pw",
            Freshness::Month => "pm",
            Freshness::Year => "py",
        });
        let offset = request
            .cursor
            .and_then(|c| c.parse::<u32>().ok())
            .unwrap_or(0);
        let mut req = self
            .http
            .get(&format!("{}{path}", self.base))
            .header("X-Subscription-Token", key)
            .header("Accept", "application/json")
            .query(&[
                ("q", request.query),
                ("count", &request.page_size.min(20).to_string()),
                ("offset", &offset.to_string()),
                ("country", request.country),
                ("search_lang", request.language),
            ]);
        if let Some(f) = freshness {
            req = req.query(&[("freshness", f)]);
        }
        let (_, value) = self.http.send_json("brave", req).await?;
        let web = value
            .pointer("/web/results")
            .or_else(|| value.pointer("/news/results"))
            .cloned()
            .unwrap_or(serde_json::Value::Array(crate::http::result_array(&value)));
        let hits = web
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| {
                hit_from_value(
                    ProviderId::Brave,
                    v,
                    &["url"],
                    &["title"],
                    &["description"],
                    &["page_age", "age"],
                    &[],
                )
            })
            .collect::<Vec<_>>();
        let next = if hits.len() as u32 >= request.page_size.min(20) {
            Some((offset + request.page_size.min(20)).to_string())
        } else {
            None
        };
        Ok(SearchPage {
            hits,
            next_cursor: next,
            answer: None,
        })
    }
}
