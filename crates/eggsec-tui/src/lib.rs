mod app;
mod components;
mod help;
pub mod runtime_client;
pub mod search;
mod session;
pub mod state;
pub mod tabs;
mod theme;
pub mod ui;
pub mod utils;

#[cfg(test)]
pub(crate) mod test_utils;

pub use app::*;
pub use tabs::Tab;

/// Runtime mode for the TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeMode {
    /// Embedded in-process runtime (default).
    Embedded,
    /// Remote daemon via Unix socket.
    Daemon {
        socket_path: String,
        session_id: Option<String>,
        new_session: bool,
        attach_latest: bool,
    },
}

impl Default for RuntimeMode {
    fn default() -> Self {
        Self::Embedded
    }
}
