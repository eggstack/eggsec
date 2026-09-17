//! Compatibility facade over the transport-neutral core (Phase D).
//!
//! [`LoadTestRunner`] and [`LoadTestRunConfig`] preserve the pre-Phase-D
//! public paths used by CLI/pipeline/tool/distributed/Python callers. New
//! code should compose [`LoadTestPlan`](super::plan::LoadTestPlan) +
//! [`LoadTestExecutor`](super::executor::LoadTestExecutor) directly with an
//! explicit [`NetworkAuthority`](eggsec_transport::NetworkAuthority) and
//! [`ProgressSink`](super::progress::ProgressSink).
//!
//! `tui_mode` is retained on these facade types for source compatibility but
//! is ignored by execution: the core never prints and never touches
//! `indicatif`. CLI progress rendering lives in `run_cli` (process-host
//! layer); TUI/library/daemon consumers use structured events.
//!
//! Execution scope is mandatory: the runner stores `Option<Scope>` and
//! ordinary `run()` fails closed without an attached scope. There is no
//! wildcard default. Attach the `EnforcementContext` scope snapshot that
//! authorized the operation via `with_scope()`/`set_scope()`, or use
//! `run_with(transport, authority, ...)` with a caller-supplied authority.

use std::sync::Arc;
use std::time::Duration;

use eggsec_transport::NetworkAuthority;
use tokio_util::sync::CancellationToken;

use super::adapter::{plan_from_adapter, AdapterInput, RequestTemplate};
use super::executor::LoadTestExecutor;
use super::plan::LoadTestPlan;
use super::progress::NoopSink;
use crate::config::EggsecConfig;
use crate::error::{EggsecError, Result};
use crate::output::report::Report;
use crate::types::CommonHttpArgs;

/// Plain load-test run configuration (no Clap derives).
///
/// Engine-facing contract used by the pipeline, Python bindings, and
/// tool/API consumers. CLI parsing converts `LoadArgs` into this type.
///
/// `tui_mode` is deprecated and ignored: progress is structured (see
/// `progress.rs`). It remains so existing construction sites compile.
#[derive(Debug, Clone)]
pub struct LoadTestRunConfig {
    pub url: String,
    pub requests: u64,
    pub concurrency: usize,
    pub timeout: Duration,
    pub method: String,
    pub body: Option<String>,
    pub headers: Vec<String>,
    pub common: CommonHttpArgs,
    pub tui_mode: bool,
}

impl LoadTestRunConfig {
    pub fn new(
        url: impl Into<String>,
        requests: u64,
        concurrency: usize,
        timeout: Duration,
    ) -> Self {
        Self {
            url: url.into(),
            requests,
            concurrency,
            timeout,
            method: "GET".to_string(),
            body: None,
            headers: Vec::new(),
            common: CommonHttpArgs::default(),
            tui_mode: false,
        }
    }
}

#[cfg(feature = "cli")]
impl From<crate::cli::LoadArgs> for LoadTestRunConfig {
    fn from(args: crate::cli::LoadArgs) -> Self {
        Self {
            url: args.url,
            requests: args.requests,
            concurrency: args.concurrency,
            timeout: Duration::from_secs(args.timeout.unwrap_or(crate::cli::timeout::LOAD_TIMEOUT)),
            method: args.method,
            body: args.body,
            headers: args.headers,
            common: args.common.into(),
            tui_mode: false,
        }
    }
}

pub struct LoadTestRunner {
    url: String,
    total_requests: u64,
    concurrency: usize,
    timeout: Duration,
    method: String,
    body: Option<String>,
    raw_headers: Vec<String>,
    insecure: bool,
    proxy: Option<String>,
    proxy_auth: Option<String>,
    user_agent: Option<String>,
    rate_limit: Option<u32>,
    auth: Option<String>,
    bearer: Option<String>,
    cookie: Option<String>,
    api_key: Option<String>,
    /// Explicit execution scope. `None` until the caller attaches the scope
    /// snapshot that authorized the operation. Ordinary `run()` fails closed
    /// without it; `run_with(transport, authority, ...)` stays explicit and
    /// never consults this field.
    scope: Option<crate::config::Scope>,
    config_defaults: Option<ConfigDefaults>,
    /// Deprecated, ignored by execution (retained for source compat).
    #[allow(dead_code)]
    tui_mode: bool,
}

