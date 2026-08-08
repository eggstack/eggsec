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

pub fn init_logging(
    format: LogFormat,
    log_dir: Option<PathBuf>,
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
        None => match format {
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
        },
    };

    if let Err(e) = result {
        eprintln!("Failed to initialize logging: {e}");
    }

    guard
}
