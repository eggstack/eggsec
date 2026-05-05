use crate::error::SlapperError;
use crate::tool::dispatcher::ToolDispatcher;
use crate::tool::planner::{ExecutionPlan, ToolExecution};
use crate::tool::request::ToolRequest;
use crate::tool::response::ToolResponse;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

#[derive(Clone)]
pub struct Orchestrator {
    dispatcher: ToolDispatcher,
    execution_state: Arc<RwLock<ExecutionState>>,
}

#[derive(Debug)]
pub struct ExecutionState {
    pub execution_id: Uuid,
    pub stage_results: HashMap<String, StageResult>,
    pub completed_count: usize,
    pub failed_count: usize,
}

#[derive(Debug, Clone)]
pub struct StageResult {
    pub stage_name: String,
    pub success: bool,
    pub tool_results: Vec<StageToolResult>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct StageToolResult {
    pub tool_id: String,
    pub capability: Option<String>,
    pub success: bool,
    pub response: Option<ToolResponse>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

impl Orchestrator {
    pub fn new(dispatcher: ToolDispatcher) -> Self {
        Self {
            dispatcher,
            execution_state: Arc::new(RwLock::new(ExecutionState {
                execution_id: Uuid::new_v4(),
                stage_results: HashMap::new(),
                completed_count: 0,
                failed_count: 0,
            })),
        }
    }

    pub async fn execute_plan(
        &self,
        plan: &ExecutionPlan,
        target: &str,
        progress_tx: Option<mpsc::Sender<StageProgress>>,
    ) -> Result<ExecutionResult, SlapperError> {
        let execution_order = self.resolve_stage_order(plan);
        let overall_start = std::time::Instant::now();

        for stage in &execution_order {
            self.execute_stage(plan, stage, target, progress_tx.clone())
                .await?;
        }

        let state = self.execution_state.read().await;
        let total_duration = overall_start.elapsed().as_millis() as u64;

        Ok(ExecutionResult {
            execution_id: state.execution_id,
            stage_results: state.stage_results.clone(),
            total_duration_ms: total_duration,
            overall_success: state.failed_count == 0,
        })
    }

    fn resolve_stage_order(&self, plan: &ExecutionPlan) -> Vec<String> {
        let mut resolved: Vec<String> = Vec::new();
        let mut available: HashMap<String, bool> = plan
            .stages
            .iter()
            .map(|s| (s.name.clone(), false))
            .collect();

        while available.values().any(|v| !*v) {
            let mut made_progress = false;

            for stage in &plan.stages {
                if *available.get(&stage.name).unwrap_or(&true) {
                    continue;
                }

                let deps_resolved = stage
                    .depends_on
                    .iter()
                    .all(|dep| *available.get(dep).unwrap_or(&false));

                if deps_resolved {
                    resolved.push(stage.name.clone());
                    *available.get_mut(&stage.name).unwrap() = true;
                    made_progress = true;
                }
            }

            if !made_progress {
                for (name, resolved_flag) in available.iter_mut() {
                    if !*resolved_flag {
                        resolved.push(name.clone());
                        *resolved_flag = true;
                    }
                }
                break;
            }
        }

        resolved
    }

    async fn execute_stage(
        &self,
        plan: &ExecutionPlan,
        stage_name: &str,
        target: &str,
        progress_tx: Option<mpsc::Sender<StageProgress>>,
    ) -> Result<(), SlapperError> {
        let stage = plan
            .stages
            .iter()
            .find(|s| s.name == stage_name)
            .ok_or_else(|| SlapperError::Config(format!("Stage '{}' not found", stage_name)))?;

        let stage_start = std::time::Instant::now();
        let mut tool_results = Vec::new();

        if stage.parallel {
            let handles: Vec<_> = stage
                .tools
                .iter()
                .map(|tool| {
                    let dispatcher = self.dispatcher.clone();
                    let tool = tool.clone();
                    let target = target.to_string();
                    async move {
                        let request = Self::build_request(&tool, &target);
                        let start = std::time::Instant::now();
                        let result = dispatcher.dispatch(request).await;
                        let duration = start.elapsed().as_millis() as u64;
                        (tool, result, duration)
                    }
                })
                .collect();

            for handle in handles {
                let (tool, result, duration) = handle.await;
                let tool_result = self.process_tool_result(tool, result, duration);
                tool_results.push(tool_result);
            }
        } else {
            for tool in &stage.tools {
                let request = Self::build_request(tool, target);
                let start = std::time::Instant::now();
                let result = self.dispatcher.dispatch(request).await;
                let duration = start.elapsed().as_millis() as u64;
                let tool_result = self.process_tool_result(tool.clone(), result, duration);
                tool_results.push(tool_result);
            }
        }

        let duration_ms = stage_start.elapsed().as_millis() as u64;
        let success = tool_results.iter().all(|r| r.success);

        let stage_result = StageResult {
            stage_name: stage_name.to_string(),
            success,
            tool_results,
            duration_ms,
        };

        let mut state = self.execution_state.write().await;
        state
            .stage_results
            .insert(stage_name.to_string(), stage_result);

        if let Some(tx) = progress_tx {
            let _ = tx
                .send(StageProgress {
                    execution_id: state.execution_id,
                    stage: stage_name.to_string(),
                    progress: state.completed_count as f32
                        / (state.completed_count + state.failed_count + 1).max(1) as f32,
                    message: format!("Completed stage: {}", stage_name),
                })
                .await;
        }

        Ok(())
    }

    fn build_request(tool: &ToolExecution, target: &str) -> ToolRequest {
        ToolRequest {
            id: Uuid::new_v4().to_string(),
            tool: tool.tool_id.clone(),
            target: crate::tool::request::Target::url(target),
            params: serde_json::json!({}),
            options: Default::default(),
            cancellation_token: None,
        }
    }

    fn process_tool_result(
        &self,
        tool: ToolExecution,
        result: Result<ToolResponse, SlapperError>,
        duration_ms: u64,
    ) -> StageToolResult {
        match result {
            Ok(response) => StageToolResult {
                tool_id: tool.tool_id,
                capability: tool.capability,
                success: response.status == crate::tool::response::ResponseStatus::Success,
                response: Some(response),
                error: None,
                duration_ms,
            },
            Err(e) => StageToolResult {
                tool_id: tool.tool_id,
                capability: tool.capability,
                success: false,
                response: None,
                error: Some(e.to_string()),
                duration_ms,
            },
        }
    }

    pub async fn get_execution_state(&self) -> ExecutionState {
        let state = self.execution_state.read().await;
        ExecutionState {
            execution_id: state.execution_id,
            stage_results: state.stage_results.clone(),
            completed_count: state.completed_count,
            failed_count: state.failed_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StageProgress {
    pub execution_id: Uuid,
    pub stage: String,
    pub progress: f32,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub execution_id: Uuid,
    pub stage_results: HashMap<String, StageResult>,
    pub total_duration_ms: u64,
    pub overall_success: bool,
}
