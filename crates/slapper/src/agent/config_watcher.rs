use anyhow::Result;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

pub trait ConfigReloader: Send + Sync {
    fn reload(&self, path: &Path) -> Result<()>;
}

pub struct ConfigWatcher {
    watcher: Debouncer<RecommendedWatcher>,
}

impl ConfigWatcher {
    pub fn new<P: AsRef<Path>>(
        config_paths: Vec<P>,
        reloader: Arc<dyn ConfigReloader>,
    ) -> Result<Self> {
        let (tx, mut rx) = mpsc::channel(100);

        let watcher = new_debouncer(Duration::from_secs(1), move |res: DebounceEventResult| {
            if let Err(e) = tx.blocking_send(res) {
                tracing::error!("Failed to send debounced event: {}", e);
            }
        })?;

        let mut watcher = watcher;

        for path in &config_paths {
            let path = path.as_ref();
            if path.exists() {
                watcher.watcher().watch(path, RecursiveMode::NonRecursive)?;
                tracing::debug!("Watching config file: {:?}", path);
            }
        }

        let reloader_clone = reloader;
        tokio::spawn(async move {
            while let Some(result) = rx.recv().await {
                match result {
                    Ok(events) => {
                        for event in events {
                            if matches!(event.kind, notify_debouncer_mini::DebouncedEventKind::Any) {
                                tracing::info!("Config file changed: {:?}", event.path);
                                if let Err(e) = reloader_clone.reload(&event.path) {
                                    tracing::error!("Failed to reload config: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Watch error: {:?}", e);
                    }
                }
            }
        });

        Ok(Self { watcher })
    }

    pub fn watch<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.watcher
            .watcher()
            .watch(path.as_ref(), RecursiveMode::NonRecursive)?;
        tracing::debug!("Now watching: {:?}", path.as_ref());
        Ok(())
    }
}

pub struct SlapperConfigReloader {
    portfolio_path: Option<PathBuf>,
    config_path: Option<PathBuf>,
}

impl SlapperConfigReloader {
    pub fn new(portfolio_path: Option<PathBuf>, config_path: Option<PathBuf>) -> Self {
        Self {
            portfolio_path,
            config_path,
        }
    }
}

impl ConfigReloader for SlapperConfigReloader {
    fn reload(&self, path: &Path) -> Result<()> {
        if let Some(ref portfolio_path) = self.portfolio_path {
            if path == portfolio_path {
                tracing::info!("Portfolio config changed, reload requested");
                return Ok(());
            }
        }
        if let Some(ref config_path) = self.config_path {
            if path == config_path {
                tracing::info!("Main config changed, reload requested");
                return Ok(());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slapper_config_reloader() {
        let portfolio_path = PathBuf::from("/tmp/portfolio.json");
        let config_path = PathBuf::from("/tmp/slapper.toml");

        let reloader = SlapperConfigReloader::new(Some(portfolio_path.clone()), Some(config_path.clone()));

        assert!(reloader.reload(&portfolio_path).is_ok());
        assert!(reloader.reload(&config_path).is_ok());
        assert!(reloader.reload(&PathBuf::from("/tmp/other.toml")).is_ok());
    }
}
