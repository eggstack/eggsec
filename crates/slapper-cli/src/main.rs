use std::path::PathBuf;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::Shell;

use slapper::cli::Cli;
use slapper::logging::{init_logging, LogFormat};

fn generate_shell_completion(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "slapper", &mut std::io::stdout());
    Ok(())
}

fn agent_log_dir(cli: &Cli) -> Option<PathBuf> {
    #[cfg(feature = "rest-api")]
    {
        if let Some(slapper::cli::Commands::Agent(ref args)) = cli.command {
            let memory_dir = shellexpand::tilde(&args.memory_dir);
            return Some(PathBuf::from(memory_dir.as_ref()).join("logs"));
        }
    }
    let _ = &cli;
    None
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.generate_config {
        println!("{}", slapper::config::get_default_config());
        return Ok(());
    }

    if let Some(shell) = cli.generate_shell_completion {
        generate_shell_completion(shell)?;
        return Ok(());
    }

    let log_dir = agent_log_dir(&cli);
    let _guard = init_logging(
        if cli.json {
            LogFormat::Json
        } else {
            LogFormat::Pretty
        },
        log_dir,
    );

    // Launch TUI directly when no command is given and stdout is a terminal.
    if cli.command.is_none() && std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        return slapper_tui::run(cli.config.clone());
    }

    let config = slapper::config::load_config(cli.config.as_deref())?;
    let scope = slapper::config::load_scope(cli.scope.as_deref())?;

    let ctx = slapper::commands::CommandContext::new(config, scope, cli.json)
        .with_config_path(cli.config.clone());
    slapper::commands::handle_command(cli, &ctx).await?;

    Ok(())
}
