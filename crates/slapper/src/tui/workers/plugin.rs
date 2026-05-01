use crate::tui::workers::TaskResult;

#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
pub async fn run_load_plugins(
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::commands::handlers::plugin::discover_all_plugins;

    let plugins = discover_all_plugins();

    let _ = result_tx.send(TaskResult::PluginsLoaded(plugins)).await;

    Ok(())
}
