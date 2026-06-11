use std::path::PathBuf;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::Shell;

use eggsec::cli::Cli;
use eggsec::logging::{init_logging, LogFormat};

fn generate_shell_completion(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "eggsec", &mut std::io::stdout());
    Ok(())
}

fn agent_log_dir(cli: &Cli) -> Option<PathBuf> {
    #[cfg(feature = "rest-api")]
    {
        if let Some(eggsec::cli::Commands::Agent(ref args)) = cli.command {
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
        println!("{}", eggsec::config::get_default_config());
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
        return eggsec_tui::run(cli.config.clone());
    }

    let config = eggsec::config::load_config(cli.config.as_deref())?;
    let loaded_scope = eggsec::config::load_scope_with_source(cli.scope.as_deref())?;

    let execution_profile = if matches!(cli.command.as_ref(), Some(eggsec::cli::Commands::Ci(_))) {
        eggsec::config::ExecutionProfile::CiStrict
    } else if cli.strict_scope {
        eggsec::config::ExecutionProfile::ManualGuarded
    } else {
        eggsec::config::ExecutionProfile::ManualPermissive
    };

    let mut ctx =
        eggsec::commands::CommandContext::new(config, loaded_scope.scope.clone(), cli.json)
            .with_config_path(cli.config.clone())
            .with_execution_profile(execution_profile)
            .with_loaded_scope(loaded_scope);

    // Populate manual override from CLI flags (only effective for ManualPermissive).
    // --strict-scope, CI, MCP, agent ignore these.
    let manual_override = eggsec::config::ManualOverride {
        assume_yes: cli.yes,
        allow_out_of_scope: cli.allow_out_of_scope,
        allow_explicit_exclusion: cli.allow_excluded_target,
        allow_high_risk: cli.allow_high_risk,
        allow_nonbaseline_capability: cli.allow_nonbaseline_capability,
        allow_private_resolution: cli.allow_out_of_scope, // broad for now; specific future flags can refine
        allow_cross_host_redirect: cli.allow_out_of_scope,
        reason: cli.manual_override_reason.clone(),
    };
    ctx = ctx.with_manual_override(manual_override);

    eggsec::commands::handle_command(cli, &ctx).await?;

    Ok(())
}
