//! First-class GitHub Search: repositories, code, and users.

use async_trait::async_trait;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, pick_f64, pick_str};
use crate::types::{ProviderId, ProviderSearchRequest, SearchHit, SearchPage, SearchType};

use super::Provider;

const MISSING_KEY: &str = "GITHUB_TOKEN or GITHUB_API_KEY not set";

pub struct Github {
    http: HttpClient,
    tokens: Vec<String>,
    base: String,
}

impl Github {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            tokens: config.keys.github.clone(),
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
        !self.tokens.is_empty()
    }
    fn skip_reason(&self) -> Option<String> {
        self.tokens.is_empty().then(|| MISSING_KEY.into())
    }
    fn estimated_search_usd(&self) -> f64 {
        0.0
    }
    fn max_page_size(&self) -> u32 {
        100
    }
    fn notes(&self) -> &'static str {
        "First-class. Default fan-out uses repository search. Code: search_type/kind=code. Users: search_type/kind=users. Keys: GITHUB_TOKEN or GITHUB_API_KEY (GH_TOKEN alias)."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        crate::try_keys!(&self.tokens, "github", MISSING_KEY, |token| self
            .search_with(token, request)
            .await,)
    }
}

impl Github {
    async fn search_with(
        &self,
        token: &str,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
        let page = request
            .cursor
            .and_then(|c| c.parse::<u32>().ok())
            .unwrap_or(1);
        let kind = github_kind(request.search_type);
        let accept = if kind == GithubKind::Code {
            "application/vnd.github.text-match+json"
        } else {
            "application/vnd.github+json"
        };
        let (_, value) = self
            .http
            .send_json(
                "github",
                self.http
                    .get(&format!("{}{}", self.base, kind.path()))
                    .bearer_auth(token)
                    .header("Accept", accept)
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
            .filter_map(|v| map_hit(kind, v))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GithubKind {
    Repos,
    Code,
    Users,
}

impl GithubKind {
    fn path(self) -> &'static str {
        match self {
            Self::Repos => "/search/repositories",
            Self::Code => "/search/code",
            Self::Users => "/search/users",
        }
    }
}

/// Map a vertical onto a GitHub Search endpoint.
fn github_kind(search_type: SearchType) -> GithubKind {
    match search_type {
        SearchType::Code => GithubKind::Code,
        SearchType::Users => GithubKind::Users,
        _ => GithubKind::Repos,
    }
}

/// Parse `github_search` kind / search_type tokens.
pub fn parse_kind(kind: Option<&str>) -> SearchType {
    match kind {
        Some(raw) => SearchType::parse(raw).unwrap_or(SearchType::Web),
        None => SearchType::Web,
    }
}

fn map_hit(kind: GithubKind, value: &serde_json::Value) -> Option<SearchHit> {
    match kind {
        GithubKind::Repos => {
            let url = pick_str(value, &["html_url", "url"])?;
            let title =
                pick_str(value, &["full_name", "name", "title"]).unwrap_or_else(|| url.clone());
            let mut hit = SearchHit::new(
                ProviderId::Github,
                title,
                url,
                pick_str(value, &["description"]).unwrap_or_default(),
            );
            hit.published_at = pick_str(value, &["updated_at", "pushed_at"]);
            hit.score = pick_f64(value, &["score"]);
            Some(hit)
        }
        GithubKind::Code => {
            let url = pick_str(value, &["html_url", "url"])?;
            let path = pick_str(value, &["path", "name"]).unwrap_or_else(|| url.clone());
            let repo = value
                .pointer("/repository/full_name")
                .and_then(|v| v.as_str())
                .unwrap_or("code");
            let snippet = first_text_match(value)
                .or_else(|| pick_str(value, &["path"]))
                .unwrap_or_default();
            let mut hit =
                SearchHit::new(ProviderId::Github, format!("{repo}: {path}"), url, snippet);
            hit.published_at = value
                .pointer("/repository/updated_at")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned);
            hit.score = pick_f64(value, &["score"]);
            Some(hit)
        }
        GithubKind::Users => {
            let login = pick_str(value, &["login", "name"])?;
            let url = pick_str(value, &["html_url"])
                .unwrap_or_else(|| format!("https://github.com/{login}"));
            let kind_label = pick_str(value, &["type"]).unwrap_or_else(|| "User".into());
            let snippet = pick_str(value, &["bio"]).unwrap_or(kind_label);
            let mut hit = SearchHit::new(ProviderId::Github, login, url, snippet);
            hit.score = pick_f64(value, &["score"]);
            Some(hit)
        }
    }
}

fn first_text_match(value: &serde_json::Value) -> Option<String> {
    value
        .get("text_matches")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .find_map(|m| pick_str(m, &["fragment"]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::GITHUB_KEY_NAMES;
    use serde_json::json;

    #[test]
    fn kind_paths() {
        assert_eq!(github_kind(SearchType::Web).path(), "/search/repositories");
        assert_eq!(github_kind(SearchType::Code).path(), "/search/code");
        assert_eq!(github_kind(SearchType::Users).path(), "/search/users");
        assert_eq!(parse_kind(Some("code")), SearchType::Code);
        assert_eq!(parse_kind(Some("users")), SearchType::Users);
        assert_eq!(parse_kind(Some("repo")), SearchType::Web);
        assert_eq!(parse_kind(None), SearchType::Web);
    }

    #[test]
    fn maps_repo_code_and_user_hits() {
        let repo = map_hit(
            GithubKind::Repos,
            &json!({
                "full_name": "RaviTharuma/omnisearch",
                "html_url": "https://github.com/RaviTharuma/omnisearch",
                "description": "Parallel search MCP",
                "score": 1.0
            }),
        )
        .unwrap();
        assert_eq!(repo.title, "RaviTharuma/omnisearch");

        let code = map_hit(
            GithubKind::Code,
            &json!({
                "path": "src/lib.rs",
                "html_url": "https://github.com/RaviTharuma/omnisearch/blob/main/src/lib.rs",
                "repository": {"full_name": "RaviTharuma/omnisearch"},
                "text_matches": [{"fragment": "pub struct AppState"}]
            }),
        )
        .unwrap();
        assert_eq!(code.title, "RaviTharuma/omnisearch: src/lib.rs");
        assert!(code.snippet.contains("AppState"));

        let user = map_hit(
            GithubKind::Users,
            &json!({
                "login": "octocat",
                "html_url": "https://github.com/octocat",
                "type": "User",
                "score": 42.0
            }),
        )
        .unwrap();
        assert_eq!(user.title, "octocat");
        assert_eq!(user.url, "https://github.com/octocat");
        assert_eq!(user.score, Some(42.0));
    }

    #[test]
    fn documents_github_key_aliases() {
        assert!(GITHUB_KEY_NAMES.contains(&"GITHUB_TOKEN"));
        assert!(GITHUB_KEY_NAMES.contains(&"GITHUB_API_KEY"));
    }
}
