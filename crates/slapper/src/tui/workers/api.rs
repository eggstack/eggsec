use crate::tui::workers::TaskResult;

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
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::fuzzer::payloads::graphql::{
        GraphQLFuzzer, GraphQLTestResult, GraphQLVulnerability,
    };
    use crate::tui::tabs::graphql::GraphQlResults;
    use std::time::Instant;

    let start = Instant::now();
    let _ = progress_tx.send((0, 100)).await;

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

    let _ = progress_tx.send((25, 100)).await;

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
                    test_result.success = is_graphql_error(&response_text)
                        || is_graphql_success(&response_text)
                        || status != 400;

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

            let progress = 25 + ((idx as u64 * 70) / total_queries as u64);
            let _ = progress_tx.send((progress.min(95), 100)).await;
        }
    } else {
        total_requests += batch_count;
    }

    let _ = progress_tx.send((98, 100)).await;

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

    let _ = progress_tx.send((100, 100)).await;
    let _ = result_tx.send(TaskResult::GraphQl(results)).await;

    Ok(())
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
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::fuzzer::payloads::oauth::{OAuthFuzzer, OAuthTestResult};
    use crate::tui::tabs::oauth::OAuthResults;

    let _ = progress_tx.send((0, 100)).await;

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

    let _ = progress_tx.send((20, 100)).await;

    let mut all_results: Vec<OAuthTestResult> = fuzzer.test_issuer().await;
    let mut total_requests = all_results.len();

    if redirect_test {
        let redirect_results = fuzzer.test_redirect_uri(&format!("{}/authorize", url));
        total_requests += redirect_results.len();
        all_results.extend(redirect_results);
    }

    let _ = progress_tx.send((50, 100)).await;

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

    let _ = progress_tx.send((75, 100)).await;

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

    let _ = progress_tx.send((100, 100)).await;
    let _ = result_tx.send(TaskResult::OAuth(results)).await;

    Ok(())
}

#[cfg(feature = "nse")]
pub async fn run_nse(
    target: String,
    script: String,
    script_args: Option<String>,
    custom_script: Option<String>,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::tui::tabs::nse::NseResults;
    use slapper_nse::NseExecutor;

    let _ = progress_tx.send((0, 100)).await;

    let target_clone = target.clone();
    let script_clone = script.clone();
    let (output, errors, success) = tokio::task::spawn_blocking(move || {
        let mut executor = NseExecutor::with_target(&target_clone)
            .map_err(|e| anyhow::anyhow!("Failed to create NSE executor: {}", e))?;

        if let Some(ref args) = script_args {
            executor
                .set_script_args(args)
                .map_err(|e| anyhow::anyhow!("Invalid script args: {}", e))?;
        }

        let script_content = if let Some(ref script_path) = custom_script {
            std::fs::read_to_string(script_path).map_err(|e| {
                anyhow::anyhow!("Failed to read custom script '{}': {}", script_path, e)
            })?
        } else {
            slapper_nse::get_builtin_script(&script_clone)
        };

        let output = executor
            .run_script(&script_content)
            .map_err(|e| anyhow::anyhow!("Script execution failed: {}", e))?;

        Ok::<_, anyhow::Error>((output, String::new(), true))
    })
    .await
    .map_err(|e| anyhow::anyhow!("Task execution failed: {}", e))?;

    let _ = progress_tx.send((100, 100)).await;

    let results = NseResults {
        target,
        script,
        output,
        errors,
        success,
    };

    let _ = result_tx.send(TaskResult::Nse(results)).await;

    Ok(())
}
