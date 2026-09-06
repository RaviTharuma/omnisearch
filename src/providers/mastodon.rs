//! Mastodon public search (instance + optional token).

use async_trait::async_trait;
use serde_json::Value;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::http::{HttpClient, pick_str};
use crate::types::{ProviderId, ProviderSearchRequest, SearchHit, SearchPage};

use super::Provider;

pub struct Mastodon {
    http: HttpClient,
    instance: Option<String>,
    token: Option<String>,
}

impl Mastodon {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            instance: config.keys.mastodon_instance.clone(),
            token: config.keys.mastodon_token.clone(),
        }
    }
}

#[async_trait]
impl Provider for Mastodon {
    fn id(&self) -> ProviderId {
        ProviderId::Mastodon
    }
    fn is_configured(&self) -> bool {
        self.instance.is_some()
    }
    fn skip_reason(&self) -> Option<String> {
        self.instance
            .is_none()
            .then(|| "MASTODON_INSTANCE not set".into())
    }
    fn estimated_search_usd(&self) -> f64 {
        0.0
    }
    fn requires_key(&self) -> bool {
        false
    }
    fn notes(&self) -> &'static str {
        "GET {instance}/api/v2/search. MASTODON_ACCESS_TOKEN optional for authenticated search."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        let instance = self
            .instance
            .as_deref()
            .ok_or_else(|| Error::NotConfigured {
                provider: "mastodon".into(),
                reason: "MASTODON_INSTANCE not set".into(),
            })?;
        let base = instance.trim_end_matches('/');
        let mut builder = self.http.get(&format!("{base}/api/v2/search")).query(&[
            ("q", request.query),
            ("type", "statuses"),
            ("limit", &request.page_size.min(40).to_string()),
        ]);
        if let Some(token) = &self.token {
            builder = builder.bearer_auth(token);
        }
        let value = self.http.json("mastodon", builder).await?;
        let hits = value
            .get("statuses")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|s| {
                let url = pick_str(s, &["url", "uri"])?;
                let content = pick_str(s, &["content"]).unwrap_or_default();
                let mut hit = SearchHit::new(
                    ProviderId::Mastodon,
                    content.chars().take(80).collect::<String>(),
                    url,
                    crate::ground::strip_markup(&content),
                );
                hit.published_at = pick_str(s, &["created_at"]);
                Some(hit)
            })
            .collect();
        Ok(super::page(hits))
    }
}
