use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use crate::ai::client::AiClient;
use crate::ai::errors::{AiError, Result};
use crate::tool::planner::{ExecutionPlan, PlanRequest, ChainPlanner};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct PlanHistoryEntry {
    pub plan: ExecutionPlan,
    pub outcome: PlanOutcome,
    pub timestamp: u64,
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

    pub fn with_learning_cache(mut self, cache: Arc<RwLock<HashMap<String, CachedPlan>>>) -> Self {
        self.learning_cache = cache;
        self
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
            ExecutionPlan {
                stages: vec![],
                estimated_duration_ms: 0,
                total_tools: 0,
            }
        }
    }

    async fn query_ai_for_plan(
        &self,
        client: &AiClient,
        request: &PlanRequest,
    ) -> Result<ExecutionPlan> {
        let cache_key = format!("{}:{}", request.goal, request.target);

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

        let request_builder = client
            .apply_auth(reqwest::Client::new().post(client.api_url()).json(&body));

        let response = request_builder.send().await.map_err(AiError::from)?;

        let result: serde_json::Value = response.json().await?;

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

    fn parse_ai_response(&self, content: &str, request: &PlanRequest) -> crate::ai::errors::Result<ExecutionPlan> {
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
            return self.query_ai_for_adjustments(client, current_plan, findings, target).await;
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

        let request_builder = client
            .apply_auth(reqwest::Client::new().post(client.api_url()).json(&body));

        let response = request_builder.send().await.map_err(AiError::from)?;

        let result: serde_json::Value = response.json().await?;

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
        AdaptivePlanSuggestion {
            suggested_modifications: vec![],
            confidence: 0.5,
            reasoning: content.to_string(),
            estimated_improvement: None,
        }
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
