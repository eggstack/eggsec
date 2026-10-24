//! Desktop GUI frontend for Eggsec.
//!
//! The GUI is a **daemon-only** frontend (`ADR-0007`): it reaches the engine
//! exclusively through the daemon's Unix socket, using the shared
//! `eggsec-frontend-client` contract. It therefore depends on no engine
//! crate, no daemon crate, and no terminal-frontend crate.
//!
//! The shell technology is Tauri v2, pinned to the 2.11.x line (`ADR-0009`).
//! The Tauri dependency lives entirely inside this crate.

pub mod availability;
pub mod config;
pub mod connection;
pub mod coverage;
pub mod daemon;
pub mod export;
pub mod form;
pub mod tasks;

#[cfg(feature = "shell")]
pub mod commands;
#[cfg(feature = "shell")]
pub mod shell;

use std::fmt;

/// Everything that can go wrong inside the GUI, before an error message
/// reaches the webview.
#[derive(Debug)]
pub enum GuiError {
    /// The GUI was built without the `shell` feature.
    ShellNotBuilt,
    /// The daemon could not be reached or refused the connection.
    Connection(String),
    /// A command-line argument was not understood.
    Usage(String),
    /// The shell framework failed to start.
    Shell(String),
}

impl fmt::Display for GuiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GuiError::ShellNotBuilt => write!(
                f,
                "the Eggsec GUI shell was not built into this binary: rebuild with the \
                 `shell` feature (the default) to get the desktop window"
            ),
            GuiError::Connection(message) => write!(f, "{message}"),
            GuiError::Usage(message) => write!(f, "{message}"),
            GuiError::Shell(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for GuiError {}

/// Run the GUI.
///
/// With the `shell` feature this opens the desktop window. Without it, it
/// reports that the shell was not built and exits non-zero — that is what
/// keeps the `--no-default-features` workspace build green without pretending
/// the binary works.
#[cfg(feature = "shell")]
pub fn run() -> Result<(), GuiError> {
    let options = config::GuiOptions::from_args(std::env::args().skip(1))?;
    shell::run(options)
}

/// Run the GUI in a build that carries no shell.
#[cfg(not(feature = "shell"))]
pub fn run() -> Result<(), GuiError> {
    Err(GuiError::ShellNotBuilt)
}
