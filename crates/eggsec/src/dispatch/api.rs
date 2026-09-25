use std::time::Duration;

#[cfg(feature = "nse")]
use crate::dispatch::types::NseResults;
use crate::dispatch::types::{send_progress, GraphQlResults, OAuthResults, TaskResult};

#[allow(clippy::too_many_arguments)]
pub async fn run_graphql(
    url: String,
    do_introspection: bool,
    do_injection: bool,
    do_depth_bypass: bool,
    do_alias_overload: bool,
    _concurrency: usize,
    timeout: u64,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult> {
    use crate::fuzzer::payloads::graphql::{
        GraphQLFuzzer, GraphQLTestResult, GraphQLVulnerability,
    };
    use std::time::Instant;

    let op_timeout = Duration::from_secs(300);
    tokio::time::timeout(op_timeout, async {
        let start = Instant::now();
        send_progress(&progress_tx, 0, 100).await;

        let client = crate::utils::get_shared_insecure_http_client();

        let mut fuzzer = GraphQLFuzzer::new(url.clone())
            .with_introspection(do_introspection)
            .with_depth_bypass(do_depth_bypass)
            .with_alias_overload(do_alias_overload);

        let mut total_requests = 0usize;
        let mut errors = 0usize;
        let mut introspection_enabled = false;
        let mut depth_limit_bypassed = false;
        let mut alias_overload_vulnerable = false;
        let mut injection_findings = Vec::new();

        if do_introspection {
            match fuzzer.run_introspection(&client).await {
                Ok(success) => {
                    introspection_enabled = success;
                    total_requests += 1;
                    if success {
                        let test_results = fuzzer.test_introspection_enabled();
                        for tr in test_results {
                            if tr.success {
                                let finding = format!(
                                    "[{}] Introspection {}: {}",
                                    tr.severity, tr.vulnerability, tr.description
                                );
                                injection_findings.push(finding);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("GraphQL introspection request failed: {}", e);
                    errors += 1;
                }
            }
        }

        send_progress(&progress_tx, 25, 100).await;

        let test_queries = fuzzer.generate_injection_queries(do_depth_bypass, do_alias_overload);
        let batch_queries = fuzzer.generate_batch_queries(do_alias_overload);
        let batch_count = batch_queries.len();
        let all_queries: Vec<(String, GraphQLTestResult)> = test_queries
            .into_iter()
            .map(|tr| (tr.query.clone(), tr))
            .chain(batch_queries.into_iter().map(|tr| (tr.query.clone(), tr)))
            .collect();

        if do_injection && !all_queries.is_empty() {
            let total_queries = all_queries.len();
            for (idx, (query, mut test_result)) in all_queries.into_iter().enumerate() {
                total_requests += 1;

                let body = serde_json::json!({ "query": query });
                match client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .timeout(std::time::Duration::from_millis(timeout))
                    .json(&body)
                    .send()
                    .await
                {
                    Ok(response) => {
                        let status = response.status().as_u16();
                        let response_text = match response.text().await {
                            Ok(text) => text,
                            Err(e) => {
                                tracing::warn!("Failed to read GraphQL response body: {}", e);
                                String::new()
                            }
                        };

                        test_result.response_snippet = response_text.chars().take(200).collect();
                        test_result.success = !is_graphql_error(&response_text)
                            && (is_graphql_success(&response_text) || status == 200);

                        match test_result.vulnerability {
                            GraphQLVulnerability::DepthLimitBypass if test_result.success => {
                                depth_limit_bypassed = true;
                                let finding = format!(
                                "[{}] Depth Limit Bypass: Query depth exceeded limit (status {})",
                                test_result.severity, status
                            );
                                injection_findings.push(finding);
                            }
                            GraphQLVulnerability::AliasOverload if test_result.success => {
                                alias_overload_vulnerable = true;
                                let finding = format!(
                                    "[{}] Alias Overload: Multiple aliases accepted (status {})",
                                    test_result.severity, status
                                );
                                injection_findings.push(finding);
                            }
                            GraphQLVulnerability::QueryInjection if test_result.success => {
                                let finding = format!(
                                    "[{}] {}: {} (status {})",
                                    test_result.severity,
                                    test_result.vulnerability,
                                    test_result.description,
                                    status
                                );
                                injection_findings.push(finding);
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        tracing::warn!("GraphQL query request failed: {}", e);
                        errors += 1;
                    }
                }

                let progress = 25 + ((idx as u64 * 70) / total_queries.max(1) as u64);
                send_progress(&progress_tx, progress.min(95), 100).await;
            }
        } else {
            total_requests += batch_count;
        }

        send_progress(&progress_tx, 98, 100).await;

        let results = GraphQlResults {
            target: url.clone(),
            introspection_enabled,
            depth_limit_bypassed,
            alias_overload_vulnerable,
            injection_findings,
            total_requests,
            errors,
            duration_ms: start.elapsed().as_millis() as u64,
        };

        send_progress(&progress_tx, 100, 100).await;
        Ok(TaskResult::GraphQl(results))
    })
    .await
    .unwrap_or_else(|_| Err(anyhow::anyhow!("GraphQL test timed out after 300 seconds")))
}

fn is_graphql_error(response: &str) -> bool {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(response) {
        if json.get("errors").is_some() {
            return true;
        }
    }
    false
}

fn is_graphql_success(response: &str) -> bool {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(response) {
        if let Some(data) = json.get("data") {
            return !data.is_null();
        }
    }
    false
}

#[allow(clippy::too_many_arguments)]
pub async fn run_oauth(
    url: String,
    client_id: Option<String>,
    redirect_uri: Option<String>,
    redirect_test: bool,
    scope_test: bool,
    state_test: bool,
    grant_test: bool,
    _concurrency: usize,
    _timeout: u64,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult> {
    use crate::fuzzer::payloads::oauth::{OAuthFuzzer, OAuthTestResult};

    let op_timeout = Duration::from_secs(300);
    tokio::time::timeout(op_timeout, async {
        send_progress(&progress_tx, 0, 100).await;

        let start_time = std::time::Instant::now();
        let client = crate::utils::get_shared_insecure_http_client();

        let default_client_id = client_id
            .clone()
            .unwrap_or_else(|| "test_client".to_string());
        let default_redirect_uri = redirect_uri
            .clone()
            .unwrap_or_else(|| format!("{}/callback", url.trim_end_matches('/')));

        let mut fuzzer = OAuthFuzzer::new(default_client_id, default_redirect_uri)
            .with_issuer(url.clone())
            .with_client(client)
            .with_redirect_test(redirect_test)
            .with_scope_test(scope_test)
            .with_state_test(state_test)
            .with_grant_test(grant_test);

        send_progress(&progress_tx, 20, 100).await;

        let mut all_results: Vec<OAuthTestResult> = fuzzer.test_issuer().await;
        let mut total_requests = all_results.len();

        if redirect_test {
            let redirect_results = fuzzer.test_redirect_uri(&format!("{}/authorize", url));
            total_requests += redirect_results.len();
            all_results.extend(redirect_results);
        }

        send_progress(&progress_tx, 50, 100).await;

        if scope_test {
            let scope_results = fuzzer.test_scope_escalation(&format!("{}/authorize", url));
            total_requests += scope_results.len();
            all_results.extend(scope_results);
        }

        if state_test {
            let state_results = fuzzer.test_state_parameter(&format!("{}/authorize", url));
            total_requests += state_results.len();
            all_results.extend(state_results);
        }

        send_progress(&progress_tx, 75, 100).await;

        if grant_test {
            let grant_results = fuzzer.test_grant_type_mixing(&format!("{}/token", url));
            total_requests += grant_results.len();
            all_results.extend(grant_results);
        }

        let redirect_vulnerabilities: Vec<String> = all_results
            .iter()
            .filter(|r| r.success && r.vulnerability.to_string().contains("Redirect"))
            .map(|r| format!("{} - {}", r.description, r.proof))
            .collect();

        let scope_vulnerabilities: Vec<String> = all_results
            .iter()
            .filter(|r| r.success && r.vulnerability.to_string().contains("Scope"))
            .map(|r| format!("{} - {}", r.description, r.proof))
            .collect();

        let state_vulnerabilities: Vec<String> = all_results
            .iter()
            .filter(|r| r.success && r.vulnerability.to_string().contains("State"))
            .map(|r| format!("{} - {}", r.description, r.proof))
            .collect();

        let grant_vulnerabilities: Vec<String> = all_results
            .iter()
            .filter(|r| r.success && r.vulnerability.to_string().contains("Grant"))
            .map(|r| format!("{} - {}", r.description, r.proof))
            .collect();

        let errors = all_results.iter().filter(|r| !r.success).count();

        let duration_ms = start_time.elapsed().as_millis() as u64;

        let results = OAuthResults {
            target: url,
            redirect_vulnerabilities,
            scope_vulnerabilities,
            state_vulnerabilities,
            grant_vulnerabilities,
            total_requests,
            errors,
            duration_ms,
        };

        send_progress(&progress_tx, 100, 100).await;
        Ok(TaskResult::OAuth(results))
    })
    .await
    .unwrap_or_else(|_| Err(anyhow::anyhow!("OAuth test timed out after 300 seconds")))
}

#[cfg(feature = "nse")]
pub async fn run_nse(
    target: String,
    script: String,
    script_args: Option<String>,
    custom_script: Option<String>,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult> {
    use crate::nse::{NseRunRequest, ResolvedNseExecutionProfile};

    send_progress(&progress_tx, 0, 100).await;

    let target_clone = target.clone();
    let script_clone = script.clone();
    let (output, errors, success, report) = tokio::time::timeout(
        tokio::time::Duration::from_secs(300),
        tokio::task::spawn_blocking(move || {
            // NOTE: This dispatch path is currently only reached from TUI
            // (a manual surface). When automated surfaces (agent/MCP/daemon)
            // are added, they must pass an appropriate profile through
            // RunRequest and construct the request with that profile.
            let profile = ResolvedNseExecutionProfile::manual_permissive(Some(&target_clone));
            // Custom script files resolve through ScriptResolver (no direct
            // filesystem read); named scripts use the built-in source
            // identity. Execution and report assembly are runtime-owned.
            let source = if let Some(ref script_path) = custom_script {
                crate::nse::NseScriptSource::File {
                    path: std::path::PathBuf::from(script_path),
                }
            } else {
                crate::nse::NseScriptSource::Builtin {
                    name: script_clone.clone(),
                }
            };
            let mut request = NseRunRequest::new(&target_clone, source, profile);
            if let Some(ref args) = script_args {
                request = request.with_script_args(args);
            }
            let report =
                crate::nse::execute_nse_run(request).map_err(|e| anyhow::anyhow!("{}", e))?;

            let success = report.rules.iter().any(|r| r.matched) || report.output.has_output;

            Ok::<_, anyhow::Error>((
                report.output.content.clone(),
                String::new(),
                success,
                Some(report),
            ))
        }),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Task execution failed: {}", e))
    .and_then(|result| match result {
        Ok(inner) => inner.map_err(|e| anyhow::anyhow!("NSE script failed: {}", e)),
        Err(e) => {
            if e.is_panic() {
                tracing::warn!("NSE task panicked: {:?}", e);
                Err(anyhow::anyhow!("NSE task panicked"))
            } else {
                tracing::warn!("NSE task failed: {:?}", e);
                Err(anyhow::anyhow!("NSE task failed: {}", e))
            }
        }
    })?;

    send_progress(&progress_tx, 100, 100).await;

    let results = NseResults {
        target,
        script,
        output,
        errors,
        success,
        report,
    };

    Ok(TaskResult::Nse(results))
}

#[cfg(all(test, feature = "nse"))]
mod nse_canonical_dispatch_tests {
    use super::*;

    fn test_channel() -> tokio::sync::mpsc::Sender<(u64, u64)> {
        tokio::sync::mpsc::channel(100).0
    }

    #[tokio::test]
    async fn dispatch_nse_builtin_report_is_complete() {
        let result = run_nse(
            "127.0.0.1".to_string(),
            "banner".to_string(),
            None,
            None,
            test_channel(),
        )
        .await
        .expect("dispatch builtin run succeeds");
        let TaskResult::Nse(results) = result else {
            panic!("expected TaskResult::Nse");
        };
        let report = results.report.expect("dispatch must return a report");
        // Convergence fix: the dispatch path now sets the executor target.
        assert_eq!(report.target, "127.0.0.1");
        assert_eq!(report.script_name, "banner");
        assert_eq!(report.script_source.kind, "builtin");
        assert_eq!(report.profile.kind, "manual-permissive");
        // Resolver diagnostics are no longer dropped by dispatch.
        assert_eq!(report.resolver.total_diagnostics, 1);
        assert_eq!(report.resolver.resolved_count, 1);
        // Evidence is now extracted on the dispatch path.
        assert!(
            !report.evidence.is_empty(),
            "dispatch reports must carry evidence when output exists"
        );
    }

    #[tokio::test]
    async fn dispatch_nse_custom_file_resolves_through_resolver() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("dispatch-parity.nse");
        std::fs::write(&path, "hostrule = function(host) return false end").expect("write fixture");

        let result = run_nse(
            "127.0.0.1".to_string(),
            "custom".to_string(),
            None,
            Some(path.display().to_string()),
            test_channel(),
        )
        .await
        .expect("dispatch file run succeeds");
        let TaskResult::Nse(results) = result else {
            panic!("expected TaskResult::Nse");
        };
        assert!(results.success, "non-empty output marks success");
        let report = results.report.expect("dispatch must return a report");
        assert_eq!(report.script_source.kind, "file");
        assert_eq!(report.resolver.resolved_count, 1);
        assert_eq!(report.rules.len(), 1);
        assert!(!report.rules[0].matched);
        assert!(!report.evidence.is_empty());
    }

    #[tokio::test]
    async fn dispatch_nse_missing_file_is_an_error_not_empty_success() {
        let result = run_nse(
            "127.0.0.1".to_string(),
            "custom".to_string(),
            None,
            Some("/tmp/eggsec-dispatch-missing-does-not-exist.nse".to_string()),
            test_channel(),
        )
        .await;
        assert!(
            result.is_err(),
            "unresolvable custom files must fail dispatch, not empty-succeed"
        );
    }
}
