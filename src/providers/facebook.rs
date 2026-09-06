//! Facebook Pages Search (official). Public post keyword search is not available.

use async_trait::async_trait;
use serde_json::Value;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, pick_str};
use crate::types::{ProviderId, ProviderSearchRequest, SearchHit, SearchPage};

use super::{Keyed, Provider};

pub struct Facebook {
    inner: Keyed,
}

impl Facebook {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            inner: Keyed::new(
                http,
                config.keys.facebook_token.clone(),
                &config.endpoints.meta_graph,
            ),
        }
    }
}

#[async_trait]
impl Provider for Facebook {
    fn id(&self) -> ProviderId {
        ProviderId::Facebook
    }
    fn is_configured(&self) -> bool {
        self.inner.configured()
    }
    fn skip_reason(&self) -> Option<String> {
        self.inner
            .keys
            .is_empty()
            .then(|| "FACEBOOK_ACCESS_TOKEN or META_ACCESS_TOKEN not set".into())
    }
    fn estimated_search_usd(&self) -> f64 {
        0.0
    }
    fn notes(&self) -> &'static str {
        "Official GET /pages/search only. Graph API no longer offers public post keyword search. Page Public Content Access may be required."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        crate::try_keys!(
            &self.inner.keys,
            "facebook",
            "FACEBOOK_ACCESS_TOKEN not set",
            |token| self.search_with(token, request).await,
        )
    }
}

impl Facebook {
    async fn search_with(
        &self,
        token: &str,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
        let value = self
            .inner
            .http
            .json(
                "facebook",
                self.inner
                    .http
                    .get(&self.inner.url("/pages/search"))
                    .query(&[
                        ("q", request.query),
                        ("fields", "id,name,link,about,fan_count"),
                        ("access_token", token),
                    ]),
            )
            .await?;
        let hits = value
            .get("data")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|p| {
                let url = pick_str(p, &["link"]).or_else(|| {
                    pick_str(p, &["id"]).map(|id| format!("https://www.facebook.com/{id}"))
                })?;
                Some(SearchHit::new(
                    ProviderId::Facebook,
                    pick_str(p, &["name"]).unwrap_or_else(|| url.clone()),
                    url,
                    pick_str(p, &["about"]).unwrap_or_default(),
                ))
            })
            .collect();
        let next = value
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
