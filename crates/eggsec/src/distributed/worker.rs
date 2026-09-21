#[cfg(any(feature = "tool-api", feature = "rest-api", feature = "grpc-api"))]
use crate::config::{metadata_for_tool_id, ExecutionSurface};
use crate::config::{EnforcementContext, LoadedScope};
use crate::distributed::remote::{CoordinatorSession, SessionConfig};
#[cfg(any(feature = "tool-api", feature = "rest-api", feature = "grpc-api"))]
use crate::distributed::TaskType;
use crate::distributed::{Task, TaskResult, CAPABILITIES};
use crate::error::{EggsecError, Result};
use crate::scanner::endpoints::EndpointScanConfig;
#[cfg(any(feature = "tool-api", feature = "rest-api", feature = "grpc-api"))]
use crate::tool::{
    create_default_registry, EnforcedDispatcher, Target, ToolDispatcher, ToolRequest,
};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::{mpsc, watch, Mutex, Semaphore};
use tokio::task::JoinHandle;

const MAX_TASKS_PER_REQUEST: usize = 5;

/// Task-processing timeout: no distributed task may outlive this bound or
/// detach from shutdown/cancellation.
const TASK_PROCESSING_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// Single capacity-accounting truth shared by task acquisition and task
/// execution (Phase D).
///
/// `reserved` counts coordinator-assigned work that is not yet terminal
/// (queued locally plus actively executing). The invariant
/// `reserved <= max_concurrency` is enforced by atomic compare-and-swap on
/// every reservation, so two loops can never observe the same free slot and
/// over-reserve. Execution permits bound live tasks; statistics derive from
/// the same counts (see `WorkerStats::tasks_in_progress`).
#[derive(Debug)]
pub(crate) struct CapacityTracker {
    max_concurrency: usize,
    reserved: AtomicUsize,
    execution_permits: Arc<Semaphore>,
}

impl CapacityTracker {
    pub(crate) fn new(max_concurrency: usize) -> Self {
        Self {
            max_concurrency,
            reserved: AtomicUsize::new(0),
            execution_permits: Arc::new(Semaphore::new(max_concurrency)),
        }
    }

    /// Currently reservable slots (assigned-but-not-terminal subtracted).
    pub(crate) fn available(&self) -> usize {
        self.max_concurrency
            .saturating_sub(self.reserved.load(Ordering::Acquire))
    }

    /// How many tasks to ask the coordinator for this tick: at most
    /// `MAX_TASKS_PER_REQUEST` and never more than locally available
    /// capacity. Zero means the tick must skip the network operation.
    pub(crate) fn request_size(&self) -> usize {
        MAX_TASKS_PER_REQUEST.min(self.available())
    }

    /// Atomically reserve up to `want` slots. Returns the admitted count,
    /// which is always `<= want` and keeps `reserved <= max_concurrency`.
    /// A shortfall means the coordinator over-delivered (or a concurrent
    /// tick reserved first); the caller must not execute beyond the
    /// admitted count.
    pub(crate) fn try_reserve(&self, want: usize) -> usize {
        match self
            .reserved
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                let room = self.max_concurrency.saturating_sub(current);
                if room == 0 {
                    None
                } else {
                    Some(current + room.min(want))
                }
            }) {
            Ok(previous) => (self.max_concurrency.saturating_sub(previous)).min(want),
            Err(_) => 0,
        }
    }

    /// Release one reservation (exactly once per admitted task, on every
    /// terminal path: success, execution error, timeout, or cancellation).
    pub(crate) fn release(&self) {
        self.reserved.fetch_sub(1, Ordering::AcqRel);
    }

    /// Currently reserved (assigned but not terminal) count.
    /// Test-only introspection for the capacity proofs.
    #[cfg(test)]
    pub(crate) fn reserved(&self) -> usize {
        self.reserved.load(Ordering::Acquire)
    }

    /// Execution permits bounding live tasks.
    pub(crate) fn permits(&self) -> Arc<Semaphore> {
        Arc::clone(&self.execution_permits)
    }
}

fn parse_coordinator_url(url: &str) -> Result<(&str, u16)> {
    let url = url
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_end_matches('/');

    let parts: Vec<&str> = url.split(':').collect();
    if parts.len() != 2 {
        return Err(EggsecError::Config(format!(
            "Invalid coordinator URL format: {} (expected host:port)",
            url
        )));
    }

    let host = parts[0];
    let port: u16 = parts[1]
        .parse()
        .map_err(|_| EggsecError::Config(format!("Invalid port in coordinator URL: {}", url)))?;

    Ok((host, port))
}

