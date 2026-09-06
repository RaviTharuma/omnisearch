//! Provider health, 429/outage cooldown, and adaptive ranking.

use std::time::{Duration, Instant};

use dashmap::DashMap;

use crate::error::Error;
use crate::types::{ProviderHealth, ProviderId};

#[derive(Clone, Debug, Default)]
struct Stats {
    success: u64,
    failure: u64,
    cooldown_until: Option<Instant>,
    last_latency_ms: Option<u64>,
    last_error: Option<String>,
}

/// Shared health board used by auto routing and failover.
#[derive(Default)]
pub struct HealthBoard {
    stats: DashMap<ProviderId, Stats>,
}

impl HealthBoard {
    /// Create an empty board.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a successful call and clear cooldown.
    pub fn mark_success(&self, id: ProviderId, latency: Duration) {
        self.stats
            .entry(id)
            .and_modify(|s| {
                s.success += 1;
                s.cooldown_until = None;
                s.last_latency_ms = Some(latency.as_millis() as u64);
                s.last_error = None;
            })
            .or_insert(Stats {
                success: 1,
                last_latency_ms: Some(latency.as_millis() as u64),
                ..Stats::default()
            });
    }

    /// Record a failure; 429/timeouts start a cooldown.
    pub fn mark_failure(&self, id: ProviderId, err: &Error, cooldown: Duration, latency: Duration) {
        let cool = matches!(err, Error::RateLimited { .. } | Error::Timeout { .. });
        let message = err.to_string();
        self.stats
            .entry(id)
            .and_modify(|s| {
                s.failure += 1;
                s.last_latency_ms = Some(latency.as_millis() as u64);
                s.last_error = Some(message.clone());
                if cool {
                    s.cooldown_until = Some(Instant::now() + cooldown);
                }
            })
            .or_insert(Stats {
                failure: 1,
                last_latency_ms: Some(latency.as_millis() as u64),
                last_error: Some(message),
                cooldown_until: cool.then(|| Instant::now() + cooldown),
                ..Stats::default()
            });
    }

    /// True when the provider is inside a cooldown window.
    pub fn is_cooling(&self, id: ProviderId) -> bool {
        self.stats
            .get(&id)
            .and_then(|s| s.cooldown_until)
            .is_some_and(|until| until > Instant::now())
    }

    /// Success ratio in `[0, 1]`, unknown providers score 0.5.
    pub fn score(&self, id: ProviderId) -> f64 {
        match self.stats.get(&id) {
            None => 0.5,
            Some(s) => {
                let total = s.success + s.failure;
                if total == 0 {
                    0.5
                } else {
                    s.success as f64 / total as f64
                }
            }
        }
    }

    /// Non-secret snapshot used by `search_health`.
    pub fn snapshot(&self, id: ProviderId) -> HealthSnapshot {
        match self.stats.get(&id) {
            None => HealthSnapshot::default(),
            Some(s) => {
                let remaining = s.cooldown_until.and_then(|until| {
                    until
                        .checked_duration_since(Instant::now())
                        .map(|d| d.as_secs())
                });
                HealthSnapshot {
                    success: s.success,
                    failure: s.failure,
                    cooling: remaining.is_some(),
                    cooldown_remaining_secs: remaining.unwrap_or(0),
                    last_latency_ms: s.last_latency_ms,
                    last_error: s.last_error.clone(),
                }
            }
        }
    }
}

/// Raw board numbers before combining with registry metadata.
#[derive(Debug, Clone, Default)]
pub struct HealthSnapshot {
    pub success: u64,
    pub failure: u64,
    pub cooling: bool,
    pub cooldown_remaining_secs: u64,
    pub last_latency_ms: Option<u64>,
    pub last_error: Option<String>,
}

impl HealthSnapshot {
    /// Merge with static provider metadata.
    pub fn into_health(
        self,
        id: &str,
        configured: bool,
        requires_key: bool,
        estimated_search_usd: f64,
        notes: String,
    ) -> ProviderHealth {
        ProviderHealth {
            id: id.to_string(),
            configured,
            requires_key,
            cooling: self.cooling,
            cooldown_remaining_secs: self.cooldown_remaining_secs,
            success: self.success,
            failure: self.failure,
            last_latency_ms: self.last_latency_ms,
            last_error: self.last_error,
            estimated_search_usd,
            notes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;

    #[test]
    fn cooldown_after_429() {
        let board = HealthBoard::new();
        let err = Error::rate_limited("tavily", "slow down");
        board.mark_failure(
            ProviderId::Tavily,
            &err,
            Duration::from_secs(30),
            Duration::from_millis(12),
        );
        assert!(board.is_cooling(ProviderId::Tavily));
        assert!(board.score(ProviderId::Tavily) < 0.5);
        let snap = board.snapshot(ProviderId::Tavily);
        assert_eq!(snap.failure, 1);
        assert_eq!(snap.last_latency_ms, Some(12));
        assert!(snap.last_error.unwrap().contains("rate limited"));
    }
}
