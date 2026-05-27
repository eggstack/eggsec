use crate::error::SlapperError;
use crate::tool::dispatcher::ToolDispatcher;
use crate::tool::planner::{ExecutionPlan, ToolExecution};
use crate::tool::request::ToolRequest;
use crate::tool::response::ToolResponse;
use futures::future::join_all;
use rustc_hash::{FxHashMap, FxHashSet};
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
    pub stage_results: FxHashMap<String, StageResult>,
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
                stage_results: FxHashMap::default(),
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
        let execution_order = self.resolve_stage_order(plan)?;
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

    fn resolve_stage_order(&self, plan: &ExecutionPlan) -> Result<Vec<String>, SlapperError> {
        let mut resolved: Vec<String> = Vec::new();
        let mut available: FxHashMap<String, bool> = plan
            .stages
            .iter()
            .map(|s| (s.name.clone(), false))
            .collect();
        let mut visiting: FxHashSet<String> = FxHashSet::default();

        while available.values().any(|v| !*v) {
            let mut made_progress = false;

            for stage in &plan.stages {
                if *available.get(&stage.name).unwrap_or(&true) {
                    continue;
                }

                if visiting.contains(&stage.name) {
                    return Err(SlapperError::Validation(format!(
                        "Circular dependency detected: stage '{}' depends on a stage in its own dependency chain",
                        stage.name
                    )));
                }

                let deps_resolved = stage
                    .depends_on
                    .iter()
                    .all(|dep| *available.get(dep).unwrap_or(&false));

                if deps_resolved {
                    visiting.insert(stage.name.clone());
                    resolved.push(stage.name.clone());
                    *available.get_mut(&stage.name).unwrap() = true;
                    visiting.remove(&stage.name);
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

        Ok(resolved)
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
            let depends_on = stage.depends_on.clone();
            let mut handles: Vec<_> = stage
                .tools
                .iter()
                .map(|tool| {
                    let dispatcher = self.dispatcher.clone();
                    let tool = tool.clone();
                    let target = target.to_string();
                    let stage_results = Arc::clone(&self.execution_state);
                    let depends_on = depends_on.clone();
                    async move {
                        let previous_output = {
                            let state = stage_results.read().await;
                            let deps_results: Vec<_> = depends_on
                                .iter()
                                .filter_map(|dep| state.stage_results.get(dep))
                                .collect();
                            if deps_results.is_empty() {
                                serde_json::Value::Null
                            } else {
                                serde_json::json!({
                                    "stages": deps_results.iter().map(|r| {
                                        serde_json::json!({
                                            "name": r.stage_name,
                                            "success": r.success,
                                            "tools": r.tool_results.iter().map(|t| {
                                                serde_json::json!({
                                                    "tool_id": t.tool_id,
                                                    "success": t.success,
                                                })
                                            }).collect::<Vec<_>>()
                                        })
                                    }).collect::<Vec<_>>()
                                })
                            }
                        };
                        let mut request = Self::build_request(&tool, &target);
                        request.params["results"] = previous_output;
                        let start = std::time::Instant::now();
                        let result = dispatcher.dispatch(request).await;
                        let duration = start.elapsed().as_millis() as u64;
                        (tool, result, duration)
                    }
                })
                .collect();

            let results = join_all(handles).await;
            for (tool, result, duration) in results {
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
            if let Err(e) = tx
                .send(StageProgress {
                    execution_id: state.execution_id,
                    stage: stage_name.to_string(),
                    progress: state.completed_count as f32
                        / (state.completed_count + state.failed_count + 1).max(1) as f32,
                    message: format!("Completed stage: {}", stage_name),
                })
                .await
            {
                tracing::warn!(error = %e, "Failed to send stage progress for '{}'", stage_name);
            }
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
    pub stage_results: FxHashMap<String, StageResult>,
    pub total_duration_ms: u64,
    pub overall_success: bool,
}