fn worker_capabilities() -> Vec<String> {
    CAPABILITIES.iter().map(|s| s.to_string()).collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub worker_id: String,
    pub coordinator_url: String,
    pub max_concurrency: usize,
    pub heartbeat_interval_secs: u64,
    #[serde(default)]
    pub tls_domain: Option<String>,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            worker_id: uuid::Uuid::new_v4().to_string(),
            coordinator_url: "http://localhost:8080".to_string(),
            max_concurrency: 10,
            heartbeat_interval_secs: 30,
            tls_domain: Some("localhost".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStats {
    pub worker_id: String,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub tasks_in_progress: usize,
    pub last_heartbeat_secs: i64,
}

pub struct Worker {
    config: WorkerConfig,
    stats: Arc<Mutex<WorkerStats>>,
    capacity: Arc<CapacityTracker>,
    /// Shared coordinator session (Phase E). Set during registration;
    /// heartbeats, task acquisition, and result submission all multiplex
    /// over it instead of constructing a fresh TLS client per message.
    session: Option<Arc<CoordinatorSession>>,
    sender: Option<mpsc::Sender<Task>>,
    receiver: Option<mpsc::Receiver<Task>>,
    heartbeat_handle: Option<JoinHandle<()>>,
    task_request_handle: Option<JoinHandle<()>>,
    task_processor_handle: Option<JoinHandle<()>>,
    psk: String,
    #[cfg(any(feature = "tool-api", feature = "rest-api", feature = "grpc-api"))]
    enforcement: Arc<EnforcementContext>,
    #[cfg(any(feature = "tool-api", feature = "rest-api", feature = "grpc-api"))]
    dispatcher: EnforcedDispatcher,
    shutdown_tx: watch::Sender<bool>,
}

impl Worker {
    pub fn new(config: WorkerConfig, psk: String) -> Self {
        Self::with_enforcement(
            config,
            psk,
            EnforcementContext::agent_strict(
                crate::config::ExecutionPolicy::default(),
                LoadedScope::default_empty(),
            ),
        )
    }

    /// Construct a worker with the explicit scope and policy it must enforce.
    /// The worker always rebuilds this as an AgentStrict context because tasks
    /// received over the coordinator connection are automated work.
    pub fn with_enforcement(
        config: WorkerConfig,
        psk: String,
        _enforcement: EnforcementContext,
    ) -> Self {
        let (shutdown_tx, _) = watch::channel(false);
        Self {
            config: config.clone(),
            stats: Arc::new(Mutex::new(WorkerStats {
                worker_id: config.worker_id.clone(),
                tasks_completed: 0,
                tasks_failed: 0,
                tasks_in_progress: 0,
                last_heartbeat_secs: chrono::Utc::now().timestamp(),
            })),
            capacity: Arc::new(CapacityTracker::new(config.max_concurrency)),
            session: None,
            sender: None,
            receiver: None,
            heartbeat_handle: None,
            task_request_handle: None,
            task_processor_handle: None,
            psk,
            #[cfg(any(feature = "tool-api", feature = "rest-api", feature = "grpc-api"))]
            enforcement: Arc::new(EnforcementContext::agent_strict(
                _enforcement.execution_policy.clone(),
                _enforcement.loaded_scope.clone(),
            )),
            #[cfg(any(feature = "tool-api", feature = "rest-api", feature = "grpc-api"))]
            dispatcher: EnforcedDispatcher::new(ToolDispatcher::new(create_default_registry())),
            shutdown_tx,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        // Phase D1: `max_concurrency` is a real execution/resource contract.
        // Zero is a structured configuration error, never silently coerced.
        if self.config.max_concurrency == 0 {
            return Err(EggsecError::Config(
                "max_concurrency must be greater than zero".to_string(),
            ));
        }
        self.register_with_coordinator().await?;

        // The channel buffer equals the capacity contract: reservations are
        // capped at `max_concurrency`, so enqueueing reserved tasks never
        // blocks and locally queued work stays within budget.
        let (tx, rx) = mpsc::channel::<Task>(self.config.max_concurrency);
        self.sender = Some(tx);
        self.receiver = Some(rx);

        self.start_heartbeat_loop().await;
        self.start_task_request_loop().await?;
        self.start_task_processing_loop().await;

        Ok(())
    }

    async fn register_with_coordinator(&mut self) -> Result<()> {
        let hostname = hostname::get()?.to_string_lossy().to_string();

        let (host, port) = parse_coordinator_url(&self.config.coordinator_url)?;

        let domain = self.config.tls_domain.clone().ok_or_else(|| {
            EggsecError::Config("TLS domain is required for worker connections".to_string())
        })?;
        // Phase E: establish the shared coordinator session. Registration
        // runs over the session and its metadata is remembered for reconnect
        // replay; steady-state control traffic reuses this connection
        // instead of constructing a fresh TLS client per message.
        let session = CoordinatorSession::spawn(SessionConfig {
            host: host.to_string(),
            port,
            psk: self.psk.clone(),
            tls_domain: Some(domain),
            plaintext_allowed: false,
        });
        session
            .register(
                self.config.worker_id.clone(),
                hostname,
                worker_capabilities(),
            )
            .await?;
        self.session = Some(Arc::new(session));

        Ok(())
    }

    async fn start_heartbeat_loop(&mut self) {
        let worker_id = self.config.worker_id.clone();
        let interval = self.config.heartbeat_interval_secs;
        let stats = Arc::clone(&self.stats);
        let session = self.session.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        let Some(session) = session else {
            tracing::error!("Coordinator session is required for worker heartbeat");
            return;
        };

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let (current_jobs, completed_jobs, failed_jobs) = {
                            let s = stats.lock().await;
                            (s.tasks_in_progress, s.tasks_completed, s.tasks_failed)
                        };

                        let status = serde_json::json!({
                            "worker_id": worker_id,
                            "status": if current_jobs > 0 { "busy" } else { "idle" },
                            "current_jobs": current_jobs,
                            "completed_jobs": completed_jobs,
                            "failed_jobs": failed_jobs,
                        });

                        if let Err(e) = session.heartbeat(status.to_string()).await {
                            tracing::warn!("Heartbeat failed: {}", e);
                        }
                    }
                    _ = shutdown_rx.changed() => {
                        tracing::info!("Heartbeat loop shutting down");
                        break;
                    }
                }
            }
        });
        self.heartbeat_handle = Some(handle);
    }

    async fn start_task_request_loop(&mut self) -> Result<()> {
        let worker_id = self.config.worker_id.clone();
        let capacity = Arc::clone(&self.capacity);
        let session = self.session.clone();
        let sender = self
            .sender
            .clone()
            .ok_or_else(|| EggsecError::Config("sender must be set before start".into()))?;
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        let Some(session) = session else {
            return Err(EggsecError::Config(
                "coordinator session must be set before start".into(),
            ));
        };

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Phase D3: capacity-aware acquisition. No local
                        // capacity means no network operation this tick; the
                        // request size is capped by available capacity. The
                        // wire format is unchanged (`max_tasks` is just
                        // smaller). Phase E: the request multiplexes over the
                        // shared session instead of a fresh TLS client.
                        let want = capacity.request_size();
                        if want == 0 {
                            continue;
                        }
                        match session.request_tasks(worker_id.clone(), want).await {
                            Ok(tasks) => {
                                // Reserve before exposing to the processing
                                // loop (single CAS: concurrent ticks cannot
                                // over-reserve the same slots).
                                let admitted = capacity.try_reserve(tasks.len());
                                if admitted < tasks.len() {
                                    tracing::warn!(
                                        requested = want,
                                        returned = tasks.len(),
                                        admitted,
                                        "Coordinator over-delivered tasks beyond available capacity; \
                                         excess assignments are left for stale-task recovery rather \
                                         than executed beyond the configured limit"
                                    );
                                }
                                let mut pending_reservations = admitted;
                                for task in tasks.into_iter().take(admitted) {
                                    if sender.send(task).await.is_err() {
                                        tracing::warn!(
                                            "Failed to enqueue assigned task: processor is gone"
                                        );
                                        break;
                                    }
                                    // This reservation is now owned by the
                                    // queued task; the terminal path releases
                                    // it.
                                    pending_reservations -= 1;
                                }
                                // Release reservations for tasks never exposed
                                // to the processor so counters stay truthful.
                                for _ in 0..pending_reservations {
                                    capacity.release();
                                }
                            }
                            Err(e) => {
                                tracing::debug!("Task request failed: {}", e);
                            }
                        }
                    }
                    _ = shutdown_rx.changed() => {
                        tracing::info!("Task request loop shutting down");
                        break;
                    }
                }
            }
        });
        self.task_request_handle = Some(handle);
        Ok(())
    }

    async fn start_task_processing_loop(&mut self) {
        if let Some(receiver) = self.receiver.take() {
            let stats = Arc::clone(&self.stats);
            let capacity = Arc::clone(&self.capacity);
            #[cfg(any(feature = "tool-api", feature = "rest-api", feature = "grpc-api"))]
            let enforcement = Arc::clone(&self.enforcement);
            #[cfg(any(feature = "tool-api", feature = "rest-api", feature = "grpc-api"))]
            let dispatcher = self.dispatcher.clone();
            let session = self.session.clone();
            let max_concurrency = self.config.max_concurrency;
            let mut shutdown_rx = self.shutdown_tx.subscribe();

            let handle = tokio::spawn(async move {
                let mut receiver = receiver;
                // Phase D4: the running set is owned here (JoinSet), never
                // detached. Each task holds one execution permit and one
                // capacity reservation; the parent accounts every completion
                // exactly once, so `tasks_in_progress <= max_concurrency`
                // holds through success, error, timeout, and shutdown paths.
                let mut in_flight = tokio::task::JoinSet::new();

                async fn account(
                    stats: &Arc<Mutex<WorkerStats>>,
                    capacity: &Arc<CapacityTracker>,
                    success: bool,
                ) {
                    {
                        let mut s = stats.lock().await;
                        // D5: progress derives from the capacity truth —
                        // increments happen only for reserved admissions, so
                        // this can never exceed `max_concurrency`.
                        s.tasks_in_progress = s.tasks_in_progress.saturating_sub(1);
                        if success {
                            s.tasks_completed = s.tasks_completed.saturating_add(1);
                        } else {
                            s.tasks_failed = s.tasks_failed.saturating_add(1);
                        }
                    }
                    capacity.release();
                }

                loop {
                    tokio::select! {
                        // Admit only while live work is within budget. The
                        // permit is provably available (`running < reserved
                        // <= max_concurrency`), so acquisition never blocks;
                        // a closed semaphore (impossible while `capacity` is
                        // held) fails closed with accounting restored.
                        task = receiver.recv(), if in_flight.len() < max_concurrency => {
                            let Some(task) = task else {
                                break;
                            };
                            let permit = match capacity.permits().acquire_owned().await {
                                Ok(permit) => permit,
                                Err(_) => {
                                    tracing::error!("Capacity semaphore closed; dropping task");
                                    account(&stats, &capacity, false).await;
                                    continue;
                                }
                            };
                            {
                                let mut s = stats.lock().await;
                                s.tasks_in_progress += 1;
                            }

                            let session = session.clone();
                            let stats = Arc::clone(&stats);
                            let capacity = Arc::clone(&capacity);                            #[cfg(any(feature = "tool-api", feature = "rest-api", feature = "grpc-api"))]
                            let enforcement = Arc::clone(&enforcement);
                            #[cfg(any(feature = "tool-api", feature = "rest-api", feature = "grpc-api"))]
                            let dispatcher = dispatcher.clone();

                            in_flight.spawn(async move {
                                let _permit = permit;
                                let task_id = task.id.clone();
                                let outcome = tokio::time::timeout(
                                    TASK_PROCESSING_TIMEOUT,
                                    async {
                                        #[cfg(any(
                                            feature = "tool-api",
                                            feature = "rest-api",
                                            feature = "grpc-api"
                                        ))]
                                        let result =
                                            process_task(task, enforcement, dispatcher).await;
                                        #[cfg(not(any(
                                            feature = "tool-api",
                                            feature = "rest-api",
                                            feature = "grpc-api"
                                        )))]
                                        let result = process_task(task).await;

                                        let task_result = match result {
                                            Ok(r) => r,
                                            Err(e) => {
                                                tracing::error!("Task processing error: {}", e);
                                                TaskResult {
                                                    task_id: task_id.clone(),
                                                    success: false,
                                                    output: String::new(),
                                                    error: Some(e.to_string()),
                                                    duration_millis: 0,
                                                }
                                            }
                                        };

                                        let success = task_result.success;
                                        // Phase E: result submission
                                        // multiplexes over the shared
                                        // session — no fresh TLS client per
                                        // task. Submission stays best-effort
                                        // (warn on failure), exactly as
                                        // before; accounting is unchanged.
                                        match session.as_ref() {
                                            Some(session) => {
                                                if let Err(e) = session
                                                    .send_result(task_result)
                                                    .await
                                                {
                                                    tracing::warn!(
                                                        "Failed to send task result to coordinator: {}",
                                                        e
                                                    );
                                                }
                                            }
                                            None => {
                                                tracing::warn!(
                                                    "No coordinator session; task result not submitted"
                                                );
                                            }
                                        }
                                        success
                                    },
                                )
                                .await;
                                match outcome {
                                    Ok(success) => account(&stats, &capacity, success).await,
                                    Err(_) => {
                                        tracing::warn!(
                                            task_id = %task_id,
                                            "worker task processing timed out after 300s"
                                        );
                                        account(&stats, &capacity, false).await;
                                    }
                                }
                            });
                        }
                        Some(join_result) = in_flight.join_next(), if !in_flight.is_empty() => {
                            if let Err(e) = join_result {
                                // Panic or abort: the permit drops with the
                                // task; restore the stats/reservation truth.
                                tracing::warn!("Worker task join failure: {}", e);
                                account(&stats, &capacity, false).await;
                            }
                        }
                        _ = shutdown_rx.changed() => {
                            tracing::info!("Task processor shutting down; draining owned tasks");
                            break;
                        }
                    }
                }

                // No detached tasks: cancel owned children and account every
                // outcome exactly once before exiting.
                in_flight.abort_all();
                while let Some(join_result) = in_flight.join_next().await {
                    if let Err(e) = join_result {
                        tracing::debug!("Worker task cancelled during drain: {}", e);
                        account(&stats, &capacity, false).await;
                    }
                }
            });
            self.task_processor_handle = Some(handle);
        }
    }

    pub async fn get_stats(&self) -> WorkerStats {
        self.stats.lock().await.clone()
    }

    /// Currently reserved (assigned but not terminal) slots. Crate-internal
    /// capacity introspection for tests; not part of the public surface.
    #[cfg(test)]
    pub(crate) fn reserved_slots(&self) -> usize {
        self.capacity.reserved()
    }

    pub fn shutdown(&mut self) {
        tracing::info!("Worker shutting down");
        if let Err(e) = self.shutdown_tx.send(true) {
            tracing::warn!("Failed to send shutdown signal (no receivers): {:?}", e);
        }
        // Phase E6: stop the coordinator session first so pending control
        // callers fail fast instead of waiting out reconnect backoff.
        if let Some(session) = self.session.take() {
            session.shutdown();
        }
        if let Some(handle) = self.heartbeat_handle.take() {
            handle.abort();
        }
        if let Some(handle) = self.task_request_handle.take() {
            handle.abort();
        }
        if let Some(handle) = self.task_processor_handle.take() {
            handle.abort();
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        // Receivers may already be dropped during normal shutdown; send failure is expected.
        if let Err(error) = self.shutdown_tx.send(true) {
            tracing::debug!(?error, "Worker shutdown receiver already dropped");
        }
        if let Some(handle) = self.heartbeat_handle.take() {
            handle.abort();
        }
        if let Some(handle) = self.task_request_handle.take() {
            handle.abort();
        }
        if let Some(handle) = self.task_processor_handle.take() {
            handle.abort();
        }
    }
}

