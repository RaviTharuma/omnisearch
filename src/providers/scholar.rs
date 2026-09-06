//! Semantic Scholar paper search (no key required for basic use).

use async_trait::async_trait;
use serde_json::Value;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, pick_str};
use crate::types::{ProviderId, ProviderSearchRequest, SearchHit, SearchPage};

use super::{Provider, offset, page_next};

pub struct Scholar {
    http: HttpClient,
    base: String,
}

impl Scholar {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            base: config.endpoints.scholar.clone(),
        }
    }
}

#[async_trait]
impl Provider for Scholar {
    fn id(&self) -> ProviderId {
        ProviderId::Scholar
    }
    fn is_configured(&self) -> bool {
        true
    }
    fn estimated_search_usd(&self) -> f64 {
        0.0
    }
    fn requires_key(&self) -> bool {
        false
    }
    fn max_page_size(&self) -> u32 {
        100
    }
    fn notes(&self) -> &'static str {
        "Semantic Scholar /graph/v1/paper/search. Scholarly vertical."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        let start = offset(request.cursor);
        let page_size = request.page_size.min(100);
        let value = self
            .http
            .json(
                "scholar",
                self.http
                    .get(&format!("{}/graph/v1/paper/search", self.base))
                    .query(&[
                        ("query", request.query),
                        ("limit", &page_size.to_string()),
                        ("offset", &start.to_string()),
                        ("fields", "title,url,abstract,year,externalIds"),
                    ]),
            )
            .await?;
        let hits = value
            .get("data")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|p| {
                let title = pick_str(p, &["title"])?;
                let url = pick_str(p, &["url"]).unwrap_or_else(|| {
                    pick_str(p, &["paperId"])
                        .map(|id| format!("https://www.semanticscholar.org/paper/{id}"))
                        .unwrap_or_default()
                });
                if url.is_empty() {
                    return None;
                }
                let mut hit = SearchHit::new(
                    ProviderId::Scholar,
                    title,
                    url,
                    pick_str(p, &["abstract"]).unwrap_or_default(),
                );
                if let Some(year) = p.get("year").and_then(Value::as_u64) {
                    hit.published_at = Some(format!("{year}-01-01T00:00:00Z"));
                }
                Some(hit)
            })
            .collect::<Vec<_>>();
        Ok(page_next(hits, page_size, start + page_size))
    }
}
