//! Named account isolation, round-robin selection, failover and redacted health.
use crate::{
    config::Config,
    error::{Error, Result},
    http::HttpClient,
    providers::{Provider, Registry},
    types::{ExtractedDoc, ProviderId, ProviderSearchRequest, SearchPage},
};
use async_trait::async_trait;
use serde::Serialize;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Serialize)]
pub struct AccountHealth {
    pub provider: ProviderId,
    pub name: String,
    pub configured: bool,
    pub success_count: u64,
    pub failure_count: u64,
    pub cooldown_remaining_secs: u64,
    pub last_latency_ms: Option<u64>,
    /// Error category only: upstream error bodies can contain credentials.
    pub last_error: Option<String>,
}
#[derive(Default)]
struct Stats {
    success: u64,
    failure: u64,
    until: Option<Instant>,
    latency: Option<u64>,
    error: Option<String>,
}
pub(crate) struct Account {
    name: String,
    provider: Arc<dyn Provider>,
    stats: Mutex<Stats>,
}
impl Account {
    fn available(&self) -> bool {
        self.provider.is_configured()
            && self
                .stats
                .lock()
                .unwrap()
                .until
                .is_none_or(|t| t <= Instant::now())
    }
    fn record<T>(&self, result: &Result<T>, start: Instant, cooldown: Duration) {
        let mut stats = self.stats.lock().unwrap();
        stats.latency = Some(start.elapsed().as_millis().min(u64::MAX as u128) as u64);
        match result {
            Ok(_) => {
                stats.success += 1;
                stats.until = None;
                stats.error = None;
            }
            Err(error) => {
                stats.failure += 1;
                stats.error = Some(error_category(error).into());
                // All account failures cool down, including auth, 5xx and transport errors.
                stats.until = Instant::now().checked_add(cooldown);
            }
        }
    }
}
fn error_category(error: &Error) -> &'static str {
    match error {
        Error::RateLimited { .. } => "rate_limited",
        Error::Timeout { .. } => "timeout",
        Error::NotConfigured { .. } => "not_configured",
        Error::Invalid(_) => "invalid",
        _ => "provider_error",
    }
}
#[derive(Default, Clone)]
pub struct AccountHealthBoard {
    accounts: Vec<Arc<Account>>,
}
impl AccountHealthBoard {
    pub fn snapshot(&self) -> Vec<AccountHealth> {
        let mut snapshots: Vec<_> = self
            .accounts
            .iter()
            .map(|a| {
                let s = a.stats.lock().unwrap();
                let remaining = s
                    .until
                    .map(|t| t.saturating_duration_since(Instant::now()))
                    .unwrap_or_default();
                AccountHealth {
                    provider: a.provider.id(),
                    name: a.name.clone(),
                    configured: a.provider.is_configured(),
                    success_count: s.success,
                    failure_count: s.failure,
                    cooldown_remaining_secs: remaining.as_secs()
                        + u64::from(remaining.subsec_nanos() > 0),
                    last_latency_ms: s.latency,
                    last_error: s.error.clone(),
                }
            })
            .collect();
        snapshots
            .sort_by(|a, b| (a.provider.as_str(), &a.name).cmp(&(b.provider.as_str(), &b.name)));
        snapshots
    }