#[cfg(any(feature = "tool-api", feature = "rest-api", feature = "grpc-api"))]
async fn process_task(
    task: Task,
    enforcement: Arc<EnforcementContext>,
    dispatcher: EnforcedDispatcher,
) -> Result<TaskResult> {
    let start_time = std::time::Instant::now();
    let task_id = task.id.clone();
    let operation = match task.task_type {
        TaskType::PortScan => "scan-ports",
        TaskType::ServiceFingerprint => "fingerprint",
        TaskType::EndpointDiscovery => "scan-endpoints",
        TaskType::Fuzz => "fuzz",
        TaskType::WafTest => "waf-detect",
        TaskType::LoadTest => "load-test",
        TaskType::Recon => "recon",
    };
    let metadata = metadata_for_tool_id(operation).ok_or_else(|| {
        EggsecError::Config(format!(
            "no operation metadata for distributed task {}",
            operation
        ))
    })?;
    let descriptor = metadata
        .try_descriptor_for_target(Some(&task.target))
        .map_err(|error| EggsecError::Config(error.to_string()))?;
    let approved = enforcement
        .approve(ExecutionSurface::SecurityAgent, descriptor)
        .map_err(|error| EggsecError::Config(error.to_string()))?;
    let request = ToolRequest::new(operation, Target::url(task.target))
        .with_params(serde_json::to_value(task.payload)?);
    let result = dispatcher.dispatch_checked(&approved, request).await;

    let duration = start_time.elapsed();
    let success = result.is_ok();
    let output = match result.as_ref() {
        Ok(o) => serde_json::to_string(o)?,
        Err(_) => String::new(),
    };
    let error = result.err().map(|e| e.to_string());

    Ok(TaskResult {
        task_id,
        success,
        output,
        error,
        duration_millis: duration.as_millis() as u64,
    })
}

