//! Wikipedia MediaWiki search (no key).

use async_trait::async_trait;
use serde_json::Value;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, pick_str};
use crate::types::{ProviderId, ProviderSearchRequest, SearchHit, SearchPage};

use super::Provider;

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
    fn skip_reason(&self) -> Option<String> {
        None
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
        let offset = request
            .cursor
            .and_then(|c| c.parse::<u32>().ok())
            .unwrap_or(0);
        let (_, value) = self
            .http
            .send_json(
                "wikipedia",
                self.http.get(&format!("{}/w/api.php", self.base)).query(&[
                    ("action", "query"),
                    ("list", "search"),
                    ("srsearch", request.query),
                    ("srlimit", &request.page_size.min(50).to_string()),
                    ("sroffset", &offset.to_string()),
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
                    html_to_text(&snippet),
                ))
            })
            .collect::<Vec<_>>();
        let next = if hits.len() as u32 >= request.page_size.min(50) {
            Some((offset + request.page_size.min(50)).to_string())
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

fn html_to_text(input: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in input.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}
