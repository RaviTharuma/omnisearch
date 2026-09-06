//! In-process search cache with TTL.

use std::time::{Duration, Instant};

use dashmap::DashMap;
use sha2::{Digest, Sha256};

use crate::types::SearchResponse;

#[derive(Clone)]
struct Entry {
    stored_at: Instant,
    value: SearchResponse,
}

/// Process-local cache. Not shared across hosts.
#[derive(Default)]
pub struct SearchCache {
    map: DashMap<String, Entry>,
}

impl SearchCache {
    /// Create an empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stable key for a query + routing fingerprint.
    pub fn key(parts: &[&str]) -> String {
        let mut hasher = Sha256::new();
        for part in parts {
            hasher.update(part.as_bytes());
            hasher.update([0xff]);
        }
        hex::encode(hasher.finalize())
    }

    /// Fetch a fresh entry.
    pub fn get(&self, key: &str, ttl: Duration) -> Option<SearchResponse> {
        let entry = self.map.get(key)?;
        if entry.stored_at.elapsed() > ttl {
            drop(entry);
            self.map.remove(key);
            return None;
        }
        Some(entry.value.clone())
    }

    /// Store a response.
    pub fn put(&self, key: String, value: SearchResponse) {
        self.map.insert(
            key,
            Entry {
                stored_at: Instant::now(),
                value,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::SearchCache;

    #[test]
    fn keys_differ_by_input() {
        assert_ne!(SearchCache::key(&["alpha"]), SearchCache::key(&["beta"]));
    }
}
