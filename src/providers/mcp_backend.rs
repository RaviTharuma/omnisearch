//! Optional downstream MCP/JSON search backends.

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::config::{Config, McpBackend};
use crate::error::{Error, Result};
use crate::http::{HttpClient, result_array};
use crate::types::{ProviderId, ProviderSearchRequest, SearchPage};

use super::{Provider, hit_from_value};

pub struct McpBackends {
    http: HttpClient,
    backends: Vec<McpBackend>,
}

impl McpBackends {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            backends: config.mcp_backends.clone(),
        }
    }
}

#[async_trait]
impl Provider for McpBackends {
    fn id(&self) -> ProviderId {
        ProviderId::McpBackend
    }
    fn is_configured(&self) -> bool {
        !self.backends.is_empty()
    }
    fn skip_reason(&self) -> Option<String> {
        self.backends
            .is_empty()
            .then(|| "OMNISEARCH_MCP_BACKENDS not set".into())
    }
    fn estimated_search_usd(&self) -> f64 {
        0.0
    }
    fn requires_key(&self) -> bool {
        false
    }
    fn notes(&self) -> &'static str {
        "Proxies OMNISEARCH_MCP_BACKENDS (name|url|token). Tries MCP tools/call then a JSON {query} POST."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        if self.backends.is_empty() {
            return Err(Error::NotConfigured {
                provider: "mcp_backend".into(),
                reason: "no backends".into(),
            });
        }
        let mut hits = Vec::new();
        let mut last_err = None;
        for backend in &self.backends {
            match self.call_backend(backend, request).await {
                Ok(mut page) => {
                    for hit in &mut page.hits {
                        hit.provider = format!("mcp:{}", backend.name);
                        if !hit.sources.iter().any(|s| s == &hit.provider) {
                            hit.sources.push(hit.provider.clone());
                        }
                    }
                    hits.extend(page.hits);
                }
                Err(err) => last_err = Some(err),
            }
        }
        if hits.is_empty()
            && let Some(err) = last_err
        {
            return Err(err);
        }
        Ok(SearchPage {
            hits,
            next_cursor: None,
            answer: None,
        })
    }
}

impl McpBackends {
    async fn call_backend(
        &self,
        backend: &McpBackend,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
        let mcp_body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "search",
                "arguments": { "query": request.query }
            }
        });
        let mut builder = self.http.post(&backend.url).json(&mcp_body);
        if let Some(token) = &backend.token {
            builder = builder.bearer_auth(token);
        }
        if let Ok(value) = self.http.json("mcp_backend", builder).await
            && let Some(page) = parse_backend_value(&value)
        {
            return Ok(page);
        }
        let mut builder = self
            .http
            .post(&backend.url)
            .json(&json!({ "query": request.query, "limit": request.page_size }));
        if let Some(token) = &backend.token {
            builder = builder.bearer_auth(token);
        }
        let value = self.http.json("mcp_backend", builder).await?;
        parse_backend_value(&value)
            .ok_or_else(|| Error::provider("mcp_backend", "backend returned no results"))
    }
}

fn parse_backend_value(value: &Value) -> Option<SearchPage> {
    let payload = value
        .pointer("/result/structuredContent")
        .or_else(|| value.pointer("/result"))
        .unwrap_or(value);
    let rows = result_array(payload);
    if rows.is_empty() {
        return None;
    }
    let hits = rows
        .iter()
        .filter_map(|v| {
            hit_from_value(
                ProviderId::McpBackend,
                v,
                &["url"],
                &["title"],
                &["snippet", "content"],
                &["published_at"],
                &["score"],
            )
        })
        .collect::<Vec<_>>();
    Some(SearchPage {
        hits,
        next_cursor: None,
        answer: None,
    })
}
