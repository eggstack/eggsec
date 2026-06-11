#![allow(async_fn_in_trait)]

use crate::fuzzer::engine::FuzzResult;
pub use crate::fuzzer::payloads::graphql::{
    GraphQLFuzzer, GraphQLTestResult, GraphQLVulnerability,
};
pub use crate::fuzzer::payloads::grpc::{GrpcFuzzer, GrpcTestResult, GrpcVulnerability};
pub use crate::fuzzer::payloads::idor::{IdorFuzzer, IdorTestResult};
pub use crate::fuzzer::payloads::jwt::{JwtFuzzer, JwtTestResult};
pub use crate::fuzzer::payloads::oauth::{OAuthFuzzer, OAuthTestResult};
pub use crate::fuzzer::payloads::ssti::{SstiFuzzer, SstiTestResult, TemplateEngine};
pub use crate::fuzzer::payloads::websocket::{
    WebSocketFuzzer, WebSocketTestResult, WebSocketVulnerability,
};
use crate::fuzzer::payloads::{Payload, PayloadType};
use reqwest::Client;

pub trait AdvancedFuzzer {
    async fn fuzz(&mut self, client: &Client) -> Vec<FuzzResult>;
    fn name(&self) -> &str;
}

pub trait FuzzerResultConverter<T> {
    fn into_fuzz_result(self) -> FuzzResult;
}

impl AdvancedFuzzer for GraphQLFuzzer {
    async fn fuzz(&mut self, client: &reqwest::Client) -> Vec<FuzzResult> {
        let mut results = Vec::new();

        if self.enable_introspection {
            if let Ok(true) = self.run_introspection(client).await {
                let introspection_results = self.test_introspection_enabled();
                for r in introspection_results {
                    results.push(r.into_fuzz_result());
                }
            }
        }

        let injection_results =
            self.generate_injection_queries(self.enable_depth_bypass, self.enable_alias_overload);
        for r in injection_results {
            results.push(r.into_fuzz_result());
        }

        let batch_results = self.generate_batch_queries(self.enable_alias_overload);
        for r in batch_results {
            results.push(r.into_fuzz_result());
        }

        results
    }

    fn name(&self) -> &str {
        "graphql"
    }
}

impl FuzzerResultConverter<GraphQLTestResult> for GraphQLTestResult {
    fn into_fuzz_result(self) -> FuzzResult {
        let description = self.description.clone();
        FuzzResult {
            payload: Payload {
                payload_type: PayloadType::GraphQL,
                payload: self.query,
                description,
                severity: self.severity,
                tags: vec![format!("{:?}", self.vulnerability)],
            },
            status_code: if self.success { 200 } else { 500 },
            response_time_ms: 0,
            response_length: Some(self.response_snippet.len() as u64),
            response_body: Some(self.response_snippet),
            is_waf_blocked: false,
            is_anomaly: self.success,
            is_redos_suspected: false,
            leaks_found: if self.success {
                vec![self.description]
            } else {
                vec![]
            },
            error: None,
            owasp_category: Some(crate::waf::types::OwaspCategory::A03_2021_Injection.to_string()),
            detected_severity: self.severity,
        }
    }
}

impl AdvancedFuzzer for JwtFuzzer {
    async fn fuzz(&mut self, client: &reqwest::Client) -> Vec<FuzzResult> {
        let mut results = Vec::new();

        let target_url = self.target_url.clone().unwrap_or_default();

        if !target_url.is_empty() {
            let mut fuzzer_with_client = JwtFuzzer::new()
                .with_target_url(target_url.clone())
                .with_client(client.clone());

            if let Some(ref token) = self.original_token {
                fuzzer_with_client = fuzzer_with_client.with_original_token(token.clone());
            }

            let none_results = fuzzer_with_client.test_none_algorithm_attack().await;
            for r in none_results {
                results.push(r.into_fuzz_result());
            }

            if let Some(ref token) = self.original_token {
                let server_results = JwtFuzzer::new()
                    .with_target_url(target_url)
                    .with_client(client.clone())
                    .test_token_against_server(token)
                    .await;
                for r in server_results {
                    results.push(r.into_fuzz_result());
                }
            }
        }

        let none_results = self.test_none_algorithm();
        for r in none_results {
            results.push(r.into_fuzz_result());
        }

        let key_results = self.test_key_injection("");
        for r in key_results {
            results.push(r.into_fuzz_result());
        }

        results
    }

