//! The GUI's connection to a running daemon.
//!
//! The GUI is daemon-only by construction (`ADR-0007`): it reaches the engine
//! only through the daemon's Unix socket, using the shared
//! `eggsec-frontend-client` contract. It never spawns, supervises, or stops
//! a daemon — it connects to one that is already running and, when the socket
//! is unreachable, says precisely how to start a *usable* one.

use eggsec_daemon_protocol::operations::DescribeOperationsResponse;
use eggsec_daemon_protocol::protocol::{DaemonCapabilities, DAEMON_PROTOCOL_VERSION};
use eggsec_frontend_client::RuntimeClient;
use eggsec_frontend_client::{ClientIdentity, ClientKind, DaemonClientError, DaemonRuntimeClient};
use eggsec_runtime::request::RuntimeSurface;
use eggsec_runtime::session::SessionScope;
use eggsec_runtime::SessionId;

use crate::GuiError;

/// The identity the GUI presents to the daemon.
///
/// The GUI declares itself as a GUI client, with its own label and its own
/// request-id namespace, so the daemon's audit trail distinguishes it from
/// the CLI and the TUI.
pub fn gui_identity() -> ClientIdentity {
    ClientIdentity {
        kind: ClientKind::Gui,
        label: "eggsec-gui".to_owned(),
        request_prefix: "gui-".to_owned(),
    }
}

/// The scope the GUI stamps on the session it creates.
///
/// The GUI cannot widen scope beyond what the operator supplied, and it does
/// not read the engine's loaded scope — that lives behind the daemon. The
/// session is therefore marked as not explicitly scoped, with the source
/// recorded so the audit trail can tell where it came from.
pub fn gui_session_scope() -> SessionScope {
    SessionScope {
        is_explicit: false,
        source: "gui-default".to_owned(),
        path: None,
    }
}

/// The command that starts a daemon the GUI can actually use.
///
/// The daemon must be BUILT with the engine executor compiled in:
/// `--full-executor` is a runtime flag over the daemon's `service` cargo
/// feature, so a binary built without the cargo feature accepts the flag
/// and falls back to a no-op executor that rejects every task.
pub fn daemon_start_command(socket_path: &str) -> String {
    format!("eggsec-daemon --full-executor --socket-path {socket_path}")
}

/// The diagnostic shown when the daemon's socket cannot be reached.
///
/// It names the socket path and the exact command that starts a usable
/// daemon, because "connection refused" on its own leaves the operator
/// guessing which daemon build will actually work.
pub fn unreachable_diagnostic(socket_path: &str, error: &DaemonClientError) -> String {
    format!(
        "cannot reach the Eggsec daemon at {socket_path}: {error}\n\n\
         Start a usable daemon with:\n  {}\n\n\
         `eggsec daemon start` hosts the engine executor, so a CLI-hosted \
         daemon is usable; the daemon binary must be BUILT with the `service` \
         cargo feature (its deprecated `full-executor` alias selects the same \
         profile) for the --full-executor flag to have any effect.",
        daemon_start_command(socket_path)
    )
}

/// A live GUI connection: the daemon client, the GUI's session, and the
/// catalog the daemon published.
pub struct GuiConnection {
    /// The daemon client, already declared as a GUI client.
    pub client: DaemonRuntimeClient,
    /// The session the GUI created, stamped `DesktopGuiManual`.
    pub session_id: SessionId,
    /// The operation catalog and the derived request schema.
    ///
    /// Shared behind an `Arc` so a command can read the catalog without
    /// holding the GUI's state lock across an await.
    pub catalog: std::sync::Arc<DescribeOperationsResponse>,
    /// The daemon's capability advertisement.
    pub capabilities: DaemonCapabilities,
    /// The daemon protocol version the daemon speaks.
    pub protocol_version: u32,
}

