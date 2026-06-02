use crate::error::{Result, SlapperError};
use crate::utils::stealth::tool_user_agent;
use base64::{engine::general_purpose, Engine as _};
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
use crate::cli::{CommonHttpArgs, LoadArgs};
use crate::config::SlapperConfig;
use crate::output::report::Report;

pub struct LoadTestRunner {
    url: String,
    total_requests: u64,
    concurrency: usize,
    timeout: Duration,
    method: Method,
    body: Option<String>,
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
            return Err(SlapperError::Validation(
                "Concurrency must be greater than 0".to_string(),
            ));
        }
        if total_requests == 0 {
            return Err(SlapperError::Validation(
                "Total requests must be greater than 0".to_string(),
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

    pub fn from_args(args: LoadArgs) -> Result<Self> {
        Self::from_args_with_tui_mode(args, false)
    }

    pub fn from_args_with_tui_mode(args: LoadArgs, tui_mode: bool) -> Result<Self> {
        let mut runner = Self::new_with_tui_mode(
            args.url,
            args.requests,
            args.concurrency,
            Duration::from_secs(args.timeout),
            tui_mode,
        )?;

        runner.set_method(args.method.clone());

        if let Some(body) = args.body {
            runner.set_body(body);
        }

        let headers = crate::utils::parse_headers(&args.headers);
        for (key, value) in headers {
            runner.add_header(key, value);
        }

        runner.set_common(args.common);

        Ok(runner)
    }

    pub fn from_args_with_config(args: LoadArgs, config: &SlapperConfig) -> Result<Self> {
        let timeout = if args.timeout == 30 {
            Duration::from_secs(config.http.timeout_secs)
        } else {
            Duration::from_secs(args.timeout)
        };

        let mut runner =
            Self::new_with_tui_mode(args.url, args.requests, args.concurrency, timeout, false)?;

        runner.set_method(args.method.clone());

        if let Some(body) = args.body {
            runner.set_body(body);
        }

        let headers = crate::utils::parse_headers(&args.headers);
        for (key, value) in headers {
            runner.add_header(key, value);
        }

        runner.set_common_with_config(args.common, config);

        Ok(runner)
    }

    pub fn set_common(&mut self, common: CommonHttpArgs) {
        self.insecure = common.insecure;
        self.proxy = common.proxy;
        self.proxy_auth = common.proxy_auth;
        self.rate_limit = common.rate_limit;

        if let Some(ua) = common.user_agent {
            self.user_agent = ua;
        }

        self.apply_auth_headers(common.auth, common.bearer, common.cookie, common.api_key);
    }

    pub fn set_common_with_config(&mut self, common: CommonHttpArgs, config: &SlapperConfig) {
        self.insecure = common.insecure || !config.http.verify_tls;
        self.proxy = common.proxy.or(config.http.proxy.clone());
        self.proxy_auth = common.proxy_auth.or(config
            .http
            .proxy_auth
            .as_ref()
            .map(|s| s.expose_secret().to_string()));
        self.rate_limit = common.rate_limit.or(config.scan.rate_limit_per_second);

        if let Some(ua) = common.user_agent {
            self.user_agent = ua;
        } else if let Some(ref ua) = config.http.default_user_agent {
            self.user_agent = ua.clone();
        }

        self.apply_auth_headers(common.auth, common.bearer, common.cookie, common.api_key);

        for (key, value) in &config.http.default_headers {
            self.add_header(key.clone(), value.clone());
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
            _ => Method::GET,
        };
    }

    pub fn set_body(&mut self, body: String) {
        self.body = Some(body);
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

        let client = client_builder.build().map_err(|e| {
            crate::error::SlapperError::from(e).with_timeout(self.timeout.as_millis() as u64)
        })?;

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

        let rate_limit_sem = self.rate_limit.map(|rate| {
            let sem = Arc::new(Semaphore::new(rate as usize));
            let min_interval = Duration::from_secs_f64(1.0 / f64::from(rate));
            let sem_clone = sem.clone();
            tokio::spawn(async move {
                loop {
                    sleep(min_interval).await;
                    sem_clone.add_permits(1);
                }
            });
            sem
        });

        let cancellation_token = CancellationToken::new();

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
                        if let Err(e) = sem.acquire().await {
                            tracing::warn!("Failed to acquire rate limit semaphore: {} - continuing without rate limiting", e);
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
                    let latency = request_start.elapsed();

                    match result {
                        Ok(response) => {
                            let status = response.status();
                            let status_code = status.as_u16();
                            if status_code >= 400 {
                                let _ = response.bytes().await;
                            }
                            let mut metrics = metrics.lock().await;
                            metrics.record_http_response(latency, status_code);
                        }
                        Err(e) => {
                            let mut metrics = metrics.lock().await;
                            metrics.record_failure(e.to_string());
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
        workers.abort_all();

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
