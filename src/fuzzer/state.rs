#![allow(dead_code)]

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, SET_COOKIE};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

fn try_header_name(s: &str) -> Option<HeaderName> {
    HeaderName::from_bytes(s.as_bytes()).ok()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpSession {
    pub cookies: HashMap<String, Cookie>,
    pub tokens: HashMap<String, String>,
    #[serde(skip)]
    pub headers: HeaderMap,
    pub state_data: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub expires: Option<String>,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: Option<String>,
}

impl Default for HttpSession {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpSession {
    pub fn new() -> Self {
        Self {
            cookies: HashMap::new(),
            tokens: HashMap::new(),
            headers: HeaderMap::new(),
            state_data: HashMap::new(),
        }
    }

    pub fn from_cookies(cookies: &str) -> Self {
        let mut session = Self::new();
        for cookie in cookies.split(';') {
            let parts: Vec<&str> = cookie.splitn(2, '=').collect();
            if parts.len() == 2 {
                let name = parts[0].trim().to_string();
                let value = parts[1].trim().to_string();
                session.cookies.insert(name, Cookie {
                    value,
                    domain: None,
                    path: None,
                    expires: None,
                    http_only: false,
                    secure: false,
                    same_site: None,
                });
            }
        }
        session
    }

    pub fn add_cookie(&mut self, name: String, value: String) {
        self.cookies.insert(name, Cookie {
            value,
            domain: None,
            path: None,
            expires: None,
            http_only: false,
            secure: false,
            same_site: None,
        });
    }

    pub fn add_cookie_full(&mut self, name: String, cookie: Cookie) {
        self.cookies.insert(name, cookie);
    }

    pub fn get_cookie_header(&self) -> String {
        self.cookies
            .iter()
            .map(|(k, v)| format!("{}={}", k, v.value))
            .collect::<Vec<_>>()
            .join("; ")
    }

    pub fn update_from_response(&mut self, headers: &HeaderMap) {
        if let Some(cookie_headers) = headers.get_all(SET_COOKIE).iter().next() {
            if let Ok(cookie_str) = cookie_headers.to_str() {
                self.parse_set_cookie(cookie_str);
            }
        }
        
        for (name, value) in headers.iter() {
            if let Ok(_value_str) = value.to_str() {
                if name.as_str().starts_with("X-") || name.as_str() == "Authorization" {
                    self.headers.insert(name.clone(), value.clone());
                }
            }
        }
    }

    fn parse_set_cookie(&mut self, cookie_str: &str) {
        let parts: Vec<&str> = cookie_str.split(';').collect();
        if parts.is_empty() {
            return;
        }
        
        let name_value: Vec<&str> = parts[0].splitn(2, '=').collect();
        if name_value.len() != 2 {
            return;
        }
        
        let name = name_value[0].trim().to_string();
        let value = name_value[1].trim().to_string();
        
        let mut domain = None;
        let mut path = None;
        let mut expires = None;
        let mut http_only = false;
        let mut secure = false;
        let mut same_site = None;
        
        for part in parts.iter().skip(1) {
            let part = part.trim();
            let lower = part.to_lowercase();
            if lower.starts_with("domain=") {
                domain = Some(part[7..].trim().to_string());
            } else if lower.starts_with("path=") {
                path = Some(part[5..].trim().to_string());
            } else if lower.starts_with("expires=") {
                expires = Some(part[8..].trim().to_string());
            } else if lower == "httponly" {
                http_only = true;
            } else if lower == "secure" {
                secure = true;
            } else if lower.starts_with("samesite=") {
                same_site = Some(part[9..].trim().to_string());
            }
        }
        
        self.cookies.insert(name, Cookie {
            value,
            domain,
            path,
            expires,
            http_only,
            secure,
            same_site,
        });
    }

    pub fn set_token(&mut self, key: &str, value: &str) {
        self.tokens.insert(key.to_string(), value.to_string());
    }

    pub fn get_token(&self, key: &str) -> Option<&String> {
        self.tokens.get(key)
    }

    pub fn set_state(&mut self, key: &str, value: &str) {
        self.state_data.insert(key.to_string(), value.to_string());
    }

    pub fn get_state(&self, key: &str) -> Option<&String> {
        self.state_data.get(key)
    }
}

pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, HttpSession>>>,
    default_session: Arc<RwLock<HttpSession>>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            default_session: Arc::new(RwLock::new(HttpSession::new())),
        }
    }

    pub async fn create_session(&self, name: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(name.to_string(), HttpSession::new());
    }

    pub async fn get_session(&self, name: &str) -> Option<HttpSession> {
        let sessions = self.sessions.read().await;
        sessions.get(name).cloned()
    }

    pub async fn update_session(&self, name: &str, session: HttpSession) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(name.to_string(), session);
    }

    pub async fn get_default(&self) -> HttpSession {
        self.default_session.read().await.clone()
    }

    pub async fn update_default(&self, session: &HttpSession) {
        let mut default = self.default_session.write().await;
        *default = session.clone();
    }

    pub async fn add_cookie(&self, name: &str, value: &str) {
        let mut default = self.default_session.write().await;
        default.add_cookie(name.to_string(), value.to_string());
    }

    pub async fn update_from_response(&self, headers: &HeaderMap) {
        let mut default = self.default_session.write().await;
        default.update_from_response(headers);
    }

    pub async fn apply_to_headers(&self, headers: &mut HeaderMap) {
        let session = self.default_session.read().await;
        
        if !session.cookies.is_empty() {
            let cookie_header = session.get_cookie_header();
            if !cookie_header.is_empty() {
                if let Ok(value) = HeaderValue::from_str(&cookie_header) {
                    headers.insert(reqwest::header::COOKIE, value);
                }
            }
        }
        
        for (name, value) in &session.headers {
            headers.insert(name.clone(), value.clone());
        }
    }
}

