use crate::cli::{ConfigArgs, ConfigCommand};
use crate::commands::handlers::CommandContext;
use crate::config::load_config;
use anyhow::Result;

pub async fn handle_config(_ctx: &CommandContext, args: ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCommand::Validate(validate_args) => {
            let config_path = validate_args.config.as_deref();

            load_config(config_path)
                .map_err(|e| anyhow::anyhow!("Configuration validation failed: {}", e))?;
            println!("✓ Configuration is valid");
            println!(
                "  Config file: {}",
                config_path.unwrap_or("~/.config/slapper/slapper.toml")
            );
            Ok(())
        }
        ConfigCommand::Show(show_args) => {
            let config_path = show_args.config.as_deref();

            let config = load_config(config_path)
                .map_err(|e| anyhow::anyhow!("Failed to load configuration: {}", e))?;
            if let Some(ref path) = show_args.config {
                println!("# Configuration from: {}", path);
            } else {
                println!("# Default configuration");
            }
            println!();
            let toml_str = toml::to_string_pretty(&config)
                .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;
            println!("{}", toml_str);
            Ok(())
        }
    }
}
