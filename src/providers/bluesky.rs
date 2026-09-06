//! Bluesky public post search.

use async_trait::async_trait;
use serde_json::Value;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, pick_str};
use crate::types::{ProviderId, ProviderSearchRequest, SearchHit, SearchPage};

use super::Provider;

pub struct Bluesky {
    http: HttpClient,
    base: String,
}

impl Bluesky {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            base: config.endpoints.bluesky.clone(),
        }
    }
}

#[async_trait]
impl Provider for Bluesky {
    fn id(&self) -> ProviderId {
        ProviderId::Bluesky
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
        100
    }
    fn notes(&self) -> &'static str {
        "Public app.bsky.feed.searchPosts. No app password required for public search."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        let mut builder = self
            .http
            .get(&format!("{}/xrpc/app.bsky.feed.searchPosts", self.base))
            .query(&[
                ("q", request.query),
                ("limit", &request.page_size.min(100).to_string()),
            ]);
        if let Some(cursor) = request.cursor {
            builder = builder.query(&[("cursor", cursor)]);
        }
        let (_, value) = self.http.send_json("bluesky", builder).await?;
        let hits = value
            .get("posts")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|p| {
                let text = p
                    .pointer("/record/text")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let uri = pick_str(p, &["uri"])?;
                let url = at_uri_to_http(&uri, p);
                let mut hit = SearchHit::new(
                    ProviderId::Bluesky,
                    text.chars().take(80).collect::<String>(),
                    url,
                    text,
                );
                hit.published_at = p
                    .pointer("/record/createdAt")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                Some(hit)
            })
            .collect();
        Ok(SearchPage {
            hits,
            next_cursor: pick_str(&value, &["cursor"]),
            answer: None,
        })
    }
}

fn at_uri_to_http(uri: &str, post: &Value) -> String {
    if let Some(handle) = post.pointer("/author/handle").and_then(Value::as_str)
        && let Some(rkey) = uri.rsplit('/').next()
    {
        return format!("https://bsky.app/profile/{handle}/post/{rkey}");
    }
    uri.to_string()
}
