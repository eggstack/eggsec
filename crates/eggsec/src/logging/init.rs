use std::path::PathBuf;

use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

#[derive(Debug, Clone, Copy, Default)]
pub enum LogFormat {
    #[default]
    Pretty,
    Json,
    Compact,
}

/// Console emission policy for the logging initialization boundary.
///
/// The rich TUI owns the alternate screen while active, so Ratatui/Crossterm
/// must be the only writer to the controlling terminal. Rich TUI mode therefore
/// uses `Disabled`: no stdout/stderr formatter is constructed. Tracing call
/// sites remain valid diagnostics; they simply emit no terminal bytes.
///
/// Non-TUI surfaces (CLI, CI, daemon console) use `Enabled` and keep the
/// existing console formatter behavior.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ConsoleLogging {
    #[default]
    Enabled,
    Disabled,
}

/// Pure policy selection: rich TUI launch intent maps to no-console output.
///
/// This is the single decision point the process host uses before subscriber
/// construction. Tested without touching the global tracing subscriber.
pub fn resolve_console_logging(is_rich_tui_launch: bool) -> ConsoleLogging {
    if is_rich_tui_launch {
        ConsoleLogging::Disabled
    } else {
        ConsoleLogging::Enabled
    }
}

/// Whether the given policy includes a console formatting layer.
///
/// Production `init_logging_with_console` consults this predicate before
/// constructing any stdout/stderr `fmt::layer()`. Tests use the same predicate
/// to build injectable-writer subscribers and prove emission behavior.
pub fn console_layer_enabled(console: ConsoleLogging) -> bool {
    matches!(console, ConsoleLogging::Enabled)
}

pub fn init_logging(
    format: LogFormat,
    log_dir: Option<PathBuf>,
) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    init_logging_with_console(format, log_dir, ConsoleLogging::Enabled)
}

