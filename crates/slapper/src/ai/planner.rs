use crate::ai::client::AiClient;
use crate::ai::errors::{AiError, Result};
use crate::tool::planner::{ChainPlanner, ExecutionPlan, PlanRequest};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AdaptivePlanSuggestion {
    pub suggested_modifications: Vec<PlanModification>,
    pub confidence: f32,
    pub reasoning: String,
    pub estimated_improvement: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanModification {
    pub modification_type: ModificationType,
    pub target_stage: Option<String>,
    pub description: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModificationType {
    AddStage,
    RemoveStage,
    ReorderStages,
    ModifyStage,
    AddTool,
    RemoveTool,
    IncreaseCoverage,
    ReduceDuration,
}

#[derive(Debug, Clone)]
pub struct PlanOutcome {
    pub success: bool,
    pub findings_count: usize,
    pub severity_distribution: HashMap<String, usize>,
    pub duration_ms: u64,
    pub target: String,
}

pub struct AiPlanner {
    client: Option<AiClient>,
    chain_planner: ChainPlanner,
    learning_cache: Arc<RwLock<HashMap<String, CachedPlan>>>,
    fallback_enabled: bool,
}

#[derive(Debug, Clone)]
struct CachedPlan {
    plan: ExecutionPlan,
    success_rate: f32,
    use_count: usize,
    last_used: u64,
}

impl AiPlanner {
    pub fn new(client: Option<AiClient>, chain_planner: ChainPlanner) -> Self {
        Self {
            client,
            chain_planner,
            learning_cache: Arc::new(RwLock::new(HashMap::new())),
            fallback_enabled: true,
        }
    }

    pub async fn create_plan(&self, request: &PlanRequest) -> ExecutionPlan {
        if let Some(ref client) = self.client {
            match self.query_ai_for_plan(client, request).await {
                Ok(plan) => return plan,
                Err(e) => {
                    tracing::warn!("AI planning failed, falling back to chain planner: {}", e);
                }
            }
        }

        if self.fallback_enabled {
            self.chain_planner.plan(request)
        } else {
            tracing::warn!(
                "Fallback planning disabled but required AI plan was unavailable; using chain planner"
            );
            self.chain_planner.plan(request)
        }
    }

    async fn query_ai_for_plan(
        &self,
        client: &AiClient,
        request: &PlanRequest,
    ) -> Result<ExecutionPlan> {
        let cache_key = format!(
            "{}:{}:{}:{}",
            request.goal,
            request.target,
            request.attack_surfaces.len(),
            request.max_duration_ms.unwrap_or(0)
        );

        {
            let cache = self.learning_cache.read();
            if let Some(cached) = cache.get(&cache_key) {
                if cached.use_count > 3 && cached.success_rate > 0.8 {
                    tracing::debug!("Using cached plan for {}", cache_key);
                    return Ok(cached.plan.clone());
                }
            }
        }

        let prompt = format!(
            "Create a security testing execution plan for target: {}\n\
             Goal: {}\n\
             Target type: {:?}\n\
             Attack surfaces: {:?}\n\
             Max duration: {:?}ms\n\
             Include load testing: {}\n\
             Include stress testing: {}\n\n\
             Return a JSON plan with stages and tools.",
            request.target,
            request.goal,
            request.target_type,
            request.attack_surfaces,
            request.max_duration_ms,
            request.include_load_testing,
            request.include_stress_testing
        );

        let messages = vec![
            serde_json::json!({
                "role": "system",
                "content": "You are a security testing orchestrator. Generate optimized execution plans."
            }),
            serde_json::json!({
                "role": "user",
                "content": prompt
            }),
        ];

        let body = serde_json::json!({
            "model": client.model(),
            "messages": messages,
            "max_tokens": 4096,
            "temperature": 0.7,
        });

        let result = client.chat_completion_from_messages(&body).await?;

        if let Some(choices) = result.get("choices") {
            if let Some(choice) = choices.get(0) {
                if let Some(content) = choice
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                {
                    let plan = self.parse_ai_response(content, request)?;
                    self.cache_plan(cache_key, plan.clone());
                    return Ok(plan);
                }
            }
        }

        Err(AiError::InvalidResponse)
    }

    fn parse_ai_response(
        &self,
        content: &str,
        request: &PlanRequest,
    ) -> crate::ai::errors::Result<ExecutionPlan> {
        if let Ok(plan) = serde_json::from_str::<ExecutionPlan>(content) {
            return Ok(plan);
        }

        let cleaned = content
            .lines()
            .skip_while(|l| !l.contains('{'))
            .take_while(|l| !l.contains("```"))
            .collect::<String>();

        if let Ok(plan) = serde_json::from_str::<ExecutionPlan>(&cleaned) {
            return Ok(plan);
        }

        tracing::warn!("Failed to parse AI response, using chain planner fallback");
        Ok(self.chain_planner.plan(request))
    }

    fn cache_plan(&self, key: String, plan: ExecutionPlan) {
        let mut cache = self.learning_cache.write();
        cache.insert(
            key,
            CachedPlan {
                plan,
                success_rate: 0.5,
                use_count: 0,
                last_used: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            },
        );
    }

    pub async fn suggest_adjustments(
        &self,
        current_plan: &ExecutionPlan,
        findings: &[crate::ai::types::ScanFinding],
        target: &str,
    ) -> crate::ai::errors::Result<AdaptivePlanSuggestion> {
        if let Some(ref client) = self.client {
            return self
                .query_ai_for_adjustments(client, current_plan, findings, target)
                .await;
        }

        Ok(AdaptivePlanSuggestion {
            suggested_modifications: vec![],
            confidence: 0.0,
            reasoning: "AI client not available".to_string(),
            estimated_improvement: None,
        })
    }

    async fn query_ai_for_adjustments(
        &self,
        client: &AiClient,
        current_plan: &ExecutionPlan,
        findings: &[crate::ai::types::ScanFinding],
        target: &str,
    ) -> Result<AdaptivePlanSuggestion> {
        let critical_count = findings.iter().filter(|f| f.severity.as_int() >= 4).count();
        let high_count = findings.iter().filter(|f| f.severity.as_int() >= 3).count();

        let prompt = format!(
            "Analyze this execution plan and suggest improvements.\n\
             Target: {}\n\
             Current stages: {:?}\n\
             Findings so far: {} critical, {} high\n\
             Suggest modifications to improve coverage or efficiency.",
            target,
            current_plan.stage_names(),
            critical_count,
            high_count
        );

        let messages = vec![
            serde_json::json!({
                "role": "system",
                "content": "You are a security testing orchestrator. Analyze plans and suggest improvements."
            }),
            serde_json::json!({
                "role": "user",
                "content": prompt
            }),
        ];

        let body = serde_json::json!({
            "model": client.model(),
            "messages": messages,
            "max_tokens": 2048,
            "temperature": 0.7,
        });

        let result = client.chat_completion_from_messages(&body).await?;

        if let Some(choices) = result.get("choices") {
            if let Some(choice) = choices.get(0) {
                if let Some(content) = choice
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                {
                    return Ok(self.parse_adjustment_response(content));
                }
            }
        }

        Ok(AdaptivePlanSuggestion {
            suggested_modifications: vec![],
            confidence: 0.0,
            reasoning: "Failed to get AI response".to_string(),
            estimated_improvement: None,
        })
    }

    fn parse_adjustment_response(&self, content: &str) -> AdaptivePlanSuggestion {
        if let Ok(mods) = serde_json::from_str::<Vec<PlanModification>>(content) {
            return AdaptivePlanSuggestion {
                suggested_modifications: mods,
                confidence: 0.8,
                reasoning: "Successfully parsed JSON modifications".to_string(),
                estimated_improvement: None,
            };
        }

        let modifications = self.extract_modifications_from_text(content);
        let confidence = if modifications.is_empty() { 0.3 } else { 0.6 };

        AdaptivePlanSuggestion {
            suggested_modifications: modifications,
            confidence,
            reasoning: content.to_string(),
            estimated_improvement: self.extract_estimated_improvement(content),
        }
    }

    fn extract_modifications_from_text(&self, content: &str) -> Vec<PlanModification> {
        let mut modifications = Vec::new();
        let lower = content.to_lowercase();

        let action_keywords = [
            (
                "add",
                "adding",
                ModificationType::AddStage,
                vec!["stage", "tool", "phase"],
            ),
            (
                "remove",
                "removing",
                ModificationType::RemoveStage,
                vec!["stage", "tool"],
            ),
            (
                "reorder",
                "reordering",
                ModificationType::ReorderStages,
                vec!["stage", "phase", "step"],
            ),
            (
                "modify",
                "modifying",
                ModificationType::ModifyStage,
                vec!["stage", "phase"],
            ),
            (
                "increase",
                "increasing",
                ModificationType::IncreaseCoverage,
                vec!["coverage"],
            ),
            (
                "reduce",
                "reducing",
                ModificationType::ReduceDuration,
                vec!["duration"],
            ),
        ];

        for (action_base, action_ing, mod_type, targets) in &action_keywords {
            if let Some(action_pos) = lower.find(action_base).or_else(|| lower.find(action_ing)) {
                for target in targets {
                    if let Some(target_pos) = lower.find(target) {
                        if (action_pos as i64 - target_pos as i64).abs() < 20 {
                            modifications.push(PlanModification {
                                modification_type: mod_type.clone(),
                                target_stage: self.extract_stage_reference(content),
                                description: format!(
                                    "AI suggested: {}",
                                    self.extract_sentence_containing(content, action_base)
                                ),
                                rationale: "Extracted from AI response".to_string(),
                            });
                            break;
                        }
                    }
                }
            }
        }

        modifications
    }

    fn extract_stage_reference(&self, content: &str) -> Option<String> {
        let stage_keywords = ["stage", "phase", "step"];
        for line in content.lines() {
            let lower = line.to_lowercase();
            for kw in &stage_keywords {
                if lower.contains(kw) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    for (i, _part) in parts.iter().enumerate() {
                        if lower.contains(kw) && i + 1 < parts.len() {
                            return Some(
                                parts[i + 1]
                                    .trim_matches(|c| c == ':' || c == ',')
                                    .to_string(),
                            );
                        }
                    }
                    let cleaned = line
                        .trim()
                        .trim_end_matches(|c| c == ':' || c == ',' || c == '.');
                    return Some(cleaned.to_string());
                }
            }
        }
        None
    }

    fn extract_sentence_containing(&self, content: &str, keyword: &str) -> String {
        for sentence in content.split(|c| c == '.' || c == '\n') {
            if sentence.to_lowercase().contains(keyword) {
                return sentence.trim().to_string();
            }
        }
        content.chars().take(100).collect::<String>() + "..."
    }

    fn extract_estimated_improvement(&self, content: &str) -> Option<String> {
        let improvement_keywords = [
            "improve",
            "improvement",
            "reduce",
            "increase",
            "efficiency",
            "coverage",
        ];
        let lower = content.to_lowercase();

        for kw in &improvement_keywords {
            if let Some(pos) = lower.find(kw) {
                let start = content[..pos].rfind('.').map(|p| p + 1).unwrap_or(0);
                let end = content[pos..]
                    .find('.')
                    .map(|p| pos + p)
                    .unwrap_or(content.len());
                let snippet = content[start..end].trim();
                if !snippet.is_empty() && snippet.len() > 10 {
                    return Some(snippet.to_string());
                }
            }
        }
        None
    }

    pub fn record_outcome(&self, plan: &ExecutionPlan, outcome: &PlanOutcome) {
        let key = format!("{}:{}", plan.total_tools, outcome.target);
        let mut cache = self.learning_cache.write();

        if let Some(cached) = cache.get_mut(&key) {
            cached.use_count += 1;
            cached.success_rate = (cached.success_rate * (cached.use_count - 1) as f32
                + if outcome.success { 1.0 } else { 0.0 })
                / cached.use_count as f32;
            cached.last_used = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
        } else {
            cache.insert(
                key,
                CachedPlan {
                    plan: plan.clone(),
                    success_rate: if outcome.success { 1.0 } else { 0.0 },
                    use_count: 1,
                    last_used: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                },
            );
        }
    }

    pub fn clear_cache(&self) {
        let mut cache = self.learning_cache.write();
        cache.clear();
    }

    pub fn cache_size(&self) -> usize {
        let cache = self.learning_cache.read();
        cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_planner() -> AiPlanner {
        use crate::tool::create_default_registry;
        let registry = create_default_registry();
        let chain_planner = ChainPlanner::new(registry);
        AiPlanner::new(None, chain_planner)
    }

    #[test]
    fn test_parse_modifications_from_text_add_stage() {
        let planner = create_test_planner();
        let content =
            "I recommend you add a new stage for SSL analysis after the initial recon phase.";
        let suggestion = planner.parse_adjustment_response(content);

        assert!(!suggestion.suggested_modifications.is_empty());
        assert!(suggestion
            .suggested_modifications
            .iter()
            .any(|m| m.modification_type == ModificationType::AddStage));
    }

    #[test]
    fn test_parse_modifications_from_text_increase_coverage() {
        let planner = create_test_planner();
        let content = "To increase coverage, you should add more payload variations.";
        let suggestion = planner.parse_adjustment_response(content);

        assert!(!suggestion.suggested_modifications.is_empty());
        assert!(suggestion
            .suggested_modifications
            .iter()
            .any(|m| m.modification_type == ModificationType::IncreaseCoverage));
    }

    #[test]
    fn test_parse_modifications_from_text_reduce_duration() {
        let planner = create_test_planner();
        let content = "Consider reducing duration of the fuzzing phase to save time.";
        let suggestion = planner.parse_adjustment_response(content);

        assert!(!suggestion.suggested_modifications.is_empty());
        assert!(suggestion
            .suggested_modifications
            .iter()
            .any(|m| m.modification_type == ModificationType::ReduceDuration));
    }

    #[test]
    fn test_parse_modifications_multiple_types() {
        let planner = create_test_planner();
        let content = "Add a stage for API testing. Increase coverage by adding more SQL injection payloads. Reduce duration of the enumeration phase.";
        let suggestion = planner.parse_adjustment_response(content);

        assert!(suggestion.suggested_modifications.len() >= 3);
    }

    #[test]
    fn test_parse_modifications_json_format() {
        let planner = create_test_planner();
        let json_content = r#"[{"modification_type":"add_stage","target_stage":"api_test","description":"Add API testing stage","rationale":"Better coverage"}]"#;
        let suggestion = planner.parse_adjustment_response(json_content);

        assert!(!suggestion.suggested_modifications.is_empty());
        assert_eq!(suggestion.confidence, 0.8);
    }

    #[test]
    fn test_parse_modifications_empty_content() {
        let planner = create_test_planner();
        let suggestion = planner.parse_adjustment_response("No specific recommendations.");

        assert!(suggestion.suggested_modifications.is_empty());
        assert_eq!(suggestion.confidence, 0.3);
    }

    #[test]
    fn test_extract_stage_reference() {
        let planner = create_test_planner();
        let content = "In the recon stage, you should add more checks.";
        let stage = planner.extract_stage_reference(content);
        assert!(stage.is_some());
    }

    #[test]
    fn test_extract_estimated_improvement() {
        let planner = create_test_planner();
        let content = "This change would improve scan efficiency by 30%.";
        let improvement = planner.extract_estimated_improvement(content);
        assert!(improvement.is_some());
    }

    #[test]
    fn test_record_outcome_updates_success_rate() {
        let planner = create_test_planner();
        let plan = ExecutionPlan {
            stages: vec![],
            estimated_duration_ms: 1000,
            total_tools: 5,
        };
        let outcome = PlanOutcome {
            success: true,
            findings_count: 3,
            severity_distribution: HashMap::new(),
            duration_ms: 1000,
            target: "http://example.com".to_string(),
        };

        planner.record_outcome(&plan, &outcome);
        assert_eq!(planner.cache_size(), 1);
    }

    #[test]
    fn test_planner_cache_clear() {
        let planner = create_test_planner();
        let plan = ExecutionPlan {
            stages: vec![],
            estimated_duration_ms: 1000,
            total_tools: 5,
        };
        let outcome = PlanOutcome {
            success: true,
            findings_count: 3,
            severity_distribution: HashMap::new(),
            duration_ms: 1000,
            target: "http://example.com".to_string(),
        };

        planner.record_outcome(&plan, &outcome);
        assert_eq!(planner.cache_size(), 1);
        planner.clear_cache();
        assert_eq!(planner.cache_size(), 0);
    }
}
