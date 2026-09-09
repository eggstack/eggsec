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
    pub scope: Option<ScopeSpec>,
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

    pub fn with_scope(mut self, scope: ScopeSpec) -> Self {
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

/// Declarative scope specification attached to a tool request.
///
/// This is transport data describing caller intent. It is **not** an
/// authorization decision: only the engine (`eggsec::config::Scope` evaluated
/// through `EnforcementContext`) may authorize execution. A `ScopeSpec` must
/// be converted into an engine scope via the explicit fallible conversion in
/// `eggsec::config` before any policy check, and the effective authorization
/// is the intersection of the engine scope and the converted specification
/// (both must allow; either may deny).
///
/// Wire compatibility: serde field names are unchanged from the legacy
/// `Scope` DTO, so existing JSON/Python payloads deserialize. The Rust type
/// was renamed to make the declarative role explicit; `Scope` remains as a
/// deprecated alias.
///
/// There is intentionally no `is_allowed()`/`authorize()` method on this
/// type. Glob matching here previously diverged from engine CIDR,
/// DNS multi-address, port, and non-public-address policy. Callers needing a
/// decision must convert and evaluate through the engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeSpec {
    /// Caller-declared hostname patterns (e.g. `example.com`, `*.example.com`).
    /// Interpreted only after conversion to engine `ScopeRule`s.
    #[serde(default)]
    pub allowed_patterns: Vec<String>,
    /// Caller-declared exclusions. Never weakened during conversion.
    #[serde(default)]
    pub excluded_patterns: Vec<String>,
    /// Caller-declared IP/CIDR literals. Each entry must parse as an IP
    /// address or CIDR during conversion; unparsable entries fail closed.
    #[serde(default)]
    pub allowed_ips: Vec<String>,
    /// Hint about subdomain intent for hostname patterns. Advisory only;
    /// authoritative subdomain semantics come from the engine `*.` rule
    /// handling after conversion.
    #[serde(default)]
    pub allow_subdomains: bool,
}

/// Preferred explicit name for the protocol-neutral declarative scope.
pub type ToolScopeSpec = ScopeSpec;

/// Deprecated compatibility alias for the legacy `Scope` DTO name.
///
/// The type is identical to [`ScopeSpec`]; the alias exists only so existing
/// `eggsec_tool_core::Scope` / `eggsec::tool::Scope` paths keep compiling.
/// It carries no authorization semantics. Migrate to `ScopeSpec` or
/// `ToolScopeSpec`.
#[deprecated(
    since = "0.1.0",
    note = "Declarative only; renamed to ScopeSpec (alias ToolScopeSpec). It never authorizes execution — convert to eggsec::config::Scope and evaluate via EnforcementContext."
)]
pub type Scope = ScopeSpec;

impl Default for ScopeSpec {
    /// Fail-closed default: no declared allowances.
    ///
    /// This is intentionally *not* the legacy permissive `["*"]` default.
    /// An empty specification converts to an engine scope with no allowed
    /// targets, which denies. Use [`ScopeSpec::allow_all`] only when the
    /// caller explicitly intends a permissive declaration (still subject to
    /// engine policy intersection).
    fn default() -> Self {
        Self {
            allowed_patterns: Vec::new(),
            excluded_patterns: Vec::new(),
            allowed_ips: Vec::new(),
            allow_subdomains: false,
        }
    }
}

impl ScopeSpec {
    /// Explicitly permissive declaration (legacy `Default` shape).
    ///
    /// Still declarative only: after conversion the engine scope and policy
    /// must also allow before anything executes.
    pub fn allow_all() -> Self {
        Self {
            allowed_patterns: vec!["*".to_string()],
            excluded_patterns: Vec::new(),
            allowed_ips: Vec::new(),
            allow_subdomains: true,
        }
    }

    /// Explicit deny-all declaration.
    pub fn deny_all() -> Self {
        Self {
            allowed_patterns: Vec::new(),
            excluded_patterns: vec!["*".to_string()],
            allowed_ips: Vec::new(),
            allow_subdomains: false,
        }
    }

    /// Returns `true` when no allowances are declared.
    pub fn is_empty_declaration(&self) -> bool {
        self.allowed_patterns.is_empty() && self.allowed_ips.is_empty()
    }
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
            timeout_ms: Some(30_000),
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
