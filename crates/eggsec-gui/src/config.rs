//! Command-line configuration for the desktop GUI.
//!
//! The GUI is configured by flags only. The workspace has no socket-path
//! environment variable and no XDG lookup today — the default is hardcoded in
//! three independent places — so introducing one here would create a second,
//! differently-named source of truth. Collapsing the duplication is a
//! separate corrective milestone.

use std::path::PathBuf;

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
    /// The daemon executable to spawn and supervise, when the operator
    /// opted in to that lifecycle.
    ///
    /// `None` means the GUI attaches to a daemon the operator started and
    /// never spawns one.
    pub daemon_bin: Option<PathBuf>,
}

impl Default for GuiOptions {
    fn default() -> Self {
        Self {
            socket_path: DEFAULT_SOCKET_PATH.to_owned(),
            daemon_bin: None,
        }
    }
}

impl GuiOptions {
    /// Parse the GUI's command line.
    ///
    /// Accepts `--socket <PATH>`, `--socket=<PATH>`, `--daemon-bin <PATH>`,
    /// and `--daemon-bin=<PATH>`. `--help` and `-h` produce the usage text.
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
                "--daemon-bin" => match args.next() {
                    Some(path) => options.daemon_bin = Some(PathBuf::from(path)),
                    None => {
                        return Err(GuiError::Usage(
                            "--daemon-bin requires a path argument".to_owned(),
                        ))
                    }
                },
                "--help" | "-h" => return Err(GuiError::Usage(usage())),
                other => {
                    if let Some(path) = other.strip_prefix("--socket=") {
                        options.socket_path = path.to_owned();
                    } else if let Some(path) = other.strip_prefix("--daemon-bin=") {
                        options.daemon_bin = Some(PathBuf::from(path));
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
         Usage: eggsec-gui [--socket <PATH>] [--daemon-bin <PATH>]\n\n\
         Options:\n  \
           --socket <PATH>   Path of the running daemon's Unix socket\n                       \
           (default: {DEFAULT_SOCKET_PATH})\n  \
           --daemon-bin <PATH>\n                    \
           The daemon executable to spawn and supervise\n                       \
           (default: none — the GUI never starts a daemon it was not told to)\n  \
           -h, --help        Show this message\n\n\
         The GUI connects to an already-running daemon. Start one with:\n  \
           eggsec-daemon --full-executor --socket-path {DEFAULT_SOCKET_PATH}\n\n\
           The daemon binary must be BUILT with the `service` cargo feature \
           (its deprecated `full-executor` alias selects the same profile); the \
           --full-executor flag alone cannot add an engine that was not \
           compiled in.\n\n\
         With --daemon-bin, the GUI starts that executable itself and \
         supervises it: it waits for the daemon to answer before using it, and \
         it stops the daemon when the GUI exits. The path is used exactly as \
         given — nothing is looked up on PATH, and no daemon is ever started \
         without this option.\n"
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
    fn no_daemon_binary_is_named_by_default() {
        let options = GuiOptions::from_args(args(&[])).expect("no arguments is valid");
        assert!(
            options.daemon_bin.is_none(),
            "the GUI must never spawn a daemon the operator did not name"
        );
    }

    #[test]
    fn the_daemon_binary_flag_is_parsed_in_both_spellings() {
        let spaced = GuiOptions::from_args(args(&["--daemon-bin", "/opt/eggsec/daemon"]))
            .expect("a spaced daemon-bin flag is valid");
        assert_eq!(spaced.daemon_bin, Some(PathBuf::from("/opt/eggsec/daemon")));

        let joined = GuiOptions::from_args(args(&["--daemon-bin=/opt/eggsec/daemon"]))
            .expect("a joined daemon-bin flag is valid");
        assert_eq!(joined.daemon_bin, Some(PathBuf::from("/opt/eggsec/daemon")));
    }

    #[test]
    fn a_missing_daemon_binary_value_is_a_usage_error() {
        let error =
            GuiOptions::from_args(args(&["--daemon-bin"])).expect_err("a value is required");
        assert!(error
            .to_string()
            .contains("--daemon-bin requires a path argument"));
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

    #[test]
    fn the_usage_states_the_default_and_the_opt_in_spawn() {
        let text = usage();
        assert!(
            text.contains("connects to an already-running daemon"),
            "the default must still be stated: {text}"
        );
        assert!(
            text.contains("--daemon-bin"),
            "the opt-in spawn must be described: {text}"
        );
        assert!(
            text.contains("nothing is looked up on PATH"),
            "the spawn route must say that no binary is resolved from PATH: {text}"
        );
    }
}
