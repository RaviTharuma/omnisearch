use omnisearch::{
    config::{Config, ProviderKeys},
    http::HttpClient,
    providers::{
        Provider, Registry,
        omniroute::{OmniRoute, parse_connections},
    },
    types::{ProviderId, ProviderSearchRequest, SearchType},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};
fn config() -> Config {
    let mut c = Config::from_env();
    c.keys = ProviderKeys::default();
    c.accounts.clear();
    c.accounts_error = None;
    c.gateways.clear();
    c.gateways_error = None;
    c
}
fn req() -> ProviderSearchRequest<'static> {
    ProviderSearchRequest {
        query: "rust",
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
fn connection(
    url: &str,
    name: &str,
    key: &str,
) -> omnisearch::providers::omniroute::GatewayConnection {
    parse_connections(&serde_json::json!([{"name":name,"base_url":url,"api_key":key,"provider":"tavily","search_type":"web"}]).to_string()).unwrap().remove(0)
}
#[tokio::test]
async fn authenticates_normalized_contract_without_upstream_keys() {
    let s = MockServer::start().await;
    let mut c = config();
    Mock::given(method("POST")).and(path("/v1/search")).and(header("authorization","Bearer gateway-secret")).and(body_json(serde_json::json!({"query":"rust","max_results":10,"provider":"tavily","search_type":"web"}))).respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"results":[{"title":"Rust","url":"https://rust-lang.org","snippet":"safe","published_date":"2026-01-01"}]}))).expect(1).mount(&s).await;
    c.gateways = vec![connection(&s.uri(), "primary", "gateway-secret")];
    let r = Registry::new(&c, HttpClient::new(&c).unwrap());
    let p = r.get(ProviderId::Omniroute).unwrap();
    assert!(p.is_configured());
    let hits = p.search(&req()).await.unwrap().hits;
    assert_eq!(hits[0].snippet, "safe");
    assert_eq!(hits[0].published_at.as_deref(), Some("2026-01-01"));
    assert!(!format!("{c:?}").contains("gateway-secret"));
    assert_eq!(r.account_health.snapshot()[0].success_count, 1);
}
#[tokio::test]
async fn gateway_failover_cooldown_and_redaction() {
    for status in [401, 403, 429, 500, 502, 503] {
        let bad = MockServer::start().await;
        let good = MockServer::start().await;
        let mut c = config();
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(status).set_body_string("DO-NOT-LEAK upstream-secret"),
            )
            .expect(1)
            .mount(&bad)
            .await;
        Mock::given(header("authorization", "Bearer second-secret"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"results":[]})),
            )
            .expect(2)
            .mount(&good)
            .await;
        c.gateways = vec![
            connection(&bad.uri(), "bad", "first-secret"),
            connection(&good.uri(), "good", "second-secret"),
        ];
        let r = Registry::new(&c, HttpClient::new(&c).unwrap());
        let p = r.get(ProviderId::Omniroute).unwrap();
        p.search(&req()).await.unwrap();
        p.search(&req()).await.unwrap();
        let h = r.account_health.snapshot();
        assert_eq!(h[0].failure_count, 1);
        assert!(h[0].cooldown_remaining_secs > 0);
        assert_eq!(h[1].success_count, 2);
        let health = serde_json::to_string(&h).unwrap();
        assert!(!health.contains("secret"));
        assert!(!health.contains("DO-NOT-LEAK"));
    }
}
#[tokio::test]
async fn rejects_malformed_success_and_sanitizes_errors() {
    let s = MockServer::start().await;
    let c = config();
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string("secret-invalid-json"))
        .mount(&s)
        .await;
    let p = OmniRoute::new(
        HttpClient::new(&c).unwrap(),
        connection(&s.uri(), "one", "key"),
    )
    .unwrap();
    let e = p.search(&req()).await.unwrap_err().to_string();
    assert!(e.contains("invalid gateway search response"));
    assert!(!e.contains("secret"));
}
#[tokio::test]
async fn invalid_query_does_not_contact_or_cool_gateways() {
    let s = MockServer::start().await;
    let mut c = config();
    c.gateways.push(connection(&s.uri(), "one", "key"));
    let r = Registry::new(&c, HttpClient::new(&c).unwrap());
    let query = "x".repeat(501);
    let mut request = req();
    request.query = &query;
    assert!(matches!(
        r.get(ProviderId::Omniroute).unwrap().search(&request).await,
        Err(omnisearch::error::Error::Invalid(_))
    ));
    assert!(s.received_requests().await.unwrap().is_empty());
    assert_eq!(r.account_health.snapshot()[0].failure_count, 0);
    assert!(r.get(ProviderId::Omniroute).unwrap().estimated_search_usd() > 0.0);
}
#[tokio::test]
async fn slow_account_times_out_then_falls_back() {
    let slow = MockServer::start().await;
    let fast = MockServer::start().await;
    Mock::given(path("/v1/search"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_secs(2))
                .set_body_json(serde_json::json!({"results":[]})),
        )
        .expect(1)
        .mount(&slow)
        .await;
    Mock::given(path("/v1/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"results":[]})))
        .expect(1)
        .mount(&fast)
        .await;
    let mut c = config();
    c.gateways.push(connection(&slow.uri(), "slow", "key"));
    c.default_timeout_secs = 1;
    c.gateways.push(connection(&fast.uri(), "fast", "key"));
    let r = Registry::new(&c, HttpClient::new(&c).unwrap());
    r.get(ProviderId::Omniroute)
        .unwrap()
        .search(&req())
        .await
        .unwrap();
    let health = r.account_health.snapshot();
    assert_eq!(
        health
            .iter()
            .find(|a| a.name == "slow")
            .unwrap()
            .failure_count,
        1
    );
    assert_eq!(
        health
            .iter()
            .find(|a| a.name == "fast")
            .unwrap()
            .success_count,
        1
    );
}
#[test]
fn validates_urls_and_names() {
    for url in [
        "ftp://example.com",
        "https://user:secret@example.com",
        "https://example.com?key=secret",
        "https://example.com/#secret",
    ] {
        assert!(
            parse_connections(
                &serde_json::json!([{"name":"one","base_url":url,"api_key":"key"}]).to_string()
            )
            .is_err()
        );
    }
    for base in [
        "https://example.com",
        "https://example.com/v1/",
        "https://example.com/api/v1",
        "https://example.com/v1/search",
    ] {
        assert!(
            connection(base, "one", "key")
                .endpoint()
                .unwrap()
                .ends_with("/v1/search")
        );
    }
    assert!(parse_connections("[{\"secret\":\"key\"}]").is_err());
}