#[derive(Debug, Clone, Default)]
struct ConfigDefaults {
    timeout_secs: Option<u64>,
    verify_tls: bool,
    proxy: Option<String>,
    proxy_auth: Option<String>,
    rate: Option<u32>,
    user_agent: Option<String>,
    default_headers: std::collections::HashMap<String, String>,
}

impl LoadTestRunner {
    pub fn new(
        url: String,
        total_requests: u64,
        concurrency: usize,
        timeout: Duration,
    ) -> Result<Self> {
        Self::new_with_tui_mode(url, total_requests, concurrency, timeout, false)
    }

    pub fn new_with_tui_mode(
        url: String,
        total_requests: u64,
        concurrency: usize,
        timeout: Duration,
        tui_mode: bool,
    ) -> Result<Self> {
        // Validate eagerly (same messages as the plan constructor).
        LoadTestPlan::new(url.clone(), total_requests, concurrency, timeout)
            .map_err(EggsecError::Validation)?;
        Ok(Self {
            url,
            total_requests,
            concurrency,
            timeout,
            method: "GET".to_string(),
            body: None,
            raw_headers: Vec::new(),
            insecure: false,
            proxy: None,
            proxy_auth: None,
            user_agent: None,
            rate_limit: None,
            auth: None,
            bearer: None,
            cookie: None,
            api_key: None,
            // No wildcard default: the caller must attach the scope snapshot
            // that authorized the operation (fail-closed `run()`).
            scope: None,
            config_defaults: None,
            tui_mode,
        })
    }

    /// Attach an explicit scope for per-request authority checks.
    #[must_use]
    pub fn with_scope(mut self, scope: crate::config::Scope) -> Self {
        self.scope = Some(scope);
        self
    }

    /// Attach an explicit scope for per-request authority checks.
    pub fn set_scope(&mut self, scope: crate::config::Scope) {
        self.scope = Some(scope);
    }

    /// Borrow the explicit execution scope, if attached.
    #[must_use]
    pub fn scope(&self) -> Option<&crate::config::Scope> {
        self.scope.as_ref()
    }

    /// Require the attached execution scope, failing closed before I/O.
    fn require_scope(&self) -> Result<crate::config::Scope> {
        self.scope.clone().ok_or_else(|| {
            EggsecError::Validation(
                "load-test execution scope is missing: attach the EnforcementContext scope \
                 snapshot via with_scope()/set_scope() (no wildcard default; see \
                 ApprovedExecution)"
                    .to_string(),
            )
        })
    }

    #[cfg(feature = "cli")]
    pub fn from_args_with_tui_mode(args: crate::cli::LoadArgs, tui_mode: bool) -> Result<Self> {
        Self::from_config_with_mode(args.into(), tui_mode)
    }

    #[cfg(feature = "cli")]
    pub fn from_args_with_config(
        args: crate::cli::LoadArgs,
        config: &EggsecConfig,
    ) -> Result<Self> {
        Self::from_config_with_engine(args.into(), config)
    }

    /// Construct a runner from a plain [`LoadTestRunConfig`].
    pub fn from_config(cfg: LoadTestRunConfig) -> Result<Self> {
        Self::from_config_with_mode(cfg, false)
    }

