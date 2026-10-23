//! Strict, local-only URL filtering. Never load a user's browsing URLs over the network.
use std::net::IpAddr;
use url::{Host, Url};

pub const MAX_URL_LENGTH: usize = 8192;

/// Preserve meaningful paths and query parameters; discard internal and private URLs.
pub fn clean(raw: &str) -> Option<String> {
    let value = raw.trim();
    if value.is_empty() || value.len() > MAX_URL_LENGTH {
        return None;
    }
    let mut url = Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    match url.host()? {
        Host::Domain(domain) => {
            let domain = domain.to_ascii_lowercase();
            if domain == "localhost"
                || domain.ends_with(".localhost")
                || domain.ends_with(".local")
                || domain.ends_with(".internal")
                || !domain.contains('.')
            {
                return None;
            }
        }
        Host::Ipv4(ip) => {
            if !public_ip(IpAddr::V4(ip)) {
                return None;
            }
        }
        Host::Ipv6(ip) => {
            if !public_ip(IpAddr::V6(ip)) {
                return None;
            }
        }
    }
    // Browser histories can contain OAuth callback URLs and bearer credentials.
    // Do not copy those secrets between applications.
    if url.query_pairs().any(|(name, _)| {
        matches!(
            name.to_ascii_lowercase().as_str(),
            "access_token"
                | "id_token"
                | "refresh_token"
                | "token"
                | "api_key"
                | "apikey"
                | "secret"
                | "password"
                | "authorization"
                | "auth_token"
                | "session_token"
                | "code"
                | "oauth_code"
        )
    }) {
        return None;
    }
    // Fragments may contain client-side secrets, and browser history autocomplete
    // does not need them to locate the page.
    url.set_fragment(None);
    Some(url.into())
}

fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            !v.is_private()
                && !v.is_loopback()
                && !v.is_link_local()
                && !v.is_broadcast()
                && !v.is_unspecified()
                && !v.is_documentation()
                && !v.is_multicast()
                && v.octets()[0] != 0
                && v.octets()[0] != 100
                && v.octets()[0] < 224
        }
        IpAddr::V6(v) => {
            !v.is_loopback()
                && !v.is_unspecified()
                && !v.is_multicast()
                && !v.is_unique_local()
                && !v.is_unicast_link_local()
        }
    }
}

/// Safari's `domain_expansion` indexes the host without its final public suffix.
pub fn safari_domain_expansion(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let domain = parsed.domain()?;
    let parts: Vec<_> = domain.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let suffix = parts[parts.len() - 2..].join(".");
    let remove = if matches!(
        suffix.as_str(),
        "co.uk" | "org.uk" | "ac.uk" | "gov.uk" | "com.au" | "com.br" | "co.in" | "co.jp" | "co.nz"
    ) {
        2
    } else {
        1
    };
    if parts.len() <= remove {
        return None;
    }
    Some(parts[..parts.len() - remove].join("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_share_public_web_urls() {
        assert_eq!(
            clean("HTTPS://Example.COM/path?q=hello#section").unwrap(),
            "https://example.com/path?q=hello"
        );
        assert!(clean("chrome://settings/").is_none());
        assert!(clean("file:///private/secret.txt").is_none());
        assert!(clean("http://localhost:3000/").is_none());
        assert!(clean("http://192.168.0.1/").is_none());
        assert!(clean("https://my-company.internal/").is_none());
        assert!(clean("https://example.com/?access_token=secret").is_none());
        assert!(clean("https://example.com/?code=login-secret").is_none());
        assert!(clean("https://user:password@example.com/").is_none());
        assert_eq!(
            clean("https://example.com/articles?a=1&b=2"),
            Some("https://example.com/articles?a=1&b=2".to_owned())
        );
    }

    #[test]
    fn safari_autocomplete_expansion() {
        assert_eq!(
            safari_domain_expansion("https://www.mozilla.org/").as_deref(),
            Some("www.mozilla")
        );
        assert_eq!(
            safari_domain_expansion("https://news.bbc.co.uk/").as_deref(),
            Some("news.bbc")
        );
    }
}
