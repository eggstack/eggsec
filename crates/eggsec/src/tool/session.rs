//! Advanced session management with authentication and CSRF handling.
//!
//! Provides comprehensive session tracking including multiple authentication
//! methods, CSRF token management, login sequences, form detection, and MFA support.
//! This implementation draws from ZAP and Burp's authenticated scanning capabilities.

use crate::types::SensitiveString;
use crate::utils::create_insecure_http_client;
use regex::Regex;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::RwLock;

static CSRF_INPUT_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<input[^>]*name="([^"]+)"[^>]*value="([^"]+)"[^>]*>"#).unwrap());

static CSRF_META_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<meta[^>]*name="csrf-token"[^>]*content="([^"]+)""#).unwrap());

static FORM_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"<form[^>]*>"#).unwrap());

static INPUT_TAG_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<input[^>]*>"#).unwrap());

static LOGGED_IN_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)logout|sign.?out|sign.?off").unwrap(),
        Regex::new(r"(?i)dashboard|profile|settings").unwrap(),
        Regex::new(r"(?i)welcome,?\s+\w+").unwrap(),
    ]
});

static LOGGED_OUT_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)sign.?in|log.?in|login").unwrap(),
        Regex::new(r"(?i)please.?login|session.?expired").unwrap(),
    ]
});

/// Authentication method types supported by Eggsec
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthMethod {
    Basic {
        username: String,
        password: SensitiveString,
    },
    Bearer {
        token: SensitiveString,
    },
    OAuth2 {
        client_id: String,
        client_secret: SensitiveString,
        token_url: String,
        scopes: Vec<String>,
    },
    APIKey {
        key: SensitiveString,
        header_name: String,
    },
    Digest {
        username: String,
        password: SensitiveString,
    },
    NTLM {
        username: String,
        password: SensitiveString,
        domain: Option<String>,
    },
    /// Form-based authentication with credentials and optional MFA
    FormBased {
        username: String,
        password: SensitiveString,
        login_url: String,
        username_field: String,
        password_field: String,
        /// Optional MFA configuration
        mfa: Option<MfaConfig>,
    },
}

impl AuthMethod {
    /// Apply authentication to a ToolRequest
    pub fn apply_to_request(
        &self,
        request: &mut crate::tool::request::ToolRequest,
    ) -> Result<(), crate::error::EggsecError> {
        match self {
            AuthMethod::Basic { username, password } => {
                let credentials = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    format!("{}:{}", username, password.expose_secret()),
                );
                request.params["auth_header"] = serde_json::json!(format!("Basic {}", credentials));
            }
            AuthMethod::Bearer { token } => {
                request.params["auth_header"] =
                    serde_json::json!(format!("Bearer {}", token.expose_secret()));
            }
            AuthMethod::APIKey { key, header_name } => {
                request.params["api_key_header"] = serde_json::json!(header_name);
                request.params["api_key_value"] = serde_json::json!(key.expose_secret());
            }
            AuthMethod::OAuth2 { .. } => {
                request.params["auth_type"] = serde_json::json!("oauth2");
            }
            AuthMethod::Digest { .. } => {
                request.params["auth_type"] = serde_json::json!("digest");
            }
            AuthMethod::NTLM { .. } => {
                request.params["auth_type"] = serde_json::json!("ntlm");
            }
            AuthMethod::FormBased { .. } => {
                request.params["auth_type"] = serde_json::json!("form_based");
            }
        }
        Ok(())
    }

    pub fn auth_type_name(&self) -> &'static str {
        match self {
            AuthMethod::Basic { .. } => "basic",
            AuthMethod::Bearer { .. } => "bearer",
            AuthMethod::OAuth2 { .. } => "oauth2",
            AuthMethod::APIKey { .. } => "api_key",
            AuthMethod::Digest { .. } => "digest",
            AuthMethod::NTLM { .. } => "ntlm",
            AuthMethod::FormBased { .. } => "form_based",
        }
    }
}

/// MFA configuration for handling multi-factor authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MfaConfig {
    /// Time-based One-Time Password (Google Authenticator, etc.)
    Totp { secret: SensitiveString },
    /// Backup codes for account recovery
    BackupCodes { codes: Vec<String> },
    /// Email-based verification code
    Email { email: String },
    /// SMS-based verification code
    Sms { phone: String },
    /// Custom MFA flow requiring multiple steps
    Custom {
        mfa_url: String,
        mfa_field: String,
        submit_url: Option<String>,
    },
}

/// CSRF token tracking with extraction metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsrfToken {
    pub token: String,
    pub url: String,
    pub header_name: String,
    pub param_name: Option<String>,
    pub acquired_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Token format detected (header, form_param, cookie)
    pub token_location: CsrfTokenLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CsrfTokenLocation {
    Header,
    FormParam,
    Cookie,
}

impl CsrfToken {
    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            return chrono::Utc::now() > expires;
        }
        false
    }
}

