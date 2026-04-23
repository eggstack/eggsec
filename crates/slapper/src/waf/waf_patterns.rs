use crate::constants::waf::BLOCKED_STATUS_CODES;

pub use crate::waf::data::{get_waf_signatures, WafSignature};

pub fn get_blocked_status_codes() -> Vec<u16> {
    BLOCKED_STATUS_CODES.to_vec()
}

pub fn get_common_waf_response_patterns() -> Vec<&'static str> {
    vec![
        "access denied",
        "blocked by",
        "request blocked",
        "waf",
        "firewall",
        "security",
        "not acceptable",
        "forbidden",
        "unauthorized",
        "your ip has been blocked",
        "rate limit",
        "too many requests",
        "suspicious activity",
        "malicious",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signatures_not_empty() {
        let sigs = get_waf_signatures();
        assert!(!sigs.is_empty());
    }

    #[test]
    fn test_cloudflare_signature_exists() {
        let sigs = get_waf_signatures();
        assert!(sigs.contains_key("cloudflare"));
        let cf = &sigs["cloudflare"];
        assert_eq!(cf.name, "Cloudflare");
        assert!(!cf.headers.is_empty());
        assert!(!cf.cookies.is_empty());
        assert!(!cf.body_patterns.is_empty());
        assert!(!cf.ip_ranges.is_empty());
    }

    #[test]
    fn test_cloudflare_headers_contain_cf_ray() {
        let sigs = get_waf_signatures();
        let cf = &sigs["cloudflare"];
        assert!(cf.headers.iter().any(|h| h.contains("cf-ray")));
        assert!(cf.headers.iter().any(|h| h.contains("cf-cache-status")));
    }

    #[test]
    fn test_cloudflare_cookies() {
        let sigs = get_waf_signatures();
        let cf = &sigs["cloudflare"];
        assert!(cf.cookies.iter().any(|c| c.contains("__cfduid")));
    }

    #[test]
    fn test_akamai_signature_exists() {
        let sigs = get_waf_signatures();
        assert!(sigs.contains_key("akamai"));
        let ak = &sigs["akamai"];
        assert_eq!(ak.name, "Akamai");
        assert!(ak.headers.iter().any(|h| h.contains("akamai")));
    }

    #[test]
    fn test_aws_waf_signature() {
        let sigs = get_waf_signatures();
        assert!(sigs.contains_key("aws_waf"));
        let aws = &sigs["aws_waf"];
        assert_eq!(aws.name, "AWS WAF");
        assert!(aws.headers.iter().any(|h| h.contains("x-amzn")));
        assert!(aws.headers.iter().any(|h| h.contains("awselb")));
    }

    #[test]
    fn test_imperva_signature() {
        let sigs = get_waf_signatures();
        assert!(sigs.contains_key("imperva"));
        let imp = &sigs["imperva"];
        assert_eq!(imp.name, "Imperva");
        assert!(!imp.cookies.is_empty());
        assert!(imp.cookies.iter().any(|c| c.contains("incap_ses")));
    }

    #[test]
    fn test_modsecurity_signature() {
        let sigs = get_waf_signatures();
        assert!(sigs.contains_key("modsecurity"));
        let modsec = &sigs["modsecurity"];
        assert_eq!(modsec.name, "ModSecurity");
        assert!(modsec
            .body_patterns
            .iter()
            .any(|p| p.contains("web application firewall")));
    }

    #[test]
    fn test_wordfence_signature() {
        let sigs = get_waf_signatures();
        assert!(sigs.contains_key("wordfence"));
        let wf = &sigs["wordfence"];
        assert_eq!(wf.name, "Wordfence");
        assert!(wf.headers.iter().any(|h| h.contains("x-wordfence")));
    }

    #[test]
    fn test_generic_waf_block_signature() {
        let sigs = get_waf_signatures();
        assert!(sigs.contains_key("denied_by_waf"));
        let generic = &sigs["denied_by_waf"];
        assert_eq!(generic.name, "Generic WAF Block");
        assert!(generic
            .body_patterns
            .iter()
            .any(|p| p.contains("access denied")));
        assert!(generic.body_patterns.iter().any(|p| p.contains("blocked")));
        assert!(generic.body_patterns.iter().any(|p| p.contains("firewall")));
    }

    #[test]
    fn test_all_signatures_have_names() {
        let sigs = get_waf_signatures();
        for (key, sig) in &sigs {
            assert!(!sig.name.is_empty(), "Signature '{}' has empty name", key);
        }
    }

    #[test]
    fn test_all_signatures_have_at_least_one_indicator() {
        let sigs = get_waf_signatures();
        for (key, sig) in &sigs {
            let has_indicator = !sig.headers.is_empty()
                || !sig.cookies.is_empty()
                || !sig.body_patterns.is_empty()
                || !sig.ip_ranges.is_empty();
            assert!(
                has_indicator,
                "Signature '{}' has no detection indicators",
                key
            );
        }
    }

    #[test]
    fn test_blocked_status_codes() {
        let codes = get_blocked_status_codes();
        assert!(codes.contains(&403));
        assert!(codes.contains(&406));
        assert!(codes.contains(&429));
        assert!(codes.contains(&503));
        assert_eq!(codes.len(), 4);
    }

    #[test]
    fn test_common_waf_response_patterns_not_empty() {
        let patterns = get_common_waf_response_patterns();
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_common_waf_response_patterns_contains_key_terms() {
        let patterns = get_common_waf_response_patterns();
        assert!(patterns.contains(&"access denied"));
        assert!(patterns.contains(&"blocked by"));
        assert!(patterns.contains(&"waf"));
        assert!(patterns.contains(&"firewall"));
        assert!(patterns.contains(&"forbidden"));
        assert!(patterns.contains(&"too many requests"));
    }

    #[test]
    fn test_common_waf_patterns_lowercase() {
        let patterns = get_common_waf_response_patterns();
        for pattern in patterns {
            assert_eq!(
                pattern,
                &pattern.to_lowercase(),
                "Pattern '{}' should be lowercase",
                pattern
            );
        }
    }

    #[test]
    fn test_all_signatures_keyed_by_lowercase() {
        let sigs = get_waf_signatures();
        for key in sigs.keys() {
            assert_eq!(
                key,
                &key.to_lowercase(),
                "Signature key '{}' should be lowercase",
                key
            );
        }
    }

    #[test]
    fn test_ip_ranges_valid_format() {
        let sigs = get_waf_signatures();
        for (key, sig) in &sigs {
            for range in &sig.ip_ranges {
                assert!(
                    range.contains('/'),
                    "IP range '{}' in '{}' should contain '/'",
                    range,
                    key
                );
            }
        }
    }

    #[test]
    fn test_all_signature_names_unique() {
        let sigs = get_waf_signatures();
        let mut names: Vec<&str> = sigs.values().map(|s| s.name.as_str()).collect();
        let before_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), before_len, "Signature names should be unique");
    }
}
