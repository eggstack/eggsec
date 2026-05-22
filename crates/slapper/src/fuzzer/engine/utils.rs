use crate::error::Result;
use reqwest::{Client, Method};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::utils::urlencoding;
use crate::waf::types::Severity;

use super::super::detection::{PatternMatcher, TimingAnalyzer};
use super::super::mutator::generate_mutations;
use super::super::payloads::Payload;

use super::types::{BaselineResponse, FuzzResult, FuzzSession, OwaspSummary};

use super::core::FuzzEngine;

impl FuzzEngine {
    pub(crate) fn mutate_payloads(&self, payloads: &[Payload]) -> Vec<Payload> {
        let mut mutated = Vec::new();

        for payload in payloads {
            mutated.push(payload.clone());

            let mutations = generate_mutations(&payload.payload, self.args.mutation_count);
            for mutated_payload in mutations {
                if mutated_payload != payload.payload {
                    mutated.push(Payload {
                        payload_type: payload.payload_type,
                        payload: mutated_payload,
                        description: format!("{} (mutated)", payload.description),
                        severity: payload.severity,
                        tags: payload
                            .tags
                            .iter()
                            .cloned()
                            .chain(std::iter::once("mutated".to_string()))
                            .collect(),
                    });
                }
            }
        }

        mutated
    }

    pub(crate) fn build_session(
        &self,
        results: Vec<FuzzResult>,
        duration: Duration,
        baseline: Option<BaselineResponse>,
    ) -> FuzzSession {
        let successful = results.iter().filter(|r| r.error.is_none()).count();
        let failed = results.len() - successful;
        // A bypass means a finding was observed without being blocked by WAF controls.
        let waf_bypasses = results
            .iter()
            .filter(|r| r.is_vulnerable() && !r.is_waf_blocked)
            .count();
        let leaks = results.iter().filter(|r| !r.leaks_found.is_empty()).count();
        let anomalies = results.iter().filter(|r| r.is_anomaly).count();
        let redos = results.iter().filter(|r| r.is_redos_suspected).count();
        let findings = results.iter().filter(|r| r.is_vulnerable()).count();
        let owasp_summary = OwaspSummary::from_results(&results);

        FuzzSession {
            target_url: self.args.url.clone(),
            mode: format!("{:?}", self.args.mode),
            payload_type: self.args.payload_type.clone(),
            total_payloads: results.len(),
            successful_requests: successful,
            failed_requests: failed,
            waf_bypasses,
            potential_leaks: leaks,
            time_anomalies: anomalies,
            redos_suspected: redos,
            duration_ms: duration.as_millis() as u64,
            total_requests: results.len(),
            findings,
            results,
            owasp_summary,
            baseline,
        }
    }

    pub(crate) async fn capture_baseline_for_diffing(&mut self) -> Result<()> {
        if let Some(ref mut differ) = self.differ {
            let start = Instant::now();
            let response = self
                .client
                .get(&self.args.url)
                .header("User-Agent", &self.user_agent)
                .send()
                .await?;

            let status_code = response.status().as_u16();
            let headers = response.headers().clone();
            let body = response.bytes().await;
            let body = match body {
                Ok(b) => b.to_vec(),
                Err(e) => {
                    tracing::debug!("Failed to read baseline response body: {}", e);
                    vec![]
                }
            };
            let timing_ms = start.elapsed().as_millis() as u64;

            differ.capture_baseline(status_code, &headers, &body, timing_ms);
        }
        Ok(())
    }

    pub(crate) async fn apply_diffing(
        &mut self,
        results: Vec<FuzzResult>,
    ) -> Result<Vec<FuzzResult>> {
        if let Some(ref mut differ) = self.differ {
            let mut diffed_results = Vec::new();

            for result in results {
                let mut updated_result = result.clone();

                let start = Instant::now();
                let url = build_url(
                    &self.args.url,
                    self.args.param.as_deref(),
                    &updated_result.payload.payload,
                )?;
                let method = parse_method(&self.args.method);

                let mut request = self.client.request(method, url);
                request = request.header("User-Agent", &self.user_agent);
                if let Some(ref bearer) = self.args.common.bearer {
                    request = request.bearer_auth(bearer);
                }

                if let Ok(resp) = request.send().await {
                    let status_code = resp.status().as_u16();
                    let headers = resp.headers().clone();
                    let body = match resp.bytes().await {
                        Ok(b) => b.to_vec(),
                        Err(e) => {
                            tracing::debug!("Failed to read diffing response body: {}", e);
                            vec![]
                        }
                    };
                    let timing_ms = start.elapsed().as_millis() as u64;

                    let diff = differ.diff(status_code, &headers, &body, timing_ms);

                    if diff.diff.status_changed {
                        updated_result.is_anomaly = true;
                        updated_result
                            .leaks_found
                            .push(format!("Status changed: {}", diff.diff.status_changed));
                    }
                    if diff.diff.body_length_diff.abs()
                        > crate::constants::waf::LENGTH_DIFF_THRESHOLD as isize
                    {
                        updated_result.is_anomaly = true;
                        updated_result
                            .leaks_found
                            .push(format!("Body length diff: {}", diff.diff.body_length_diff));
                    }
                }

                diffed_results.push(updated_result);
            }

            Ok(diffed_results)
        } else {
            Ok(results)
        }
    }