/// Login sequence definition - ZAP-style recording of login steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginSequence {
    pub name: String,
    pub steps: Vec<LoginStep>,
    pub csrf_required: bool,
    pub session_cookie_names: Vec<String>,
    /// URL patterns that indicate successful login
    pub logged_in_indicators: Vec<String>,
    /// URL patterns that indicate logged out
    pub logged_out_indicators: Vec<String>,
}

impl Default for LoginSequence {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            steps: Vec::new(),
            csrf_required: true,
            session_cookie_names: vec![
                "session".to_string(),
                "PHPSESSID".to_string(),
                "JSESSIONID".to_string(),
            ],
            logged_in_indicators: vec![
                "logout".to_string(),
                "signout".to_string(),
                "dashboard".to_string(),
                "welcome".to_string(),
            ],
            logged_out_indicators: vec![
                "login".to_string(),
                "signin".to_string(),
                "log in".to_string(),
            ],
        }
    }
}

/// Individual step in a login sequence
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LoginStep {
    /// Make an HTTP request
    Request {
        url: String,
        method: String,
        headers: FxHashMap<String, String>,
        body: Option<String>,
    },
    /// Extract a field from the response
    ExtractField {
        from_response_field: ResponseField,
        variable_name: String,
        /// Optional regex to extract specific part
        pattern: Option<String>,
    },
    /// Extract a cookie from the response
    ExtractCookie {
        cookie_name: String,
        variable_name: String,
    },
    /// Set a header from a variable
    SetHeader { header_name: String, value: String },
    /// Wait for a specified duration (for dynamic content)
    Wait { milliseconds: u64 },
    /// Conditional step based on response
    Conditional {
        condition: Condition,
        then_steps: Vec<LoginStep>,
        else_steps: Option<Vec<LoginStep>>,
    },
}

/// Where to extract data from in a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseField {
    /// Extract from response body (HTML/JSON)
    Body,
    /// Extract from response headers
    Header(String),
    /// Extract from set-cookie header
    SetCookie,
    /// Extract from JSON response path (e.g., "data.token")
    JsonPath(String),
    /// Extract using regex from body
    Regex(String),
}

/// Condition for conditional login steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    /// Check if response contains specific text
    Contains(String),
    /// Check if response status code matches
    StatusCode(u16),
    /// Check if response redirects
    Redirects,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResult {
    pub success: bool,
    pub session_cookies: FxHashMap<String, String>,
    pub extracted_values: FxHashMap<String, String>,
    pub error_message: Option<String>,
    pub final_url: Option<String>,
    pub response_code: Option<u16>,
}

impl Default for LoginResult {
    fn default() -> Self {
        Self {
            success: false,
            session_cookies: FxHashMap::default(),
            extracted_values: FxHashMap::default(),
            error_message: None,
            final_url: None,
            response_code: None,
        }
    }
}

/// Current session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub session_id: String,
    pub auth_method: Option<AuthMethod>,
    pub csrf_tokens: Vec<CsrfToken>,
    pub login_sequence: Option<LoginSequence>,
    pub login_result: Option<LoginResult>,
    pub cookies: FxHashMap<String, String>,
    pub headers: FxHashMap<String, String>,
    pub custom_data: FxHashMap<String, serde_json::Value>,
    /// URL used for last successful authentication
    pub authenticated_url: Option<String>,
    /// Last time session was verified as active
    pub last_verified: Option<chrono::DateTime<chrono::Utc>>,
    /// Current auth token (e.g., OAuth access token)
    pub auth_token: Option<SensitiveString>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            auth_method: None,
            csrf_tokens: Vec::new(),
            login_sequence: None,
            login_result: None,
            cookies: FxHashMap::default(),
            headers: FxHashMap::default(),
            custom_data: FxHashMap::default(),
            authenticated_url: None,
            last_verified: None,
            auth_token: None,
        }
    }
}

