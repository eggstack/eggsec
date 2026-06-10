//! OAST (Out-of-Band Application Security Testing) payloads.
//!
//! Provides payloads designed for blind vulnerability detection
//! using OAST techniques with Interactsh-compatible URLs.

use crate::fuzzer::payloads::{Payload, PayloadType};

pub fn get_oast_payloads() -> Vec<Payload> {
    vec![
        Payload {
            payload_type: PayloadType::Oast,
            payload: "{{rootdomain}}".to_string(),
            description: "OAST root domain substitution".to_string(),
            severity: crate::types::Severity::Critical,
            tags: vec!["ssrf".to_string(), "oast".to_string()],
        },
        Payload {
            payload_type: PayloadType::Oast,
            payload: "http://{{rootdomain}}".to_string(),
            description: "OAST HTTP URL with root domain".to_string(),
            severity: crate::types::Severity::Critical,
            tags: vec!["ssrf".to_string(), "oast".to_string()],
        },
        Payload {
            payload_type: PayloadType::Oast,
            payload: "https://{{rootdomain}}".to_string(),
            description: "OAST HTTPS URL with root domain".to_string(),
            severity: crate::types::Severity::Critical,
            tags: vec!["ssrf".to_string(), "oast".to_string()],
        },
        Payload {
            payload_type: PayloadType::Oast,
            payload: "{{rootdomain}}/?v=".to_string(),
            description: "OAST DNS callback via query param".to_string(),
            severity: crate::types::Severity::Critical,
            tags: vec!["ssrf".to_string(), "oast".to_string()],
        },
        Payload {
            payload_type: PayloadType::Oast,
            payload: "{{rootdomain}}#".to_string(),
            description: "OAST DNS callback via fragment".to_string(),
            severity: crate::types::Severity::Critical,
            tags: vec!["ssrf".to_string(), "oast".to_string()],
        },
        Payload {
            payload_type: PayloadType::Oast,
            payload: "http://{{rootdomain}}/.{{random}}.dns".to_string(),
            description: "OAST with random subdomain".to_string(),
            severity: crate::types::Severity::Critical,
            tags: vec!["ssrf".to_string(), "oast".to_string()],
        },
        Payload {
            payload_type: PayloadType::Oast,
            payload: "http://{{rootdomain}}".to_string(),
            description: "OAST HTTP without protocol".to_string(),
            severity: crate::types::Severity::High,
            tags: vec!["ssrf".to_string(), "oast".to_string()],
        },
        Payload {
            payload_type: PayloadType::Oast,
            payload: "{{rootdomain}}".to_string(),
            description: "OAST raw domain without URL".to_string(),
            severity: crate::types::Severity::High,
            tags: vec!["ssrf".to_string(), "oast".to_string()],
        },
        Payload {
            payload_type: PayloadType::Oast,
            payload: "dns://{{rootdomain}}".to_string(),
            description: "OAST DNS scheme".to_string(),
            severity: crate::types::Severity::High,
            tags: vec!["ssrf".to_string(), "oast".to_string()],
        },
        Payload {
            payload_type: PayloadType::Oast,
            payload: "http://{{rootdomain}}/{{random}}".to_string(),
            description: "OAST with random path".to_string(),
            severity: crate::types::Severity::High,
            tags: vec!["ssrf".to_string(), "oast".to_string()],
        },
    ]
}

pub const OAST_PLACEHOLDER: &str = "{{rootdomain}}";
pub const OAST_RANDOM_PLACEHOLDER: &str = "{{random}}";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oast_payloads_exist() {
        let payloads = get_oast_payloads();
        assert!(!payloads.is_empty());
    }

    #[test]
    fn test_oast_payloads_have_correct_type() {
        let payloads = get_oast_payloads();
        for payload in payloads {
            assert_eq!(payload.payload_type, PayloadType::Oast);
        }
    }

    #[test]
    fn test_oast_placeholders_defined() {
        assert!(!OAST_PLACEHOLDER.is_empty());
        assert!(!OAST_RANDOM_PLACEHOLDER.is_empty());
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_oast_payloads();
        assert!(
            payloads.len() >= 5,
            "Must have substantial oast payload coverage, got {}",
            payloads.len()
        );
    }
}
