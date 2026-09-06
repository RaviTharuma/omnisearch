//! RRF merge, URL/title dedupe, spam filter, and domain diversity.

use std::collections::{HashMap, HashSet};

use crate::types::{QualityReport, SearchHit};
use crate::urlutil::{host_of, normalize_title, normalize_url};

/// Known low-value / mirror hosts. Conservative list only.
const SPAM_HOSTS: &[&str] = &[
    "bit.ly",
    "tinyurl.com",
    "t.co",
    "click.redditmail.com",
    "googleadservices.com",
    "doubleclick.net",
];

/// Merge ranked provider lists with Reciprocal Rank Fusion.
pub fn rrf_merge(lists: Vec<Vec<SearchHit>>, k: f64, max_per_domain: usize) -> MergeOutput {
    let mut spam_dropped = 0u32;
    let mut acc: HashMap<String, Acc> = HashMap::new();

    for list in lists {
        for (idx, hit) in list.into_iter().enumerate() {
            if is_spam(&hit.url) {
                spam_dropped += 1;
                continue;
            }
            let url_key = normalize_url(&hit.url);
            let rank = (idx + 1) as f64;
            let add = 1.0 / (k + rank);
            acc.entry(url_key)
                .and_modify(|a| a.add(&hit, add))
                .or_insert_with(|| Acc::new(hit, add));
        }
    }

    // Title near-duplicate pass: keep the higher RRF URL.
    let mut by_title: HashMap<String, String> = HashMap::new();
    let keys: Vec<String> = acc.keys().cloned().collect();
    for key in keys {
        let title_key = normalize_title(&acc[&key].hit.title);
        if title_key.len() < 12 {
            continue;
        }
        if let Some(existing) = by_title.get(&title_key) {
            let existing_score = acc[existing].rrf;
            let current_score = acc[&key].rrf;
            if current_score <= existing_score {
                if let Some(loser) = acc.remove(&key)
                    && let Some(winner) = acc.get_mut(existing)
                {
                    winner.absorb(loser);
                }
            } else if let Some(loser) = acc.remove(existing) {
                if let Some(winner) = acc.get_mut(&key) {
                    winner.absorb(loser);
                }
                by_title.insert(title_key, key);
            }
        } else {
            by_title.insert(title_key, key);
        }
    }

    let mut ranked: Vec<Acc> = acc.into_values().collect();
    ranked.sort_by(|a, b| {
        b.rrf
            .partial_cmp(&a.rrf)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let diversified = apply_domain_diversity(ranked, max_per_domain);
    let unique_domains = diversified
        .iter()
        .filter_map(|h| host_of(&h.url))
        .collect::<HashSet<_>>()
        .len() as u32;
    let providers: HashSet<String> = diversified.iter().flat_map(|h| h.sources.clone()).collect();
    let with_published_at = diversified
        .iter()
        .filter(|h| h.published_at.is_some())
        .count() as u32;

    let mut domain_counts: HashMap<String, u32> = HashMap::new();
    for hit in &diversified {
        if let Some(host) = host_of(&hit.url) {
            *domain_counts.entry(host).or_default() += 1;
        }
    }
    let mut top_domains: Vec<_> = domain_counts
        .into_iter()
        .map(|(domain, count)| crate::types::DomainCount { domain, count })
        .collect();
    top_domains.sort_by_key(|a| std::cmp::Reverse(a.count));
    top_domains.truncate(8);

    let report = QualityReport {
        unique_results: diversified.len() as u32,
        unique_domains,
        providers_contributing: providers.len() as u32,
        with_published_at,
        spam_dropped,
        top_domains,
    };

    MergeOutput {
        hits: diversified,
        quality: report,
    }
}

/// Merge result plus diagnostics.
pub struct MergeOutput {
    pub hits: Vec<SearchHit>,
    pub quality: QualityReport,
}

struct Acc {
    hit: SearchHit,
    rrf: f64,
}

impl Acc {
    fn new(mut hit: SearchHit, rrf: f64) -> Self {
        if hit.sources.is_empty() {
            hit.sources.push(hit.provider.clone());
        }
        hit.score = Some(rrf);
        Self { hit, rrf }
    }

    fn add(&mut self, other: &SearchHit, add: f64) {
        self.rrf += add;
        self.hit.score = Some(self.rrf);
        let src = if other.sources.is_empty() {
            vec![other.provider.clone()]
        } else {
            other.sources.clone()
        };
        for s in src {
            if !self.hit.sources.iter().any(|e| e == &s) {
                self.hit.sources.push(s);
            }
        }
        if self.hit.snippet.is_empty() && !other.snippet.is_empty() {
            self.hit.snippet = other.snippet.clone();
        }
        if self.hit.published_at.is_none() {
            self.hit.published_at = other.published_at.clone();
        }
    }

    fn absorb(&mut self, other: Acc) {
        self.add(&other.hit, 0.0);
        self.rrf += other.rrf;
        self.hit.score = Some(self.rrf);
    }
}

fn is_spam(url: &str) -> bool {
    host_of(url).is_some_and(|h| {
        SPAM_HOSTS
            .iter()
            .any(|s| h == *s || h.ends_with(&format!(".{s}")))
    })
}

/// Prefer diverse domains first; overflow is appended, not dropped.
fn apply_domain_diversity(ranked: Vec<Acc>, max_per_domain: usize) -> Vec<SearchHit> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut primary = Vec::new();
    let mut overflow = Vec::new();
    for acc in ranked {
        let host = host_of(&acc.hit.url).unwrap_or_else(|| "unknown".into());
        let n = counts.entry(host).or_default();
        if *n < max_per_domain {
            *n += 1;
            primary.push(acc.hit);
        } else {
            overflow.push(acc.hit);
        }
    }
    primary.extend(overflow);
    primary
}

/// Drop hits whose published_at is older than the freshness window when parseable.
pub fn apply_freshness(hits: Vec<SearchHit>, since: &str) -> Vec<SearchHit> {
    let Ok(cutoff) = chrono::DateTime::parse_from_rfc3339(since) else {
        return hits;
    };
    hits.into_iter()
        .filter(|h| match &h.published_at {
            None => true,
            Some(raw) => chrono::DateTime::parse_from_rfc3339(raw)
                .ok()
                .or_else(|| {
                    chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d")
                        .ok()
                        .and_then(|d| d.and_hms_opt(0, 0, 0))
                        .map(|dt| {
                            chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                                dt,
                                chrono::Utc,
                            )
                            .fixed_offset()
                        })
                })
                .is_none_or(|dt| dt >= cutoff),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ProviderId, SearchHit};

    #[test]
    fn rrf_dedupes_url_and_tracks_sources() {
        let a = vec![
            SearchHit::new(ProviderId::Tavily, "Alpha", "https://example.com/a", "one"),
            SearchHit::new(ProviderId::Tavily, "Beta", "https://example.com/b", "two"),
        ];
        let b = vec![SearchHit::new(
            ProviderId::Exa,
            "Alpha copy",
            "https://example.com/a?utm_source=x",
            "other",
        )];
        let out = rrf_merge(vec![a, b], 60.0, 8);
        assert_eq!(out.hits.len(), 2);
        let merged = out
            .hits
            .iter()
            .find(|h| h.url.contains("/a"))
            .expect("merged url");
        assert!(merged.sources.contains(&"tavily".into()));
        assert!(merged.sources.contains(&"exa".into()));
        assert!(merged.score.unwrap() > 0.0);
    }

    #[test]
    fn drops_spam_hosts() {
        let list = vec![SearchHit::new(
            ProviderId::Brave,
            "ad",
            "https://bit.ly/abc",
            "x",
        )];
        let out = rrf_merge(vec![list], 60.0, 8);
        assert!(out.hits.is_empty());
        assert_eq!(out.quality.spam_dropped, 1);
    }
}
