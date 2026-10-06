#![allow(clippy::vec_init_then_push)]
//! Static IDOR payloads: sequential-ID and UUID probing, path traversal, and
//! mass-assignment variants.
//!
//! These are **data**. The IDOR *prober* — `IdorFuzzer` and its live
//! `reqwest` horizontal/vertical escalation methods — stays in the `eggsec`
//! engine, in `eggsec::fuzzer::payloads::idor`.

use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=1".to_string(),
        description: "IDOR test - sequential ID parameter".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "authorization".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "user_id=1".to_string(),
        description: "IDOR test - alternate parameter name".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "parameter".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=admin".to_string(),
        description: "IDOR test - admin as ID".to_string(),
        severity: Severity::Critical,
        tags: vec!["idor".to_string(), "privilege_escalation".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=2".to_string(),
        description: "IDOR test - sequential increment ID".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "sequential".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=99999".to_string(),
        description: "IDOR test - sequential large ID".to_string(),
        severity: Severity::Medium,
        tags: vec!["idor".to_string(), "sequential".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=550e8400-e29b-41d4-a716-446655440000".to_string(),
        description: "IDOR test - UUID format ID".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "uuid".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "/api/users/2".to_string(),
        description: "IDOR test - path-based IDOR traversal".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "path_traversal".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=MQ==".to_string(),
        description: "IDOR test - base64 encoded ID".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "encoding".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: r#"{"id": 2}"#.to_string(),
        description: "IDOR test - JSON body parameter".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "json".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=-1".to_string(),
        description: "IDOR test - negative ID".to_string(),
        severity: Severity::Critical,
        tags: vec!["idor".to_string(), "negative".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=0".to_string(),
        description: "IDOR test - zero ID".to_string(),
        severity: Severity::Critical,
        tags: vec!["idor".to_string(), "zero".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id[]=1&id[]=2".to_string(),
        description: "IDOR test - array parameter injection".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "array".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "user[id]=2".to_string(),
        description: "IDOR test - nested object parameter".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "nested".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=encrypted_value_here".to_string(),
        description: "IDOR test - encrypted ID value".to_string(),
        severity: Severity::Medium,
        tags: vec!["idor".to_string(), "encrypted".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=00000000-0000-0000-0000-000000000000".to_string(),
        description: "IDOR test - null UUID".to_string(),
        severity: Severity::Critical,
        tags: vec!["idor".to_string(), "uuid".to_string(), "null".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=0x1".to_string(),
        description: "IDOR test - hex formatted ID".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "hex".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "uid=1".to_string(),
        description: "IDOR test - alternate uid parameter".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "parameter".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "account_id=1".to_string(),
        description: "IDOR test - alternate account_id parameter".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "parameter".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "profile_id=1".to_string(),
        description: "IDOR test - alternate profile_id parameter".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "parameter".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=1.0".to_string(),
        description: "IDOR test - decimal formatted ID".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "decimal".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=test".to_string(),
        description: "IDOR test - string ID value".to_string(),
        severity: Severity::Medium,
        tags: vec!["idor".to_string(), "string".to_string()],
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
            assert_eq!(p.payload_type, PayloadType::Idor);
            assert!(!p.payload.is_empty());
            assert!(!p.tags.is_empty());
        }
    }

    #[test]
    fn covers_sequential_uuid_and_path_shapes() {
        let payloads = get_payloads();
        assert!(
            payloads.iter().any(|p| p.payload == "id=1"),
            "sequential ID"
        );
        assert!(
            payloads.iter().any(|p| p.payload == "id=admin"),
            "privilege escalation"
        );
        assert!(
            payloads
                .iter()
                .any(|p| p.payload.contains("550e8400-e29b-41d4")),
            "UUID-format ID"
        );
        assert!(
            payloads.iter().any(|p| p.payload.starts_with("/api/")),
            "path-based IDOR traversal"
        );
    }
}
