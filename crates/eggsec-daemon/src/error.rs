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
}
