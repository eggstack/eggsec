use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub job_id: String,
    pub task_type: crate::distributed::TaskType,
    pub target: String,
    pub payload: FxHashMap<String, serde_json::Value>,
    #[serde(default)]
    pub worker_id: Option<String>,
    #[serde(default)]
    pub assigned_at_secs: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub duration_millis: u64,
}

pub struct TaskQueue {
    pending: Arc<RwLock<VecDeque<Task>>>,
    in_progress: Arc<RwLock<FxHashMap<String, Task>>>,
    completed: Arc<RwLock<VecDeque<TaskResult>>>,
    max_size: usize,
}

impl TaskQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            pending: Arc::new(RwLock::new(VecDeque::new())),
            in_progress: Arc::new(RwLock::new(FxHashMap::default())),
            completed: Arc::new(RwLock::new(VecDeque::new())),
            max_size,
        }
    }

    pub async fn enqueue(&self, task: Task) -> Result<(), QueueError> {
        let mut pending = self.pending.write().await;

        if pending.len() >= self.max_size {
            return Err(QueueError::QueueFull);
        }

        pending.push_back(task);
        Ok(())
    }

    pub async fn dequeue(&self, worker_id: &str) -> Result<Option<Task>, QueueError> {
        let now = chrono::Utc::now().timestamp();
        let mut pending = self.pending.write().await;
        let mut task = match pending.pop_front() {
            Some(t) => t,
            None => return Ok(None),
        };

        task.worker_id = Some(worker_id.to_string());
        task.assigned_at_secs = Some(now);

        let mut in_progress = self.in_progress.write().await;
        in_progress.insert(task.id.clone(), task.clone());

        Ok(Some(task))
    }

    pub async fn reassign_stale_tasks(&self, timeout_secs: i64) -> Vec<Task> {
        let mut stale_tasks = Vec::new();
        let now = chrono::Utc::now().timestamp();

        let mut in_progress = self.in_progress.write().await;
        let mut pending = self.pending.write().await;

        in_progress.retain(|_id, task| {
            if let Some(assigned_at) = task.assigned_at_secs {
                if now - assigned_at > timeout_secs {
                    stale_tasks.push(task.clone());
                    return false;
                }
            }
            true
        });

        for task in stale_tasks.iter() {
            let mut t = task.clone();
            t.worker_id = None;
            t.assigned_at_secs = None;
            pending.push_back(t);
        }

        drop(in_progress);
        drop(pending);

        stale_tasks
    }

    pub async fn complete(&self, result: TaskResult) {
        let task_id = result.task_id.clone();

        {
            let mut in_progress = self.in_progress.write().await;
            in_progress.remove(&task_id);
        }

        {
            let mut completed = self.completed.write().await;
            completed.push_back(result);

            while completed.len() > self.max_size {
                completed.pop_front();
            }
        }
    }

    pub async fn get_pending_count(&self) -> usize {
        let pending = self.pending.read().await;
        pending.len()
    }

    pub async fn get_in_progress_count(&self) -> usize {
        let in_progress = self.in_progress.read().await;
        in_progress.len()
    }

    pub async fn get_completed_count(&self) -> usize {
        let completed = self.completed.read().await;
        completed.len()
    }

    pub async fn get_results(&self) -> Vec<TaskResult> {
        let completed = self.completed.read().await;
        completed.iter().cloned().collect()
    }

    pub async fn clear(&self) {
        let mut pending = self.pending.write().await;
        pending.clear();

        let mut in_progress = self.in_progress.write().await;
        in_progress.clear();

        let mut completed = self.completed.write().await;
        completed.clear();
    }
}

#[derive(Debug, Clone)]
pub enum QueueError {
    QueueFull,
    TaskNotFound,
}

impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueError::QueueFull => write!(f, "Queue is full"),
            QueueError::TaskNotFound => write!(f, "Task not found"),
        }
    }
}

impl std::error::Error for QueueError {}
