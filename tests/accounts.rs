use accounts::wrap_providers;
use config::{Config, ProviderAccount, ProviderKeys};
use http::HttpClient;
use omnisearch::{accounts, config, error, http, providers, types};
use providers::Registry;
use types::{ProviderId, ProviderSearchRequest, SearchType};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};

fn config() -> Config {
    let mut c = Config::from_env();
    c.keys = ProviderKeys::default();
    c.accounts.clear();
    c.accounts_error = None;
    c
}
fn request() -> ProviderSearchRequest<'static> {
    ProviderSearchRequest {
        query: "test",
        cursor: None,
        page_size: 10,
        search_type: SearchType::Web,
        freshness: None,
        country: "US",
        language: "en",
        account: None,
        depth: None,
    }
}
fn parse(s: &str) -> Vec<ProviderAccount> {
    serde_json::from_str(s).unwrap()
}
fn wrapped(c: &Config, id: ProviderId) -> accounts::WrappedProviders {
    let http = HttpClient::new(c).unwrap();
    let mut legacy = c.clone();
    legacy.accounts.clear();
    wrap_providers(
        vec![Registry::new(&legacy, http.clone()).get(id).unwrap()],
        c,
        http,
    )
    .unwrap()
}

#[tokio::test]
async fn mixed_x_accounts_never_call_paid_fallback_without_auto_allowance() {
    use omnisearch::orchestrator::{AppState, search};
    use types::SearchRequest;

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/2/tweets/search/recent"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "output": []
        })))
        .expect(0)
        .mount(&server)
        .await;
    let mut c = config();
    c.gateways.clear();
    c.gateways_error = None;
    // This is the config value populated by OMNISEARCH_AUTO_ALLOW_USD=0.
    // Do not mutate process-wide environment in a concurrent test suite.
    c.auto_allow_usd = 0.0;
    c.accounts = parse(
        r#"[
        {"provider":"x","name":"bearer","credentials":{"bearer_token":"mock-bearer"}},
        {"provider":"x","name":"paid","credentials":{"xai_api_key":"mock-paid"}}
    ]"#,
    );
    c.endpoints.x = server.uri();
    c.endpoints.xai = server.uri();
    c.endpoints.wikipedia = server.uri();
    c.endpoints.scholar = server.uri();
    c.endpoints.bluesky = server.uri();
    c.endpoints.reddit = server.uri();
    c.endpoints.reddit_oauth = server.uri();
    for _ in 0..2 {
        let state = AppState::new(c.clone()).unwrap();
        let result = search(&state, SearchRequest::new("mixed account admission")).await;
        assert!(
            server
                .received_requests()
                .await
                .unwrap()
                .iter()
                .all(|request| request.url.path() != "/v1/responses"),
            "automatic admission must never execute a paid named account"
        );
        assert!(result.meta.skipped.iter().any(|skip| skip.provider == "x"));
        c.accounts.reverse();
    }
}