    fn name(&self) -> &str {
        "jwt"
    }
}

impl FuzzerResultConverter<JwtTestResult> for JwtTestResult {
    fn into_fuzz_result(self) -> FuzzResult {
        let description = self.description.clone();
        FuzzResult {
            payload: Payload {
                payload_type: PayloadType::Jwt,
                payload: self.token,
                description,
                severity: self.severity,
                tags: vec![format!("{:?}", self.vulnerability)],
            },
            status_code: if self.success { 200 } else { 401 },
            response_time_ms: 0,
            response_length: None,
            response_body: None,
            is_waf_blocked: false,
            is_anomaly: self.success,
            is_redos_suspected: false,
            leaks_found: if self.success {
                vec![self.description]
            } else {
                vec![]
            },
            error: None,
            owasp_category: Some(
                crate::waf::types::OwaspCategory::A02_2023_BrokenAuthentication.to_string(),
            ),
            detected_severity: self.severity,
        }
    }
}

impl AdvancedFuzzer for OAuthFuzzer {
    async fn fuzz(&mut self, client: &reqwest::Client) -> Vec<FuzzResult> {
        let mut results = Vec::new();

        let mut fuzzer_with_client =
            OAuthFuzzer::new(self.client_id.clone(), self.redirect_uri.clone())
                .with_client(client.clone());

        if let Some(ref issuer) = self.issuer_url {
            fuzzer_with_client = fuzzer_with_client.with_issuer(issuer.clone());

            if self.enable_redirect || self.enable_scope || self.enable_state || self.enable_grant {
                let issuer_results = fuzzer_with_client.test_issuer().await;
                for r in issuer_results {
                    results.push(r.into_fuzz_result());
                }
            }
        }

        if self.enable_redirect {
            let issuer = self
                .issuer_url
                .clone()
                .unwrap_or_else(|| "https://example.com".to_string());
            let endpoints = self.discover_endpoints(&issuer);

            for endpoint in endpoints {
                let redirect_results = self.test_redirect_uri(&endpoint.url);
                for r in redirect_results {
                    results.push(r.into_fuzz_result());
                }
            }
        }

        if self.enable_scope {
            let issuer = self
                .issuer_url
                .clone()
                .unwrap_or_else(|| "https://example.com".to_string());
            let scope_results = self.test_scope_escalation(&format!("{}/authorize", issuer));
            for r in scope_results {
                results.push(r.into_fuzz_result());
            }
        }

        if self.enable_state {
            let issuer = self
                .issuer_url
                .clone()
                .unwrap_or_else(|| "https://example.com".to_string());
            let state_results = self.test_state_parameter(&format!("{}/authorize", issuer));
            for r in state_results {
                results.push(r.into_fuzz_result());
            }
        }

        if self.enable_grant {
            let issuer = self
                .issuer_url
                .clone()
                .unwrap_or_else(|| "https://example.com".to_string());
            let grant_results = self.test_grant_type_mixing(&format!("{}/token", issuer));
            for r in grant_results {
                results.push(r.into_fuzz_result());
            }
        }

        results
    }

    fn name(&self) -> &str {
        "oauth"
    }
}

impl FuzzerResultConverter<OAuthTestResult> for OAuthTestResult {
    fn into_fuzz_result(self) -> FuzzResult {
        let description = self.description.clone();
        FuzzResult {
            payload: Payload {
                payload_type: PayloadType::OAuth,
                payload: self.endpoint,
                description,
                severity: self.severity,
                tags: vec![format!("{:?}", self.vulnerability)],
            },
            status_code: if self.success { 200 } else { 400 },
            response_time_ms: 0,
            response_length: None,
            response_body: None,
            is_waf_blocked: false,
            is_anomaly: self.success,
            is_redos_suspected: false,
            leaks_found: if self.success {
                vec![self.description]
            } else {
                vec![]
            },
            error: None,
            owasp_category: Some(
                crate::waf::types::OwaspCategory::A02_2023_BrokenAuthentication.to_string(),
            ),
            detected_severity: self.severity,
        }
    }
}