#[cfg(not(any(feature = "tool-api", feature = "rest-api", feature = "grpc-api")))]
async fn process_task(task: Task) -> Result<TaskResult> {
    Err(EggsecError::Config(format!(
        "distributed task execution requires the tool-api feature (task {})",
        task.id
    )))
}

#[allow(dead_code)]
async fn process_port_scan(task: Task) -> Result<serde_json::Value> {
    let target = &task.target;
    let ports = task
        .payload
        .get("ports")
        .and_then(|v| v.as_str())
        .unwrap_or("1-1000");
    let concurrency: usize = task
        .payload
        .get("concurrency")
        .and_then(|v| v.as_u64())
        .unwrap_or(100) as usize;
    let timeout: u64 = task
        .payload
        .get("timeout")
        .and_then(|v| v.as_u64())
        .unwrap_or(5);

    let parsed_ports = crate::utils::parsing::parse_ports(ports)?;

    let results = crate::scanner::ports::scan_ports(
        target,
        crate::scanner::ports::PortScanConfig {
            ports: parsed_ports,
            concurrency,
            timeout_duration: std::time::Duration::from_secs(timeout),
            tui_mode: false,
            spoof_config: crate::scanner::spoof::SpoofConfig::default(),
            progress_tx: None,
            max_results: None,
        },
    )
    .await?;

    Ok(serde_json::json!({
        "target": target,
        "open_ports": results.open_ports,
        "scan_duration_ms": results.duration_ms,
    }))
}

