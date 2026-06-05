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

pub fn init_logging(format: LogFormat) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let registry = tracing_subscriber::registry().with(filter);

    let result = match format {
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
    };

    if let Err(e) = result {
        eprintln!("Failed to initialize logging: {e}");
    }
}
