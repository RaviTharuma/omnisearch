//! Grounded snippets: fetch public pages and frame a query-relevant excerpt.

use crate::ssrf::assert_public_http_url;
use crate::types::SearchHit;

/// Rewrite `fallback` using the first query-relevant window in `body`.
pub fn frame_snippet(query: &str, title: &str, body: &str, fallback: &str) -> String {
    let text = strip_markup(body);
    if text.chars().count() < 24 {
        return fallback.to_string();
    }
    let hay = format!("{title} {text}").to_ascii_lowercase();
    let terms: Vec<String> = query
        .split_whitespace()
        .map(|t| {
            t.trim_matches(|c: char| !c.is_alphanumeric())
                .to_ascii_lowercase()
        })
        .filter(|t| t.len() > 2)
        .collect();
    let lower = text.to_ascii_lowercase();
    let idx = terms
        .iter()
        .filter_map(|term| lower.find(term))
        .min()
        .unwrap_or(0);
    if !terms.is_empty() && !terms.iter().any(|t| hay.contains(t.as_str())) {
        return fallback.to_string();
    }
    let start = idx.saturating_sub(80);
    let window: String = text
        .get(start..)
        .unwrap_or(text.as_str())
        .chars()
        .take(280)
        .collect();
    let trimmed = window.trim();
    if trimmed.len() < 24 {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

/// Strip a conservative subset of markup without a full HTML parser.
pub fn strip_markup(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    let mut in_script = false;
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !in_tag && starts_with_ignore(&input[i..], "<script") {
            in_script = true;
            in_tag = true;
            i += 1;
            continue;
        }
        if in_script && starts_with_ignore(&input[i..], "</script") {
            in_script = false;
            in_tag = true;
            i += 1;
            continue;
        }
        let c = input[i..].chars().next().unwrap_or(' ');
        let len = c.len_utf8();
        if c == '<' {
            in_tag = true;
        } else if c == '>' && in_tag {
            in_tag = false;
            if !in_script {
                out.push(' ');
            }
        } else if !in_tag && !in_script {
            out.push(c);
        }
        i += len;
    }
    collapse_ws(&html_unescape(&out))
}

fn starts_with_ignore(input: &str, prefix: &str) -> bool {
    input
        .get(..prefix.len())
        .is_some_and(|s| s.eq_ignore_ascii_case(prefix))
}

fn html_unescape(input: &str) -> String {
    input
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn collapse_ws(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_space = false;
    for c in input.chars() {
        if c.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            prev_space = false;
            out.push(c);
        }
    }
    out.trim().to_string()
}

/// Fetch one public URL and frame a snippet. Returns None on SSRF or empty body.
pub async fn ground_one(
    http: &crate::http::HttpClient,
    query: &str,
    hit: &SearchHit,
) -> Option<String> {
    let url = assert_public_http_url(&hit.url).ok()?;
    let response = http.get(url.as_str()).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let body = response.text().await.ok()?;
    let framed = frame_snippet(query, &hit.title, &body, &hit.snippet);
    if framed.is_empty() || framed == hit.snippet {
        None
    } else {
        Some(framed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_around_query_term() {
        let body = "<html><script>ignore me rust</script><p>Intro filler. The Swiss AI regulation 2026 draft is public.</p></html>";
        let snippet = frame_snippet("Swiss regulation", "News", body, "engine");
        assert!(snippet.contains("Swiss AI regulation"));
        assert!(!snippet.contains("ignore me"));
    }

    #[test]
    fn falls_back_when_body_is_unrelated() {
        let snippet = frame_snippet("quantum", "Title", "<p>weather today in bern</p>", "engine");
        assert_eq!(snippet, "engine");
    }
}
