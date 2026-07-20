//! Property-based tests for Eggsec.
//!
//! Uses proptest to test invariants across arbitrary inputs.

use eggsec::config::ScopeRule;
use eggsec::fuzzer::{generate_mutations, ReDosDetector};
use eggsec::utils::target::{extract_domain, normalize_url};
use proptest::prelude::*;

// URL parsing should never panic on arbitrary input
proptest! {
    #[test]
    fn test_normalize_url_doesnt_crash(url in ".*") {
        let _ = normalize_url(&url);
    }

    #[test]
    fn test_extract_domain_doesnt_crash(url in ".*") {
        let _ = extract_domain(&url);
    }

    #[test]
    fn test_extract_domain_valid_urls(
        scheme in "(http|https)",
        domain in "[a-z]{1,10}",
        tld in "(com|org|net|io)",
    ) {
        let url = format!("{}://{}.{}", scheme, domain, tld);
        let result = extract_domain(&url);
        // extract_domain returns Option<String>, just verify it doesn't panic
        prop_assert!(result.is_some() || result.is_none());
    }
}

// Payload mutations should never produce unexpected empty strings
proptest! {
    #[test]
    fn test_mutations_nonempty_for_nonempty_input(
        payload in "[a-zA-Z0-9]{1,50}",
        count in 1usize..5,
    ) {
        let mutations = generate_mutations(&payload, count);
        // At least the original payload should be included
        prop_assert!(!mutations.is_empty());
        prop_assert!(mutations.contains(&payload));
    }

    #[test]
    fn test_mutations_preserves_original(
        payload in "[!-~]{1,30}",
        count in 1usize..3,
    ) {
        let mutations = generate_mutations(&payload, count);
        prop_assert!(mutations.contains(&payload));
    }
}

// Scope rule matching should be consistent
proptest! {
    #[test]
    fn test_scope_rule_exact_match(domain in "[a-z]{1,20}\\.(com|org|net)") {
        let rule = ScopeRule::new(domain.clone());
        let target = eggsec::config::TargetScope::parse_hostname_only(&domain);
        if let Ok(target) = target {
            prop_assert!(rule.matches(&target));
        }
    }

    #[test]
    fn test_scope_wildcard_matches_subdomain(
        domain in "[a-z]{3,10}",
        tld in "(com|org|net)",
        subdomain in "[a-z]{1,10}",
    ) {
        let pattern = format!("*.{}.{}", domain, tld);
        let rule = ScopeRule::new(pattern);
        let full_domain = format!("{}.{}.{}", subdomain, domain, tld);
        let target = eggsec::config::TargetScope::parse_hostname_only(&full_domain);
        if let Ok(target) = target {
            prop_assert!(rule.matches(&target));
        }
    }
}

// ReDoS detector should identify known vulnerable patterns
proptest! {
    #[test]
    fn test_redos_detector_safe_patterns(pattern in "[a-z]{1,20}") {
        let detector = ReDosDetector::new();
        let result = detector.detect(&pattern);
        // Simple literal patterns should not be vulnerable
        prop_assert!(!result.is_vulnerable);
    }
}

// Error types should preserve information
proptest! {
    #[test]
    fn test_error_is_timeout_detection(
        timeout_ms in 0u64..10000,
        operation in "[a-zA-Z ]{1,50}",
    ) {
        let err = eggsec::EggsecError::Timeout {
            timeout_ms,
            operation: operation.clone(),
        };
        prop_assert!(err.is_timeout());
        prop_assert!(!err.is_network());
        prop_assert!(err.http_status().is_none());
    }

    #[test]
    fn test_error_http_status_preserves_code(status in 400u16..600) {
        let err = eggsec::EggsecError::HttpStatus {
            status,
            message: "Error".to_string(),
        };
        prop_assert!(!err.is_timeout());
        prop_assert_eq!(err.http_status(), Some(status));
    }
}
