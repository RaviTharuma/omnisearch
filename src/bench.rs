//! Provider latency bench.

use serde::Serialize;
use std::time::Instant;

use crate::orchestrator::AppState;
use crate::types::{ProviderSearchRequest, SearchType};

#[derive(Debug, Serialize)]
pub struct BenchRow {
    pub provider: String,
    pub configured: bool,
    pub ok: bool,
    pub latency_ms: u64,
    pub results: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn run_bench(state: &AppState, query: &str) -> Vec<BenchRow> {
    let mut rows = Vec::new();
    for info in state.registry.infos() {
        let Some(id) = crate::types::ProviderId::parse(&info.id).ok() else {
            continue;
        };
        let Some(provider) = state.registry.get(id) else {
            continue;
        };
        if !info.configured {
            rows.push(BenchRow {
                provider: info.id,
                configured: false,
                ok: false,
                latency_ms: 0,
                results: 0,
                error: Some("not configured".into()),
            });
            continue;
        }
        let started = Instant::now();
        let req = ProviderSearchRequest {
            query,
            cursor: None,
            page_size: 5,
            search_type: SearchType::Web,
            freshness: None,
            country: &state.config.country,
            language: &state.config.language,
        };
        match provider.search(&req).await {
            Ok(page) => rows.push(BenchRow {
                provider: info.id,
                configured: true,
                ok: true,
                latency_ms: started.elapsed().as_millis() as u64,
                results: page.hits.len() as u32,
                error: None,
            }),
            Err(err) => rows.push(BenchRow {
                provider: info.id,
                configured: true,
                ok: false,
                latency_ms: started.elapsed().as_millis() as u64,
                results: 0,
                error: Some(err.to_string()),
            }),
        }
    }
    rows
}