    /// Construct a runner from a plain [`LoadTestRunConfig`] with explicit
    /// TUI mode (deprecated, ignored).
    pub fn from_config_with_mode(cfg: LoadTestRunConfig, tui_mode: bool) -> Result<Self> {
        let mut runner = Self::new_with_tui_mode(
            cfg.url,
            cfg.requests,
            cfg.concurrency,
            cfg.timeout,
            tui_mode,
        )?;
        runner.method = cfg.method;
        runner.body = cfg.body;
        runner.raw_headers = cfg.headers;
        runner.apply_common_fields(&cfg.common);
        Ok(runner)
    }

    /// Construct a runner from a plain [`LoadTestRunConfig`] and an
    /// [`EggsecConfig`]. Honors config-derived defaults for headers, proxy,
    /// user-agent, and timeout fallbacks where the plain config leaves them
    /// unset.
    pub fn from_config_with_engine(cfg: LoadTestRunConfig, config: &EggsecConfig) -> Result<Self> {
        let timeout = if cfg.timeout.is_zero() {
            Duration::from_secs(config.http.timeout_secs.max(1))
        } else {
            cfg.timeout
        };
        let mut runner =
            Self::new_with_tui_mode(cfg.url, cfg.requests, cfg.concurrency, timeout, false)?;
        runner.method = cfg.method;
        runner.body = cfg.body;
        runner.raw_headers = cfg.headers;
        runner.apply_common_fields(&cfg.common);
        runner.config_defaults = Some(ConfigDefaults {
            timeout_secs: Some(config.http.timeout_secs),
            verify_tls: config.http.verify_tls,
            proxy: config.http.proxy.clone(),
            proxy_auth: config
                .http
                .proxy_auth
                .as_ref()
                .map(|s| s.expose_secret().to_string()),
            rate: config.scan.rate_limit_per_second,
            user_agent: config.http.default_user_agent.clone(),
            default_headers: config
                .http
                .default_headers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        });
        Ok(runner)
    }

    fn apply_common_fields(&mut self, common: &CommonHttpArgs) {
        self.insecure = common.insecure;
        self.proxy = common.proxy.clone();
        self.proxy_auth = common.proxy_auth.clone();
        self.user_agent = common.user_agent.clone();
        self.rate_limit = common.rate_limit;
        self.auth = common.auth.clone();
        self.bearer = common.bearer.clone();
        self.cookie = common.cookie.clone();
        self.api_key = common.api_key.clone();
        if common.rate_limit == Some(0) {
            tracing::warn!("Rate limit of 0 is invalid, ignoring rate limit setting");
            self.rate_limit = None;
        } else if let Some(rate) = common.rate_limit {
            if rate > 100_000 {
                tracing::warn!(
                    "Rate limit {} req/s exceeds recommended maximum of 100,000; \
                     rate limiting may be ineffective at this level",
                    rate
                );
            }
        }
    }

    pub fn set_common(&mut self, common: CommonHttpArgs) {
        self.apply_common_fields(&common);
    }

    pub fn set_common_with_config(&mut self, common: CommonHttpArgs, config: &EggsecConfig) {
        self.apply_common_fields(&common);
        // Merge config defaults (same precedence as the pre-Phase-D runner:
        // explicit CLI flags win, config fills gaps; insecure is OR).
        self.insecure = common.insecure || !config.http.verify_tls;
        if self.proxy.is_none() {
            self.proxy = config.http.proxy.clone();
        }
        if self.proxy_auth.is_none() {
            self.proxy_auth = config
                .http
                .proxy_auth
                .as_ref()
                .map(|s| s.expose_secret().to_string());
        }
        if self.rate_limit.is_none() {
            self.rate_limit = config.scan.rate_limit_per_second;
        }
        if self.user_agent.is_none() {
            self.user_agent = config.http.default_user_agent.clone();
        }
        let defaults: std::collections::HashMap<String, String> = config
            .http
            .default_headers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        self.config_defaults = Some(ConfigDefaults {
            timeout_secs: Some(config.http.timeout_secs),
            verify_tls: true,
            proxy: None,
            proxy_auth: None,
            rate: None,
            user_agent: None,
            default_headers: defaults,
        });
    }

