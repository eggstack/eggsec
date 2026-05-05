use crate::cli::{ConfigArgs, ConfigCommand};
use crate::commands::handlers::CommandContext;
use crate::config::load_config;
use anyhow::Result;

pub async fn handle_config(_ctx: &CommandContext, args: ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCommand::Validate(validate_args) => {
            let config_path = validate_args.config.as_deref();

            match load_config(config_path) {
                Ok(_) => {
                    println!("✓ Configuration is valid");
                    println!(
                        "  Config file: {}",
                        config_path.unwrap_or("~/.config/slapper/config.toml")
                    );
                }
                Err(e) => {
                    eprintln!("✗ Configuration validation failed:");
                    eprintln!("  {}", e);
                    std::process::exit(1);
                }
            }
        }
        ConfigCommand::Show(show_args) => {
            let config_path = show_args.config.as_deref();

            match load_config(config_path) {
                Ok(config) => {
                    if let Some(ref path) = show_args.config {
                        println!("# Configuration from: {}", path);
                    } else {
                        println!("# Default configuration");
                    }
                    println!();
                    match toml::to_string_pretty(&config) {
                        Ok(toml_str) => println!("{}", toml_str),
                        Err(e) => eprintln!("Failed to serialize config: {}", e),
                    }
                }
                Err(e) => {
                    eprintln!("✗ Failed to load configuration: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
    Ok(())
}
