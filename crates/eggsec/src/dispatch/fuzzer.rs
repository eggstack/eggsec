use crate::dispatch::types::{send_progress, TaskResult};

#[allow(clippy::too_many_arguments)]
pub async fn run_fuzz(
    target: String,
    payload_type: String,
    mode: String,
    mutations: bool,
    mutation_count: usize,
    method: String,
    param: Option<String>,
    concurrency: usize,
    timeout: u64,
    graphql_introspection: bool,
    graphql_depth_bypass: bool,
    graphql_alias_overload: bool,
    oauth_redirect_test: bool,
    oauth_scope_test: bool,
    oauth_state_test: bool,
    oauth_grant_test: bool,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult> {
    use crate::fuzzer::config::{FuzzConfig, FuzzMode};
    use crate::fuzzer::engine::FuzzEngine;

    let mode_lower = mode.to_lowercase();
    let fuzz_mode = if mode_lower == "burst" {
        FuzzMode::Burst
    } else if mode_lower == "adaptive" {
        FuzzMode::Adaptive
    } else {
        FuzzMode::Sequential
    };

    let config = FuzzConfig {
        url: target,
        payload_type,
        mode: fuzz_mode,
        mutate: mutations,
        mutation_count,
        method,
        param,
        concurrency,
        timeout,
        graphql_introspection,
        graphql_depth_bypass,
        graphql_alias_overload,
        oauth_redirect: oauth_redirect_test,
        oauth_scope: oauth_scope_test,
        oauth_state: oauth_state_test,
        oauth_grant: oauth_grant_test,
        common: crate::types::CommonHttpArgs::default(),
        ..Default::default()
    };

    let mut engine = FuzzEngine::new_with_tui_mode(config, true)?;
    let session = match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        engine.run_return_session(),
    )
    .await
    {
        Ok(Ok(session)) => session,
        Ok(Err(e)) => return Err(e.into()),
        Err(_) => return Err(anyhow::anyhow!("Fuzz session timed out after 60s")),
    };

    send_progress(&progress_tx, 1, 1).await;
    Ok(TaskResult::Fuzz(session))
}

pub async fn run_waf(
    target: String,
    bypass_mode: bool,
    techniques: Vec<String>,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult> {
    use crate::waf::WafDetector;

    let detector = WafDetector::new()?;
    let detection =
        match tokio::time::timeout(std::time::Duration::from_secs(30), detector.detect(&target))
            .await
        {
            Ok(Ok(d)) => d,
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => return Err(anyhow::anyhow!("WAF detection timed out after 30s")),
        };

    if bypass_mode {
        use crate::waf::{get_auto_profile, BypassEngine, TestType};

        let header_bypass = techniques
            .iter()
            .any(|t| t.eq_ignore_ascii_case("header") || t.eq_ignore_ascii_case("all"));
        let evasion = techniques
            .iter()
            .any(|t| t.eq_ignore_ascii_case("evasion") || t.eq_ignore_ascii_case("all"));
        let smuggling = techniques
            .iter()
            .any(|t| t.eq_ignore_ascii_case("smuggling") || t.eq_ignore_ascii_case("all"));

        let args = crate::fuzzer::config::WafConfig {
            url: target.clone(),
            detect_only: false,
            bypass: true,
            header_bypass,
            evasion,
            smuggling,
            profile: "auto".to_string(),
            test_type: None,
            concurrency: 10,
            timeout: 15,
            json: false,
            verbose: false,
            quiet: false,
            output: None,
            common: crate::types::CommonHttpArgs::default(),
        };

        let bypass_engine = BypassEngine::new(&args, Some(get_auto_profile()), TestType::All)?;
        let bypasses = match tokio::time::timeout(
            std::time::Duration::from_secs(60),
            bypass_engine.run_bypasses(&detection),
        )
        .await
        {
            Ok(Ok(b)) => b,
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => return Err(anyhow::anyhow!("WAF bypass timed out after 60s")),
        };
        send_progress(&progress_tx, 1, 1).await;
        Ok(TaskResult::WafBypass {
            detection,
            bypasses,
        })
    } else {
        send_progress(&progress_tx, 1, 1).await;
        Ok(TaskResult::WafDetection(detection))
    }
}

pub async fn run_waf_stress(
    target: String,
    concurrency: usize,
    timeout: u64,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult> {
    use crate::fuzzer::config::WafStressConfig;
    use crate::fuzzer::run_waf_stress as fuzzer_run_waf_stress;

    let config = WafStressConfig {
        url: target,
        concurrency,
        timeout,
        common: crate::types::CommonHttpArgs::default(),
        ..Default::default()
    };

    match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        fuzzer_run_waf_stress(config),
    )
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            tracing::warn!("WAF stress failed: {}", e);
            send_progress(&progress_tx, 1, 1).await;
            return Ok(TaskResult::Error(e.to_string()));
        }
        Err(_) => {
            tracing::warn!("WAF stress timed out after 60s");
            send_progress(&progress_tx, 1, 1).await;
            return Ok(TaskResult::Error(
                "WAF stress timed out after 60s".to_string(),
            ));
        }
    }
    send_progress(&progress_tx, 1, 1).await;
    tracing::debug!(
        "WAF stress completed (fuzzer_run_waf_stress returned no results, sending empty WafStress)"
    );
    Ok(TaskResult::WafStress(vec![]))
}