pub fn init_logging_with_console(
    format: LogFormat,
    log_dir: Option<PathBuf>,
    console: ConsoleLogging,
) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let registry = tracing_subscriber::registry().with(filter);

    let mut guard = None;

    let result = match log_dir {
        Some(dir) => {
            if let Err(e) = std::fs::create_dir_all(&dir) {
                eprintln!("Failed to create log directory {}: {e}", dir.display());
            }
            let file_appender = tracing_appender::rolling::RollingFileAppender::new(
                tracing_appender::rolling::Rotation::DAILY,
                &dir,
                "agent.log",
            );
            let (non_blocking, g) = tracing_appender::non_blocking(file_appender);
            guard = Some(g);

            let file_layer = fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .json();

            if !console_layer_enabled(console) {
                // TUI mode with explicit log directory: file layer only, no
                // console formatter. No new default persistent TUI log
                // directory is created here; `log_dir` comes from an existing
                // caller (agent memory dir).
                registry.with(file_layer).try_init()
            } else {
                match format {
                    LogFormat::Json => registry
                        .with(file_layer)
                        .with(
                            fmt::layer()
                                .json()
                                .with_span_events(FmtSpan::CLOSE)
                                .with_target(true)
                                .with_thread_ids(true)
                                .with_thread_names(true),
                        )
                        .try_init(),
                    LogFormat::Compact => registry
                        .with(file_layer)
                        .with(
                            fmt::layer()
                                .compact()
                                .with_target(true)
                                .with_thread_ids(false)
                                .with_line_number(true),
                        )
                        .try_init(),
                    LogFormat::Pretty => registry
                        .with(file_layer)
                        .with(
                            fmt::layer()
                                .pretty()
                                .with_target(true)
                                .with_thread_ids(false)
                                .with_line_number(true),
                        )
                        .try_init(),
                }
            }
        }
        None => {
            if !console_layer_enabled(console) {
                // TUI mode without a log directory: install the filtered
                // registry with no formatting writer so tracing call sites
                // remain valid but emit no terminal bytes. Stdout *and*
                // stderr are both forbidden as side channels while the
                // alternate screen is owned.
                registry.try_init()
            } else {
                match format {
                    LogFormat::Json => registry
                        .with(
                            fmt::layer()
                                .json()
                                .with_span_events(FmtSpan::CLOSE)
                                .with_target(true)
                                .with_thread_ids(true)
                                .with_thread_names(true),
                        )
                        .try_init(),
                    LogFormat::Compact => registry
                        .with(
                            fmt::layer()
                                .compact()
                                .with_target(true)
                                .with_thread_ids(false)
                                .with_line_number(true),
                        )
                        .try_init(),
                    LogFormat::Pretty => registry
                        .with(
                            fmt::layer()
                                .pretty()
                                .with_target(true)
                                .with_thread_ids(false)
                                .with_line_number(true),
                        )
                        .try_init(),
                }
            }
        }
    };

    if let Err(e) = result {
        eprintln!("Failed to initialize logging: {e}");
    }

    guard
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::layer::SubscriberExt;

    #[derive(Clone, Default)]
    struct SharedCapture {
        buf: Arc<Mutex<Vec<u8>>>,
    }

    impl SharedCapture {
        fn new() -> (Self, Arc<Mutex<Vec<u8>>>) {
            let buf = Arc::new(Mutex::new(Vec::new()));
            (Self { buf: buf.clone() }, buf)
        }

        fn contents(buf: &Arc<Mutex<Vec<u8>>>) -> String {
            String::from_utf8_lossy(&buf.lock().expect("capture lock")).into_owned()
        }
    }

    impl Write for SharedCapture {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.buf
                .lock()
                .expect("capture lock")
                .extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SharedCapture {
        type Writer = SharedCapture;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    /// Build a local (non-global) subscriber mirroring the production
    /// console/file branching, but with injectable in-memory writers.
    ///
    /// `with_file` simulates `log_dir.is_some()`: when true a JSON file layer
    /// with the given writer is attached. The console layer is attached only
    /// when `console_layer_enabled(console)` holds, exactly as production does.
    fn build_test_subscriber(
        console: ConsoleLogging,
        with_file: bool,
        console_writer: SharedCapture,
        file_writer: SharedCapture,
    ) -> tracing::Dispatch {
        let filter = EnvFilter::new("info");
        let registry = tracing_subscriber::registry().with(filter);

        if with_file {
            let file_layer = fmt::layer()
                .with_writer(file_writer)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .json();
            if console_layer_enabled(console) {
                let console_layer = fmt::layer()
                    .pretty()
                    .with_writer(console_writer)
                    .with_target(true)
                    .with_thread_ids(false)
                    .with_line_number(true);
                let subscriber = registry.with(file_layer).with(console_layer);
                tracing::Dispatch::new(subscriber)
            } else {
                let subscriber = registry.with(file_layer);
                tracing::Dispatch::new(subscriber)
            }
        } else if console_layer_enabled(console) {
            let console_layer = fmt::layer()
                .pretty()
                .with_writer(console_writer)
                .with_target(true)
                .with_thread_ids(false)
                .with_line_number(true);
            let subscriber = registry.with(console_layer);
            tracing::Dispatch::new(subscriber)
        } else {
            // No console, no file: filtered registry with no formatting writer.
            let subscriber = registry;
            tracing::Dispatch::new(subscriber)
        }
    }

    fn emit_probe(dispatcher: &tracing::Dispatch, message: &str) {
        tracing::dispatcher::with_default(dispatcher, || {
            tracing::info!("{}", message);
        });
    }

    #[test]
    fn resolve_console_logging_maps_tui_launch_to_disabled() {
        assert_eq!(resolve_console_logging(true), ConsoleLogging::Disabled);
        assert_eq!(resolve_console_logging(false), ConsoleLogging::Enabled);
    }

    #[test]
    fn console_layer_enabled_matches_policy() {
        assert!(console_layer_enabled(ConsoleLogging::Enabled));
        assert!(!console_layer_enabled(ConsoleLogging::Disabled));
    }

    #[test]
    fn console_enabled_no_file_emits_to_console() {
        let (console_writer, console_buf) = SharedCapture::new();
        let (file_writer, _file_buf) = SharedCapture::new();
        let dispatcher =
            build_test_subscriber(ConsoleLogging::Enabled, false, console_writer, file_writer);
        emit_probe(&dispatcher, "phase-a-probe-console-only");
        assert!(
            SharedCapture::contents(&console_buf).contains("phase-a-probe-console-only"),
            "console-enabled/no-file must emit to the console writer"
        );
    }

    #[test]
    fn console_enabled_with_file_emits_to_both() {
        let (console_writer, console_buf) = SharedCapture::new();
        let (file_writer, file_buf) = SharedCapture::new();
        let dispatcher =
            build_test_subscriber(ConsoleLogging::Enabled, true, console_writer, file_writer);
        emit_probe(&dispatcher, "phase-a-probe-both-layers");
        assert!(
            SharedCapture::contents(&console_buf).contains("phase-a-probe-both-layers"),
            "console-enabled/file must emit to the console writer"
        );
        assert!(
            SharedCapture::contents(&file_buf).contains("phase-a-probe-both-layers"),
            "console-enabled/file must emit to the file writer"
        );
    }

    #[test]
    fn console_disabled_no_file_emits_no_terminal_bytes() {
        let (console_writer, console_buf) = SharedCapture::new();
        let (file_writer, _file_buf) = SharedCapture::new();
        // The console capture is intentionally *not* attached when disabled;
        // emitting must leave it empty and must not fail.
        let dispatcher =
            build_test_subscriber(ConsoleLogging::Disabled, false, console_writer, file_writer);
        emit_probe(&dispatcher, "phase-a-probe-no-console-no-file");
        assert!(
            SharedCapture::contents(&console_buf).is_empty(),
            "console-disabled/no-file must leave the console capture empty"
        );
    }

    #[test]
    fn console_disabled_with_file_is_file_only() {
        let (console_writer, console_buf) = SharedCapture::new();
        let (file_writer, file_buf) = SharedCapture::new();
        let dispatcher =
            build_test_subscriber(ConsoleLogging::Disabled, true, console_writer, file_writer);
        emit_probe(&dispatcher, "phase-a-probe-file-only");
        // File-only claim requires an observed event in the file writer while
        // the console capture remains empty.
        assert!(
            SharedCapture::contents(&file_buf).contains("phase-a-probe-file-only"),
            "console-disabled/file must emit to the file writer"
        );
        assert!(
            SharedCapture::contents(&console_buf).is_empty(),
            "console-disabled/file must leave the console capture empty"
        );
    }
}
