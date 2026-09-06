//! Typed errors for provider calls, validation, and MCP mapping.

use rmcp::ErrorData;
use thiserror::Error;

/// Application error.
#[derive(Debug, Error)]
pub enum Error {
    /// Caller sent an unusable request.
    #[error("invalid request: {0}")]
    Invalid(String),
    /// Provider is not configured in this process.
    #[error("{provider} is not configured: {reason}")]
    NotConfigured { provider: String, reason: String },
    /// HTTP or upstream provider failure.
    #[error("{provider}: {message}")]
    Provider { provider: String, message: String },
    /// Provider responded 429 or asked us to back off.
    #[error("{provider} rate limited: {message}")]
    RateLimited { provider: String, message: String },
    /// Per-provider or global timeout.
    #[error("{provider} timed out after {seconds}s")]
    Timeout { provider: String, seconds: u64 },
    /// Requested URL failed SSRF checks.
    #[error("blocked URL: {0}")]
    Ssrf(String),
    /// HTTP client error.
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    /// JSON codec error.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Error {
    /// Build a provider-scoped error.
    pub fn provider(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Provider {
            provider: provider.into(),
            message: message.into(),
        }
    }

    /// Build a rate-limit error.
    pub fn rate_limited(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::RateLimited {
            provider: provider.into(),
            message: message.into(),
        }
    }

    /// Map to an MCP tool error.
    pub fn to_mcp(&self) -> ErrorData {
        match self {
            Self::Invalid(message) => ErrorData::invalid_params(message.clone(), None),
            Self::Ssrf(message) => ErrorData::invalid_params(message.clone(), None),
            Self::NotConfigured { .. } => ErrorData::invalid_params(self.to_string(), None),
            _ => ErrorData::internal_error(self.to_string(), None),
        }
    }
}

/// Convenience result alias.
pub type Result<T> = std::result::Result<T, Error>;
