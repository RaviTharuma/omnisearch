//! Reddit keyword search (public JSON or OAuth).

use async_trait::async_trait;
use serde_json::Value;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::http::{HttpClient, pick_str};
use crate::types::{ProviderId, ProviderSearchRequest, SearchHit, SearchPage};

use super::Provider;

pub struct Reddit {
    http: HttpClient,
    client_id: Option<String>,
    client_secret: Option<String>,
    public_base: String,
    oauth_base: String,
    user_agent: String,
}

impl Reddit {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            client_id: config.keys.reddit_client_id.clone(),
            client_secret: config.keys.reddit_client_secret.clone(),
            public_base: config.endpoints.reddit.clone(),
            oauth_base: config.endpoints.reddit_oauth.clone(),
            user_agent: config.user_agent.clone(),
        }
    }

    async fn oauth_token(&self) -> Result<Option<String>> {
        let (Some(id), Some(secret)) = (&self.client_id, &self.client_secret) else {
            return Ok(None);
        };
        let value = self
            .http
            .json(
                "reddit",
                self.http
                    .post(&format!("{}/api/v1/access_token", self.public_base))
                    .header("User-Agent", &self.user_agent)
                    .basic_auth(id, Some(secret))
                    .form(&[("grant_type", "client_credentials")]),
            )
            .await?;
        Ok(pick_str(&value, &["access_token"]))
    }
}

#[async_trait]
impl Provider for Reddit {
    fn id(&self) -> ProviderId {
        ProviderId::Reddit
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
        "Public search.json always available. OAuth used when REDDIT_CLIENT_ID/SECRET are set."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        let token = self.oauth_token().await.ok().flatten();
        let (base, path) = if token.is_some() {
            (self.oauth_base.as_str(), "/search")
        } else {
            (self.public_base.as_str(), "/search.json")
        };
        let mut builder = self
            .http
            .get(&format!("{base}{path}"))
            .header("User-Agent", &self.user_agent)
            .query(&[
                ("q", request.query),
                ("limit", &request.page_size.min(100).to_string()),
                ("sort", "relevance"),
            ]);
        if let Some(after) = request.cursor {
            builder = builder.query(&[("after", after)]);
        }
        if let Some(token) = token {
            builder = builder.bearer_auth(token);
        }
        let value = self.http.json("reddit", builder).await?;
        let children = value
            .pointer("/data/children")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let hits: Vec<SearchHit> = children
            .iter()
            .filter_map(|child| {
                let data = child.get("data")?;
                let permalink = pick_str(data, &["permalink"])?;
                let url = if permalink.starts_with("http") {
                    permalink
                } else {
                    format!("https://www.reddit.com{permalink}")
                };
                let mut hit = SearchHit::new(
                    ProviderId::Reddit,
                    pick_str(data, &["title"]).unwrap_or_else(|| url.clone()),
                    url,
                    pick_str(data, &["selftext"]).unwrap_or_default(),
                );
                if let Some(created) = data.get("created_utc").and_then(Value::as_f64) {
                    hit.published_at = chrono::DateTime::from_timestamp(created as i64, 0)
                        .map(|dt| dt.to_rfc3339());
                }
                Some(hit)
            })
            .collect();
        let next = value
            .pointer("/data/after")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        if hits.is_empty() && value.get("error").is_some() {
            return Err(Error::provider("reddit", value.to_string()));
        }
        Ok(SearchPage {
            hits,
            next_cursor: next,
            answer: None,
        })
    }
}
