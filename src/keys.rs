//! API key rings and dual-key failover.

use crate::error::Error;

/// Collect `NAME`, `NAME_2`, `NAME_3` (non-empty) from the environment.
pub fn env_key_ring(name: &str) -> Vec<String> {
    let mut keys = Vec::new();
    for suffix in ["", "_2", "_3"] {
        let var = format!("{name}{suffix}");
        if let Ok(value) = std::env::var(&var)
            && !value.is_empty()
        {
            keys.push(value);
        }
    }
    keys
}

/// GitHub token environment names (first non-empty wins, then `_2` / `_3`).
pub const GITHUB_KEY_NAMES: &[&str] = &["GITHUB_TOKEN", "GITHUB_API_KEY", "GH_TOKEN"];

/// Union of several primary names, each with `_2` / `_3` suffixes.
pub fn env_key_rings(names: &[&str]) -> Vec<String> {
    let mut keys = Vec::new();
    for name in names {
        for key in env_key_ring(name) {
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
    }
    keys
}

/// True when another key on the same provider should be tried.
pub fn is_failover(err: &Error) -> bool {
    match err {
        Error::RateLimited { .. } | Error::Timeout { .. } => true,
        Error::Provider { message, .. } => {
            message.contains("HTTP 429")
                || message.contains("HTTP 5")
                || message.contains("transport error")
        }
        _ => false,
    }
}

/// Try each key until one succeeds. Failover only on 429 / 5xx / timeout.
#[macro_export]
macro_rules! try_keys {
    ($keys:expr, $provider:expr, $missing:expr, |$key:ident| $body:expr $(,)?) => {{
        let __keys: &[String] = $keys;
        if __keys.is_empty() {
            Err($crate::error::Error::NotConfigured {
                provider: ($provider).into(),
                reason: ($missing).into(),
            })
        } else {
            let mut __last = None;
            let mut __out = None;
            for (__i, $key) in __keys.iter().enumerate() {
                let $key: &str = $key.as_str();
                match $body {
                    Ok(v) => {
                        __out = Some(Ok(v));
                        break;
                    }
                    Err(err) if __i + 1 < __keys.len() && $crate::keys::is_failover(&err) => {
                        __last = Some(err);
                    }
                    Err(err) => {
                        __out = Some(Err(err));
                        break;
                    }
                }
            }
            __out.unwrap_or_else(|| Err(__last.expect("key failover")))
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_key_names_include_token_and_api_key() {
        assert_eq!(
            GITHUB_KEY_NAMES,
            &["GITHUB_TOKEN", "GITHUB_API_KEY", "GH_TOKEN"]
        );
    }

    #[test]
    fn failover_matches_429_and_5xx() {
        assert!(is_failover(&Error::rate_limited("tavily", "slow")));
        assert!(is_failover(&Error::provider("tavily", "HTTP 503: down")));
        assert!(!is_failover(&Error::provider("tavily", "HTTP 400: bad")));
        assert!(!is_failover(&Error::Invalid("nope".into())));
    }

    #[tokio::test]
    async fn second_key_used_after_429() {
        let keys = vec!["bad".into(), "good".into()];
        let used = crate::try_keys!(&keys, "tavily", "missing", |key| {
            if key == "bad" {
                Err(Error::rate_limited("tavily", "no"))
            } else {
                Ok(key.to_string())
            }
        })
        .unwrap();
        assert_eq!(used, "good");
    }
}
