//! X.com search via X API v2 or xAI Responses x_search.

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::config::Config;
use crate::error::{Error, Result};
use crate::http::{HttpClient, pick_str};
use crate::types::{ProviderId, ProviderSearchRequest, SearchHit, SearchPage};

use super::Provider;

pub struct XSearch {
    http: HttpClient,
    bearer: Option<String>,
    xai: Option<String>,
    x_base: String,
    xai_base: String,
}

impl XSearch {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            bearer: config.keys.x_bearer.clone(),
            xai: config.keys.xai.clone(),
            x_base: config.endpoints.x.clone(),
            xai_base: config.endpoints.xai.clone(),
        }
    }

    async fn search_x_api(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        let token = self.bearer.as_deref().unwrap();
        let mut builder = self
            .http
            .get(&format!("{}/2/tweets/search/recent", self.x_base))
            .bearer_auth(token)
            .query(&[
                ("query", request.query),
                ("max_results", &request.page_size.clamp(10, 100).to_string()),
                ("tweet.fields", "created_at,public_metrics,author_id"),
            ]);
        if let Some(cursor) = request.cursor {
            builder = builder.query(&[("next_token", cursor)]);
        }
        let (_, value) = self.http.send_json("x", builder).await?;
        let hits = value
            .get("data")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|t| {
                let id = pick_str(t, &["id"])?;
                let text = pick_str(t, &["text"]).unwrap_or_default();
                let mut hit = SearchHit::new(
                    ProviderId::X,
                    text.chars().take(80).collect::<String>(),
                    format!("https://x.com/i/web/status/{id}"),
                    text,
                );
                hit.published_at = pick_str(t, &["created_at"]);
                Some(hit)
            })
            .collect();
        let next = value
            .pointer("/meta/next_token")
            .and_then(Value::as_str)
            .map(str::to_string);
        Ok(SearchPage {
            hits,
            next_cursor: next,
            answer: None,
        })
    }

    async fn search_xai(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        let key = self.xai.as_deref().unwrap();
        let (_, value) = self
            .http
            .send_json(
                "x",
                self.http
                    .post(&format!("{}/v1/responses", self.xai_base))
                    .bearer_auth(key)
                    .json(&json!({
                        "model": "grok-4",
                        "input": request.query,
                        "tools": [{ "type": "x_search" }]
                    })),
            )
            .await?;
        let mut hits = Vec::new();
        collect_urls(&value, &mut hits);
        let answer = pick_str(&value, &["output_text"]);
        Ok(SearchPage {
            hits,
            next_cursor: None,
            answer,
        })
    }
}

fn collect_urls(value: &Value, hits: &mut Vec<SearchHit>) {
    match value {
        Value::Object(map) => {
            if let (Some(url), title) = (
                map.get("url").and_then(Value::as_str),
                map.get("title")
                    .or_else(|| map.get("text"))
                    .and_then(Value::as_str),
            ) && (url.contains("x.com") || url.contains("twitter.com"))
            {
                hits.push(SearchHit::new(
                    ProviderId::X,
                    title.unwrap_or(url),
                    url,
                    title.unwrap_or_default(),
                ));
            }
            for v in map.values() {
                collect_urls(v, hits);
            }
        }
        Value::Array(items) => {
            for v in items {
                collect_urls(v, hits);
            }
        }
        _ => {}
    }
}

#[async_trait]
impl Provider for XSearch {
    fn id(&self) -> ProviderId {
        ProviderId::X
    }
    fn is_configured(&self) -> bool {
        self.bearer.is_some() || self.xai.is_some()
    }
    fn skip_reason(&self) -> Option<String> {
        if self.is_configured() {
            None
        } else {
            Some("X_BEARER_TOKEN or XAI_API_KEY not set".into())
        }
    }
    fn estimated_search_usd(&self) -> f64 {
        if self.bearer.is_some() { 0.0 } else { 0.02 }
    }
    fn max_page_size(&self) -> u32 {
        100
    }
    fn notes(&self) -> &'static str {
        "Prefers X API v2 recent search (X_BEARER_TOKEN). Falls back to xAI Responses x_search (XAI_API_KEY)."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        if self.bearer.is_some() {
            return self.search_x_api(request).await;
        }
        if self.xai.is_some() {
            return self.search_xai(request).await;
        }
        Err(Error::NotConfigured {
            provider: "x".into(),
            reason: "X_BEARER_TOKEN or XAI_API_KEY not set".into(),
        })
    }
}
