use anyhow::{anyhow, Result};
use std::collections::BTreeSet;
use std::net::{IpAddr, ToSocketAddrs};

pub fn parse_ports(port_spec: &str) -> Result<Vec<u16>> {
    let mut ports = BTreeSet::new();

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
            ports.insert(part.parse()?);
        }
    }

    Ok(ports.into_iter().collect())
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

    let ip = addrs
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Could not resolve host: {}", host))?;

    if ip.is_loopback() {
        anyhow::bail!("Resolved to loopback address blocked");
    }
    if is_private_ip(&ip) {
        anyhow::bail!("Resolved to private IP address blocked");
    }
    Ok(ip)
}

fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            octets[0] == 10
                || (octets[0] == 172 && (15..=31).contains(&octets[1]))
                || (octets[0] == 192 && octets[1] == 168)
                || (octets[0] == 169 && octets[1] == 254)
                || (octets[0] == 127)
        }
        IpAddr::V6(ipv6) => {
            ipv6.is_loopback()
                || (0xfc00..=0xfdff).contains(&ipv6.segments()[0])
                || (0xfe80..=0xfebf).contains(&ipv6.segments()[0])
        }
    }
}

#[inline]
pub fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    let haystack_bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();

    if needle_bytes.len() > haystack_bytes.len() {
        return false;
    }

    haystack_bytes.windows(needle_bytes.len()).any(|window| {
        // SAFETY: We're comparing bytes from valid UTF-8 strings
        let candidate = std::str::from_utf8(window).unwrap_or("");
        candidate.eq_ignore_ascii_case(needle)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

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
    fn test_parse_ports_deduplicates_overlapping_ranges() {
        let ports = parse_ports("80-82,81-83").unwrap();
        assert_eq!(ports, vec![80, 81, 82, 83]);
    }

    #[test]
    fn test_parse_ports_deduplicates_exact_duplicates() {
        let ports = parse_ports("80,80,80").unwrap();
        assert_eq!(ports, vec![80]);
    }

    #[test]
    fn test_resolve_host_blocks_private() {
        let result = resolve_host("localhost");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("loopback"));

        let result = resolve_host("192.168.1.1");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("private"));
    }

    #[test]
    fn test_is_private_ip_ipv6() {
        use std::str::FromStr;

        // fc00::/7 (unique-local)
        let ip = IpAddr::from_str("fc00::1").unwrap();
        assert!(is_private_ip(&ip), "fc00::1 should be private");
        let ip = IpAddr::from_str("fd00::1").unwrap();
        assert!(is_private_ip(&ip), "fd00::1 should be private");
        let ip = IpAddr::from_str("fdff::1").unwrap();
        assert!(is_private_ip(&ip), "fdff::1 should be private");

        // fe80::/10 (link-local)
        let ip = IpAddr::from_str("fe80::1").unwrap();
        assert!(is_private_ip(&ip), "fe80::1 should be private");
        let ip = IpAddr::from_str("febf::1").unwrap();
        assert!(is_private_ip(&ip), "febf::1 should be private");

        // Public addresses that should NOT match
        let ip = IpAddr::from_str("2001:db8::1").unwrap();
        assert!(!is_private_ip(&ip), "2001:db8::1 should not be private");
        let ip = IpAddr::from_str("fe00::1").unwrap();
        assert!(!is_private_ip(&ip), "fe00::1 should not be private");
        let ip = IpAddr::from_str("fec0::1").unwrap();
        assert!(!is_private_ip(&ip), "fec0::1 should not be private");
    }

    #[test]
    fn test_parse_ports_invalid_range() {
        assert!(parse_ports("100-50").is_err());
    }

    #[test]
    fn test_parse_ports_invalid_format() {
        assert!(parse_ports("abc").is_err());
    }

    proptest! {
        #[test]
        fn test_parse_ports_all_valid_u16(port in 1u16..65535) {
            let spec = port.to_string();
            let ports = parse_ports(&spec).unwrap();
            prop_assert_eq!(ports, vec![port]);
        }

        #[test]
        fn test_parse_ports_range_property(start in 1u16..65530, len in 1u16..10) {
            let end = start.saturating_add(len).min(65535);
            let spec = format!("{}-{}", start, end);
            let ports = parse_ports(&spec).unwrap();
            let expected_count = (end - start + 1) as usize;
            prop_assert_eq!(ports.len(), expected_count);
            prop_assert_eq!(ports[0], start);
            prop_assert_eq!(*ports.last().unwrap(), end);
        }

        #[test]
        fn test_parse_ports_returns_valid_ports(port in 1u16..65535) {
            let spec = format!("{},80,443", port);
            let ports = parse_ports(&spec).unwrap();
            for p in &ports {
                prop_assert!(*p >= 1);
            }
        }
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

    proptest! {
        #[test]
        fn test_parse_headers_key_always_nonempty(key in "[a-zA-Z][a-zA-Z0-9-]*", value in "[ -~]{0,50}") {
            let header = format!("{}: {}", key, value);
            let parsed = parse_headers(&[header]);
            prop_assert!(!parsed.is_empty());
            prop_assert_eq!(&parsed[0].0, key.trim());
        }
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

    proptest! {
        #[test]
        fn test_parse_url_validated_accepts_http_https(host in "[a-z][a-z0-9]{0,10}\\.[a-z]{2,4}", scheme in proptest::sample::select(vec!["http", "https"])) {
            let url = format!("{}://{}/path", scheme, host);
            let parsed = parse_url_validated(&url);
            prop_assert!(parsed.is_ok());
            let parsed = parsed.unwrap();
            prop_assert_eq!(parsed.scheme(), scheme);
        }

        #[test]
        fn test_parse_url_validated_rejects_non_http(scheme in proptest::sample::select(vec!["ftp", "file", "ssh", "telnet", "mailto"])) {
            let url = format!("{}://example.com", scheme);
            let result = parse_url_validated(&url);
            prop_assert!(result.is_err());
        }
    }
}
