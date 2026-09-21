use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, oneshot, watch, RwLock};

use crate::distributed::command::{CommandExecutor, CommandMessage, ResponseMessage};
use crate::distributed::io::{LineWriter, StreamWrapper, TlsClient, TlsServer};
use crate::distributed::{queue::TaskQueue, CAPABILITIES};
use crate::error::{EggsecError, Result};
use crate::utils::connect_with_nodelay_timeout;

const MAX_CONNECTIONS: usize = 100;
const RATE_LIMIT_PER_MINUTE: u32 = 60;
const RATE_LIMIT_WINDOW_SECS: u64 = 60;

#[derive(Clone)]
pub struct TlsConfig {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

pub struct RemoteListener {
    psk: String,
    shutdown_tx: broadcast::Sender<()>,
    connections: Arc<RwLock<FxHashSet<String>>>,
    rate_limits: Arc<RwLock<FxHashMap<String, Vec<Instant>>>>,
    max_connections: usize,
    rate_limit: u32,
    ip_allowlist: Option<Vec<String>>,
    tls_server: Option<Arc<TlsServer>>,
    plaintext_allowed: bool,
    task_queue: Arc<TaskQueue>,
    workers: Arc<RwLock<FxHashMap<String, crate::distributed::WorkerRegistration>>>,
    /// Total accepted TCP connections (monotonic; test/evidence introspection).
    accepted: Arc<AtomicUsize>,
    /// Total successful TLS handshakes (monotonic; test introspection).
    /// Incremented only after `accept_tls` succeeds, so plaintext
    /// listeners report zero while TLS listeners report 1:1 with
    /// handshake completions.
    tls_handshakes: Arc<AtomicUsize>,
    /// Total successful PSK authentications (monotonic; test introspection).
    authenticated: Arc<AtomicUsize>,
}

/// Per-connection dependencies (keeps `handle_connection` under the
/// clippy argument-count lint).
#[derive(Clone)]
struct ConnectionDeps {
    psk: String,
    connections: Arc<RwLock<FxHashSet<String>>>,
    tls_acceptor: Option<tokio_rustls::TlsAcceptor>,
    task_queue: Arc<TaskQueue>,
    workers: Arc<RwLock<FxHashMap<String, crate::distributed::WorkerRegistration>>>,
    tls_handshakes: Arc<AtomicUsize>,
    authenticated: Arc<AtomicUsize>,
}

impl RemoteListener {
    pub fn new(psk: String) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            psk,
            shutdown_tx,
            connections: Arc::new(RwLock::new(FxHashSet::default())),
            rate_limits: Arc::new(RwLock::new(FxHashMap::default())),
            max_connections: MAX_CONNECTIONS,
            rate_limit: RATE_LIMIT_PER_MINUTE,
            ip_allowlist: None,
            tls_server: None,
            plaintext_allowed: false,
            task_queue: Arc::new(TaskQueue::new(
                crate::constants::DEFAULT_TASK_QUEUE_CAPACITY,
            )),
            workers: Arc::new(RwLock::new(FxHashMap::default())),
            accepted: Arc::new(AtomicUsize::new(0)),
            tls_handshakes: Arc::new(AtomicUsize::new(0)),
            authenticated: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn with_config(psk: String, max_connections: usize, rate_limit: u32) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            psk,
            shutdown_tx,
            connections: Arc::new(RwLock::new(FxHashSet::default())),
            rate_limits: Arc::new(RwLock::new(FxHashMap::default())),
            max_connections,
            rate_limit,
            ip_allowlist: None,
            tls_server: None,
            plaintext_allowed: false,
            task_queue: Arc::new(TaskQueue::new(
                crate::constants::DEFAULT_TASK_QUEUE_CAPACITY,
            )),
            workers: Arc::new(RwLock::new(FxHashMap::default())),
            accepted: Arc::new(AtomicUsize::new(0)),
            tls_handshakes: Arc::new(AtomicUsize::new(0)),
            authenticated: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn with_allowlist(psk: String, allowlist: Vec<String>) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            psk,
            shutdown_tx,
            connections: Arc::new(RwLock::new(FxHashSet::default())),
            rate_limits: Arc::new(RwLock::new(FxHashMap::default())),
            max_connections: MAX_CONNECTIONS,
            rate_limit: RATE_LIMIT_PER_MINUTE,
            ip_allowlist: Some(allowlist),
            tls_server: None,
            plaintext_allowed: false,
            task_queue: Arc::new(TaskQueue::new(
                crate::constants::DEFAULT_TASK_QUEUE_CAPACITY,
            )),
            workers: Arc::new(RwLock::new(FxHashMap::default())),
            accepted: Arc::new(AtomicUsize::new(0)),
            tls_handshakes: Arc::new(AtomicUsize::new(0)),
            authenticated: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn with_tls(psk: String, tls_config: TlsConfig) -> Result<Self> {
        let tls_server = TlsServer::from_pem(&tls_config.cert_path, &tls_config.key_path)
            .map_err(|e| EggsecError::Network(format!("Failed to initialize TLS: {}", e)))?;

        let (shutdown_tx, _) = broadcast::channel(1);
        Ok(Self {
            psk,
            shutdown_tx,
            connections: Arc::new(RwLock::new(FxHashSet::default())),
            rate_limits: Arc::new(RwLock::new(FxHashMap::default())),
            max_connections: MAX_CONNECTIONS,
            rate_limit: RATE_LIMIT_PER_MINUTE,
            ip_allowlist: None,
            tls_server: Some(Arc::new(tls_server)),
            plaintext_allowed: false,
            task_queue: Arc::new(TaskQueue::new(
                crate::constants::DEFAULT_TASK_QUEUE_CAPACITY,
            )),
            workers: Arc::new(RwLock::new(FxHashMap::default())),
            accepted: Arc::new(AtomicUsize::new(0)),
            tls_handshakes: Arc::new(AtomicUsize::new(0)),
            authenticated: Arc::new(AtomicUsize::new(0)),
        })
    }

    pub fn new_plaintext(psk: String) -> Self {
        tracing::warn!("Creating plaintext distributed listener; use TLS for any non-isolated lab");
        let mut listener = Self::new(psk);
        listener.plaintext_allowed = true;
        listener
    }

    pub fn is_tls(&self) -> bool {
        self.tls_server.is_some()
    }

    fn get_capabilities() -> Vec<String> {
        CAPABILITIES.iter().map(|s| s.to_string()).collect()
    }

    pub async fn get_workers(&self) -> Vec<crate::distributed::WorkerRegistration> {
        self.workers.read().await.values().cloned().collect()
    }

    /// Total accepted TCP connections (monotonic). Crate-internal
    /// introspection for session-reuse evidence; not part of the wire surface.
    #[cfg(test)]
    pub(crate) fn accepted_count(&self) -> usize {
        self.accepted.load(Ordering::Relaxed)
    }

    /// Total successful TLS handshakes (monotonic). Incremented only after
    /// `accept_tls` succeeds; plaintext listeners stay at zero.
    #[cfg(test)]
    pub(crate) fn tls_handshake_count(&self) -> usize {
        self.tls_handshakes.load(Ordering::Relaxed)
    }

    /// Total successful PSK authentications (monotonic). Each accepted
    /// connection authenticates exactly once, so steady-state session reuse
    /// shows accepts/auths approaching one per connection lifetime rather
    /// than one per control message.
    #[cfg(test)]
    pub(crate) fn authenticated_count(&self) -> usize {
        self.authenticated.load(Ordering::Relaxed)
    }

    /// Completed task results recorded by the coordinator. Crate-internal
    /// introspection for session association tests.
    #[cfg(test)]
    pub(crate) async fn completed_results(&self) -> Vec<crate::distributed::queue::TaskResult> {
        self.task_queue.get_results().await
    }

    pub async fn get_queue_counts(&self) -> (usize, usize, usize) {
        (
            self.task_queue.get_pending_count().await,
            self.task_queue.get_in_progress_count().await,
            self.task_queue.get_completed_count().await,
        )
    }

    pub fn shutdown(&self) {
        if let Err(e) = self.shutdown_tx.send(()) {
            tracing::warn!("Failed to send shutdown signal: {:?}", e);
        }
    }

    async fn check_rate_limit(
        rate_limits: &Arc<RwLock<FxHashMap<String, Vec<Instant>>>>,
        ip: &str,
        limit: u32,
    ) -> bool {
        let mut limits = rate_limits.write().await;
        let now = Instant::now();
        let window = Duration::from_secs(RATE_LIMIT_WINDOW_SECS);

        // Clean old entries and get current count
        let timestamps = limits.entry(ip.to_string()).or_insert_with(Vec::new);
        timestamps.retain(|t| now.duration_since(*t) < window);

        if timestamps.len() >= limit as usize {
            return false;
        }

        timestamps.push(now);
        true
    }

    fn ip_matches_allowlist(ip: IpAddr, allowlist: &[String]) -> bool {
        for entry in allowlist {
            if let Ok(cidr) = entry.parse::<ipnetwork::IpNetwork>() {
                if cidr.contains(ip) {
                    return true;
                }
            } else if let Ok(addr) = entry.parse::<IpAddr>() {
                if addr == ip {
                    return true;
                }
            }
        }
        false
    }

    pub async fn start(&self, port: u16) -> Result<()> {
        if !self.is_tls() && !self.plaintext_allowed {
            return Err(EggsecError::Config(
                "TLS is required for distributed listeners; configure a certificate and key"
                    .to_string(),
            ));
        }
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = TcpListener::bind(addr).await?;

        let protocol = if self.is_tls() { "TLS" } else { "plaintext" };
        tracing::info!("Remote listener started on port {} ({})", port, protocol);
        tracing::info!(
            "Max connections: {}, Rate limit: {}/min",
            self.max_connections,
            self.rate_limit
        );

        if self.ip_allowlist.is_some() {
            tracing::info!("IP allowlist: enabled");
        }

        tracing::info!("Waiting for connections...");

        let tls_acceptor = self.tls_server.as_ref().map(|s| s.clone_acceptor());
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let rate_limits = Arc::clone(&self.rate_limits);

        // Periodic cleanup of stale rate limit entries
        let cleanup_handle = tokio::spawn(async move {
            let mut cleanup_interval =
                tokio::time::interval(std::time::Duration::from_secs(RATE_LIMIT_WINDOW_SECS));
            loop {
                cleanup_interval.tick().await;
                let mut limits = rate_limits.write().await;
                let now = Instant::now();
                let window = Duration::from_secs(RATE_LIMIT_WINDOW_SECS);
                limits.retain(|_ip, timestamps| {
                    timestamps.retain(|t| now.duration_since(*t) < window);
                    !timestamps.is_empty()
                });
            }
        });

        // Periodic stale task reassignment
        let task_queue = Arc::clone(&self.task_queue);
        let stale_handle = tokio::spawn(async move {
            let mut stale_interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                stale_interval.tick().await;
                let stale_tasks = task_queue
                    .reassign_stale_tasks(crate::constants::WORKER_STALE_TIMEOUT_SECS)
                    .await;
                if !stale_tasks.is_empty() {
                    tracing::warn!(
                        count = stale_tasks.len(),
                        "Reassigned stale tasks from disconnected workers"
                    );
                }
            }
        });

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            // Check IP allowlist
                            if let Some(ref allowlist) = self.ip_allowlist {
                                if !Self::ip_matches_allowlist(addr.ip(), allowlist) {
                                    tracing::warn!("Connection rejected: IP {} not in allowlist", addr.ip());
                                    continue;
                                }
                            }

                            // Check connection limit
                            let conn_count = self.connections.read().await.len();
                            if conn_count >= self.max_connections {
                                tracing::warn!("Connection rejected: max connections ({}) reached", self.max_connections);
                                continue;
                            }

                            // Check rate limit
                            let ip_str = addr.ip().to_string();
                            if !Self::check_rate_limit(&self.rate_limits, &ip_str, self.rate_limit).await {
                                tracing::warn!("Connection rejected: rate limit exceeded for {}", ip_str);
                                continue;
                            }

