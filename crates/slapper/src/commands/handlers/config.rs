use crate::cli::ConfigArgs;
use crate::commands::handlers::CommandContext;
use crate::config::loader;
use anyhow::Result;

pub async fn handle_config(_ctx: &CommandContext, args: ConfigArgs) -> Result<()> {
    use crate::cli::ConfigCommand;

    match args.command {
        ConfigCommand::Validate(validate_args) => {
            let config_path = validate_args.config.as_deref();

            match loader::load_config(config_path) {
                Ok(config) => {
                    println!("Configuration is valid");
                    println!("  Config file: {:?}", config_path
                        .map(String::from)
                        .or_else(|| loader::find_config_file(None).map(|p: std::path::PathBuf| p.to_string_lossy().to_string()))
                        .unwrap_or_else(|| "default".to_string()));
                    println!("  HTTP timeout: {}s", config.http.timeout_secs);
                    println!("  Scan concurrency: {}", config.scan.default_concurrency);
                    println!("  Port timeout: {}s", config.scan.port_timeout_secs);
                    Ok(())
                }
                Err(e) => {
                    println!("Configuration validation failed:");
                    println!("  Error: {}", e);
                    anyhow::bail!("Invalid configuration");
                }
            }
        }
        ConfigCommand::Show(show_args) => {
            if show_args.defaults {
                let default_config = crate::config::SlapperConfig::default();
                println!("Default Configuration:");
                println!();
                println!("{}", serde_json::to_string_pretty(&default_config).unwrap_or_default());
            } else {
                match loader::load_config(None) {
                    Ok(config) => {
                        println!("Effective Configuration:");
                        println!();
                        println!("{}", serde_json::to_string_pretty(&config).unwrap_or_default());
                    }
                    Err(e) => {
                        println!("Could not load configuration: {}", e);
                        println!("\nUsing defaults:");
                        let default_config = crate::config::SlapperConfig::default();
                        println!("{}", serde_json::to_string_pretty(&default_config).unwrap_or_default());
                    }
                }
            }
            Ok(())
        }
    }
}