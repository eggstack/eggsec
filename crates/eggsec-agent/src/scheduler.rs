use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Leased,
    Completed,
    Failed,
    Cancelled,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn is_due(task: &ScheduledTask, now: u64) -> bool {
    task.scheduled_for.map(|s| s <= now).unwrap_or(true)
}

fn lease_expired(task: &ScheduledTask, now: u64) -> bool {
    task.leased_until.map(|l| now >= l).unwrap_or(false)
}

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
    pub status: TaskStatus,
    pub assigned_agent_id: Option<Uuid>,
    pub leased_until: Option<u64>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub completed_at: Option<u64>,
    pub updated_at: Option<u64>,
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
    max_retries: usize,
    default_priority: TaskPriority,
    default_retry_delay_ms: u64,
}

impl TaskScheduler {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(RwLock::new(VecDeque::new())),
            max_retries: eggsec_core::constants::DEFAULT_MAX_RETRIES as usize,
            default_priority: TaskPriority::Normal,
            default_retry_delay_ms: eggsec_core::constants::DEFAULT_SCHEDULER_RETRY_DELAY_MS,
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

    pub fn with_default_retry_delay(mut self, delay_ms: u64) -> Self {
        self.default_retry_delay_ms = delay_ms;
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

    pub async fn lease_next_task(
        &self,
        agent_id: Uuid,
        lease_duration_ms: u64,
    ) -> Option<ScheduledTask> {
        let mut queue = self.queue.write().await;
        let now = now_ms();
        let expiry = now + lease_duration_ms;

        // Reclaim expired leases so stranded tasks can be picked up again.
        for task in queue.iter_mut() {
            if task.status == TaskStatus::Leased && lease_expired(task, now) {
                task.status = TaskStatus::Pending;
                task.assigned_agent_id = None;
                task.leased_until = None;
                task.updated_at = Some(now);
            }
        }

        if let Some(pos) = queue
            .iter()
            .position(|t| t.status == TaskStatus::Pending && is_due(t, now))
        {
            let task = &mut queue[pos];
            task.status = TaskStatus::Leased;
            task.assigned_agent_id = Some(agent_id);
            task.leased_until = Some(expiry);
            task.updated_at = Some(now);
            Some(task.clone())
        } else {
            None
        }
    }

    pub async fn lease_task(&self, task_id: Uuid, agent_id: Uuid, lease_duration_ms: u64) -> bool {
        let mut queue = self.queue.write().await;
        let now = now_ms();

        if let Some(task) = queue.iter_mut().find(|t| t.id == task_id) {
            if task.status == TaskStatus::Leased {
                if lease_expired(task, now) {
                    task.status = TaskStatus::Pending;
                    task.assigned_agent_id = None;
                    task.leased_until = None;
                } else {
                    return false;
                }
            }

            if task.status != TaskStatus::Pending || !is_due(task, now) {
                return false;
            }
            task.status = TaskStatus::Leased;
            task.assigned_agent_id = Some(agent_id);
            task.leased_until = Some(now + lease_duration_ms);
            task.updated_at = Some(now);
            true
        } else {
            false
        }
    }

    pub async fn submit_result(
        &self,
        task_id: Uuid,
        success: bool,
        result: Option<serde_json::Value>,
        error: Option<String>,
    ) -> bool {
        let mut queue = self.queue.write().await;
        let now = now_ms();

        if let Some(task) = queue
            .iter_mut()
            .find(|t| t.id == task_id && t.status == TaskStatus::Leased)
        {
            if success {
                task.status = TaskStatus::Completed;
                task.result = result;
                task.completed_at = Some(now);
                task.assigned_agent_id = None;
                task.leased_until = None;
            } else {
                if task.retry_count < task.max_retries {
                    task.retry_count += 1;
                    task.status = TaskStatus::Pending;
                    task.scheduled_for = Some(now + self.default_retry_delay_ms);
                    task.error = error;
                    task.assigned_agent_id = None;
                    task.leased_until = None;
                } else {
                    task.status = TaskStatus::Failed;
                    task.error = error;
                    task.completed_at = Some(now);
                    task.assigned_agent_id = None;
                    task.leased_until = None;
                }
            }
            task.updated_at = Some(now);
            true
        } else {
            false
        }
    }

    pub async fn requeue(&self, task: ScheduledTask) -> bool {
        if task.retry_count >= task.max_retries {
            return false;
        }

        let mut updated_task = task;
        updated_task.retry_count += 1;
        updated_task.status = TaskStatus::Pending;
        self.schedule(updated_task).await;
        true
    }

    pub async fn requeue_with_delay(&self, task: ScheduledTask, delay_ms: u64) {
        let mut updated_task = task;
        updated_task.retry_count += 1;
        updated_task.scheduled_for = Some(now_ms() + delay_ms);
        updated_task.status = TaskStatus::Pending;
        self.schedule(updated_task).await;
    }

    pub async fn cancel(&self, task_id: Uuid) -> bool {
        let mut queue = self.queue.write().await;
        let now = now_ms();

        if let Some(task) = queue.iter_mut().find(|t| t.id == task_id) {
            match task.status {
                TaskStatus::Pending | TaskStatus::Leased | TaskStatus::Failed => {
                    task.status = TaskStatus::Cancelled;
                    task.updated_at = Some(now);
                    return true;
                }
                _ => return false,
            }
        }
        false
    }

    pub async fn pending_count(&self) -> usize {
        let queue = self.queue.read().await;
        queue
            .iter()
            .filter(|t| t.status == TaskStatus::Pending)
            .count()
    }

    pub async fn retry_count(&self) -> usize {
        let queue = self.queue.read().await;
        queue
            .iter()
            .filter(|t| t.status == TaskStatus::Failed && t.retry_count < t.max_retries)
            .count()
    }

    pub async fn clear(&self) {
        let mut queue = self.queue.write().await;
        queue.clear();
    }

    pub async fn peek(&self) -> Option<ScheduledTask> {
        let queue = self.queue.read().await;
        queue.front().cloned()
    }

    pub async fn get_task(&self, task_id: Uuid) -> Option<ScheduledTask> {
        let queue = self.queue.read().await;
        queue.iter().find(|t| t.id == task_id).cloned()
    }

    pub async fn list_by_status(&self, status: TaskStatus) -> Vec<ScheduledTask> {
        let queue = self.queue.read().await;
        queue
            .iter()
            .filter(|t| t.status == status)
            .cloned()
            .collect()
    }

    pub async fn list_all_tasks(&self) -> Vec<ScheduledTask> {
        let queue = self.queue.read().await;
        queue.iter().cloned().collect()
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
            created_at: now_ms(),
            scheduled_for: None,
            status: TaskStatus::Pending,
            assigned_agent_id: None,
            leased_until: None,
            result: None,
            error: None,
            completed_at: None,
            updated_at: None,
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
                sender: tx.clone(),
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

    pub async fn send(
        &self,
        task: ScheduledTask,
    ) -> Result<(), mpsc::error::SendError<ScheduledTask>> {
        self.sender.send(task).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lease_next_task_reclaims_expired_lease() {
        let scheduler = TaskScheduler::new();
        let mut task = scheduler.create_task("scan", serde_json::json!({}));
        task.status = TaskStatus::Leased;
        task.assigned_agent_id = Some(Uuid::new_v4());
        task.leased_until = Some(now_ms().saturating_sub(1));

        scheduler.schedule(task.clone()).await;

        let next = scheduler.lease_next_task(Uuid::new_v4(), 10_000).await;
        assert!(next.is_some());
        assert_eq!(next.unwrap().id, task.id);
    }
}