impl SessionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_auth_method(mut self, auth_method: AuthMethod) -> Self {
        self.auth_method = Some(auth_method);
        self
    }

    pub fn add_csrf_token(&mut self, token: CsrfToken) {
        self.csrf_tokens.push(token);
    }

    pub fn get_csrf_token(&self, url: &str) -> Option<&CsrfToken> {
        self.csrf_tokens
            .iter()
            .find(|t| t.url == url && !t.is_expired())
    }

    pub fn add_cookie(&mut self, name: &str, value: &str) {
        self.cookies.insert(name.to_string(), value.to_string());
    }

    pub fn get_cookie(&self, name: &str) -> Option<&str> {
        self.cookies.get(name).map(|s| s.as_str())
    }

    pub fn add_header(&mut self, name: &str, value: &str) {
        self.headers.insert(name.to_string(), value.to_string());
    }

    pub fn get_header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(|s| s.as_str())
    }

    pub fn set_custom_data(&mut self, key: &str, value: serde_json::Value) {
        self.custom_data.insert(key.to_string(), value);
    }

    pub fn get_custom_data(&self, key: &str) -> Option<&serde_json::Value> {
        self.custom_data.get(key)
    }

    pub fn clear_auth(&mut self) {
        self.auth_method = None;
        self.csrf_tokens.clear();
        self.login_result = None;
        self.auth_token = None;
    }

    pub fn is_authenticated(&self) -> bool {
        self.auth_method.is_some()
            || !self.cookies.is_empty()
            || self
                .login_result
                .as_ref()
                .map(|r| r.success)
                .unwrap_or(false)
    }

    pub fn apply_to_request(
        &self,
        request: &mut crate::tool::request::ToolRequest,
    ) -> Result<(), crate::error::EggsecError> {
        if let Some(ref auth) = self.auth_method {
            auth.apply_to_request(request)?;
        }

        // Add session cookies
        for (name, value) in &self.cookies {
            let cookie_str = format!("{}={}", name, value);
            if let Some(existing) = request.params.get("cookie_header").and_then(|v| v.as_str()) {
                request.params["cookie_header"] =
                    serde_json::json!(format!("{}; {}", existing, cookie_str));
            } else {
                request.params["cookie_header"] = serde_json::json!(cookie_str);
            }
        }

        // Add custom headers
        for (name, value) in &self.headers {
            let header_key = format!("header_{}", name);
            request.params[header_key] = serde_json::json!(value);
        }

        // Add CSRF token if available for this URL
        if let Some(csrf) = self.get_csrf_token(&request.target.value) {
            if let Some(param_name) = &csrf.param_name {
                request.params["csrf_param"] = serde_json::json!(param_name);
                request.params["csrf_value"] = serde_json::json!(&csrf.token);
            }
            request.params["csrf_header"] = serde_json::json!(&csrf.header_name);
        }

        Ok(())
    }
}

/// Login sequence executor - executes recorded login steps
pub struct LoginExecutor {
    client: reqwest::Client,
}

impl LoginExecutor {
    pub fn new(timeout_secs: u64) -> crate::error::Result<Self> {
        let client = create_insecure_http_client(timeout_secs)?;
        Ok(Self { client })
    }

    /// Execute a login sequence and return the result
    pub async fn execute_sequence(
        &self,
        sequence: &LoginSequence,
        _auth_method: &AuthMethod,
    ) -> crate::error::Result<LoginResult> {
        let mut variables: FxHashMap<String, String> = FxHashMap::default();
        let session_cookies: FxHashMap<String, String> = FxHashMap::default();
        let final_url: Option<String> = None;
        let mut response_code: Option<u16> = None;
        let mut response_headers: FxHashMap<String, String> = FxHashMap::default();

        for step in &sequence.steps {
            match step {
                LoginStep::Request {
                    url,
                    method,
                    headers,
                    body,
                } => {
                    // Substitute variables in URL
                    let url = self.substitute_variables(url, &variables);
                    // Substitute variables in body
                    let body = body
                        .as_ref()
                        .map(|b| self.substitute_variables(b, &variables));

                    let mut request = self.client.request(
                        match method.to_uppercase().as_str() {
                            "GET" => reqwest::Method::GET,
                            "POST" => reqwest::Method::POST,
                            "PUT" => reqwest::Method::PUT,
                            "DELETE" => reqwest::Method::DELETE,
                            _ => reqwest::Method::GET,
                        },
                        &url,
                    );

                    // Add custom headers
                    for (key, value) in headers {
                        let key = self.substitute_variables(key, &variables);
                        let value = self.substitute_variables(value, &variables);
                        request = request.header(&key, &value);
                    }

                    // Add body if present
                    if let Some(body) = body {
                        request = request
                            .header(
                                reqwest::header::CONTENT_TYPE,
                                "application/x-www-form-urlencoded",
                            )
                            .body(body);
                    }

                    let response =
                        tokio::time::timeout(std::time::Duration::from_secs(30), request.send())
                            .await
                            .map_err(|e| {
                                crate::error::EggsecError::Network(format!(
                                    "Login request timed out: {}",
                                    e
                                ))
                            })?
                            .map_err(|e| {
                                crate::error::EggsecError::Network(format!(
                                    "Login request failed: {}",
                                    e
                                ))
                            })?;

                    response_code = Some(response.status().as_u16());
                    let _final_url = response.url().to_string();

                    // Store response for extraction steps
                    let headers_map: FxHashMap<String, String> = response
                        .headers()
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                        .collect();
                    response_headers = headers_map.clone();
                    let body = response.text().await.unwrap_or_else(|e| {
                        tracing::warn!("Failed to read login response body: {}", e);
                        String::new()
                    });

                    // Store response body for extraction
                    variables.insert("_response_body".to_string(), body.clone());
                    variables.insert(
                        "_response_status".to_string(),
                        response_code.unwrap_or(0).to_string(),
                    );
                }
                LoginStep::ExtractField {
                    from_response_field,
                    variable_name,
                    pattern,
                } => {
                    let value = match from_response_field {
                        ResponseField::Body => {
                            variables.get("_response_body").cloned().unwrap_or_default()
                        }
                        ResponseField::Header(name) => {
                            response_headers.get(name).cloned().unwrap_or_default()
                        }
                        ResponseField::SetCookie => {
                            // Extract from cookies
                            session_cookies
                                .get(variable_name)
                                .cloned()
                                .unwrap_or_default()
                        }
                        ResponseField::JsonPath(path) => {
                            // Simple JSON extraction (expand for full JSONPath support)
                            if let Some(body) = variables.get("_response_body") {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                                    self.extract_json_path(&json, path).unwrap_or_default()
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            }
                        }
                        ResponseField::Regex(pattern) => {
                            if let Some(body) = variables.get("_response_body") {
                                if let Ok(re) = Regex::new(pattern) {
                                    re.find(body)
                                        .map(|m| m.as_str().to_string())
                                        .unwrap_or_default()
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            }
                        }
                    };

                    // Apply optional pattern
                    let value = if let Some(pat) = pattern {
                        if let Ok(re) = Regex::new(pat) {
                            re.find(&value)
                                .map(|m| m.as_str().to_string())
                                .unwrap_or(value)
                        } else {
                            value
                        }
                    } else {
                        value
                    };

                    variables.insert(variable_name.clone(), value);
                }
                LoginStep::ExtractCookie {
                    cookie_name,
                    variable_name,
                } => {
                    if let Some(value) = session_cookies.get(cookie_name) {
                        variables.insert(variable_name.clone(), value.clone());
                    }
                }
                LoginStep::SetHeader { header_name, value } => {
                    let value = self.substitute_variables(value, &variables);
                    variables.insert(format!("header_{}", header_name), value);
                }
                LoginStep::Wait { milliseconds } => {
                    tokio::time::sleep(Duration::from_millis(*milliseconds)).await;
                }
                LoginStep::Conditional {
                    condition,
                    then_steps,
                    else_steps,
                } => {
                    let matches = self.evaluate_condition(condition, &variables);
                    let steps = if matches {
                        then_steps
                    } else {
                        else_steps.as_ref().unwrap_or(then_steps)
                    };
                    // Recursively execute nested steps
                    // (simplified - full implementation would need recursive execution)
                    for step in steps {
                        // Placeholder for future recursive step execution
                        let _ = step;
                    }
                }
            }
        }

        // Check if login was successful based on indicators
        let success = self.check_login_success(&sequence.logged_in_indicators, &variables);

        Ok(LoginResult {
            success,
            session_cookies,
            extracted_values: variables,
            error_message: None,
            final_url,
            response_code,
        })
    }

    fn substitute_variables(
        &self,
        template: &str,
        variables: &FxHashMap<String, String>,
    ) -> String {
        let mut result = template.to_string();
        for (key, value) in variables {
            result = result.replace(&format!("${{{}}}", key), value);
        }
        result
    }

    fn extract_json_path(&self, json: &serde_json::Value, path: &str) -> Option<String> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = json.clone();
        for part in parts {
            current = current.get(part)?.clone();
        }
        Some(current.as_str().unwrap_or("").to_string())
    }

