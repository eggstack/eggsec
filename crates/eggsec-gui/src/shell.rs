//! The Tauri shell: the window, the IPC registration, and startup.
//!
//! The shell framework appears only in this module and in `commands.rs`.
//! Nothing outside `crates/eggsec-gui` may depend on it.

use crate::commands;
use crate::commands::GuiState;
use crate::config::GuiOptions;
use crate::daemon::TerminateSignal;
use crate::GuiError;
use tauri::Manager;

/// Start the desktop window.
///
/// The GUI does not spawn a daemon unless the operator named one with
/// `--daemon-bin`; by default it attaches to a daemon that is already
/// running. The socket path the operator chose is carried in the state's
/// default connection attempt, so the window opens with the catalog already
/// loaded when the daemon is reachable.
///
/// A daemon the GUI started itself is the GUI's to stop: the exit path
/// terminates it, and a signal watcher terminates it too, so a supervised
/// daemon never outlives the window that started it.
pub fn run(options: GuiOptions) -> Result<(), GuiError> {
    tracing::info!("starting the Eggsec GUI against {}", options.socket_path);

    let app = tauri::Builder::default()
        .manage(GuiState::new(options))
        .setup(|app| {
            spawn_shutdown_watcher(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::connect_daemon,
            commands::list_operations,
            commands::list_task_kinds,
            commands::form_for,
            commands::submit_task,
            commands::cancel_task,
            commands::cancel_active,
            commands::task_snapshot,
            commands::coverage,
            commands::export_report,
            commands::report_for,
            commands::daemon_status
        ])
        .build(tauri::generate_context!())
        .map_err(|error| {
            GuiError::Shell(format!("the Eggsec GUI shell failed to start: {error}"))
        })?;

    app.run(move |app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            stop_supervised_daemon(app_handle);
        }
    });

    Ok(())
}

/// Watch for the signals that mean the GUI is going away.
///
/// A GUI that is killed must not leave the daemon it started running: the
/// daemon would outlive the window that started it. The watcher stops the
/// supervised daemon and then leaves the process.
///
/// **This task is exempt from the 30–300s timeout rule.** It is a
/// long-lived signal waiter whose lifetime is the process's, and killing it
/// would leave the supervised daemon orphaned.
fn spawn_shutdown_watcher(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        wait_for_shutdown_signal().await;
        tracing::info!("the GUI is going away: stopping the daemon it started");
        stop_supervised_daemon_and_wait(&app).await;
        std::process::exit(0);
    });
}

/// Wait for a signal that means the GUI should stop.
#[cfg(unix)]
async fn wait_for_shutdown_signal() {
    let mut terminate =
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(signal) => signal,
            Err(error) => {
                tracing::warn!("the GUI could not watch for the terminate signal: {error}");
                return;
            }
        };
    let mut interrupt =
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()) {
            Ok(signal) => signal,
            Err(error) => {
                tracing::warn!("the GUI could not watch for the interrupt signal: {error}");
                return;
            }
        };
    tokio::select! {
        _ = terminate.recv() => {}
        _ = interrupt.recv() => {}
    }
}

/// Wait for a signal that means the GUI should stop.
///
/// This platform has no signal handling in the GUI: the window's own exit
/// path stops the supervised daemon.
#[cfg(not(unix))]
async fn wait_for_shutdown_signal() {}

/// Stop the daemon this GUI started, if it started one.
///
/// The terminate signal is sent synchronously, because the GUI's runtime is
/// going away and nothing can be awaited here. The escalation to an
/// unconditional kill is scheduled on whatever runtime is still alive.
fn stop_supervised_daemon(app_handle: &tauri::AppHandle) {
    let state = app_handle.state::<GuiState>();
    let Some(supervised) = state.take_supervised() else {
        return;
    };

    match supervised.signal_terminate() {
        TerminateSignal::Sent => {
            tracing::info!("the GUI asked the daemon it started to stop");
        }
        TerminateSignal::AlreadyGone => return,
        TerminateSignal::Unsupported => {
            tracing::warn!(
                "this platform has no terminate signal: the daemon the GUI started is force-killed \
                 instead"
            );
        }
    }

    // The escalation runs on Tauri's runtime when one is still alive. If it
    // is not, the terminate signal has already been delivered and the
    // daemon's own shutdown path takes it from there.
    tauri::async_runtime::spawn(async move {
        let outcome = supervised.terminate().await;
        tracing::info!("the daemon the GUI started stopped: {outcome:?}");
    });
}

/// Stop the daemon this GUI started, and wait for it to be gone.
///
/// This is the signal path: the process is about to leave, so the wait is
/// bounded but it happens before the process does.
async fn stop_supervised_daemon_and_wait(app_handle: &tauri::AppHandle) {
    let state = app_handle.state::<GuiState>();
    let Some(supervised) = state.take_supervised() else {
        return;
    };
    let outcome = supervised.terminate().await;
    tracing::info!("the daemon the GUI started stopped: {outcome:?}");
}
