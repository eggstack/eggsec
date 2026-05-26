
#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
use super::TaskResult;

#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
pub async fn run_load_plugins(
    config_plugins_dir: Option<std::path::PathBuf>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::commands::handlers::plugin::discover_all_plugins;

    let plugins = discover_all_plugins(config_plugins_dir);

    if let Err(e) = result_tx.send(TaskResult::PluginsLoaded(plugins)).await {
        tracing::warn!("Failed to send plugins loaded result: {}", e);
    }

    Ok(())
}

#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
pub async fn run_plugin_check(
    plugin_name: String,
    target: String,
    timeout_secs: u64,
    config_plugins_dir: Option<std::path::PathBuf>,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::tui::tabs::plugin::{Finding, PluginResults};
    use slapper_plugin::Plugin;
    use std::time::Instant;

    if let Err(e) = progress_tx.send((0, 3)).await {
        tracing::warn!("Failed to send progress: {}", e);
    }

    #[cfg(feature = "python-plugins")]
    {
        let mut python_mgr = crate::plugin::PythonPluginManager::with_block_suspicious_plugins(true);
        let plugin_dirs =
            crate::plugin::PluginManager::default_plugin_dirs(config_plugins_dir.clone());
        for dir in &plugin_dirs {
            if dir.exists() {
                python_mgr.load_plugins(dir)?;
            }
        }

        let has_python_check = python_mgr
            .get_checks()
            .iter()
            .any(|check| check.name == plugin_name);

        if has_python_check {
            if let Err(e) = progress_tx.send((1, 3)).await {
                tracing::warn!("Failed to send progress: {}", e);
            }
            let results = tokio::time::timeout(
                std::time::Duration::from_secs(timeout_secs),
                python_mgr.run_check(&plugin_name, &target),
            )
            .await
            .map_err(|_| anyhow::anyhow!("Plugin execution timed out after {} seconds", timeout_secs))??;
            if let Err(e) = progress_tx.send((2, 3)).await {
                tracing::warn!("Failed to send progress: {}", e);
            }

            let mapped = PluginResults {
                plugin_name: plugin_name.clone(),
                target,
                success: results.success,
                findings: results
                    .findings
                    .into_iter()
                    .map(|f| Finding {
                        title: f.title,
                        severity: f.severity,
                        description: f.description,
                        evidence: f.evidence,
                    })
                    .collect(),
                errors: results.errors,
                execution_time_ms: results.execution_time_ms,
            };

            if let Err(e) = progress_tx.send((3, 3)).await {
                tracing::warn!("Failed to send progress: {}", e);
            }
            if let Err(e) = result_tx.send(TaskResult::PluginResult(mapped)).await {
                tracing::warn!("Failed to send plugin result: {}", e);
            }
            return Ok(());
        }
    }

    #[cfg(feature = "ruby-plugins")]
    {
        let plugin_dirs = crate::plugin::PluginManager::default_plugin_dirs(config_plugins_dir);
        if let Ok(mut loader) = crate::ruby::PluginLoader::new(plugin_dirs) {
            let _ = loader.discover_plugins();
            if loader.list_plugins().iter().any(|p| p.name == plugin_name) {
                if let Err(e) = progress_tx.send((1, 3)).await {
                    tracing::warn!("Failed to send progress: {}", e);
                }
                let started = Instant::now();
                let result = loader.run_plugin(&plugin_name, &target, timeout_secs)?;
                if let Err(e) = progress_tx.send((2, 3)).await {
                    tracing::warn!("Failed to send progress: {}", e);
                }

                let mapped = PluginResults {
                    plugin_name: plugin_name.clone(),
                    target,
                    success: result.success,
                    findings: result
                        .findings
                        .into_iter()
                        .map(|f| Finding {
                            title: f.finding_type,
                            severity: f.severity,
                            description: f.description,
                            evidence: f.evidence,
                        })
                        .collect(),
                    errors: result.error.into_iter().collect(),
                    execution_time_ms: started.elapsed().as_millis() as u64,
                };

                if let Err(e) = progress_tx.send((3, 3)).await {
                    tracing::warn!("Failed to send progress: {}", e);
                }
                if let Err(e) = result_tx.send(TaskResult::PluginResult(mapped)).await {
                    tracing::warn!("Failed to send plugin result: {}", e);
                }
                return Ok(());
            }
        }
    }

    anyhow::bail!(
        "Plugin '{}' not found. Use plugin list to see available checks/plugins.",
        plugin_name
    );
}