#[allow(dead_code)]
async fn process_fingerprint(task: Task) -> Result<serde_json::Value> {
    let target = &task.target;
    let ports: Vec<u16> = task
        .payload
        .get("ports")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_u64())
                .map(|p| p as u16)
                .collect()
        })
        .unwrap_or_else(|| {
            vec![
                21, 22, 23, 25, 53, 80, 110, 143, 443, 445, 993, 995, 1433, 1521, 3306, 3389, 5432,
                5900, 6379, 8080, 8443, 27017,
            ]
        });
    let timeout: u64 = task
        .payload
        .get("timeout")
        .and_then(|v| v.as_u64())
        .unwrap_or(5);

    let results = crate::scanner::fingerprint::fingerprint_services(
        target,
        ports,
        std::time::Duration::from_secs(timeout),
        false,
        20,
        None,
        None,
    )
    .await?;

    Ok(serde_json::json!({
        "target": target,
        "services": results.results,
    }))
}

#[allow(dead_code)]
async fn process_endpoints(task: Task) -> Result<serde_json::Value> {
    let target = &task.target;
    let wordlist = if let Some(w) = task.payload.get("wordlist").and_then(|v| v.as_str()) {
        crate::scanner::wordlist::Wordlist::from_file(w)
            .await?
            .into_endpoints()
    } else {
        vec![
            "/admin".to_string(),
            "/api".to_string(),
            "/login".to_string(),
            "/config".to_string(),
            "/.env".to_string(),
            "/status".to_string(),
            "/health".to_string(),
        ]
    };
    let concurrency: usize = task
        .payload
        .get("concurrency")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;

    let results = crate::scanner::endpoints::scan_endpoints(EndpointScanConfig {
        base_url: target.to_string(),
        endpoints: wordlist,
        concurrency,
        timeout_duration: std::time::Duration::from_secs(10),
        include_404: false,
        tui_mode: false,
        spoof_config: std::sync::Arc::new(crate::scanner::spoof::SpoofConfig::default()),
        verify_tls: true,
        progress_tx: None,
        max_results: None,
    })
    .await?;

    Ok(serde_json::json!({
        "target": target,
        "endpoints": results.results,
    }))
}

