//! First-class Brave Search adapter (web + news).

use async_trait::async_trait;

use crate::config::Config;
use crate::error::Result;
use crate::http::HttpClient;
use crate::types::{Freshness, ProviderId, ProviderSearchRequest, SearchPage, SearchType};

use super::{Keyed, Provider, map_rows, offset, page_next};

pub struct Brave {
    inner: Keyed,
}

impl Brave {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            inner: Keyed::new(http, config.keys.brave.clone(), &config.endpoints.brave),
        }
    }
}

#[async_trait]
impl Provider for Brave {
    fn id(&self) -> ProviderId {
        ProviderId::Brave
    }
    fn is_configured(&self) -> bool {
        self.inner.configured()
    }
    fn skip_reason(&self) -> Option<String> {
        self.inner.skip("BRAVE_API_KEY")
    }
    fn estimated_search_usd(&self) -> f64 {
        0.003
    }
    fn max_page_size(&self) -> u32 {
        20
    }
    fn notes(&self) -> &'static str {
        "First-class web + news search. Always in default parallel fan-out when BRAVE_API_KEY is set. Supports country, search_lang, freshness. Dual-key: BRAVE_API_KEY_2."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        crate::try_keys!(&self.inner.keys, "brave", "BRAVE_API_KEY not set", |key| {
            self.search_with(key, request).await
        })
    }
}

impl Brave {
    async fn search_with(
        &self,
        key: &str,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
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
        let start = offset(request.cursor);
        let page_size = request.page_size.min(20);
        let mut req = self
            .inner
            .http
            .get(&self.inner.url(path))
            .header("X-Subscription-Token", key)
            .header("Accept", "application/json")
            .query(&[
                ("q", request.query),
                ("count", &page_size.to_string()),
                ("offset", &start.to_string()),
                ("country", request.country),
                ("search_lang", request.language),
            ]);
        if let Some(f) = freshness {
            req = req.query(&[("freshness", f)]);
        }
        let value = self.inner.http.json("brave", req).await?;
        let rows = value
            .pointer("/web/results")
            .or_else(|| value.pointer("/news/results"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_else(|| crate::http::result_array(&value));
        Ok(page_next(
            map_rows(
                ProviderId::Brave,
                &rows,
                &["url"],
                &["title"],
                &["description"],
                &["page_age", "age"],
                &[],
            ),
            page_size,
            start + page_size,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::hit_from_value;
    use serde_json::json;

    #[test]
    fn maps_web_and_news_hits() {
        let web = hit_from_value(
            ProviderId::Brave,
            &json!({
                "url": "https://brave.com/search",
                "title": "Brave Search",
                "description": "Independent search",
                "page_age": "2026-01-01"
            }),
            &["url"],
            &["title"],
            &["description"],
            &["page_age", "age"],
            &[],
        )
        .unwrap();
        assert_eq!(web.title, "Brave Search");
        assert_eq!(web.published_at.as_deref(), Some("2026-01-01"));

        let news = hit_from_value(
            ProviderId::Brave,
            &json!({
                "url": "https://news.example/a",
                "title": "Headline",
                "description": "Today"
            }),
            &["url"],
            &["title"],
            &["description"],
            &["page_age", "age"],
            &[],
        )
        .unwrap();
        assert_eq!(news.provider, "brave");
    }
}