impl AdvancedFuzzer for IdorFuzzer {
    async fn fuzz(&mut self, client: &reqwest::Client) -> Vec<FuzzResult> {
        let mut results = Vec::new();

        let mut fuzzer_with_client =
            IdorFuzzer::new(self.base_url.clone()).with_client(client.clone());

        if let Some(ref base_id) = self.base_user_id {
            fuzzer_with_client = fuzzer_with_client.with_base_user_id(base_id.clone());
        }

        if !self.user_ids.is_empty() {
            fuzzer_with_client = fuzzer_with_client.with_user_ids(self.user_ids.clone());
        }

        if let Some(ref cookies) = self.authenticated_cookies {
            fuzzer_with_client = fuzzer_with_client.with_authentication(cookies.clone());
        }

        let horizontal_results = fuzzer_with_client.test_horizontal_escalation().await;
        for r in horizontal_results {
            results.push(r.into_fuzz_result());
        }

        let vertical_results = fuzzer_with_client.test_vertical_escalation().await;
        for r in vertical_results {
            results.push(r.into_fuzz_result());
        }

        results
    }

    fn name(&self) -> &str {
        "idor"
    }
}

impl FuzzerResultConverter<IdorTestResult> for IdorTestResult {
    fn into_fuzz_result(self) -> FuzzResult {
        let description = self.description.clone();
        FuzzResult {
            payload: Payload {
                payload_type: PayloadType::Idor,
                payload: self.endpoint,
                description,
                severity: self.severity,
                tags: vec![format!("{:?}", self.vulnerability)],
            },
            status_code: if self.success {
                200
            } else {
                crate::constants::STATUS_FORBIDDEN
            },
            response_time_ms: 0,
            response_length: None,
            response_body: None,
            is_waf_blocked: false,
            is_anomaly: self.success,
            is_redos_suspected: false,
            leaks_found: if self.success {
                vec![self.description]
            } else {
                vec![]
            },
            error: None,
            owasp_category: Some(
                crate::waf::types::OwaspCategory::A01_2023_BrokenObjectLevelAuthorization
                    .to_string(),
            ),
            detected_severity: self.severity,
        }
    }
}

impl AdvancedFuzzer for SstiFuzzer {
    async fn fuzz(&mut self, client: &reqwest::Client) -> Vec<FuzzResult> {
        let mut results = Vec::new();

        let mut fuzzer_with_client = SstiFuzzer::new().with_client(client.clone());

        if let Some(ref url) = self.target_url {
            fuzzer_with_client = fuzzer_with_client.with_target_url(url.clone());
        }

        if let Some(ref param) = self.param_name {
            fuzzer_with_client = fuzzer_with_client.with_param_name(param.clone());
        }

        let server_results = fuzzer_with_client.test_ssti_on_server().await;
        for r in server_results {
            results.push(r.into_fuzz_result());
        }

        let payloads = self.generate_payloads();
        for r in payloads {
            results.push(r.into_fuzz_result());
        }

        results
    }

    fn name(&self) -> &str {
        "ssti"
    }
}

impl FuzzerResultConverter<SstiTestResult> for SstiTestResult {
    fn into_fuzz_result(self) -> FuzzResult {
        let description = self.description.clone();
        FuzzResult {
            payload: Payload {
                payload_type: PayloadType::Ssti,
                payload: self.payload,
                description,
                severity: self.severity,
                tags: vec![format!("{:?}", self.engine)],
            },
            status_code: if self.success { 500 } else { 200 },
            response_time_ms: 0,
            response_length: None,
            response_body: None,
            is_waf_blocked: false,
            is_anomaly: self.success,
            is_redos_suspected: false,
            leaks_found: if self.success {
                vec![self.description]
            } else {
                vec![]
            },
            error: None,
            owasp_category: Some(crate::waf::types::OwaspCategory::A03_2021_Injection.to_string()),
            detected_severity: self.severity,
        }
    }
}

