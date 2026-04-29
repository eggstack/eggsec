use anyhow::Result;
use std::path::PathBuf;

use crate::agent::{Agent, AgentConfig, Priority, TargetConfig, TargetPortfolio};
#[cfg(feature = "ai-integration")]
use crate::agent::SkillLoader;
use crate::cli::agent::{AgentArgs, AgentCommand};
use crate::commands::handlers::CommandContext;

pub async fn handle_agent(_ctx: &CommandContext, args: AgentArgs) -> Result<()> {
    let use_ai = args.with_ai;
    let ai_config_path = args.ai_config.clone();
    let portfolio_path = args.portfolio.clone();
    let memory_dir = expand_path(&args.memory_dir);
    let poll_interval = args.poll_interval;
    
    match args.command {
        None => {
            println!("Agent commands:");
            println!("  slapper agent run          - Run the autonomous agent");
            println!("  slapper agent targets     - Manage targets");
            println!("  slapper agent skills      - Manage skills");
            println!("  slapper agent status      - Show agent status");
            Ok(())
        }
        Some(AgentCommand::Run(run_args)) => {
            handle_agent_run_impl(use_ai, ai_config_path, portfolio_path, memory_dir, poll_interval, run_args).await
        }
        Some(AgentCommand::Targets(targets_args)) => handle_targets(targets_args, portfolio_path).await,
        Some(AgentCommand::Skills(skills_args)) => handle_skills(skills_args).await,
        Some(AgentCommand::Status) => handle_status_impl(use_ai, ai_config_path, portfolio_path).await,
    }
}

async fn handle_agent_run_impl(
    use_ai: bool, 
    ai_config_path: Option<String>, 
    portfolio_path: Option<String>,
    memory_dir: PathBuf,
    poll_interval: u64,
    run_args: crate::cli::agent::RunArgs
) -> Result<()> {
    let mut config = AgentConfig {
        portfolio_path: portfolio_path.map(|p| expand_path(&p)),
        memory_dir,
        poll_interval_secs: poll_interval,
        ai_config: None,
    };

    if use_ai {
        if let Some(ref ai_path) = ai_config_path {
            let path = PathBuf::from(ai_path);
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(ai_settings) = toml::from_str::<crate::config::AiConfig>(&content) {
                    config.ai_config = Some(ai_settings);
                }
            }
        }
    }

    let use_ai_final = use_ai && config.ai_config.is_some();
    let run_once = run_args.once;
    let ai_config = config.ai_config.clone();

    let mut agent = Agent::new(config).await?;

    #[cfg(feature = "ai-integration")]
    {
        use crate::agent::Agent as AgentTrait;
        if use_ai_final {
            let mut agent_with_ai = agent.with_ai_client(ai_config.unwrap()).await;
            if run_once {
                agent_with_ai.execute_scan("dummy", "recon").await?;
            } else {
                agent_with_ai.run().await?;
            }
        } else if run_once {
            println!("Agent run completed (once mode)");
        } else {
            agent.run().await?;
        }
    }

    #[cfg(not(feature = "ai-integration"))]
    {
        if use_ai_final {
            eprintln!("Warning: AI features not enabled. Recompile with --features ai-integration");
        }
        if run_once {
            println!("Agent run completed (once mode)");
        } else {
            agent.run().await?;
        }
    }

    Ok(())
}

async fn handle_status_impl(_use_ai: bool, _ai_config_path: Option<String>, portfolio_path: Option<String>) -> Result<()> {
    println!("Agent Status");
    println!("{}", "=".repeat(50));

    if let Some(ref path) = portfolio_path {
        println!("Portfolio: {}", path);
    } else {
        println!("Portfolio: not configured");
    }

    let portfolio = if let Some(ref path_str) = portfolio_path {
        let path = PathBuf::from(path_str);
        TargetPortfolio::load_from_file(&path).unwrap_or_else(|_| TargetPortfolio::new())
    } else {
        TargetPortfolio::new()
    };

    let targets = portfolio.get_all_targets();
    println!("\nTargets: {} total", targets.len());

    let enabled_targets: Vec<_> = targets.iter().filter(|(_, cfg)| cfg.enabled).collect();
    let disabled_targets: Vec<_> = targets.iter().filter(|(_, cfg)| !cfg.enabled).collect();

    println!("  Enabled: {}", enabled_targets.len());
    println!("  Disabled: {}", disabled_targets.len());

    if !targets.is_empty() {
        println!("\nTarget Details:");
        println!("{}", "-".repeat(50));
        for (id, config) in &targets {
            let status = if config.enabled { "enabled" } else { "disabled" };
            let schedule = config.schedule.as_deref().unwrap_or("none");
            let last_scan = config.last_scan.map(|t| t.to_rfc3339()).unwrap_or_else(|| "never".to_string());
            let scan_count = config.scan_history.len();

            println!("  {} [{}]", id, status);
            println!("    Target: {}", config.target);
            println!("    Schedule: {}", schedule);
            println!("    Last scan: {}", last_scan);
            println!("    Scan history: {} scans", scan_count);
            println!("    Priority: {:?}", config.priority);
            println!("    Scan depth: {:?}", config.scan_depth);
            if !config.alert_channels.is_empty() {
                println!("    Alerts: {} channels", config.alert_channels.len());
            }
            println!();
        }
    }

    println!("{}", "=".repeat(50));
    Ok(())
}

fn expand_path(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            let mut p = PathBuf::from(home);
            p.push(&path[2..]);
            return p;
        }
    }
    PathBuf::from(path)
}

