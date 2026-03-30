use crate::error::Result;
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Method;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

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

        let results = Arc::new(Mutex::new(Vec::new()));

        for payload in payloads {
            let result = self.send_payload(&payload).await?;
            results.lock().await.push(result);
            if let Some(ref pb) = progress {
                pb.inc(1);
            }
        }

        if let Some(ref pb) = progress {
            pb.finish_and_clear();
        }
        let results = results.lock().await.clone();
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

        let results: Arc<Mutex<Vec<FuzzResult>>> =
            Arc::new(Mutex::new(Vec::with_capacity(payload_count)));
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.args.concurrency));
        let mut handles = Vec::new();

        for payload in payloads {
            let permit = semaphore.clone().acquire_owned().await?;
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

                if let Ok(r) = result {
                    results.lock().await.push(r);
                } else {
                    tracing::debug!("Fuzz request failed: {:?}", result.err());
                }

                if let Some(ref pb) = progress {
                    pb.inc(1);
                }
                drop(permit);
            });

            handles.push(handle);
        }

        join_all(handles).await;
        if let Some(ref pb) = progress {
            pb.finish_and_clear();
        }
        let results = results.lock().await.clone();
        Ok(results)
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

    pub(crate) async fn run_burst_with_session(&mut self, payloads: Vec<Payload>) -> Result<Vec<FuzzResult>> {
        let mut futures = Vec::new();
        for payload in &payloads {
            futures.push(self.send_fuzz_request(payload, Method::GET));
        }

        let results: Vec<Result<FuzzResult>> = join_all(futures).await;
        let results: Vec<FuzzResult> = results.into_iter().collect::<Result<Vec<_>>>()?;

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

        let limiter = AdaptiveRateLimiter::new(
            self.args.concurrency as u64,
            1,
            self.args.timeout * 1000,
        );

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

    pub(crate) async fn send_fuzz_request(&self, payload: &Payload, method: Method) -> Result<FuzzResult> {
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

                Ok(FuzzResult {
                    payload: payload.clone(),
                    status_code: status,
                    response_time_ms: elapsed.as_millis() as u64,
                    response_length: length,
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
