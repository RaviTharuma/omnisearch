//! Kagi Search API.

use async_trait::async_trait;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, pick_str, result_array};
use crate::types::{ProviderId, ProviderSearchRequest, SearchPage, SearchType};

use super::{Provider, hit_from_value};

pub struct Kagi {
    http: HttpClient,
    key: Option<String>,
    base: String,
}

impl Kagi {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            key: config.keys.kagi.clone(),
            base: config.endpoints.kagi.clone(),
        }
    }
}

#[async_trait]
impl Provider for Kagi {
    fn id(&self) -> ProviderId {
        ProviderId::Kagi
    }
    fn is_configured(&self) -> bool {
        self.key.is_some()
    }
    fn skip_reason(&self) -> Option<String> {
        self.key.is_none().then(|| "KAGI_API_KEY not set".into())
    }
    fn estimated_search_usd(&self) -> f64 {
        0.01
    }
    fn notes(&self) -> &'static str {
        "Authorization: Bot <KAGI_API_KEY>. News uses /api/v0/news."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        let key = self
            .key
            .as_deref()
            .ok_or_else(|| crate::error::Error::NotConfigured {
                provider: "kagi".into(),
                reason: "KAGI_API_KEY not set".into(),
            })?;
        let path = if matches!(request.search_type, SearchType::News) {
            "/api/v0/news"
        } else {
            "/api/v0/search"
        };
        let (_, value) = self
            .http
            .send_json(
                "kagi",
                self.http
                    .get(&format!("{}{path}", self.base))
                    .header("Authorization", format!("Bot {key}"))
                    .query(&[
                        ("q", request.query),
                        ("limit", &request.page_size.to_string()),
                    ]),
            )
            .await?;
        let hits = result_array(&value)
            .iter()
            .filter_map(|v| {
                hit_from_value(
                    ProviderId::Kagi,
                    v,
                    &["url"],
                    &["title"],
                    &["snippet"],
                    &["published"],
                    &[],
                )
            })
            .collect();
        Ok(SearchPage {
            hits,
            next_cursor: None,
            answer: pick_str(&value, &["output", "answer"]),
        })
    }
}
