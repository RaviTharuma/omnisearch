//! GitHub repository and code search.

use async_trait::async_trait;

use crate::config::Config;
use crate::error::Result;
use crate::http::HttpClient;
use crate::types::{ProviderId, ProviderSearchRequest, SearchPage, SearchType};

use super::{Provider, hit_from_value};

pub struct Github {
    http: HttpClient,
    token: Option<String>,
    base: String,
}

impl Github {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            token: config.keys.github.clone(),
            base: config.endpoints.github.clone(),
        }
    }
}

#[async_trait]
impl Provider for Github {
    fn id(&self) -> ProviderId {
        ProviderId::Github
    }
    fn is_configured(&self) -> bool {
        self.token.is_some()
    }
    fn skip_reason(&self) -> Option<String> {
        self.token.is_none().then(|| "GITHUB_TOKEN not set".into())
    }
    fn estimated_search_usd(&self) -> f64 {
        0.0
    }
    fn max_page_size(&self) -> u32 {
        100
    }
    fn notes(&self) -> &'static str {
        "Repo search by default; code search when search_type=code."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        let token = self
            .token
            .as_deref()
            .ok_or_else(|| crate::error::Error::NotConfigured {
                provider: "github".into(),
                reason: "GITHUB_TOKEN not set".into(),
            })?;
        let page = request
            .cursor
            .and_then(|c| c.parse::<u32>().ok())
            .unwrap_or(1);
        let path = if matches!(request.search_type, SearchType::Code) {
            "/search/code"
        } else {
            "/search/repositories"
        };
        let (_, value) = self
            .http
            .send_json(
                "github",
                self.http
                    .get(&format!("{}{path}", self.base))
                    .bearer_auth(token)
                    .header("Accept", "application/vnd.github+json")
                    .header("X-GitHub-Api-Version", "2022-11-28")
                    .query(&[
                        ("q", request.query),
                        ("per_page", &request.page_size.min(100).to_string()),
                        ("page", &page.to_string()),
                    ]),
            )
            .await?;
        let hits = value
            .get("items")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|v| {
                hit_from_value(
                    ProviderId::Github,
                    v,
                    &["html_url", "url"],
                    &["full_name", "name", "title"],
                    &["description"],
                    &["updated_at", "pushed_at"],
                    &["score"],
                )
            })
            .collect::<Vec<_>>();
        let next = if hits.len() as u32 >= request.page_size.min(100) {
            Some((page + 1).to_string())
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
