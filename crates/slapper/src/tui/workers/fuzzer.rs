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
        common: CommonHttpArgs::default(),
    };

    let mut engine = FuzzEngine::new_with_tui_mode(args, true)?;
    let session = engine.run_return_session().await?;

    let _ = result_tx.send(TaskResult::Fuzz(session)).await;
    let _ = progress_tx.send((1, 1)).await;
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
    let detection = detector.detect(&target).await?;

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
        let bypasses = bypass_engine.run_bypasses(&detection).await?;
        let _ = result_tx
            .send(TaskResult::WafBypass {
                detection,
                bypasses,
            })
            .await;
    } else {
        let _ = result_tx.send(TaskResult::WafDetection(detection)).await;
    }

    let _ = progress_tx.send((1, 1)).await;
    Ok(())
}