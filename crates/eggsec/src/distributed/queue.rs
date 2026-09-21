use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

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

/// Phase F: one private queue state under a single Tokio mutex, so
/// pending→in_progress and in_progress→completed are short atomic state
/// transitions. The previous split-`RwLock` layout acquired the locks in
/// opposite orders on different paths (`dequeue` took pending→in_progress
/// while stale reassignment took in_progress→pending) and cloned every
/// dequeued task; the unified state removes the lock-order inversion and
/// moves (rather than clones) tasks between states. All operations are
/// synchronous in-memory transitions — the mutex is never held across an
/// await — so a single mutex is strictly simpler than three `RwLock`s at
/// this control-plane throughput.
#[derive(Debug, Default)]
struct QueueState {
    pending: VecDeque<Task>,
    in_progress: FxHashMap<String, Task>,
    completed: VecDeque<TaskResult>,
}

pub struct TaskQueue {
    state: Arc<Mutex<QueueState>>,
    max_size: usize,
}

impl TaskQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            state: Arc::new(Mutex::new(QueueState::default())),
            max_size,
        }
    }

    pub async fn enqueue(&self, task: Task) -> Result<(), QueueError> {
        let mut state = self.state.lock().await;

        if state.pending.len() >= self.max_size {
            return Err(QueueError::QueueFull);
        }

        state.pending.push_back(task);
        Ok(())
    }

    pub async fn dequeue(&self, worker_id: &str) -> Result<Option<Task>, QueueError> {
        let now = chrono::Utc::now().timestamp();
        let mut state = self.state.lock().await;
        let mut task = match state.pending.pop_front() {
            Some(task) => task,
            None => return Ok(None),
        };

        task.worker_id = Some(worker_id.to_string());
        task.assigned_at_secs = Some(now);

        state.in_progress.insert(task.id.clone(), task.clone());

        Ok(Some(task))
    }

    pub async fn reassign_stale_tasks(&self, timeout_secs: i64) -> Vec<Task> {
        let now = chrono::Utc::now().timestamp();

        let mut state = self.state.lock().await;
        let mut stale_tasks = Vec::new();
        state.in_progress.retain(|_id, task| {
            if let Some(assigned_at) = task.assigned_at_secs {
                if now - assigned_at > timeout_secs {
                    stale_tasks.push(task.clone());
                    return false;
                }
            }
            true
        });

        for task in &stale_tasks {
            let mut task = task.clone();
            task.worker_id = None;
            task.assigned_at_secs = None;
            state.pending.push_back(task);
        }

        stale_tasks
    }

    pub async fn complete(&self, result: TaskResult) {
        let mut state = self.state.lock().await;
        state.in_progress.remove(&result.task_id);
        state.completed.push_back(result);

        while state.completed.len() > self.max_size {
            state.completed.pop_front();
        }
    }

    pub async fn get_pending_count(&self) -> usize {
        self.state.lock().await.pending.len()
    }

    pub async fn get_in_progress_count(&self) -> usize {
        self.state.lock().await.in_progress.len()
    }

    pub async fn get_completed_count(&self) -> usize {
        self.state.lock().await.completed.len()
    }

    pub async fn get_results(&self) -> Vec<TaskResult> {
        self.state.lock().await.completed.iter().cloned().collect()
    }

    pub async fn clear(&self) {
        let mut state = self.state.lock().await;
        state.pending.clear();
        state.in_progress.clear();
        state.completed.clear();
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
