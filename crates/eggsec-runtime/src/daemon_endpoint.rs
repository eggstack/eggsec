//! The daemon's endpoint: one default, one environment override, one rule.
//!
//! The daemon listens on a Unix socket. Which path it listens on is a
//! *deployment* fact, and this module is the single place in the workspace
//! that states it:
//!
//! - [`DEFAULT_DAEMON_SOCKET_PATH`] is the built-in default.
//! - [`DAEMON_SOCKET_ENV_VAR`] is the environment variable that overrides it.
//! - [`resolve_daemon_socket_path`] is the precedence rule, as a pure
//!   function.
//! - [`daemon_socket_path_from_env`] is the only function in the workspace
//!   that reads the environment.
//!
//! The precedence is **explicit flag → environment → built-in default**, and it
//! is applied here rather than at the call sites, so that the daemon binary,
//! the CLI, the TUI and the GUI cannot disagree about the path.
//!
//! The resolver is deliberately pure: it takes the raw environment value and
//! returns the resolved one, and never reads the process environment. That is
//! what makes the precedence testable without mutating a process-global, and
//! it is why [`daemon_socket_path_from_env`] — the only reader — is a separate
//! function.

/// The environment variable that overrides the daemon's default socket path.
///
/// The name is the one the repository already used in
/// `scripts/build-daemon-profile.sh`, so adopting it turns that script's line
/// from a promise the binaries did not keep into a fact. It is **not**
/// `EGGSEC_SOCKET`, which is named and rejected in
/// `docs/GUI_DISTRIBUTION_HANDOFF.md`.
pub const DAEMON_SOCKET_ENV_VAR: &str = "EGGSEC_DAEMON_SOCKET";

/// The daemon's built-in default socket path.
///
/// This is the one place in the workspace's executable surface that states the
/// literal. Every surface resolves to it through
/// [`resolve_daemon_socket_path`].
pub const DEFAULT_DAEMON_SOCKET_PATH: &str = "/tmp/eggsec-daemon.sock";

/// Resolve a raw environment value into the socket path to use.
///
/// The rule, in one place:
///
/// | Input | Output |
/// |---|---|
/// | `None` (the variable is unset) | [`DEFAULT_DAEMON_SOCKET_PATH`] |
/// | `Some("")` (set but empty) | [`DEFAULT_DAEMON_SOCKET_PATH`] |
/// | `Some("   ")` (whitespace only) | [`DEFAULT_DAEMON_SOCKET_PATH`] |
/// | `Some("/run/user/1000/eggsec.sock")` | that value, verbatim |
/// | `Some(" /run/user/1000/eggsec.sock ")` | the **trimmed** value |
///
/// An empty or whitespace-only value is **absent**, not an empty path: a
/// daemon must never be asked to bind the empty path. The trim is part of the
/// rule, so it lives here and not at the call sites.
///
/// This function is pure — it never reads the process environment.
pub fn resolve_daemon_socket_path(env_value: Option<&str>) -> &str {
    match env_value {
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                DEFAULT_DAEMON_SOCKET_PATH
            } else {
                trimmed
            }
        }
        None => DEFAULT_DAEMON_SOCKET_PATH,
    }
}

/// Resolve the daemon's socket path from the process environment.
///
/// This is the only function in the workspace that reads
/// [`DAEMON_SOCKET_ENV_VAR`]. The surfaces call it; they do not read the
/// environment themselves.
pub fn daemon_socket_path_from_env() -> String {
    resolve_daemon_socket_path(std::env::var(DAEMON_SOCKET_ENV_VAR).ok().as_deref()).to_owned()
}

#[cfg(test)]
mod tests {
    use super::{resolve_daemon_socket_path, DAEMON_SOCKET_ENV_VAR, DEFAULT_DAEMON_SOCKET_PATH};

    #[test]
    fn an_absent_variable_resolves_to_the_built_in_default() {
        assert_eq!(resolve_daemon_socket_path(None), DEFAULT_DAEMON_SOCKET_PATH);
    }

    #[test]
    fn an_empty_value_resolves_to_the_built_in_default() {
        assert_eq!(
            resolve_daemon_socket_path(Some("")),
            DEFAULT_DAEMON_SOCKET_PATH
        );
    }

    #[test]
    fn a_whitespace_only_value_resolves_to_the_built_in_default() {
        assert_eq!(
            resolve_daemon_socket_path(Some("   \t ")),
            DEFAULT_DAEMON_SOCKET_PATH
        );
    }

    #[test]
    fn an_explicit_value_resolves_verbatim() {
        assert_eq!(
            resolve_daemon_socket_path(Some("/run/user/1000/eggsec.sock")),
            "/run/user/1000/eggsec.sock"
        );
    }

    #[test]
    fn an_explicit_value_is_trimmed() {
        assert_eq!(
            resolve_daemon_socket_path(Some(" /run/user/1000/eggsec.sock ")),
            "/run/user/1000/eggsec.sock"
        );
    }

    #[test]
    fn the_default_path_value_is_pinned() {
        // The one place in the workspace that may state the literal.
        assert_eq!(DEFAULT_DAEMON_SOCKET_PATH, "/tmp/eggsec-daemon.sock");
    }

    #[test]
    fn the_environment_variable_name_is_pinned() {
        assert_eq!(DAEMON_SOCKET_ENV_VAR, "EGGSEC_DAEMON_SOCKET");
    }
}
