use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub id: Uuid,
    pub task_type: String,
    pub payload: serde_json::Value,
    pub priority: TaskPriority,
    pub retry_count: usize,
    pub max_retries: usize,
    pub created_at: u64,
    pub scheduled_for: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Critical,
    High,
    Normal,
    Low,
}

impl TaskPriority {
    pub fn as_u8(&self) -> u8 {
        match self {
            TaskPriority::Critical => 0,
            TaskPriority::High => 1,
            TaskPriority::Normal => 2,
            TaskPriority::Low => 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskOutcome {
    pub task_id: Uuid,
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub executed_at: u64,
}

#[derive(Clone)]
pub struct TaskScheduler {
    queue: Arc<RwLock<VecDeque<ScheduledTask>>>,
    retry_queue: Arc<RwLock<VecDeque<ScheduledTask>>>,
    max_retries: usize,
    default_priority: TaskPriority,
}

impl TaskScheduler {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(RwLock::new(VecDeque::new())),
            retry_queue: Arc::new(RwLock::new(VecDeque::new())),
            max_retries: 3,
            default_priority: TaskPriority::Normal,
        }
    }

    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn with_default_priority(mut self, priority: TaskPriority) -> Self {
        self.default_priority = priority;
        self
    }

    pub async fn schedule(&self, task: ScheduledTask) {
        let mut queue = self.queue.write().await;
        let mut tasks: Vec<_> = queue.drain(..).collect();
        tasks.push(task);
        tasks.sort_by_key(|t| t.priority.as_u8());
        *queue = tasks.into();
    }

    pub async fn schedule_batch(&self, tasks: Vec<ScheduledTask>) {
        let mut queue = self.queue.write().await;
        let mut all_tasks: Vec<_> = queue.drain(..).collect();
        all_tasks.extend(tasks);
        all_tasks.sort_by_key(|t| t.priority.as_u8());
        *queue = all_tasks.into();
    }

    pub async fn next_task(&self) -> Option<ScheduledTask> {
        let mut queue = self.queue.write().await;
        queue.pop_front()
    }

    pub async fn requeue(&self, task: ScheduledTask) -> bool {
        if task.retry_count >= task.max_retries {
            return false;
        }

        let mut retry_queue = self.retry_queue.write().await;
        let mut updated_task = task;
        updated_task.retry_count += 1;
        retry_queue.push_back(updated_task);
        true
    }

    pub async fn requeue_with_delay(&self, task: ScheduledTask, delay_ms: u64) {
        let mut updated_task = task;
        updated_task.retry_count += 1;
        updated_task.scheduled_for = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64 + delay_ms,
        );

        let mut queue = self.queue.write().await;
        let mut tasks: Vec<_> = queue.drain(..).collect();
        tasks.push(updated_task);
        tasks.sort_by_key(|t| t.priority.as_u8());
        *queue = tasks.into();
    }

    pub async fn cancel(&self, task_id: Uuid) -> bool {
        let mut queue = self.queue.write().await;
        let original_len = queue.len();
        queue.retain(|t| t.id != task_id);
        queue.len() != original_len
    }

    pub async fn pending_count(&self) -> usize {
        let queue = self.queue.read().await;
        queue.len()
    }

    pub async fn retry_count(&self) -> usize {
        let retry_queue = self.retry_queue.read().await;
        retry_queue.len()
    }

    pub async fn clear(&self) {
        let mut queue = self.queue.write().await;
        queue.clear();
        let mut retry_queue = self.retry_queue.write().await;
        retry_queue.clear();
    }

    pub async fn peek(&self) -> Option<ScheduledTask> {
        let queue = self.queue.read().await;
        queue.front().cloned()
    }

    pub fn create_task(
        &self,
        task_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> ScheduledTask {
        ScheduledTask {
            id: Uuid::new_v4(),
            task_type: task_type.into(),
            payload,
            priority: self.default_priority,
            retry_count: 0,
            max_retries: self.max_retries,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            scheduled_for: None,
        }
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TaskQueue {
    sender: mpsc::Sender<ScheduledTask>,
    receiver: Arc<RwLock<Option<mpsc::Receiver<ScheduledTask>>>>,
}

impl TaskQueue {
    pub fn new(capacity: usize) -> (Self, mpsc::Sender<ScheduledTask>) {
        let (tx, rx) = mpsc::channel::<ScheduledTask>(capacity);
        (
            Self {
                sender: tx,
                receiver: Arc::new(RwLock::new(Some(rx))),
            },
            tx,
        )
    }

    pub async fn recv(&self) -> Option<ScheduledTask> {
        let mut receiver = self.receiver.write().await;
        if let Some(rx) = receiver.as_mut() {
            rx.recv().await
        } else {
            None
        }
    }

    pub async fn send(&self, task: ScheduledTask) -> Result<(), mpsc::error::SendError<ScheduledTask>> {
        self.sender.send(task).await
    }
}