    pub(crate) fn build_fuzz_url(&self, payload: &str) -> String {
        let url = &self.args.url;
        if let Some(param) = &self.args.param {
            if url.contains('?') {
                format!("{}&{}={}", url, param, urlencoding::encode(payload))
            } else {
                format!("{}?{}={}", url, param, urlencoding::encode(payload))
            }
        } else {
            url.clone()
        }
    }

    pub(crate) async fn update_session_from_results(&mut self, results: &[FuzzResult]) {
        if let Some(ref mut session) = self.http_session {
            for result in results {
                if result.status_code == 200 || result.status_code == 302 {
                    for leak in &result.leaks_found {
                        if leak.contains("session")
                            || leak.contains("token")
                            || leak.contains("auth")
                        {
                            session
                                .state_data
                                .insert("auth_detected".to_string(), leak.clone());
                        }
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn send_payload_async(
    client: Client,
    base_url: &str,
    method: &str,
    param: Option<&str>,
    payload: &Payload,
    timing_analyzer: Arc<Mutex<TimingAnalyzer>>,
    pattern_matcher: PatternMatcher,
    user_agent: &str,
) -> Result<FuzzResult> {
    let url = build_url(base_url, param, &payload.payload)?;
    let http_method = parse_method(method);

    let start = Instant::now();

    let response = client
        .request(http_method, url.clone())
        .header("User-Agent", user_agent)
        .send()
        .await;

    let response_time = start.elapsed();
    let mut timing: tokio::sync::MutexGuard<'_, TimingAnalyzer> = timing_analyzer.lock().await;
    let timing_result = timing.record(response_time);

    match response {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let content_length = resp.content_length();

            let body = match resp.text().await {
                Ok(text) => text,
                Err(e) => {
                    tracing::debug!("Failed to read response body for {}: {}", url, e);
                    String::new()
                }
            };
            let leaks = pattern_matcher.scan(&body);

            let is_waf_blocked = status == 403 || status == 406 || status == 429;

            let owasp_str = payload.payload_type.to_string();
            let detected_severity = compute_severity(
                &payload.severity,
                is_waf_blocked,
                timing_result.is_redos_suspected,
                !leaks.is_empty(),
            );

            Ok(FuzzResult {
                payload: payload.clone(),
                status_code: status,
                response_time_ms: timing_result.response_time_ms,
                response_length: content_length,
                response_body: Some(body),
                is_waf_blocked,
                is_anomaly: timing_result.is_anomaly,
                is_redos_suspected: timing_result.is_redos_suspected,
                leaks_found: leaks
                    .iter()
                    .map(|l| format!("{}: {}", l.category, l.pattern))
                    .collect(),
                error: None,
                owasp_category: Some(owasp_str),
                detected_severity,
            })
        }
        Err(e) => {
            let owasp_str = payload.payload_type.to_string();
            Ok(FuzzResult {
                payload: payload.clone(),
                status_code: 0,
                response_time_ms: timing_result.response_time_ms,
                response_length: None,
                response_body: None,
                is_waf_blocked: false,
                is_anomaly: timing_result.is_anomaly,
                is_redos_suspected: timing_result.is_redos_suspected,
                leaks_found: Vec::new(),
                error: Some(e.to_string()),
                owasp_category: Some(owasp_str),
                detected_severity: Severity::Info,
            })
        }
    }
}

fn build_url(base_url: &str, param: Option<&str>, payload: &str) -> Result<url::Url> {
    let mut url = url::Url::parse(base_url)?;

    if let Some(p) = param {
        {
            let mut query_pairs = url.query_pairs_mut();
            query_pairs.append_pair(p, payload);
        }
    } else {
        let path = url.path();
        let new_path = if path.ends_with('/') {
            format!("{}{}", path, urlencoding::encode(payload))
        } else {
            format!("{}/{}", path, urlencoding::encode(payload))
        };
        url.set_path(&new_path);
    }

    Ok(url)
}

fn parse_method(method: &str) -> Method {
    match method.to_uppercase().as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        "PATCH" => Method::PATCH,
        "HEAD" => Method::HEAD,
        "OPTIONS" => Method::OPTIONS,
        _ => Method::GET,
    }
}

fn compute_severity(
    base_severity: &Severity,
    is_waf_blocked: bool,
    is_redos: bool,
    has_leak: bool,
) -> Severity {
    if is_redos || (is_waf_blocked && has_leak) {
        Severity::Critical
    } else if has_leak {
        Severity::High
    } else if is_waf_blocked {
        Severity::Medium
    } else {
        *base_severity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{CommonHttpArgs, FuzzArgs, FuzzMode};
    use crate::fuzzer::payloads::{Payload, PayloadType};
    use std::time::Duration;

    fn make_test_engine() -> FuzzEngine {
        let args = FuzzArgs {
            url: "http://example.com".to_string(),
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
        };
        FuzzEngine::new(args).expect("engine should construct")
    }

    fn make_result(is_vulnerable: bool, is_waf_blocked: bool) -> FuzzResult {
        FuzzResult {
            payload: Payload {
                payload_type: PayloadType::Sqli,
                payload: "test".to_string(),
                description: "test".to_string(),
                severity: Severity::Low,
                tags: vec![],
            },
            status_code: 200,
            response_time_ms: 10,
            response_length: Some(1),
            response_body: Some("ok".to_string()),
            is_waf_blocked,
            is_anomaly: is_vulnerable,
            is_redos_suspected: false,
            leaks_found: vec![],
            error: None,
            owasp_category: Some("sqli".to_string()),
            detected_severity: Severity::Low,
        }
    }

    #[test]
    fn test_parse_method_get() {
        assert_eq!(parse_method("GET"), Method::GET);
        assert_eq!(parse_method("get"), Method::GET);
    }

    #[test]
    fn test_parse_method_post() {
        assert_eq!(parse_method("POST"), Method::POST);
        assert_eq!(parse_method("post"), Method::POST);
    }

    #[test]
    fn test_parse_method_put() {
        assert_eq!(parse_method("PUT"), Method::PUT);
    }

    #[test]
    fn test_parse_method_delete() {
        assert_eq!(parse_method("DELETE"), Method::DELETE);
    }

    #[test]
    fn test_parse_method_patch() {
        assert_eq!(parse_method("PATCH"), Method::PATCH);
    }

    #[test]
    fn test_parse_method_head() {
        assert_eq!(parse_method("HEAD"), Method::HEAD);
    }

    #[test]
    fn test_parse_method_options() {
        assert_eq!(parse_method("OPTIONS"), Method::OPTIONS);
    }

    #[test]
    fn test_parse_method_unknown_defaults_to_get() {
        assert_eq!(parse_method("UNKNOWN"), Method::GET);
        assert_eq!(parse_method(""), Method::GET);
    }

    #[test]
    fn test_compute_severity_redos_is_critical() {
        assert_eq!(
            compute_severity(&Severity::Info, false, true, false),
            Severity::Critical
        );
    }

    #[test]
    fn test_compute_severity_waf_and_leak_is_critical() {
        assert_eq!(
            compute_severity(&Severity::Low, true, false, true),
            Severity::Critical
        );
    }

    #[test]
    fn test_compute_severity_leak_only_is_high() {
        assert_eq!(
            compute_severity(&Severity::Info, false, false, true),
            Severity::High
        );
    }

    #[test]
    fn test_compute_severity_waf_only_is_medium() {
        assert_eq!(
            compute_severity(&Severity::Low, true, false, false),
            Severity::Medium
        );
    }

    #[test]
    fn test_compute_severity_no_flags_returns_base() {
        assert_eq!(
            compute_severity(&Severity::Low, false, false, false),
            Severity::Low
        );
        assert_eq!(
            compute_severity(&Severity::High, false, false, false),
            Severity::High
        );
    }

    #[test]
    fn test_build_url_with_param() {
        let result = build_url("http://example.com", Some("q"), "test' OR 1=1--");
        assert!(result.is_ok());
        let url = result.unwrap();
        assert!(url.to_string().contains("q="));
    }

    #[test]
    fn test_build_url_without_param_appends_to_path() {
        let result = build_url("http://example.com/api", None, "test");
        assert!(result.is_ok());
        let url = result.unwrap();
        assert!(url.path().contains("/api/"));
    }

    #[test]
    fn test_build_url_with_trailing_slash() {
        let result = build_url("http://example.com/api/", None, "test");
        assert!(result.is_ok());
        let url = result.unwrap();
        assert!(url.path().ends_with("/test"));
    }

    #[test]
    fn test_build_url_invalid_base() {
        let result = build_url("not-a-url", None, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_build_session_waf_bypasses_counts_unblocked_findings_only() {
        let engine = make_test_engine();
        let results = vec![
            make_result(true, false),  // vulnerable and unblocked => bypass
            make_result(true, true),   // vulnerable but blocked => not a bypass
            make_result(false, false), // not vulnerable => not a bypass
        ];

        let session = engine.build_session(results, Duration::from_millis(5), None);
        assert_eq!(session.waf_bypasses, 1);
        assert_eq!(session.findings, 2);
    }
}