    pub fn set_method(&mut self, method: String) {
        self.method = method;
    }

    pub fn set_body(&mut self, body: String) {
        self.body = Some(body);
    }

    pub fn add_header(&mut self, key: String, value: String) {
        self.raw_headers.push(format!("{key}:{value}"));
    }

    fn build_plan(&self) -> Result<(LoadTestPlan, RequestTemplate)> {
        let defaults = self.config_defaults.clone().unwrap_or(ConfigDefaults {
            verify_tls: true,
            ..Default::default()
        });
        let input = AdapterInput {
            url: &self.url,
            requests: self.total_requests,
            concurrency: self.concurrency,
            timeout: self.timeout,
            method: &self.method,
            body: self.body.as_deref(),
            headers: &self.raw_headers,
            insecure: self.insecure,
            proxy: self.proxy.as_deref(),
            proxy_auth: self.proxy_auth.as_deref(),
            auth: self.auth.as_deref(),
            bearer: self.bearer.as_deref(),
            cookie: self.cookie.as_deref(),
            api_key: self.api_key.as_deref(),
            user_agent: self.user_agent.as_deref(),
            rate_limit: self.rate_limit,
        };
        plan_from_adapter(
            input,
            defaults.timeout_secs,
            defaults.verify_tls,
            defaults.proxy,
            defaults.proxy_auth,
            defaults.rate,
            defaults.user_agent,
            &defaults.default_headers,
        )
        .map_err(EggsecError::Validation)
    }

    /// Build the transport-neutral plan (for executor composition).
    pub fn plan(&self) -> Result<(LoadTestPlan, RequestTemplate)> {
        self.build_plan()
    }

    /// Run with the attached execution scope and no progress output.
    /// Fails closed without a scope (no wildcard default). CLI presentation
    /// uses `run_cli` with an indicatif renderer; TUI/daemon use the executor
    /// with a structured sink.
    pub async fn run(&self) -> Result<super::metrics::LoadTestResults> {
        self.run_with_cancellation(CancellationToken::new()).await
    }

    /// Run with an explicit cancellation token (daemon/runtime composition).
    /// Requires an attached execution scope; fails before transport creation
    /// or I/O when missing. Direct and supported proxied routes dispatch
    /// through the pinned Eggfetch backend; unsupported proxied shapes fail
    /// closed (no Reqwest fallback).
    pub async fn run_with_cancellation(
        &self,
        cancellation: CancellationToken,
    ) -> Result<super::metrics::LoadTestResults> {
        let scope = self.require_scope()?;
        let (plan, template) = self.build_plan()?;
        let transport = Arc::new(eggsec_transport_eggfetch::EggfetchTransport::new(Arc::new(
            eggsec_transport::SystemTransportResolver,
        )));
        let authority: Arc<dyn NetworkAuthority> =
            Arc::new(super::backend::OwnedScopeAuthority::new(scope));
        let executor = LoadTestExecutor::new(plan, template, transport, authority, cancellation);
        executor
            .run(&NoopSink)
            .await
            .map_err(EggsecError::Validation)
    }

    /// Run with a caller-supplied transport + authority + sink (tests and
    /// structured-progress consumers). The facade's stored scope is ignored;
    /// the caller owns authorization.
    pub async fn run_with<T, S>(
        &self,
        transport: Arc<T>,
        authority: Arc<dyn NetworkAuthority>,
        cancellation: CancellationToken,
        sink: &S,
    ) -> Result<super::metrics::LoadTestResults>
    where
        T: eggsec_transport::HttpTransport + 'static,
        S: super::progress::ProgressSink,
    {
        let (plan, template) = self.build_plan()?;
        let executor = LoadTestExecutor::new(plan, template, transport, authority, cancellation);
        executor.run(sink).await.map_err(EggsecError::Validation)
    }
}

impl Report for super::metrics::LoadTestResults {
    fn title(&self) -> &str {
        "Load Test Report"
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}
