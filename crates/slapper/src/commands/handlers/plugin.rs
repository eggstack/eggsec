#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
use crate::commands::handlers::CommandContext;
#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
use anyhow::Result;
#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
use slapper_plugin::Plugin;
#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
use std::path::PathBuf;

#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
pub async fn handle_plugin(_ctx: &CommandContext, args: crate::cli::PluginArgs) -> Result<()> {
    use crate::cli::PluginCommand;
    #[cfg(feature = "python-plugins")]
    use crate::plugin::PluginConfig;

    let plugin_dirs =
        crate::plugin::PluginManager::default_plugin_dirs(_ctx.config.paths.plugins_dir.clone());

    match &args.command {
        PluginCommand::List(list_args) => {
            let mut found_any = false;

            #[cfg(feature = "python-plugins")]
            {
                let mut manager =
                    crate::plugin::PluginManager::with_config_dir(_ctx.config.paths.plugins_dir.clone());
                manager.discover_plugins();
                for info in manager.list_plugins() {
                    found_any = true;
                    println!("  {} (v{}) [Python]", info.name, info.version);
                    if list_args.verbose {
                        if !info.author.is_empty() {
                            println!("    Author: {}", info.author);
                        }
                        if !info.description.is_empty() {
                            println!("    {}", info.description);
                        }
                    }
                }
            }

            #[cfg(feature = "ruby-plugins")]
            {
                if let Ok(mut loader) = crate::ruby::PluginLoader::new(plugin_dirs.clone()) {
                    if let Ok(discovered) = loader.discover_plugins() {
                        for plugin in &discovered {
                            found_any = true;
                            println!("  {} (v{}) [Ruby]", plugin.name, plugin.version);
                            if list_args.verbose {
                                if let Some(ref author) = plugin.author {
                                    if !author.is_empty() {
                                        println!("    Author: {}", author);
                                    }
                                }
                                if let Some(ref desc) = plugin.description {
                                    if !desc.is_empty() {
                                        println!("    {}", desc);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !found_any {
                println!("No plugins found.");
                println!("Add plugins to: ~/.config/slapper/plugins/ or ./plugins/");
            }
        }
        PluginCommand::Run(run_args) => {
            // Try Python plugins first
            #[cfg(feature = "python-plugins")]
            {
                let plugin_config = PluginConfig {
                    block_suspicious_plugins: true,
                    timeout_secs: _ctx.config.http.timeout_secs.min(300),
                    ..PluginConfig::default()
                };
                let mut python_mgr = crate::plugin::PythonPluginManager::from_config(&plugin_config);
                for dir in &plugin_dirs {
                    if dir.exists() {
                        if let Err(e) = python_mgr.load_plugins(dir) {
                            tracing::warn!("Failed to load plugins from {:?}: {}", dir, e);
                        }
                    }
                }

                let has_python_check = python_mgr
                    .get_checks()
                    .iter()
                    .any(|check| check.name == run_args.name);

                if has_python_check {
                    let results = tokio::time::timeout(
                        std::time::Duration::from_secs(plugin_config.timeout_secs),
                        python_mgr.run_check(&run_args.name, &run_args.target),
                    )
                    .await
                    .map_err(|_| {
                        anyhow::anyhow!(
                            "Plugin execution timed out after {} seconds",
                            plugin_config.timeout_secs
                        )
                    })??;
                    println!(
                        "Running Python plugin '{}' against target '{}'",
                        run_args.name, run_args.target
                    );
                    println!("\nPlugin Results:");
                    for finding in &results.findings {
                        println!("  - {:?}", finding);
                    }
                    for error in &results.errors {
                        println!("  Error: {}", error);
                    }
                    if let Some(output_file) = &run_args.output {
                        tokio::fs::write(output_file, serde_json::to_string_pretty(&results)?)
                            .await?;
                        println!("\nResults written to: {}", output_file);
                    }
                    return Ok(());
                }
            }

            // Try Ruby plugins
            #[cfg(feature = "ruby-plugins")]
            {
                if let Ok(mut loader) = crate::ruby::PluginLoader::new(plugin_dirs) {
                    let _ = loader.discover_plugins();
                    if loader
                        .list_plugins()
                        .iter()
                        .any(|p| p.name == run_args.name)
                    {
                        println!(
                            "Running Ruby plugin '{}' against target '{}'",
                            run_args.name, run_args.target
                        );

                        let result = loader.run_plugin(
                            &run_args.name,
                            &run_args.target,
                            _ctx.config.http.timeout_secs.min(300),
                        )?;

                        println!("\nPlugin Results:");
                        println!("  Success: {}", result.success);
                        if !result.message.is_empty() {
                            println!("  Message: {}", result.message);
                        }
                        if !result.findings.is_empty() {
                            println!("  Findings:");
                            for finding in &result.findings {
                                println!(
                                    "    [{}] {} - {}",
                                    finding.severity, finding.finding_type, finding.description
                                );
                            }
                        }
                        if let Some(ref err) = result.error {
                            println!("  Error: {}", err);
                        }
                        if let Some(output_file) = &run_args.output {
                            tokio::fs::write(output_file, serde_json::to_string_pretty(&result)?)
                                .await?;
                            println!("\nResults written to: {}", output_file);
                        }
                        return Ok(());
                    }
                }
            }

            anyhow::bail!(
                "Plugin '{}' not found. Use 'slapper plugin list' to see available plugins.",
                run_args.name
            );
        }
    }

    Ok(())
}

#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
pub fn discover_all_plugins(
    config_plugins_dir: Option<PathBuf>,
) -> Vec<crate::tui::tabs::plugin::PluginInfo> {
    use crate::tui::tabs::plugin::PluginInfo;

    #[cfg(feature = "ruby-plugins")]
    let plugin_dirs = slapper_plugin::PluginManager::default_plugin_dirs(config_plugins_dir.clone());
    let mut all_plugins = Vec::new();

    #[cfg(feature = "python-plugins")]
    {
        let mut manager = slapper_plugin::PluginManager::with_config_dir(config_plugins_dir);
        let discovered = manager.discover_plugins();
        for info in discovered {
            all_plugins.push(PluginInfo::from(info));
        }
    }

    #[cfg(feature = "ruby-plugins")]
    {
        if let Ok(mut loader) = crate::ruby::PluginLoader::new(plugin_dirs) {
            if let Ok(discovered) = loader.discover_plugins() {
                for plugin in discovered {
                    all_plugins.push(PluginInfo {
                        name: plugin.name,
                        version: plugin.version,
                        description: plugin.description.unwrap_or_default(),
                        author: plugin.author.unwrap_or_default(),
                        tags: vec![],
                        language: "Ruby".to_string(),
                    });
                }
            }
        }
    }

    all_plugins
}
