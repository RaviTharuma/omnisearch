//! Kagi Search API.

use async_trait::async_trait;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, pick_str};
use crate::types::{ProviderId, ProviderSearchRequest, SearchPage, SearchType};

use super::{Keyed, Provider, map_hits, page_answer};

pub struct Kagi {
    inner: Keyed,
}

impl Kagi {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            inner: Keyed::new(http, config.keys.kagi.clone(), &config.endpoints.kagi),
        }
    }
}

#[async_trait]
impl Provider for Kagi {
    fn id(&self) -> ProviderId {
        ProviderId::Kagi
    }
    fn is_configured(&self) -> bool {
        self.inner.configured()
    }
    fn skip_reason(&self) -> Option<String> {
        self.inner.skip("KAGI_API_KEY")
    }
    fn estimated_search_usd(&self) -> f64 {
        0.01
    }
    fn notes(&self) -> &'static str {
        "Authorization: Bot <KAGI_API_KEY>. News uses /api/v0/news."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        crate::try_keys!(&self.inner.keys, "kagi", "KAGI_API_KEY not set", |key| {
            self.search_with(key, request).await
        })
    }
}

impl Kagi {
    async fn search_with(
        &self,
        key: &str,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
        let path = if matches!(request.search_type, SearchType::News) {
            "/api/v0/news"
        } else {
            "/api/v0/search"
        };
        let value = self
            .inner
            .http
            .json(
                "kagi",
                self.inner
                    .http
                    .get(&self.inner.url(path))
                    .header("Authorization", format!("Bot {key}"))
                    .query(&[
                        ("q", request.query),
                        ("limit", &request.page_size.to_string()),
                    ]),
            )
            .await?;
        Ok(page_answer(
            map_hits(
                ProviderId::Kagi,
                &value,
                &["url"],
                &["title"],
                &["snippet"],
                &["published"],
                &[],
            ),
            pick_str(&value, &["output", "answer"]),
        ))
    }
}