#[test]
fn validates_typed_credentials_and_redacts_secrets() {
    let mut c = config();
    c.accounts = parse(
        r#"[{"provider":"reddit","name":"work","credentials":{"client_id":"id-secret","client_secret":"secret-value"}},{"provider":"instagram","name":"work","credentials":{"token":"token-secret","user_id":"user-secret"}}]"#,
    );
    c.validate_accounts().unwrap();
    let debug = format!("{c:?}");
    for secret in ["id-secret", "secret-value", "token-secret", "user-secret"] {
        assert!(!debug.contains(secret));
    }
    c.accounts =
        parse(r#"[{"provider":"reddit","name":"work","credentials":{"api_key":"wrong-secret"}}]"#);
    assert!(c.validate_accounts().is_err());
}

#[test]
fn rejects_invalid_duplicate_and_unknown_account_settings() {
    let mut c = config();
    for raw in [
        "invalid-secret",
        r#"[{"provider":"brave","name":"a","credentials":{"api_key":""}}]"#,
        r#"[{"provider":"unknown","name":"a","credentials":{"api_key":"secret"}}]"#,
        r#"[{"provider":"brave","name":"a","credentials":{"api_key":"secret","typo":"secret"}}]"#,
        r#"[{"provider":"brave","name":"a","credentials":{"api_key":"secret"}},{"provider":"brave","name":"a","credentials":{"api_key":"other"}}]"#,
    ] {
        c.set_accounts_json(raw);
        let error = c.validate_accounts().unwrap_err().to_string();
        assert!(!error.contains("secret"));
        assert!(wrapped_result(&c).is_err());
    }
}
fn wrapped_result(c: &Config) -> error::Result<accounts::WrappedProviders> {
    wrap_providers(vec![], c, HttpClient::new(c).unwrap())
}

#[test]
fn supports_special_and_public_providers_without_credential_mixing() {
    let cases = [
        (
            "reddit",
            serde_json::json!({"client_id":"id", "client_secret":"secret"}),
        ),
        (
            "instagram",
            serde_json::json!({"token":"secret", "user_id":"id"}),
        ),
        ("facebook", serde_json::json!({"token":"secret"})),
        ("x", serde_json::json!({"bearer_token":"secret"})),
        ("x", serde_json::json!({"xai_api_key":"secret"})),
        (
            "mastodon",
            serde_json::json!({"instance":"https://example.com", "token":"secret"}),
        ),
        ("wikipedia", serde_json::json!({})),
        ("scholar", serde_json::json!({})),
        ("bluesky", serde_json::json!({})),
        (
            "mcp_backend",
            serde_json::json!({"url":"https://example.com/mcp", "token":"secret"}),
        ),
    ];
    for (provider, credentials) in cases {
        let mut c = config();
        c.set_accounts_json(
            &serde_json::json!([{"provider":provider,"name":"work","credentials":credentials}])
                .to_string(),
        );
        c.validate_accounts().unwrap();
        let mut isolated = c.clone();
        isolated.keys.x_bearer = vec!["legacy".into()];
        c.accounts[0].apply(&mut isolated).unwrap();
        assert!(isolated.accounts.is_empty());
        assert!(!format!("{:?}", isolated.keys).contains("legacy"));
        assert!(!isolated.keys.x_bearer.contains(&"legacy".to_owned()));
        assert!(
            Registry::new(&isolated, HttpClient::new(&isolated).unwrap())
                .get(c.accounts[0].provider)
                .unwrap()
                .is_configured()
        );
    }
}

#[test]
fn config_debug_redacts_http_auth_tokens() {
    let mut c = config();
    c.auth_tokens = vec!["private-auth-secret".into()];
    assert!(!format!("{c:?}").contains("private-auth-secret"));
}

#[test]
fn named_mcp_is_added_when_legacy_registry_has_no_backend() {
    let mut c = config();
    c.set_accounts_json(r#"[{"provider":"mcp_backend","name":"remote","credentials":{"url":"https://example.com/mcp"}}]"#);
    let w = wrap_providers(Vec::new(), &c, HttpClient::new(&c).unwrap()).unwrap();
    assert_eq!(w.providers.len(), 1);
    assert_eq!(w.providers[0].id(), ProviderId::McpBackend);
}

#[tokio::test]
async fn extraction_fails_over_and_shares_account_health() {
    let server = MockServer::start().await;
    Mock::given(path("/extract"))
        .and(header("authorization", "Bearer first"))
        .respond_with(ResponseTemplate::new(401).set_body_string("first"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/extract")).and(header("authorization", "Bearer second"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"results":[{"url":"https://example.com", "raw_content":"document"}]}))).expect(1).mount(&server).await;
    let mut c = config();
    c.endpoints.tavily = server.uri();
    c.set_accounts_json(r#"[{"provider":"tavily","name":"one","credentials":{"api_key":"first"}},{"provider":"tavily","name":"two","credentials":{"api_key":"second"}}]"#);
    let w = wrapped(&c, ProviderId::Tavily);
    assert!(w.providers[0].supports_extract());
    let docs = w.providers[0]
        .extract(&["https://example.com".into()], None)
        .await
        .unwrap();
    assert_eq!(docs[0].content, "document");
    assert_eq!(w.health.snapshot()[0].failure_count, 1);
    assert_eq!(w.health.snapshot()[1].success_count, 1);
}

#[tokio::test]
async fn exhausted_accounts_do_not_retry_or_leak_and_zero_cooldown_recovers() {
    let server = MockServer::start().await;
    Mock::given(path("/res/v1/web/search"))
        .respond_with(ResponseTemplate::new(429).set_body_string("private-secret"))
        .expect(1)
        .mount(&server)
        .await;
    let mut c = config();
    c.endpoints.brave = server.uri();
    c.set_accounts_json(
        r#"[{"provider":"brave","name":"one","credentials":{"api_key":"private-secret"}}]"#,
    );
    let w = wrapped(&c, ProviderId::Brave);
    assert!(
        !w.providers[0]
            .search(&request())
            .await
            .unwrap_err()
            .to_string()
            .contains("private-secret")
    );
    assert!(w.providers[0].search(&request()).await.is_err());
    assert_eq!(w.health.snapshot()[0].failure_count, 1);
    server.reset().await;
    Mock::given(path("/res/v1/web/search"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"web":{"results":[]}})),
        )
        .expect(2)
        .mount(&server)
        .await;
    c.cooldown_secs = 0;
    let w = wrapped(&c, ProviderId::Brave);
    w.providers[0].search(&request()).await.unwrap();
    w.providers[0].search(&request()).await.unwrap();
    assert_eq!(w.health.snapshot()[0].success_count, 2);
}

#[tokio::test]
async fn reddit_oauth_credentials_stay_paired_across_rotation() {
    use base64::Engine;
    let server = MockServer::start().await;
    for (id, secret, token) in [
        ("id-one", "secret-one", "token-one"),
        ("id-two", "secret-two", "token-two"),
    ] {
        let auth = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!("{id}:{secret}"))
        );
        Mock::given(path("/api/v1/access_token"))
            .and(header("authorization", auth.as_str()))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"access_token":token})),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(path("/search"))
            .and(header("authorization", format!("Bearer {token}").as_str()))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"data":{"children":[]}})),
            )
            .expect(1)
            .mount(&server)
            .await;
    }
    let mut c = config();
    c.endpoints.reddit = server.uri();
    c.endpoints.reddit_oauth = server.uri();
    c.set_accounts_json(r#"[{"provider":"reddit","name":"one","credentials":{"client_id":"id-one","client_secret":"secret-one"}},{"provider":"reddit","name":"two","credentials":{"client_id":"id-two","client_secret":"secret-two"}}]"#);
    let w = wrapped(&c, ProviderId::Reddit);
    w.providers[0].search(&request()).await.unwrap();
    w.providers[0].search(&request()).await.unwrap();
    assert!(w.health.snapshot().iter().all(|a| a.success_count == 1));
}

