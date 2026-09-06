//! Parallel fan-out, RRF merge, unlimited pagination, and partial-success tests.

use std::time::{Duration, Instant};

use omnisearch::config::Config;
use omnisearch::orchestrator::{AppState, search};
use omnisearch::types::{ProviderId, SearchMode, SearchRequest, SearchType};
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
    config.endpoints.reddit = "http://127.0.0.1:9".into();
    config.endpoints.reddit_oauth = "http://127.0.0.1:9".into();
    config.keys.tavily = vec!["tavily-test".into()];
    config.keys.exa = vec!["exa-test".into()];
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
    assert_eq!(first.meta.stop_reason.as_deref(), Some("complete"));
    assert!(!first.meta.provider_used.is_empty());
}

#[tokio::test]
async fn partial_success_is_not_cached() {
    let tavily = MockServer::start().await;
    let exa = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [{"title":"ok","url":"https://example.com/ok","content":"x"}]
        })))
        .expect(2)
        .mount(&tavily)
        .await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(500).set_body_string("nope"))
        .expect(2)
        .mount(&exa)
        .await;

    let state = AppState::new(base_config(&tavily.uri(), &exa.uri())).unwrap();
    let mut req = SearchRequest::new("no-cache-partial");
    req.providers = Some(vec![ProviderId::Tavily, ProviderId::Exa]);
    let first = search(&state, req.clone()).await;
    let second = search(&state, req).await;
    assert_eq!(
        first.meta.stop_reason.as_deref(),
        Some("partial_provider_failure")
    );
    assert!(!first.meta.cache_hit);
    assert!(!second.meta.cache_hit);
}

#[tokio::test]
async fn cache_partial_explicit_allow() {
    let tavily = MockServer::start().await;
    let exa = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [{"title":"ok","url":"https://example.com/ok","content":"x"}]
        })))
        .expect(1)
        .mount(&tavily)
        .await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(500).set_body_string("nope"))
        .expect(1)
        .mount(&exa)
        .await;

    let state = AppState::new(base_config(&tavily.uri(), &exa.uri())).unwrap();
    let mut req = SearchRequest::new("allow-partial-cache");
    req.providers = Some(vec![ProviderId::Tavily, ProviderId::Exa]);
    req.cache_partial = true;
    let first = search(&state, req.clone()).await;
    let second = search(&state, req).await;
    assert!(!first.meta.cache_hit);
    assert!(second.meta.cache_hit);
}

#[tokio::test]
async fn dual_key_failover_on_429() {
    let tavily = MockServer::start().await;
    let exa = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(429).set_body_string("slow"))
        .up_to_n_times(1)
        .mount(&tavily)
        .await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [{"title":"ok","url":"https://example.com/dual","content":"second key"}]
        })))
        .mount(&tavily)
        .await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"results":[]})))
        .mount(&exa)
        .await;

    let mut config = base_config(&tavily.uri(), &exa.uri());
    config.keys.tavily = vec!["bad-key".into(), "good-key".into()];
    let state = AppState::new(config).unwrap();
    let mut req = SearchRequest::new("dual key");
    req.providers = Some(vec![ProviderId::Tavily]);
    req.no_cache = true;
    let out = search(&state, req).await;
    assert_eq!(out.meta.successful, vec!["tavily".to_string()]);
    assert_eq!(out.results.len(), 1);
    assert!(out.meta.cost_usd > 0.0);
}

#[tokio::test]
async fn ladder_skips_paid_when_free_meets_evidence() {
    let tavily = MockServer::start().await;
    let exa = MockServer::start().await;
    let wiki = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/w/api.php"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "query": {"search": [
                {"title":"Rust","snippet":"systems language"}
            ]}
        })))
        .mount(&wiki)
        .await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [{"title":"paid","url":"https://example.com/paid","content":"no"}]
        })))
        .expect(0)
        .mount(&tavily)
        .await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"results":[]})))
        .expect(0)
        .mount(&exa)
        .await;

    let mut config = base_config(&tavily.uri(), &exa.uri());
    config.endpoints.wikipedia = wiki.uri();
    config.auto_allow_usd = 1.0;
    let state = AppState::new(config).unwrap();
    let mut req = SearchRequest::new("ladder rust");
    req.mode = SearchMode::Ladder;
    req.providers = None;
    req.evidence_min = Some(1);
    req.no_cache = true;
    let out = search(&state, req).await;
    assert_eq!(out.meta.stop_reason.as_deref(), Some("evidence"));
    assert!(out.meta.successful.contains(&"wikipedia".to_string()));
    assert!(
        !out.meta
            .successful
            .iter()
            .any(|p| p == "tavily" || p == "exa")
    );
    assert!(out.results.iter().any(|h| h.confidence.is_some()));
}

