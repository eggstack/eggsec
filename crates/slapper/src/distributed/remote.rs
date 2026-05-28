use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};

use crate::distributed::command::{CommandExecutor, CommandMessage, ResponseMessage};
use crate::distributed::io::{LineWriter, StreamWrapper, TlsClient, TlsServer};
use crate::distributed::{queue::TaskQueue, CAPABILITIES};
use crate::error::{Result, SlapperError};
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
    connections: Arc<RwLock<Vec<String>>>,
    rate_limits: Arc<RwLock<FxHashMap<String, Vec<Instant>>>>,
    max_connections: usize,
    rate_limit: u32,
    ip_allowlist: Option<Vec<String>>,
    tls_server: Option<Arc<TlsServer>>,
    task_queue: Arc<TaskQueue>,
}

impl RemoteListener {
    pub fn new(psk: String) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            psk,
            shutdown_tx,
            connections: Arc::new(RwLock::new(Vec::new())),
            rate_limits: Arc::new(RwLock::new(FxHashMap::default())),
            max_connections: MAX_CONNECTIONS,
            rate_limit: RATE_LIMIT_PER_MINUTE,
            ip_allowlist: None,
            tls_server: None,
            task_queue: Arc::new(TaskQueue::new(10000)),
        }
    }

    pub fn with_config(psk: String, max_connections: usize, rate_limit: u32) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            psk,
            shutdown_tx,
            connections: Arc::new(RwLock::new(Vec::new())),
            rate_limits: Arc::new(RwLock::new(FxHashMap::default())),
            max_connections,
            rate_limit,
            ip_allowlist: None,
            tls_server: None,
            task_queue: Arc::new(TaskQueue::new(10000)),
        }
    }

    pub fn with_allowlist(psk: String, allowlist: Vec<String>) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            psk,
            shutdown_tx,
            connections: Arc::new(RwLock::new(Vec::new())),
            rate_limits: Arc::new(RwLock::new(FxHashMap::default())),
            max_connections: MAX_CONNECTIONS,
            rate_limit: RATE_LIMIT_PER_MINUTE,
            ip_allowlist: Some(allowlist),
            tls_server: None,
            task_queue: Arc::new(TaskQueue::new(10000)),
        }
    }

    pub fn with_tls(psk: String, tls_config: TlsConfig) -> Result<Self> {
        let tls_server = TlsServer::from_pem(&tls_config.cert_path, &tls_config.key_path)
            .map_err(|e| SlapperError::Network(format!("Failed to initialize TLS: {}", e)))?;

        let (shutdown_tx, _) = broadcast::channel(1);
        Ok(Self {
            psk,
            shutdown_tx,
            connections: Arc::new(RwLock::new(Vec::new())),
            rate_limits: Arc::new(RwLock::new(FxHashMap::default())),
            max_connections: MAX_CONNECTIONS,
            rate_limit: RATE_LIMIT_PER_MINUTE,
            ip_allowlist: None,
            tls_server: Some(Arc::new(tls_server)),
            task_queue: Arc::new(TaskQueue::new(10000)),
        })
    }

    pub fn new_plaintext(psk: String) -> Self {
        Self::new(psk)
    }

    pub fn is_tls(&self) -> bool {
        self.tls_server.is_some()
    }

    fn get_capabilities() -> Vec<String> {
        CAPABILITIES.iter().map(|s| s.to_string()).collect()
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
            let mut cleanup_interval = tokio::time::interval(std::time::Duration::from_secs(RATE_LIMIT_WINDOW_SECS));
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

                            let psk = self.psk.clone();
                            let connections = Arc::clone(&self.connections);
                            let tls_acceptor = tls_acceptor.clone();
                            let task_queue = Arc::clone(&self.task_queue);
                            let handle = tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection(stream, addr, psk, connections, tls_acceptor, task_queue).await {
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
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_connection(
        stream: TcpStream,
        addr: SocketAddr,
        psk: String,
        connections: Arc<RwLock<Vec<String>>>,
        tls_acceptor: Option<tokio_rustls::TlsAcceptor>,
        task_queue: Arc<TaskQueue>,
    ) -> Result<()> {
        tracing::info!("Connection from {}", addr);

        let stream = match tls_acceptor {
            Some(acceptor) => match StreamWrapper::accept_tls(&acceptor, stream).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(addr = %addr, "TLS handshake failed: {}", e);
                    return Err(SlapperError::Network(format!(
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
            &auth_line.ok_or_else(|| SlapperError::Validation("No auth".to_string()))?,
        )?;

        if !bool::from(auth.psk.as_bytes().ct_eq(psk.as_bytes())) {
            let error = ResponseMessage::error("auth".to_string(), "Invalid PSK".to_string(), None);
            line_writer
                .write_line(&serde_json::to_string(&error)?)
                .await?;
            return Err(SlapperError::Validation(format!(
                "Invalid PSK from {}",
                addr
            )));
        }

        // Register connection
        connections.write().await.push(addr.to_string());
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
                    .map_err(|e| SlapperError::Runtime(format!("Failed to get hostname: {}", e)))?
                    .to_string_lossy()
                    .to_string(),
            ),
            capabilities: Some(Self::get_capabilities()),
        };
        line_writer
            .write_line(&serde_json::to_string(&welcome)?)
            .await?;

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
                        .into_iter()
                        .filter(|cap| CAPABILITIES.contains(&cap.as_str()))
                        .collect();
                    if valid_caps.len() != claimed_count {
                        tracing::warn!(
                            worker = %hostname,
                            "Worker advertised capabilities not in CAPABILITIES list; filtering"
                        );
                    }
                    let response = ResponseMessage::registration(id, hostname, valid_caps);
                    line_writer
                        .write_line(&serde_json::to_string(&response)?)
                        .await?;
                }
                CommandMessage::Heartbeat { id, status } => {
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
                    let tasks_json = serde_json::to_string(&tasks)
                        .unwrap_or_else(|_| "[]".to_string());
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
            }
        }

        // Cleanup
        connections.write().await.retain(|c| c != &addr_str);
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
        }
    }

    pub fn with_tls(psk: String, domain: &str) -> Result<Self> {
        let tls = TlsClient::new(domain).map_err(|e| {
            SlapperError::Network(format!("Failed to initialize TLS client: {}", e))
        })?;
        Ok(Self {
            psk,
            tls: Some(tls),
            cached_addr: None,
        })
    }

    pub fn new_plaintext(psk: String) -> Self {
        Self::new(psk)
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
                .map_err(|e| SlapperError::Network(format!("Failed to resolve host: {}", e)))?
                .next()
                .ok_or_else(|| SlapperError::Network("No addresses found for host".to_string()))?;
            self.cache_resolution(resolved);
            resolved
        };

        self.connect_to_coordinator_with_addr(&addr).await
    }

    async fn connect_to_coordinator_with_addr(&mut self, addr: &SocketAddr) -> Result<LineWriter> {
        let connect_timeout = std::time::Duration::from_secs(5);
        let stream = connect_with_nodelay_timeout(addr, connect_timeout)
            .await
            .map_err(|e| SlapperError::Network(format!("Failed to connect: {}", e)))?;

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
                        return Err(SlapperError::Network(format!(
                            "TLS handshake failed: {}",
                            e
                        )));
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
                    .ok_or_else(|| SlapperError::Network("No response".to_string()))?;
                Ok::<_, SlapperError>(serde_json::from_str::<ResponseMessage>(&line)?)
            })
            .await
            .map_err(|_| {
                SlapperError::Network("Authentication response timed out".to_string())
            })??;

        if !auth_response.success {
            return Err(SlapperError::Validation(format!(
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
            id: uuid::Uuid::new_v4().to_string(),
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
                    .ok_or_else(|| SlapperError::Network("No response".to_string()))?;
                Ok::<_, SlapperError>(serde_json::from_str::<ResponseMessage>(&line)?)
            })
            .await
            .map_err(|_| SlapperError::Network("Registration response timed out".to_string()))??;

        if !response.success {
            return Err(SlapperError::Validation(format!(
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
                .map_err(|e| SlapperError::Network(format!("Failed to resolve host: {}", e)))?
                .next()
                .ok_or_else(|| SlapperError::Network("No addresses found for host".to_string()))?;
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
                    .ok_or_else(|| SlapperError::Network("No response".to_string()))?;
                Ok::<_, SlapperError>(serde_json::from_str::<ResponseMessage>(&line)?)
            })
            .await
            .map_err(|_| SlapperError::Network("Heartbeat response timed out".to_string()))??;

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
                .map_err(|e| SlapperError::Network(format!("Failed to resolve host: {}", e)))?
                .next()
                .ok_or_else(|| SlapperError::Network("No addresses found for host".to_string()))?;
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
                    .ok_or_else(|| SlapperError::Network("No response".to_string()))?;
                Ok::<_, SlapperError>(serde_json::from_str::<ResponseMessage>(&line)?)
            })
            .await
            .map_err(|_| SlapperError::Network("Result response timed out".to_string()))??;

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
                .map_err(|e| SlapperError::Network(format!("Failed to resolve host: {}", e)))?
                .next()
                .ok_or_else(|| SlapperError::Network("No addresses found for host".to_string()))?;
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
                    .ok_or_else(|| SlapperError::Network("No response".to_string()))?;
                Ok::<_, SlapperError>(serde_json::from_str::<ResponseMessage>(&line)?)
            })
            .await
            .map_err(|_| SlapperError::Network("Task request response timed out".to_string()))??;

        if !response.success {
            return Ok(Vec::new());
        }

        let tasks: Vec<crate::distributed::queue::Task> = response
            .output
            .and_then(|o| serde_json::from_str(&o).ok())
            .unwrap_or_default();

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
                .map_err(|e| SlapperError::Network(format!("Failed to resolve host: {}", e)))?
                .next()
                .ok_or_else(|| SlapperError::Network("No addresses found for host".to_string()))?;
            self.cache_resolution(resolved);
            resolved
        };

        let connect_timeout = std::time::Duration::from_secs(5);
        let stream = connect_with_nodelay_timeout(&addr, connect_timeout)
            .await
            .map_err(|e| SlapperError::Network(format!("Failed to connect: {}", e)))?;

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
                        return Err(SlapperError::Network(format!(
                            "TLS handshake failed: {}",
                            e
                        )));
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
                    .ok_or_else(|| SlapperError::Network("No response".to_string()))?;
                Ok::<_, SlapperError>(serde_json::from_str::<ResponseMessage>(&line)?)
            })
            .await
            .map_err(|_| {
                SlapperError::Network("Authentication response timed out".to_string())
            })??;

        if !auth_response.success {
            return Err(SlapperError::Validation(format!(
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
                .ok_or_else(|| SlapperError::Network("No response".to_string()))?;
            Ok::<_, SlapperError>(serde_json::from_str::<ResponseMessage>(&response_line)?)
        })
        .await
        .map_err(|_| {
            SlapperError::Network(format!(
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
}
