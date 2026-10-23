//! Command-line configuration for the desktop GUI.
//!
//! The GUI is configured by flags only. The workspace has no socket-path
//! environment variable and no XDG lookup today — the default is hardcoded in
//! three independent places — so introducing one here would create a second,
//! differently-named source of truth. Collapsing the duplication is a
//! separate corrective milestone.

use crate::GuiError;

/// The default path of the daemon's Unix socket.
///
/// This matches the three existing defaults in the workspace:
/// `crates/eggsec/src/cli/mod.rs` (the global `--socket` flag),
/// `crates/eggsec-daemon/src/config.rs` (`DaemonConfig::default`), and
/// `crates/eggsec-daemon/src/main.rs` (the binary's `--socket-path` flag).
pub const DEFAULT_SOCKET_PATH: &str = "/tmp/eggsec-daemon.sock";

/// The options the GUI was started with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuiOptions {
    /// Path of the daemon's Unix socket.
    pub socket_path: String,
}

impl Default for GuiOptions {
    fn default() -> Self {
        Self {
            socket_path: DEFAULT_SOCKET_PATH.to_owned(),
        }
    }
}

impl GuiOptions {
    /// Parse the GUI's command line.
    ///
    /// Accepts `--socket <PATH>` and `--socket=<PATH>`. `--help` and `-h`
    /// produce the usage text.
    pub fn from_args<I>(args: I) -> Result<Self, GuiError>
    where
        I: IntoIterator<Item = String>,
    {
        let mut options = GuiOptions::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--socket" => match args.next() {
                    Some(path) => options.socket_path = path,
                    None => {
                        return Err(GuiError::Usage(
                            "--socket requires a path argument".to_owned(),
                        ))
                    }
                },
                "--help" | "-h" => return Err(GuiError::Usage(usage())),
                other => {
                    if let Some(path) = other.strip_prefix("--socket=") {
                        options.socket_path = path.to_owned();
                    } else {
                        return Err(GuiError::Usage(format!(
                            "unknown argument: {other}\n\n{}",
                            usage()
                        )));
                    }
                }
            }
        }
        if options.socket_path.is_empty() {
            return Err(GuiError::Usage(
                "--socket requires a non-empty path".to_owned(),
            ));
        }
        Ok(options)
    }
}

/// The GUI's usage text.
pub fn usage() -> String {
    format!(
        "eggsec-gui — the Eggsec desktop frontend\n\n\
         Usage: eggsec-gui [--socket <PATH>]\n\n\
         Options:\n  \
           --socket <PATH>   Path of the running daemon's Unix socket\n                       \
           (default: {DEFAULT_SOCKET_PATH})\n  \
           -h, --help        Show this message\n\n\
         The GUI connects to an already-running daemon. Start one with:\n  \
           eggsec-daemon --full-executor --socket-path {DEFAULT_SOCKET_PATH}\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn default_socket_path_matches_the_workspace_defaults() {
        let options = GuiOptions::from_args(args(&[])).expect("no arguments is valid");
        assert_eq!(options.socket_path, DEFAULT_SOCKET_PATH);
        assert_eq!(options.socket_path, "/tmp/eggsec-daemon.sock");
    }

    #[test]
    fn socket_flag_is_parsed_in_both_spellings() {
        let spaced = GuiOptions::from_args(args(&["--socket", "/run/eggsec.sock"]))
            .expect("a spaced socket flag is valid");
        assert_eq!(spaced.socket_path, "/run/eggsec.sock");

        let joined = GuiOptions::from_args(args(&["--socket=/run/eggsec.sock"]))
            .expect("a joined socket flag is valid");
        assert_eq!(joined.socket_path, "/run/eggsec.sock");
    }

    #[test]
    fn missing_socket_value_is_a_usage_error() {
        let error = GuiOptions::from_args(args(&["--socket"])).expect_err("a value is required");
        assert!(error
            .to_string()
            .contains("--socket requires a path argument"));
    }

    #[test]
    fn empty_socket_value_is_a_usage_error() {
        let error = GuiOptions::from_args(args(&["--socket="])).expect_err("a value is required");
        assert!(error.to_string().contains("non-empty path"));
    }

    #[test]
    fn unknown_argument_is_a_usage_error() {
        let error = GuiOptions::from_args(args(&["--nope"])).expect_err("unknown flags are errors");
        assert!(error.to_string().contains("unknown argument: --nope"));
    }

    #[test]
    fn usage_names_the_daemon_start_command() {
        let text = usage();
        assert!(text.contains("eggsec-daemon --full-executor"));
        // A default-built daemon hosts a no-op executor that rejects every
        // task, so the usage text must not point at the plain daemon.
        assert!(!text.contains("eggsec daemon start"));
    }
}