#[tokio::test]
async fn search_health_reports_requires_key_and_latency() {
    let tavily = MockServer::start().await;
    let exa = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [{"title":"ok","url":"https://example.com/h","content":"x"}]
        })))
        .mount(&tavily)
        .await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"results":[]})))
        .mount(&exa)
        .await;

    let state = AppState::new(base_config(&tavily.uri(), &exa.uri())).unwrap();
    let mut req = SearchRequest::new("health");
    req.providers = Some(vec![ProviderId::Tavily]);
    req.no_cache = true;
    let _ = search(&state, req).await;
    let health = state.search_health();
    let tavily = health.iter().find(|h| h.id == "tavily").expect("tavily");
    assert!(tavily.configured);
    assert!(tavily.requires_key);
    assert!(tavily.success >= 1);
    assert!(tavily.last_latency_ms.is_some());
    let wiki = health.iter().find(|h| h.id == "wikipedia").expect("wiki");
    assert!(!wiki.requires_key);
}

fn isolate_must_have(brave: &str, github: &str) -> Config {
    let mut config = Config::from_env();
    config.keys = Default::default();
    config.keys.brave = vec!["brave-test".into()];
    config.keys.github = vec!["github-test".into()];
    config.keenable_public = false;
    config.mcp_backends.clear();
    config.endpoints.brave = brave.to_string();
    config.endpoints.github = github.to_string();
    config.endpoints.tavily = "http://127.0.0.1:9".into();
    config.endpoints.exa = "http://127.0.0.1:9".into();
    config.endpoints.wikipedia = "http://127.0.0.1:9".into();
    config.endpoints.scholar = "http://127.0.0.1:9".into();
    config.endpoints.bluesky = "http://127.0.0.1:9".into();
    config.endpoints.reddit = "http://127.0.0.1:9".into();
    config.endpoints.reddit_oauth = "http://127.0.0.1:9".into();
    config.auto_allow_usd = 1.0;
    config.default_timeout_secs = 5;
    config.cache_ttl_secs = 60;
    config
}

#[tokio::test]
async fn default_fanout_includes_brave_and_github_when_keyed() {
    let brave = MockServer::start().await;
    let github = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/res/v1/web/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "web": {"results": [
                {"title":"Brave hit","url":"https://brave.example/a","description":"from brave"}
            ]}
        })))
        .mount(&brave)
        .await;

    Mock::given(method("GET"))
        .and(path("/search/repositories"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [{
                "full_name": "acme/widget",
                "html_url": "https://github.com/acme/widget",
                "description": "from github"
            }]
        })))
        .mount(&github)
        .await;

    let state = AppState::new(isolate_must_have(&brave.uri(), &github.uri())).unwrap();
    let mut req = SearchRequest::new("widget search");
    req.providers = None;
    req.mode = SearchMode::All;
    req.no_cache = true;
    let out = search(&state, req).await;

    assert!(
        out.meta.selected.contains(&"brave".into()),
        "brave must be selected in default fan-out: {:?}",
        out.meta.selected
    );
    assert!(
        out.meta.selected.contains(&"github".into()),
        "github must be selected in default fan-out: {:?}",
        out.meta.selected
    );
    assert!(out.meta.successful.contains(&"brave".into()));
    assert!(out.meta.successful.contains(&"github".into()));
    assert!(out.results.iter().any(|h| h.url.contains("brave.example")));
    assert!(
        out.results
            .iter()
            .any(|h| h.url.contains("github.com/acme/widget"))
    );
}

#[tokio::test]
async fn github_users_and_code_kinds() {
    let github = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search/users"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [{
                "login": "octocat",
                "html_url": "https://github.com/octocat",
                "type": "User",
                "score": 1.0
            }]
        })))
        .mount(&github)
        .await;
    Mock::given(method("GET"))
        .and(path("/search/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [{
                "path": "src/lib.rs",
                "html_url": "https://github.com/acme/widget/blob/main/src/lib.rs",
                "repository": {"full_name": "acme/widget"},
                "text_matches": [{"fragment": "pub fn search"}]
            }]
        })))
        .mount(&github)
        .await;

    let mut config = isolate_must_have("http://127.0.0.1:9", &github.uri());
    config.keys.brave.clear();
    let state = AppState::new(config).unwrap();

    let mut users = SearchRequest::new("octocat");
    users.providers = Some(vec![ProviderId::Github]);
    users.search_type = SearchType::Users;
    users.no_cache = true;
    let user_out = search(&state, users).await;
    assert_eq!(user_out.results[0].title, "octocat");
    assert_eq!(user_out.results[0].url, "https://github.com/octocat");

    let mut code = SearchRequest::new("pub fn search");
    code.providers = Some(vec![ProviderId::Github]);
    code.search_type = SearchType::Code;
    code.no_cache = true;
    let code_out = search(&state, code).await;
    assert_eq!(code_out.results[0].title, "acme/widget: src/lib.rs");
    assert!(code_out.results[0].snippet.contains("pub fn search"));
}
