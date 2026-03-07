#![allow(dead_code)]

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

#[derive(Debug, Clone, Copy, Default)]
pub enum LogLevel {
    #[default]
    Info,
    Debug,
    Trace,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "trace"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(format!("Unknown log level: {}", s)),
        }
    }
}

pub fn init_logging(level: LogLevel, format: LogFormat, json_output: bool) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level.to_string()));

    let registry = tracing_subscriber::registry().with(filter);

    if json_output || matches!(format, LogFormat::Json) {
        registry
            .with(
                fmt::layer()
                    .json()
                    .with_span_events(FmtSpan::CLOSE)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_thread_names(true),
            )
            .init();
    } else {
        registry
            .with(
                fmt::layer()
                    .pretty()
                    .with_target(true)
                    .with_thread_ids(false)
                    .with_line_number(true),
            )
            .init();
    }
}

#[macro_export]
macro_rules! log_request {
    ($method:expr_2021, $url:expr_2021, $status:expr_2021, $duration_ms:expr_2021) => {
        tracing::info!(
            method = %$method,
            url = %$url,
            status = $status,
            duration_ms = $duration_ms,
            "HTTP request completed"
        );
    };
}

#[macro_export]
macro_rules! log_scan_progress {
    ($stage:expr_2021, $current:expr_2021, $total:expr_2021) => {
        tracing::debug!(
            stage = %$stage,
            current = $current,
            total = $total,
            progress_pct = ($current as f64 / $total as f64 * 100.0),
            "Scan progress"
        );
    };
}

#[macro_export]
macro_rules! log_finding {
    ($severity:expr_2021, $finding_type:expr_2021, $target:expr_2021, $description:expr_2021) => {
        tracing::warn!(
            severity = %$severity,
            finding_type = %$finding_type,
            target = %$target,
            description = %$description,
            "Security finding"
        );
    };
}

#[macro_export]
macro_rules! log_error_context {
    ($error:expr_2021, $context:expr_2021) => {
        tracing::error!(
            error = %$error,
            context = %$context,
            "Operation failed"
        );
    };
}