#[tokio::test]
async fn pagination_stays_on_origin_account_and_invalid_cursors_fail_closed() {
    use wiremock::matchers::query_param;
    let server = MockServer::start().await;
    Mock::given(path("/youtube/v3/search"))
        .and(query_param("key", "one"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"items":[], "nextPageToken":"next"})),
        )
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(path("/youtube/v3/search"))
        .and(query_param("key", "two"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"items":[]})))
        .expect(1)
        .mount(&server)
        .await;
    let mut c = config();
    c.endpoints.youtube = server.uri();
    c.set_accounts_json(r#"[{"provider":"youtube","name":"one","credentials":{"api_key":"one"}},{"provider":"youtube","name":"two","credentials":{"api_key":"two"}}]"#);
    let w = wrapped(&c, ProviderId::Youtube);
    let first = w.providers[0].search(&request()).await.unwrap();
    let cursor = first.next_cursor.unwrap();
    let mut next = request();
    next.cursor = Some(&cursor);
    w.providers[0].search(&next).await.unwrap();
    w.providers[0].search(&request()).await.unwrap();
    next.cursor = Some("invalid");
    assert!(matches!(
        w.providers[0].search(&next).await,
        Err(error::Error::Invalid(_))
    ));
    next.cursor = Some(r#"{"account":"missing","cursor":"next"}"#);
    assert!(matches!(
        w.providers[0].search(&next).await,
        Err(error::Error::Invalid(_))
    ));
    assert_eq!(w.health.snapshot()[0].success_count, 2);
    assert_eq!(w.health.snapshot()[1].success_count, 1);
}

#[test]
fn no_accounts_preserves_legacy_pointer() {
    let c = config();
    let http = HttpClient::new(&c).unwrap();
    let p = Registry::new(&c, http.clone())
        .get(ProviderId::Brave)
        .unwrap();
    let result = wrap_providers(vec![p.clone()], &c, http).unwrap();
    assert!(std::sync::Arc::ptr_eq(&p, &result.providers[0]));
    assert!(result.health.snapshot().is_empty());
}

#[tokio::test]
async fn failover_cooldown_and_health_never_expose_upstream_secrets() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("x-subscription-token", "bad-secret"))
        .respond_with(ResponseTemplate::new(429).set_body_string("bad-secret leaked"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(header("x-subscription-token", "good-secret"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"web":{"results":[]}})),
        )
        .expect(2)
        .mount(&server)
        .await;
    let mut c = config();
    c.endpoints.brave = server.uri();
    c.cooldown_secs = 60;
    c.keys.brave = vec!["legacy-must-not-be-used".into()];
    c.accounts = parse(
        r#"[{"provider":"brave","name":"bad","credentials":{"api_key":"bad-secret"}},{"provider":"brave","name":"good","credentials":{"api_key":"good-secret"}}]"#,
    );
    let w = wrapped(&c, ProviderId::Brave);
    w.providers[0].search(&request()).await.unwrap();
    w.providers[0].search(&request()).await.unwrap();
    let snapshots = w.health.snapshot();
    assert_eq!(snapshots.len(), 2);
    let bad = snapshots.iter().find(|s| s.name == "bad").unwrap();
    assert_eq!(bad.failure_count, 1);
    assert!(bad.cooldown_remaining_secs > 0);
    assert_eq!(
        snapshots
            .iter()
            .find(|s| s.name == "good")
            .unwrap()
            .success_count,
        2
    );
    assert!(
        !serde_json::to_string(&snapshots)
            .unwrap()
            .contains("secret")
    );
}