async fn handle_targets(args: crate::cli::agent::TargetsArgs, portfolio_path: Option<String>) -> Result<()> {
    match args.command {
        crate::cli::agent::TargetsCommand::List => {
            let portfolio = TargetPortfolio::new();
            println!("Targets:");
            for (id, config) in portfolio.get_all_targets() {
                println!("  {} -> {} (enabled: {})", id, config.target, config.enabled);
            }
            Ok(())
        }
        crate::cli::agent::TargetsCommand::Add(add_args) => {
            let priority = match add_args.priority.to_lowercase().as_str() {
                "low" => Priority::Low,
                "high" => Priority::High,
                "critical" => Priority::Critical,
                _ => Priority::Normal,
            };

            let config = TargetConfig {
                target: add_args.target,
                target_type: add_args.target_type,
                priority,
                schedule: add_args.schedule,
                alert_channels: Vec::new(),
                last_scan: None,
                scan_history: Vec::new(),
                baseline_findings: Vec::new(),
                enabled: true,
                scan_depth: crate::agent::portfolio::ScanDepth::default(),
                off_peak_window: None,
            };

            let mut portfolio = TargetPortfolio::new();
            portfolio.add_target(add_args.id, config);
            portfolio.save()?;

            println!("Target added successfully");
            Ok(())
        }
        crate::cli::agent::TargetsCommand::Update(update_args) => {
            let path = portfolio_path.as_ref().map(PathBuf::from).unwrap_or_else(|| {
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".config").join("slapper").join("portfolio.json"))
                    .unwrap_or_else(|_| PathBuf::from("portfolio.json"))
            });
            let mut portfolio = TargetPortfolio::load_from_file(&path).unwrap_or_else(|_| TargetPortfolio::new());
            if let Some(mut target) = portfolio.get_mut_target(&update_args.id) {
                if let Some(new_target) = update_args.target {
                    target.target = new_target;
                }
                if let Some(schedule) = update_args.schedule {
                    target.schedule = Some(schedule);
                }
                if let Some(priority) = update_args.priority {
                    target.priority = match priority.to_lowercase().as_str() {
                        "low" => Priority::Low,
                        "high" => Priority::High,
                        "critical" => Priority::Critical,
                        _ => Priority::Normal,
                    };
                }
                if let Some(depth) = update_args.scan_depth {
                    target.scan_depth = match depth.to_lowercase().as_str() {
                        "shallow" => crate::agent::portfolio::ScanDepth::Shallow,
                        "deep" => crate::agent::portfolio::ScanDepth::Deep,
                        _ => target.scan_depth,
                    };
                }
                portfolio.save()?;
                println!("Target {} updated successfully", update_args.id);
            } else {
                println!("Target {} not found", update_args.id);
            }
            Ok(())
        }
        crate::cli::agent::TargetsCommand::Remove { id } => {
            let mut portfolio = TargetPortfolio::new();
            if portfolio.remove_target(&id) {
                portfolio.save()?;
                println!("Target {} removed", id);
            } else {
                println!("Target {} not found", id);
            }
            Ok(())
        }
        crate::cli::agent::TargetsCommand::Enable { id } => {
            let mut portfolio = TargetPortfolio::new();
            if let Some(mut target) = portfolio.get_mut_target(&id) {
                target.enabled = true;
                portfolio.save()?;
                println!("Target {} enabled", id);
            } else {
                println!("Target {} not found", id);
            }
            Ok(())
        }
        crate::cli::agent::TargetsCommand::Disable { id } => {
            let mut portfolio = TargetPortfolio::new();
            if let Some(mut target) = portfolio.get_mut_target(&id) {
                target.enabled = false;
                portfolio.save()?;
                println!("Target {} disabled", id);
            } else {
                println!("Target {} not found", id);
            }
            Ok(())
        }
    }
}

async fn handle_skills(args: crate::cli::agent::SkillsArgs) -> Result<()> {
    #[cfg(feature = "ai-integration")]
    {
        match args.command {
            crate::cli::agent::SkillsCommand::List => {
                let default_dirs = vec![
                    PathBuf::from("~/.config/slapper/skills"),
                ];
                let loader = SkillLoader::new(default_dirs);
                let skills = loader.load_skills()?;
                println!("Available skills:");
                for skill in skills {
                    println!("  {} - {}", skill.name, skill.description);
                }
                Ok(())
            }
            crate::cli::agent::SkillsCommand::Load { path } => {
                let path = expand_path(&path);
                let loader = SkillLoader::new(vec![path]);
                let skills = loader.load_skills()?;
                println!("Loaded {} skills", skills.len());
                Ok(())
            }
            crate::cli::agent::SkillsCommand::Show { name } => {
                let default_dirs = vec![
                    PathBuf::from("~/.config/slapper/skills"),
                ];
                let loader = SkillLoader::new(default_dirs);
                let skills = loader.load_skills()?;
                if let Some(skill) = skills.iter().find(|s| s.name == name) {
                    let tools = skill.metadata.as_ref()
                        .map(|m| m.tools.join(", "))
                        .unwrap_or_default();
                    println!("# {}\n\n{}\n\nTools: {}", skill.name, skill.description, tools);
                } else {
                    println!("Skill '{}' not found", name);
                }
                Ok(())
            }
        }
    }
    #[cfg(not(feature = "ai-integration"))]
    {
        let _ = args;
        println!("AI integration not enabled. Rebuild with --features ai-integration");
        Ok(())
    }
}