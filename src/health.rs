//! Provider health, 429/outage cooldown, and adaptive ranking.

use std::time::{Duration, Instant};

use dashmap::DashMap;

use crate::error::Error;
use crate::types::ProviderId;

#[derive(Clone, Debug, Default)]
struct Stats {
    success: u64,
    failure: u64,
    cooldown_until: Option<Instant>,
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
    pub fn mark_success(&self, id: ProviderId) {
        self.stats
            .entry(id)
            .and_modify(|s| {
                s.success += 1;
                s.cooldown_until = None;
            })
            .or_insert(Stats {
                success: 1,
                ..Stats::default()
            });
    }

    /// Record a failure; 429/timeouts start a cooldown.
    pub fn mark_failure(&self, id: ProviderId, err: &Error, cooldown: Duration) {
        let cool = matches!(err, Error::RateLimited { .. } | Error::Timeout { .. });
        self.stats
            .entry(id)
            .and_modify(|s| {
                s.failure += 1;
                if cool {
                    s.cooldown_until = Some(Instant::now() + cooldown);
                }
            })
            .or_insert(Stats {
                failure: 1,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;

    #[test]
    fn cooldown_after_429() {
        let board = HealthBoard::new();
        let err = Error::rate_limited("tavily", "slow down");
        board.mark_failure(ProviderId::Tavily, &err, Duration::from_secs(30));
        assert!(board.is_cooling(ProviderId::Tavily));
        assert!(board.score(ProviderId::Tavily) < 0.5);
    }
}