impl GuiConnection {
    /// Connect to the daemon, verify it, open a session, and fetch the
    /// operation catalog.
    ///
    /// The order matters. The protocol version is checked before anything
    /// else, because a daemon that speaks a different protocol cannot be
    /// trusted to interpret a session request the way this GUI means it.
    pub async fn connect(socket_path: &str) -> Result<Self, GuiError> {
        let client = DaemonRuntimeClient::connect(socket_path, gui_identity())
            .await
            .map_err(|error| GuiError::Connection(unreachable_diagnostic(socket_path, &error)))?;

        let health = client
            .health()
            .await
            .map_err(|error| GuiError::Connection(unreachable_diagnostic(socket_path, &error)))?;

        if health.protocol_version != DAEMON_PROTOCOL_VERSION {
            return Err(GuiError::Connection(format!(
                "the daemon at {socket_path} speaks daemon protocol version {}, but this GUI \
                 speaks version {DAEMON_PROTOCOL_VERSION}. The GUI refuses to operate against an \
                 incompatible daemon.",
                health.protocol_version
            )));
        }

        let capabilities = client
            .daemon_capabilities()
            .await
            .map_err(|error| GuiError::Connection(unreachable_diagnostic(socket_path, &error)))?;

        if !capabilities.operation_schema {
            return Err(GuiError::Connection(format!(
                "the daemon at {socket_path} predates the operation-schema publication: it cannot \
                 publish the operation catalog or the request schema this GUI renders. Restart it \
                 with `{}`.",
                daemon_start_command(socket_path)
            )));
        }

        let session_id = client
            .create_session(
                RuntimeSurface::DesktopGuiManual,
                Some(gui_session_scope()),
                Vec::new(),
            )
            .await
            .map_err(|error| {
                GuiError::Connection(format!(
                    "the daemon at {socket_path} refused to open a desktop-GUI session: {error}"
                ))
            })?;

        let catalog = client.describe_operations().await.map_err(|error| {
            GuiError::Connection(format!(
                "the daemon at {socket_path} could not publish the operation catalog: {error}"
            ))
        })?;

        Ok(Self {
            client,
            session_id,
            catalog: std::sync::Arc::new(catalog),
            capabilities,
            protocol_version: health.protocol_version,
        })
    }

    /// Whether the connected daemon advertises no task kinds at all.
    ///
    /// The daemon hosts a no-op executor in that case: it answers every
    /// command correctly and rejects every task. The GUI reports the
    /// condition rather than presenting a submit button that cannot work.
    pub fn daemon_advertises_no_task_kinds(&self) -> bool {
        self.capabilities.runtime.task_kinds.is_empty()
    }

    /// The diagnostic shown when the connected daemon cannot run anything.
    pub fn no_op_executor_diagnostic(&self, socket_path: &str) -> String {
        format!(
            "the daemon at {socket_path} advertises no task kinds: it hosts a no-op executor that \
             rejects every submitted task. Restart it with `{}`.",
            daemon_start_command(socket_path)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_identity_is_the_gui_client_kind() {
        let identity = gui_identity();
        assert_eq!(identity.kind, ClientKind::Gui);
        assert_eq!(identity.label, "eggsec-gui");
        assert_eq!(identity.request_id(0), "gui-0");
    }

    #[test]
    fn gui_session_scope_does_not_widen_the_scope() {
        let scope = gui_session_scope();
        assert!(
            !scope.is_explicit,
            "the GUI must not claim an explicit scope"
        );
        assert_eq!(scope.source, "gui-default");
        assert!(scope.path.is_none(), "the GUI must not invent a scope path");
    }

    #[test]
    fn unreachable_diagnostic_names_the_socket_and_the_start_command() {
        let error = DaemonClientError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no such file or directory",
        ));
        let diagnostic = unreachable_diagnostic("/tmp/eggsec.sock", &error);
        assert!(diagnostic.contains("/tmp/eggsec.sock"));
        assert!(diagnostic.contains("eggsec-daemon --full-executor --socket-path /tmp/eggsec.sock"));
        // The runtime flag alone cannot add an engine that was not compiled
        // in, so the diagnostic has to name the BUILD-time requirement too.
        // Without it the operator would build a default daemon, pass the
        // flag, and get a daemon that rejects every task.
        assert!(
            diagnostic.contains("`service` cargo feature"),
            "the diagnostic must name the cargo feature the daemon must be built with: {diagnostic}"
        );
    }

    #[test]
    fn start_command_requires_the_full_executor() {
        let command = daemon_start_command("/tmp/eggsec.sock");
        assert!(command.contains("--full-executor"));
        assert!(command.contains("--socket-path /tmp/eggsec.sock"));
    }
}
