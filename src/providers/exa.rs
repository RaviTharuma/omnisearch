//! Exa neural / keyword search and contents extract.

use async_trait::async_trait;
use serde_json::json;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, result_array};
use crate::types::{ExtractedDoc, ProviderId, ProviderSearchRequest, SearchPage, SearchType};

use super::{Provider, hit_from_value};

pub struct Exa {
    http: HttpClient,
    key: Option<String>,
    base: String,
}

impl Exa {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            key: config.keys.exa.clone(),
            base: config.endpoints.exa.clone(),
        }
    }
}

#[async_trait]
impl Provider for Exa {
    fn id(&self) -> ProviderId {
        ProviderId::Exa
    }
    fn is_configured(&self) -> bool {
        self.key.is_some()
    }
    fn skip_reason(&self) -> Option<String> {
        self.key.is_none().then(|| "EXA_API_KEY not set".into())
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
        "Neural / keyword search plus /contents extract."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        let key = self
            .key
            .as_deref()
            .ok_or_else(|| crate::error::Error::NotConfigured {
                provider: "exa".into(),
                reason: "EXA_API_KEY not set".into(),
            })?;
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
        let (_, value) = self
            .http
            .send_json(
                "exa",
                self.http
                    .post(&format!("{}/search", self.base))
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
        Ok(SearchPage {
            hits,
            next_cursor: None,
            answer: None,
        })
    }

    async fn extract(&self, urls: &[String]) -> Result<Vec<ExtractedDoc>> {
        let key = self
            .key
            .as_deref()
            .ok_or_else(|| crate::error::Error::NotConfigured {
                provider: "exa".into(),
                reason: "EXA_API_KEY not set".into(),
            })?;
        let (_, value) = self
            .http
            .send_json(
                "exa",
                self.http
                    .post(&format!("{}/contents", self.base))
                    .header("x-api-key", key)
                    .json(&json!({ "urls": urls, "text": true })),
            )
            .await?;
        Ok(super::tavily::extract_docs(ProviderId::Exa, &value))
    }
}