    fn evaluate_condition(
        &self,
        condition: &Condition,
        variables: &FxHashMap<String, String>,
    ) -> bool {
        match condition {
            Condition::Contains(text) => variables
                .get("_response_body")
                .map(|b| b.contains(text))
                .unwrap_or(false),
            Condition::StatusCode(code) => variables
                .get("_response_status")
                .and_then(|s| s.parse::<u16>().ok())
                .map(|c| c == *code)
                .unwrap_or(false),
            Condition::Redirects => variables
                .get("_response_status")
                .and_then(|s| s.parse::<i32>().ok())
                .map(|c: i32| c >= 300 && c < 400)
                .unwrap_or(false),
        }
    }

    fn check_login_success(
        &self,
        indicators: &[String],
        variables: &FxHashMap<String, String>,
    ) -> bool {
        if let Some(body) = variables.get("_response_body") {
            for indicator in indicators {
                if body.to_lowercase().contains(&indicator.to_lowercase()) {
                    return true;
                }
            }
        }
        false
    }
}

/// CSRF token extractor - extracts tokens from HTTP responses
pub struct CsrfExtractor {
    /// Known CSRF token attribute names (from ZAP)
    token_names: Vec<String>,
}

impl CsrfExtractor {
    pub fn new() -> Self {
        Self {
            token_names: vec![
                "_token".to_string(),
                "csrf_token".to_string(),
                "csrf".to_string(),
                "authenticity_token".to_string(),
                "xsrf".to_string(),
                "xsrf_token".to_string(),
                "token".to_string(),
                "form_token".to_string(),
                "request_token".to_string(),
                "__RequestVerificationToken".to_string(),
                "OWASP_CSRFTOKEN".to_string(),
                "anticsrf".to_string(),
            ],
        }
    }

    /// Extract CSRF tokens from an HTML response
    pub fn extract_from_html(&self, html: &str, url: &str) -> Vec<CsrfToken> {
        let mut tokens = Vec::new();

        // Extract from hidden input fields
        for cap in CSRF_INPUT_PATTERN.captures_iter(html) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let value = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            if self.is_csrf_token_name(name) && !value.is_empty() {
                tokens.push(CsrfToken {
                    token: value.to_string(),
                    url: url.to_string(),
                    header_name: "X-CSRF-Token".to_string(),
                    param_name: Some(name.to_string()),
                    acquired_at: chrono::Utc::now(),
                    expires_at: None,
                    token_location: CsrfTokenLocation::FormParam,
                });
            }
        }

