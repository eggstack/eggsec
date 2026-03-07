#![cfg(feature = "stress-testing")]

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