                            let deps = ConnectionDeps {
                                psk: self.psk.clone(),
                                connections: Arc::clone(&self.connections),
                                tls_acceptor: tls_acceptor.clone(),
                                task_queue: Arc::clone(&self.task_queue),
                                workers: Arc::clone(&self.workers),
                                tls_handshakes: Arc::clone(&self.tls_handshakes),
                                authenticated: Arc::clone(&self.authenticated),
                            };
                            self.accepted.fetch_add(1, Ordering::Relaxed);
                            let handle = tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection(stream, addr, deps).await {
                                    tracing::error!("Connection error: {}", e);
                                }
                            });
                            let addr_clone = addr.to_string();
                            tokio::spawn(async move {
                                if let Err(e) = handle.await {
                                    tracing::error!("Connection task panicked for {}: {}", addr_clone, e);
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("Failed to accept connection: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("Shutting down listener...");
                    cleanup_handle.abort();
                    stale_handle.abort();
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_connection(
        stream: TcpStream,
        addr: SocketAddr,
        deps: ConnectionDeps,
    ) -> Result<()> {
        let ConnectionDeps {
            psk,
            connections,
            tls_acceptor,
            task_queue,
            workers,
            tls_handshakes,
            authenticated,
        } = deps;
        tracing::info!("Connection from {}", addr);
        tracing::info!("Connection from {}", addr);

        let stream = match tls_acceptor {
            Some(acceptor) => match StreamWrapper::accept_tls(&acceptor, stream).await {
                Ok(s) => {
                    tls_handshakes.fetch_add(1, Ordering::Relaxed);
                    s
                }
                Err(e) => {
                    tracing::error!(addr = %addr, "TLS handshake failed: {}", e);
                    return Err(EggsecError::Network(format!(
                        "TLS handshake failed from {}: {}",
                        addr, e
                    )));
                }
            },
            None => StreamWrapper::plain(stream),
        };

        let mut line_writer = LineWriter::new(stream);

        // Read auth message
        let auth_line = line_writer.read_line().await?;
        let auth: AuthMessage = serde_json::from_str(
            &auth_line.ok_or_else(|| EggsecError::Validation("No auth".to_string()))?,
        )?;

        if !bool::from(auth.psk.as_bytes().ct_eq(psk.as_bytes())) {
            let error = ResponseMessage::error("auth".to_string(), "Invalid PSK".to_string(), None);
            line_writer
                .write_line(&serde_json::to_string(&error)?)
                .await?;
            return Err(EggsecError::Validation(format!(
                "Invalid PSK from {}",
                addr
            )));
        }

        // Register connection
        connections.write().await.insert(addr.to_string());
        authenticated.fetch_add(1, Ordering::Relaxed);
        tracing::info!(addr = %addr, "Authenticated successfully");

        // Send welcome
        let welcome = ResponseMessage {
            id: "auth".to_string(),
            msg_type: "authenticated".to_string(),
            success: true,
            output: Some("Authenticated".to_string()),
            error: None,
            duration_ms: None,
            hostname: Some(
                hostname::get()
                    .map_err(|e| EggsecError::Runtime(format!("Failed to get hostname: {}", e)))?
                    .to_string_lossy()
                    .to_string(),
            ),
            capabilities: Some(Self::get_capabilities()),
        };
        line_writer
            .write_line(&serde_json::to_string(&welcome)?)
            .await?;

        // Track which worker_id is associated with this connection for cleanup
        let mut connected_worker_id: Option<String> = None;

        // Handle commands loop
        let addr_str = addr.to_string();
        loop {
            let line = match line_writer.read_line().await {
                Ok(Some(l)) => l,
                Ok(None) => break, // EOF
                Err(e) => {
                    tracing::debug!("Read error: {}", e);
                    break;
                }
            };

            let request: CommandMessage = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    let error = ResponseMessage::error(
                        "unknown".to_string(),
                        format!("Invalid request: {}", e),
                        None,
                    );
                    line_writer
                        .write_line(&serde_json::to_string(&error)?)
                        .await?;
                    continue;
                }
            };

            match request {
                CommandMessage::Execute {
                    id,
                    command,
                    timeout,
                    env,
                } => {
                    tracing::info!(command = ?command, "Executing remote command");
                    match CommandExecutor::execute(command, timeout, env).await {
                        Ok((output, duration_ms)) => {
                            let response = ResponseMessage::success(id, output, duration_ms);
                            line_writer
                                .write_line(&serde_json::to_string(&response)?)
                                .await?;
                        }
                        Err(e) => {
                            let response = ResponseMessage::error(id, e, None);
                            line_writer
                                .write_line(&serde_json::to_string(&response)?)
                                .await?;
                        }
                    }
                }
                CommandMessage::Register {
                    id,
                    hostname,
                    capabilities,
                } => {
                    let claimed_count = capabilities.len();
                    let valid_caps: Vec<String> = capabilities
                        .iter()
                        .filter(|cap| CAPABILITIES.contains(&cap.as_str()))
                        .cloned()
                        .collect();
                    if valid_caps.len() != claimed_count {
                        tracing::warn!(
                            worker = %hostname,
                            "Worker advertised capabilities not in CAPABILITIES list; filtering"
                        );
                    }

                    let cap_types: Vec<crate::distributed::TaskType> = valid_caps
                        .iter()
                        .filter_map(|cap| match cap.as_str() {
                            "PortScan" => Some(crate::distributed::TaskType::PortScan),
                            "ServiceFingerprint" => {
                                Some(crate::distributed::TaskType::ServiceFingerprint)
                            }
                            "EndpointDiscovery" => {
                                Some(crate::distributed::TaskType::EndpointDiscovery)
                            }
                            "Fuzz" => Some(crate::distributed::TaskType::Fuzz),
                            "WafTest" => Some(crate::distributed::TaskType::WafTest),
                            "LoadTest" => Some(crate::distributed::TaskType::LoadTest),
                            "Recon" => Some(crate::distributed::TaskType::Recon),
                            _ => None,
                        })
                        .collect();

                    let registration = crate::distributed::WorkerRegistration {
                        worker_id: id.clone(),
                        hostname: hostname.clone(),
                        capabilities: cap_types,
                        max_concurrency: 10,
                        status: crate::distributed::WorkerStatus::Idle,
                        last_heartbeat_secs: Some(chrono::Utc::now().timestamp()),
                    };
                    workers.write().await.insert(id.clone(), registration);
                    connected_worker_id = Some(id.clone());
                    tracing::info!(worker_id = %id, hostname = %hostname, "Worker registered");

                    let response = ResponseMessage::registration(id, hostname, valid_caps);
                    line_writer
                        .write_line(&serde_json::to_string(&response)?)
                        .await?;
                }
                CommandMessage::Heartbeat { id, status } => {
                    // Update worker status in registry
                    if let Some(worker_id) = &connected_worker_id {
                        let mut workers_guard = workers.write().await;
                        if let Some(reg) = workers_guard.get_mut(worker_id) {
                            reg.last_heartbeat_secs = Some(chrono::Utc::now().timestamp());
                            // Parse status JSON to update worker state
                            if let Ok(status_val) =
                                serde_json::from_str::<serde_json::Value>(&status)
                            {
                                if let Some(status_str) =
                                    status_val.get("status").and_then(|v| v.as_str())
                                {
                                    reg.status = match status_str {
                                        "busy" => crate::distributed::WorkerStatus::Busy,
                                        "idle" => crate::distributed::WorkerStatus::Idle,
                                        _ => crate::distributed::WorkerStatus::Idle,
                                    };
                                }
                            }
                        }
                    }

                    let response = ResponseMessage {
                        id,
                        msg_type: "heartbeat_ack".to_string(),
                        success: true,
                        output: Some(status),
                        error: None,
                        duration_ms: None,
                        hostname: None,
                        capabilities: None,
                    };
                    line_writer
                        .write_line(&serde_json::to_string(&response)?)
                        .await?;
                }
                CommandMessage::Result { id, result } => {
                    task_queue.complete(result).await;
                    let response = ResponseMessage {
                        id,
                        msg_type: "result_ack".to_string(),
                        success: true,
                        output: Some("Result received".to_string()),
                        error: None,
                        duration_ms: None,
                        hostname: None,
                        capabilities: None,
                    };
                    line_writer
                        .write_line(&serde_json::to_string(&response)?)
                        .await?;
                }
                CommandMessage::RequestTasks {
                    id,
                    worker_id,
                    max_tasks,
                } => {
                    let mut tasks = Vec::new();
                    for _ in 0..max_tasks {
                        match task_queue.dequeue(&worker_id).await {
                            Ok(Some(task)) => tasks.push(task),
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                    let tasks_json =
                        serde_json::to_string(&tasks).unwrap_or_else(|_| "[]".to_string());
                    let response = ResponseMessage {
                        id,
                        msg_type: "tasks_assigned".to_string(),
                        success: true,
                        output: Some(tasks_json),
                        error: None,
                        duration_ms: None,
                        hostname: None,
                        capabilities: None,
                    };
                    line_writer
                        .write_line(&serde_json::to_string(&response)?)
                        .await?;
                }
                CommandMessage::AssignTasks { .. } => {
                    // Workers don't receive AssignTasks; this is coordinator-only
                }
                CommandMessage::EnqueueTask { id, task } => match task_queue.enqueue(task).await {
                    Ok(()) => {
                        let response = ResponseMessage {
                            id,
                            msg_type: "enqueue_ack".to_string(),
                            success: true,
                            output: Some("Task enqueued".to_string()),
                            error: None,
                            duration_ms: None,
                            hostname: None,
                            capabilities: None,
                        };
                        line_writer
                            .write_line(&serde_json::to_string(&response)?)
                            .await?;
                    }
                    Err(e) => {
                        let response = ResponseMessage::error(id, e.to_string(), None);
                        line_writer
                            .write_line(&serde_json::to_string(&response)?)
                            .await?;
                    }
                },
                CommandMessage::StatusRequest { id } => {
                    let workers_snapshot: Vec<crate::distributed::WorkerRegistration> =
                        workers.read().await.values().cloned().collect();
                    let (pending, in_progress, completed) = (
                        task_queue.get_pending_count().await,
                        task_queue.get_in_progress_count().await,
                        task_queue.get_completed_count().await,
                    );
                    let status_data = serde_json::json!({
                        "workers": workers_snapshot,
                        "queue": {
                            "pending": pending,
                            "in_progress": in_progress,
                            "completed": completed,
                        },
                    });
                    let response = ResponseMessage {
                        id,
                        msg_type: "status".to_string(),
                        success: true,
                        output: Some(status_data.to_string()),
                        error: None,
                        duration_ms: None,
                        hostname: None,
                        capabilities: None,
                    };
                    line_writer
                        .write_line(&serde_json::to_string(&response)?)
                        .await?;
                }
            }
        }

        // Cleanup — mark worker as disconnected but keep in registry for status visibility
        if let Some(worker_id) = &connected_worker_id {
            if let Some(reg) = workers.write().await.get_mut(worker_id) {
                reg.status = crate::distributed::WorkerStatus::Disconnected;
            }
            tracing::info!(worker_id = %worker_id, "Worker connection closed");
        }
        connections.write().await.remove(&addr_str);
        tracing::info!(addr = %addr, "Client disconnected");
        Ok(())
    }

    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthMessage {
    psk: String,
}

pub struct RemoteClient {
    psk: String,
    tls: Option<TlsClient>,
    cached_addr: Option<(SocketAddr, Instant)>,
    plaintext_allowed: bool,
}

impl Drop for RemoteClient {
    fn drop(&mut self) {
        tracing::debug!("RemoteClient dropped, cleaning up connection");
    }
}

impl RemoteClient {
    pub fn new(psk: String) -> Self {
        Self {
            psk,
            tls: None,
            cached_addr: None,
            plaintext_allowed: false,
        }
    }

    pub fn with_tls(psk: String, domain: &str) -> Result<Self> {
        let tls = TlsClient::new(domain)
            .map_err(|e| EggsecError::Network(format!("Failed to initialize TLS client: {}", e)))?;
        Ok(Self {
            psk,
            tls: Some(tls),
            cached_addr: None,
            plaintext_allowed: false,
        })
    }

    /// Test-only verified-TLS client: verifies against the given test root
    /// DER instead of the WebPKI roots. Crate-private, no feature flag, no
    /// insecure bypass.
    #[cfg(test)]
    pub(crate) fn with_test_root(psk: String, domain: &str, root_der: &[u8]) -> Result<Self> {
        let tls = TlsClient::with_test_root(domain, root_der).map_err(|e| {
            EggsecError::Network(format!("Failed to initialize test TLS client: {}", e))
        })?;
        Ok(Self {
            psk,
            tls: Some(tls),
            cached_addr: None,
            plaintext_allowed: false,
        })
    }

    pub fn new_plaintext(psk: String) -> Self {
        tracing::warn!(
            "Creating plaintext distributed client; the PSK and task data will be sent unencrypted"
        );
        let mut client = Self::new(psk);
        client.plaintext_allowed = true;
        client
    }

    pub fn is_tls(&self) -> bool {
        self.tls.is_some()
    }

    /// Returns a cached DNS resolution if still within TTL (60s).
    ///
    /// NOTE: Cached addresses are not re-validated for reachability within the TTL.
    /// Connection failures are handled by the caller, which falls back to fresh
    /// DNS resolution on the next attempt.
    fn resolve_cached(&self, _host: &str, _port: u16) -> Option<SocketAddr> {
        let now = Instant::now();
        if let Some((addr, cached_at)) = self.cached_addr {
            if now.duration_since(cached_at) < Duration::from_secs(60) {
                tracing::debug!(addr = %addr, "Using cached DNS resolution");
                return Some(addr);
            }
        }
        None
    }

    fn cache_resolution(&mut self, addr: SocketAddr) {
        self.cached_addr = Some((addr, Instant::now()));
    }

    async fn connect_to_coordinator(&mut self, host: &str, port: u16) -> Result<LineWriter> {
        let host_port = format!("{}:{}", host, port);

        let addr = if let Some(cached) = self.resolve_cached(host, port) {
            cached
        } else {
            let resolved: SocketAddr = tokio::net::lookup_host(&host_port)
                .await
                .map_err(|e| EggsecError::Network(format!("Failed to resolve host: {}", e)))?
                .next()
                .ok_or_else(|| EggsecError::Network("No addresses found for host".to_string()))?;
            self.cache_resolution(resolved);
            resolved
        };

        self.connect_to_coordinator_with_addr(&addr).await
    }

    async fn connect_to_coordinator_with_addr(&mut self, addr: &SocketAddr) -> Result<LineWriter> {
        if self.tls.is_none() && !self.plaintext_allowed {
            return Err(EggsecError::Config(
                "TLS is required for distributed clients; use with_tls or explicitly opt into plaintext"
                    .to_string(),
            ));
        }
        let connect_timeout = std::time::Duration::from_secs(5);
        let stream = connect_with_nodelay_timeout(addr, connect_timeout)
            .await
            .map_err(|e| EggsecError::Network(format!("Failed to connect: {}", e)))?;

        let stream = match &self.tls {
            Some(tls_client) => {
                #[cfg(feature = "insecure-tls")]
                {
                    let peer_addr = stream.peer_addr().ok();
                    let local_addr = stream.local_addr().ok();
                    tls_client.increment_insecure_connection();
                    tracing::warn!(
                        local_addr = ?local_addr,
                        peer_addr = ?peer_addr,
                        domain = %tls_client.domain(),
                        "Establishing INSECURE TLS connection (certificate verification disabled)"
                    );
                }
                match StreamWrapper::connect_tls(
                    tls_client.connector(),
                    tls_client.domain(),
                    stream,
                )
                .await
                {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("TLS handshake failed: {}", e);
                        return Err(EggsecError::Network(format!("TLS handshake failed: {}", e)));
                    }
                }
            }
            None => StreamWrapper::plain(stream),
        };

        let mut line_writer = LineWriter::new(stream);

        let auth = AuthMessage {
            psk: self.psk.clone(),
        };
        line_writer
            .write_line(&serde_json::to_string(&auth)?)
            .await?;

        let auth_response: ResponseMessage =
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                let line = line_writer
                    .read_line()
                    .await?
                    .ok_or_else(|| EggsecError::Network("No response".to_string()))?;
                Ok::<_, EggsecError>(serde_json::from_str::<ResponseMessage>(&line)?)
            })
            .await
            .map_err(|_| EggsecError::Network("Authentication response timed out".to_string()))??;

        if !auth_response.success {
            return Err(EggsecError::Validation(format!(
                "Authentication failed: {:?}",
                auth_response.error
            )));
        }

        Ok(line_writer)
    }

    pub async fn register_worker(
        &mut self,
        host: &str,
        port: u16,
        worker_id: String,
        hostname: String,
        capabilities: Vec<String>,
    ) -> Result<()> {
        let mut line_writer = self.connect_to_coordinator(host, port).await?;

        let cmd = CommandMessage::Register {
            id: worker_id.clone(),
            hostname,
            capabilities,
        };

        line_writer
            .write_line(&serde_json::to_string(&cmd)?)
            .await?;

        let response: ResponseMessage =
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                let line = line_writer
                    .read_line()
                    .await?
                    .ok_or_else(|| EggsecError::Network("No response".to_string()))?;
                Ok::<_, EggsecError>(serde_json::from_str::<ResponseMessage>(&line)?)
            })
            .await
            .map_err(|_| EggsecError::Network("Registration response timed out".to_string()))??;

        if !response.success {
            return Err(EggsecError::Validation(format!(
                "Registration failed: {:?}",
                response.error
            )));
        }

        tracing::info!(worker_id = %worker_id, "Worker registered successfully");
        Ok(())
    }

    pub async fn send_heartbeat(
        &mut self,
        host: &str,
        port: u16,
        _worker_id: String,
        status: String,
    ) -> Result<()> {
        let host_port = format!("{}:{}", host, port);

        let addr = if let Some(cached) = self.resolve_cached(host, port) {
            cached
        } else {
            let resolved: SocketAddr = tokio::net::lookup_host(&host_port)
                .await
                .map_err(|e| EggsecError::Network(format!("Failed to resolve host: {}", e)))?
                .next()
                .ok_or_else(|| EggsecError::Network("No addresses found for host".to_string()))?;
            self.cache_resolution(resolved);
            resolved
        };

        let mut line_writer = self.connect_to_coordinator_with_addr(&addr).await?;

        let cmd = CommandMessage::Heartbeat {
            id: uuid::Uuid::new_v4().to_string(),
            status,
        };

        line_writer
            .write_line(&serde_json::to_string(&cmd)?)
            .await?;

        let _response: ResponseMessage =
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                let line = line_writer
                    .read_line()
                    .await?
                    .ok_or_else(|| EggsecError::Network("No response".to_string()))?;
                Ok::<_, EggsecError>(serde_json::from_str::<ResponseMessage>(&line)?)
            })
            .await
            .map_err(|_| EggsecError::Network("Heartbeat response timed out".to_string()))??;

        Ok(())
    }

    pub async fn send_result(
        &mut self,
        host: &str,
        port: u16,
        result: crate::distributed::queue::TaskResult,
    ) -> Result<()> {
        let host_port = format!("{}:{}", host, port);

        let addr = if let Some(cached) = self.resolve_cached(host, port) {
            cached
        } else {
            let resolved: SocketAddr = tokio::net::lookup_host(&host_port)
                .await
                .map_err(|e| EggsecError::Network(format!("Failed to resolve host: {}", e)))?
                .next()
                .ok_or_else(|| EggsecError::Network("No addresses found for host".to_string()))?;
            self.cache_resolution(resolved);
            resolved
        };

        let mut line_writer = self.connect_to_coordinator_with_addr(&addr).await?;

        let cmd = CommandMessage::Result {
            id: uuid::Uuid::new_v4().to_string(),
            result,
        };

        line_writer
            .write_line(&serde_json::to_string(&cmd)?)
            .await?;

        let _response: ResponseMessage =
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                let line = line_writer
                    .read_line()
                    .await?
                    .ok_or_else(|| EggsecError::Network("No response".to_string()))?;
                Ok::<_, EggsecError>(serde_json::from_str::<ResponseMessage>(&line)?)
            })
            .await
            .map_err(|_| EggsecError::Network("Result response timed out".to_string()))??;

        Ok(())
    }

    pub async fn request_tasks(
        &mut self,
        host: &str,
        port: u16,
        worker_id: String,
        max_tasks: usize,
    ) -> Result<Vec<crate::distributed::queue::Task>> {
        let host_port = format!("{}:{}", host, port);

        let addr = if let Some(cached) = self.resolve_cached(host, port) {
            cached
        } else {
            let resolved: SocketAddr = tokio::net::lookup_host(&host_port)
                .await
                .map_err(|e| EggsecError::Network(format!("Failed to resolve host: {}", e)))?
                .next()
                .ok_or_else(|| EggsecError::Network("No addresses found for host".to_string()))?;
            self.cache_resolution(resolved);
            resolved
        };

        let mut line_writer = self.connect_to_coordinator_with_addr(&addr).await?;

        let cmd = CommandMessage::RequestTasks {
            id: uuid::Uuid::new_v4().to_string(),
            worker_id,
            max_tasks,
        };

        line_writer
            .write_line(&serde_json::to_string(&cmd)?)
            .await?;

        let response: ResponseMessage =
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                let line = line_writer
                    .read_line()
                    .await?
                    .ok_or_else(|| EggsecError::Network("No response".to_string()))?;
                Ok::<_, EggsecError>(serde_json::from_str::<ResponseMessage>(&line)?)
            })
            .await
            .map_err(|_| EggsecError::Network("Task request response timed out".to_string()))??;

        if !response.success {
            return Ok(Vec::new());
        }

        let tasks: Vec<crate::distributed::queue::Task> = match response.output {
            Some(o) => serde_json::from_str(&o).unwrap_or_else(|e| {
                tracing::warn!("Failed to deserialize task list from coordinator: {}", e);
                Vec::new()
            }),
            None => Vec::new(),
        };

        Ok(tasks)
    }

    pub async fn execute(
        &mut self,
        host: &str,
        port: u16,
        command: Vec<String>,
        timeout_secs: Option<u64>,
    ) -> Result<crate::distributed::command::RemoteResult> {
        let host_port = format!("{}:{}", host, port);

        let addr = if let Some(cached) = self.resolve_cached(host, port) {
            cached
        } else {
            let resolved: SocketAddr = tokio::net::lookup_host(&host_port)
                .await
                .map_err(|e| EggsecError::Network(format!("Failed to resolve host: {}", e)))?
                .next()
                .ok_or_else(|| EggsecError::Network("No addresses found for host".to_string()))?;
            self.cache_resolution(resolved);
            resolved
        };

        if self.tls.is_none() && !self.plaintext_allowed {
            return Err(EggsecError::Config(
                "TLS is required for distributed clients; use with_tls or explicitly opt into plaintext"
                    .to_string(),
            ));
        }
        let connect_timeout = std::time::Duration::from_secs(5);
        let stream = connect_with_nodelay_timeout(&addr, connect_timeout)
            .await
            .map_err(|e| EggsecError::Network(format!("Failed to connect: {}", e)))?;

        let stream = match &self.tls {
            Some(tls_client) => {
                #[cfg(feature = "insecure-tls")]
                {
                    let peer_addr = stream.peer_addr().ok();
                    let local_addr = stream.local_addr().ok();
                    tls_client.increment_insecure_connection();
                    tracing::warn!(
                        local_addr = ?local_addr,
                        peer_addr = ?peer_addr,
                        domain = %tls_client.domain(),
                        "Establishing INSECURE TLS connection (certificate verification disabled)"
                    );
                }
                match StreamWrapper::connect_tls(
                    tls_client.connector(),
                    tls_client.domain(),
                    stream,
                )
                .await
                {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("TLS handshake failed: {}", e);
                        return Err(EggsecError::Network(format!("TLS handshake failed: {}", e)));
                    }
                }
            }
            None => StreamWrapper::plain(stream),
        };

        let mut line_writer = LineWriter::new(stream);

        let auth = AuthMessage {
            psk: self.psk.clone(),
        };
        line_writer
            .write_line(&serde_json::to_string(&auth)?)
            .await?;

        // Auth response timeout
        let auth_response: ResponseMessage =
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                let line = line_writer
                    .read_line()
                    .await?
                    .ok_or_else(|| EggsecError::Network("No response".to_string()))?;
                Ok::<_, EggsecError>(serde_json::from_str::<ResponseMessage>(&line)?)
            })
            .await
            .map_err(|_| EggsecError::Network("Authentication response timed out".to_string()))??;

        if !auth_response.success {
            return Err(EggsecError::Validation(format!(
                "Authentication failed: {:?}",
                auth_response.error
            )));
        }

        let hostname = auth_response.hostname.unwrap_or_else(|| host.to_string());

        let cmd = CommandMessage::Execute {
            id: uuid::Uuid::new_v4().to_string(),
            command,
            timeout: timeout_secs,
            env: None,
        };

        line_writer
            .write_line(&serde_json::to_string(&cmd)?)
            .await?;

        // Response timeout (default 60 seconds if not specified)
        let response_timeout = std::time::Duration::from_secs(timeout_secs.unwrap_or(60));
        let response: ResponseMessage = tokio::time::timeout(response_timeout, async {
            let response_line = line_writer
                .read_line()
                .await?
                .ok_or_else(|| EggsecError::Network("No response".to_string()))?;
            Ok::<_, EggsecError>(serde_json::from_str::<ResponseMessage>(&response_line)?)
        })
        .await
        .map_err(|_| {
            EggsecError::Network(format!(
                "Response timed out after {} seconds",
                response_timeout.as_secs()
            ))
        })??;

        Ok(crate::distributed::command::RemoteResult::new(
            hostname,
            response.success,
            response.output.unwrap_or_default(),
            response.error,
            response.duration_ms.unwrap_or(0),
        ))
    }

    pub async fn request_status(&mut self, host: &str, port: u16) -> Result<serde_json::Value> {
        let mut line_writer = self.connect_to_coordinator(host, port).await?;

        let cmd = CommandMessage::StatusRequest {
            id: uuid::Uuid::new_v4().to_string(),
        };

        line_writer
            .write_line(&serde_json::to_string(&cmd)?)
            .await?;

        let response: ResponseMessage =
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                let line = line_writer
                    .read_line()
                    .await?
                    .ok_or_else(|| EggsecError::Network("No response".to_string()))?;
                Ok::<_, EggsecError>(serde_json::from_str::<ResponseMessage>(&line)?)
            })
            .await
            .map_err(|_| EggsecError::Network("Status request timed out".to_string()))??;

        if !response.success {
            return Err(EggsecError::Network(
                response
                    .error
                    .unwrap_or_else(|| "Status request failed".to_string()),
            ));
        }

        let data: serde_json::Value = match response.output {
            Some(o) => match serde_json::from_str(&o) {
                Ok(value) => value,
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse coordinator status payload: {}; returning empty object",
                        e
                    );
                    serde_json::json!({})
                }
            },
            None => serde_json::json!({}),
        };

        Ok(data)
    }

    pub async fn enqueue_task(
        &mut self,
        host: &str,
        port: u16,
        task: crate::distributed::queue::Task,
    ) -> Result<()> {
        let mut line_writer = self.connect_to_coordinator(host, port).await?;

        let cmd = CommandMessage::EnqueueTask {
            id: uuid::Uuid::new_v4().to_string(),
            task,
        };

        line_writer
            .write_line(&serde_json::to_string(&cmd)?)
            .await?;

        let response: ResponseMessage =
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                let line = line_writer
                    .read_line()
                    .await?
                    .ok_or_else(|| EggsecError::Network("No response".to_string()))?;
                Ok::<_, EggsecError>(serde_json::from_str::<ResponseMessage>(&line)?)
            })
            .await
            .map_err(|_| EggsecError::Network("Enqueue task response timed out".to_string()))??;

        if !response.success {
            return Err(EggsecError::Network(
                response
                    .error
                    .unwrap_or_else(|| "Failed to enqueue task".to_string()),
            ));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Phase E: persistent coordinator session reuse.
// ---------------------------------------------------------------------------

/// Bound on queued session commands. Result submission must not become an
/// unbounded memory sink; producers backpressure on a full queue while the
/// actor drains. Failed establishments fail fast inside the reconnect window
/// (no actor sleeps), so bursts never head-of-line block.
pub(crate) const SESSION_COMMAND_QUEUE: usize = 64;

/// Connection configuration for a [`CoordinatorSession`].
#[derive(Debug, Clone)]
pub(crate) struct SessionConfig {
    pub host: String,
    pub port: u16,
    pub psk: String,
    /// TLS server domain. `None` with `plaintext_allowed` selects the
    /// explicit plaintext opt-in (isolated lab/tests only).
    pub tls_domain: Option<String>,
    pub plaintext_allowed: bool,
}

/// Worker registration metadata remembered by the session and replayed on
/// every (re)connect before connection-local heartbeat state is relied upon.
#[derive(Debug, Clone)]
struct SessionRegistration {
    worker_id: String,
    hostname: String,
    capabilities: Vec<String>,
}

/// Actor-protocol command. The line protocol is request/response without
/// multiplexing correlation, so steady-state commands are serialized by the
/// single connection owner and results return over `oneshot` channels.
enum SessionCommand {
    Register {
        registration: SessionRegistration,
        reply: oneshot::Sender<Result<()>>,
    },
    Heartbeat {
        status: String,
        reply: oneshot::Sender<Result<()>>,
    },
    RequestTasks {
        worker_id: String,
        max_tasks: usize,
        reply: oneshot::Sender<Result<Vec<crate::distributed::queue::Task>>>,
    },
    SendResult {
        result: crate::distributed::queue::TaskResult,
        reply: oneshot::Sender<Result<()>>,
    },
}

impl SessionCommand {
    /// Fail the caller with `err` (the session is down or shutting down).
    fn fail(self, err: EggsecError) {
        match self {
            SessionCommand::Register { reply, .. } => {
                let _ = reply.send(Err(err));
            }
            SessionCommand::Heartbeat { reply, .. } => {
                let _ = reply.send(Err(err));
            }
            SessionCommand::RequestTasks { reply, .. } => {
                let _ = reply.send(Err(err));
            }
            SessionCommand::SendResult { reply, .. } => {
                let _ = reply.send(Err(err));
            }
        }
    }
}

/// Persistent, authenticated coordinator session (Phase E).
///
/// One actor task owns the connection (`LineWriter`) for its whole lifetime:
/// registration, heartbeats, capacity-aware task acquisition, and result
/// submission all multiplex over a single TCP/TLS/PSK setup per healthy
/// connection lifetime instead of one setup per message. The one-shot
/// [`RemoteClient`] methods are unchanged for CLI/tool callers.
///
/// Retry disposition (documented per method):
/// - heartbeat: safe to retry once after reconnect (idempotent status
///   update; the next cadence tick recovers anything else);
/// - task acquisition: never transparently retried — a lost response after
///   the coordinator dequeues tasks must not cause hidden loss or duplicate
///   execution, so the error surfaces and the next normal poll recovers
///   through existing stale-task behavior;
/// - result submission: never transparently retried — `complete()` appends
///   to the completed deque, so replay could duplicate results; the error
///   surfaces (matching the pre-session warn-and-continue behavior);
/// - registration: performed on every (re)connect while remembered, before
///   any heartbeat depends on connection-local worker identity.
pub(crate) struct CoordinatorSession {
    commands: mpsc::Sender<SessionCommand>,
    shutdown_tx: watch::Sender<bool>,
    actor: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl CoordinatorSession {
    /// Spawn the session owner. No I/O happens until the first command
    /// (registration drives the first connect).
    pub(crate) fn spawn(config: SessionConfig) -> Self {
        let (commands, receiver) = mpsc::channel(SESSION_COMMAND_QUEUE);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let actor_handle = tokio::spawn(session_actor(config, receiver, shutdown_rx));
        Self {
            commands,
            shutdown_tx,
            actor: std::sync::Mutex::new(Some(actor_handle)),
        }
    }

    /// Test-only spawn with verified test trust (DER-encoded test root).
    /// Uses the same actor loop as production; only the TLS trust differs.
    #[cfg(test)]
    pub(crate) fn spawn_with_test_root(config: SessionConfig, root_der: Vec<u8>) -> Self {
        let (commands, receiver) = mpsc::channel(SESSION_COMMAND_QUEUE);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let actor_handle = tokio::spawn(session_actor_with_test_root(
            config,
            receiver,
            shutdown_rx,
            root_der,
        ));
        Self {
            commands,
            shutdown_tx,
            actor: std::sync::Mutex::new(Some(actor_handle)),
        }
    }

    /// Register (or re-register) the worker. The metadata is remembered for
    /// reconnect replay.
    pub(crate) async fn register(
        &self,
        worker_id: String,
        hostname: String,
        capabilities: Vec<String>,
    ) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.commands
            .send(SessionCommand::Register {
                registration: SessionRegistration {
                    worker_id,
                    hostname,
                    capabilities,
                },
                reply: reply_tx,
            })
            .await
            .map_err(|_| EggsecError::Network("coordinator session is shut down".to_string()))?;
        reply_rx
            .await
            .map_err(|_| EggsecError::Network("coordinator session shut down".to_string()))?
    }

    /// Send a heartbeat over the shared session.
    pub(crate) async fn heartbeat(&self, status: String) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.commands
            .send(SessionCommand::Heartbeat {
                status,
                reply: reply_tx,
            })
            .await
            .map_err(|_| EggsecError::Network("coordinator session is shut down".to_string()))?;
        reply_rx
            .await
            .map_err(|_| EggsecError::Network("coordinator session shut down".to_string()))?
    }

    /// Acquire tasks over the shared session. Never transparently retried.
    pub(crate) async fn request_tasks(
        &self,
        worker_id: String,
        max_tasks: usize,
    ) -> Result<Vec<crate::distributed::queue::Task>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.commands
            .send(SessionCommand::RequestTasks {
                worker_id,
                max_tasks,
                reply: reply_tx,
            })
            .await
            .map_err(|_| EggsecError::Network("coordinator session is shut down".to_string()))?;
        reply_rx
            .await
            .map_err(|_| EggsecError::Network("coordinator session shut down".to_string()))?
    }

    /// Submit a task result over the shared session. Never transparently
    /// retried (see the disposition above).
    pub(crate) async fn send_result(
        &self,
        result: crate::distributed::queue::TaskResult,
    ) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.commands
            .send(SessionCommand::SendResult {
                result,
                reply: reply_tx,
            })
            .await
            .map_err(|_| EggsecError::Network("coordinator session is shut down".to_string()))?;
        reply_rx
            .await
            .map_err(|_| EggsecError::Network("coordinator session shut down".to_string()))?
    }

    /// Stop accepting new commands, fail pending callers, and close the
    /// connection owner without leaving a detached reconnect loop.
    pub(crate) fn shutdown(&self) {
        if let Err(e) = self.shutdown_tx.send(true) {
            tracing::debug!(?e, "Coordinator session shutdown receiver already dropped");
        }
        if let Ok(mut actor) = self.actor.lock() {
            if let Some(handle) = actor.take() {
                handle.abort();
            }
        }
    }
}

