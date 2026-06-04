use crate::error::Result;
use dashmap::DashMap;
use futures::future::join_all;
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
        let mut payload_catalog = Vec::with_capacity(payload_count);

        for (idx, payload) in payloads.into_iter().enumerate() {
            payload_catalog.push(payload.clone());
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
            let auth_context_entry = self.auth_context_entry.clone();

            let handle = tokio::spawn(async move {
                let Ok(_permit) = semaphore.acquire_owned().await else {
                    tracing::warn!("Semaphore closed before request dispatch");
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
                            error: Some("semaphore closed before request dispatch".to_string()),
                            owasp_category: Some(payload_clone.payload_type.to_string()),
                            detected_severity: Severity::Info,
                        },
                    );
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
                    auth_context_entry.as_ref(),
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

        for join_result in join_all(handles).await {
            if let Err(e) = join_result {
                tracing::warn!("Fuzz worker task join failure: {}", e);
            }
        }
        if let Some(ref pb) = progress {
            pb.finish_and_clear();
        }
        for (idx, payload) in payload_catalog.into_iter().enumerate() {
            results.entry(idx).or_insert_with(|| FuzzResult {
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
                owasp_category: Some(payload.payload_type.to_string()),
                detected_severity: Severity::Info,
            });
        }
        let mut ordered_results: Vec<(usize, FuzzResult)> = match Arc::try_unwrap(results) {
            Ok(map) => map.into_iter().collect(),
            Err(_) => {
                tracing::error!("Failed to unwrap results - workers still holding references");
                return Err(crate::error::SlapperError::Runtime(
                    "Fuzz engine state inconsistent: workers still running".into(),
                ));
            }
        };
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
    use crate::cli::{CommonHttpArgs, FuzzArgs, FuzzMode};
    use crate::fuzzer::payloads::{Payload, PayloadType, Severity};

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
}
