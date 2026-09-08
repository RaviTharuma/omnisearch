//! OmniRoute's normalized POST /v1/search contract (not an OpenAI chat proxy).
use crate::{
    error::{Error, Result},
    http::HttpClient,
    providers::Provider,
    types::*,
};
use async_trait::async_trait;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayConnection {
    pub name: String,
    /// Origin, optionally ending in /v1 or /api/v1.
    pub base_url: String,
    pub api_key: String,
    /// OmniRoute registry ID or alias. Omit for its automatic routing.
    pub provider: Option<String>,
    #[serde(default = "web")]
    pub search_type: String,
    #[serde(default = "default_cost")]
    pub estimated_search_usd: f64,
}
fn default_cost() -> f64 {
    0.01
}
fn web() -> String {
    "web".into()
}
impl std::fmt::Debug for GatewayConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GatewayConnection([REDACTED])")
    }
}
impl GatewayConnection {
    pub fn endpoint(&self) -> Result<String> {
        let invalid = || Error::Invalid("invalid OmniRoute connection configuration".into());
        let u = url::Url::parse(&self.base_url).map_err(|_| invalid())?;
        if !self.estimated_search_usd.is_finite() || self.estimated_search_usd < 0.0 {
            return Err(Error::Invalid("invalid gateway cost estimate".into()));
        }
        if self.name.trim().is_empty()
            || self.api_key.trim().is_empty()
            || !matches!(u.scheme(), "http" | "https")
            || u.host_str().is_none()
            || !u.username().is_empty()
            || u.password().is_some()
            || u.query().is_some()
            || u.fragment().is_some()
            || !matches!(self.search_type.as_str(), "web" | "news" | "x")
            || self.provider.as_ref().is_some_and(|p| p.trim().is_empty())
        {
            return Err(invalid());
        }
        reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.api_key))
            .map_err(|_| invalid())?;
        let base = self.base_url.trim_end_matches('/');
        if base.ends_with("/v1") {
            Ok(format!("{base}/search"))
        } else {
            Ok(format!("{base}/v1/search"))
        }
    }
}
pub fn parse_connections(raw: &str) -> Result<Vec<GatewayConnection>> {
    let connections: Vec<GatewayConnection> = serde_json::from_str(raw).map_err(|_| {
        Error::Invalid("OMNISEARCH_OMNIROUTE_GATEWAYS must be a valid connection array".into())
    })?;
    let mut names = std::collections::HashSet::new();
    for c in &connections {
        c.endpoint()?;
        if !names.insert(&c.name) {
            return Err(Error::Invalid("duplicate OmniRoute connection name".into()));
        }
    }
    Ok(connections)
}
pub struct OmniRoute {
    http: HttpClient,
    connection: GatewayConnection,
    endpoint: String,
}
impl OmniRoute {
    pub fn new(http: HttpClient, connection: GatewayConnection) -> Result<Self> {
        let endpoint = connection.endpoint()?;
        Ok(Self {
            http,
            connection,
            endpoint,
        })
    }
}
#[derive(Deserialize)]
struct Response {
    results: Vec<Item>,
}
#[derive(Deserialize)]
struct Item {
    title: String,
    url: String,
    #[serde(default)]
    snippet: Option<String>,
    #[serde(default)]
    published_date: Option<String>,
}
#[async_trait]
impl Provider for OmniRoute {
    fn id(&self) -> ProviderId {
        ProviderId::Omniroute
    }
    fn is_configured(&self) -> bool {
        true
    }
    fn supports_search(&self) -> bool {
        true
    }
    // Pricing depends on gateway routing; zero means unknown, not free.
    fn estimated_search_usd(&self) -> f64 {
        self.connection.estimated_search_usd
    }
    async fn search(&self, req: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        if req.query.trim().is_empty() || req.query.chars().count() > 500 {
            return Err(Error::Invalid(
                "OmniRoute query must contain 1–500 characters".into(),
            ));
        }
        if req.cursor.is_some() {
            return Ok(SearchPage::default());
        }
        let mut body = serde_json::json!({"query":req.query,"max_results":req.page_size.clamp(1,20),"search_type":self.connection.search_type});
        if let Some(provider) = &self.connection.provider {
            body["provider"] = provider.clone().into();
        }
        let request = self
            .http
            .post(&self.endpoint)
            .bearer_auth(&self.connection.api_key)
            .json(&body);
        // Never propagate upstream response bodies, headers or reqwest URLs.
        let response = request.send().await.map_err(|_| Error::Provider {
            provider: self.id().to_string(),
            message: "gateway transport failure".into(),
        })?;
        let status = response.status();
        if !status.is_success() {
            let message = format!("gateway HTTP {}", status.as_u16());
            return Err(if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                Error::rate_limited("omniroute", message)
            } else {
                Error::provider("omniroute", message)
            });
        }
        let bytes = response.bytes().await.map_err(|_| Error::Provider {
            provider: self.id().to_string(),
            message: "gateway response read failed".into(),
        })?;
        let parsed: Response = serde_json::from_slice(&bytes).map_err(|_| Error::Provider {
            provider: self.id().to_string(),
            message: "invalid gateway search response".into(),
        })?;
        let hits = parsed
            .results
            .into_iter()
            .take(req.page_size.min(20) as usize)
            .map(|item| {
                let mut hit = SearchHit::new(
                    self.id(),
                    item.title,
                    item.url,
                    item.snippet.unwrap_or_default(),
                );
                hit.published_at = item.published_date;
                hit
            })
            .collect();
        Ok(SearchPage {
            hits,
            next_cursor: None,
            answer: None,
        })
    }
}
