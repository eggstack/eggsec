
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};

use crate::distributed::command::{CommandExecutor, CommandMessage, ResponseMessage};
use crate::distributed::io::{LineWriter, StreamWrapper, TlsClient, TlsServer};

const MAX_CONNECTIONS: usize = 100;
const RATE_LIMIT_PER_MINUTE: u32 = 60;
const RATE_LIMIT_WINDOW_SECS: u64 = 60;

#[derive(Clone)]
pub struct TlsConfig {
    pub pkcs12_path: PathBuf,
    pub password: String,
}

pub struct RemoteListener {
    psk: String,
    shutdown_tx: broadcast::Sender<()>,
    connections: Arc<RwLock<Vec<String>>>,
    rate_limits: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
    max_connections: usize,
    rate_limit: u32,
    ip_allowlist: Option<Vec<String>>,
    tls_server: Option<Arc<TlsServer>>,
}

impl RemoteListener {
    pub fn new(psk: String) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            psk,
            shutdown_tx,
            connections: Arc::new(RwLock::new(Vec::new())),
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
            max_connections: MAX_CONNECTIONS,
            rate_limit: RATE_LIMIT_PER_MINUTE,
            ip_allowlist: None,
            tls_server: None,
        }
    }

    pub fn with_config(psk: String, max_connections: usize, rate_limit: u32) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            psk,
            shutdown_tx,
            connections: Arc::new(RwLock::new(Vec::new())),
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
            max_connections,
            rate_limit,
            ip_allowlist: None,
            tls_server: None,
        }
    }

    pub fn with_allowlist(psk: String, allowlist: Vec<String>) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            psk,
            shutdown_tx,
            connections: Arc::new(RwLock::new(Vec::new())),
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
            max_connections: MAX_CONNECTIONS,
            rate_limit: RATE_LIMIT_PER_MINUTE,
            ip_allowlist: Some(allowlist),
            tls_server: None,
        }
    }

    pub fn with_tls(psk: String, tls_config: TlsConfig) -> anyhow::Result<Self> {
        let tls_server = TlsServer::from_pkcs12(&tls_config.pkcs12_path, &tls_config.password)
            .map_err(|e| anyhow::anyhow!("Failed to initialize TLS: {}", e))?;

        let (shutdown_tx, _) = broadcast::channel(1);
        Ok(Self {
            psk,
            shutdown_tx,
            connections: Arc::new(RwLock::new(Vec::new())),
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
            max_connections: MAX_CONNECTIONS,
            rate_limit: RATE_LIMIT_PER_MINUTE,
            ip_allowlist: None,
            tls_server: Some(Arc::new(tls_server)),
        })
    }

    pub fn new_plaintext(psk: String) -> Self {
        Self::new(psk)
    }

    pub fn is_tls(&self) -> bool {
        self.tls_server.is_some()
    }

    fn get_capabilities() -> Vec<String> {
        vec![
            "scan-ports".to_string(),
            "scan-endpoints".to_string(),
            "fuzz".to_string(),
            "load".to_string(),
            "recon".to_string(),
            "graphql".to_string(),
            "oauth".to_string(),
            "waf".to_string(),
            "waf-stress".to_string(),
            "fingerprint".to_string(),
            "packet".to_string(),
            "traceroute".to_string(),
            "icmp".to_string(),
        ]
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }

    async fn check_rate_limit(
        rate_limits: &Arc<RwLock<HashMap<String, Vec<Instant>>>>,
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

    pub async fn start(&self, port: u16) -> anyhow::Result<()> {
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
                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection(stream, addr, psk, connections, tls_acceptor).await {
                                    tracing::error!("Connection error: {}", e);
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
        tls_acceptor: Option<tokio_native_tls::TlsAcceptor>,
    ) -> anyhow::Result<()> {
        tracing::info!("Connection from {}", addr);

        let stream = match tls_acceptor {
            Some(acceptor) => match StreamWrapper::accept_tls(&acceptor, stream).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("TLS handshake failed: {}", e);
                    return Err(anyhow::anyhow!("TLS handshake failed: {}", e));
                }
            },
            None => StreamWrapper::plain(stream),
        };

        let mut line_writer = LineWriter::new(stream);

        // Read auth message
        let auth_line = line_writer.read_line().await?;
        let auth: AuthMessage =
            serde_json::from_str(&auth_line.ok_or_else(|| anyhow::anyhow!("No auth"))?)?;

        if !bool::from(auth.psk.as_bytes().ct_eq(psk.as_bytes())) {
            let error = ResponseMessage::error("auth".to_string(), "Invalid PSK".to_string(), None);
            line_writer
                .write_line(&serde_json::to_string(&error)?)
                .await?;
            return Err(anyhow::anyhow!("Invalid PSK from {}", addr));
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
            hostname: Some(hostname::get()?.to_string_lossy().to_string()),
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
                    let response = ResponseMessage::registration(id, hostname, capabilities);
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
}

impl RemoteClient {
    pub fn new(psk: String) -> Self {
        Self { psk, tls: None }
    }

    pub fn with_tls(psk: String, domain: &str) -> anyhow::Result<Self> {
        let tls = TlsClient::new(domain)
            .map_err(|e| anyhow::anyhow!("Failed to initialize TLS client: {}", e))?;
        Ok(Self { psk, tls: Some(tls) })
    }

    pub fn new_plaintext(psk: String) -> Self {
        Self::new(psk)
    }

    pub fn is_tls(&self) -> bool {
        self.tls.is_some()
    }

    pub async fn execute(
        &self,
        host: &str,
        port: u16,
        command: Vec<String>,
        timeout_secs: Option<u64>,
    ) -> anyhow::Result<crate::distributed::command::RemoteResult> {
        let host_port = format!("{}:{}", host, port);
        let addr = tokio::net::lookup_host(&host_port)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to resolve host: {}", e))?
            .next()
            .ok_or_else(|| anyhow::anyhow!("No addresses found for host"))?;

        let connect_timeout = std::time::Duration::from_secs(5);
        let stream = tokio::time::timeout(connect_timeout, TcpStream::connect(addr))
            .await
            .map_err(|_| anyhow::anyhow!("Connection timed out after 5 seconds"))?
            .map_err(|e| anyhow::anyhow!("Failed to connect: {}", e))?;

        let stream = match &self.tls {
            Some(tls_client) => {
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
                        return Err(anyhow::anyhow!("TLS handshake failed: {}", e));
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
                    .ok_or_else(|| anyhow::anyhow!("No response"))?;
                Ok::<_, anyhow::Error>(serde_json::from_str::<ResponseMessage>(&line)?)
            })
            .await
            .map_err(|_| anyhow::anyhow!("Authentication response timed out"))??;

        if !auth_response.success {
            return Err(anyhow::anyhow!(
                "Authentication failed: {:?}",
                auth_response.error
            ));
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
                .ok_or_else(|| anyhow::anyhow!("No response"))?;
            Ok::<_, anyhow::Error>(serde_json::from_str::<ResponseMessage>(&response_line)?)
        })
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "Response timed out after {} seconds",
                response_timeout.as_secs()
            )
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