#[allow(dead_code)]
async fn process_fuzz(task: Task) -> Result<serde_json::Value> {
    let target = &task.target;
    let payload_type = task
        .payload
        .get("payload_type")
        .and_then(|v| v.as_str())
        .unwrap_or("all");
    let concurrency: usize = task
        .payload
        .get("concurrency")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;

    let auth_context_path = task
        .payload
        .get("auth_context_path")
        .and_then(|v| v.as_str())
        .map(String::from);
    let auth_role = task
        .payload
        .get("auth_role")
        .and_then(|v| v.as_str())
        .map(String::from);

    let common = crate::types::CommonHttpArgs {
        auth_context: auth_context_path,
        auth_role,
        ..Default::default()
    };

    let config = crate::fuzzer::config::FuzzConfig {
        url: target.to_string(),
        payload_type: payload_type.to_string(),
        mode: crate::fuzzer::config::FuzzMode::Sequential,
        mutate: false,
        mutation_count: 3,
        concurrency,
        timeout: 10,
        method: "GET".to_string(),
        graphql_introspection: true,
        graphql_depth_bypass: true,
        graphql_alias_overload: true,
        oauth_redirect: true,
        oauth_scope: true,
        oauth_state: true,
        oauth_grant: true,
        common,
        ..Default::default()
    };

    let mut engine = crate::fuzzer::engine::FuzzEngine::new(config)?;
    let session = engine.run_return_session().await?;

    let findings: Vec<_> = session
        .results
        .iter()
        .take(50)
        .map(|f| {
            serde_json::json!({
                "payload": f.payload,
                "severity": f.detected_severity,
                "description": f.payload.description,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "target": target,
        "total_requests": session.successful_requests + session.failed_requests,
        "findings": findings,
    }))
}

#[allow(dead_code)]
async fn process_waf(task: Task) -> Result<serde_json::Value> {
    let target = &task.target;
    let detect_only = task
        .payload
        .get("detect_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let auth_context_path = task
        .payload
        .get("auth_context_path")
        .and_then(|v| v.as_str())
        .map(String::from);
    let auth_role = task
        .payload
        .get("auth_role")
        .and_then(|v| v.as_str())
        .map(String::from);

    let common = crate::types::CommonHttpArgs {
        auth_context: auth_context_path,
        auth_role,
        ..Default::default()
    };

    let config = crate::fuzzer::config::WafConfig {
        url: target.to_string(),
        detect_only,
        bypass: !detect_only,
        header_bypass: true,
        smuggling: true,
        evasion: true,
        profile: "auto".to_string(),
        test_type: None,
        concurrency: 10,
        timeout: 15,
        common,
        ..Default::default()
    };

    let mut engine = crate::waf::WafEngine::new(config)?;
    engine.run().await?;

    Ok(serde_json::json!({
        "target": target,
        "status": "completed",
    }))
}

#[allow(dead_code)]
async fn process_load_test(_task: Task) -> Result<serde_json::Value> {
    // Fail closed: distributed tasks carry no execution-scope snapshot in the
    // current `Task` shape. Load testing without the approval scope must not
    // synthesize wildcard authorization. Route distributed load testing
    // through `EnforcedDispatcher::dispatch_execution()` with an
    // `ApprovedExecution` bundle once `Task` carries scope provenance.
    Err(EggsecError::Validation(
        "distributed load-test execution scope is missing: Task carries no scope snapshot; \
         use EnforcedDispatcher::dispatch_execution() with an ApprovedExecution bundle \
         (no wildcard default)"
            .to_string(),
    ))
}

#[allow(dead_code)]
async fn process_recon(task: Task) -> Result<serde_json::Value> {
    let target = &task.target;
    let no_tech = task
        .payload
        .get("no_tech")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let no_dns = task
        .payload
        .get("no_dns")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut request = crate::recon::ReconRequest::new(target.clone()).with_concurrency(10);
    request.no_tech = no_tech;
    request.no_dns = no_dns;

    let config = crate::config::EggsecConfig::default();
    let stage = Arc::new(parking_lot::Mutex::new(String::new()));
    crate::recon::runner::run_full_recon_from_request(&request, &config, stage, false).await?;

    Ok(serde_json::json!({
        "target": target,
        "status": "completed",
    }))
}

#[cfg(all(
    test,
    any(feature = "tool-api", feature = "rest-api", feature = "grpc-api")
))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn worker_rejects_tasks_without_an_explicit_scope() {
        let task = Task {
            id: "task-1".to_string(),
            job_id: "job-1".to_string(),
            task_type: TaskType::PortScan,
            target: "203.0.113.10".to_string(),
            payload: rustc_hash::FxHashMap::default(),
            worker_id: None,
            assigned_at_secs: None,
        };
        let enforcement = Arc::new(EnforcementContext::agent_strict(
            crate::config::ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        ));
        let dispatcher = EnforcedDispatcher::new(ToolDispatcher::new(create_default_registry()));

        let error = process_task(task, enforcement, dispatcher)
            .await
            .expect_err("an empty strict scope must reject the task");
        assert!(error.to_string().contains("denied"));
    }
}

