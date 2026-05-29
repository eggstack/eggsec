use crate::tool::traits::AttackSurface;
use crate::tool::{ToolInfo, ToolRegistry};
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub stages: Vec<ExecutionStage>,
    pub estimated_duration_ms: u64,
    pub total_tools: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStage {
    pub name: String,
    pub tools: Vec<ToolExecution>,
    pub parallel: bool,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    pub tool_id: String,
    pub capability: Option<String>,
    pub attack_surface: Vec<String>,
    pub estimated_duration_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRequest {
    pub goal: String,
    pub target: String,
    pub target_type: TargetType,
    pub attack_surfaces: Vec<AttackSurface>,
    pub max_duration_ms: Option<u64>,
    pub include_load_testing: bool,
    pub include_stress_testing: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetType {
    Web,
    Api,
    Network,
    Mixed,
}

impl Default for PlanRequest {
    fn default() -> Self {
        Self {
            goal: "full_assessment".to_string(),
            target: String::new(),
            target_type: TargetType::Mixed,
            attack_surfaces: vec![
                AttackSurface::Web,
                AttackSurface::Api,
                AttackSurface::Network,
            ],
            max_duration_ms: None,
            include_load_testing: false,
            include_stress_testing: false,
        }
    }
}

#[derive(Clone)]
pub struct ChainPlanner {
    registry: ToolRegistry,
}

impl ChainPlanner {
    pub fn new(registry: ToolRegistry) -> Self {
        Self { registry }
    }

    pub fn plan(&self, request: &PlanRequest) -> ExecutionPlan {
        let tools = self.registry.list();
        let mut stages = Vec::new();
        let mut used_tools: FxHashSet<String> = FxHashSet::default();
        let mut estimated_duration: u64 = 0;

        match request.goal.as_str() {
            "recon" | "reconnaissance" | "discovery" => {
                let stage = self.build_recon_stage(&tools, &request.target, &mut used_tools);
                if let Some(s) = stage {
                    estimated_duration += s
                        .tools
                        .iter()
                        .map(|t| t.estimated_duration_ms as u64)
                        .sum::<u64>();
                    stages.push(s);
                }
            }
            "vuln_scan" | "vulnerability_scan" | "fuzz" => {
                if let Some(recon) =
                    self.build_recon_stage(&tools, &request.target, &mut used_tools)
                {
                    estimated_duration += recon
                        .tools
                        .iter()
                        .map(|t| t.estimated_duration_ms as u64)
                        .sum::<u64>();
                    stages.push(recon);
                }
                if let Some(scan) =
                    self.build_vuln_scan_stage(&tools, &request.target, &mut used_tools)
                {
                    estimated_duration += scan
                        .tools
                        .iter()
                        .map(|t| t.estimated_duration_ms as u64)
                        .sum::<u64>();
                    stages.push(scan);
                }
            }
            "full_assessment" | "full" | "complete" => {
                if let Some(recon) =
                    self.build_recon_stage(&tools, &request.target, &mut used_tools)
                {
                    estimated_duration += recon
                        .tools
                        .iter()
                        .map(|t| t.estimated_duration_ms as u64)
                        .sum::<u64>();
                    stages.push(recon);
                }
                if let Some(scan) =
                    self.build_vuln_scan_stage(&tools, &request.target, &mut used_tools)
                {
                    estimated_duration += scan
                        .tools
                        .iter()
                        .map(|t| t.estimated_duration_ms as u64)
                        .sum::<u64>();
                    stages.push(scan);
                }
                if let Some(api) = self.build_api_stage(&tools, &request.target, &mut used_tools) {
                    estimated_duration += api
                        .tools
                        .iter()
                        .map(|t| t.estimated_duration_ms as u64)
                        .sum::<u64>();
                    stages.push(api);
                }
                if request.include_load_testing {
                    if let Some(perf) =
                        self.build_load_test_stage(&tools, &request.target, &mut used_tools)
                    {
                        estimated_duration += perf
                            .tools
                            .iter()
                            .map(|t| t.estimated_duration_ms as u64)
                            .sum::<u64>();
                        stages.push(perf);
                    }
                }
            }
            "api" | "api_security" => {
                if let Some(api) = self.build_api_stage(&tools, &request.target, &mut used_tools) {
                    estimated_duration += api
                        .tools
                        .iter()
                        .map(|t| t.estimated_duration_ms as u64)
                        .sum::<u64>();
                    stages.push(api);
                }
            }
            "quick" | "fast" => {
                if let Some(scan) =
                    self.build_quick_scan_stage(&tools, &request.target, &mut used_tools)
                {
                    estimated_duration += scan
                        .tools
                        .iter()
                        .map(|t| t.estimated_duration_ms as u64)
                        .sum::<u64>();
                    stages.push(scan);
                }
            }
            _ => {
                let stage =
                    self.build_full_pipeline_stage(&tools, &request.target, &mut used_tools);
                estimated_duration += stage
                    .tools
                    .iter()
                    .map(|t| t.estimated_duration_ms as u64)
                    .sum::<u64>();
                stages.push(stage);
            }
        }

        ExecutionPlan {
            stages,
            estimated_duration_ms: estimated_duration,
            total_tools: used_tools.len(),
        }
    }

    fn build_recon_stage(
        &self,
        tools: &[ToolInfo],
        _target: &str,
        used: &mut FxHashSet<String>,
    ) -> Option<ExecutionStage> {
        let mut stage_tools = Vec::new();

        for tool in tools {
            if tool.category == crate::tool::traits::ToolCategory::Recon && !used.contains(&tool.id)
            {
                for cap in &tool.capabilities {
                    if cap.name == "full_recon" || cap.name == "dns" || cap.name == "tech_detection"
                    {
                        stage_tools.push(ToolExecution {
                            tool_id: tool.id.clone(),
                            capability: Some(cap.name.clone()),
                            attack_surface: cap
                                .attack_surface
                                .iter()
                                .map(|s| format!("{:?}", s).to_lowercase())
                                .collect(),
                            estimated_duration_ms: cap.estimated_duration_ms,
                        });
                        used.insert(tool.id.clone());
                        break;
                    }
                }
            }
        }

        if stage_tools.is_empty() {
            return None;
        }

        Some(ExecutionStage {
            name: "reconnaissance".to_string(),
            tools: stage_tools,
            parallel: true,
            depends_on: vec![],
        })
    }

    fn build_vuln_scan_stage(
        &self,
        tools: &[ToolInfo],
        _target: &str,
        used: &mut FxHashSet<String>,
    ) -> Option<ExecutionStage> {
        let mut stage_tools = Vec::new();

        for tool in tools {
            if tool.category == crate::tool::traits::ToolCategory::Scanning
                && !used.contains(&tool.id)
            {
                for cap in &tool.capabilities {
                    if cap.name == "scan_endpoints" {
                        stage_tools.push(ToolExecution {
                            tool_id: tool.id.clone(),
                            capability: Some(cap.name.clone()),
                            attack_surface: cap
                                .attack_surface
                                .iter()
                                .map(|s| format!("{:?}", s).to_lowercase())
                                .collect(),
                            estimated_duration_ms: cap.estimated_duration_ms,
                        });
                        used.insert(tool.id.clone());
                        break;
                    }
                }
            }
            if tool.category == crate::tool::traits::ToolCategory::Fuzzing
                && !used.contains(&tool.id)
            {
                for cap in &tool.capabilities {
                    if cap.name == "sqli" || cap.name == "xss" || cap.name == "ssrf" {
                        stage_tools.push(ToolExecution {
                            tool_id: tool.id.clone(),
                            capability: Some(cap.name.clone()),
                            attack_surface: cap
                                .attack_surface
                                .iter()
                                .map(|s| format!("{:?}", s).to_lowercase())
                                .collect(),
                            estimated_duration_ms: cap.estimated_duration_ms,
                        });
                        used.insert(tool.id.clone());
                    }
                }
            }
        }

        if stage_tools.is_empty() {
            return None;
        }

        Some(ExecutionStage {
            name: "vulnerability_scanning".to_string(),
            tools: stage_tools,
            parallel: true,
            depends_on: vec!["reconnaissance".to_string()],
        })
    }

    fn build_api_stage(
        &self,
        tools: &[ToolInfo],
        _target: &str,
        used: &mut FxHashSet<String>,
    ) -> Option<ExecutionStage> {
        let mut stage_tools = Vec::new();

        for tool in tools {
            if tool.category == crate::tool::traits::ToolCategory::Fuzzing
                && !used.contains(&tool.id)
            {
                for cap in &tool.capabilities {
                    if cap.name == "graphql" || cap.name == "jwt" {
                        stage_tools.push(ToolExecution {
                            tool_id: tool.id.clone(),
                            capability: Some(cap.name.clone()),
                            attack_surface: cap
                                .attack_surface
                                .iter()
                                .map(|s| format!("{:?}", s).to_lowercase())
                                .collect(),
                            estimated_duration_ms: cap.estimated_duration_ms,
                        });
                        used.insert(tool.id.clone());
                    }
                }
            }
        }

        if stage_tools.is_empty() {
            return None;
        }

        Some(ExecutionStage {
            name: "api_security".to_string(),
            tools: stage_tools,
            parallel: true,
            depends_on: vec!["reconnaissance".to_string()],
        })
    }

    fn build_load_test_stage(
        &self,
        tools: &[ToolInfo],
        _target: &str,
        used: &mut FxHashSet<String>,
    ) -> Option<ExecutionStage> {
        let mut stage_tools = Vec::new();

        for tool in tools {
            if tool.category == crate::tool::traits::ToolCategory::LoadTest
                && !used.contains(&tool.id)
            {
                stage_tools.push(ToolExecution {
                    tool_id: tool.id.clone(),
                    capability: None,
                    attack_surface: vec!["performance".to_string()],
                    estimated_duration_ms: 120000,
                });
                used.insert(tool.id.clone());
                break;
            }
        }

        if stage_tools.is_empty() {
            return None;
        }

        Some(ExecutionStage {
            name: "load_testing".to_string(),
            tools: stage_tools,
            parallel: false,
            depends_on: vec!["vulnerability_scanning".to_string()],
        })
    }

    fn build_quick_scan_stage(
        &self,
        tools: &[ToolInfo],
        _target: &str,
        used: &mut FxHashSet<String>,
    ) -> Option<ExecutionStage> {
        let mut stage_tools = Vec::new();

        for tool in tools {
            if tool.category == crate::tool::traits::ToolCategory::Pipeline
                && !used.contains(&tool.id)
            {
                for cap in &tool.capabilities {
                    if cap.name == "quick" {
                        stage_tools.push(ToolExecution {
                            tool_id: tool.id.clone(),
                            capability: Some(cap.name.clone()),
                            attack_surface: cap
                                .attack_surface
                                .iter()
                                .map(|s| format!("{:?}", s).to_lowercase())
                                .collect(),
                            estimated_duration_ms: cap.estimated_duration_ms,
                        });
                        used.insert(tool.id.clone());
                        break;
                    }
                }
            }
        }

        if stage_tools.is_empty() {
            return None;
        }

        Some(ExecutionStage {
            name: "quick_scan".to_string(),
            tools: stage_tools,
            parallel: false,
            depends_on: vec![],
        })
    }

    fn build_full_pipeline_stage(
        &self,
        tools: &[ToolInfo],
        _target: &str,
        used: &mut FxHashSet<String>,
    ) -> ExecutionStage {
        let mut stage_tools = Vec::new();

        for tool in tools {
            if tool.category == crate::tool::traits::ToolCategory::Pipeline
                && !used.contains(&tool.id)
            {
                for cap in &tool.capabilities {
                    if cap.name == "full" {
                        stage_tools.push(ToolExecution {
                            tool_id: tool.id.clone(),
                            capability: Some(cap.name.clone()),
                            attack_surface: cap
                                .attack_surface
                                .iter()
                                .map(|s| format!("{:?}", s).to_lowercase())
                                .collect(),
                            estimated_duration_ms: cap.estimated_duration_ms,
                        });
                        used.insert(tool.id.clone());
                        break;
                    }
                }
            }
        }

        ExecutionStage {
            name: "full_pipeline".to_string(),
            tools: stage_tools,
            parallel: false,
            depends_on: vec![],
        }
    }

    pub fn suggest_tools_for_attack_surface(&self, surface: AttackSurface) -> Vec<ToolInfo> {
        self.registry
            .list()
            .into_iter()
            .filter(|tool| {
                tool.capabilities
                    .iter()
                    .any(|cap| cap.attack_surface.contains(&surface))
            })
            .collect()
    }

    pub fn get_tool_dependencies(&self, tool_id: &str) -> Vec<String> {
        self.registry
            .get(tool_id)
            .map(|tool| {
                tool.capabilities()
                    .iter()
                    .flat_map(|cap| cap.prerequisites.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn validate_plan(&self, plan: &ExecutionPlan) -> PlanValidation {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        let mut resolved_stages: FxHashSet<String> = FxHashSet::default();

        for stage in &plan.stages {
            for dep in &stage.depends_on {
                if !resolved_stages.contains(dep) && !plan.stages.iter().any(|s| s.name == *dep) {
                    errors.push(format!(
                        "Stage '{}' depends on undeclared stage '{}'",
                        stage.name, dep
                    ));
                }
            }
            resolved_stages.insert(stage.name.clone());
        }

        let duration_check = plan.estimated_duration_ms;
        if duration_check > 3600000 {
            warnings.push("Plan estimated to take over 1 hour".to_string());
        }

        PlanValidation {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanValidation {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ExecutionPlan {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self)
            .inspect_err(|e| {
                tracing::debug!(error = %e, "Failed to serialize execution plan");
            })
            .unwrap_or_default()
    }

    pub fn total_tools(&self) -> usize {
        self.stages.iter().map(|s| s.tools.len()).sum()
    }

    pub fn stage_names(&self) -> Vec<String> {
        self.stages.iter().map(|s| s.name.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::create_default_registry;

    #[test]
    fn test_plan_generation() {
        let registry = create_default_registry();
        let planner = ChainPlanner::new(registry);

        let request = PlanRequest {
            goal: "full_assessment".to_string(),
            target: "https://example.com".to_string(),
            ..Default::default()
        };

        let plan = planner.plan(&request);

        assert!(!plan.stages.is_empty());
        assert!(plan.total_tools() > 0);
    }

    #[test]
    fn test_plan_validation() {
        let registry = create_default_registry();
        let planner = ChainPlanner::new(registry);

        let request = PlanRequest {
            goal: "recon".to_string(),
            target: "https://example.com".to_string(),
            ..Default::default()
        };

        let plan = planner.plan(&request);
        let validation = planner.validate_plan(&plan);

        assert!(validation.valid);
    }
}
