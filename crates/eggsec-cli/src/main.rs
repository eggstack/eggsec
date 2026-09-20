use std::path::PathBuf;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::Shell;

use eggsec::cli::Cli;
mod logging;
use logging::{init_logging_with_console, resolve_console_logging, ConsoleLogging, LogFormat};

#[cfg(feature = "daemon-client")]
mod daemon_cli;

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

/// Derive the [`ExecutionSurface`] from the CLI command and flags.
///
/// Feature-gated command variants (MCP, agent, serve) are matched only when
/// the `rest-api` feature is enabled. The `grpc-api` serve command also maps
/// to `RestApi`.
#[cfg(feature = "rest-api")]
fn resolve_execution_surface(cli: &Cli) -> eggsec::config::ExecutionSurface {
    use eggsec::cli::Commands;
    match cli.command.as_ref() {
        Some(Commands::Ci(_)) => eggsec::config::ExecutionSurface::Ci,
        Some(Commands::Agent(_)) => eggsec::config::ExecutionSurface::SecurityAgent,
        Some(Commands::McpServe(_)) | Some(Commands::CodeggMcp(_)) => {
            eggsec::config::ExecutionSurface::McpServer
        }
        Some(Commands::Serve(_)) => eggsec::config::ExecutionSurface::RestApi,
        _ if cli.strict_scope => eggsec::config::ExecutionSurface::CliManualStrict,
        _ => eggsec::config::ExecutionSurface::CliManual,
    }
}

#[cfg(not(feature = "rest-api"))]
fn resolve_execution_surface(cli: &Cli) -> eggsec::config::ExecutionSurface {
    if matches!(cli.command.as_ref(), Some(eggsec::cli::Commands::Ci(_))) {
        eggsec::config::ExecutionSurface::Ci
    } else if cli.strict_scope {
        eggsec::config::ExecutionSurface::CliManualStrict
    } else {
        eggsec::config::ExecutionSurface::CliManual
    }
}

/// Whether the process will enter rich TUI mode.
///
/// Pure policy selection over launch eligibility, tested without touching the
/// global tracing subscriber or the real stdout. The caller passes the
/// already-computed facts: no subcommand was given and stdout is a terminal.
/// The `tui_feature` flag mirrors the `tui` Cargo feature gate so headless
/// builds never select the TUI surface.
pub fn rich_tui_launch_requested(
    has_command: bool,
    stdout_is_terminal: bool,
    tui_feature: bool,
) -> bool {
    tui_feature && !has_command && stdout_is_terminal
}

/// Resolve the console emission policy before subscriber construction.
///
/// Rich TUI mode disables console formatting so Ratatui/Crossterm is the only
/// writer to the controlling terminal. All other surfaces keep console output.
pub fn console_policy_for_launch(is_rich_tui_launch: bool) -> ConsoleLogging {
    resolve_console_logging(is_rich_tui_launch)
}

