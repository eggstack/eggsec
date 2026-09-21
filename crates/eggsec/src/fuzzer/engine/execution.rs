use crate::error::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;

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
        self.run_concurrent_inner(payloads, mode_name).await
    }

    pub(crate) async fn run_burst_with_session(
        &mut self,
        payloads: Vec<Payload>,
    ) -> Result<Vec<FuzzResult>> {
        let results = self.run_concurrent_inner(payloads, "burst").await?;

        if self.args.session {
            self.update_session_from_results(&results).await;
        }

        Ok(results)
    }

    async fn run_concurrent_inner(
        &self,
        payloads: Vec<Payload>,
        mode_name: &str,
    ) -> Result<Vec<FuzzResult>> {
        let payload_count = payloads.len();
        // Fail closed rather than deadlock the admission loop below.
        // (Construction clamps concurrency to >= 1; this guards direct
        // struct mutation.)
        if self.args.concurrency == 0 {
            return Err(crate::error::EggsecError::Runtime(
                "concurrency must be greater than zero".to_string(),
            ));
        }

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

        // Phase B: bounded scheduler. At most `concurrency` payload futures
        // are in flight at any moment; a new item is admitted as one
        // completes. The original payload index travels with each completion
        // so output remains deterministically ordered. Peak live Tokio work
        // and retained handle state are O(concurrency), not O(payloads).
        let concurrency = self.args.concurrency;
        let mut results: Vec<Option<FuzzResult>> = (0..payload_count).map(|_| None).collect();
        let mut in_flight = tokio::task::JoinSet::new();
        let mut next: usize = 0;

        while next < payload_count || !in_flight.is_empty() {
            while next < payload_count && in_flight.len() < concurrency {
                let idx = next;
                next += 1;
                let payload = payloads[idx].clone();
                let client = self.client.clone();
                let url = self.args.url.clone();
                let method = self.args.method.clone();
                let param = self.args.param.clone();
                let timing_analyzer = self.timing_analyzer.clone();
                let pattern_matcher = self.pattern_matcher.clone();
                let user_agent = self.user_agent.clone();
                let auth_context_entry = self.auth_context_entry.clone();
                let payload_type = payload.payload_type.to_string();

                // Project invariant: every spawned tokio task carries a
                // timeout wrapper (30-300s).
                in_flight.spawn(async move {
                    let outcome = tokio::time::timeout(
                        std::time::Duration::from_secs(300),
                        send_payload_async(
                            client,
                            &url,
                            &method,
                            param.as_deref(),
                            &payload,
                            timing_analyzer,
                            pattern_matcher,
                            &user_agent,
                            auth_context_entry.as_ref(),
                        ),
                    )
                    .await;
                    let result = match outcome {
                        Ok(Ok(r)) => r,
                        Ok(Err(e)) => {
                            tracing::warn!("Fuzz request failed: {:?}", e);
                            FuzzResult {
                                payload: payload.clone(),
                                status_code: 0,
                                response_time_ms: 0,
                                response_length: None,
                                response_body: None,
                                is_waf_blocked: false,
                                is_anomaly: false,
                                is_redos_suspected: false,
                                leaks_found: vec![],
                                error: Some(e.to_string()),
                                owasp_category: Some(payload_type),
                                detected_severity: Severity::Info,
                            }
                        }
                        Err(_) => FuzzResult {
                            payload: payload.clone(),
                            status_code: 0,
                            response_time_ms: 0,
                            response_length: None,
                            response_body: None,
                            is_waf_blocked: false,
                            is_anomaly: false,
                            is_redos_suspected: false,
                            leaks_found: vec![],
                            error: Some("worker task failed or cancelled".to_string()),
                            owasp_category: Some(payload_type),
                            detected_severity: Severity::Info,
                        },
                    };
                    (idx, result)
                });
            }

            if let Some(join_result) = in_flight.join_next().await {
                match join_result {
                    Ok((idx, result)) => {
                        // Preserve payload order for deterministic output.
                        results[idx] = Some(result);
                    }
                    Err(e) => {
                        tracing::warn!("Fuzz worker task join failure: {}", e);
                    }
                }
                if let Some(ref pb) = progress {
                    pb.inc(1);
                }
            }
        }
        if let Some(ref pb) = progress {
            pb.finish_and_clear();
        }
        let mut final_results = Vec::with_capacity(payload_count);
        for (idx, slot) in results.into_iter().enumerate() {
            final_results.push(slot.unwrap_or_else(|| FuzzResult {
                payload: payloads[idx].clone(),
                status_code: 0,
                response_time_ms: 0,
                response_length: None,
                response_body: None,
                is_waf_blocked: false,
                is_anomaly: false,
                is_redos_suspected: false,
                leaks_found: vec![],
                error: Some("worker task failed or cancelled".to_string()),
                owasp_category: Some(payloads[idx].payload_type.to_string()),
                detected_severity: Severity::Info,
            }));
        }
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
            self.auth_context_entry.as_ref(),
        )
        .await
    }

    pub(crate) async fn run_sequential_with_session(
        &mut self,
        payloads: Vec<Payload>,
    ) -> Result<Vec<FuzzResult>> {
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

        let mut results = Vec::with_capacity(payloads.len());

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
            if rate < 1 {
                tracing::warn!("Adaptive rate limiter backed off to 0, stopping");
                break;
            }

            let result = self.send_payload(&payload).await;

            match result {
                Ok(r) => {
                    let is_error = r.error.is_some()
                        || r.status_code == 0
                        || r.status_code == crate::constants::STATUS_RATE_LIMITED
                        || r.status_code == crate::constants::STATUS_SERVER_ERROR;

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
}

#[cfg(test)]
mod tests {
    use crate::fuzzer::config::{FuzzConfig, FuzzMode};
    use crate::fuzzer::payloads::{Payload, PayloadType, Severity};
    use crate::types::CommonHttpArgs;

    fn make_fuzz_args(url: &str) -> FuzzConfig {
        FuzzConfig {
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
    async fn test_send_payload_engine_construction() {
        let args = make_fuzz_args("http://localhost:1");
        let engine = super::super::core::FuzzEngine::new(args);
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_run_sequential_with_session_uses_analyzed_payload_path() {
        let mut args = make_fuzz_args("http://localhost:1");
        args.session = true;
        let mut engine = super::super::core::FuzzEngine::new(args).unwrap();

        let payloads = vec![Payload {
            payload_type: PayloadType::Sqli,
            payload: "' OR 1=1--".to_string(),
            description: "test".to_string(),
            severity: Severity::Medium,
            tags: vec!["test".to_string()],
        }];

        let results = engine.run_sequential_with_session(payloads).await.unwrap();
        assert_eq!(results.len(), 1);
        // send_payload_async populates OWASP category from payload type even on request error.
        assert!(results[0].owasp_category.is_some());
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

    fn make_payloads(n: usize) -> Vec<Payload> {
        (0..n)
            .map(|i| Payload {
                payload_type: PayloadType::Sqli,
                payload: format!("bounded-payload-{i}"),
                description: format!("test {i}"),
                severity: Severity::Medium,
                tags: vec!["test".to_string()],
            })
            .collect()
    }

    /// Phase B: bounded scheduling must preserve deterministic payload/result
    /// association across completions arriving in any order.
    #[tokio::test]
    async fn test_run_concurrent_preserves_payload_order() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&server)
            .await;

        let mut args = make_fuzz_args(&server.uri());
        args.concurrency = 4;
        let engine = super::super::core::FuzzEngine::new_with_tui_mode(args, true).unwrap();
        let payloads = make_payloads(25);
        let results = engine
            .run_concurrent(payloads.clone(), "TEST")
            .await
            .unwrap();
        assert_eq!(results.len(), payloads.len());
        for (i, result) in results.iter().enumerate() {
            assert_eq!(
                result.payload.payload, payloads[i].payload,
                "result index {i} is not associated with input payload {i}"
            );
        }
        assert!(results.iter().all(|r| r.error.is_none()));
    }

    /// Phase B: worker failure must keep the same fallback/error
    /// representation as the pre-bounded scheduler (status 0, error text,
    /// OWASP category populated, deterministic order).
    #[tokio::test]
    async fn test_run_concurrent_error_fallback_shape() {
        let mut args = make_fuzz_args("http://127.0.0.1:1");
        args.concurrency = 8;
        let engine = super::super::core::FuzzEngine::new_with_tui_mode(args, true).unwrap();
        let payloads = make_payloads(5);
        let results = engine
            .run_concurrent(payloads.clone(), "TEST")
            .await
            .unwrap();
        assert_eq!(results.len(), payloads.len());
        for (i, result) in results.iter().enumerate() {
            assert_eq!(result.payload.payload, payloads[i].payload);
            assert_eq!(result.status_code, 0);
            assert!(result.error.is_some(), "missing error fallback at {i}");
            assert!(
                result.owasp_category.is_some(),
                "missing OWASP category at {i}"
            );
        }
    }
}