#[test]
fn every_keyed_provider_builds_an_independent_configured_adapter() {
    for provider in [
        "tavily",
        "exa",
        "firecrawl",
        "linkup",
        "brave",
        "kagi",
        "github",
        "youtube",
        "youcom",
        "parallel",
        "querit",
        "tinyfish",
        "keenable",
        "perplexity",
    ] {
        let mut c = config();
        c.set_accounts_json(&serde_json::json!([{"provider":provider,"name":"one","credentials":{"api_key":"secret"}}]).to_string());
        c.validate_accounts().unwrap();
        let id = serde_json::from_value(serde_json::json!(provider)).unwrap();
        let w = wrapped(&c, id);
        assert!(w.providers[0].is_configured(), "{provider}");
        assert_eq!(w.health.snapshot().len(), 1);
    }
}

#[tokio::test]
async fn pin_by_account_selects_named_credential_and_unknown_fails_closed() {
    let server = MockServer::start().await;
    Mock::given(path("/res/v1/web/search"))
        .and(header("x-subscription-token", "work-secret"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"web":{"results":[]}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/res/v1/web/search"))
        .and(header("x-subscription-token", "personal-secret"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"web":{"results":[]}})),
        )
        .expect(0)
        .mount(&server)
        .await;
    let mut c = config();
    c.endpoints.brave = server.uri();
    c.set_accounts_json(
        r#"[{"provider":"brave","name":"work","credentials":{"api_key":"work-secret"}},{"provider":"brave","name":"personal","credentials":{"api_key":"personal-secret"}}]"#,
    );
    let w = wrapped(&c, ProviderId::Brave);
    let mut pinned = request();
    pinned.account = Some("work");
    w.providers[0].search(&pinned).await.unwrap();
    pinned.account = Some("missing");
    assert!(matches!(
        w.providers[0].search(&pinned).await,
        Err(error::Error::Invalid(_))
    ));
    assert!(!w.providers[0].known_account("missing"));
    assert!(w.providers[0].known_account("work"));
}

#[tokio::test]
async fn firecrawl_map_uses_named_account_pool_not_legacy_keys() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/map"))
        .and(header("authorization", "Bearer account-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "links": ["https://example.com/a"]
        })))
        .expect(2)
        .mount(&server)
        .await;
    let mut c = config();
    c.endpoints.firecrawl = server.uri();
    c.keys.firecrawl = vec!["legacy-must-not-be-used".into()];
    c.set_accounts_json(
        r#"[{"provider":"firecrawl","name":"work","credentials":{"api_key":"account-secret"}}]"#,
    );
    let w = wrapped(&c, ProviderId::Firecrawl);
    let value = w.providers[0]
        .map_urls("https://example.com", None, Some(5), Some("work"))
        .await
        .unwrap();
    assert!(value.get("links").is_some());
    // Accounts-only setups must work even when legacy keys are cleared.
    c.keys.firecrawl.clear();
    let w = wrapped(&c, ProviderId::Firecrawl);
    w.providers[0]
        .map_urls("https://example.com", None, Some(5), None)
        .await
        .unwrap();
}

#[tokio::test]
async fn linkup_extract_and_depth_use_fetch_and_search_options() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .and(header("authorization", "Bearer linkup-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results":[{"url":"https://example.com","name":"Example","content":"hi"}]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/fetch"))
        .and(header("authorization", "Bearer linkup-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "markdown": "# hello"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let mut c = config();
    c.endpoints.linkup = server.uri();
    c.set_accounts_json(
        r#"[{"provider":"linkup","name":"work","credentials":{"api_key":"linkup-secret"}}]"#,
    );
    let w = wrapped(&c, ProviderId::Linkup);
    assert!(w.providers[0].supports_extract());
    let mut deep = request();
    deep.depth = Some("deep");
    let page = w.providers[0].search(&deep).await.unwrap();
    assert_eq!(page.hits.len(), 1);
    let search_body: serde_json::Value = serde_json::from_slice(
        &server
            .received_requests()
            .await
            .unwrap()
            .into_iter()
            .find(|r| r.url.path() == "/v1/search")
            .unwrap()
            .body,
    )
    .unwrap();
    assert_eq!(search_body["depth"], "deep");
    assert_eq!(search_body["maxResults"], 10);
    let docs = w.providers[0]
        .extract(&["https://example.com".into()], Some("work"))
        .await
        .unwrap();
    assert_eq!(docs[0].content, "# hello");
}

