//! Tavily search + extract.

use async_trait::async_trait;
use serde_json::json;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, pick_str};
use crate::types::{ExtractedDoc, ProviderId, ProviderSearchRequest, SearchPage, SearchType};

use super::{Keyed, Provider, extract_docs, freshness_token, map_hits, page_answer};

pub struct Tavily {
    inner: Keyed,
}

impl Tavily {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            inner: Keyed::new(http, config.keys.tavily.clone(), &config.endpoints.tavily),
        }
    }
}

#[async_trait]
impl Provider for Tavily {
    fn id(&self) -> ProviderId {
        ProviderId::Tavily
    }
    fn is_configured(&self) -> bool {
        self.inner.configured()
    }
    fn skip_reason(&self) -> Option<String> {
        self.inner.skip("TAVILY_API_KEY")
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
        "Web search and URL extract. Supports news topic and time_range. Dual-key: TAVILY_API_KEY_2."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        crate::try_keys!(
            &self.inner.keys,
            "tavily",
            "TAVILY_API_KEY not set",
            |key| { self.search_with(key, request).await }
        )
    }

    async fn extract(
        &self,
        urls: &[String],
        _account: Option<&str>,
    ) -> Result<Vec<ExtractedDoc>> {
        crate::try_keys!(
            &self.inner.keys,
            "tavily",
            "TAVILY_API_KEY not set",
            |key| { self.extract_with(key, urls).await }
        )
    }
}

impl Tavily {
    async fn search_with(
        &self,
        key: &str,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
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
        let value = self
            .inner
            .http
            .json(
                "tavily",
                self.inner
                    .http
                    .post(&self.inner.url("/search"))
                    .bearer_auth(key)
                    .json(&body),
            )
            .await?;
        Ok(page_answer(
            map_hits(
                ProviderId::Tavily,
                &value,
                &["url"],
                &["title"],
                &["content", "snippet"],
                &["published_date", "published_at"],
                &["score"],
            ),
            pick_str(&value, &["answer"]),
        ))
    }

    async fn extract_with(&self, key: &str, urls: &[String]) -> Result<Vec<ExtractedDoc>> {
        let value = self
            .inner
            .http
            .json(
                "tavily",
                self.inner
                    .http
                    .post(&self.inner.url("/extract"))
                    .bearer_auth(key)
                    .json(&json!({ "urls": urls })),
            )
            .await?;
        Ok(extract_docs(ProviderId::Tavily, &value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::hit_from_value;
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
