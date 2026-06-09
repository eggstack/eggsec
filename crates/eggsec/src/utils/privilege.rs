#![cfg(any(feature = "stress-testing", feature = "packet-inspection"))]

use std::io;

#[cfg(unix)]
pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

#[cfg(not(unix))]
pub fn is_root() -> bool {
    false
}

#[cfg(unix)]
pub fn check_privileged(operation: &str) -> io::Result<()> {
    if !is_root() {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "{} requires root privileges. Run with sudo or as root user.",
                operation
            ),
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(unix))]
pub fn check_privileged(operation: &str) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        format!("{} is not supported on this platform.", operation),
    ))
}

pub fn require_root(operation: &str) -> anyhow::Result<()> {
    use anyhow::Context;

    check_privileged(operation)
        .context(format!("{} requires root privileges", operation))
        .map_err(|e| anyhow::anyhow!("{}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_root_returns_bool() {
        let _ = is_root();
    }

    #[test]
    fn test_check_privileged_non_root() {
        if !is_root() {
            let result = check_privileged("test operation");
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
            assert!(err.to_string().contains("test operation"));
            assert!(err.to_string().contains("sudo"));
        }
    }

    #[test]
    fn test_check_privileged_root() {
        if is_root() {
            let result = check_privileged("test operation");
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_require_root_non_root_fails() {
        if !is_root() {
            let result = require_root("test operation");
            assert!(result.is_err());
            let err = result.unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains("test operation"));
            assert!(msg.contains("root privileges"));
        }
    }

    #[test]
    fn test_require_root_root_succeeds() {
        if is_root() {
            let result = require_root("test operation");
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_is_root_consistent() {
        let first = is_root();
        let second = is_root();
        assert_eq!(first, second);
    }
}
