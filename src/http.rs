//! Shared HTTP client and status mapping.

use std::time::Duration;

use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde_json::Value;

use crate::config::Config;
use crate::error::{Error, Result};

/// Thin wrapper around reqwest with a consistent User-Agent and timeout.
#[derive(Clone)]
pub struct HttpClient {
    inner: Client,
}

impl HttpClient {
    /// Build a client from process config.
    pub fn new(config: &Config) -> Result<Self> {
        let inner = Client::builder()
            .user_agent(&config.user_agent)
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .redirect(reqwest::redirect::Policy::limited(4))
            .build()?;
        Ok(Self { inner })
    }

    /// Start a request.
    pub fn request(&self, method: Method, url: &str) -> RequestBuilder {
        self.inner.request(method, url)
    }

    /// Convenience GET.
    pub fn get(&self, url: &str) -> RequestBuilder {
        self.inner.get(url)
    }

    /// Convenience POST.
    pub fn post(&self, url: &str) -> RequestBuilder {
        self.inner.post(url)
    }

    /// Send a request and map HTTP failures to typed errors.
    pub async fn send_json(
        &self,
        provider: &str,
        builder: RequestBuilder,
    ) -> Result<(StatusCode, Value)> {
        let response = builder
            .send()
            .await
            .map_err(|err| Error::provider(provider, format!("transport error: {err}")))?;
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(Error::rate_limited(
                provider,
                if body.is_empty() {
                    "HTTP 429".into()
                } else {
                    truncate(&body, 400)
                },
            ));
        }
        if !status.is_success() {
            return Err(Error::provider(
                provider,
                format!("HTTP {status}: {}", truncate(&body, 400)),
            ));
        }
        if body.trim().is_empty() {
            return Ok((status, Value::Null));
        }
        let value = serde_json::from_str(&body).unwrap_or(Value::String(body));
        Ok((status, value))
    }
}

/// Truncate noisy upstream bodies for logs and errors.
fn truncate(input: &str, max: usize) -> String {
    let mut out: String = input.chars().take(max).collect();
    if input.chars().count() > max {
        out.push('…');
    }
    out
}

/// Read a string field from the first matching key.
pub fn pick_str(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(found) = value.get(*key) {
            if let Some(s) = found.as_str() {
                if !s.is_empty() {
                    return Some(s.to_string());
                }
            } else if let Some(n) = found.as_f64() {
                return Some(n.to_string());
            }
        }
    }
    None
}

/// Read an f64 from the first matching key.
pub fn pick_f64(value: &Value, keys: &[&str]) -> Option<f64> {
    for key in keys {
        if let Some(found) = value.get(*key) {
            if let Some(n) = found.as_f64() {
                return Some(n);
            }
            if let Some(s) = found.as_str()
                && let Ok(n) = s.parse()
            {
                return Some(n);
            }
        }
    }
    None
}

/// Walk common result-array locations.
pub fn result_array(value: &Value) -> Vec<Value> {
    const PATHS: &[&str] = &[
        "results",
        "data",
        "items",
        "hits",
        "web",
        "organic",
        "documents",
        "posts",
        "statuses",
    ];
    for path in PATHS {
        if let Some(arr) = value.get(*path).and_then(Value::as_array) {
            return arr.clone();
        }
    }
    if let Some(web) = value.pointer("/results/web").and_then(Value::as_array) {
        return web.clone();
    }
    if let Some(arr) = value.as_array() {
        return arr.clone();
    }
    Vec::new()
}
