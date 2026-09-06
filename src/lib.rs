//! Omnisearch: a unified multi-provider MCP search server.

pub mod bench;
pub mod cache;
pub mod config;
pub mod error;
pub mod health;
pub mod http;
pub mod http_server;
pub mod intent;
pub mod merge;
pub mod orchestrator;
pub mod providers;
pub mod ssrf;
pub mod tools;
pub mod types;
pub mod urlutil;

pub use orchestrator::AppState;
pub use tools::OmniServer;
pub use types::{ProviderId, SearchRequest, SearchResponse};
