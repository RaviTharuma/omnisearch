//! URL normalization and domain helpers.

use url::Url;

/// Lowercase host, drop fragment and common tracking params, trim trailing slash.
pub fn normalize_url(raw: &str) -> String {
    let Ok(mut url) = Url::parse(raw.trim()) else {
        return raw.trim().to_string();
    };
    url.set_fragment(None);
    let tracking = [
        "utm_source",
        "utm_medium",
        "utm_campaign",
        "utm_term",
        "utm_content",
        "fbclid",
        "gclid",
        "mc_cid",
        "mc_eid",
    ];
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| !tracking.iter().any(|t| k.eq_ignore_ascii_case(t)))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    if pairs.is_empty() {
        url.set_query(None);
    } else {
        url.query_pairs_mut().clear().extend_pairs(pairs);
    }
    if url.path().ends_with('/') && url.path() != "/" {
        let trimmed = url.path().trim_end_matches('/').to_string();
        url.set_path(&trimmed);
    }
    url.to_string()
}

/// Registrable-ish host for diversity scoring.
pub fn host_of(raw: &str) -> Option<String> {
    Url::parse(raw)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_ascii_lowercase()))
}

/// Collapse a title for near-duplicate detection.
pub fn normalize_title(title: &str) -> String {
    title
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_tracking_and_slash() {
        let a = normalize_url("https://Example.com/path/?utm_source=x&keep=1");
        let b = normalize_url("https://example.com/path?keep=1");
        assert_eq!(a, b);
    }
}