        // Also check meta tags
        for cap in CSRF_META_PATTERN.captures_iter(html) {
            if let Some(value) = cap.get(1) {
                tokens.push(CsrfToken {
                    token: value.as_str().to_string(),
                    url: url.to_string(),
                    header_name: "X-CSRF-Token".to_string(),
                    param_name: None,
                    acquired_at: chrono::Utc::now(),
                    expires_at: None,
                    token_location: CsrfTokenLocation::Header,
                });
            }
        }

        tokens
    }

    /// Extract CSRF token from JSON response
    pub fn extract_from_json(&self, json: &serde_json::Value, url: &str) -> Vec<CsrfToken> {
        let mut tokens = Vec::new();

        // Check common JSON token paths
        let token_paths = vec![
            "token",
            "csrf_token",
            "csrf",
            "authenticity_token",
            "data.token",
            "response.token",
        ];

        for path in token_paths {
            if let Some(value) = json.get(path) {
                if let Some(token) = value.as_str() {
                    if !token.is_empty() {
                        tokens.push(CsrfToken {
                            token: token.to_string(),
                            url: url.to_string(),
                            header_name: "X-CSRF-Token".to_string(),
                            param_name: Some(path.to_string()),
                            acquired_at: chrono::Utc::now(),
                            expires_at: None,
                            token_location: CsrfTokenLocation::FormParam,
                        });
                    }
                }
            }
        }

        tokens
    }

    fn is_csrf_token_name(&self, name: &str) -> bool {
        let name_lower = name.to_lowercase();
        self.token_names
            .iter()
            .any(|n| name_lower.contains(&n.to_lowercase()))
    }
}

impl Default for CsrfExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Login form detector - parses HTML to find login forms (ZAP-style)
pub struct FormDetector {
    /// Common username field names
    username_fields: Vec<String>,
    /// Common password field names
    password_fields: Vec<String>,
}

impl FormDetector {
    pub fn new() -> Self {
        Self {
            username_fields: vec![
                "username".to_string(),
                "email".to_string(),
                "user".to_string(),
                "login".to_string(),
                "user_name".to_string(),
                "userid".to_string(),
                "account".to_string(),
                "uid".to_string(),
            ],
            password_fields: vec![
                "password".to_string(),
                "pass".to_string(),
                "pwd".to_string(),
                "passwd".to_string(),
            ],
        }
    }

    /// Detect login forms in HTML and return form details
    pub fn detect_login_form(&self, html: &str, base_url: &str) -> Option<LoginForm> {
        let forms: Vec<_> = FORM_PATTERN.find_iter(html).collect();

        for form_match in forms {
            let form_start = form_match.start();
            let form_end = form_match.end();

            // Find the closing </form>
            let remaining = &html[form_end..];
            let form_close = remaining.find("</form>").unwrap_or(remaining.len());
            let full_form = &html[form_start..form_end + form_close + 7];

            // Check if form has both username and password
            let has_username = self.has_field_named(full_form, &self.username_fields);
            let has_password = self.has_field_named(full_form, &self.password_fields);

            if has_username && has_password {
                // Extract form action
                let action = self
                    .extract_attribute(full_form, "action")
                    .map(|a| {
                        if a.starts_with("http") {
                            a
                        } else if a.starts_with("/") {
                            format!("{}{}", base_url.trim_end_matches('/'), a)
                        } else {
                            format!("{}/{}", base_url.trim_end_matches('/'), a)
                        }
                    })
                    .unwrap_or_else(|| base_url.to_string());

                // Extract method
                let method = self
                    .extract_attribute(full_form, "method")
                    .map(|m| m.to_uppercase())
                    .unwrap_or_else(|| "POST".to_string());

                // Extract field names
                let username_field = self
                    .find_field_name(full_form, &self.username_fields)
                    .unwrap_or_else(|| "username".to_string());
                let password_field = self
                    .find_field_name(full_form, &self.password_fields)
                    .unwrap_or_else(|| "password".to_string());

                return Some(LoginForm {
                    action,
                    method,
                    username_field,
                    password_field,
                    other_fields: self.extract_other_fields(full_form),
                });
            }
        }

        None
    }

    fn has_field_named(&self, html: &str, names: &[String]) -> bool {
        self.find_field_name(html, names).is_some()
    }

