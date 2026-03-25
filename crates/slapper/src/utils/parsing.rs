
use anyhow::{anyhow, Result};
use std::net::{IpAddr, ToSocketAddrs};

pub fn parse_ports(port_spec: &str) -> Result<Vec<u16>> {
    let mut ports = Vec::new();

    for part in port_spec.split(',') {
        let part = part.trim();
        if part.contains('-') {
            let range: Vec<&str> = part.split('-').collect();
            if range.len() != 2 {
                return Err(anyhow!("Invalid port range: {}", part));
            }
            let start: u16 = range[0].parse()?;
            let end: u16 = range[1].parse()?;
            if start > end {
                return Err(anyhow!("Invalid port range: {} (start > end)", part));
            }
            ports.extend(start..=end);
        } else {
            ports.push(part.parse()?);
        }
    }

    Ok(ports)
}

pub fn parse_headers(headers: &[String]) -> Vec<(String, String)> {
    headers
        .iter()
        .filter_map(|h| {
            let parts: Vec<&str> = h.splitn(2, ':').collect();
            if parts.len() == 2 {
                Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
            } else {
                None
            }
        })
        .collect()
}

pub fn parse_url(url: &str) -> Result<url::Url, anyhow::Error> {
    url::Url::parse(url).map_err(|e| anyhow::anyhow!("{}", e))
}

pub fn parse_url_validated(url: &str) -> Result<url::Url> {
    let parsed = url::Url::parse(url)?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(anyhow!("URL must use http or https scheme"));
    }
    if parsed.host_str().is_none() {
        return Err(anyhow!("URL must have a host"));
    }
    Ok(parsed)
}

pub fn resolve_host(host: &str) -> Result<IpAddr> {
    let addrs: Vec<IpAddr> = (host, 0).to_socket_addrs()?.map(|sa| sa.ip()).collect();

    addrs
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Could not resolve host: {}", host))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ports_single() {
        let ports = parse_ports("80").unwrap();
        assert_eq!(ports, vec![80]);
    }

    #[test]
    fn test_parse_ports_multiple() {
        let ports = parse_ports("80,443,8080").unwrap();
        assert_eq!(ports, vec![80, 443, 8080]);
    }

    #[test]
    fn test_parse_ports_range() {
        let ports = parse_ports("80-83").unwrap();
        assert_eq!(ports, vec![80, 81, 82, 83]);
    }

    #[test]
    fn test_parse_ports_mixed() {
        let ports = parse_ports("22,80-82,443").unwrap();
        assert_eq!(ports, vec![22, 80, 81, 82, 443]);
    }

    #[test]
    fn test_parse_ports_invalid_range() {
        assert!(parse_ports("100-50").is_err());
    }

    #[test]
    fn test_parse_ports_invalid_format() {
        assert!(parse_ports("abc").is_err());
    }

    #[test]
    fn test_parse_headers_valid() {
        let headers = vec!["Content-Type: application/json".to_string()];
        let parsed = parse_headers(&headers);
        assert_eq!(
            parsed,
            vec![("Content-Type".to_string(), "application/json".to_string())]
        );
    }

    #[test]
    fn test_parse_headers_multiple() {
        let headers = vec![
            "Content-Type: application/json".to_string(),
            "Authorization: Bearer token".to_string(),
        ];
        let parsed = parse_headers(&headers);
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn test_parse_headers_invalid() {
        let headers = vec!["InvalidHeader".to_string()];
        let parsed = parse_headers(&headers);
        assert!(parsed.is_empty());
    }

    #[test]
    fn test_parse_url_valid() {
        let url = parse_url("https://example.com/path").unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("example.com"));
    }

    #[test]
    fn test_parse_url_invalid_scheme() {
        assert!(parse_url_validated("ftp://example.com").is_err());
    }

    #[test]
    fn test_parse_url_validated_valid() {
        let url = parse_url_validated("https://example.com/path").unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("example.com"));
    }

    #[test]
    fn test_parse_url_validated_no_host() {
        assert!(parse_url_validated("https://").is_err());
    }
}
