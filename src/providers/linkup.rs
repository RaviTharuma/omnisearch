//! Linkup agentic search and fetch extract.

use async_trait::async_trait;
use serde_json::json;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::http::{HttpClient, pick_str};
use crate::types::{ExtractedDoc, ProviderId, ProviderSearchRequest, SearchPage};

use super::{Keyed, Provider, map_hits, page};

pub struct Linkup {
    inner: Keyed,
}

impl Linkup {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            inner: Keyed::new(http, config.keys.linkup.clone(), &config.endpoints.linkup),
        }
    }
}

#[async_trait]
impl Provider for Linkup {
    fn id(&self) -> ProviderId {
        ProviderId::Linkup
    }
    fn is_configured(&self) -> bool {
        self.inner.configured()
    }
    fn skip_reason(&self) -> Option<String> {
        self.inner.skip("LINKUP_API_KEY")
    }
    fn supports_extract(&self) -> bool {
        true
    }
    fn estimated_search_usd(&self) -> f64 {
        0.007
    }
    fn max_page_size(&self) -> u32 {
        100
    }
    fn notes(&self) -> &'static str {
        "POST /v1/search (depth fast/standard/deep, maxResults) plus /v1/fetch extract."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        crate::try_keys!(
            &self.inner.keys,
            "linkup",
            "LINKUP_API_KEY not set",
            |key| { self.search_with(key, request).await }
        )
    }

    async fn extract(&self, urls: &[String], _account: Option<&str>) -> Result<Vec<ExtractedDoc>> {
        crate::try_keys!(
            &self.inner.keys,
            "linkup",
            "LINKUP_API_KEY not set",
            |key| { self.extract_with(key, urls).await }
        )
    }
}

impl Linkup {
    fn depth(request: &ProviderSearchRequest<'_>) -> Result<&'static str> {
        match request
            .depth
            .map(|d| d.trim().to_ascii_lowercase())
            .as_deref()
        {
            None | Some("") | Some("standard") => Ok("standard"),
            Some("fast") => Ok("fast"),
            Some("flash") => Ok("flash"),
            Some("deep") => Ok("deep"),
            Some(other) => Err(Error::Invalid(format!(
                "unknown linkup depth '{other}' (expected fast, flash, standard, or deep)"
            ))),
        }
    }

    async fn search_with(
        &self,
        key: &str,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
        let mut body = json!({
            "q": request.query,
            "depth": Self::depth(request)?,
            "outputType": "searchResults",
            "maxResults": request.page_size.max(1),
        });
        if let Some(fresh) = request.freshness {
            body["fromDate"] = json!(fresh.since_rfc3339()[..10]);
        }
        let value = self
            .inner
            .http
            .json(
                "linkup",
                self.inner
                    .http
                    .post(&self.inner.url("/v1/search"))
                    .bearer_auth(key)
                    .json(&body),
            )
            .await?;
        Ok(page(map_hits(
            ProviderId::Linkup,
            &value,
            &["url"],
            &["name", "title"],
            &["content", "snippet"],
            &[],
            &[],
        )))
    }

    async fn extract_with(&self, key: &str, urls: &[String]) -> Result<Vec<ExtractedDoc>> {
        let mut out = Vec::new();
        for url in urls {
            let value = self
                .inner
                .http
                .json(
                    "linkup",
                    self.inner
                        .http
                        .post(&self.inner.url("/v1/fetch"))
                        .bearer_auth(key)
                        .json(&json!({
                            "url": url,
                            "mode": "standard",
                            "renderJs": true,
                        })),
                )
                .await?;
            out.push(ExtractedDoc {
                url: url.clone(),
                title: pick_str(&value, &["title"]),
                content: pick_str(&value, &["markdown", "content", "rawHtml"]).unwrap_or_default(),
                provider: "linkup".into(),
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Freshness, SearchType};

    fn req<'a>(depth: Option<&'a str>) -> ProviderSearchRequest<'a> {
        ProviderSearchRequest {
            query: "test",
            cursor: None,
            page_size: 10,
            search_type: SearchType::Web,
            freshness: Some(Freshness::Week),
            country: "US",
            language: "en",
            account: None,
            depth,
        }
    }

    #[test]
    fn accepts_documented_depths() {
        for depth in [
            None,
            Some("standard"),
            Some("fast"),
            Some("flash"),
            Some("deep"),
        ] {
            assert!(Linkup::depth(&req(depth)).is_ok());
        }
        assert!(matches!(
            Linkup::depth(&req(Some("agentic"))),
            Err(Error::Invalid(_))
        ));
    }
}
