//! Tavily search + extract.

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, pick_str, result_array};
use crate::types::{ExtractedDoc, ProviderId, ProviderSearchRequest, SearchPage, SearchType};

use super::{Provider, freshness_token, hit_from_value};

pub struct Tavily {
    http: HttpClient,
    key: Option<String>,
    base: String,
}

impl Tavily {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            key: config.keys.tavily.clone(),
            base: config.endpoints.tavily.clone(),
        }
    }
}

#[async_trait]
impl Provider for Tavily {
    fn id(&self) -> ProviderId {
        ProviderId::Tavily
    }
    fn is_configured(&self) -> bool {
        self.key.is_some()
    }
    fn skip_reason(&self) -> Option<String> {
        self.key.is_none().then(|| "TAVILY_API_KEY not set".into())
    }
    fn supports_extract(&self) -> bool {
        true
    }
    fn estimated_search_usd(&self) -> f64 {
        0.008
    }
    fn max_page_size(&self) -> u32 {
        20
    }
    fn notes(&self) -> &'static str {
        "Web search and URL extract. Supports news topic and time_range."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        let key = self
            .key
            .as_deref()
            .ok_or_else(|| crate::error::Error::NotConfigured {
                provider: "tavily".into(),
                reason: "TAVILY_API_KEY not set".into(),
            })?;
        let mut body = json!({
            "query": request.query,
            "max_results": request.page_size.min(20),
            "include_answer": true,
        });
        if matches!(request.search_type, SearchType::News) {
            body["topic"] = json!("news");
        }
        if let Some(token) = freshness_token(request.freshness) {
            body["time_range"] = json!(token);
        }
        let (_, value) = self
            .http
            .send_json(
                "tavily",
                self.http
                    .post(&format!("{}/search", self.base))
                    .bearer_auth(key)
                    .json(&body),
            )
            .await?;
        let hits = result_array(&value)
            .iter()
            .filter_map(|v| {
                hit_from_value(
                    ProviderId::Tavily,
                    v,
                    &["url"],
                    &["title"],
                    &["content", "snippet"],
                    &["published_date", "published_at"],
                    &["score"],
                )
            })
            .collect();
        Ok(SearchPage {
            hits,
            next_cursor: None,
            answer: pick_str(&value, &["answer"]),
        })
    }

    async fn extract(&self, urls: &[String]) -> Result<Vec<ExtractedDoc>> {
        let key = self
            .key
            .as_deref()
            .ok_or_else(|| crate::error::Error::NotConfigured {
                provider: "tavily".into(),
                reason: "TAVILY_API_KEY not set".into(),
            })?;
        let (_, value) = self
            .http
            .send_json(
                "tavily",
                self.http
                    .post(&format!("{}/extract", self.base))
                    .bearer_auth(key)
                    .json(&json!({ "urls": urls })),
            )
            .await?;
        Ok(extract_docs(ProviderId::Tavily, &value))
    }
}

pub(crate) fn extract_docs(id: ProviderId, value: &Value) -> Vec<ExtractedDoc> {
    result_array(value)
        .into_iter()
        .filter_map(|v| {
            let url = pick_str(&v, &["url"])?;
            Some(ExtractedDoc {
                url,
                title: pick_str(&v, &["title"]),
                content: pick_str(&v, &["raw_content", "content", "text", "markdown"])
                    .unwrap_or_default(),
                provider: id.as_str().to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_tavily_hits() {
        let v = json!({"url":"https://a.test","title":"A","content":"hello","score":0.4});
        let hit = hit_from_value(
            ProviderId::Tavily,
            &v,
            &["url"],
            &["title"],
            &["content"],
            &["published_date"],
            &["score"],
        )
        .unwrap();
        assert_eq!(hit.title, "A");
        assert_eq!(hit.score, Some(0.4));
    }
}
