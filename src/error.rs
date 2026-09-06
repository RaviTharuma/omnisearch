//! Typed errors for provider calls, validation, and MCP mapping.

use rmcp::ErrorData;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid request: {0}")]
    Invalid(String),
    #[error("{provider} is not configured: {reason}")]
    NotConfigured { provider: String, reason: String },
    #[error("{provider}: {message}")]
    Provider { provider: String, message: String },
    #[error("{provider} rate limited: {message}")]
    RateLimited { provider: String, message: String },
    #[error("{provider} timed out after {seconds}s")]
    Timeout { provider: String, seconds: u64 },
    #[error("blocked URL: {0}")]
    Ssrf(String),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Error {
    pub fn provider(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Provider {
            provider: provider.into(),
            message: message.into(),
        }
    }

    pub fn rate_limited(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::RateLimited {
            provider: provider.into(),
            message: message.into(),
        }
    }

    pub fn to_mcp(&self) -> ErrorData {
        match self {
            Self::Invalid(message) => ErrorData::invalid_params(message.clone(), None),
            Self::Ssrf(message) => ErrorData::invalid_params(message.clone(), None),
            Self::NotConfigured { .. } => ErrorData::invalid_params(self.to_string(), None),
            _ => ErrorData::internal_error(self.to_string(), None),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
