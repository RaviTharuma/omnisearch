//! Parallel fan-out, RRF merge, unlimited pagination, and partial-success tests.

use std::time::{Duration, Instant};

use omnisearch::config::Config;
use omnisearch::orchestrator::{AppState, search};
use omnisearch::types::{ProviderId, SearchMode, SearchRequest};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn base_config(tavily: &str, exa: &str) -> Config {
    let mut config = Config::from_env();
    config.endpoints.tavily = tavily.to_string();
    config.endpoints.exa = exa.to_string();
    config.endpoints.wikipedia = "http://127.0.0.1:9".into();
    config.endpoints.scholar = "http://127.0.0.1:9".into();
    config.endpoints.bluesky = "http://127.0.0.1:9".into();
    config.keys.tavily = Some("tavily-test".into());
    config.keys.exa = Some("exa-test".into());
    config.cache_ttl_secs = 60;
    config.auto_allow_usd = 1.0;
    config.default_timeout_secs = 5;
    config
}

#[tokio::test]
async fn parallel_fanout_merges_and_dedupes() {
    let tavily = MockServer::start().await;
    let exa = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(180))
                .set_body_json(json!({
                    "results": [
                        {"title":"Shared","url":"https://example.com/a","content":"from tavily","score":0.9},
                        {"title":"Only Tavily","url":"https://example.com/t","content":"t-only"}
                    ]
                })),
        )
        .mount(&tavily)
        .await;

    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(180))
                .set_body_json(json!({
                    "results": [
                        {"title":"Shared","url":"https://example.com/a?utm_source=x","text":"from exa"},
                        {"title":"Only Exa","url":"https://example.com/e","text":"e-only"}
                    ]
                })),
        )
        .mount(&exa)
        .await;

    let state = AppState::new(base_config(&tavily.uri(), &exa.uri())).unwrap();
    let mut req = SearchRequest::new("parallel rust search");
    req.providers = Some(vec![ProviderId::Tavily, ProviderId::Exa]);
    req.no_cache = true;
    let started = Instant::now();
    let out = search(&state, req).await;
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_millis(320),
        "expected concurrent fan-out, took {elapsed:?}"
    );
    assert_eq!(out.results.len(), 3);
    let shared = out
        .results
        .iter()
        .find(|h| h.url.contains("/a"))
        .expect("shared url");
    assert!(shared.sources.contains(&"tavily".into()));
    assert!(shared.sources.contains(&"exa".into()));
    assert_eq!(out.meta.successful.len(), 2);
    assert!(out.meta.failed.is_empty());
}

#[tokio::test]
async fn unlimited_pages_until_exhausted() {
    let tavily = MockServer::start().await;
    let exa = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": (0..15).map(|i| json!({
                "title": format!("Page item {i}"),
                "url": format!("https://example.com/p/{i}"),
                "content": "x"
            })).collect::<Vec<_>>()
        })))
        .mount(&tavily)
        .await;

    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"results": []})))
        .mount(&exa)
        .await;

    let state = AppState::new(base_config(&tavily.uri(), &exa.uri())).unwrap();
    let mut req = SearchRequest::new("unlimited aggregation");
    req.providers = Some(vec![ProviderId::Tavily, ProviderId::Exa]);
    req.limit = None;
    req.unlimited = true;
    req.no_cache = true;
    let out = search(&state, req).await;
    assert_eq!(out.unique_count, 15);
    assert!(out.unique_count > 10);
}

#[tokio::test]
async fn limit_is_soft_and_aggregates_across_providers() {
    let tavily = MockServer::start().await;
    let exa = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [
                {"title":"T1","url":"https://example.com/t1","content":"a"},
                {"title":"T2","url":"https://example.com/t2","content":"b"}
            ]
        })))
        .mount(&tavily)
        .await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [
                {"title":"E1","url":"https://example.com/e1","text":"c"},
                {"title":"E2","url":"https://example.com/e2","text":"d"}
            ]
        })))
        .mount(&exa)
        .await;

    let state = AppState::new(base_config(&tavily.uri(), &exa.uri())).unwrap();
    let mut req = SearchRequest::new("soft limit");
    req.providers = Some(vec![ProviderId::Tavily, ProviderId::Exa]);
    req.limit = Some(2);
    req.unlimited = false;
    req.no_cache = true;
    let out = search(&state, req).await;
    assert_eq!(
        out.results.len(),
        4,
        "soft limit must aggregate, not cap at 2"
    );
}

#[tokio::test]
async fn partial_success_when_one_provider_fails() {
    let tavily = MockServer::start().await;
    let exa = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [{"title":"ok","url":"https://example.com/ok","content":"x"}]
        })))
        .mount(&tavily)
        .await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(500).set_body_string("nope"))
        .mount(&exa)
        .await;

    let state = AppState::new(base_config(&tavily.uri(), &exa.uri())).unwrap();
    let mut req = SearchRequest::new("partial");
    req.providers = Some(vec![ProviderId::Tavily, ProviderId::Exa]);
    req.mode = SearchMode::All;
    req.no_cache = true;
    let out = search(&state, req).await;
    assert_eq!(out.meta.successful, vec!["tavily".to_string()]);
    assert_eq!(out.meta.failed.len(), 1);
    assert_eq!(out.meta.failed[0].provider, "exa");
    assert_eq!(out.results.len(), 1);
}

#[tokio::test]
async fn cache_hit_on_second_call() {
    let tavily = MockServer::start().await;
    let exa = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [{"title":"c","url":"https://example.com/c","content":"x"}]
        })))
        .expect(1)
        .mount(&tavily)
        .await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"results":[]})))
        .expect(1)
        .mount(&exa)
        .await;

    let state = AppState::new(base_config(&tavily.uri(), &exa.uri())).unwrap();
    let mut req = SearchRequest::new("cache me");
    req.providers = Some(vec![ProviderId::Tavily, ProviderId::Exa]);
    let first = search(&state, req.clone()).await;
    let second = search(&state, req).await;
    assert!(!first.meta.cache_hit);
    assert!(second.meta.cache_hit);
}
