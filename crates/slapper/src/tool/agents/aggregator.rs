use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AggregatedResult {
    pub execution_id: Uuid,
    pub total_tasks: usize,
    pub successful_tasks: usize,
    pub failed_tasks: usize,
    pub total_duration_ms: u64,
    pub stage_summaries: Vec<StageSummary>,
    pub tool_summaries: Vec<ToolSummary>,
    pub errors: Vec<AggregatedError>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StageSummary {
    pub stage_name: String,
    pub total_tools: usize,
    pub successful_tools: usize,
    pub failed_tools: usize,
    pub duration_ms: u64,
    pub success_rate: f32,
}

#[derive(Debug, Clone)]
pub struct ToolSummary {
    pub tool_id: String,
    pub tool_name: String,
    pub total_executions: usize,
    pub successful_executions: usize,
    pub failed_executions: usize,
    pub average_duration_ms: f64,
    pub total_duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct AggregatedError {
    pub tool_id: String,
    pub error_message: String,
    pub occurrence_count: usize,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

#[derive(Clone)]
pub struct ResultAggregator {
    results: Arc<RwLock<HashMap<Uuid, AggregatedResult>>>,
    in_progress: Arc<RwLock<HashMap<Uuid, InProgressExecution>>>,
}

struct InProgressExecution {
    #[allow(dead_code)]
    execution_id: Uuid,
    #[allow(dead_code)]
    started_at: DateTime<Utc>,
    stage_results: HashMap<String, StageResultAccumulator>,
    tool_results: HashMap<String, ToolResultAccumulator>,
    errors: Vec<ErrorAccumulator>,
}

struct StageResultAccumulator {
    stage_name: String,
    tool_results: Vec<bool>,
    duration_ms: u64,
}

struct ToolResultAccumulator {
    tool_id: String,
    tool_name: String,
    success_count: usize,
    failure_count: usize,
    durations_ms: Vec<u64>,
}

struct ErrorAccumulator {
    tool_id: String,
    error_message: String,
    count: usize,
    first_seen: DateTime<Utc>,
    last_seen: DateTime<Utc>,
}

impl ResultAggregator {
    pub fn new() -> Self {
        Self {
            results: Arc::new(RwLock::new(HashMap::new())),
            in_progress: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_execution(&self, execution_id: Uuid) {
        let mut in_progress = self.in_progress.write().await;
        in_progress.insert(
            execution_id,
            InProgressExecution {
                execution_id,
                started_at: Utc::now(),
                stage_results: HashMap::new(),
                tool_results: HashMap::new(),
                errors: Vec::new(),
            },
        );
    }

    pub async fn record_stage_start(&self, execution_id: Uuid, stage_name: &str) {
        let mut in_progress = self.in_progress.write().await;
        if let Some(exec) = in_progress.get_mut(&execution_id) {
            exec.stage_results.insert(
                stage_name.to_string(),
                StageResultAccumulator {
                    stage_name: stage_name.to_string(),
                    tool_results: Vec::new(),
                    duration_ms: 0,
                },
            );
        }
    }

    pub async fn record_stage_result(
        &self,
        execution_id: Uuid,
        stage_name: &str,
        tool_id: &str,
        success: bool,
        duration_ms: u64,
    ) {
        let mut in_progress = self.in_progress.write().await;
        if let Some(exec) = in_progress.get_mut(&execution_id) {
            if let Some(stage) = exec.stage_results.get_mut(stage_name) {
                stage.tool_results.push(success);
                stage.duration_ms += duration_ms;
            }

            let tool_entry = exec
                .tool_results
                .entry(tool_id.to_string())
                .or_insert_with(|| ToolResultAccumulator {
                    tool_id: tool_id.to_string(),
                    tool_name: tool_id.to_string(),
                    success_count: 0,
                    failure_count: 0,
                    durations_ms: Vec::new(),
                });

            if success {
                tool_entry.success_count += 1;
            } else {
                tool_entry.failure_count += 1;
            }
            tool_entry.durations_ms.push(duration_ms);
        }
    }

    pub async fn record_error(&self, execution_id: Uuid, tool_id: &str, error_message: &str) {
        let mut in_progress = self.in_progress.write().await;
        if let Some(exec) = in_progress.get_mut(&execution_id) {
            let now = Utc::now();
            if let Some(existing) = exec
                .errors
                .iter_mut()
                .find(|e| e.tool_id == tool_id && e.error_message == error_message)
            {
                existing.count += 1;
                existing.last_seen = now;
            } else {
                exec.errors.push(ErrorAccumulator {
                    tool_id: tool_id.to_string(),
                    error_message: error_message.to_string(),
                    count: 1,
                    first_seen: now,
                    last_seen: now,
                });
            }
        }
    }

    pub async fn finish_execution(&self, execution_id: Uuid) -> Option<AggregatedResult> {
        let in_progress_data = {
            let mut in_progress = self.in_progress.write().await;
            in_progress.remove(&execution_id)
        };

        if let Some(exec) = in_progress_data {
            let stage_summaries: Vec<StageSummary> = exec
                .stage_results
                .values()
                .map(|s| {
                    let total = s.tool_results.len();
                    let successful = s.tool_results.iter().filter(|&&v| v).count();
                    StageSummary {
                        stage_name: s.stage_name.clone(),
                        total_tools: total,
                        successful_tools: successful,
                        failed_tools: total - successful,
                        duration_ms: s.duration_ms,
                        success_rate: if total > 0 {
                            successful as f32 / total as f32
                        } else {
                            0.0
                        },
                    }
                })
                .collect();

            let tool_summaries: Vec<ToolSummary> = exec
                .tool_results
                .values()
                .map(|t| {
                    let total_duration: u64 = t.durations_ms.iter().sum();
                    ToolSummary {
                        tool_id: t.tool_id.clone(),
                        tool_name: t.tool_name.clone(),
                        total_executions: t.success_count + t.failure_count,
                        successful_executions: t.success_count,
                        failed_executions: t.failure_count,
                        average_duration_ms: if !t.durations_ms.is_empty() {
                            total_duration as f64 / t.durations_ms.len() as f64
                        } else {
                            0.0
                        },
                        total_duration_ms: total_duration,
                    }
                })
                .collect();

            let errors: Vec<AggregatedError> = exec
                .errors
                .into_iter()
                .map(|e| AggregatedError {
                    tool_id: e.tool_id,
                    error_message: e.error_message,
                    occurrence_count: e.count,
                    first_seen: e.first_seen,
                    last_seen: e.last_seen,
                })
                .collect();

            let total_tasks: usize = tool_summaries.iter().map(|t| t.total_executions).sum();
            let successful_tasks: usize =
                tool_summaries.iter().map(|t| t.successful_executions).sum();
            let failed_tasks: usize = tool_summaries.iter().map(|t| t.failed_executions).sum();
            let total_duration_ms: u64 = tool_summaries.iter().map(|t| t.total_duration_ms).sum();

            let result = AggregatedResult {
                execution_id,
                total_tasks,
                successful_tasks,
                failed_tasks,
                total_duration_ms,
                stage_summaries,
                tool_summaries,
                errors,
                completed_at: Utc::now(),
            };

            let mut results = self.results.write().await;
            results.insert(execution_id, result.clone());

            Some(result)
        } else {
            None
        }
    }

    pub async fn get_result(&self, execution_id: Uuid) -> Option<AggregatedResult> {
        let results = self.results.read().await;
        results.get(&execution_id).cloned()
    }

    pub async fn get_all_results(&self) -> Vec<AggregatedResult> {
        let results = self.results.read().await;
        results.values().cloned().collect()
    }

    pub async fn clear_old_results(&self, older_than: chrono::Duration) {
        let cutoff = Utc::now() - older_than;
        let mut results = self.results.write().await;
        results.retain(|_, r| r.completed_at > cutoff);
    }
}

impl Default for ResultAggregator {
    fn default() -> Self {
        Self::new()
    }
}
