use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const DEFAULT_MAX_ENTRIES: usize = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEntry {
    pub request_id: String,
    pub tool_id: String,
    pub capability: Option<String>,
    pub target: String,
    pub target_type: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub findings_count: usize,
    pub errors_count: usize,
    pub summary: String,
}

#[derive(Debug)]
pub struct ExecutionHistory {
    entries: Arc<RwLock<Vec<ExecutionEntry>>>,
    max_entries: usize,
}

impl ExecutionHistory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::with_capacity(max_entries))),
            max_entries,
        }
    }

    pub fn with_default_capacity() -> Self {
        Self::new(DEFAULT_MAX_ENTRIES)
    }

    pub fn record(
        &self,
        request: &crate::tool::ToolRequest,
        response: &crate::tool::ToolResponse,
        capability: Option<String>,
    ) {
        let mut entries = self.entries.write();

        let entry = ExecutionEntry {
            request_id: request.id.clone(),
            tool_id: request.tool.clone(),
            capability,
            target: request.target.value.clone(),
            target_type: request.target.target_type.to_string(),
            status: response.status.to_string(),
            started_at: response.metadata.started_at,
            completed_at: response.metadata.completed_at,
            duration_ms: response.metadata.duration_ms,
            findings_count: response.metadata.findings_count,
            errors_count: response.errors.len(),
            summary: generate_summary(&response),
        };

        entries.push(entry);

        if entries.len() > self.max_entries {
            entries.remove(0);
        }
    }

    pub fn get_recent(&self, limit: usize) -> Vec<ExecutionEntry> {
        let entries = self.entries.read();
        entries.iter().rev().take(limit).cloned().collect()
    }

    pub fn get_for_target(&self, target: &str) -> Vec<ExecutionEntry> {
        let entries = self.entries.read();
        entries
            .iter()
            .filter(|e| e.target.contains(target))
            .cloned()
            .collect()
    }

    pub fn get_for_tool(&self, tool_id: &str) -> Vec<ExecutionEntry> {
        let entries = self.entries.read();
        entries
            .iter()
            .filter(|e| e.tool_id == tool_id)
            .cloned()
            .collect()
    }

    pub fn get_failed(&self) -> Vec<ExecutionEntry> {
        let entries = self.entries.read();
        entries
            .iter()
            .filter(|e| e.status == "failed")
            .cloned()
            .collect()
    }

    pub fn clear(&self) {
        self.entries.write().clear();
    }

    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }
}

impl Default for ExecutionHistory {
    fn default() -> Self {
        Self::with_default_capacity()
    }
}

impl Clone for ExecutionHistory {
    fn clone(&self) -> Self {
        Self {
            entries: Arc::clone(&self.entries),
            max_entries: self.max_entries,
        }
    }
}

fn generate_summary(response: &crate::tool::ToolResponse) -> String {
    let status = &response.status;
    let findings = response.findings.len();
    let errors = response.errors.len();

    match status {
        crate::tool::ResponseStatus::Success if findings == 0 => {
            "Completed successfully with no findings".to_string()
        }
        crate::tool::ResponseStatus::Success => {
            format!("Completed with {} finding(s)", findings)
        }
        crate::tool::ResponseStatus::PartialSuccess => {
            format!(
                "Partially completed: {} findings, {} error(s)",
                findings, errors
            )
        }
        crate::tool::ResponseStatus::Failed if errors > 0 => {
            let first_error = response
                .errors
                .first()
                .map(|e| e.message.as_str())
                .unwrap_or("Unknown error");
            format!("Failed: {}", first_error)
        }
        crate::tool::ResponseStatus::Failed => "Failed".to_string(),
        crate::tool::ResponseStatus::Timeout => "Timed out".to_string(),
        crate::tool::ResponseStatus::ScopeViolation => "Scope violation detected".to_string(),
        crate::tool::ResponseStatus::Cancelled => "Cancelled".to_string(),
    }
}
