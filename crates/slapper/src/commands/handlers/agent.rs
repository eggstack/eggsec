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
    
    match args.command {
        None => {
            println!("Agent commands:");
            println!("  slapper agent run          - Run the autonomous agent");
            println!("  slapper agent targets     - Manage targets");
            println!("  slapper agent skills      - Manage skills");
            println!("  slapper agent status      - Show agent status");
            Ok(())
        }
        Some(AgentCommand::Run(run_args)) => handle_agent_run_impl(use_ai, ai_config_path, run_args).await,
        Some(AgentCommand::Targets(targets_args)) => handle_targets(targets_args).await,
        Some(AgentCommand::Skills(skills_args)) => handle_skills(skills_args).await,
        Some(AgentCommand::Status) => handle_status_impl(use_ai, ai_config_path).await,
    }
}

async fn handle_agent_run_impl(use_ai: bool, ai_config_path: Option<String>, run_args: crate::cli::agent::RunArgs) -> Result<()> {
    let memory_dir = expand_path("~/.config/slapper/memory");

    let mut config = AgentConfig {
        portfolio_path: None,
        memory_dir,
        poll_interval_secs: 60,
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

    Ok(())
}

async fn handle_status_impl(_use_ai: bool, _ai_config_path: Option<String>) -> Result<()> {
    println!("Agent Status");
    println!("  Poll interval: 60s");
    println!("  Portfolio: not configured");
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

async fn handle_targets(args: crate::cli::agent::TargetsArgs) -> Result<()> {
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
            };

            let mut portfolio = TargetPortfolio::new();
            portfolio.add_target(add_args.id, config);
            portfolio.save()?;

            println!("Target added successfully");
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
            if let Some(target) = portfolio.get_mut_target(&id) {
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
            if let Some(target) = portfolio.get_mut_target(&id) {
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
                    println!("# {}\n\n{}\n\nTools: {}",
                        skill.name,
                        skill.description,
                        skill.metadata.tools.join(", "));
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