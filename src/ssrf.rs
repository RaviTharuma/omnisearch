//! SSRF guards for extract/fetch URLs.

use std::net::IpAddr;

use ipnet::IpNet;
use url::Url;

use crate::error::{Error, Result};

/// Return the URL if it is safe to fetch from this process.
pub fn assert_public_http_url(raw: &str) -> Result<Url> {
    let url = Url::parse(raw).map_err(|err| Error::Ssrf(format!("invalid URL: {err}")))?;
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(Error::Ssrf(format!("unsupported scheme '{scheme}'")));
    }
    let host = url
        .host_str()
        .ok_or_else(|| Error::Ssrf("URL missing host".into()))?;
    let host_l = host.to_ascii_lowercase();
    if host_l == "localhost"
        || host_l.ends_with(".localhost")
        || host_l.ends_with(".local")
        || host_l == "metadata.google.internal"
    {
        return Err(Error::Ssrf(format!("blocked host '{host}'")));
    }
    if let Ok(ip) = host.parse::<IpAddr>()
        && is_blocked_ip(ip)
    {
        return Err(Error::Ssrf(format!("blocked address '{ip}'")));
    }
    Ok(url)
}

/// True when an IP must not be fetched.
fn is_blocked_ip(ip: IpAddr) -> bool {
    if ip.is_unspecified() || ip.is_loopback() || ip.is_multicast() {
        return true;
    }
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.octets()[0] == 0
                || (v4.octets()[0] == 169 && v4.octets()[1] == 254)
        }
        IpAddr::V6(v6) => {
            v6.is_unique_local()
                || v6.is_unicast_link_local()
                || IpNet::new(IpAddr::V6(v6), 128)
                    .map(|n| {
                        "fc00::/7"
                            .parse::<IpNet>()
                            .ok()
                            .is_some_and(|p| p.contains(&n.addr()))
                    })
                    .unwrap_or(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::assert_public_http_url;

    #[test]
    fn allows_public_https() {
        assert!(assert_public_http_url("https://example.com/x").is_ok());
    }

    #[test]
    fn blocks_loopback_and_metadata() {
        assert!(assert_public_http_url("http://127.0.0.1/").is_err());
        assert!(assert_public_http_url("http://169.254.169.254/latest").is_err());
        assert!(assert_public_http_url("http://localhost/admin").is_err());
        assert!(assert_public_http_url("file:///etc/passwd").is_err());
    }
}
