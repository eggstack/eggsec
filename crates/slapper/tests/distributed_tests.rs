use slapper::distributed::queue::{TaskQueue, TaskResult};
use slapper::distributed::{Task, TaskType};
use std::collections::HashMap;

fn make_task(id: &str, job_id: &str) -> Task {
    Task {
        id: id.to_string(),
        job_id: job_id.to_string(),
        task_type: TaskType::PortScan,
        target: "example.com".to_string(),
        payload: HashMap::new(),
    }
}

fn make_result(task_id: &str, success: bool) -> TaskResult {
    TaskResult {
        task_id: task_id.to_string(),
        success,
        output: if success {
            "ok".to_string()
        } else {
            String::new()
        },
        error: if success {
            None
        } else {
            Some("error".to_string())
        },
        duration_millis: 100,
    }
}

#[tokio::test]
async fn test_queue_new() {
    let queue = TaskQueue::new(100);
    assert_eq!(queue.get_pending_count().await, 0);
    assert_eq!(queue.get_in_progress_count().await, 0);
    assert_eq!(queue.get_completed_count().await, 0);
}

#[tokio::test]
async fn test_enqueue_dequeue() {
    let queue = TaskQueue::new(100);

    queue.enqueue(make_task("task-1", "job-1")).await.unwrap();
    assert_eq!(queue.get_pending_count().await, 1);

    let task = queue.dequeue().await.unwrap();
    assert_eq!(task.id, "task-1");
    assert_eq!(queue.get_pending_count().await, 0);
    assert_eq!(queue.get_in_progress_count().await, 1);
}

#[tokio::test]
async fn test_dequeue_empty() {
    let queue = TaskQueue::new(100);
    let task = queue.dequeue().await;
    assert!(task.is_none());
}

#[tokio::test]
async fn test_enqueue_fifo_order() {
    let queue = TaskQueue::new(100);

    queue.enqueue(make_task("first", "job-1")).await.unwrap();
    queue.enqueue(make_task("second", "job-1")).await.unwrap();
    queue.enqueue(make_task("third", "job-1")).await.unwrap();

    assert_eq!(queue.dequeue().await.unwrap().id, "first");
    assert_eq!(queue.dequeue().await.unwrap().id, "second");
    assert_eq!(queue.dequeue().await.unwrap().id, "third");
}

#[tokio::test]
async fn test_enqueue_full() {
    let queue = TaskQueue::new(2);

    queue.enqueue(make_task("task-1", "job-1")).await.unwrap();
    queue.enqueue(make_task("task-2", "job-1")).await.unwrap();

    let result = queue.enqueue(make_task("task-3", "job-1")).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_complete_task() {
    let queue = TaskQueue::new(100);

    queue.enqueue(make_task("task-1", "job-1")).await.unwrap();
    let _task = queue.dequeue().await.unwrap();
    assert_eq!(queue.get_in_progress_count().await, 1);

    queue.complete(make_result("task-1", true)).await;
    assert_eq!(queue.get_in_progress_count().await, 0);
    assert_eq!(queue.get_completed_count().await, 1);
}

#[tokio::test]
async fn test_get_results() {
    let queue = TaskQueue::new(100);

    queue.enqueue(make_task("task-1", "job-1")).await.unwrap();
    let _ = queue.dequeue().await.unwrap();
    queue.complete(make_result("task-1", true)).await;

    queue.enqueue(make_task("task-2", "job-1")).await.unwrap();
    let _ = queue.dequeue().await.unwrap();
    queue.complete(make_result("task-2", false)).await;

    let results = queue.get_results().await;
    assert_eq!(results.len(), 2);
    assert!(results[0].success);
    assert!(!results[1].success);
}

#[tokio::test]
async fn test_clear() {
    let queue = TaskQueue::new(100);

    queue.enqueue(make_task("task-1", "job-1")).await.unwrap();
    queue.enqueue(make_task("task-2", "job-1")).await.unwrap();
    let _ = queue.dequeue().await.unwrap();

    queue.clear().await;
    assert_eq!(queue.get_pending_count().await, 0);
    assert_eq!(queue.get_in_progress_count().await, 0);
    assert_eq!(queue.get_completed_count().await, 0);
}

#[tokio::test]
async fn test_completed_evicts_oldest() {
    let queue = TaskQueue::new(3);

    for i in 0..5 {
        let id = format!("task-{}", i);
        queue.enqueue(make_task(&id, "job-1")).await.unwrap();
        let _ = queue.dequeue().await.unwrap();
        queue.complete(make_result(&id, true)).await;
    }

    assert_eq!(queue.get_completed_count().await, 3);

    let results = queue.get_results().await;
    assert_eq!(results[0].task_id, "task-2");
    assert_eq!(results[1].task_id, "task-3");
    assert_eq!(results[2].task_id, "task-4");
}

#[tokio::test]
async fn test_task_serde_roundtrip() {
    let task = make_task("task-1", "job-1");
    let json = serde_json::to_string(&task).unwrap();
    let parsed: Task = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.id, "task-1");
    assert_eq!(parsed.job_id, "job-1");
    assert_eq!(parsed.target, "example.com");
}

#[tokio::test]
async fn test_task_result_serde_roundtrip() {
    let result = make_result("task-1", true);
    let json = serde_json::to_string(&result).unwrap();
    let parsed: TaskResult = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.task_id, "task-1");
    assert!(parsed.success);
    assert_eq!(parsed.duration_millis, 100);
}
