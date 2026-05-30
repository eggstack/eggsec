use crate::tui::workers::TaskResult;

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
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::cli::{CommonHttpArgs, FuzzArgs, FuzzMode};
    use crate::fuzzer::engine::FuzzEngine;

    let fuzz_mode = if mode.to_lowercase() == "burst" {
        FuzzMode::Burst
    } else if mode.to_lowercase() == "adaptive" {
        FuzzMode::Adaptive
    } else {
        FuzzMode::Sequential
    };

    let args = FuzzArgs {
        url: target,
        payload_type,
        mode: fuzz_mode,
        mutate: mutations,
        mutation_count,
        grammar_fuzz: false,
        grammar_type: None,
        adaptive_rate: false,
        session: false,
        diffing: false,
        capture_baseline: false,
        enhanced_redos: false,
        waf_fingerprint: false,
        chaining: false,
        chain_file: None,
        method,
        param,
        concurrency,
        timeout,
        json: false,
        output: None,
        verbose: false,
        quiet: false,
        format: None,
        target: None,
        jwt_token: None,
        oauth_issuer: None,
        oauth_client_id: None,
        oauth_client_secret: None,
        idor_base_id: None,
        idor_user_ids: None,
        ssti_param: None,
        graphql_introspection,
        graphql_depth_bypass,
        graphql_alias_overload,
        oauth_redirect: oauth_redirect_test,
        oauth_scope: oauth_scope_test,
        oauth_state: oauth_state_test,
        oauth_grant: oauth_grant_test,
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
        common: CommonHttpArgs::default(),
    };

    let mut engine = FuzzEngine::new_with_tui_mode(args, true)?;
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

    if let Err(e) = result_tx.send(TaskResult::Fuzz(session)).await {
        tracing::warn!("Failed to send fuzz results: {}", e);
    }
    if let Err(e) = progress_tx.send((1, 1)).await {
        tracing::warn!("Failed to send progress: {}", e);
    }
    Ok(())
}

pub async fn run_waf(
    target: String,
    bypass_mode: bool,
    techniques: Vec<String>,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
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
        use crate::cli::WafArgs;
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

        let args = WafArgs {
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
            common: crate::cli::CommonHttpArgs::default(),
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
        if let Err(e) = result_tx
            .send(TaskResult::WafBypass {
                detection,
                bypasses,
            })
            .await
        {
            tracing::warn!("Failed to send WAF bypass results: {}", e);
        }
    } else {
        if let Err(e) = result_tx.send(TaskResult::WafDetection(detection)).await {
            tracing::warn!("Failed to send WAF detection results: {}", e);
        }
    }

    if let Err(e) = progress_tx.send((1, 1)).await {
        tracing::warn!("Failed to send progress: {}", e);
    }
    Ok(())
}

pub async fn run_waf_stress(
    target: String,
    concurrency: usize,
    timeout: u64,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::cli::WafStressArgs;
    use crate::fuzzer::run_waf_stress as fuzzer_run_waf_stress;

    let args = WafStressArgs {
        url: target,
        concurrency,
        timeout,
        json: false,
        verbose: false,
        quiet: false,
        output: None,
        common: Default::default(),
    };

    match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        fuzzer_run_waf_stress(args),
    )
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            tracing::warn!("WAF stress failed: {}", e);
            if let Err(e) = progress_tx.send((1, 1)).await {
                tracing::warn!("Failed to send progress: {}", e);
            }
            if let Err(send_err) = result_tx
                .send(TaskResult::Error(e.to_string()))
                .await
            {
                tracing::warn!("Failed to send WAF stress error: {}", send_err);
            }
            return Err(e.into());
        }
        Err(_) => {
            tracing::warn!("WAF stress timed out after 60s");
            if let Err(e) = progress_tx.send((1, 1)).await {
                tracing::warn!("Failed to send progress: {}", e);
            }
            if let Err(send_err) = result_tx
                .send(TaskResult::Error(
                    "WAF stress timed out after 60s".to_string(),
                ))
                .await
            {
                tracing::warn!("Failed to send WAF stress timeout error: {}", send_err);
            }
            return Err(anyhow::anyhow!("WAF stress timed out after 60s"));
        }
    }
    if let Err(e) = progress_tx.send((1, 1)).await {
        tracing::warn!("Failed to send progress: {}", e);
    }
    if let Err(e) = result_tx.send(TaskResult::WafStress(vec![])).await {
        tracing::warn!("Failed to send WAF stress results: {}", e);
    }
    Ok(())
}
