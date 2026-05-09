use crate::error::Result;
use dashmap::DashMap;
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Method;
use std::sync::Arc;
use std::time::Instant;

use crate::waf::types::Severity;

use super::super::payloads::Payload;
use super::types::FuzzResult;
use super::utils::send_payload_async;

use super::core::FuzzEngine;

impl FuzzEngine {
    pub(crate) async fn run_sequential(&self, payloads: Vec<Payload>) -> Result<Vec<FuzzResult>> {
        let progress = if self.tui_mode {
            None
        } else {
            let pb = Arc::new(ProgressBar::new(payloads.len() as u64));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} payloads ({eta})")
                    .unwrap_or_else(|_| ProgressStyle::default_bar())
                    .progress_chars("#>-"),
            );
            Some(pb)
        };

        let mut results = Vec::new();

        for payload in payloads {
            let result = self.send_payload(&payload).await?;
            results.push(result);
            if let Some(ref pb) = progress {
                pb.inc(1);
            }
        }

        if let Some(ref pb) = progress {
            pb.finish_and_clear();
        }
        Ok(results)
    }

    pub(crate) async fn run_burst(&self, payloads: Vec<Payload>) -> Result<Vec<FuzzResult>> {
        self.run_concurrent(payloads, "BURST MODE").await
    }

    pub(crate) async fn run_adaptive(&self, payloads: Vec<Payload>) -> Result<Vec<FuzzResult>> {
        self.run_concurrent(payloads, "ADAPTIVE MODE").await
    }

    pub(crate) async fn run_concurrent(
        &self,
        payloads: Vec<Payload>,
        mode_name: &str,
    ) -> Result<Vec<FuzzResult>> {
        self.run_concurrent_inner(payloads, mode_name, false).await
    }

    pub(crate) async fn run_burst_with_session(
        &mut self,
        payloads: Vec<Payload>,
    ) -> Result<Vec<FuzzResult>> {
        let results = self.run_concurrent_inner(payloads, "burst", true).await?;

        if self.args.session {
            self.update_session_from_results(&results).await;
        }

        Ok(results)
    }

    async fn run_concurrent_inner(
        &self,
        payloads: Vec<Payload>,
        mode_name: &str,
        _update_session: bool,
    ) -> Result<Vec<FuzzResult>> {
        let payload_count = payloads.len();

        let progress = if self.tui_mode {
            None
        } else {
            let pb = Arc::new(ProgressBar::new(payload_count as u64));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(&format!("{{spinner:.green}} [{{elapsed_precise}}] [{{bar:40.cyan/blue}}] {{pos}}/{{len}} - {}", mode_name))
                    .unwrap_or_else(|_| ProgressStyle::default_bar())
                    .progress_chars("#>-"),
            );
            Some(pb)
        };

        let results: Arc<DashMap<usize, FuzzResult>> = Arc::new(DashMap::new());
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.args.concurrency));
        let mut handles = Vec::new();

        for (idx, payload) in payloads.into_iter().enumerate() {
            let semaphore = semaphore.clone();
            let client = self.client.clone();
            let url = self.args.url.clone();
            let method = self.args.method.clone();
            let param = self.args.param.clone();
            let timing_analyzer = self.timing_analyzer.clone();
            let pattern_matcher = self.pattern_matcher.clone();
            let results = results.clone();
            let progress = progress.clone();
            let payload_clone = payload.clone();
            let user_agent = self.user_agent.clone();

            let handle = tokio::spawn(async move {
                let Ok(_permit) = semaphore.acquire_owned().await else {
                    tracing::warn!("Semaphore closed before request dispatch");
                    return;
                };
                let result = send_payload_async(
                    client,
                    &url,
                    &method,
                    param.as_deref(),
                    &payload_clone,
                    timing_analyzer,
                    pattern_matcher,
                    &user_agent,
                )
                .await;

                match result {
                    Ok(r) => {
                        // Preserve payload order for deterministic output.
                        results.insert(idx, r);
                    }
                    Err(e) => {
                        tracing::warn!("Fuzz request failed: {:?}", e);
                        results.insert(
                            idx,
                            FuzzResult {
                                payload: payload_clone.clone(),
                                status_code: 0,
                                response_time_ms: 0,
                                response_length: None,
                                response_body: None,
                                is_waf_blocked: false,
                                is_anomaly: false,
                                is_redos_suspected: false,
                                leaks_found: vec![],
                                error: Some(e.to_string()),
                                owasp_category: Some(payload_clone.payload_type.to_string()),
                                detected_severity: Severity::Info,
                            },
                        );
                    }
                }

                if let Some(ref pb) = progress {
                    pb.inc(1);
                }
            });

            handles.push(handle);
        }

        join_all(handles).await;
        if let Some(ref pb) = progress {
            pb.finish_and_clear();
        }
        let mut ordered_results: Vec<(usize, FuzzResult)> = Arc::try_unwrap(results)
            .expect("all workers completed")
            .into_iter()
            .collect();
        ordered_results.sort_by_key(|(k, _)| *k);
        let final_results: Vec<FuzzResult> = ordered_results.into_iter().map(|(_, v)| v).collect();
        Ok(final_results)
    }

    pub(crate) async fn send_payload(&self, payload: &Payload) -> Result<FuzzResult> {
        send_payload_async(
            self.client.clone(),
            &self.args.url,
            &self.args.method,
            self.args.param.as_deref(),
            payload,
            self.timing_analyzer.clone(),
            self.pattern_matcher.clone(),
            &self.user_agent,
        )
        .await
    }

    pub(crate) async fn run_sequential_with_session(
        &mut self,
        payloads: Vec<Payload>,
    ) -> Result<Vec<FuzzResult>> {
        let mut results = Vec::with_capacity(payloads.len());

        for payload in payloads {
            let result = self.send_fuzz_request(&payload, Method::GET).await?;
            results.push(result);
        }

        if self.args.session {
            self.update_session_from_results(&results).await;
        }

        Ok(results)
    }

    pub(crate) async fn run_adaptive_with_session(
        &mut self,
        payloads: Vec<Payload>,
    ) -> Result<Vec<FuzzResult>> {
        use super::super::rate_limit::AdaptiveRateLimiter;

        let limiter =
            AdaptiveRateLimiter::new(self.args.concurrency as u64, 1, self.args.timeout * 1000);

        let mut results = Vec::with_capacity(payloads.len());

        for payload in payloads {
            let rate = limiter.get_rate();
            if rate == 0 {
                tracing::warn!("Adaptive rate limiter backed off to 0, stopping");
                break;
            }

            let result = self.send_fuzz_request(&payload, Method::GET).await;

            match result {
                Ok(r) => {
                    let is_error = r.error.is_some()
                        || r.status_code == 0
                        || r.status_code == 429
                        || r.status_code == 503;

                    if is_error {
                        limiter.record_error(Some(r.status_code));
                    } else {
                        limiter.record_success();
                    }
                    results.push(r);
                }
                Err(e) => {
                    limiter.record_timeout();
                    tracing::debug!("Adaptive fuzz request failed: {e}");
                }
            }
        }

        if self.args.session {
            self.update_session_from_results(&results).await;
        }

        Ok(results)
    }

    pub(crate) async fn send_fuzz_request(
        &self,
        payload: &Payload,
        method: Method,
    ) -> Result<FuzzResult> {
        let url = self.build_fuzz_url(&payload.payload);

        let start = Instant::now();
        let mut request = self
            .client
            .request(method.clone(), &url)
            .header("User-Agent", &self.user_agent);

        if self.args.session {
            if let Some(ref session) = self.http_session {
                for (name, cookie) in &session.cookies {
                    request = request.header("Cookie", format!("{}={}", name, cookie.value));
                }
            }
        }

        if let Some(ref bearer) = self.args.common.bearer {
            request = request.bearer_auth(bearer);
        }

        let response = request.send().await;
        let elapsed = start.elapsed();

        match response {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let length = resp.content_length();
                let body = resp.text().await.ok();

                Ok(FuzzResult {
                    payload: payload.clone(),
                    status_code: status,
                    response_time_ms: elapsed.as_millis() as u64,
                    response_length: length,
                    response_body: body,
                    is_waf_blocked: false,
                    is_anomaly: false,
                    is_redos_suspected: false,
                    leaks_found: vec![],
                    error: None,
                    owasp_category: None,
                    detected_severity: payload.severity,
                })
            }
            Err(e) => Ok(FuzzResult {
                payload: payload.clone(),
                status_code: 0,
                response_time_ms: elapsed.as_millis() as u64,
                response_length: None,
                response_body: None,
                is_waf_blocked: false,
                is_anomaly: false,
                is_redos_suspected: false,
                leaks_found: vec![],
                error: Some(e.to_string()),
                owasp_category: None,
                detected_severity: Severity::Info,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::{CommonHttpArgs, FuzzArgs, FuzzMode};

    fn make_fuzz_args(url: &str) -> FuzzArgs {
        FuzzArgs {
            url: url.to_string(),
            payload_type: "sqli".to_string(),
            common: CommonHttpArgs::default(),
            method: "GET".to_string(),
            param: None,
            concurrency: 10,
            timeout: 5,
            verbose: false,
            quiet: false,
            json: false,
            output: None,
            mutate: false,
            mutation_count: 5,
            grammar_fuzz: false,
            grammar_type: None,
            session: false,
            diffing: false,
            capture_baseline: false,
            mode: FuzzMode::Sequential,
            target: None,
            graphql_introspection: false,
            graphql_depth_bypass: false,
            graphql_alias_overload: false,
            jwt_token: None,
            oauth_client_id: None,
            oauth_client_secret: None,
            oauth_redirect: false,
            oauth_scope: false,
            oauth_state: false,
            oauth_grant: false,
            oauth_issuer: None,
            idor_base_id: None,
            idor_user_ids: None,
            ssti_param: None,
            adaptive_rate: false,
            enhanced_redos: false,
            waf_fingerprint: false,
            chaining: false,
            chain_file: None,
            format: None,
            schema: None,
            discover_only: false,
            auto_discover_schema: false,
            calibrate: false,
            fc: None,
            fs: None,
            fw: None,
            fl: None,
            ft: None,
            fr: None,
        }
    }

    #[test]
    fn test_fuzz_engine_execution_construction() {
        let args = make_fuzz_args("http://example.com");
        let engine = super::super::core::FuzzEngine::new(args);
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_send_fuzz_request_engine_construction() {
        let args = make_fuzz_args("http://localhost:1");
        let engine = super::super::core::FuzzEngine::new(args);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_build_fuzz_url_with_param() {
        let mut args = make_fuzz_args("http://example.com");
        args.param = Some("q".to_string());
        let engine = super::super::core::FuzzEngine::new(args).unwrap();
        let url = engine.build_fuzz_url("test' OR 1=1--");
        assert!(url.contains("q="));
        assert!(url.contains("test"));
    }

    #[test]
    fn test_build_fuzz_url_without_param() {
        let args = make_fuzz_args("http://example.com/api");
        let engine = super::super::core::FuzzEngine::new(args).unwrap();
        let url = engine.build_fuzz_url("test");
        assert_eq!(url, "http://example.com/api");
    }

    #[test]
    fn test_build_fuzz_url_with_existing_query() {
        let mut args = make_fuzz_args("http://example.com/api?foo=bar");
        args.param = Some("q".to_string());
        let engine = super::super::core::FuzzEngine::new(args).unwrap();
        let url = engine.build_fuzz_url("test");
        assert!(url.contains("foo=bar"));
        assert!(url.contains("q=test"));
    }
}