pub struct AuthHandler {
    pub session_manager: SessionManager,
    pub auth_type: AuthType,
    pub credentials: Option<AuthCredentials>,
}

#[derive(Debug, Clone)]
pub enum AuthType {
    None,
    Basic,
    Bearer,
    ApiKey,
    OAuth2,
    JWT,
}

#[derive(Debug, Clone)]
pub struct AuthCredentials {
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
    pub api_key: Option<String>,
    pub api_key_header: Option<String>,
}

impl AuthHandler {
    pub fn new() -> Self {
        Self {
            session_manager: SessionManager::new(),
            auth_type: AuthType::None,
            credentials: None,
        }
    }

    pub fn with_basic_auth(username: &str, password: &str) -> Self {
        Self {
            session_manager: SessionManager::new(),
            auth_type: AuthType::Basic,
            credentials: Some(AuthCredentials {
                username: Some(username.to_string()),
                password: Some(password.to_string()),
                token: None,
                api_key: None,
                api_key_header: None,
            }),
        }
    }

    pub fn with_bearer_token(token: &str) -> Self {
        Self {
            session_manager: SessionManager::new(),
            auth_type: AuthType::Bearer,
            credentials: Some(AuthCredentials {
                username: None,
                password: None,
                token: Some(token.to_string()),
                api_key: None,
                api_key_header: None,
            }),
        }
    }

    pub fn with_api_key(key: &str, header: &str) -> Self {
        Self {
            session_manager: SessionManager::new(),
            auth_type: AuthType::ApiKey,
            credentials: Some(AuthCredentials {
                username: None,
                password: None,
                token: None,
                api_key: Some(key.to_string()),
                api_key_header: Some(header.to_string()),
            }),
        }
    }

    pub fn apply_auth(&self, headers: &mut HeaderMap) {
        match &self.auth_type {
            AuthType::None => {}
            AuthType::Basic => {
                if let Some(creds) = &self.credentials {
                    if let (Some(username), Some(password)) = (&creds.username, &creds.password) {
                        use base64::Engine;
                        let encoded = base64::engine::general_purpose::STANDARD.encode(
                            format!("{}:{}", username, password)
                        );
                        if let Ok(value) = HeaderValue::from_str(&format!("Basic {}", encoded)) {
                            headers.insert(reqwest::header::AUTHORIZATION, value);
                        }
                    }
                }
            }
            AuthType::Bearer => {
                if let Some(creds) = &self.credentials {
                    if let Some(token) = &creds.token {
                        if let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", token)) {
                            headers.insert(reqwest::header::AUTHORIZATION, value);
                        }
                    }
                }
            }
            AuthType::ApiKey => {
                if let Some(creds) = &self.credentials {
                    if let (Some(key), Some(header)) = (&creds.api_key, &creds.api_key_header) {
                        if let Some(name) = try_header_name(header) {
                            if let Ok(value) = HeaderValue::from_str(key) {
                                headers.insert(name, value);
                            }
                        }
                    }
                }
            }
            AuthType::OAuth2 | AuthType::JWT => {
                if let Some(creds) = &self.credentials {
                    if let Some(token) = &creds.token {
                        if let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", token)) {
                            headers.insert(reqwest::header::AUTHORIZATION, value);
                        }
                    }
                }
            }
        }
    }
}

impl Default for AuthHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_parsing() {
        let mut session = HttpSession::new();
        session.parse_set_cookie("session=abc123; Path=/; HttpOnly; Secure");
        
        assert!(session.cookies.contains_key("session"));
        let cookie = &session.cookies["session"];
        assert_eq!(cookie.value, "abc123");
        assert!(cookie.http_only);
        assert!(cookie.secure);
    }
}