impl AdvancedFuzzer for WebSocketFuzzer {
    async fn fuzz(&mut self, _client: &reqwest::Client) -> Vec<FuzzResult> {
        let mut results = Vec::new();

        #[cfg(not(feature = "websocket"))]
        {
            let tests = self.generate_all_tests();
            for r in tests {
                results.push(r.into_fuzz_result());
            }
        }

        #[cfg(feature = "websocket")]
        {
            let tests = self.generate_all_tests();
            for r in tests {
                if r.vulnerability
                    != crate::fuzzer::payloads::websocket::WebSocketVulnerability::Injection
                {
                    results.push(r.into_fuzz_result());
                }
            }

            let injection_payloads: Vec<String> = self
                .generate_injection_tests()
                .iter()
                .map(|t| t.message.clone())
                .collect();

            let config = crate::websocket::WebSocketTestConfig {
                url: self.url.clone(),
                timeout_secs: 10,
                injection_payloads,
                test_connection: true,
                test_origins: true,
                test_injection: true,
                test_dos: true,
                test_message_fuzz: true,
            };

            let report = crate::websocket::run_live_tests(&config).await;

            if let Some(conn) = &report.connection_test {
                results.push(FuzzResult {
                    payload: Payload {
                        payload_type: PayloadType::Websocket,
                        payload: conn.url.clone(),
                        description: "WebSocket connection test".to_string(),
                        severity: if conn.connected {
                            Severity::Info
                        } else {
                            Severity::High
                        },
                        tags: vec!["websocket".to_string(), "connection".to_string()],
                    },
                    status_code: if conn.connected { 101 } else { 0 },
                    response_time_ms: conn.latency_ms.unwrap_or(0.0) as u64,
                    response_length: None,
                    response_body: conn.error.clone(),
                    is_waf_blocked: false,
                    is_anomaly: !conn.connected,
                    is_redos_suspected: false,
                    leaks_found: Vec::new(),
                    error: conn.error.clone(),
                    owasp_category: None,
                    detected_severity: if conn.connected {
                        Severity::Info
                    } else {
                        Severity::High
                    },
                });
            }

            for test in &report.origin_tests {
                results.push(FuzzResult {
                    payload: Payload {
                        payload_type: PayloadType::Websocket,
                        payload: test.origin.clone(),
                        description: format!("Origin validation: {}", test.details),
                        severity: if test.accepted {
                            Severity::High
                        } else {
                            Severity::Info
                        },
                        tags: vec![
                            "websocket".to_string(),
                            "origin".to_string(),
                            "cswsh".to_string(),
                        ],
                    },
                    status_code: test.status_code.unwrap_or(0),
                    response_time_ms: 0,
                    response_length: None,
                    response_body: Some(test.details.clone()),
                    is_waf_blocked: false,
                    is_anomaly: test.accepted,
                    is_redos_suspected: false,
                    leaks_found: if test.accepted {
                        vec![format!("Origin '{}' accepted", test.origin)]
                    } else {
                        Vec::new()
                    },
                    error: None,
                    owasp_category: Some(
                        crate::waf::types::OwaspCategory::A01_2021_BrokenAccessControl.to_string(),
                    ),
                    detected_severity: if test.accepted {
                        Severity::High
                    } else {
                        Severity::Info
                    },
                });
            }

            for test in &report.injection_tests {
                results.push(FuzzResult {
                    payload: Payload {
                        payload_type: PayloadType::Websocket,
                        payload: test.payload.clone(),
                        description: test.details.clone(),
                        severity: if test.vulnerability_detected {
                            Severity::Critical
                        } else {
                            Severity::Info
                        },
                        tags: vec!["websocket".to_string(), "injection".to_string()],
                    },
                    status_code: if test.sent { 200 } else { 0 },
                    response_time_ms: 0,
                    response_length: test.response_content.as_ref().map(|r| r.len() as u64),
                    response_body: test.response_content.clone(),
                    is_waf_blocked: false,
                    is_anomaly: test.vulnerability_detected,
                    is_redos_suspected: false,
                    leaks_found: if test.vulnerability_detected {
                        vec![test.details.clone()]
                    } else {
                        Vec::new()
                    },
                    error: None,
                    owasp_category: Some(
                        crate::waf::types::OwaspCategory::A03_2021_Injection.to_string(),
                    ),
                    detected_severity: if test.vulnerability_detected {
                        Severity::Critical
                    } else {
                        Severity::Info
                    },
                });
            }

            for test in &report.fuzz_tests {
                results.push(FuzzResult {
                    payload: Payload {
                        payload_type: PayloadType::Websocket,
                        payload: test.test_name.clone(),
                        description: test.details.clone(),
                        severity: if test.vulnerability_detected {
                            Severity::Medium
                        } else {
                            Severity::Info
                        },
                        tags: vec!["websocket".to_string(), "fuzzing".to_string()],
                    },
                    status_code: if test.sent { 200 } else { 0 },
                    response_time_ms: 0,
                    response_length: test.server_response.as_ref().map(|r| r.len() as u64),
                    response_body: test.server_response.clone(),
                    is_waf_blocked: false,
                    is_anomaly: test.vulnerability_detected,
                    is_redos_suspected: false,
                    leaks_found: if test.vulnerability_detected {
                        vec![test.details.clone()]
                    } else {
                        Vec::new()
                    },
                    error: None,
                    owasp_category: Some(
                        crate::waf::types::OwaspCategory::A05_2021_SecurityMisconfiguration
                            .to_string(),
                    ),
                    detected_severity: if test.vulnerability_detected {
                        Severity::Medium
                    } else {
                        Severity::Info
                    },
                });
            }
        }

        results
    }

