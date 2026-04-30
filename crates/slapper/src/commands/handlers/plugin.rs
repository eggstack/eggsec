
#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
use anyhow::Result;
#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
use crate::commands::handlers::CommandContext;
#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
use slapper_plugin::Plugin;


#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
pub async fn handle_plugin(_ctx: &CommandContext, args: crate::cli::PluginArgs) -> Result<()> {
    use crate::cli::PluginCommand;

    let plugin_dirs = crate::plugin::PluginManager::default_plugin_dirs(None);

    match &args.command {
        PluginCommand::List(list_args) => {
            let mut found_any = false;

            #[cfg(feature = "python-plugins")]
            {
                let mut manager = crate::plugin::PluginManager::new();
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
                let mut python_mgr = crate::plugin::PythonPluginManager::new();
                let plugin_dirs = crate::plugin::PluginManager::default_plugin_dirs(None);
                for dir in &plugin_dirs {
                    if dir.exists() {
                        if let Err(e) = python_mgr.load_plugins(dir) {
                            tracing::warn!("Failed to load plugins from {:?}: {}", dir, e);
                        }
                    }
                }

                let results = python_mgr.run_check(&run_args.name, &run_args.target).await?;
                if !results.findings.is_empty() {
                    println!("Running Python plugin '{}' against target '{}'", run_args.name, run_args.target);
                    println!("\nPlugin Results:");
                    for finding in &results.findings {
                        println!("  - {:?}", finding);
                    }
                    if let Some(output_file) = &run_args.output {
                        tokio::fs::write(output_file, serde_json::to_string_pretty(&results)?).await?;
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
                    if loader.list_plugins().iter().any(|p| p.name == run_args.name) {
                        println!("Running Ruby plugin '{}' against target '{}'", run_args.name, run_args.target);

                        let result = loader.run_plugin(&run_args.name, &run_args.target, 300)?;

                        println!("\nPlugin Results:");
                        println!("  Success: {}", result.success);
                        if !result.message.is_empty() {
                            println!("  Message: {}", result.message);
                        }
                        if !result.findings.is_empty() {
                            println!("  Findings:");
                            for finding in &result.findings {
                                println!("    [{}] {} - {}", finding.severity, finding.finding_type, finding.description);
                            }
                        }
                        if let Some(ref err) = result.error {
                            println!("  Error: {}", err);
                        }
                        if let Some(output_file) = &run_args.output {
                            tokio::fs::write(output_file, serde_json::to_string_pretty(&result)?).await?;
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
pub fn discover_all_plugins() -> Vec<crate::tui::tabs::plugin::PluginInfo> {
    use crate::tui::tabs::plugin::PluginInfo;

    let plugin_dirs = slapper_plugin::PluginManager::default_plugin_dirs(None);
    let mut all_plugins = Vec::new();

    #[cfg(feature = "python-plugins")]
    {
        let mut manager = slapper_plugin::PluginManager::new();
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
