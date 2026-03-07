use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    pub id: String,
    pub tool: String,
    pub target: Target,
    pub params: serde_json::Value,
    pub options: RequestOptions,
}

impl ToolRequest {
    pub fn new(tool: impl Into<String>, target: Target) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            tool: tool.into(),
            target,
            params: serde_json::json!({}),
            options: RequestOptions::default(),
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
    if pattern.starts_with("*.") {
        let suffix = &pattern[2..];
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
    pub headers: Option<HashMap<String, String>>,
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
    pub credentials: HashMap<String, String>,
}

impl AuthConfig {
    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        let mut creds = HashMap::new();
        creds.insert("username".to_string(), username.into());
        creds.insert("password".to_string(), password.into());
        Self {
            auth_type: AuthType::Basic,
            credentials: creds,
        }
    }

    pub fn bearer(token: impl Into<String>) -> Self {
        let mut creds = HashMap::new();
        creds.insert("token".to_string(), token.into());
        Self {
            auth_type: AuthType::Bearer,
            credentials: creds,
        }
    }

    pub fn api_key(key: impl Into<String>, header: impl Into<String>) -> Self {
        let mut creds = HashMap::new();
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
