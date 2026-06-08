use std::net::{IpAddr, SocketAddr};

use crate::error::SlapperError;

/// Parse a string into a `SocketAddr`, returning a descriptive error on failure.
///
/// This replaces the `addr.parse().unwrap()` pattern used throughout the codebase.
pub fn parse_socket_addr(addr: &str) -> Result<SocketAddr, SlapperError> {
    addr.parse().map_err(|e| {
        SlapperError::AddressParse(format!("Invalid socket address '{}': {}", addr, e))
    })
}

pub fn extract_target_from_url(url: &str) -> Option<String> {
    // Try to parse with url crate first to handle auth in URLs
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            if let Some(port) = parsed.port() {
                return Some(format!("{}:{}", host, port));
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

    if target.contains(':') {
        let parts: Vec<&str> = target.splitn(2, ':').collect();
        if parts.len() == 2 {
            if let Ok(port) = parts[1].parse::<u16>() {
                return Some((parts[0].to_string(), port));
            }
        }
    }

    None
}

pub fn is_ip_address(s: &str) -> bool {
    s.parse::<IpAddr>().is_ok()
}

pub fn parse_host_port(target: &str, default_port: u16) -> (String, u16) {
    if target.contains(':') {
        let parts: Vec<&str> = target.splitn(2, ':').collect();
        if parts.len() == 2 {
            let port = parts[1].parse().unwrap_or(default_port);
            return (parts[0].to_string(), port);
        }
    }
    (target.to_string(), default_port)
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

    Some(cleaned.split('/').next()?.split(':').next()?.to_string())
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