    fn find_field_name(&self, html: &str, names: &[String]) -> Option<String> {
        for name in names {
            let pattern = format!(r#"name=["']({}|{}_?)["']"#, name, name);
            let re = Regex::new(&pattern).ok()?;
            if let Some(cap) = re.captures(html) {
                if let Some(name_match) = cap.get(1) {
                    return Some(name_match.as_str().to_string());
                }
            }
        }
        None
    }

    fn extract_attribute(&self, html: &str, attr: &str) -> Option<String> {
        let pattern = format!(r#"{}=["']([^"']+)["']"#, attr);
        let re = Regex::new(&pattern).ok()?;
        re.captures(html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
    }

    fn extract_other_fields(&self, html: &str) -> Vec<FormField> {
        let mut fields = Vec::new();
        for cap in INPUT_TAG_PATTERN.find_iter(html) {
            let input = cap.as_str();
            if let (Some(name), Some(type_)) = (
                self.extract_attribute(input, "name"),
                self.extract_attribute(input, "type"),
            ) {
                if name != "username" && name != "password" {
                    fields.push(FormField {
                        name,
                        field_type: type_,
                        value: self.extract_attribute(input, "value"),
                    });
                }
            }
        }

        fields
    }
}

impl Default for FormDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Detected login form information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginForm {
    pub action: String,
    pub method: String,
    pub username_field: String,
    pub password_field: String,
    pub other_fields: Vec<FormField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    pub name: String,
    pub field_type: String,
    pub value: Option<String>,
}

/// Session verifier - checks if session is still active
pub struct SessionVerifier {
    client: reqwest::Client,
    logged_in_patterns: Vec<Regex>,
    logged_out_patterns: Vec<Regex>,
}

impl SessionVerifier {
    pub fn new(timeout_secs: u64) -> crate::error::Result<Self> {
        let client = create_insecure_http_client(timeout_secs)?;

        Ok(Self {
            client,
            logged_in_patterns: LOGGED_IN_PATTERNS.clone(),
            logged_out_patterns: LOGGED_OUT_PATTERNS.clone(),
        })
    }

    /// Verify if session is still authenticated
    pub async fn verify(
        &self,
        verification_url: &str,
        cookies: &FxHashMap<String, String>,
    ) -> crate::error::Result<SessionVerification> {
        // Build cookie header
        let cookie_str: String = cookies
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ");

        let response = self
            .client
            .get(verification_url)
            .header(reqwest::header::COOKIE, &cookie_str)
            .send()
            .await
            .map_err(|e| {
                crate::error::EggsecError::Network(format!("Verification request failed: {}", e))
            })?;

        let status = response.status();
        let body = response.text().await.unwrap_or_else(|e| {
            tracing::warn!("Failed to read verification response body: {}", e);
            String::new()
        });

        // Check for logged in indicators
        let logged_in = self.logged_in_patterns.iter().any(|p| p.is_match(&body));

        // Check for logged out indicators
        let logged_out = self.logged_out_patterns.iter().any(|p| p.is_match(&body));

        let session_status = if logged_in && !logged_out {
            SessionStatus::Authenticated
        } else if logged_out && !logged_in {
            SessionStatus::Expired
        } else {
            // Ambiguous - check status code
            if status.is_success() {
                SessionStatus::Authenticated
            } else {
                SessionStatus::Unknown
            }
        };

        Ok(SessionVerification {
            status: session_status,
            logged_in_indicators_found: logged_in,
            logged_out_indicators_found: logged_out,
            response_status: status.as_u16(),
            timestamp: chrono::Utc::now(),
        })
    }

    /// Update verification patterns
    pub fn with_logged_in_patterns(mut self, patterns: Vec<String>) -> Self {
        self.logged_in_patterns = patterns.iter().filter_map(|p| match Regex::new(p) {
            Ok(r) => Some(r),
            Err(e) => {
                tracing::warn!(target: "tool", "Invalid logged-in regex pattern '{}': {}", p, e);
                None
            }
        }).collect();
        self
    }

    pub fn with_logged_out_patterns(mut self, patterns: Vec<String>) -> Self {
        self.logged_out_patterns = patterns.iter().filter_map(|p| match Regex::new(p) {
            Ok(r) => Some(r),
            Err(e) => {
                tracing::warn!(target: "tool", "Invalid logged-out regex pattern '{}': {}", p, e);
                None
            }
        }).collect();
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Authenticated,
    Expired,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionVerification {
    pub status: SessionStatus,
    pub logged_in_indicators_found: bool,
    pub logged_out_indicators_found: bool,
    pub response_status: u16,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Session manager with re-authentication support
pub struct AuthenticatedSessionManager {
    sessions: Arc<RwLock<FxHashMap<String, SessionState>>>,
    default_ttl_seconds: i64,
    verification_url: Option<String>,
}

impl AuthenticatedSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(FxHashMap::default())),
            default_ttl_seconds: 3600,
            verification_url: None,
        }
    }

    pub fn with_verification_url(mut self, url: Option<String>) -> Self {
        self.verification_url = url;
        self
    }

    pub fn with_ttl(mut self, ttl_seconds: i64) -> Self {
        self.default_ttl_seconds = ttl_seconds;
        self
    }

    pub async fn create_session(&self) -> String {
        let state = SessionState::default();
        let id = state.session_id.clone();
        self.sessions.write().await.insert(id.clone(), state);
        id
    }

    pub async fn get_session(&self, session_id: &str) -> Option<SessionState> {
        self.sessions.read().await.get(session_id).cloned()
    }

    pub async fn update_session(&self, session: &SessionState) {
        self.sessions
            .write()
            .await
            .insert(session.session_id.clone(), session.clone());
    }

    pub async fn delete_session(&self, session_id: &str) {
        self.sessions.write().await.remove(session_id);
    }

    pub async fn list_sessions(&self) -> Vec<SessionState> {
        self.sessions.read().await.values().cloned().collect()
    }

    /// Execute login sequence and store result
    pub async fn login(
        &self,
        session_id: &str,
        sequence: &LoginSequence,
        auth_method: &AuthMethod,
    ) -> crate::error::Result<LoginResult> {
        let executor = LoginExecutor::new(self.default_ttl_seconds as u64)?;
        let result = executor.execute_sequence(sequence, auth_method).await?;

        // Update session with login result
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            // Store cookies
            for (name, value) in &result.session_cookies {
                session.add_cookie(name, value);
            }
            // Store extracted values
            for (key, value) in &result.extracted_values {
                session.set_custom_data(key, serde_json::json!(value));
            }
            session.login_result = Some(result.clone());
            session.login_sequence = Some(sequence.clone());
        }

        Ok(result)
    }

    /// Verify session is still valid and attempt re-authentication if expired
    pub async fn verify_and_refresh(
        &self,
        session_id: &str,
    ) -> crate::error::Result<SessionVerification> {
        let session = self
            .sessions
            .read()
            .await
            .get(session_id)
            .cloned()
            .ok_or_else(|| crate::error::EggsecError::Config("Session not found".to_string()))?;

        // If no verification URL set, assume session is valid
        let verification_url = match &self.verification_url {
            Some(url) => url.clone(),
            None => {
                return Ok(SessionVerification {
                    status: SessionStatus::Authenticated,
                    logged_in_indicators_found: false,
                    logged_out_indicators_found: false,
                    response_status: 0,
                    timestamp: chrono::Utc::now(),
                })
            }
        };

        let verifier = SessionVerifier::new(self.default_ttl_seconds as u64)?;
        let verification = verifier.verify(&verification_url, &session.cookies).await?;

        // If session expired and we have login sequence, try re-auth
        if verification.status == SessionStatus::Expired {
            if let (Some(sequence), Some(auth_method)) =
                (&session.login_sequence, &session.auth_method)
            {
                let result = self.login(session_id, sequence, auth_method).await?;
                tracing::info!(
                    "Re-authenticated session {}: success={}",
                    session_id,
                    result.success
                );
            }
        }

        Ok(verification)
    }

    /// Extract and store CSRF tokens for a URL
    pub async fn extract_csrf_tokens(&self, session_id: &str, url: &str, response_body: &str) {
        let extractor = CsrfExtractor::new();

        // Try HTML extraction first
        let tokens = if response_body.contains("<html") || response_body.contains("<form") {
            extractor.extract_from_html(response_body, url)
        } else {
            // Try JSON
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(response_body) {
                extractor.extract_from_json(&json, url)
            } else {
                Vec::new()
            }
        };

        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            for token in tokens {
                session.add_csrf_token(token);
            }
        }
    }

