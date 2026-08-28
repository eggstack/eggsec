use std::net::{IpAddr, SocketAddr};

use crate::error::EggsecError;

/// Parse a string into a `SocketAddr`, returning a descriptive error on failure.
///
/// This replaces the `addr.parse().unwrap()` pattern used throughout the codebase.
pub fn parse_socket_addr(addr: &str) -> Result<SocketAddr, EggsecError> {
    addr.parse()
        .map_err(|e| EggsecError::AddressParse(format!("Invalid socket address '{}': {}", addr, e)))
}

pub fn extract_target_from_url(url: &str) -> Option<String> {
    // Try to parse with url crate first to handle auth in URLs
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            if let Some(port) = parsed.port() {
                return Some(if host.parse::<IpAddr>().is_ok_and(|ip| ip.is_ipv6()) {
                    format!("[{}]:{}", host, port)
                } else {
                    format!("{}:{}", host, port)
                });
            }
            return Some(host.to_string());
        }
    }

    // Fallback for URLs without scheme
    url.trim_start_matches("http://")
        .trim_start_matches("https://")
        .split('/')
        .next()
        .map(|s| s.to_string())
}

pub fn extract_host_port(url: &str) -> Option<(String, u16)> {
    let target = extract_target_from_url(url)?;
    let (host, port) = parse_host_port(&target, 0);
    (port != 0).then_some((host, port))
}

pub fn is_ip_address(s: &str) -> bool {
    s.parse::<IpAddr>().is_ok()
}

pub fn parse_host_port(target: &str, default_port: u16) -> (String, u16) {
    if let Ok(addr) = target.parse::<SocketAddr>() {
        return (addr.ip().to_string(), addr.port());
    }

    if let Some((host, port)) = target.rsplit_once("]:") {
        if let Ok(port) = port.parse() {
            return (host.trim_start_matches('[').to_string(), port);
        }
    }

    if target.matches(':').count() == 1 {
        if let Some((host, port)) = target.split_once(':') {
            return (host.to_string(), port.parse().unwrap_or(default_port));
        }
    }

    (
        target
            .strip_prefix('[')
            .and_then(|host| host.strip_suffix(']'))
            .unwrap_or(target)
            .to_string(),
        default_port,
    )
}

pub fn normalize_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{}", url)
    }
}

pub fn strip_url_protocol(url: &str) -> &str {
    url.strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .unwrap_or(url)
}

pub fn extract_domain(url: &str) -> Option<String> {
    let cleaned = url
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_start_matches("www.");

    let host = cleaned.split('/').next()?;
    Some(parse_host_port(host, 0).0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_target_from_url() {
        assert_eq!(
            extract_target_from_url("https://example.com"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_target_from_url("http://example.com/path"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_target_from_url("example.com:8080"),
            Some("example.com:8080".to_string())
        );
    }

    #[test]
    fn test_is_ip_address() {
        assert!(is_ip_address("192.168.1.1"));
        assert!(is_ip_address("::1"));
        assert!(!is_ip_address("example.com"));
    }

    #[test]
    fn test_parse_host_port() {
        assert_eq!(
            parse_host_port("example.com", 80),
            ("example.com".to_string(), 80)
        );
        assert_eq!(
            parse_host_port("example.com:8080", 80),
            ("example.com".to_string(), 8080)
        );
        assert_eq!(
            parse_host_port("192.168.1.1:443", 80),
            ("192.168.1.1".to_string(), 443)
        );
        assert_eq!(
            parse_host_port("2001:db8::1", 80),
            ("2001:db8::1".to_string(), 80)
        );
        assert_eq!(parse_host_port("[::1]:8080", 80), ("::1".to_string(), 8080));
    }

    #[test]
    fn test_extract_ipv6_host_port() {
        assert_eq!(
            extract_host_port("http://[::1]:8080/path"),
            Some(("::1".to_string(), 8080))
        );
        assert_eq!(
            extract_domain("https://2001:db8::1"),
            Some("2001:db8::1".into())
        );
    }

    #[test]
    fn test_normalize_url() {
        assert_eq!(normalize_url("example.com"), "https://example.com");
        assert_eq!(normalize_url("http://example.com"), "http://example.com");
        assert_eq!(normalize_url("https://example.com"), "https://example.com");
    }

    #[test]
    fn test_parse_socket_addr() {
        assert!(parse_socket_addr("127.0.0.1:8080").is_ok());
        assert!(parse_socket_addr("[::1]:8080").is_ok());
        assert!(parse_socket_addr("invalid").is_err());
        assert!(parse_socket_addr("127.0.0.1:99999").is_err());
        assert!(parse_socket_addr("").is_err());
    }
}
