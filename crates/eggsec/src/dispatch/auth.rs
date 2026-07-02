use crate::dispatch::types::{send_progress, TaskResult};

fn empty_auth_report(target: String) -> crate::auth::AuthTestReport {
    crate::auth::AuthTestReport {
        target,
        tests_run: Vec::new(),
        brute_force: None,
        credential_stuffing: None,
        lockout_detection: None,
        rate_limit: None,
        mfa: None,
        session: None,
        timing: None,
        password_policy: None,
        total_attempts: 0,
        findings: Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn run_auth_task(
    target: String,
    username: Option<String>,
    password_list: Option<String>,
    _credential_file: Option<String>,
    max_attempts: usize,
    concurrency: usize,
    timeout: u64,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult> {
    use crate::auth::AuthEngine;

    send_progress(&progress_tx, 0, 8).await;

    let mut engine = AuthEngine::new(max_attempts, concurrency, timeout, true)?;

    let usernames = username.into_iter().collect::<Vec<_>>();
    let passwords = password_list.map(|p| vec![p]).unwrap_or_default();
    engine.load_wordlists(usernames, passwords);

    let report = match tokio::time::timeout(
        std::time::Duration::from_secs(timeout * 8),
        engine.run_full_test(&target),
    )
    .await
    {
        Ok(Ok(report)) => report,
        Ok(Err(e)) => {
            tracing::warn!("Auth test error: {}", e);
            empty_auth_report(target)
        }
        Err(_) => {
            engine.stop();
            empty_auth_report(target)
        }
    };

    send_progress(&progress_tx, 8, 8).await;
    Ok(TaskResult::Auth(report))
}
