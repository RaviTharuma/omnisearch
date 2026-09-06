//! Streamable HTTP transport with bearer tokens and a simple RPM limiter.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use dashmap::DashMap;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::orchestrator::AppState;
use crate::tools::OmniServer;

#[derive(Clone)]
struct AuthState {
    tokens: Vec<String>,
    rpm: u32,
    hits: Arc<DashMap<String, Vec<Instant>>>,
}

/// Serve MCP over streamable HTTP.
pub async fn serve_http(state: Arc<AppState>) -> anyhow::Result<()> {
    let bind: SocketAddr = state
        .config
        .http_bind
        .parse()
        .unwrap_or_else(|_| "127.0.0.1:48731".parse().unwrap());
    let tokens = state.config.auth_tokens.clone();
    if tokens.is_empty() {
        warn!("AUTH_TOKENS is empty; HTTP transport accepts unauthenticated requests");
    }
    let auth = AuthState {
        rpm: state.config.http_rpm,
        tokens,
        hits: Arc::new(DashMap::new()),
    };
    let inner = state.clone();
    let service: StreamableHttpService<OmniServer, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(OmniServer::new(inner.clone())),
            Default::default(),
            StreamableHttpServerConfig::default().with_cancellation_token(CancellationToken::new()),
        );
    let app = Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(auth, enforce_auth));
    info!(%bind, "HTTP MCP listening");
    let listener = tokio::net::TcpListener::bind(bind).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn enforce_auth(
    State(auth): State<AuthState>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let presented = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .strip_prefix("Bearer ")
        .unwrap_or("")
        .to_string();
    if !auth.tokens.is_empty() && !auth.tokens.iter().any(|t| t == &presented) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let bucket = if presented.is_empty() {
        "anonymous".to_string()
    } else {
        presented
    };
    if !allow(&auth.hits, &bucket, auth.rpm) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    Ok(next.run(request).await)
}

fn allow(hits: &DashMap<String, Vec<Instant>>, key: &str, rpm: u32) -> bool {
    let now = Instant::now();
    let window = Duration::from_secs(60);
    let mut entry = hits.entry(key.to_string()).or_default();
    entry.retain(|t| now.duration_since(*t) < window);
    if entry.len() as u32 >= rpm {
        return false;
    }
    entry.push(now);
    true
}
