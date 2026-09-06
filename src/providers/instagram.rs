//! Instagram Graph API hashtag search (official, not free-text).

use async_trait::async_trait;
use serde_json::Value;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::http::{HttpClient, pick_str};
use crate::types::{ProviderId, ProviderSearchRequest, SearchHit, SearchPage};

use super::Provider;

pub struct Instagram {
    http: HttpClient,
    tokens: Vec<String>,
    user_id: Option<String>,
    base: String,
}

impl Instagram {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            tokens: config.keys.instagram_token.clone(),
            user_id: config.keys.instagram_user_id.clone(),
            base: config.endpoints.meta_graph.clone(),
        }
    }
}

fn hashtag_from_query(query: &str) -> String {
    if let Some(tag) = query.split_whitespace().find(|p| p.starts_with('#')) {
        return tag
            .trim_start_matches('#')
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
    }
    query
        .split_whitespace()
        .next()
        .unwrap_or(query)
        .trim_start_matches('#')
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

#[async_trait]
impl Provider for Instagram {
    fn id(&self) -> ProviderId {
        ProviderId::Instagram
    }
    fn is_configured(&self) -> bool {
        !self.tokens.is_empty() && self.user_id.is_some()
    }
    fn skip_reason(&self) -> Option<String> {
        if self.is_configured() {
            None
        } else {
            Some("INSTAGRAM_ACCESS_TOKEN and INSTAGRAM_BUSINESS_ACCOUNT_ID required".into())
        }
    }
    fn estimated_search_usd(&self) -> f64 {
        0.0
    }
    fn max_page_size(&self) -> u32 {
        50
    }
    fn notes(&self) -> &'static str {
        "Official Graph API hashtag search only. Free-text post search is not available. 30 unique hashtags / 7 days."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        crate::try_keys!(
            &self.tokens,
            "instagram",
            "INSTAGRAM_ACCESS_TOKEN not set",
            |token| self.search_with(token, request).await,
        )
    }
}

impl Instagram {
    async fn search_with(
        &self,
        token: &str,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
        let user_id = self
            .user_id
            .as_deref()
            .ok_or_else(|| Error::NotConfigured {
                provider: "instagram".into(),
                reason: "INSTAGRAM_BUSINESS_ACCOUNT_ID not set".into(),
            })?;
        let tag = hashtag_from_query(request.query);
        if tag.is_empty() {
            return Err(Error::Invalid(
                "instagram search needs a hashtag or single keyword".into(),
            ));
        }
        let resolved = self
            .http
            .json(
                "instagram",
                self.http
                    .get(&format!("{}/ig_hashtag_search", self.base))
                    .query(&[
                        ("user_id", user_id),
                        ("q", tag.as_str()),
                        ("access_token", token),
                    ]),
            )
            .await?;
        let hashtag_id = resolved
            .pointer("/data/0/id")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::provider("instagram", "hashtag id missing"))?;
        let mut builder = self
            .http
            .get(&format!("{}/{hashtag_id}/recent_media", self.base))
            .query(&[
                ("user_id", user_id),
                (
                    "fields",
                    "id,caption,media_type,permalink,timestamp,like_count",
                ),
                ("limit", &request.page_size.min(50).to_string()),
                ("access_token", token),
            ]);
        if let Some(after) = request.cursor {
            builder = builder.query(&[("after", after)]);
        }
        let media = self.http.json("instagram", builder).await?;
        let hits = media
            .get("data")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|m| {
                let url = pick_str(m, &["permalink"])?;
                let caption = pick_str(m, &["caption"]).unwrap_or_default();
                let mut hit = SearchHit::new(
                    ProviderId::Instagram,
                    caption.chars().take(80).collect::<String>(),
                    url,
                    caption,
                );
                hit.published_at = pick_str(m, &["timestamp"]);
                Some(hit)
            })
            .collect();
        let next = media
            .pointer("/paging/cursors/after")
            .and_then(Value::as_str)
            .map(str::to_string);
        Ok(SearchPage {
            hits,
            next_cursor: next,
            answer: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::hashtag_from_query;

    #[test]
    fn extracts_hashtag() {
        assert_eq!(hashtag_from_query("#Coffee art"), "Coffee");
        assert_eq!(hashtag_from_query("rustlang macros"), "rustlang");
    }
}