    fn name(&self) -> &str {
        "websocket"
    }
}

impl FuzzerResultConverter<WebSocketTestResult> for WebSocketTestResult {
    fn into_fuzz_result(self) -> FuzzResult {
        use crate::fuzzer::payloads::websocket::WebSocketVulnerability;

        let description = self.description.clone();
        let owasp_category = match self.vulnerability {
            WebSocketVulnerability::Injection => {
                Some(crate::waf::types::OwaspCategory::A03_2021_Injection.to_string())
            }
            WebSocketVulnerability::DoS => Some(
                crate::waf::types::OwaspCategory::A05_2021_SecurityMisconfiguration.to_string(),
            ),
            WebSocketVulnerability::CrossSiteWebSocketHijacking => {
                Some(crate::waf::types::OwaspCategory::A01_2021_BrokenAccessControl.to_string())
            }
            WebSocketVulnerability::OriginBypass => {
                Some(crate::waf::types::OwaspCategory::A01_2021_BrokenAccessControl.to_string())
            }
            WebSocketVulnerability::MessageFuzzing | WebSocketVulnerability::FrameFuzzing => {
                Some(crate::waf::types::OwaspCategory::A03_2021_Injection.to_string())
            }
            WebSocketVulnerability::AuthBypass => {
                Some(crate::waf::types::OwaspCategory::A07_2021_AuthFailures.to_string())
            }
        };

        let mut tags = vec!["websocket".to_string(), format!("{:?}", self.vulnerability)];
        if self.success {
            tags.push("confirmed".to_string());
        }

        FuzzResult {
            payload: Payload {
                payload_type: PayloadType::Websocket,
                payload: self.message,
                description,
                severity: self.severity,
                tags,
            },
            status_code: if self.success { 101 } else { 400 },
            response_time_ms: 0,
            response_length: None,
            response_body: if self.response_snippet.is_empty() {
                None
            } else {
                Some(self.response_snippet)
            },
            is_waf_blocked: false,
            is_anomaly: self.success,
            is_redos_suspected: false,
            leaks_found: if self.success {
                vec![self.description]
            } else {
                vec![]
            },
            error: None,
            owasp_category,
            detected_severity: self.severity,
        }
    }
}

impl AdvancedFuzzer for GrpcFuzzer {
    async fn fuzz(&mut self, _client: &reqwest::Client) -> Vec<FuzzResult> {
        let mut results = Vec::new();

        let injection_tests = self.generate_injection_tests();
        for r in injection_tests {
            results.push(r.into_fuzz_result());
        }

        results
    }

    fn name(&self) -> &str {
        "grpc"
    }
}

impl FuzzerResultConverter<GrpcTestResult> for GrpcTestResult {
    fn into_fuzz_result(self) -> FuzzResult {
        let description = self.description.clone();
        FuzzResult {
            payload: Payload {
                payload_type: PayloadType::Grpc,
                payload: self.method,
                description,
                severity: self.severity,
                tags: vec![format!("{:?}", self.vulnerability)],
            },
            status_code: if self.success { 200 } else { 400 },
            response_time_ms: 0,
            response_length: None,
            response_body: None,
            is_waf_blocked: false,
            is_anomaly: self.success,
            is_redos_suspected: false,
            leaks_found: if self.success {
                vec![self.description]
            } else {
                vec![]
            },
            error: None,
            owasp_category: Some(crate::waf::types::OwaspCategory::A03_2021_Injection.to_string()),
            detected_severity: self.severity,
        }
    }
}
