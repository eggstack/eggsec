//! Advanced session management with authentication and CSRF handling.
//!
//! Provides comprehensive session tracking including multiple authentication
//! methods, CSRF token management, and login sequences.

use crate::types::SensitiveString;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthMethod {
    Basic { username: String, password: SensitiveString },
    Bearer { token: SensitiveString },
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
}

impl AuthMethod {
    pub fn apply_to_request(
        &self,
        request: &mut crate::tool::request::ToolRequest,
    ) -> Result<(), crate::error::SlapperError> {
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsrfToken {
    pub token: String,
    pub url: String,
    pub header_name: String,
    pub param_name: Option<String>,
    pub acquired_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl CsrfToken {
    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            return chrono::Utc::now() > expires;
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginSequence {
    pub name: String,
    pub steps: Vec<LoginStep>,
    pub csrf_required: bool,
    pub session_cookie_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LoginStep {
    Request {
        url: String,
        method: String,
        headers: HashMap<String, String>,
        body: Option<String>,
    },
    ExtractField {
        from_response_field: String,
        variable_name: String,
    },
    ExtractCookie {
        cookie_name: String,
        variable_name: String,
    },
    SetHeader {
        header_name: String,
        value: String,
    },
    Wait {
        milliseconds: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResult {
    pub success: bool,
    pub session_cookies: HashMap<String, String>,
    pub extracted_values: HashMap<String, String>,
    pub error_message: Option<String>,
}

impl Default for LoginResult {
    fn default() -> Self {
        Self {
            success: false,
            session_cookies: HashMap::new(),
            extracted_values: HashMap::new(),
            error_message: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub session_id: String,
    pub auth_method: Option<AuthMethod>,
    pub csrf_tokens: Vec<CsrfToken>,
    pub login_sequence: Option<LoginSequence>,
    pub login_result: Option<LoginResult>,
    pub cookies: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub custom_data: HashMap<String, serde_json::Value>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            auth_method: None,
            csrf_tokens: Vec::new(),
            login_sequence: None,
            login_result: None,
            cookies: HashMap::new(),
            headers: HashMap::new(),
            custom_data: HashMap::new(),
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
    }

    pub fn is_authenticated(&self) -> bool {
        self.auth_method.is_some()
            || !self.cookies.is_empty()
            || self.login_result.as_ref().map(|r| r.success).unwrap_or(false)
    }

    pub fn apply_to_request(
        &self,
        request: &mut crate::tool::request::ToolRequest,
    ) -> Result<(), crate::error::SlapperError> {
        if let Some(ref auth) = self.auth_method {
            auth.apply_to_request(request)?;
        }

        for (name, value) in &self.cookies {
            let cookie_str = format!("{}={}", name, value);
            if let Some(existing) = request.params.get("cookie_header").and_then(|v| v.as_str())
            {
                request.params["cookie_header"] =
                    serde_json::json!(format!("{}; {}", existing, cookie_str));
            } else {
                request.params["cookie_header"] = serde_json::json!(cookie_str);
            }
        }

        for (name, value) in &self.headers {
            let header_key = format!("header_{}", name);
            request.params[header_key] = serde_json::json!(value);
        }

        Ok(())
    }
}

pub struct AdvancedSessionManager {
    sessions: Arc<RwLock<HashMap<String, SessionState>>>,
    default_ttl_seconds: i64,
}

impl AdvancedSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            default_ttl_seconds: 3600,
        }
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

    pub async fn cleanup_expired(&self) {
        let ttl = chrono::Duration::seconds(self.default_ttl_seconds);
        let now = chrono::Utc::now();
        let mut sessions = self.sessions.write().await;

        sessions.retain(|_, state| {
            if let Some(result) = &state.login_result {
                if result.success {
                    return true;
                }
            }
            true
        });
    }

    pub async fn set_auth_method(
        &self,
        session_id: &str,
        auth_method: AuthMethod,
    ) -> Result<(), crate::error::SlapperError> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| crate::error::SlapperError::NotFound("Session not found".to_string()))?;
        session.auth_method = Some(auth_method);
        Ok(())
    }

    pub async fn add_csrf_token(&self, session_id: &str, token: CsrfToken) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.add_csrf_token(token);
        }
    }

    pub async fn get_csrf_token(&self, session_id: &str, url: &str) -> Option<CsrfToken> {
        let sessions = self.sessions.read().await;
        sessions
            .get(session_id)
            .and_then(|s| s.get_csrf_token(url).cloned())
    }

    pub async fn add_cookie(&self, session_id: &str, name: &str, value: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.add_cookie(name, value);
        }
    }

    pub async fn set_login_result(&self, session_id: &str, result: LoginResult) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.login_result = Some(result);
        }
    }

    pub async fn clear_auth(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.clear_auth();
        }
    }

    pub async fn is_authenticated(&self, session_id: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions
            .get(session_id)
            .map(|s| s.is_authenticated())
            .unwrap_or(false)
    }
}

impl Default for AdvancedSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

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
        assert_eq!(state.get_custom_data("key"), Some(&serde_json::json!("value")));
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
        });

        state.clear_auth();
        assert!(state.auth_method.is_none());
        assert!(state.csrf_tokens.is_empty());
    }

    #[tokio::test]
    async fn test_advanced_session_manager_create() {
        let manager = AdvancedSessionManager::new();
        let id = manager.create_session().await;
        assert!(!id.is_empty());
    }

    #[tokio::test]
    async fn test_advanced_session_manager_get() {
        let manager = AdvancedSessionManager::new();
        let id = manager.create_session().await;
        let session = manager.get_session(&id).await;
        assert!(session.is_some());
        assert_eq!(session.unwrap().session_id, id);
    }

    #[tokio::test]
    async fn test_advanced_session_manager_delete() {
        let manager = AdvancedSessionManager::new();
        let id = manager.create_session().await;
        manager.delete_session(&id).await;
        let session = manager.get_session(&id).await;
        assert!(session.is_none());
    }

    #[tokio::test]
    async fn test_advanced_session_manager_set_auth() {
        let manager = AdvancedSessionManager::new();
        let id = manager.create_session().await;

        let auth = AuthMethod::Bearer {
            token: SensitiveString::from("token".to_string()),
        };
        manager.set_auth_method(&id, auth).await.unwrap();

        let session = manager.get_session(&id).await.unwrap();
        assert!(session.auth_method.is_some());
    }

    #[tokio::test]
    async fn test_advanced_session_manager_is_authenticated() {
        let manager = AdvancedSessionManager::new();
        let id = manager.create_session().await;

        assert!(!manager.is_authenticated(&id).await);

        let auth = AuthMethod::Bearer {
            token: SensitiveString::from("token".to_string()),
        };
        manager.set_auth_method(&id, auth).await.unwrap();

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
}
