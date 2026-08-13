use crate::error::{EggsecError, Result};
use crate::utils::stealth::tool_user_agent;
use base64::{engine::general_purpose, Engine as _};
use bytes::Bytes;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{Client, Method};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinSet;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing;

use super::metrics::{LoadTestResults, Metrics};
use crate::config::EggsecConfig;
use crate::output::report::Report;
use crate::types::CommonHttpArgs;

/// Plain load-test run configuration (no Clap derives).
///
/// This is the engine-facing contract used by the pipeline, Python bindings,
/// and tool/API consumers. CLI parsing converts `LoadArgs` into this type.
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
    method: Method,
    body: Option<Bytes>,
    headers: Vec<(String, String)>,
    insecure: bool,
    proxy: Option<String>,
    proxy_auth: Option<String>,
    user_agent: String,
    rate_limit: Option<u32>,
    tui_mode: bool,
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
        if concurrency == 0 {
            return Err(EggsecError::Validation(
                "Concurrency must be greater than 0".to_string(),
            ));
        }
        if total_requests == 0 {
            return Err(EggsecError::Validation(
                "Total requests must be greater than 0".to_string(),
            ));
        }
        if timeout.is_zero() {
            return Err(EggsecError::Validation(
                "Timeout must be greater than 0".to_string(),
            ));
        }

        Ok(Self {
            url,
            total_requests,
            concurrency,
            timeout,
            method: Method::GET,
            body: None,
            headers: Vec::new(),
            insecure: false,
            proxy: None,
            proxy_auth: None,
            user_agent: tool_user_agent(),
            rate_limit: None,
            tui_mode,
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
    /// TUI mode.
    pub fn from_config_with_mode(cfg: LoadTestRunConfig, tui_mode: bool) -> Result<Self> {
        let mut runner = Self::new_with_tui_mode(
            cfg.url,
            cfg.requests,
            cfg.concurrency,
            cfg.timeout,
            tui_mode,
        )?;

        runner.set_method(cfg.method);

        if let Some(body) = cfg.body {
            runner.set_body(body);
        }

        let headers = crate::utils::parse_headers(&cfg.headers);
        for (key, value) in headers {
            runner.add_header(key, value);
        }

        runner.set_common(cfg.common.clone());

        Ok(runner)
    }

    /// Construct a runner from a plain [`LoadTestRunConfig`] and an
    /// [`EggsecConfig`]. Honors config-derived defaults for headers, proxy,
    /// user-agent, and timeout fallbacks where the plain config leaves them
    /// unset.
    pub fn from_config_with_engine(cfg: LoadTestRunConfig, config: &EggsecConfig) -> Result<Self> {
        let timeout = if cfg.timeout.is_zero() {
            Duration::from_secs(config.http.timeout_secs)
        } else {
            cfg.timeout
        };

        let cfg = LoadTestRunConfig { timeout, ..cfg };

        let mut runner = Self::from_config_with_mode(cfg.clone(), false)?;
        runner.set_common_with_config(cfg.common, config);

        Ok(runner)
    }

    pub fn set_common(&mut self, common: CommonHttpArgs) {
        self.apply_common(
            common.insecure,
            common.proxy,
            common.proxy_auth,
            common.rate_limit,
            common.user_agent,
        );
        self.apply_auth_headers(common.auth, common.bearer, common.cookie, common.api_key);
    }

    pub fn set_common_with_config(&mut self, common: CommonHttpArgs, config: &EggsecConfig) {
        let insecure = common.insecure || !config.http.verify_tls;
        let proxy = common.proxy.or(config.http.proxy.clone());
        let proxy_auth = common.proxy_auth.or(config
            .http
            .proxy_auth
            .as_ref()
            .map(|s| s.expose_secret().to_string()));
        let effective_rate = common.rate_limit.or(config.scan.rate_limit_per_second);
        let user_agent = common
            .user_agent
            .or_else(|| config.http.default_user_agent.clone());

        self.apply_common(insecure, proxy, proxy_auth, effective_rate, user_agent);
        self.apply_auth_headers(common.auth, common.bearer, common.cookie, common.api_key);

        for (key, value) in &config.http.default_headers {
            self.add_header(key.clone(), value.clone());
        }
    }

    fn apply_common(
        &mut self,
        insecure: bool,
        proxy: Option<String>,
        proxy_auth: Option<String>,
        rate_limit: Option<u32>,
        user_agent: Option<String>,
    ) {
        self.insecure = insecure;
        self.proxy = proxy;
        self.proxy_auth = proxy_auth;

        if let Some(rate) = rate_limit {
            if rate == 0 {
                tracing::warn!("Rate limit of 0 is invalid, ignoring rate limit setting");
            } else if rate > 100_000 {
                tracing::warn!(
                    "Rate limit {} req/s exceeds recommended maximum of 100,000; \
                     rate limiting may be ineffective at this level",
                    rate
                );
                self.rate_limit = Some(rate);
            } else {
                self.rate_limit = Some(rate);
            }
        }

        if let Some(ua) = user_agent {
            self.user_agent = ua;
        }
    }

    fn apply_auth_headers(
        &mut self,
        auth: Option<String>,
        bearer: Option<String>,
        cookie: Option<String>,
        api_key: Option<String>,
    ) {
        if let Some(auth) = auth {
            let parts: Vec<&str> = auth.splitn(2, ':').collect();
            if parts.len() == 2 {
                let encoded =
                    general_purpose::STANDARD.encode(format!("{}:{}", parts[0], parts[1]));
                self.add_header("Authorization".to_string(), format!("Basic {}", encoded));
            } else {
                tracing::warn!(
                    "Invalid auth format (expected 'user:password'), ignoring basic auth"
                );
            }
        }

        if let Some(bearer) = bearer {
            self.add_header("Authorization".to_string(), format!("Bearer {}", bearer));
        }

        if let Some(cookie) = cookie {
            self.add_header("Cookie".to_string(), cookie);
        }

        if let Some(api_key) = api_key {
            if api_key.contains(':') {
                let parts: Vec<&str> = api_key.splitn(2, ':').collect();
                self.add_header(parts[0].to_string(), parts[1].to_string());
            } else {
                self.add_header("X-API-Key".to_string(), api_key);
            }
        }
    }

    pub fn set_method(&mut self, method: String) {
        self.method = match method.to_uppercase().as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "PUT" => Method::PUT,
            "DELETE" => Method::DELETE,
            "PATCH" => Method::PATCH,
            "HEAD" => Method::HEAD,
            "OPTIONS" => Method::OPTIONS,
            other => {
                tracing::warn!("Unknown HTTP method '{}', defaulting to GET", other);
                Method::GET
            }
        };
    }

    pub fn set_body(&mut self, body: String) {
        self.body = Some(Bytes::from(body));
    }

    pub fn add_header(&mut self, key: String, value: String) {
        self.headers.push((key, value));
    }

    pub async fn run(&self) -> Result<LoadTestResults> {
        if self.insecure {
            tracing::warn!(
                "TLS certificate verification disabled. This is insecure and should only \
                 be used in isolated testing environments."
            );
        }
        let mut client_builder = Client::builder()
            .timeout(self.timeout)
            .danger_accept_invalid_certs(self.insecure);

        if let Some(proxy_url) = &self.proxy {
            let mut proxy = reqwest::Proxy::all(proxy_url)?;
            if let Some(auth) = &self.proxy_auth {
                let parts: Vec<&str> = auth.splitn(2, ':').collect();
                if parts.len() == 2 {
                    proxy = proxy.basic_auth(parts[0], parts[1]);
                }
            }
            client_builder = client_builder.proxy(proxy);
        }

        let client = client_builder
            .build()
            .map_err(crate::error::EggsecError::from)?;

        let metrics = Arc::new(Mutex::new(Metrics::new(self.url.clone())));

        let progress = if self.tui_mode {
            None
        } else {
            let pb = Arc::new(ProgressBar::new(self.total_requests));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                    .unwrap_or_else(|_| ProgressStyle::default_bar())
                    .progress_chars("#>-"),
            );
            Some(pb)
        };

        let start = Instant::now();
        let issued_requests = Arc::new(AtomicU64::new(0));

        let cancellation_token = CancellationToken::new();

        let rate_limit_sem = self.rate_limit.map(|rate| {
            let sem = Arc::new(Semaphore::new(0));
            let min_interval = Duration::from_secs_f64(1.0 / f64::from(rate));
            let sem_clone = sem.clone();
            let token = cancellation_token.clone();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = sleep(min_interval) => {
                            sem_clone.add_permits(1);
                        }
                        _ = token.cancelled() => {
                            break;
                        }
                    }
                }
            });
            sem
        });

        let worker_count = self.concurrency.min(self.total_requests as usize);
        let mut workers = JoinSet::new();

        for _ in 0..worker_count {
            let client = client.clone();
            let url = self.url.clone();
            let method = self.method.clone();
            let body = self.body.clone();
            let headers = self.headers.clone();
            let metrics = metrics.clone();
            let progress = progress.clone();
            let user_agent = self.user_agent.clone();
            let issued_requests = issued_requests.clone();
            let rate_limit_sem = rate_limit_sem.clone();
            let total_requests = self.total_requests;
            let token = cancellation_token.clone();

            workers.spawn(async move {
                loop {
                    if token.is_cancelled() {
                        break;
                    }

                    let request_index = issued_requests.fetch_add(1, Ordering::Relaxed);
                    if request_index >= total_requests {
                        break;
                    }

                    if let Some(sem) = &rate_limit_sem {
                        match sem.acquire().await {
                            Ok(permit) => permit.forget(),
                            Err(e) => {
                                tracing::warn!("Rate limit semaphore closed: {} - continuing without rate limiting", e);
                            }
                        }
                    }

                    let request_start = Instant::now();

                    let mut req = client.request(method.clone(), &url);
                    req = req.header("User-Agent", &user_agent);

                    for (key, value) in &headers {
                        req = req.header(key, value);
                    }

                    if let Some(b) = &body {
                        req = req.body(b.clone());
                    }

                    let result = req.send().await;

                    match result {
                        Ok(response) => {
                            let latency = request_start.elapsed();
                            let status = response.status();
                            let status_code = status.as_u16();
                            // Always consume body to enable connection reuse
                            if let Err(e) = response.bytes().await {
                                tracing::trace!("Failed to drain response body: {}", e);
                            }
                            let mut metrics = metrics.lock().await;
                            metrics.record_http_response(latency, status_code);
                        }
                        Err(e) => {
                            let latency = request_start.elapsed();
                            let mut metrics = metrics.lock().await;
                            metrics.record_failure(e.to_string(), latency);
                        }
                    }

                    if let Some(ref pb) = progress {
                        pb.inc(1);
                    }
                }
            });
        }

        while let Some(join_result) = workers.join_next().await {
            match join_result {
                Ok(()) => {}
                Err(e) if e.is_panic() => {
                    tracing::error!("Load test worker panicked: {:?}", e);
                }
                Err(e) => tracing::error!("Load test worker failed: {}", e),
            }
        }

        cancellation_token.cancel();

        let total_duration = start.elapsed();
        if let Some(ref pb) = progress {
            pb.finish_and_clear();
        }

        let metrics = metrics.lock().await;
        Ok(metrics.to_results(total_duration))
    }
}

impl Report for LoadTestResults {
    fn title(&self) -> &str {
        "Load Test Report"
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}