/// Phase D capacity-accounting tests (deterministic, no network, no timing
/// races). The hard `reserved <= max_concurrency` guarantee lives in
/// [`CapacityTracker::try_reserve`]; the worker loops share that single
/// instance, so these unit proofs compose with the end-to-end bounds in
/// `distributed_tests.rs`.
#[cfg(test)]
mod capacity_tests {
    use super::*;

    #[test]
    fn zero_capacity_admits_nothing_and_requests_nothing() {
        let tracker = CapacityTracker::new(0);
        assert_eq!(tracker.available(), 0);
        assert_eq!(tracker.request_size(), 0);
        assert_eq!(tracker.try_reserve(5), 0);
        assert_eq!(tracker.reserved(), 0);
    }

    #[test]
    fn request_size_caps_at_five_and_available() {
        let tracker = CapacityTracker::new(100);
        assert_eq!(tracker.request_size(), MAX_TASKS_PER_REQUEST);
        let partial = CapacityTracker::new(3);
        assert_eq!(partial.request_size(), 3);
    }

    #[test]
    fn exact_fit_and_release_restores_capacity() {
        let tracker = CapacityTracker::new(4);
        assert_eq!(tracker.try_reserve(4), 4);
        assert_eq!(tracker.available(), 0);
        assert_eq!(tracker.request_size(), 0);
        // Terminal paths release exactly once each.
        for _ in 0..4 {
            tracker.release();
        }
        assert_eq!(tracker.reserved(), 0);
        assert_eq!(tracker.available(), 4);
        assert_eq!(tracker.request_size(), 4);
    }

    #[test]
    fn over_delivery_admits_only_room() {
        let tracker = CapacityTracker::new(3);
        // Coordinator returns more than requested: admit only the room.
        assert_eq!(tracker.try_reserve(5), 3);
        assert_eq!(tracker.reserved(), 3);
        assert_eq!(tracker.try_reserve(2), 0);
        for _ in 0..3 {
            tracker.release();
        }
        assert_eq!(tracker.available(), 3);
    }

    #[test]
    fn partial_reservation_then_completion_frees_next_request() {
        let tracker = CapacityTracker::new(2);
        assert_eq!(tracker.try_reserve(2), 2);
        // Saturated: the acquisition tick must skip the network operation.
        assert_eq!(tracker.request_size(), 0);
        // One completion frees one slot and permits the next request.
        tracker.release();
        assert_eq!(tracker.available(), 1);
        assert_eq!(tracker.request_size(), 1);
        assert_eq!(tracker.try_reserve(1), 1);
        tracker.release();
        tracker.release();
        assert_eq!(tracker.reserved(), 0);
    }

    #[test]
    fn task_processing_timeout_bound_is_pinned() {
        // The no-detached-task bound (success/error/timeout/cancel all
        // terminal within this duration).
        assert_eq!(TASK_PROCESSING_TIMEOUT, std::time::Duration::from_secs(300));
    }

