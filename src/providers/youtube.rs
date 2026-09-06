//! YouTube Data API v3 search.

use async_trait::async_trait;
use serde_json::Value;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, pick_str};
use crate::types::{ProviderId, ProviderSearchRequest, SearchHit, SearchPage};

use super::{Keyed, Provider};

pub struct Youtube {
    inner: Keyed,
}

impl Youtube {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            inner: Keyed::new(http, config.keys.youtube.clone(), &config.endpoints.youtube),
        }
    }
}

#[async_trait]
impl Provider for Youtube {
    fn id(&self) -> ProviderId {
        ProviderId::Youtube
    }
    fn is_configured(&self) -> bool {
        self.inner.configured()
    }
    fn skip_reason(&self) -> Option<String> {
        self.inner
            .keys
            .is_empty()
            .then(|| "YOUTUBE_API_KEY or GOOGLE_API_KEY not set".into())
    }
    fn estimated_search_usd(&self) -> f64 {
        0.0
    }
    fn max_page_size(&self) -> u32 {
        50
    }
    fn notes(&self) -> &'static str {
        "Official YouTube Data API v3 search.list. Paginated via pageToken."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        crate::try_keys!(
            &self.inner.keys,
            "youtube",
            "YOUTUBE_API_KEY not set",
            |key| { self.search_with(key, request).await }
        )
    }
}

impl Youtube {
    async fn search_with(
        &self,
        key: &str,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
        let mut builder = self
            .inner
            .http
            .get(&self.inner.url("/youtube/v3/search"))
            .query(&[
                ("part", "snippet"),
                ("q", request.query),
                ("type", "video"),
                ("maxResults", &request.page_size.min(50).to_string()),
                ("key", key),
            ]);
        if let Some(token) = request.cursor {
            builder = builder.query(&[("pageToken", token)]);
        }
        if let Some(fresh) = request.freshness {
            builder = builder.query(&[("publishedAfter", &fresh.since_rfc3339())]);
        }
        let value = self.inner.http.json("youtube", builder).await?;
        let hits = value
            .get("items")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|item| {
                let video_id = item.pointer("/id/videoId").and_then(Value::as_str)?;
                let snippet = item.get("snippet")?;
                let mut hit = SearchHit::new(
                    ProviderId::Youtube,
                    pick_str(snippet, &["title"]).unwrap_or_else(|| video_id.to_string()),
                    format!("https://www.youtube.com/watch?v={video_id}"),
                    pick_str(snippet, &["description"]).unwrap_or_default(),
                );
                hit.published_at = pick_str(snippet, &["publishedAt"]);
                Some(hit)
            })
            .collect();
        Ok(SearchPage {
            hits,
            next_cursor: pick_str(&value, &["nextPageToken"]),
            answer: None,
        })
    }
}