    /// True when a named account exists for the provider (credentials never exposed).
    pub fn has_account(&self, provider: ProviderId, name: &str) -> bool {
        self.accounts
            .iter()
            .any(|a| a.provider.id() == provider && a.name == name)
    }
}
pub struct WrappedProviders {
    pub providers: Vec<Arc<dyn Provider>>,
    pub health: Arc<AccountHealthBoard>,
}
/// Replace only providers with explicit accounts. Unaffected Arc identities and
/// legacy key-ring behavior are preserved. Keep `health` in Registry for tools.
pub fn wrap_providers(
    legacy: Vec<Arc<dyn Provider>>,
    config: &Config,
    http: HttpClient,
) -> Result<WrappedProviders> {
    config.validate_accounts()?;
    let mut legacy = legacy;
    // Optional providers (notably MCP) may not exist in the legacy registry.
    for spec in &config.accounts {
        if !legacy.iter().any(|p| p.id() == spec.provider) {
            let mut isolated = config.clone();
            spec.apply(&mut isolated)?;
            let adapter = Registry::new(&isolated, http.clone())
                .get(spec.provider)
                .ok_or_else(|| Error::Invalid("unsupported account provider".into()))?;
            legacy.push(adapter);
        }
    }
    let mut board = AccountHealthBoard::default();
    let mut providers = Vec::with_capacity(legacy.len());
    for provider in legacy {
        let mut accounts = Vec::new();
        for spec in config
            .accounts
            .iter()
            .filter(|a| a.provider == provider.id())
        {
            let mut isolated = config.clone();
            spec.apply(&mut isolated)?; // clears accounts BEFORE Registry::new: no recursion.
            let adapter = Registry::new(&isolated, http.clone())
                .get(spec.provider)
                .ok_or_else(|| Error::Invalid("unsupported account provider".into()))?;
            let account = Arc::new(Account {
                name: spec.name.clone(),
                provider: adapter,
                stats: Mutex::new(Stats::default()),
            });
            board.accounts.push(account.clone());
            accounts.push(account);
        }
        if accounts.is_empty() {
            providers.push(provider);
        } else {
            providers.push(Arc::new(AccountProvider {
                metadata: accounts[0].provider.clone(),
                attempt_timeout: Duration::from_secs(config.default_timeout_secs)
                    / (accounts.len() as u32 + 1),
                accounts,
                next: AtomicUsize::new(0),
                cooldown: Duration::from_secs(config.cooldown_secs),
            }) as Arc<dyn Provider>);
        }
    }
    if !config.gateways.is_empty() {
        config.validate_gateways()?;
        let accounts = config
            .gateways
            .iter()
            .map(|connection| {
                Ok(Arc::new(Account {
                    name: connection.name.clone(),
                    provider: Arc::new(crate::providers::omniroute::OmniRoute::new(
                        http.clone(),
                        connection.clone(),
                    )?),
                    stats: Mutex::new(Stats::default()),
                }))
            })
            .collect::<Result<Vec<_>>>()?;
        board.accounts.extend(accounts.iter().cloned());
        providers.push(Arc::new(AccountProvider {
            metadata: accounts[0].provider.clone(),
            attempt_timeout: Duration::from_secs(config.default_timeout_secs)
                / (accounts.len() as u32 + 1),
            accounts,
            next: AtomicUsize::new(0),
            cooldown: Duration::from_secs(config.cooldown_secs.max(1)),
        }));
    }
    Ok(WrappedProviders {
        providers,
        health: Arc::new(board),
    })
}
struct AccountProvider {
    metadata: Arc<dyn Provider>,
    accounts: Vec<Arc<Account>>,
    next: AtomicUsize,
    cooldown: Duration,
    attempt_timeout: Duration,
}
impl AccountProvider {
    fn order(&self) -> Vec<usize> {
        let start = self.next.fetch_add(1, Ordering::Relaxed) % self.accounts.len();
        (0..self.accounts.len())
            .map(|n| (start + n) % self.accounts.len())
            .collect()
    }
    /// Resolve rotation order. Explicit pin and cursor both fail closed on unknown names.
    fn resolve_order(&self, pin: Option<&str>, cursor: Option<&Cursor>) -> Result<Vec<usize>> {
        if let Some(name) = pin {
            let index = self
                .accounts
                .iter()
                .position(|a| a.name == name)
                .ok_or_else(|| Error::Invalid(format!("unknown account '{name}'")))?;
            if let Some(c) = cursor
                && c.account != name
            {
                return Err(Error::Invalid(
                    "cursor account does not match pinned account".into(),
                ));
            }
            return Ok(vec![index]);
        }
        if let Some(c) = cursor {
            let index = self
                .accounts
                .iter()
                .position(|a| a.name == c.account)
                .ok_or_else(|| Error::Invalid("unknown cursor account".into()))?;
            return Ok(vec![index]);
        }
        Ok(self.order())
    }
    fn unavailable(&self) -> Error {
        Error::rate_limited(
            self.id().as_str(),
            "all named accounts unavailable or cooling down",
        )
    }
    fn safe_error(&self, error: &Error) -> Error {
        match error {
            Error::RateLimited { .. } => {
                Error::rate_limited(self.id().as_str(), "named account rate limited")
            }
            Error::Timeout { seconds, .. } => Error::Timeout {
                provider: self.id().as_str().into(),
                seconds: *seconds,
            },
            _ => Error::provider(self.id().as_str(), "named account failed"),
        }
    }
    async fn run_accounts<T, F, Fut>(
        &self,
        pin: Option<&str>,
        timeout: Duration,
        mut call: F,
    ) -> Result<T>
    where
        F: FnMut(Arc<dyn Provider>) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let order = self.resolve_order(pin, None)?;
        let mut last = None;
        for index in order {
            let account = &self.accounts[index];
            if !account.available() {
                continue;
            }
            let start = Instant::now();
            let result = tokio::time::timeout(timeout, call(account.provider.clone()))
                .await
                .unwrap_or_else(|_| {
                    Err(Error::Timeout {
                        seconds: timeout.as_secs(),
                        provider: self.id().to_string(),
                    })
                });
            if matches!(&result, Err(Error::Invalid(_))) {
                return result;
            }
            account.record(&result, start, self.cooldown);
            match result {
                Ok(value) => return Ok(value),
                Err(error) => last = Some(self.safe_error(&error)),
            }
        }
        Err(last.unwrap_or_else(|| self.unavailable()))
    }
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    account: String,
    cursor: String,
}
#[async_trait]
impl Provider for AccountProvider {
    fn id(&self) -> ProviderId {
        self.metadata.id()
    }
    fn is_configured(&self) -> bool {
        self.accounts.iter().any(|a| a.provider.is_configured())
    }
    fn skip_reason(&self) -> Option<String> {
        (!self.is_configured()).then(|| "no configured named accounts".into())
    }
    fn supports_search(&self) -> bool {
        self.metadata.supports_search()
    }
    fn supports_extract(&self) -> bool {
        self.metadata.supports_extract()
    }
    fn estimated_search_usd(&self) -> f64 {
        // Admission must cover every account rotation or failover can select.
        self.accounts
            .iter()
            .map(|account| account.provider.estimated_search_usd())
            .fold(0.0, f64::max)
    }
    fn max_page_size(&self) -> u32 {
        self.accounts
            .iter()
            .map(|account| account.provider.max_page_size())
            .min()
            .unwrap_or(1)
    }
    fn notes(&self) -> &'static str {
        self.metadata.notes()
    }
    fn requires_key(&self) -> bool {
        self.metadata.requires_key()
    }
    fn known_account(&self, name: &str) -> bool {
        self.accounts.iter().any(|a| a.name == name)
    }
    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        let cursor: Option<Cursor> = request
            .cursor
            .map(|c| {
                serde_json::from_str(c)
                    .map_err(|_| Error::Invalid("invalid named account cursor".into()))
            })
            .transpose()?;
        let order = self.resolve_order(request.account, cursor.as_ref())?;
        let mut last = None;
        for index in order {
            let account = &self.accounts[index];
            if !account.available() {
                continue;
            }
            let mut scoped = request.clone();
            scoped.cursor = cursor.as_ref().map(|c| c.cursor.as_str());
            // Nested adapters do not re-interpret the pin.
            scoped.account = None;
            let start = Instant::now();
            let result =
                tokio::time::timeout(self.attempt_timeout, account.provider.search(&scoped))
                    .await
                    .unwrap_or_else(|_| {
                        Err(Error::Timeout {
                            seconds: self.attempt_timeout.as_secs(),
                            provider: self.id().to_string(),
                        })
                    });
            if matches!(&result, Err(Error::Invalid(_))) {
                return result;
            }
            account.record(&result, start, self.cooldown);
            match result {
                Ok(mut page) => {
                    page.next_cursor = page.next_cursor.map(|cursor| {
                        serde_json::to_string(&Cursor {
                            account: account.name.clone(),
                            cursor,
                        })
                        .expect("string cursor serializes")
                    });
                    return Ok(page);
                }
                Err(error) => {
                    last = Some(self.safe_error(&error));
                }
            }
        }
        Err(last.unwrap_or_else(|| self.unavailable()))
    }
    async fn extract(
        &self,
        urls: &[String],
        account: Option<&str>,
    ) -> Result<Vec<ExtractedDoc>> {
        self.run_accounts(account, self.attempt_timeout, |provider| {
            let urls = urls.to_vec();
            async move { provider.extract(&urls, None).await }
        })
        .await
    }
    async fn crawl(
        &self,
        url: &str,
        limit: u32,
        timeout_secs: u64,
        account: Option<&str>,
    ) -> Result<serde_json::Value> {
        let timeout = Duration::from_secs(timeout_secs.max(self.attempt_timeout.as_secs()));
        let url = url.to_string();
        self.run_accounts(account, timeout, move |provider| {
            let url = url.clone();
            async move { provider.crawl(&url, limit, timeout_secs, None).await }
        })
        .await
    }
    async fn map_urls(
        &self,
        url: &str,
        search: Option<&str>,
        limit: Option<u32>,
        account: Option<&str>,
    ) -> Result<serde_json::Value> {
        let url = url.to_string();
        let search = search.map(str::to_string);
        self.run_accounts(account, self.attempt_timeout, move |provider| {
            let url = url.clone();
            let search = search.clone();
            async move {
                provider
                    .map_urls(&url, search.as_deref(), limit, None)
                    .await
            }
        })
        .await
    }
}
