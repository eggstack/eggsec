use rustc_hash::FxHashMap;
use slapper::distributed::queue::{TaskQueue, TaskResult};
use slapper::distributed::{RemoteClient, RemoteListener, Task, TaskType};

fn make_task(id: &str, job_id: &str) -> Task {
    Task {
        id: id.to_string(),
        job_id: job_id.to_string(),
        task_type: TaskType::PortScan,
        target: "example.com".to_string(),
        payload: FxHashMap::default(),
        worker_id: None,
        assigned_at_secs: None,
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

    let task = queue.dequeue("worker-1").await.unwrap().unwrap();
    assert_eq!(task.id, "task-1");
    assert_eq!(queue.get_pending_count().await, 0);
    assert_eq!(queue.get_in_progress_count().await, 1);
}

#[tokio::test]
async fn test_dequeue_empty() {
    let queue = TaskQueue::new(100);
    let task = queue.dequeue("worker-1").await.unwrap();
    assert!(task.is_none());
}

#[tokio::test]
async fn test_enqueue_fifo_order() {
    let queue = TaskQueue::new(100);

    queue.enqueue(make_task("first", "job-1")).await.unwrap();
    queue.enqueue(make_task("second", "job-1")).await.unwrap();
    queue.enqueue(make_task("third", "job-1")).await.unwrap();

    assert_eq!(
        queue.dequeue("worker-1").await.unwrap().unwrap().id,
        "first"
    );
    assert_eq!(
        queue.dequeue("worker-2").await.unwrap().unwrap().id,
        "second"
    );
    assert_eq!(
        queue.dequeue("worker-3").await.unwrap().unwrap().id,
        "third"
    );
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
    let _task = queue.dequeue("worker-1").await.unwrap().unwrap();
    assert_eq!(queue.get_in_progress_count().await, 1);

    queue.complete(make_result("task-1", true)).await;
    assert_eq!(queue.get_in_progress_count().await, 0);
    assert_eq!(queue.get_completed_count().await, 1);
}

#[tokio::test]
async fn test_get_results() {
    let queue = TaskQueue::new(100);

    queue.enqueue(make_task("task-1", "job-1")).await.unwrap();
    let _ = queue.dequeue("worker-1").await.unwrap().unwrap();
    queue.complete(make_result("task-1", true)).await;

    queue.enqueue(make_task("task-2", "job-1")).await.unwrap();
    let _ = queue.dequeue("worker-2").await.unwrap().unwrap();
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
    let _ = queue.dequeue("worker-1").await.unwrap().unwrap();

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
        let worker_id = format!("worker-{}", i);
        let _ = queue.dequeue(&worker_id).await.unwrap().unwrap();
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

#[tokio::test]
async fn test_listener_client_auth_success() {
    let psk = "test-psk-12345";

    let listener_clone = RemoteListener::new(psk.to_string());
    let (addr_tx, addr_rx) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        // Use a TcpListener to get a free port
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = std_listener.local_addr().unwrap().port();
        let _ = addr_tx.send(port);
        drop(std_listener);

        // Now start the actual listener
        listener_clone.start(port).await.unwrap();
    });

    let port = addr_rx.await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut client = RemoteClient::new_plaintext(psk.to_string());
    let result = client
        .register_worker(
            "127.0.0.1",
            port,
            "worker-1".to_string(),
            "test-host".to_string(),
            vec!["PortScan".to_string()],
        )
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_listener_client_invalid_psk() {
    let psk = "correct-psk";

    let listener_clone = RemoteListener::new(psk.to_string());
    let (addr_tx, addr_rx) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = std_listener.local_addr().unwrap().port();
        let _ = addr_tx.send(port);
        drop(std_listener);

        listener_clone.start(port).await.unwrap();
    });

    let port = addr_rx.await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Try with wrong PSK
    let mut client = RemoteClient::new_plaintext("wrong-psk".to_string());
    let result = client
        .register_worker(
            "127.0.0.1",
            port,
            "worker-1".to_string(),
            "test-host".to_string(),
            vec!["PortScan".to_string()],
        )
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_listener_task_assignment_cycle() {
    let psk = "task-cycle-psk";

    let listener_clone = RemoteListener::new(psk.to_string());
    let (addr_tx, addr_rx) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = std_listener.local_addr().unwrap().port();
        let _ = addr_tx.send(port);
        drop(std_listener);

        // We need to access the task queue to enqueue tasks
        // Since we can't directly access it, we'll use a simpler test pattern
        listener_clone.start(port).await.unwrap();
    });

    let port = addr_rx.await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Connect and register
    let mut client = RemoteClient::new_plaintext(psk.to_string());
    let reg_result = client
        .register_worker(
            "127.0.0.1",
            port,
            "worker-1".to_string(),
            "test-host".to_string(),
            vec!["PortScan".to_string()],
        )
        .await;
    assert!(reg_result.is_ok());

    // Request tasks (queue is empty, should get empty list)
    let tasks = client
        .request_tasks("127.0.0.1", port, "worker-1".to_string(), 5)
        .await
        .unwrap();
    assert!(tasks.is_empty());
}

#[tokio::test]
async fn test_listener_heartbeat() {
    let psk = "heartbeat-psk";

    let listener_clone = RemoteListener::new(psk.to_string());
    let (addr_tx, addr_rx) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = std_listener.local_addr().unwrap().port();
        let _ = addr_tx.send(port);
        drop(std_listener);

        listener_clone.start(port).await.unwrap();
    });

    let port = addr_rx.await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut client = RemoteClient::new_plaintext(psk.to_string());
    let result = client
        .send_heartbeat(
            "127.0.0.1",
            port,
            "worker-1".to_string(),
            "idle".to_string(),
        )
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_listener_connection_count() {
    let psk = "count-psk";
    let listener = RemoteListener::new(psk.to_string());
    let count = listener.connection_count().await;
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_generate_psk_unique() {
    let psk1 = slapper::distributed::generate_psk();
    let psk2 = slapper::distributed::generate_psk();
    assert_ne!(psk1, psk2);
    assert_eq!(psk1.len(), 64);
    assert_eq!(psk2.len(), 64);
}