impl Drop for CoordinatorSession {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Send one command and read its response on an owned connection.
async fn session_exchange(
    writer: &mut LineWriter,
    command: &CommandMessage,
    timeout_secs: u64,
    timeout_message: &str,
) -> Result<ResponseMessage> {
    writer.write_line(&serde_json::to_string(command)?).await?;
    tokio::time::timeout(Duration::from_secs(timeout_secs), async {
        let line = writer
            .read_line()
            .await?
            .ok_or_else(|| EggsecError::Network("No response".to_string()))?;
        Ok::<_, EggsecError>(serde_json::from_str::<ResponseMessage>(&line)?)
    })
    .await
    .map_err(|_| EggsecError::Network(timeout_message.to_string()))?
}

/// Establish one authenticated connection and replay remembered worker
/// registration on it. Every setup performs TCP connect, TLS handshake
/// (when configured), and PSK authentication — persistent reuse means
/// fewer setups, never weaker ones.
async fn session_establish(
    client: &mut RemoteClient,
    host: &str,
    port: u16,
    registration: Option<&SessionRegistration>,
) -> Result<LineWriter> {
    let mut writer = client.connect_to_coordinator(host, port).await?;
    if let Some(registration) = registration {
        let command = CommandMessage::Register {
            id: registration.worker_id.clone(),
            hostname: registration.hostname.clone(),
            capabilities: registration.capabilities.clone(),
        };
        let response =
            session_exchange(&mut writer, &command, 10, "Registration response timed out").await?;
        if !response.success {
            return Err(EggsecError::Validation(format!(
                "Registration failed: {:?}",
                response.error
            )));
        }
        tracing::info!(worker_id = %registration.worker_id, "Worker (re-)registered on coordinator session");
    }
    Ok(writer)
}

async fn session_actor(
    config: SessionConfig,
    commands: mpsc::Receiver<SessionCommand>,
    shutdown: watch::Receiver<bool>,
) {
    let client = if let Some(domain) = config.tls_domain.clone() {
        match RemoteClient::with_tls(config.psk.clone(), &domain) {
            Ok(client) => client,
            Err(e) => {
                tracing::error!(%e, "Failed to initialize coordinator session TLS");
                return;
            }
        }
    } else if config.plaintext_allowed {
        RemoteClient::new_plaintext(config.psk.clone())
    } else {
        tracing::error!("TLS domain is required for coordinator sessions");
        return;
    };

    session_actor_loop(config, commands, shutdown, client).await;
}

/// Test-only actor entry: same loop, but the client verifies against the
/// given test root DER instead of the WebPKI roots.
#[cfg(test)]
async fn session_actor_with_test_root(
    config: SessionConfig,
    commands: mpsc::Receiver<SessionCommand>,
    shutdown: watch::Receiver<bool>,
    root_der: Vec<u8>,
) {
    let domain = match config.tls_domain.clone() {
        Some(domain) => domain,
        None => {
            tracing::error!("TLS domain is required for test coordinator sessions");
            return;
        }
    };
    let client = match RemoteClient::with_test_root(config.psk.clone(), &domain, &root_der) {
        Ok(client) => client,
        Err(e) => {
            tracing::error!(%e, "Failed to initialize test coordinator session TLS");
            return;
        }
    };

    session_actor_loop(config, commands, shutdown, client).await;
}

async fn session_actor_loop(
    config: SessionConfig,
    mut commands: mpsc::Receiver<SessionCommand>,
    mut shutdown: watch::Receiver<bool>,
    mut client: RemoteClient,
) {
    let mut writer: Option<LineWriter> = None;
    let mut registration: Option<SessionRegistration> = None;
    let mut reconnect = SessionReconnect::default();

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                tracing::info!("Coordinator session shutting down");
                break;
            }
            command = commands.recv() => {
                let Some(command) = command else {
                    break;
                };
                // Short registration commands update remembered metadata.
                if let SessionCommand::Register { registration: fresh, reply } = command {
                    registration = Some(fresh);
                    // Force (re-)establishment so registration is verified
                    // on a live connection before reporting success.
                    writer = None;
                    match reconnect.ensure_connected(&mut client, &config.host, config.port, registration.as_ref()).await {
                        Ok(established) => {
                            writer = Some(established);
                            let _ = reply.send(Ok(()));
                        }
                        Err(e) => {
                            tracing::warn!("Coordinator session registration failed: {}", e);
                            reply.send(Err(e)).unwrap_or(());
                        }
                    }
                    continue;
                }

                // Ensure a live authenticated connection (windowed: recent
                // failures fail fast without a new setup).
                if writer.is_none() {
                    match reconnect.ensure_connected(&mut client, &config.host, config.port, registration.as_ref()).await {
                        Ok(established) => {
                            writer = Some(established);
                        }
                        Err(e) => {
                            tracing::warn!("Coordinator session establish failed: {}", e);
                            command.fail(e);
                            continue;
                        }
                    }
                }

                let writer_ref = writer.as_mut().expect("session writer established");
                match command {
                    SessionCommand::Register { .. } => {
                        // Unreachable: handled above.
                    }
                    SessionCommand::Heartbeat { status, reply } => {
                        let command = CommandMessage::Heartbeat {
                            id: uuid::Uuid::new_v4().to_string(),
                            status,
                        };
                        match session_exchange(writer_ref, &command, 5, "Heartbeat response timed out").await {
                            Ok(_) => {
                                reply.send(Ok(())).unwrap_or(());
                            }
                            Err(first_err) => {
                                // Broken stream: discard and retry the
                                // idempotent heartbeat once on a fresh
                                // connection before failing the caller. The
                                // retry goes through the same reconnect
                                // window as any other establishment.
                                tracing::debug!("Session heartbeat failed, re-establishing: {}", first_err);
                                writer = None;
                                match reconnect.ensure_connected(&mut client, &config.host, config.port, registration.as_ref()).await {
                                    Ok(established) => {
                                        let retry = session_exchange(&mut *writer.insert(established), &command, 5, "Heartbeat response timed out").await;
                                        match retry {
                                            Ok(_) => reply.send(Ok(())).unwrap_or(()),
                                            Err(e) => {
                                                writer = None;
                                                reply.send(Err(e)).unwrap_or(());
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        reply.send(Err(e)).unwrap_or(());
                                    }
                                }
                            }
                        }
                    }
                    SessionCommand::RequestTasks { worker_id, max_tasks, reply } => {
                        let command = CommandMessage::RequestTasks {
                            id: uuid::Uuid::new_v4().to_string(),
                            worker_id,
                            max_tasks,
                        };
                        match session_exchange(writer_ref, &command, 10, "Task request response timed out").await {
                            Ok(response) => {
                                if !response.success {
                                    reply.send(Ok(Vec::new())).unwrap_or(());
                                    continue;
                                }
                                let tasks: Vec<crate::distributed::queue::Task> = match response.output {
                                    Some(output) => serde_json::from_str(&output).unwrap_or_else(|e| {
                                        tracing::warn!("Failed to deserialize task list from coordinator: {}", e);
                                        Vec::new()
                                    }),
                                    None => Vec::new(),
                                };
                                reply.send(Ok(tasks)).unwrap_or(());
                            }
                            Err(e) => {
                                // Never transparently retried: a lost
                                // response may follow a server-side dequeue.
                                // The next normal poll recovers via
                                // stale-task behavior.
                                tracing::debug!("Session task request failed: {}", e);
                                writer = None;
                                reply.send(Err(e)).unwrap_or(());
                            }
                        }
                    }
                    SessionCommand::SendResult { result, reply } => {
                        let command = CommandMessage::Result {
                            id: uuid::Uuid::new_v4().to_string(),
                            result,
                        };
                        match session_exchange(writer_ref, &command, 10, "Result response timed out").await {
                            Ok(_) => {
                                reply.send(Ok(())).unwrap_or(());
                            }
                            Err(e) => {
                                // Never transparently retried: replays could
                                // duplicate completed entries.
                                tracing::debug!("Session result submission failed: {}", e);
                                writer = None;
                                reply.send(Err(e)).unwrap_or(());
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Reconnect pacing state. Reconnect attempts are rate-limited to at most
/// one per backoff window (1s/2s/5s by consecutive failures, consistent with
/// worker polling cadence). Attempts inside the window fail fast WITHOUT a
/// new TCP/TLS/auth setup and WITHOUT sleeping the actor, so a broken
/// coordinator can neither cause a tight reconnect loop nor head-of-line
/// block a burst of queued commands; shutdown stays trivially responsive
/// because the actor never sleeps.
#[derive(Debug, Default)]
struct SessionReconnect {
    consecutive_failures: u32,
    last_failure: Option<Instant>,
}

impl SessionReconnect {
    fn window(&self) -> Duration {
        match self.consecutive_failures {
            0 | 1 => Duration::from_secs(1),
            2 => Duration::from_secs(2),
            _ => Duration::from_secs(5),
        }
    }

    /// Establish one authenticated connection (with registration replay),
    /// rate-limited by the backoff window.
    async fn ensure_connected(
        &mut self,
        client: &mut RemoteClient,
        host: &str,
        port: u16,
        registration: Option<&SessionRegistration>,
    ) -> Result<LineWriter> {
        if let Some(last) = self.last_failure {
            let window = self.window();
            if last.elapsed() < window {
                return Err(EggsecError::Network(
                    "coordinator unavailable (reconnect backing off)".to_string(),
                ));
            }
        }
        match session_establish(client, host, port, registration).await {
            Ok(writer) => {
                self.consecutive_failures = 0;
                self.last_failure = None;
                Ok(writer)
            }
            Err(e) => {
                self.consecutive_failures = self.consecutive_failures.saturating_add(1);
                self.last_failure = Some(Instant::now());
                Err(e)
            }
        }
    }
}

/// Phase E session tests (deterministic, plaintext loopback, isolated lab).
///
/// Plaintext exercises the identical connection-lifecycle path as TLS (TCP
/// accept → handshake step → PSK auth → command loop); TLS handshakes ride
/// the same per-setup sequence 1:1 with accepts by construction, so the
/// accept/auth counters below prove session reuse for both.
#[cfg(test)]
mod session_tests {
    use super::*;
    use crate::distributed::queue::{Task, TaskResult};

    async fn free_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = listener.local_addr().expect("addr").port();
        drop(listener);
        port
    }

    async fn start_plaintext_listener(
        psk: &str,
    ) -> (Arc<RemoteListener>, tokio::task::JoinHandle<()>, u16) {
        let port = free_port().await;
        start_plaintext_listener_on(psk, port).await
    }

    async fn start_plaintext_listener_on(
        psk: &str,
        port: u16,
    ) -> (Arc<RemoteListener>, tokio::task::JoinHandle<()>, u16) {
        let listener = Arc::new(RemoteListener::new_plaintext(psk.to_string()));
        let server = Arc::clone(&listener);
        let handle = tokio::spawn(async move {
            let _ = server.start(port).await;
        });
        tokio::time::sleep(Duration::from_millis(200)).await;
        (listener, handle, port)
    }

    fn plaintext_session(psk: &str, port: u16) -> CoordinatorSession {
        CoordinatorSession::spawn(SessionConfig {
            host: "127.0.0.1".to_string(),
            port,
            psk: psk.to_string(),
            tls_domain: None,
            plaintext_allowed: true,
        })
    }

    fn heartbeat_status(worker_id: &str) -> String {
        serde_json::json!({
            "worker_id": worker_id,
            "status": "idle",
            "current_jobs": 0,
            "completed_jobs": 0,
            "failed_jobs": 0,
        })
        .to_string()
    }

    fn result_for(task_id: &str) -> TaskResult {
        TaskResult {
            task_id: task_id.to_string(),
            success: true,
            output: "ok".to_string(),
            error: None,
            duration_millis: 1,
        }
    }

    /// Steady state: register + heartbeats + task requests + results ride
    /// one accepted connection with one PSK authentication.
    #[tokio::test]
    async fn session_reuses_single_connection_in_steady_state() {
        let psk = "test-psk-session-reuse".to_string();
        let (listener, handle, port) = start_plaintext_listener(&psk).await;
        let session = plaintext_session(&psk, port);

        let start = Instant::now();
        session
            .register(
                "worker-1".to_string(),
                "host-1".to_string(),
                vec!["PortScan".to_string()],
            )
            .await
            .expect("register");
        for _ in 0..3 {
            session
                .heartbeat(heartbeat_status("worker-1"))
                .await
                .expect("heartbeat");
        }
        let tasks = session
            .request_tasks("worker-1".to_string(), 5)
            .await
            .expect("request tasks");
        assert!(tasks.is_empty());
        session
            .send_result(result_for("task-1"))
            .await
            .expect("send result");
        let wall = start.elapsed();

        assert_eq!(listener.accepted_count(), 1);
        assert_eq!(listener.authenticated_count(), 1);
        assert_eq!(listener.connection_count().await, 1);
        println!(
            "perf session steady_state ops=6 wall_ms={} accepts={} auths={} live_connections={}",
            wall.as_millis(),
            listener.accepted_count(),
            listener.authenticated_count(),
            listener.connection_count().await
        );

        // Heartbeats depended on the connection-local registration: the
        // coordinator updated heartbeat state for the registered worker.
        let workers = listener.get_workers().await;
        assert_eq!(workers.len(), 1);
        assert_eq!(workers[0].worker_id, "worker-1");
        let last_heartbeat = workers[0].last_heartbeat_secs.expect("heartbeat timestamp");
        assert!(
            chrono::Utc::now().timestamp() - last_heartbeat < 60,
            "stale heartbeat timestamp"
        );

        session.shutdown();
        listener.shutdown();
        handle.abort();
    }

    /// Outage recovery: commands fail fast while down, and after recovery a
    /// heartbeat alone succeeds — proving the remembered registration was
    /// replayed on the new connection before connection-local heartbeat
    /// state was relied upon (no explicit re-register call).
    #[tokio::test]
    async fn session_recovers_and_replays_registration() {
        let psk = "test-psk-session-recover".to_string();
        let port = free_port().await;
        let session = plaintext_session(&psk, port);

        session
            .register(
                "worker-9".to_string(),
                "host-9".to_string(),
                vec!["PortScan".to_string()],
            )
            .await
            .expect_err("register must fail while the coordinator is down");

        let (listener, handle, _) = start_plaintext_listener_on(&psk, port).await;
        // Past the first-failure reconnect window (1s) with margin.
        tokio::time::sleep(Duration::from_millis(1500)).await;

        // No explicit re-register: the heartbeat drives re-establishment +
        // registration replay on the fresh connection.
        session
            .heartbeat(heartbeat_status("worker-9"))
            .await
            .expect("heartbeat after recovery");
        let workers = listener.get_workers().await;
        assert_eq!(workers.len(), 1);
        assert_eq!(workers[0].worker_id, "worker-9");
        assert!(
            workers[0].last_heartbeat_secs.is_some(),
            "heartbeat state requires connection-local registration"
        );
        assert_eq!(listener.authenticated_count(), 1);

        session.shutdown();
        listener.shutdown();
        handle.abort();
    }

    /// Failed authentication never falls back to an unauthenticated session:
    /// every operation fails and no auth is recorded.
    #[tokio::test]
    async fn session_wrong_psk_never_succeeds() {
        let (listener, handle, port) = start_plaintext_listener("right-psk").await;
        let session = plaintext_session("wrong-psk", port);

        assert!(session
            .register("w".to_string(), "h".to_string(), vec![])
            .await
            .is_err());
        assert!(session.heartbeat(heartbeat_status("w")).await.is_err());
        assert!(session.request_tasks("w".to_string(), 1).await.is_err());
        assert!(session.send_result(result_for("t")).await.is_err());
        assert_eq!(listener.authenticated_count(), 0);
        assert_eq!(listener.connection_count().await, 0);

        session.shutdown();
        listener.shutdown();
        handle.abort();
    }

    /// Shutdown fails pending/future callers promptly and leaves no
    /// detached reconnect loop.
    #[tokio::test]
    async fn session_shutdown_fails_callers() {
        let psk = "test-psk-session-shutdown".to_string();
        let (listener, handle, port) = start_plaintext_listener(&psk).await;
        let session = plaintext_session(&psk, port);
        session
            .register("w".to_string(), "h".to_string(), vec![])
            .await
            .expect("register");
        session.shutdown();
        let start = Instant::now();
        assert!(session.heartbeat(heartbeat_status("w")).await.is_err());
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "shutdown caller did not fail promptly"
        );
        assert_eq!(listener.accepted_count(), 1);

        listener.shutdown();
        handle.abort();
    }

    /// The command queue is bounded (structural pin) and stays live under a
    /// burst larger than the bound.
    #[tokio::test]
    async fn session_command_queue_bounded_and_live() {
        assert_eq!(SESSION_COMMAND_QUEUE, 64);
        let psk = "test-psk-session-burst".to_string();
        let (listener, handle, port) = start_plaintext_listener(&psk).await;
        let session = plaintext_session(&psk, port);
        session
            .register("w".to_string(), "h".to_string(), vec![])
            .await
            .expect("register");
        for i in 0..70 {
            session
                .heartbeat(heartbeat_status(&format!("w-{i}")))
                .await
                .expect("burst heartbeat");
        }
        assert_eq!(listener.accepted_count(), 1);

        session.shutdown();
        listener.shutdown();
        handle.abort();
    }

    /// Concurrent callers keep correct response association: every result
    /// lands under its own task id.
    #[tokio::test]
    async fn session_concurrent_results_keep_association() {
        let psk = "test-psk-session-assoc".to_string();
        let (listener, handle, port) = start_plaintext_listener(&psk).await;

        // Enqueue through the one-shot path (unchanged public behavior).
        for i in 0..5 {
            let mut client = RemoteClient::new_plaintext(psk.clone());
            client
                .enqueue_task(
                    "127.0.0.1",
                    port,
                    Task {
                        id: format!("assoc-{i}"),
                        job_id: "job-1".to_string(),
                        task_type: crate::distributed::TaskType::PortScan,
                        target: "example.com".to_string(),
                        payload: rustc_hash::FxHashMap::default(),
                        worker_id: None,
                        assigned_at_secs: None,
                    },
                )
                .await
                .expect("enqueue");
        }

        let session = plaintext_session(&psk, port);
        session
            .register("w".to_string(), "h".to_string(), vec![])
            .await
            .expect("register");
        let mut handles = Vec::new();
        for i in 0..5 {
            let session_ref = &session;
            handles.push(async move {
                session_ref
                    .send_result(result_for(&format!("assoc-{i}")))
                    .await
                    .expect("concurrent result")
            });
        }
        futures::future::join_all(handles).await;

        let completed = listener.completed_results().await;
        let mut ids: Vec<String> = completed.iter().map(|r| r.task_id.clone()).collect();
        ids.sort();
        assert_eq!(
            ids,
            vec![
                "assoc-0".to_string(),
                "assoc-1".to_string(),
                "assoc-2".to_string(),
                "assoc-3".to_string(),
                "assoc-4".to_string(),
            ]
        );

        session.shutdown();
        listener.shutdown();
        handle.abort();
    }

    /// Acquisition/result errors surface instead of hanging or speculative
    /// replay while the coordinator is down.
    #[tokio::test]
    async fn session_errors_surface_without_retry_or_hang() {
        let port = free_port().await;
        let session = plaintext_session("psk", port);
        let start = Instant::now();
        assert!(session.request_tasks("w".to_string(), 2).await.is_err());
        assert!(session.send_result(result_for("t")).await.is_err());
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "control errors did not surface promptly"
        );
        session.shutdown();
    }

    // -----------------------------------------------------------------------
    // Verified local TLS session fixture (corrective pass Workstreams 2-3).
    //
    // Uses short-lived `rcgen` localhost material, the production
    // `TlsServer::from_pem` accept path, and test-only verified trust
    // (`TlsClient::with_test_root` / `RemoteClient::with_test_root` /
    // `CoordinatorSession::spawn_with_test_root`). No public constructor,
    // no feature flag, no `insecure-tls` bypass, no checked-in key/cert;
    // PEM files live under a test tempdir that cleans up on drop.
    // -----------------------------------------------------------------------

    fn generate_tls_material() -> (Vec<u8>, String, String) {
        let sans = vec!["localhost".to_string(), "127.0.0.1".to_string()];
        let params = rcgen::CertificateParams::new(sans).expect("cert params");
        let key_pair = rcgen::KeyPair::generate().expect("key pair");
        let cert = params.self_signed(&key_pair).expect("self-signed");
        let cert_der = cert.der().to_vec();
        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();
        (cert_der, cert_pem, key_pem)
    }

    async fn start_tls_listener(
        psk: &str,
    ) -> (
        Arc<RemoteListener>,
        tokio::task::JoinHandle<()>,
        u16,
        Vec<u8>,
        tempfile::TempDir,
    ) {
        let (cert_der, cert_pem, key_pem) = generate_tls_material();
        let dir = tempfile::TempDir::new().expect("tempdir");
        let cert_path = dir.path().join("test-cert.pem");
        let key_path = dir.path().join("test-key.pem");
        std::fs::write(&cert_path, cert_pem.as_bytes()).expect("write cert");
        std::fs::write(&key_path, key_pem.as_bytes()).expect("write key");

        let port = free_port().await;
        let listener = Arc::new(
            RemoteListener::with_tls(
                psk.to_string(),
                TlsConfig {
                    cert_path,
                    key_path,
                },
            )
            .expect("tls listener"),
        );
        assert!(listener.is_tls(), "test listener must be TLS");
        let server = Arc::clone(&listener);
        let handle = tokio::spawn(async move {
            let _ = server.start(port).await;
        });
        tokio::time::sleep(Duration::from_millis(200)).await;
        (listener, handle, port, cert_der, dir)
    }

    fn tls_session(psk: &str, port: u16, root_der: Vec<u8>) -> CoordinatorSession {
        CoordinatorSession::spawn_with_test_root(
            SessionConfig {
                host: "127.0.0.1".to_string(),
                port,
                psk: psk.to_string(),
                tls_domain: Some("localhost".to_string()),
                plaintext_allowed: false,
            },
            root_der,
        )
    }

    /// Steady state over verified TLS: register + heartbeats + task request
    /// + result reuse one accepted TCP connection with one TLS handshake
    /// and one PSK authentication. No plaintext fallback (TLS-only
    /// listener; verified test trust, not the insecure bypass).
    #[tokio::test]
    async fn session_reuses_single_tls_connection_in_steady_state() {
        let psk = "test-psk-session-tls-reuse".to_string();
        let (listener, handle, port, root_der, _dir) = start_tls_listener(&psk).await;
        let session = tls_session(&psk, port, root_der);

        let start = Instant::now();
        session
            .register(
                "worker-tls-1".to_string(),
                "host-tls-1".to_string(),
                vec!["PortScan".to_string()],
            )
            .await
            .expect("tls register");
        for _ in 0..3 {
            session
                .heartbeat(heartbeat_status("worker-tls-1"))
                .await
                .expect("tls heartbeat");
        }
        let tasks = session
            .request_tasks("worker-tls-1".to_string(), 5)
            .await
            .expect("tls request tasks");
        assert!(tasks.is_empty());
        session
            .send_result(result_for("tls-task-1"))
            .await
            .expect("tls send result");
        let wall = start.elapsed();

        assert_eq!(listener.accepted_count(), 1, "one TCP accept");
        assert_eq!(
            listener.tls_handshake_count(),
            1,
            "one TLS handshake for the accepted connection"
        );
        assert_eq!(listener.authenticated_count(), 1, "one PSK auth");
        assert_eq!(listener.connection_count().await, 1);
        println!(
            "perf session tls_steady_state ops=6 wall_ms={} accepts={} handshakes={} auths={} live={}",
            wall.as_millis(),
            listener.accepted_count(),
            listener.tls_handshake_count(),
            listener.authenticated_count(),
            listener.connection_count().await
        );

        let workers = listener.get_workers().await;
        assert_eq!(workers.len(), 1);
        assert_eq!(workers[0].worker_id, "worker-tls-1");

        session.shutdown();
        listener.shutdown();
        handle.abort();
    }

    /// Deterministic established-connection sever/reconnect over verified
    /// TLS (Workstream 3).
    ///
    /// A purpose-built loopback coordinator accepts connection A, performs
    /// TLS + PSK auth, receives Register + one live heartbeat, then
    /// deliberately drops A from the server side. The client observes the
    /// loss on its next heartbeat, reconnects as connection B with a fresh
    /// TLS/auth sequence, replays Register before the heartbeat is handled
    /// as belonging to the worker, and the retried heartbeat succeeds. The
    /// old connection is never reused and no unauthenticated command is
    /// accepted. Ordering comes from observed messages/counters; only the
    /// reconnect pacing crosses an intentional timing window, and even
    /// there success is asserted via protocol observations with timeouts.
    #[tokio::test]
    async fn session_severed_tls_connection_reconnects_with_reregister() {
        use tokio::sync::{Mutex as AsyncMutex, Notify};

        let psk = "test-psk-session-tls-sever".to_string();
        let worker_id = "worker-sever-1".to_string();

        let (cert_der, cert_pem, key_pem) = generate_tls_material();
        let dir = tempfile::TempDir::new().expect("tempdir");
        let cert_path = dir.path().join("sever-cert.pem");
        let key_path = dir.path().join("sever-key.pem");
        std::fs::write(&cert_path, cert_pem.as_bytes()).expect("write cert");
        std::fs::write(&key_path, key_pem.as_bytes()).expect("write key");
        let tls_server = TlsServer::from_pem(&cert_path, &key_path).expect("tls server");
        let acceptor = tls_server.clone_acceptor();

        let tcp = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = tcp.local_addr().expect("addr").port();

        let accepted = Arc::new(AtomicUsize::new(0));
        let handshakes = Arc::new(AtomicUsize::new(0));
        let auths = Arc::new(AtomicUsize::new(0));
        let events: Arc<AsyncMutex<Vec<String>>> = Arc::new(AsyncMutex::new(Vec::new()));
        let peers: Arc<AsyncMutex<Vec<SocketAddr>>> = Arc::new(AsyncMutex::new(Vec::new()));
        let violations = Arc::new(AtomicUsize::new(0));

        let live_notify = Arc::new(Notify::new());
        let sever_notify = Arc::new(Notify::new());
        let severed_notify = Arc::new(Notify::new());
        let reconnected_notify = Arc::new(Notify::new());

        let server_handle = {
            let accepted = Arc::clone(&accepted);
            let handshakes = Arc::clone(&handshakes);
            let auths = Arc::clone(&auths);
            let events = Arc::clone(&events);
            let peers = Arc::clone(&peers);
            let violations = Arc::clone(&violations);
            let live_notify = Arc::clone(&live_notify);
            let sever_notify = Arc::clone(&sever_notify);
            let severed_notify = Arc::clone(&severed_notify);
            let reconnected_notify = Arc::clone(&reconnected_notify);
            let psk = psk.clone();
            let worker_id = worker_id.clone();
            tokio::spawn(async move {
                loop {
                    let Ok((stream, peer)) = tcp.accept().await else {
                        return;
                    };
                    let idx = accepted.fetch_add(1, Ordering::SeqCst);
                    peers.lock().await.push(peer);
                    let acceptor = acceptor.clone();
                    let handshakes = Arc::clone(&handshakes);
                    let auths = Arc::clone(&auths);
                    let events = Arc::clone(&events);
                    let violations = Arc::clone(&violations);
                    let live_notify = Arc::clone(&live_notify);
                    let sever_notify = Arc::clone(&sever_notify);
                    let severed_notify = Arc::clone(&severed_notify);
                    let reconnected_notify = Arc::clone(&reconnected_notify);
                    let psk = psk.clone();
                    let worker_id = worker_id.clone();
                    tokio::spawn(async move {
                        let stream = match StreamWrapper::accept_tls(&acceptor, stream).await {
                            Ok(s) => {
                                handshakes.fetch_add(1, Ordering::SeqCst);
                                s
                            }
                            Err(e) => {
                                tracing::debug!(%e, "sever fixture TLS handshake failed");
                                return;
                            }
                        };
                        let mut writer = LineWriter::new(stream);
                        let auth_line =
                            match tokio::time::timeout(Duration::from_secs(10), writer.read_line())
                                .await
                            {
                                Ok(Ok(Some(line))) => line,
                                _ => return,
                            };
                        let auth: AuthMessage = match serde_json::from_str(&auth_line) {
                            Ok(a) => a,
                            Err(_) => return,
                        };
                        if auth.psk != psk {
                            return;
                        }
                        auths.fetch_add(1, Ordering::SeqCst);
                        let welcome = ResponseMessage {
                            id: "auth".to_string(),
                            msg_type: "authenticated".to_string(),
                            success: true,
                            output: Some("Authenticated".to_string()),
                            error: None,
                            duration_ms: None,
                            hostname: Some("sever-coordinator".to_string()),
                            capabilities: None,
                        };
                        if writer
                            .write_line(&serde_json::to_string(&welcome).expect("welcome"))
                            .await
                            .is_err()
                        {
                            return;
                        }

                        // Per-connection command loop with explicit sever control.
                        let mut registered_on_conn = false;
                        if idx == 0 {
                            // Connection A: Register, one live heartbeat, then
                            // wait for the deliberate sever signal and drop.
                            let line = match tokio::time::timeout(
                                Duration::from_secs(10),
                                writer.read_line(),
                            )
                            .await
                            {
                                Ok(Ok(Some(l))) => l,
                                _ => return,
                            };
                            let cmd: CommandMessage = match serde_json::from_str(&line) {
                                Ok(c) => c,
                                Err(_) => return,
                            };
                            match cmd {
                                CommandMessage::Register { id, .. } => {
                                    assert_eq!(id, worker_id);
                                    registered_on_conn = true;
                                    events.lock().await.push("conn0 register".to_string());
                                    let resp =
                                        ResponseMessage::registration(id, "h".to_string(), vec![]);
                                    if writer
                                        .write_line(
                                            &serde_json::to_string(&resp).expect("registered"),
                                        )
                                        .await
                                        .is_err()
                                    {
                                        return;
                                    }
                                }
                                _ => {
                                    violations.fetch_add(1, Ordering::SeqCst);
                                    return;
                                }
                            }
                            let line = match tokio::time::timeout(
                                Duration::from_secs(10),
                                writer.read_line(),
                            )
                            .await
                            {
                                Ok(Ok(Some(l))) => l,
                                _ => return,
                            };
                            let cmd: CommandMessage = match serde_json::from_str(&line) {
                                Ok(c) => c,
                                Err(_) => return,
                            };
                            match cmd {
                                CommandMessage::Heartbeat { .. } => {
                                    if !registered_on_conn {
                                        violations.fetch_add(1, Ordering::SeqCst);
                                        return;
                                    }
                                    events.lock().await.push("conn0 heartbeat".to_string());
                                    let resp = ResponseMessage {
                                        id: "hb-a".to_string(),
                                        msg_type: "heartbeat_ack".to_string(),
                                        success: true,
                                        output: Some("{}".to_string()),
                                        error: None,
                                        duration_ms: None,
                                        hostname: None,
                                        capabilities: None,
                                    };
                                    if writer
                                        .write_line(&serde_json::to_string(&resp).expect("hb ack"))
                                        .await
                                        .is_err()
                                    {
                                        return;
                                    }
                                }
                                _ => {
                                    violations.fetch_add(1, Ordering::SeqCst);
                                    return;
                                }
                            }
                            live_notify.notify_one();
                            // Wait for the test to order the deliberate sever.
                            let _ = tokio::time::timeout(
                                Duration::from_secs(15),
                                sever_notify.notified(),
                            )
                            .await;
                            events.lock().await.push("conn0 severed".to_string());
                            severed_notify.notify_one();
                            // Drop closes an established, registered connection.
                        } else {
                            // Connection B: must see Register replay before
                            // any heartbeat is handled as the worker's.
                            let line = match tokio::time::timeout(
                                Duration::from_secs(15),
                                writer.read_line(),
                            )
                            .await
                            {
                                Ok(Ok(Some(l))) => l,
                                _ => {
                                    violations.fetch_add(1, Ordering::SeqCst);
                                    return;
                                }
                            };
                            let cmd: CommandMessage = match serde_json::from_str(&line) {
                                Ok(c) => c,
                                Err(_) => {
                                    violations.fetch_add(1, Ordering::SeqCst);
                                    return;
                                }
                            };
                            match cmd {
                                CommandMessage::Register { id, .. } => {
                                    if id != worker_id {
                                        violations.fetch_add(1, Ordering::SeqCst);
                                        return;
                                    }
                                    registered_on_conn = true;
                                    events.lock().await.push("conn1 register".to_string());
                                    let resp =
                                        ResponseMessage::registration(id, "h".to_string(), vec![]);
                                    if writer
                                        .write_line(
                                            &serde_json::to_string(&resp).expect("registered"),
                                        )
                                        .await
                                        .is_err()
                                    {
                                        return;
                                    }
                                }
                                _ => {
                                    // Heartbeat (or anything) before
                                    // Register on the fresh connection must
                                    // not be accepted as the worker's.
                                    violations.fetch_add(1, Ordering::SeqCst);
                                    return;
                                }
                            }
                            let line = match tokio::time::timeout(
                                Duration::from_secs(15),
                                writer.read_line(),
                            )
                            .await
                            {
                                Ok(Ok(Some(l))) => l,
                                _ => {
                                    violations.fetch_add(1, Ordering::SeqCst);
                                    return;
                                }
                            };
                            let cmd: CommandMessage = match serde_json::from_str(&line) {
                                Ok(c) => c,
                                Err(_) => {
                                    violations.fetch_add(1, Ordering::SeqCst);
                                    return;
                                }
                            };
                            match cmd {
                                CommandMessage::Heartbeat { .. } => {
                                    if !registered_on_conn {
                                        violations.fetch_add(1, Ordering::SeqCst);
                                        return;
                                    }
                                    events.lock().await.push("conn1 heartbeat".to_string());
                                    let resp = ResponseMessage {
                                        id: "hb-b".to_string(),
                                        msg_type: "heartbeat_ack".to_string(),
                                        success: true,
                                        output: Some("{}".to_string()),
                                        error: None,
                                        duration_ms: None,
                                        hostname: None,
                                        capabilities: None,
                                    };
                                    if writer
                                        .write_line(&serde_json::to_string(&resp).expect("hb ack"))
                                        .await
                                        .is_err()
                                    {
                                        return;
                                    }
                                    reconnected_notify.notify_one();
                                }
                                _ => {
                                    violations.fetch_add(1, Ordering::SeqCst);
                                    return;
                                }
                            }
                            // Keep B open briefly for clean shutdown.
                            let _ = tokio::time::timeout(Duration::from_secs(10), async {
                                loop {
                                    match writer.read_line().await {
                                        Ok(Some(_)) => {}
                                        _ => break,
                                    }
                                }
                            })
                            .await;
                        }
                    });
                }
            })
        };

        let session = tls_session(&psk, port, cert_der);
        session
            .register(worker_id.clone(), "h".to_string(), vec![])
            .await
            .expect("register on A");
        session
            .heartbeat(heartbeat_status(&worker_id))
            .await
            .expect("live heartbeat on A");

        tokio::time::timeout(Duration::from_secs(10), live_notify.notified())
            .await
            .expect("server saw live connection A");
        sever_notify.notify_one();
        tokio::time::timeout(Duration::from_secs(10), severed_notify.notified())
            .await
            .expect("server dropped connection A");

        // The next heartbeat observes the loss, reconnects as B with fresh
        // TLS/auth + Register replay, and the idempotent retry succeeds.
        tokio::time::timeout(
            Duration::from_secs(15),
            session.heartbeat(heartbeat_status(&worker_id)),
        )
        .await
        .expect("heartbeat future resolved")
        .expect("heartbeat retried after sever succeeds");
        tokio::time::timeout(Duration::from_secs(10), reconnected_notify.notified())
            .await
            .expect("server saw Register replay + heartbeat on B");

        assert_eq!(accepted.load(Ordering::SeqCst), 2, "A then B accepted");
        assert_eq!(
            handshakes.load(Ordering::SeqCst),
            2,
            "fresh TLS per connection"
        );
        assert_eq!(
            auths.load(Ordering::SeqCst),
            2,
            "fresh PSK auth per connection"
        );
        assert_eq!(
            violations.load(Ordering::SeqCst),
            0,
            "no unauthenticated command"
        );
        let peers = peers.lock().await;
        assert_eq!(peers.len(), 2);
        assert_ne!(peers[0], peers[1], "old connection not reused");
        drop(peers);
        let events = events.lock().await.clone();
        assert_eq!(
            events,
            vec![
                "conn0 register".to_string(),
                "conn0 heartbeat".to_string(),
                "conn0 severed".to_string(),
                "conn1 register".to_string(),
                "conn1 heartbeat".to_string(),
            ],
            "Register replayed on B before post-reconnect heartbeat"
        );
        println!(
            "perf session tls_sever accepts=2 handshakes=2 auths=2 events={:?}",
            events
        );

        session.shutdown();
        server_handle.abort();
    }

    /// `RequestTasks` is never transparently retried: a lost
    /// response-after-dequeue surfaces instead of replaying. A single
    /// `request_tasks()` call emits exactly one wire `RequestTasks` even
    /// when the exchange fails; the next explicit poll recovers normally.
    #[tokio::test]
    async fn session_request_tasks_never_retried_on_failure() {
        let psk = "test-psk-session-no-retry-tasks".to_string();
        let worker_id = "worker-no-retry-tasks".to_string();

        let (cert_der, cert_pem, key_pem) = generate_tls_material();
        let dir = tempfile::TempDir::new().expect("tempdir");
        let cert_path = dir.path().join("nrt-cert.pem");
        let key_path = dir.path().join("nrt-key.pem");
        std::fs::write(&cert_path, cert_pem.as_bytes()).expect("write cert");
        std::fs::write(&key_path, key_pem.as_bytes()).expect("write key");
        let tls_server = TlsServer::from_pem(&cert_path, &key_path).expect("tls server");
        let acceptor = tls_server.clone_acceptor();

        let tcp = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = tcp.local_addr().expect("addr").port();

        let accepted = Arc::new(AtomicUsize::new(0));
        let tasks_seen = Arc::new(AtomicUsize::new(0));
        let drop_first = Arc::new(AtomicUsize::new(1));

        let server_handle = {
            let accepted = Arc::clone(&accepted);
            let tasks_seen = Arc::clone(&tasks_seen);
            let drop_first = Arc::clone(&drop_first);
            let psk = psk.clone();
            tokio::spawn(async move {
                loop {
                    let Ok((stream, _)) = tcp.accept().await else {
                        return;
                    };
                    accepted.fetch_add(1, Ordering::SeqCst);
                    let acceptor = acceptor.clone();
                    let tasks_seen = Arc::clone(&tasks_seen);
                    let drop_first = Arc::clone(&drop_first);
                    let psk = psk.clone();
                    tokio::spawn(async move {
                        let stream = match StreamWrapper::accept_tls(&acceptor, stream).await {
                            Ok(s) => s,
                            Err(_) => return,
                        };
                        let mut writer = LineWriter::new(stream);
                        let auth_line =
                            match tokio::time::timeout(Duration::from_secs(10), writer.read_line())
                                .await
                            {
                                Ok(Ok(Some(l))) => l,
                                _ => return,
                            };
                        let auth: AuthMessage = match serde_json::from_str(&auth_line) {
                            Ok(a) => a,
                            Err(_) => return,
                        };
                        if auth.psk != psk {
                            return;
                        }
                        let welcome = ResponseMessage {
                            id: "auth".to_string(),
                            msg_type: "authenticated".to_string(),
                            success: true,
                            output: Some("Authenticated".to_string()),
                            error: None,
                            duration_ms: None,
                            hostname: Some("nrt".to_string()),
                            capabilities: None,
                        };
                        if writer
                            .write_line(&serde_json::to_string(&welcome).expect("welcome"))
                            .await
                            .is_err()
                        {
                            return;
                        }
                        loop {
                            let line = match tokio::time::timeout(
                                Duration::from_secs(10),
                                writer.read_line(),
                            )
                            .await
                            {
                                Ok(Ok(Some(l))) => l,
                                _ => return,
                            };
                            let cmd: CommandMessage = match serde_json::from_str(&line) {
                                Ok(c) => c,
                                Err(_) => return,
                            };
                            match cmd {
                                CommandMessage::Register { id, hostname, .. } => {
                                    let resp = ResponseMessage::registration(id, hostname, vec![]);
                                    if writer
                                        .write_line(
                                            &serde_json::to_string(&resp).expect("registered"),
                                        )
                                        .await
                                        .is_err()
                                    {
                                        return;
                                    }
                                }
                                CommandMessage::RequestTasks { id, .. } => {
                                    let n = tasks_seen.fetch_add(1, Ordering::SeqCst) + 1;
                                    if drop_first.load(Ordering::SeqCst) == 1 && n == 1 {
                                        // Lost-response-after-dequeue shape:
                                        // drop without responding.
                                        return;
                                    }
                                    let resp = ResponseMessage {
                                        id,
                                        msg_type: "tasks_assigned".to_string(),
                                        success: true,
                                        output: Some("[]".to_string()),
                                        error: None,
                                        duration_ms: None,
                                        hostname: None,
                                        capabilities: None,
                                    };
                                    if writer
                                        .write_line(&serde_json::to_string(&resp).expect("tasks"))
                                        .await
                                        .is_err()
                                    {
                                        return;
                                    }
                                }
                                _ => {}
                            }
                        }
                    });
                }
            })
        };

        let session = tls_session(&psk, port, cert_der);
        session
            .register(worker_id.clone(), "h".to_string(), vec![])
            .await
            .expect("register");

        let first = session.request_tasks(worker_id.clone(), 2).await;
        assert!(
            first.is_err(),
            "failed RequestTasks must surface, not retry"
        );

        // Allow any spurious transparent retry to appear; the count must
        // stay at exactly one wire message for one caller call.
        tokio::time::sleep(Duration::from_millis(600)).await;
        assert_eq!(
            tasks_seen.load(Ordering::SeqCst),
            1,
            "one caller call emits one wire RequestTasks"
        );

        // The next explicit poll recovers via a fresh setup.
        let second = tokio::time::timeout(
            Duration::from_secs(15),
            session.request_tasks(worker_id.clone(), 2),
        )
        .await
        .expect("second poll resolved")
        .expect("second poll recovers");
        assert!(second.is_empty());
        assert_eq!(
            tasks_seen.load(Ordering::SeqCst),
            2,
            "second explicit poll emits the second wire message"
        );
        assert_eq!(accepted.load(Ordering::SeqCst), 2);

        session.shutdown();
        server_handle.abort();
    }

    /// Result submission is never transparently retried: replays could
    /// duplicate completed entries. One `send_result()` emits one wire
    /// `Result`; the next explicit submission recovers.
    #[tokio::test]
    async fn session_result_never_retried_on_failure() {
        let psk = "test-psk-session-no-retry-result".to_string();
        let worker_id = "worker-no-retry-result".to_string();

        let (cert_der, cert_pem, key_pem) = generate_tls_material();
        let dir = tempfile::TempDir::new().expect("tempdir");
        let cert_path = dir.path().join("nrr-cert.pem");
        let key_path = dir.path().join("nrr-key.pem");
        std::fs::write(&cert_path, cert_pem.as_bytes()).expect("write cert");
        std::fs::write(&key_path, key_pem.as_bytes()).expect("write key");
        let tls_server = TlsServer::from_pem(&cert_path, &key_path).expect("tls server");
        let acceptor = tls_server.clone_acceptor();

        let tcp = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = tcp.local_addr().expect("addr").port();

        let accepted = Arc::new(AtomicUsize::new(0));
        let results_seen = Arc::new(AtomicUsize::new(0));

        let server_handle = {
            let accepted = Arc::clone(&accepted);
            let results_seen = Arc::clone(&results_seen);
            let psk = psk.clone();
            tokio::spawn(async move {
                loop {
                    let Ok((stream, _)) = tcp.accept().await else {
                        return;
                    };
                    accepted.fetch_add(1, Ordering::SeqCst);
                    let acceptor = acceptor.clone();
                    let results_seen = Arc::clone(&results_seen);
                    let psk = psk.clone();
                    tokio::spawn(async move {
                        let stream = match StreamWrapper::accept_tls(&acceptor, stream).await {
                            Ok(s) => s,
                            Err(_) => return,
                        };
                        let mut writer = LineWriter::new(stream);
                        let auth_line =
                            match tokio::time::timeout(Duration::from_secs(10), writer.read_line())
                                .await
                            {
                                Ok(Ok(Some(l))) => l,
                                _ => return,
                            };
                        let auth: AuthMessage = match serde_json::from_str(&auth_line) {
                            Ok(a) => a,
                            Err(_) => return,
                        };
                        if auth.psk != psk {
                            return;
                        }
                        let welcome = ResponseMessage {
                            id: "auth".to_string(),
                            msg_type: "authenticated".to_string(),
                            success: true,
                            output: Some("Authenticated".to_string()),
                            error: None,
                            duration_ms: None,
                            hostname: Some("nrr".to_string()),
                            capabilities: None,
                        };
                        if writer
                            .write_line(&serde_json::to_string(&welcome).expect("welcome"))
                            .await
                            .is_err()
                        {
                            return;
                        }
                        loop {
                            let line = match tokio::time::timeout(
                                Duration::from_secs(10),
                                writer.read_line(),
                            )
                            .await
                            {
                                Ok(Ok(Some(l))) => l,
                                _ => return,
                            };
                            let cmd: CommandMessage = match serde_json::from_str(&line) {
                                Ok(c) => c,
                                Err(_) => return,
                            };
                            match cmd {
                                CommandMessage::Register { id, hostname, .. } => {
                                    let resp = ResponseMessage::registration(id, hostname, vec![]);
                                    if writer
                                        .write_line(
                                            &serde_json::to_string(&resp).expect("registered"),
                                        )
                                        .await
                                        .is_err()
                                    {
                                        return;
                                    }
                                }
                                CommandMessage::Result { id, .. } => {
                                    let n = results_seen.fetch_add(1, Ordering::SeqCst) + 1;
                                    if n == 1 {
                                        // Drop before ack: replay would
                                        // duplicate the completed entry.
                                        return;
                                    }
                                    let resp = ResponseMessage {
                                        id,
                                        msg_type: "result_ack".to_string(),
                                        success: true,
                                        output: Some("Result received".to_string()),
                                        error: None,
                                        duration_ms: None,
                                        hostname: None,
                                        capabilities: None,
                                    };
                                    if writer
                                        .write_line(&serde_json::to_string(&resp).expect("ack"))
                                        .await
                                        .is_err()
                                    {
                                        return;
                                    }
                                }
                                _ => {}
                            }
                        }
                    });
                }
            })
        };

        let session = tls_session(&psk, port, cert_der);
        session
            .register(worker_id.clone(), "h".to_string(), vec![])
            .await
            .expect("register");

        let first = session.send_result(result_for("nrr-1")).await;
        assert!(first.is_err(), "failed result must surface, not retry");

        tokio::time::sleep(Duration::from_millis(600)).await;
        assert_eq!(
            results_seen.load(Ordering::SeqCst),
            1,
            "one caller call emits one wire Result"
        );

        tokio::time::timeout(
            Duration::from_secs(15),
            session.send_result(result_for("nrr-2")),
        )
        .await
        .expect("second submit resolved")
        .expect("second explicit submit recovers");
        assert_eq!(
            results_seen.load(Ordering::SeqCst),
            2,
            "second explicit submit emits the second wire message"
        );
        assert_eq!(accepted.load(Ordering::SeqCst), 2);

        session.shutdown();
        server_handle.abort();
    }
}
