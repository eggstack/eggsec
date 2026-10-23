//! The Tauri shell: the window, the IPC registration, and startup.
//!
//! The shell framework appears only in this module and in `commands.rs`.
//! Nothing outside `crates/eggsec-gui` may depend on it.

use crate::commands;
use crate::commands::GuiState;
use crate::config::GuiOptions;
use crate::GuiError;

/// Start the desktop window.
///
/// The GUI does not spawn, supervise, or stop a daemon: it connects to one
/// that is already running. The socket path the operator chose is carried in
/// the state's default connection attempt, so the window opens with the
/// catalog already loaded when the daemon is reachable.
pub fn run(options: GuiOptions) -> Result<(), GuiError> {
    tracing::info!("starting the Eggsec GUI against {}", options.socket_path);

    tauri::Builder::default()
        .manage(GuiState::default())
        .invoke_handler(tauri::generate_handler![
            commands::connect_daemon,
            commands::list_operations,
            commands::list_task_kinds,
            commands::form_for,
            commands::submit_task,
            commands::cancel_task,
            commands::cancel_active,
            commands::task_snapshot
        ])
        .run(tauri::generate_context!())
        .map_err(|error| GuiError::Shell(format!("the Eggsec GUI shell failed to start: {error}")))
}