    #[tokio::test]
    async fn concurrent_reservation_never_exceeds_capacity() {
        let tracker = Arc::new(CapacityTracker::new(10));
        let mut handles = Vec::new();
        for _ in 0..10 {
            let tracker = Arc::clone(&tracker);
            handles.push(tokio::spawn(async move { tracker.try_reserve(5) }));
        }
        let mut total = 0usize;
        for handle in handles {
            total += handle.await.expect("reservation task");
        }
        // Ten concurrent ticks of five on capacity ten admit exactly ten:
        // no two ticks observed the same free slots.
        assert_eq!(total, 10);
        assert_eq!(tracker.reserved(), 10);
    }

    #[tokio::test]
    async fn worker_start_rejects_zero_concurrency() {
        let config = WorkerConfig {
            max_concurrency: 0,
            ..WorkerConfig::default()
        };
        let mut worker = Worker::new(config, "test-psk".to_string());
        let err = worker
            .start()
            .await
            .expect_err("max_concurrency 0 must fail explicitly");
        assert!(err.to_string().contains("max_concurrency"));
    }

    #[test]
    fn worker_config_serialization_unchanged() {
        // Wire/config compatibility: defaults and shape are untouched.
        let config = WorkerConfig::default();
        assert_eq!(config.max_concurrency, 10);
        let json = serde_json::to_string(&config).expect("serialize");
        let decoded: WorkerConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.max_concurrency, 10);
        assert_eq!(decoded.worker_id, config.worker_id);
    }

    fn capacity_test_task(id: usize) -> Task {
        Task {
            id: format!("task-{id}"),
            job_id: "job-1".to_string(),
            task_type: crate::distributed::TaskType::PortScan,
            target: "203.0.113.10".to_string(),
            payload: rustc_hash::FxHashMap::default(),
            worker_id: None,
            assigned_at_secs: None,
        }
    }

    /// Phase D processor integration (no coordinator/TLS needed): drive the
    /// real processing loop with reserved admissions and prove live work
    /// never exceeds capacity, every terminal path releases exactly once,
    /// and shutdown leaves no detached task behind.
    ///
    /// Without the `tool-api` feature every task takes the execution-error
    /// path (fast and deterministic); the capacity/accounting behavior under
    /// test is identical on the success path.
    #[tokio::test]
    async fn processor_bounds_live_work_and_releases_everything() {
        let max_concurrency = 3usize;
        let config = WorkerConfig {
            max_concurrency,
            coordinator_url: "http://127.0.0.1:9".to_string(),
            ..WorkerConfig::default()
        };
        let mut worker = Worker::new(config, "test-psk".to_string());
        // Phase E: result submission goes through the shared session. A
        // plaintext session against a refused loopback port fails fast,
        // exercising the submit-and-account path without a coordinator.
        let session = CoordinatorSession::spawn(SessionConfig {
            host: "127.0.0.1".to_string(),
            port: 9,
            psk: "test-psk".to_string(),
            tls_domain: None,
            plaintext_allowed: true,
        });
        worker.session = Some(Arc::new(session));
        let (tx, rx) = mpsc::channel::<Task>(max_concurrency);
        worker.sender = Some(tx.clone());
        worker.receiver = Some(rx);
        worker.start_task_processing_loop().await;

        // Peak observer: one-sided upper-bound proof (any observation above
        // capacity fails; missing the true peak still passes).
        let stats_probe = Arc::clone(&worker.stats);
        let peak = Arc::new(AtomicUsize::new(0));
        let peak_task = Arc::clone(&peak);
        let observer = tokio::spawn(async move {
            for _ in 0..500 {
                let current = stats_probe.lock().await.tasks_in_progress;
                peak_task.fetch_max(current, Ordering::SeqCst);
                tokio::task::yield_now().await;
            }
        });

        // Four waves of `max_concurrency` admissions (12 tasks total),
        // mirroring capacity-aware acquisition: reserve, expose, drain.
        let waves = 4;
        for _ in 0..waves {
            let admitted = worker.capacity.try_reserve(max_concurrency);
            assert_eq!(admitted, max_concurrency);
            for i in 0..max_concurrency {
                tx.send(capacity_test_task(i)).await.expect("enqueue");
            }
            // Drain with a bounded wait (loopback-refused submits are fast).
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                if worker.reserved_slots() == 0 {
                    break;
                }
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "reservations did not drain"
                );
                tokio::task::yield_now().await;
            }
        }

        observer.abort();
        let observed_peak = peak.load(Ordering::SeqCst);
        assert!(
            observed_peak <= max_concurrency,
            "live work {observed_peak} exceeded capacity {max_concurrency}"
        );

        let stats = worker.get_stats().await;
        assert_eq!(stats.tasks_in_progress, 0);
        assert_eq!(stats.tasks_failed, (waves * max_concurrency) as u64);
        assert_eq!(stats.tasks_completed, 0);
        assert_eq!(worker.reserved_slots(), 0);

        // Shutdown leaves no detached processing task: after shutdown the
        // owned set is drained and counters stay terminal.
        worker.shutdown();
        tokio::task::yield_now().await;
        let stats = worker.get_stats().await;
        assert_eq!(stats.tasks_in_progress, 0);
        assert_eq!(worker.reserved_slots(), 0);
    }
}
