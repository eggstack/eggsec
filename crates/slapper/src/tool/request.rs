use rustc_hash::FxHashMap;

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    #[allow(clippy::should_implement_trait)]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn wrap(self) -> CancellationTokenHandle {
        CancellationTokenHandle {
            token: Arc::new(self),
            request_id: None,
        }
    }
}

impl Clone for CancellationToken {
    fn clone(&self) -> Self {
        Self {
            cancelled: Arc::clone(&self.cancelled),
        }
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancellationTokenHandle {
    #[serde(skip)]
    token: Arc<CancellationToken>,
    #[serde(skip)]
    request_id: Option<String>,
}

impl CancellationTokenHandle {
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    pub fn cancel(&self) {
        self.token.cancel();
    }

    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    pub id: String,
    pub tool: String,
    pub target: Target,
    pub params: serde_json::Value,
    pub options: RequestOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancellation_token: Option<CancellationTokenHandle>,
}

impl ToolRequest {
    pub fn new(tool: impl Into<String>, target: Target) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            tool: tool.into(),
            target,
            params: serde_json::json!({}),
            options: RequestOptions::default(),
            cancellation_token: None,
        }
    }

    pub fn with_params(mut self, params: serde_json::Value) -> Self {
        self.params = params;
        self
    }

    pub fn with_options(mut self, options: RequestOptions) -> Self {
        self.options = options;
        self
    }

    pub fn with_cancellation(mut self, token: CancellationTokenHandle) -> Self {
        self.cancellation_token = Some(token);
        self
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token
            .as_ref()
            .map(|t| t.is_cancelled())
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub target_type: TargetType,
    pub value: String,
    pub scope: Option<Scope>,
}

impl Target {
    pub fn url(value: impl Into<String>) -> Self {
        Self {
            target_type: TargetType::Url,
            value: value.into(),
            scope: None,
        }
    }

    pub fn domain(value: impl Into<String>) -> Self {
        Self {
            target_type: TargetType::Domain,
            value: value.into(),
            scope: None,
        }
    }

    pub fn ip(value: impl Into<String>) -> Self {
        Self {
            target_type: TargetType::Ip,
            value: value.into(),
            scope: None,
        }
    }

    pub fn cidr(value: impl Into<String>) -> Self {
        Self {
            target_type: TargetType::Cidr,
            value: value.into(),
            scope: None,
        }
    }

    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = Some(scope);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetType {
    Url,
    Domain,
    Ip,
    Cidr,
    File,
}

impl std::fmt::Display for TargetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetType::Url => write!(f, "url"),
            TargetType::Domain => write!(f, "domain"),
            TargetType::Ip => write!(f, "ip"),
            TargetType::Cidr => write!(f, "cidr"),
            TargetType::File => write!(f, "file"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    pub allowed_patterns: Vec<String>,
    pub excluded_patterns: Vec<String>,
    pub allowed_ips: Vec<String>,
    pub allow_subdomains: bool,
}

impl Default for Scope {
    fn default() -> Self {
        Self {
            allowed_patterns: vec!["*".to_string()],
            excluded_patterns: vec![],
            allowed_ips: vec![],
            allow_subdomains: true,
        }
    }
}

impl Scope {
    pub fn is_allowed(&self, target: &str) -> bool {
        if !self.excluded_patterns.is_empty() {
            for pattern in &self.excluded_patterns {
                if glob_match(pattern, target) {
                    return false;
                }
            }
        }

        for pattern in &self.allowed_patterns {
            if glob_match(pattern, target) {
                return true;
            }
        }

        false
    }
}

fn glob_match(pattern: &str, target: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        return target.ends_with(suffix) || target == suffix;
    }
    pattern == target
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestOptions {
    pub timeout_ms: Option<u64>,
    pub concurrency: Option<usize>,
    pub rate_limit: Option<f64>,
    pub proxy: Option<String>,
    pub headers: Option<FxHashMap<String, String>>,
    pub auth: Option<AuthConfig>,
    pub stealth: bool,
    pub follow_redirects: bool,
    pub verify_ssl: bool,
}

impl Default for RequestOptions {
    fn default() -> Self {
        Self {
            timeout_ms: Some(30000),
            concurrency: Some(10),
            rate_limit: None,
            proxy: None,
            headers: None,
            auth: None,
            stealth: false,
            follow_redirects: true,
            verify_ssl: true,
        }
    }
}

impl RequestOptions {
    pub fn timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = Some(ms);
        self
    }

    pub fn concurrency(mut self, n: usize) -> Self {
        self.concurrency = Some(n);
        self
    }

    pub fn proxy(mut self, proxy: impl Into<String>) -> Self {
        self.proxy = Some(proxy.into());
        self
    }

    pub fn auth(mut self, auth: AuthConfig) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn stealth(mut self) -> Self {
        self.stealth = true;
        self
    }

    pub fn insecure(mut self) -> Self {
        self.verify_ssl = false;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub auth_type: AuthType,
    pub credentials: FxHashMap<String, String>,
}

impl AuthConfig {
    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        let mut creds = FxHashMap::default();
        creds.insert("username".to_string(), username.into());
        creds.insert("password".to_string(), password.into());
        Self {
            auth_type: AuthType::Basic,
            credentials: creds,
        }
    }

    pub fn bearer(token: impl Into<String>) -> Self {
        let mut creds = FxHashMap::default();
        creds.insert("token".to_string(), token.into());
        Self {
            auth_type: AuthType::Bearer,
            credentials: creds,
        }
    }

    pub fn api_key(key: impl Into<String>, header: impl Into<String>) -> Self {
        let mut creds = FxHashMap::default();
        creds.insert("key".to_string(), key.into());
        creds.insert("header".to_string(), header.into());
        Self {
            auth_type: AuthType::ApiKey,
            credentials: creds,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthType {
    None,
    Basic,
    Bearer,
    ApiKey,
    OAuth2,
}

impl std::fmt::Display for AuthType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthType::None => write!(f, "none"),
            AuthType::Basic => write!(f, "basic"),
            AuthType::Bearer => write!(f, "bearer"),
            AuthType::ApiKey => write!(f, "api_key"),
            AuthType::OAuth2 => write!(f, "oauth2"),
        }
    }
}