    /// Clear authentication for a session
    pub async fn logout(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.clear_auth();
            session.cookies.clear();
        }
    }

    /// Check if session is authenticated
    pub async fn is_authenticated(&self, session_id: &str) -> bool {
        self.sessions
            .read()
            .await
            .get(session_id)
            .map(|s| s.is_authenticated())
            .unwrap_or(false)
    }
}

impl Default for AuthenticatedSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Re-export for backward compatibility
pub type AdvancedSessionManager = AuthenticatedSessionManager;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_method_basic() {
        let auth = AuthMethod::Basic {
            username: "user".to_string(),
            password: SensitiveString::from("pass".to_string()),
        };
        assert_eq!(auth.auth_type_name(), "basic");
    }

    #[test]
    fn test_auth_method_bearer() {
        let auth = AuthMethod::Bearer {
            token: SensitiveString::from("token123".to_string()),
        };
        assert_eq!(auth.auth_type_name(), "bearer");
    }

    #[test]
    fn test_csrf_token_expiry() {
        let token = CsrfToken {
            token: "test".to_string(),
            url: "https://example.com".to_string(),
            header_name: "X-CSRF".to_string(),
            param_name: None,
            acquired_at: chrono::Utc::now(),
            expires_at: Some(chrono::Utc::now() - chrono::Duration::hours(1)),
            token_location: CsrfTokenLocation::Header,
        };
        assert!(token.is_expired());
    }

    #[test]
    fn test_csrf_token_not_expired() {
        let token = CsrfToken {
            token: "test".to_string(),
            url: "https://example.com".to_string(),
            header_name: "X-CSRF".to_string(),
            param_name: None,
            acquired_at: chrono::Utc::now(),
            expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
            token_location: CsrfTokenLocation::FormParam,
        };
        assert!(!token.is_expired());
    }

    #[test]
    fn test_session_state_new() {
        let state = SessionState::new();
        assert!(!state.session_id.is_empty());
        assert!(state.auth_method.is_none());
        assert!(state.csrf_tokens.is_empty());
    }

    #[test]
    fn test_session_state_with_auth() {
        let state = SessionState::new().with_auth_method(AuthMethod::Bearer {
            token: SensitiveString::from("test".to_string()),
        });
        assert!(state.auth_method.is_some());
    }

    #[test]
    fn test_session_state_cookies() {
        let mut state = SessionState::new();
        state.add_cookie("session", "abc123");
        assert_eq!(state.get_cookie("session"), Some("abc123"));
        assert_eq!(state.get_cookie("nonexistent"), None);
    }

    #[test]
    fn test_session_state_headers() {
        let mut state = SessionState::new();
        state.add_header("Authorization", "Bearer token");
        assert_eq!(state.get_header("Authorization"), Some("Bearer token"));
    }

    #[test]
    fn test_session_state_custom_data() {
        let mut state = SessionState::new();
        state.set_custom_data("key", serde_json::json!("value"));
        assert_eq!(
            state.get_custom_data("key"),
            Some(&serde_json::json!("value"))
        );
    }

    #[test]
    fn test_session_state_is_authenticated() {
        let mut state = SessionState::new();
        assert!(!state.is_authenticated());

        state.auth_method = Some(AuthMethod::Bearer {
            token: SensitiveString::from("test".to_string()),
        });
        assert!(state.is_authenticated());
    }

    #[test]
    fn test_session_state_clear_auth() {
        let mut state = SessionState::new();
        state.auth_method = Some(AuthMethod::Bearer {
            token: SensitiveString::from("test".to_string()),
        });
        state.add_csrf_token(CsrfToken {
            token: "csrf".to_string(),
            url: "https://example.com".to_string(),
            header_name: "X-CSRF".to_string(),
            param_name: None,
            acquired_at: chrono::Utc::now(),
            expires_at: None,
            token_location: CsrfTokenLocation::Header,
        });

        state.clear_auth();
        assert!(state.auth_method.is_none());
        assert!(state.csrf_tokens.is_empty());
    }

    #[tokio::test]
    async fn test_authenticated_session_manager_create() {
        let manager = AuthenticatedSessionManager::new();
        let id = manager.create_session().await;
        assert!(!id.is_empty());
    }

    #[tokio::test]
    async fn test_authenticated_session_manager_get() {
        let manager = AuthenticatedSessionManager::new();
        let id = manager.create_session().await;
        let session = manager.get_session(&id).await;
        assert!(session.is_some());
        assert_eq!(session.unwrap().session_id, id);
    }

    #[tokio::test]
    async fn test_authenticated_session_manager_delete() {
        let manager = AuthenticatedSessionManager::new();
        let id = manager.create_session().await;
        manager.delete_session(&id).await;
        let session = manager.get_session(&id).await;
        assert!(session.is_none());
    }

    #[tokio::test]
    async fn test_authenticated_session_manager_set_auth() {
        let manager = AuthenticatedSessionManager::new();
        let id = manager.create_session().await;

        let auth = AuthMethod::Bearer {
            token: SensitiveString::from("token".to_string()),
        };

        let mut sessions = manager.sessions.write().await;
        if let Some(session) = sessions.get_mut(&id) {
            session.auth_method = Some(auth);
        }
        drop(sessions);

        let session = manager.get_session(&id).await.unwrap();
        assert!(session.auth_method.is_some());
    }

    #[tokio::test]
    async fn test_authenticated_session_manager_is_authenticated() {
        let manager = AuthenticatedSessionManager::new();
        let id = manager.create_session().await;

        assert!(!manager.is_authenticated(&id).await);

        let auth = AuthMethod::Bearer {
            token: SensitiveString::from("test".to_string()),
        };

        let mut sessions = manager.sessions.write().await;
        if let Some(session) = sessions.get_mut(&id) {
            session.auth_method = Some(auth);
        }
        drop(sessions);

        assert!(manager.is_authenticated(&id).await);
    }

    #[test]
    fn test_login_result_default() {
        let result = LoginResult::default();
        assert!(!result.success);
        assert!(result.session_cookies.is_empty());
        assert!(result.extracted_values.is_empty());
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_csrf_extractor_detects_token() {
        let extractor = CsrfExtractor::new();
        let html = r#"<input type="hidden" name="csrf_token" value="abc123">"#;
        let tokens = extractor.extract_from_html(html, "https://example.com/login");
        assert!(!tokens.is_empty());
        assert_eq!(tokens[0].token, "abc123");
    }

    #[test]
    fn test_form_detector_finds_login_form() {
        let detector = FormDetector::new();
        let html = r#"<form action="/login" method="POST">
            <input type="text" name="username">
            <input type="password" name="password">
        </form>"#;
        let form = detector.detect_login_form(html, "https://example.com");
        assert!(form.is_some());
        let form = form.unwrap();
        assert_eq!(form.action, "https://example.com/login");
        assert_eq!(form.username_field, "username");
        assert_eq!(form.password_field, "password");
    }

    #[test]
    fn test_login_sequence_default() {
        let sequence = LoginSequence::default();
        assert!(!sequence.logged_in_indicators.is_empty());
        assert!(!sequence.logged_out_indicators.is_empty());
    }
}
