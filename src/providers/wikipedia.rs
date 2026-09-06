//! Wikipedia MediaWiki search (no key).

use async_trait::async_trait;
use serde_json::Value;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, pick_str};
use crate::types::{ProviderId, ProviderSearchRequest, SearchHit, SearchPage};

use super::{Provider, offset, page_next};

pub struct Wikipedia {
    http: HttpClient,
    base: String,
}

impl Wikipedia {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            base: config.endpoints.wikipedia.clone(),
        }
    }
}

#[async_trait]
impl Provider for Wikipedia {
    fn id(&self) -> ProviderId {
        ProviderId::Wikipedia
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
        50
    }
    fn notes(&self) -> &'static str {
        "MediaWiki action=query&list=search. Always available."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        let start = offset(request.cursor);
        let page_size = request.page_size.min(50);
        let value = self
            .http
            .json(
                "wikipedia",
                self.http.get(&format!("{}/w/api.php", self.base)).query(&[
                    ("action", "query"),
                    ("list", "search"),
                    ("srsearch", request.query),
                    ("srlimit", &page_size.to_string()),
                    ("sroffset", &start.to_string()),
                    ("format", "json"),
                ]),
            )
            .await?;
        let hits = value
            .pointer("/query/search")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|row| {
                let title = pick_str(row, &["title"])?;
                let snippet = pick_str(row, &["snippet"]).unwrap_or_default();
                Some(SearchHit::new(
                    ProviderId::Wikipedia,
                    &title,
                    format!("https://en.wikipedia.org/wiki/{}", title.replace(' ', "_")),
                    crate::ground::strip_markup(&snippet),
                ))
            })
            .collect::<Vec<_>>();
        Ok(page_next(hits, page_size, start + page_size))
    }
}
