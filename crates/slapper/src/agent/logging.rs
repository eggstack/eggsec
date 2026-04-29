use std::path::PathBuf;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub struct AgentLogger {
    _guard: tracing_appender::non_blocking::WorkerGuard,
}

impl AgentLogger {
    pub fn init(log_dir: PathBuf) -> anyhow::Result<Self> {
        std::fs::create_dir_all(&log_dir)?;

        let file_appender = RollingFileAppender::new(
            Rotation::DAILY,
            &log_dir,
            "agent.log",
        );

        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let file_layer = fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .json();

        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info,slapper=debug"));

        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .init();

        tracing::info!("Agent logger initialized with log directory: {:?}", log_dir);

        Ok(Self { _guard: guard })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_agent_logger_init() {
        let temp_dir = tempdir().unwrap();
        let log_dir = temp_dir.path().join("logs");
        let logger = AgentLogger::init(log_dir.clone());
        assert!(logger.is_ok());
    }
}
