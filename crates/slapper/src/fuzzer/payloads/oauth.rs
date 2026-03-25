#![allow(clippy::vec_init_then_push)]

use crate::fuzzer::payloads::{Payload, PayloadType, Severity};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OAuthVulnerability {
    RedirectUriValidation,
    StateParameterMissing,
    ScopeEscalation,
    GrantTypeMixing,
    PKCEBypass,
    TokenLeakage,
}

impl std::fmt::Display for OAuthVulnerability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuthVulnerability::RedirectUriValidation => write!(f, "Redirect URI Validation"),
            OAuthVulnerability::StateParameterMissing => write!(f, "State Parameter Missing"),
            OAuthVulnerability::ScopeEscalation => write!(f, "Scope Escalation"),
            OAuthVulnerability::GrantTypeMixing => write!(f, "Grant Type Mixing"),
            OAuthVulnerability::PKCEBypass => write!(f, "PKCE Bypass"),
            OAuthVulnerability::TokenLeakage => write!(f, "Token Leakage"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTestResult {
    pub vulnerability: OAuthVulnerability,
    pub success: bool,
    pub endpoint: String,
    pub proof: String,
    pub severity: Severity,
    pub description: String,
}

pub struct OAuthFuzzer {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub redirect_uri: String,
    pub issuer_url: Option<String>,
    pub client: Option<Client>,
    pub enable_redirect: bool,
    pub enable_scope: bool,
    pub enable_state: bool,
    pub enable_grant: bool,
}

impl OAuthFuzzer {
    pub fn new(client_id: String, redirect_uri: String) -> Self {
        Self {
            client_id,
            client_secret: None,
            redirect_uri,
            issuer_url: None,
            client: None,
            enable_redirect: true,
            enable_scope: true,
            enable_state: true,
            enable_grant: true,
        }
    }

    pub fn with_issuer(mut self, issuer: String) -> Self {
        self.issuer_url = Some(issuer);
        self
    }

    pub fn with_client(mut self, client: Client) -> Self {
        self.client = Some(client);
        self
    }

    pub fn with_client_secret(mut self, secret: String) -> Self {
        self.client_secret = Some(secret);
        self
    }

    pub fn with_redirect_test(mut self, enabled: bool) -> Self {
        self.enable_redirect = enabled;
        self
    }

    pub fn with_scope_test(mut self, enabled: bool) -> Self {
        self.enable_scope = enabled;
        self
    }

    pub fn with_state_test(mut self, enabled: bool) -> Self {
        self.enable_state = enabled;
        self
    }

    pub fn with_grant_test(mut self, enabled: bool) -> Self {
        self.enable_grant = enabled;
        self
    }
}

impl OAuthFuzzer {
    pub fn discover_endpoints(&self, issuer: &str) -> Vec<OAuthEndpoint> {
        let mut endpoints = Vec::new();

        endpoints.push(OAuthEndpoint {
            url: format!("{}/.well-known/openid-configuration", issuer),
            kind: EndpointKind::OidcDiscovery,
        });

        endpoints.push(OAuthEndpoint {
            url: format!("{}/.well-known/oauth-authorization-server", issuer),
            kind: EndpointKind::OAuthDiscovery,
        });

        endpoints.push(OAuthEndpoint {
            url: format!("{}/authorize", issuer),
            kind: EndpointKind::Authorize,
        });

        endpoints.push(OAuthEndpoint {
            url: format!("{}/token", issuer),
            kind: EndpointKind::Token,
        });

        endpoints.push(OAuthEndpoint {
            url: format!("{}/userinfo", issuer),
            kind: EndpointKind::UserInfo,
        });

        endpoints.push(OAuthEndpoint {
            url: format!("{}/jwks", issuer),
            kind: EndpointKind::Jwks,
        });

        endpoints.push(OAuthEndpoint {
            url: format!("{}/revoke", issuer),
            kind: EndpointKind::Revoke,
        });

        endpoints
    }

    pub fn test_redirect_uri(&self, auth_endpoint: &str) -> Vec<OAuthTestResult> {
        let mut results = Vec::new();

        let malicious_uris = vec![
            "https://evil.com/callback",
            "https://evil.com",
            "http://localhost:8080/callback",
            "https://example.com.evil.com",
            "https://example.com//evil.com",
            "https://example.com/\\@evil.com",
            "https://example.com%2f%2fevil.com",
            "https://example.com%00evil.com",
            "https://example.com/;evil.com",
            "https://example.com\\@evil.com",
        ];

        for uri in malicious_uris {
            let test_uri = format!(
                "{}?response_type=code&client_id={}&redirect_uri={}&scope=openid",
                auth_endpoint,
                urlencoding::encode(&self.client_id),
                urlencoding::encode(uri)
            );

            results.push(OAuthTestResult {
                vulnerability: OAuthVulnerability::RedirectUriValidation,
                success: false,
                endpoint: test_uri,
                proof: format!("Testing redirect_uri: {}", uri),
                severity: Severity::Critical,
                description: "Potential open redirect via redirect_uri parameter".to_string(),
            });
        }

        self.test_redirect_variations(auth_endpoint, results)
    }

    fn test_redirect_variations(
        &self,
        auth_endpoint: &str,
        mut results: Vec<OAuthTestResult>,
    ) -> Vec<OAuthTestResult> {
        let variations = vec![
            (
                urlencoding::encode("https://evil.com").to_string(),
                "URL encoded",
            ),
            (
                {
                    let first = urlencoding::encode("https://evil.com");
                    urlencoding::encode(&first).to_string()
                },
                "Double URL encoded",
            ),
            (
                "https://example.com/path/../https://evil.com".to_string(),
                "Path traversal",
            ),
            (
                "https://example.com/path/..".to_string(),
                "Path traversal to root",
            ),
        ];

        for (uri, desc) in variations {
            results.push(OAuthTestResult {
                vulnerability: OAuthVulnerability::RedirectUriValidation,
                success: false,
                endpoint: format!("{}?redirect_uri={}", auth_endpoint, uri),
                proof: desc.to_string(),
                severity: Severity::High,
                description: format!("Testing redirect variation: {}", desc),
            });
        }

        results
    }

    pub fn test_state_parameter(&self, auth_endpoint: &str) -> Vec<OAuthTestResult> {
        let mut results = Vec::new();

        let tests = vec![
            ("", "Missing state parameter"),
            ("short", "Very short state"),
            ("   ", "Whitespace state"),
            ("state'\"<script>alert(1)</script>", "XSS in state"),
        ];

        for (state, desc) in tests {
            let url = if state.is_empty() {
                format!(
                    "{}?response_type=code&client_id={}&redirect_uri={}",
                    auth_endpoint,
                    urlencoding::encode(&self.client_id),
                    urlencoding::encode(&self.redirect_uri)
                )
            } else {
                format!(
                    "{}?response_type=code&client_id={}&redirect_uri={}&state={}",
                    auth_endpoint,
                    urlencoding::encode(&self.client_id),
                    urlencoding::encode(&self.redirect_uri),
                    urlencoding::encode(state)
                )
            };

            results.push(OAuthTestResult {
                vulnerability: OAuthVulnerability::StateParameterMissing,
                success: false,
                endpoint: url,
                proof: desc.to_string(),
                severity: Severity::Medium,
                description: format!("State parameter test: {}", desc),
            });
        }

        results
    }

    pub fn test_scope_escalation(&self, auth_endpoint: &str) -> Vec<OAuthTestResult> {
        let mut results = Vec::new();

        let dangerous_scopes = vec![
            ("openid profile email", "Standard scopes"),
            ("openid profile email offline_access", "Offline access"),
            (
                "openid profile email https://www.googleapis.com/auth/drive",
                "Google Drive",
            ),
            (
                "openid https://www.googleapis.com/auth/gmail.readonly",
                "Gmail read",
            ),
            (
                "openid profile email https://api.asana.com/0.8/workspaces",
                "Asana workspace",
            ),
            ("*", "Wildcard scope"),
            ("admin", "Admin scope"),
            ("read write delete", "Full permissions"),
        ];

        for (scope, desc) in dangerous_scopes {
            let url = format!(
                "{}?response_type=code&client_id={}&redirect_uri={}&scope={}",
                auth_endpoint,
                urlencoding::encode(&self.client_id),
                urlencoding::encode(&self.redirect_uri),
                urlencoding::encode(scope)
            );

            results.push(OAuthTestResult {
                vulnerability: OAuthVulnerability::ScopeEscalation,
                success: false,
                endpoint: url,
                proof: desc.to_string(),
                severity: Severity::High,
                description: format!("Testing scope: {}", desc),
            });
        }

        results
    }

    pub fn test_grant_type_mixing(&self, token_endpoint: &str) -> Vec<OAuthTestResult> {
        let mut results = Vec::new();

        let grant_tests = vec![
            ("authorization_code", "code"),
            ("authorization_code", "token"),
            ("implicit", ""),
            ("password", "password"),
            ("client_credentials", "client_credentials"),
        ];

        for (grant_type, response_type) in grant_tests {
            let mut params = vec![("grant_type", grant_type), ("client_id", &self.client_id)];

            if let Some(ref secret) = self.client_secret {
                params.push(("client_secret", secret));
            }

            if response_type == "code" || response_type == "token" {
                params.push(("code", "test_code"));
            }

            if grant_type == "password" {
                params.push(("username", "testuser"));
                params.push(("password", "testpass"));
            }

            let url = format!(
                "{}?{}#{}",
                token_endpoint,
                params
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                    .collect::<Vec<_>>()
                    .join("&"),
                if response_type == "token" {
                    "access_token=token"
                } else {
                    ""
                }
            );

            results.push(OAuthTestResult {
                vulnerability: OAuthVulnerability::GrantTypeMixing,
                success: false,
                endpoint: url,
                proof: format!("Testing grant_type: {}", grant_type),
                severity: Severity::Medium,
                description: format!("Grant type mixing test: {}", grant_type),
            });
        }

        results
    }

    pub fn test_pkce(&self, auth_endpoint: &str) -> Vec<OAuthTestResult> {
        let mut results = Vec::new();

        let tests = vec![
            ("plain", "plain (insecure)"),
            ("", "Missing PKCE"),
            ("S256", "S256 (secure)"),
        ];

        for (challenge, desc) in tests {
            let url = if challenge.is_empty() {
                format!(
                    "{}?response_type=code&client_id={}&redirect_uri={}",
                    auth_endpoint,
                    urlencoding::encode(&self.client_id),
                    urlencoding::encode(&self.redirect_uri)
                )
            } else {
                format!(
                    "{}?response_type=code&client_id={}&redirect_uri={}&code_challenge={}&code_challenge_method={}",
                    auth_endpoint,
                    urlencoding::encode(&self.client_id),
                    urlencoding::encode(&self.redirect_uri),
                    urlencoding::encode("test_challenge"),
                    challenge
                )
            };

            results.push(OAuthTestResult {
                vulnerability: OAuthVulnerability::PKCEBypass,
                success: false,
                endpoint: url,
                proof: desc.to_string(),
                severity: Severity::High,
                description: format!("PKCE test: {}", desc),
            });
        }

        results
    }

    pub async fn test_issuer(&mut self) -> Vec<OAuthTestResult> {
        let mut results = Vec::new();

        let client = match &self.client {
            Some(c) => c,
            None => return results,
        };

        let issuer = match &self.issuer_url {
            Some(url) => url,
            None => return results,
        };

        let discovery_url = format!("{}/.well-known/openid-configuration", issuer);
        let response = client.get(&discovery_url).send().await;

        match response {
            Ok(resp) => {
                if resp.status().as_u16() == 200 {
                    results.push(OAuthTestResult {
                        vulnerability: OAuthVulnerability::RedirectUriValidation,
                        success: true,
                        endpoint: discovery_url.clone(),
                        proof: "OpenID Configuration discovered".to_string(),
                        severity: Severity::Info,
                        description: format!(
                            "OpenID Configuration endpoint accessible: {}",
                            discovery_url
                        ),
                    });

                    if let Ok(config) = resp.json::<serde_json::Value>().await {
                        if let Some(auth_endpoint) = config
                            .get("authorization_endpoint")
                            .and_then(|v| v.as_str())
                        {
                            let auth_results =
                                self.test_authorization_endpoint(auth_endpoint).await;
                            results.extend(auth_results);
                        }

                        if let Some(token_endpoint) =
                            config.get("token_endpoint").and_then(|v| v.as_str())
                        {
                            let token_results = self.test_token_endpoint(token_endpoint).await;
                            results.extend(token_results);
                        }
                    }
                }
            }
            Err(e) => {
                results.push(OAuthTestResult {
                    vulnerability: OAuthVulnerability::RedirectUriValidation,
                    success: false,
                    endpoint: discovery_url,
                    proof: e.to_string(),
                    severity: Severity::Info,
                    description: "Failed to fetch OpenID Configuration".to_string(),
                });
            }
        }

        results
    }

    async fn test_authorization_endpoint(&self, auth_endpoint: &str) -> Vec<OAuthTestResult> {
        let mut results = Vec::new();

        let client = match &self.client {
            Some(c) => c,
            None => return results,
        };

        let test_redirects = vec![
            ("https://evil.com", "Basic open redirect"),
            ("https://example.com.evil.com", "Subdomain takeover"),
            ("http://localhost", "Localhost"),
            ("http://127.0.0.1", "Local IP"),
        ];

        for (redirect, desc) in test_redirects {
            let url = format!(
                "{}?client_id={}&redirect_uri={}&response_type=code",
                auth_endpoint, self.client_id, redirect
            );

            let response = client.get(&url).send().await;

            match response {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    if status == 302 || status == 303 {
                        let location = resp
                            .headers()
                            .get("location")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("");

                        if location.contains(redirect) || location.starts_with(redirect) {
                            results.push(OAuthTestResult {
                                vulnerability: OAuthVulnerability::RedirectUriValidation,
                                success: true,
                                endpoint: url,
                                proof: format!("Redirects to: {}", location),
                                severity: Severity::Critical,
                                description: format!("Open redirect vulnerability: {}", desc),
                            });
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        results
    }

    async fn test_token_endpoint(&self, token_endpoint: &str) -> Vec<OAuthTestResult> {
        let mut results = Vec::new();

        let client = match &self.client {
            Some(c) => c,
            None => return results,
        };

        let grant_types = vec![
            ("client_credentials", "Client Credentials grant"),
            ("password", "Resource Owner Password Credentials grant"),
        ];

        for (grant_type, desc) in grant_types {
            let params = [
                ("client_id", self.client_id.as_str()),
                ("grant_type", grant_type),
            ];

            let response = client.post(token_endpoint).form(&params).send().await;

            match response {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    if status == 200 {
                        results.push(OAuthTestResult {
                            vulnerability: OAuthVulnerability::GrantTypeMixing,
                            success: true,
                            endpoint: token_endpoint.to_string(),
                            proof: format!("Grant type {} accepted", grant_type),
                            severity: Severity::High,
                            description: format!(
                                "Potentially dangerous grant type allowed: {}",
                                desc
                            ),
                        });
                    } else if status == 400 {
                        let body = resp.text().await.unwrap_or_default();
                        if body.contains("unsupported_grant_type") {
                            results.push(OAuthTestResult {
                                vulnerability: OAuthVulnerability::GrantTypeMixing,
                                success: false,
                                endpoint: token_endpoint.to_string(),
                                proof: body,
                                severity: Severity::Info,
                                description: format!("Grant type {} not supported", grant_type),
                            });
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        results
    }
}

#[derive(Debug, Clone)]
pub struct OAuthEndpoint {
    pub url: String,
    pub kind: EndpointKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndpointKind {
    OidcDiscovery,
    OAuthDiscovery,
    Authorize,
    Token,
    UserInfo,
    Jwks,
    Revoke,
}

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "redirect_uri=https://evil.com/callback".to_string(),
        description: "OAuth redirect_uri manipulation".to_string(),
        severity: Severity::Critical,
        tags: vec!["oauth".to_string(), "redirect".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "scope=*".to_string(),
        description: "Wildcard scope request".to_string(),
        severity: Severity::High,
        tags: vec!["oauth".to_string(), "scope".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "grant_type=authorization_code&code=test".to_string(),
        description: "Authorization code replay".to_string(),
        severity: Severity::High,
        tags: vec!["oauth".to_string(), "replay".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::OAuth,
        payload: "response_type=token".to_string(),
        description: "Implicit flow (insecure)".to_string(),
        severity: Severity::Medium,
        tags: vec!["oauth".to_string(), "implicit".to_string()],
    });

    payloads
}
