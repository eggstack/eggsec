use crate::cli::plan::PlanArgs;
use crate::cli::ScanProfile;
use crate::commands::handlers::CommandContext;
use crate::config::{
    evaluate_operation_policy, load_scope, OperationDescriptor, OperationMode, OperationRisk,
    PolicyDecision,
};
use anyhow::Result;

#[derive(Debug, serde::Serialize)]
pub struct PlannedStage {
    pub name: String,
    pub risk: OperationRisk,
    pub required_features: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct SkippedStage {
    pub name: String,
    pub reason: String,
}

#[derive(Debug, serde::Serialize)]
pub struct PlanOutput {
    pub target: Option<String>,
    pub profile: String,
    pub operation_mode: OperationMode,
    pub max_risk: OperationRisk,
    pub stages: Vec<PlannedStage>,
    pub policy_decisions: Vec<PolicyDecision>,
    pub skipped_stages: Vec<SkippedStage>,
}

fn profile_stages(profile: ScanProfile) -> Vec<PlannedStage> {
    match profile {
        ScanProfile::Quick => vec![PlannedStage {
            name: "recon".to_string(),
            risk: OperationRisk::Passive,
            required_features: vec![],
        }],
        ScanProfile::Endpoint => vec![
            PlannedStage {
                name: "recon".to_string(),
                risk: OperationRisk::Passive,
                required_features: vec![],
            },
            PlannedStage {
                name: "endpoints".to_string(),
                risk: OperationRisk::SafeActive,
                required_features: vec![],
            },
        ],
        ScanProfile::Web | ScanProfile::Vuln | ScanProfile::Auth => vec![
            PlannedStage {
                name: "recon".to_string(),
                risk: OperationRisk::Passive,
                required_features: vec![],
            },
            PlannedStage {
                name: "ports".to_string(),
                risk: OperationRisk::SafeActive,
                required_features: vec![],
            },
            PlannedStage {
                name: "endpoints".to_string(),
                risk: OperationRisk::SafeActive,
                required_features: vec![],
            },
            PlannedStage {
                name: "fingerprint".to_string(),
                risk: OperationRisk::SafeActive,
                required_features: vec![],
            },
            PlannedStage {
                name: "fuzz".to_string(),
                risk: OperationRisk::Intrusive,
                required_features: vec![],
            },
        ],
        ScanProfile::Waf | ScanProfile::WafRegression => vec![
            PlannedStage {
                name: "recon".to_string(),
                risk: OperationRisk::Passive,
                required_features: vec![],
            },
            PlannedStage {
                name: "ports".to_string(),
                risk: OperationRisk::SafeActive,
                required_features: vec![],
            },
            PlannedStage {
                name: "waf-detect".to_string(),
                risk: OperationRisk::Intrusive,
                required_features: vec![],
            },
        ],
        ScanProfile::Full | ScanProfile::Api | ScanProfile::Deep => vec![
            PlannedStage {
                name: "recon".to_string(),
                risk: OperationRisk::Passive,
                required_features: vec![],
            },
            PlannedStage {
                name: "ports".to_string(),
                risk: OperationRisk::SafeActive,
                required_features: vec![],
            },
            PlannedStage {
                name: "endpoints".to_string(),
                risk: OperationRisk::SafeActive,
                required_features: vec![],
            },
            PlannedStage {
                name: "fingerprint".to_string(),
                risk: OperationRisk::SafeActive,
                required_features: vec![],
            },
            PlannedStage {
                name: "fuzz".to_string(),
                risk: OperationRisk::Intrusive,
                required_features: vec![],
            },
            PlannedStage {
                name: "waf-detect".to_string(),
                risk: OperationRisk::Intrusive,
                required_features: vec![],
            },
        ],
        ScanProfile::Recon => vec![PlannedStage {
            name: "recon".to_string(),
            risk: OperationRisk::Passive,
            required_features: vec![],
        }],
        ScanProfile::Stealth => vec![PlannedStage {
            name: "recon".to_string(),
            risk: OperationRisk::Passive,
            required_features: vec![],
        }],
        ScanProfile::DefenseLab | ScanProfile::SynvoidLocal => vec![
            PlannedStage {
                name: "recon".to_string(),
                risk: OperationRisk::Passive,
                required_features: vec![],
            },
            PlannedStage {
                name: "ports".to_string(),
                risk: OperationRisk::SafeActive,
                required_features: vec![],
            },
            PlannedStage {
                name: "fingerprint".to_string(),
                risk: OperationRisk::SafeActive,
                required_features: vec![],
            },
            PlannedStage {
                name: "waf-detect".to_string(),
                risk: OperationRisk::Intrusive,
                required_features: vec![],
            },
        ],
        ScanProfile::ProtocolEdge => vec![
            PlannedStage {
                name: "recon".to_string(),
                risk: OperationRisk::Passive,
                required_features: vec![],
            },
            PlannedStage {
                name: "protocol-edge".to_string(),
                risk: OperationRisk::SafeActive,
                required_features: vec!["packet-inspection".to_string()],
            },
        ],
        ScanProfile::NseSafe => vec![
            PlannedStage {
                name: "recon".to_string(),
                risk: OperationRisk::Passive,
                required_features: vec![],
            },
            PlannedStage {
                name: "nse-safe".to_string(),
                risk: OperationRisk::SafeActive,
                required_features: vec!["nse".to_string()],
            },
        ],
        ScanProfile::DbRegression => vec![
            PlannedStage {
                name: "recon".to_string(),
                risk: OperationRisk::Passive,
                required_features: vec![],
            },
            PlannedStage {
                name: "ports".to_string(),
                risk: OperationRisk::SafeActive,
                required_features: vec![],
            },
            PlannedStage {
                name: "fingerprint".to_string(),
                risk: OperationRisk::SafeActive,
                required_features: vec![],
            },
            PlannedStage {
                name: "waf-detect".to_string(),
                risk: OperationRisk::Intrusive,
                required_features: vec![],
            },
            PlannedStage {
                name: "fuzz".to_string(),
                risk: OperationRisk::Intrusive,
                required_features: vec![],
            },
        ],
    }
}

pub async fn handle_plan(ctx: &CommandContext, args: PlanArgs) -> Result<()> {
    let profile = ScanProfile::from_str(&args.profile).ok_or_else(|| {
        let valid: Vec<&str> = vec![
            "quick",
            "endpoint",
            "web",
            "waf",
            "full",
            "api",
            "recon",
            "stealth",
            "deep",
            "vuln",
            "auth",
            "defense-lab",
            "synvoid-local",
            "waf-regression",
            "protocol-edge",
            "nse-safe",
            "db-regression",
        ];
        anyhow::anyhow!(
            "Unknown profile '{}'. Valid profiles: {}",
            args.profile,
            valid.join(", ")
        )
    })?;

    let scope = args.scope.as_deref().and_then(|s| load_scope(Some(s)).ok());

    let mode = profile.operation_mode();
    let risk = profile.max_risk_budget().to_operation_risk();
    let intended_uses = profile.intended_uses();
    let stages = profile_stages(profile);

    let mut policy_decisions = Vec::new();
    let mut skipped_stages = Vec::new();
    let mut allowed_stages = Vec::new();

    for stage in &stages {
        let mut required_features = stage.required_features.clone();
        if profile.requires_packet_inspection()
            && !required_features.contains(&"packet-inspection".to_string())
        {
            required_features.push("packet-inspection".to_string());
        }
        if profile.requires_nse() && !required_features.contains(&"nse".to_string()) {
            required_features.push("nse".to_string());
        }

        let descriptor = OperationDescriptor {
            operation: stage.name.clone(),
            mode,
            risk: stage.risk,
            intended_uses: intended_uses.clone(),
            target: args.target.clone(),
            required_features,
            required_policy_flags: vec![],
            requires_private_or_local_target: profile.requires_private_scope(),
            requires_explicit_scope: profile.requires_private_scope(),
            required_capabilities: Vec::new(),
        };

        let decision =
            evaluate_operation_policy(&descriptor, &ctx.config.execution_policy, scope.as_ref());
        let stage_allowed = decision.allowed;
        policy_decisions.push(decision);

        if stage_allowed {
            allowed_stages.push(PlannedStage {
                name: stage.name.clone(),
                risk: stage.risk,
                required_features: stage.required_features.clone(),
            });
        } else {
            let reason = policy_decisions
                .last()
                .and_then(|d| d.denied_reasons.first().cloned())
                .unwrap_or_else(|| "policy denied".to_string());
            skipped_stages.push(SkippedStage {
                name: stage.name.clone(),
                reason,
            });
        }
    }

    let output = PlanOutput {
        target: args.target.clone(),
        profile: profile.to_string(),
        operation_mode: mode,
        max_risk: risk,
        stages: allowed_stages,
        policy_decisions,
        skipped_stages,
    };

    match args.format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        _ => {
            println!("Execution Plan");
            println!("==============");
            println!("Target: {}", args.target.as_deref().unwrap_or("none"));
            println!("Profile: {}", output.profile);
            println!("Mode: {}", output.operation_mode);
            println!("Max risk: {}", output.max_risk);
            println!("Stages: {}", output.stages.len());
            if !output.skipped_stages.is_empty() {
                println!("Skipped: {}", output.skipped_stages.len());
            }
            println!();
            for (i, stage) in output.stages.iter().enumerate() {
                println!("  {}. {} (risk: {})", i + 1, stage.name, stage.risk);
            }
            for skipped in &output.skipped_stages {
                println!("  - {} [SKIPPED: {}]", skipped.name, skipped.reason);
            }
            if ctx.json || !output.policy_decisions.is_empty() {
                println!();
                println!("Policy decisions:");
                for decision in &output.policy_decisions {
                    let status = if decision.allowed {
                        "ALLOWED"
                    } else {
                        "DENIED"
                    };
                    println!(
                        "  {} {}: {}",
                        status, decision.operation, decision.operation_risk
                    );
                }
            }
        }
    }

    Ok(())
}
