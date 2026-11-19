use thiserror::Error;

/// Errors specific to the eggsec-daemon crate.
#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Runtime error: {0}")]
    Runtime(#[from] eggsec_runtime::RuntimeError),

    /// The socket path is held by a live daemon.
    ///
    /// The bind guard refuses to displace a daemon that is serving the path.
    /// The variant carries the **facts** — the path and the live daemon's
    /// protocol version — not a formatted string, so the callers that render
    /// the refusal cannot invent a claim the probe did not make.
    #[error("{}", socket_path_held_message(.socket_path, .protocol_version))]
    SocketPathHeld {
        /// The socket path the live daemon is serving.
        socket_path: String,
        /// The protocol version the live daemon reported.
        protocol_version: u32,
    },

    /// Another daemon took the socket path while this daemon was reclaiming it.
    ///
    /// This is a **distinct claim** from [`DaemonError::SocketPathHeld`]: the
    /// probe did not establish that a live daemon is serving the path, so the
    /// message must not say that one is.
    #[error("{}", socket_path_taken_message(.socket_path))]
    SocketPathTaken {
        /// The socket path another daemon took.
        socket_path: String,
    },
}

/// The operator-facing refusal text for a live daemon that holds the socket
/// path.
///
/// One message, shared by the daemon binary's own pre-flight, the CLI's
/// pre-flight refusal and the server's bind refusal, so the three cannot
/// drift apart. It names the path, says a live daemon holds it, gives that
/// daemon's protocol version, and names **both** remedies — stop the running
/// daemon, or use a different path. The environment variable's name comes
/// from the canonical crate, so it cannot drift from the rule either.
///
/// The second remedy names the environment variable rather than a flag,
/// because the flag is spelled differently on each surface (`--socket-path`
/// on the daemon binary, `--socket` on the CLI and the GUI) and the
/// environment variable is the one spelling that works everywhere.
pub fn socket_path_held_message(
    socket_path: impl AsRef<str>,
    protocol_version: impl std::fmt::Display,
) -> String {
    format!(
        "refusing to displace the live daemon already serving {socket_path} \
         (protocol version {protocol_version}) — stop it first, or start this \
         daemon on another path (an explicit socket path, or {env_var}=<PATH>)",
        socket_path = socket_path.as_ref(),
        env_var = eggsec_runtime::DAEMON_SOCKET_ENV_VAR,
    )
}

/// The operator-facing refusal text for a socket path another daemon took
/// between the liveness probe and the reclaim.
///
/// It must not claim that a live daemon is serving the path: the probe did
/// not establish that.
pub fn socket_path_taken_message(socket_path: impl AsRef<str>) -> String {
    format!(
        "another daemon took the socket path {socket_path} while this daemon was \
         reclaiming it — stop it first, or start this daemon on another path \
         (an explicit socket path, or {env_var}=<PATH>)",
        socket_path = socket_path.as_ref(),
        env_var = eggsec_runtime::DAEMON_SOCKET_ENV_VAR,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_display() {
        let err = DaemonError::Io(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "connection refused",
        ));
        assert_eq!(format!("{}", err), "IO error: connection refused");
    }

    #[test]
    fn serialization_error_display() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let err = DaemonError::Serialization(json_err);
        let msg = format!("{}", err);
        assert!(msg.starts_with("Serialization error: "));
    }

    #[test]
    fn protocol_error_display() {
        let err = DaemonError::Protocol("unknown command".into());
        assert_eq!(format!("{}", err), "Protocol error: unknown command");
    }

    #[test]
    fn runtime_error_display() {
        let err = DaemonError::Runtime(eggsec_runtime::RuntimeError::SessionNotFound("s-1".into()));
        assert_eq!(format!("{}", err), "Runtime error: session not found: s-1");
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe");
        let daemon_err: DaemonError = io_err.into();
        assert!(matches!(daemon_err, DaemonError::Io(_)));
    }

    #[test]
    fn from_serde_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("bad").unwrap_err();
        let daemon_err: DaemonError = json_err.into();
        assert!(matches!(daemon_err, DaemonError::Serialization(_)));
    }

    #[test]
    fn from_runtime_error() {
        let rt_err = eggsec_runtime::RuntimeError::UnsupportedTaskKind;
        let daemon_err: DaemonError = rt_err.into();
        assert!(matches!(daemon_err, DaemonError::Runtime(_)));
    }

    #[test]
    fn the_held_refusal_names_the_path_the_daemon_and_both_remedies() {
        let err = DaemonError::SocketPathHeld {
            socket_path: "/run/eggsec.sock".into(),
            protocol_version: 2,
        };
        let message = format!("{err}");
        assert!(message.contains("/run/eggsec.sock"), "{message}");
        assert!(message.contains("live daemon"), "{message}");
        assert!(message.contains("protocol version 2"), "{message}");
        assert!(message.contains("stop it first"), "{message}");
        assert!(
            message.contains(eggsec_runtime::DAEMON_SOCKET_ENV_VAR),
            "{message}"
        );
    }

    #[test]
    fn the_taken_refusal_does_not_claim_a_live_daemon_serves_the_path() {
        let err = DaemonError::SocketPathTaken {
            socket_path: "/run/eggsec.sock".into(),
        };
        let message = format!("{err}");
        assert!(message.contains("/run/eggsec.sock"), "{message}");
        assert!(message.contains("another daemon took"), "{message}");
        assert!(!message.contains("live daemon"), "{message}");
    }
}
