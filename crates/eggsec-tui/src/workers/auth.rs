use crate::workers::TaskResult;

pub async fn run_auth_task(
    target: String,
    username: Option<String>,
    password_list: Option<String>,
    credential_file: Option<String>,
    max_attempts: usize,
    concurrency: usize,
    timeout: u64,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use eggsec::auth::AuthEngine;

    if let Err(e) = progress_tx.send((0, 8)).await {
        tracing::warn!("Failed to send auth progress: {}", e);
    }

    let mut engine = AuthEngine::new(max_attempts, concurrency, timeout, true)?;

    let usernames = username
        .into_iter()
        .collect::<Vec<_>>();
    let passwords = password_list
        .map(|p| vec![p])
        .unwrap_or_default();
    engine.load_wordlists(usernames, passwords);

    if let Some(ref cred_file) = credential_file {
        let _ = cred_file;
    }

    let report = match tokio::time::timeout(
        std::time::Duration::from_secs(timeout * 8),
        engine.run_full_test(&target),
    )
    .await
    {
        Ok(Ok(report)) => report,
        Ok(Err(e)) => {
            tracing::warn!("Auth test error: {}", e);
            eggsec::auth::AuthTestReport {
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
        Err(_) => {
            engine.stop();
            eggsec::auth::AuthTestReport {
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
    };

    if let Err(e) = progress_tx.send((8, 8)).await {
        tracing::warn!("Failed to send auth progress: {}", e);
    }
    if let Err(e) = result_tx.send(TaskResult::Auth(report)).await {
        tracing::warn!("Failed to send auth result: {}", e);
    }
    Ok(())
}