#[tokio::main]
async fn main() -> Result<()> {
    eggsec::install_tls_provider();
    let cli = Cli::parse();

    if cli.generate_config {
        println!("{}", eggsec::config::get_default_config());
        return Ok(());
    }

    if let Some(shell) = cli.generate_shell_completion {
        generate_shell_completion(shell)?;
        return Ok(());
    }

    // Resolve rich-TUI launch intent before subscriber construction so the
    // logging boundary can suppress console formatting for the alternate
    // screen. Stdout *and* stderr are both the same terminal while the TUI
    // owns it, so stderr is never used as an alternate writer.
    #[cfg(feature = "tui")]
    let is_rich_tui_launch = rich_tui_launch_requested(
        cli.command.is_none(),
        std::io::IsTerminal::is_terminal(&std::io::stdout()),
        true,
    );
    #[cfg(not(feature = "tui"))]
    let is_rich_tui_launch = rich_tui_launch_requested(
        cli.command.is_none(),
        std::io::IsTerminal::is_terminal(&std::io::stdout()),
        false,
    );

    let log_dir = agent_log_dir(&cli);
    let _guard = init_logging_with_console(
        if cli.json {
            LogFormat::Json
        } else {
            LogFormat::Pretty
        },
        log_dir,
        console_policy_for_launch(is_rich_tui_launch),
    );

    // Launch TUI directly when the pre-logging launch intent resolved true.
    // The unknown-runtime notice below runs before terminal acquisition, so it
    // cannot corrupt the alternate screen; it is intentionally retained as
    // pre-terminal stderr output to preserve the fallback-to-embedded CLI
    // contract (Phase A completion record).
    #[cfg(feature = "tui")]
    if is_rich_tui_launch {
        let mode = match cli.runtime.as_deref() {
            Some("daemon") => eggsec_tui::RuntimeMode::Daemon {
                socket_path: cli.socket.clone(),
                session_id: cli.session.clone(),
                new_session: cli.new_session,
                attach_latest: cli.attach_latest,
            },
            Some(other) => {
                eprintln!("Unknown runtime mode '{}', falling back to embedded", other);
                eggsec_tui::RuntimeMode::Embedded
            }
            None => eggsec_tui::RuntimeMode::Embedded,
        };
        return eggsec_tui::run_with_mode(cli.config.clone(), mode);
    }

    // Headless mode: no command provided and no TUI feature.
    #[cfg(not(feature = "tui"))]
    if cli.command.is_none() {
        if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
            eprintln!("No command specified. Run 'eggsec --help' for usage.");
        }
        return Ok(());
    }

    // Handle daemon client commands before general command dispatch.
    #[cfg(feature = "daemon-client")]
    if let Some(ref cmd) = cli.command {
        if daemon_cli::is_daemon_command(cmd) {
            return daemon_cli::handle_daemon_command(cmd, &cli).await;
        }
    }

    let config = eggsec::config::load_config(cli.config.as_deref())?;
    let loaded_scope = eggsec::config::load_scope_with_source(cli.scope.as_deref())?;

    let execution_surface = resolve_execution_surface(&cli);

    let mut ctx =
        eggsec::commands::CommandContext::new(config, loaded_scope.scope.clone(), cli.json)
            .with_config_path(cli.config.clone())
            .with_execution_surface(execution_surface)
            .with_loaded_scope(loaded_scope);

    // Populate manual override from CLI flags (only effective for ManualPermissive).
    // --strict-scope, CI, MCP, agent ignore these.
    let manual_override = eggsec::config::ManualOverride {
        assume_yes: cli.yes,
        allow_out_of_scope: cli.allow_out_of_scope,
        allow_explicit_exclusion: cli.allow_excluded_target,
        allow_high_risk: cli.allow_high_risk,
        allow_db_pentest: cli.allow_db_pentest,
        allow_web_proxy: cli.allow_web_proxy,
        allow_nonbaseline_capability: cli.allow_nonbaseline_capability,
        allow_private_resolution: cli.allow_private_resolution,
        allow_cross_host_redirect: cli.allow_cross_host_redirect,
        reason: cli.manual_override_reason.clone(),
    };
    ctx = ctx.with_manual_override(manual_override);

    eggsec::commands::handle_command(cli, &ctx).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::console_policy_for_launch;
    use super::rich_tui_launch_requested;
    use crate::logging::{console_layer_enabled, ConsoleLogging};

    #[test]
    fn rich_tui_launch_requires_feature_no_command_and_tty() {
        assert!(rich_tui_launch_requested(false, true, true));
        assert!(!rich_tui_launch_requested(true, true, true));
        assert!(!rich_tui_launch_requested(false, false, true));
        assert!(!rich_tui_launch_requested(false, true, false));
        assert!(!rich_tui_launch_requested(true, false, false));
    }

    #[test]
    fn console_policy_disables_for_tui_launch_only() {
        assert_eq!(console_policy_for_launch(true), ConsoleLogging::Disabled);
        assert_eq!(console_policy_for_launch(false), ConsoleLogging::Enabled);
    }

    #[test]
    fn tui_launch_implies_no_console_writer() {
        // Prove the policy, not just the enum: the TUI surface must have no
        // console writer while non-TUI surfaces keep one.
        let tui_console = console_policy_for_launch(rich_tui_launch_requested(false, true, true));
        assert!(!console_layer_enabled(tui_console));

        let cli_console = console_policy_for_launch(rich_tui_launch_requested(true, true, true));
        assert!(console_layer_enabled(cli_console));
    }
}
