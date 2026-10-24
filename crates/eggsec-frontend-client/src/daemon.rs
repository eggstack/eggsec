use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use eggsec_runtime::{
    request::RuntimeSurface, session::SessionScope, RunRequest, RuntimeCapabilities, SessionId,
    SessionSnapshot, SessionSummary, TaskId,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::{oneshot, Mutex};

use eggsec_daemon_protocol::operations::DescribeOperationsResponse;
use eggsec_daemon_protocol::protocol::{
    ClientCommand, DaemonCapabilities, ErrorCode, ServerMessage,
};

use crate::{ClientIdentity, RuntimeClient, RuntimeClientFuture, RuntimeEventReceiverHandle};

/// The daemon's health advertisement, including the protocol version it
/// speaks.
///
/// A frontend uses the protocol version to refuse to operate against a daemon
/// it cannot talk to, rather than failing later with an opaque I/O error.
#[derive(Debug, Clone)]
pub struct DaemonHealth {
    /// The daemon's status string.
    pub status: String,
    /// The daemon's version string.
    pub version: String,
    /// The daemon protocol version the daemon speaks.
    pub protocol_version: u32,
}

/// Shared channel registry for routing RuntimeEvent messages from the reader
/// task to per-subscriber receivers. Each `subscribe_events()` call inserts
/// a sender; the reader iterates all active senders to fan-out events.
type EventChannels =
    Arc<tokio::sync::RwLock<Vec<tokio::sync::mpsc::UnboundedSender<eggsec_runtime::RuntimeEvent>>>>;

#[derive(Debug, thiserror::Error)]
pub enum DaemonClientError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Connection closed")]
    ConnectionClosed,
}

struct DaemonConnection {
    _socket_path: String,
    request_counter: AtomicU64,
    /// The caller's identity: the client kind and label the daemon records,
    /// and the namespace for this client's request ids.
    identity: ClientIdentity,
    writer: Mutex<tokio::net::unix::OwnedWriteHalf>,
    pending_responses: std::sync::Arc<
        std::sync::Mutex<rustc_hash::FxHashMap<String, oneshot::Sender<ServerMessage>>>,
    >,
    /// Active event subscriber senders. The reader task fans out
    /// RuntimeEvent messages to all registered channels.
    event_channels: Arc<
        tokio::sync::RwLock<Vec<tokio::sync::mpsc::UnboundedSender<eggsec_runtime::RuntimeEvent>>>,
    >,
}

/// Runtime client that connects to a local daemon via Unix socket.
#[derive(Clone)]
pub struct DaemonRuntimeClient {
    inner: Arc<DaemonConnection>,
}

impl DaemonRuntimeClient {
    /// Connect to a daemon at the given socket path.
    ///
    /// The caller supplies its own identity: the client kind recorded in the
    /// daemon's registry, the label the daemon shows, and the namespace for
    /// this client's request ids. The crate is frontend-neutral, so it cannot
    /// choose any of those on the caller's behalf.
    pub async fn connect(
        socket_path: &str,
        identity: ClientIdentity,
    ) -> Result<Self, DaemonClientError> {
        let stream = UnixStream::connect(socket_path).await?;
        let (read_half, write_half) = stream.into_split();

        let pending_responses: std::sync::Arc<
            std::sync::Mutex<rustc_hash::FxHashMap<String, oneshot::Sender<ServerMessage>>>,
        > = std::sync::Arc::new(std::sync::Mutex::new(rustc_hash::FxHashMap::default()));

        let pending_clone = pending_responses.clone();
        let event_channels: EventChannels = Arc::new(tokio::sync::RwLock::new(Vec::new()));
        let channels_clone = event_channels.clone();

        // Spawn reader task to route responses and events
        tokio::spawn(async move {
            let mut reader = BufReader::new(read_half).lines();
            loop {
                match reader.next_line().await {
                    Ok(Some(line)) => {
                        if line.is_empty() {
                            continue;
                        }
                        let msg: ServerMessage = match serde_json::from_str(&line) {
                            Ok(msg) => msg,
                            Err(e) => {
                                tracing::warn!("Failed to parse daemon message: {}", e);
                                continue;
                            }
                        };
                        match msg {
                            ServerMessage::RuntimeEvent { event, .. } => {
                                // Fan-out RuntimeEvent to all active subscriber channels.
                                let channels = channels_clone.read().await;
                                let mut stale = Vec::new();
                                for (i, tx) in channels.iter().enumerate() {
                                    if tx.send(event.clone()).is_err() {
                                        stale.push(i);
                                    }
                                }
                                drop(channels);
                                // Clean up disconnected senders (receiver dropped).
                                if !stale.is_empty() {
                                    let mut channels = channels_clone.write().await;
                                    // Remove in reverse order to preserve indices.
                                    for &i in stale.iter().rev() {
                                        channels.remove(i);
                                    }
                                }
                            }
                            other => {
                                let request_id = match &other {
                                    ServerMessage::Ok { request_id }
                                    | ServerMessage::Error { request_id, .. }
                                    | ServerMessage::SessionCreated { request_id, .. }
                                    | ServerMessage::Sessions { request_id, .. }
                                    | ServerMessage::Snapshot { request_id, .. }
                                    | ServerMessage::TaskSubmitted { request_id, .. }
                                    | ServerMessage::Capabilities { request_id, .. }
                                    | ServerMessage::Health { request_id, .. }
                                    | ServerMessage::ClientDeclared { request_id, .. }
                                    | ServerMessage::SessionClosed { request_id, .. }
                                    | ServerMessage::PersistedSessions { request_id, .. }
                                    | ServerMessage::PersistedSnapshot { request_id, .. }
                                    | ServerMessage::Operations { request_id, .. }
                                    | ServerMessage::TaskResult { request_id, .. }
                                    | ServerMessage::TaskReport { request_id, .. } => {
                                        request_id.clone()
                                    }
                                    ServerMessage::RuntimeEvent { .. } => {
                                        tracing::warn!(
                                            "Unexpected runtime event in pending response channel"
                                        );
                                        continue;
                                    }
                                };
                                let mut pending = pending_clone.lock().unwrap_or_else(|poisoned| {
                                    tracing::warn!("daemon pending request registry mutex was poisoned; recovering state");
                                    poisoned.into_inner()
                                });
                                if let Some(sender) = pending.remove(&request_id) {
                                    if let Err(error) = sender.send(other) {
                                        tracing::debug!(
                                            ?error,
                                            "Pending daemon response receiver dropped"
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::info!("Daemon connection closed");
                        break;
                    }
                    Err(e) => {
                        tracing::warn!("Daemon read error: {}", e);
                        break;
                    }
                }
            }
        });

        let client = Self {
            inner: Arc::new(DaemonConnection {
                _socket_path: socket_path.to_string(),
                request_counter: AtomicU64::new(0),
                identity: identity.clone(),
                writer: Mutex::new(write_half),
                pending_responses,
                event_channels,
            }),
        };

        // Declare client kind to the daemon; a rejection leaves this client
        // unregistered, so later session-scoped commands would fail opaquely.
        if let Err(e) = client
            .declare_client(identity.kind.clone(), Some(identity.label.clone()))
            .await
        {
            tracing::warn!("Daemon rejected client declaration: {}", e);
        }

        Ok(client)
    }

    /// Declare this client's kind to the daemon.
    pub async fn declare_client(
        &self,
        kind: eggsec_daemon_protocol::client_registry::ClientKind,
        label: Option<String>,
    ) -> Result<ServerMessage, DaemonClientError> {
        let cmd = ClientCommand::DeclareClient {
            request_id: self.next_request_id(),
            kind,
            label,
        };
        self.send_command(cmd).await
    }

    fn next_request_id(&self) -> String {
        let id = self.inner.request_counter.fetch_add(1, Ordering::Relaxed);
        self.inner.identity.request_id(id)
    }

    /// Fetch the daemon's operation catalog and the derived request schema.
    ///
    /// This is an inherent method rather than a `RuntimeClient` trait method:
    /// the catalog is daemon-published state, and the embedded client has no
    /// catalog to publish. Daemon-only frontends call it directly.
    pub async fn describe_operations(
        &self,
    ) -> Result<DescribeOperationsResponse, DaemonClientError> {
        let cmd = ClientCommand::DescribeOperations {
            request_id: self.next_request_id(),
        };
        match self.send_command(cmd).await {
            Ok(ServerMessage::Operations { operations, .. }) => Ok(operations),
            Ok(ServerMessage::Error { message, .. }) => Err(DaemonClientError::Protocol(message)),
            Ok(other) => Err(DaemonClientError::Protocol(format!(
                "unexpected response: {other:?}"
            ))),
            Err(e) => Err(e),
        }
    }

    /// Fetch the report document for one completed task.
    ///
    /// `Ok(None)` means "no report is available": either the daemon has no
    /// report for that task, or the daemon is too old to know the command.
    /// The client MUST NOT treat either case as a failure — a client with
    /// no report available still works, it simply cannot export this task.
    ///
    /// Any other error is a genuine failure and is returned as one.
    pub async fn get_task_report(
        &self,
        session_id: &SessionId,
        task_id: &TaskId,
    ) -> Result<Option<serde_json::Value>, DaemonClientError> {
        let cmd = ClientCommand::GetTaskReport {
            request_id: self.next_request_id(),
            session_id: *session_id,
            task_id: *task_id,
        };
        match self.send_command(cmd).await {
            Ok(ServerMessage::TaskReport { report, .. }) => Ok(Some(report)),
            Ok(ServerMessage::Error {
                code: ErrorCode::UnsupportedCommand,
                ..
            })
            | Ok(ServerMessage::Error {
                code: ErrorCode::TaskNotFound,
                ..
            }) => Ok(None),
            Ok(ServerMessage::Error { message, .. }) => Err(DaemonClientError::Protocol(message)),
            Ok(other) => Err(DaemonClientError::Protocol(format!(
                "unexpected response: {other:?}"
            ))),
            Err(e) => Err(e),
        }
    }

    /// Query the daemon's health, including the protocol version it speaks.
    ///
    /// Inherent rather than a trait method for the same reason as
    /// [`Self::describe_operations`]: only a daemon-backed client has a
    /// daemon to ask.
    pub async fn health(&self) -> Result<DaemonHealth, DaemonClientError> {
        let cmd = ClientCommand::Health {
            request_id: self.next_request_id(),
        };
        match self.send_command(cmd).await {
            Ok(ServerMessage::Health {
                status,
                version,
                protocol_version,
                ..
            }) => Ok(DaemonHealth {
                status,
                version,
                protocol_version,
            }),
            Ok(ServerMessage::Error { message, .. }) => Err(DaemonClientError::Protocol(message)),
            Ok(other) => Err(DaemonClientError::Protocol(format!(
                "unexpected response: {other:?}"
            ))),
            Err(e) => Err(e),
        }
    }

    /// The daemon's full capability advertisement.
    ///
    /// The `RuntimeClient` trait's `capabilities` returns the *runtime* half
    /// of the advertisement; a daemon-only frontend also needs the daemon's
    /// own flags — notably `operation_schema`, which says whether this daemon
    /// can publish the operation catalog at all.
    pub async fn daemon_capabilities(&self) -> Result<DaemonCapabilities, DaemonClientError> {
        let cmd = ClientCommand::Capabilities {
            request_id: self.next_request_id(),
        };
        match self.send_command(cmd).await {
            Ok(ServerMessage::Capabilities { capabilities, .. }) => Ok(capabilities),
            Ok(ServerMessage::Error { message, .. }) => Err(DaemonClientError::Protocol(message)),
            Ok(other) => Err(DaemonClientError::Protocol(format!(
                "unexpected response: {other:?}"
            ))),
            Err(e) => Err(e),
        }
    }

    async fn send_command(&self, cmd: ClientCommand) -> Result<ServerMessage, DaemonClientError> {
        let request_id = match &cmd {
            ClientCommand::Health { request_id }
            | ClientCommand::Capabilities { request_id }
            | ClientCommand::CreateSession { request_id, .. }
            | ClientCommand::ListSessions { request_id }
            | ClientCommand::GetSnapshot { request_id, .. }
            | ClientCommand::GetTaskResult { request_id, .. }
            | ClientCommand::SubmitTask { request_id, .. }
            | ClientCommand::CancelTask { request_id, .. }
            | ClientCommand::CancelActive { request_id, .. }
            | ClientCommand::DeclareClient { request_id, .. }
            | ClientCommand::CloseSession { request_id, .. }
            | ClientCommand::Subscribe { request_id, .. }
            | ClientCommand::ApprovePolicy { request_id, .. }
            | ClientCommand::ListPersistedSessions { request_id }
            | ClientCommand::DescribeOperations { request_id }
            | ClientCommand::GetPersistedSnapshot { request_id, .. }
            | ClientCommand::GetTaskReport { request_id, .. } => request_id.clone(),
        };

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self
                .inner
                .pending_responses
                .lock()
                .unwrap_or_else(|poisoned| {
                    tracing::warn!("daemon pending_responses mutex was poisoned; recovering state");
                    poisoned.into_inner()
                });
            pending.insert(request_id, tx);
        }

        let json = serde_json::to_string(&cmd)?;
        {
            let mut writer = self.inner.writer.lock().await;
            writer.write_all(json.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
        }

        tokio::time::timeout(std::time::Duration::from_secs(30), rx)
            .await
            .map_err(|_| DaemonClientError::Protocol("response timeout".into()))?
            .map_err(|_| DaemonClientError::ConnectionClosed)
    }

    /// Subscribe to runtime events for a session.
    pub async fn subscribe_events(
        &self,
        session_id: SessionId,
    ) -> Result<RuntimeEventReceiverHandle, DaemonClientError> {
        let cmd = ClientCommand::Subscribe {
            request_id: self.next_request_id(),
            session_id,
        };

        let json = serde_json::to_string(&cmd)?;
        {
            let mut writer = self.inner.writer.lock().await;
            writer.write_all(json.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
        }

        // Register a channel for this subscription. The reader task will
        // fan-out RuntimeEvent messages to all registered channels.
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        {
            let mut channels = self.inner.event_channels.write().await;
            channels.push(tx);
        }
        tracing::info!("Subscribed to daemon events for session {}", session_id);
        Ok(RuntimeEventReceiverHandle::new(rx))
    }
}

impl RuntimeClient for DaemonRuntimeClient {
    fn capabilities(&self) -> RuntimeClientFuture<RuntimeCapabilities> {
        let client = self.clone();
        Box::pin(async move {
            let cmd = ClientCommand::Capabilities {
                request_id: client.next_request_id(),
            };
            match client.send_command(cmd).await {
                Ok(ServerMessage::Capabilities { capabilities, .. }) => Ok(capabilities.runtime),
                Ok(msg) => Err(format!("unexpected response: {:?}", msg)),
                Err(e) => Err(e.to_string()),
            }
        })
    }

    fn create_session(
        &self,
        surface: RuntimeSurface,
        scope: Option<SessionScope>,
        labels: Vec<String>,
    ) -> RuntimeClientFuture<SessionId> {
        let client = self.clone();
        Box::pin(async move {
            let cmd = ClientCommand::CreateSession {
                request_id: client.next_request_id(),
                surface,
                scope,
                labels,
            };
            match client.send_command(cmd).await {
                Ok(ServerMessage::SessionCreated { session_id, .. }) => Ok(session_id),
                Ok(ServerMessage::Error { message, .. }) => Err(message),
                Ok(msg) => Err(format!("unexpected response: {:?}", msg)),
                Err(e) => Err(e.to_string()),
            }
        })
    }

    fn list_sessions(&self) -> RuntimeClientFuture<Vec<SessionSummary>> {
        let client = self.clone();
        Box::pin(async move {
            let cmd = ClientCommand::ListSessions {
                request_id: client.next_request_id(),
            };
            match client.send_command(cmd).await {
                Ok(ServerMessage::Sessions { sessions, .. }) => Ok(sessions),
                Ok(msg) => Err(format!("unexpected response: {:?}", msg)),
                Err(e) => Err(e.to_string()),
            }
        })
    }

    fn snapshot(&self, session_id: SessionId) -> RuntimeClientFuture<SessionSnapshot> {
        let client = self.clone();
        Box::pin(async move {
            let cmd = ClientCommand::GetSnapshot {
                request_id: client.next_request_id(),
                session_id,
            };
            match client.send_command(cmd).await {
                Ok(ServerMessage::Snapshot { snapshot, .. }) => Ok(snapshot),
                Ok(ServerMessage::Error { message, .. }) => Err(message),
                Ok(msg) => Err(format!("unexpected response: {:?}", msg)),
                Err(e) => Err(e.to_string()),
            }
        })
    }

    fn submit(&self, session_id: SessionId, request: RunRequest) -> RuntimeClientFuture<TaskId> {
        let client = self.clone();
        Box::pin(async move {
            let cmd = ClientCommand::SubmitTask {
                request_id: client.next_request_id(),
                session_id,
                request,
            };
            match client.send_command(cmd).await {
                Ok(ServerMessage::TaskSubmitted { task_id, .. }) => Ok(task_id),
                Ok(ServerMessage::Error { message, .. }) => Err(message),
                Ok(msg) => Err(format!("unexpected response: {:?}", msg)),
                Err(e) => Err(e.to_string()),
            }
        })
    }

    fn cancel(&self, session_id: SessionId, task_id: TaskId) -> RuntimeClientFuture<()> {
        let client = self.clone();
        Box::pin(async move {
            let cmd = ClientCommand::CancelTask {
                request_id: client.next_request_id(),
                session_id,
                task_id,
            };
            match client.send_command(cmd).await {
                Ok(ServerMessage::Ok { .. }) => Ok(()),
                Ok(ServerMessage::Error { message, .. }) => Err(message),
                Ok(msg) => Err(format!("unexpected response: {:?}", msg)),
                Err(e) => Err(e.to_string()),
            }
        })
    }

    fn cancel_active(&self, session_id: SessionId) -> RuntimeClientFuture<()> {
        let client = self.clone();
        Box::pin(async move {
            let cmd = ClientCommand::CancelActive {
                request_id: client.next_request_id(),
                session_id,
            };
            match client.send_command(cmd).await {
                Ok(ServerMessage::Ok { .. }) => Ok(()),
                Ok(ServerMessage::Error { message, .. }) => Err(message),
                Ok(msg) => Err(format!("unexpected response: {:?}", msg)),
                Err(e) => Err(e.to_string()),
            }
        })
    }

    fn approve_task(
        &self,
        session_id: SessionId,
        task_id: TaskId,
        approved: bool,
        reason: Option<String>,
    ) -> RuntimeClientFuture<()> {
        let client = self.clone();
        Box::pin(async move {
            let cmd = ClientCommand::ApprovePolicy {
                request_id: client.next_request_id(),
                session_id,
                task_id,
                approved,
                reason,
            };
            match client.send_command(cmd).await {
                Ok(ServerMessage::Ok { .. }) => Ok(()),
                Ok(ServerMessage::Error { message, .. }) => Err(message),
                Ok(msg) => Err(format!("unexpected response: {:?}", msg)),
                Err(e) => Err(e.to_string()),
            }
        })
    }

    fn subscribe(&self, session_id: SessionId) -> RuntimeClientFuture<RuntimeEventReceiverHandle> {
        let client = self.clone();
        Box::pin(async move {
            client
                .subscribe_events(session_id)
                .await
                .map_err(|e| e.to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggsec_daemon_protocol::client_registry::ClientKind;
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixListener;

    /// A unique socket path for one fake daemon.
    fn fake_socket_path() -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        PathBuf::from(format!(
            "/tmp/eggsec-fc-rpt-{}-{serial}.sock",
            std::process::id()
        ))
    }

    /// A fake daemon that answers each command type with a canned response.
    ///
    /// The answer always carries the request id the client used, so the
    /// client's request-id routing is exercised the way a real daemon
    /// exercises it.
    struct FakeDaemon {
        socket_path: PathBuf,
    }

    impl FakeDaemon {
        async fn start(responses: BTreeMap<String, serde_json::Value>) -> Self {
            let socket_path = fake_socket_path();
            let listener = UnixListener::bind(&socket_path).expect("the fake daemon binds");
            tokio::spawn(async move {
                serve_fake_daemon(listener, responses).await;
            });
            Self { socket_path }
        }

        fn path(&self) -> String {
            self.socket_path.to_string_lossy().into_owned()
        }
    }

    impl Drop for FakeDaemon {
        fn drop(&mut self) {
            if let Err(error) = std::fs::remove_file(&self.socket_path) {
                tracing::debug!(
                    path = %self.socket_path.display(),
                    %error,
                    "could not remove the fake daemon socket"
                );
            }
        }
    }

    /// Serve one connection, answering each command with its canned
    /// response.
    async fn serve_fake_daemon(
        listener: UnixListener,
        responses: BTreeMap<String, serde_json::Value>,
    ) {
        let (stream, _) = match listener.accept().await {
            Ok(pair) => pair,
            Err(error) => {
                tracing::warn!(%error, "the fake daemon could not accept a connection");
                return;
            }
        };
        let (read_half, mut write_half) = stream.into_split();
        let mut lines = BufReader::new(read_half).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let request: serde_json::Value = match serde_json::from_str(&line) {
                Ok(value) => value,
                Err(error) => {
                    tracing::warn!(%error, "the fake daemon could not parse a command");
                    continue;
                }
            };
            let request_id = request
                .get("request_id")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_owned();
            let command_type = request
                .get("type")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_owned();

            let mut response = responses.get(&command_type).cloned().unwrap_or_else(|| {
                serde_json::json!({
                    "type": "Error",
                    "code": {"type": "Internal"},
                    "message": "the fake daemon has no answer for this command",
                })
            });
            if let Some(object) = response.as_object_mut() {
                object.insert(
                    "request_id".to_owned(),
                    serde_json::Value::String(request_id),
                );
            }

            let encoded = match serde_json::to_string(&response) {
                Ok(encoded) => encoded,
                Err(error) => {
                    tracing::warn!(%error, "the fake daemon could not encode a response");
                    continue;
                }
            };
            if write_half.write_all(encoded.as_bytes()).await.is_err()
                || write_half.write_all(b"\n").await.is_err()
                || write_half.flush().await.is_err()
            {
                break;
            }
        }
    }

    /// The canned responses a fake daemon needs: the client declaration
    /// handshake plus whatever the test under exercise cares about.
    fn fake_responses(report_response: serde_json::Value) -> BTreeMap<String, serde_json::Value> {
        let mut responses = BTreeMap::new();
        responses.insert(
            "DeclareClient".to_owned(),
            serde_json::json!({
                "type": "ClientDeclared",
                "client_id": "00000000-0000-0000-0000-000000000001",
            }),
        );
        responses.insert("GetTaskReport".to_owned(), report_response);
        responses
    }

    /// The crate declares no `macros` Tokio feature, so `#[tokio::test]` is
    /// unavailable. A current-thread runtime drives the fake daemon and the
    /// client together.
    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a current-thread runtime")
            .block_on(future)
    }

    async fn connect_fake(daemon: &FakeDaemon) -> DaemonRuntimeClient {
        DaemonRuntimeClient::connect(
            &daemon.path(),
            ClientIdentity {
                kind: ClientKind::Gui,
                label: "eggsec-gui".to_owned(),
                request_prefix: "gui-".to_owned(),
            },
        )
        .await
        .expect("the client connects to the fake daemon")
    }

    /// The report document comes back as the daemon sent it.
    #[test]
    fn get_task_report_returns_the_published_document() {
        block_on(async {
            let responses = fake_responses(serde_json::json!({
                "type": "TaskReport",
                "session_id": "00000000-0000-0000-0000-000000000002",
                "task_id": "00000000-0000-0000-0000-000000000003",
                "report": {"findings": [], "target": "10.0.0.1"},
            }));
            let daemon = FakeDaemon::start(responses).await;
            let client = connect_fake(&daemon).await;

            let report = client
                .get_task_report(&SessionId::new(), &TaskId::new())
                .await
                .expect("the daemon answers the report request");
            let report = report.expect("a report is available");
            assert_eq!(report["target"], "10.0.0.1");
        });
    }

    /// A daemon that does not know the command is a degraded client, not a
    /// broken one: the client must not fail.
    #[test]
    fn get_task_report_degrades_when_the_daemon_does_not_know_the_command() {
        block_on(async {
            let responses = fake_responses(serde_json::json!({
                "type": "Error",
                "code": {"type": "UnsupportedCommand"},
                "message": "unsupported command: GetTaskReport",
            }));
            let daemon = FakeDaemon::start(responses).await;
            let client = connect_fake(&daemon).await;

            let report = client
                .get_task_report(&SessionId::new(), &TaskId::new())
                .await
                .expect("an unsupported command is not a client failure");
            assert!(
                report.is_none(),
                "a daemon that does not know the command has no report to publish"
            );
        });
    }

    /// A task with no report is the same degradation: no report, no failure.
    #[test]
    fn get_task_report_degrades_when_the_task_has_no_report() {
        block_on(async {
            let responses = fake_responses(serde_json::json!({
                "type": "Error",
                "code": {"type": "TaskNotFound"},
                "message": "no report is available for this task",
            }));
            let daemon = FakeDaemon::start(responses).await;
            let client = connect_fake(&daemon).await;

            let report = client
                .get_task_report(&SessionId::new(), &TaskId::new())
                .await
                .expect("a missing report is not a client failure");
            assert!(report.is_none());
        });
    }

    /// Any other refusal is a genuine failure and is reported as one.
    #[test]
    fn get_task_report_reports_a_genuine_failure_as_an_error() {
        block_on(async {
            let responses = fake_responses(serde_json::json!({
                "type": "Error",
                "code": {"type": "Internal"},
                "message": "the report store is unavailable",
            }));
            let daemon = FakeDaemon::start(responses).await;
            let client = connect_fake(&daemon).await;

            let error = client
                .get_task_report(&SessionId::new(), &TaskId::new())
                .await
                .expect_err("a genuine failure is reported as one");
            match error {
                DaemonClientError::Protocol(message) => {
                    assert_eq!(message, "the report store is unavailable")
                }
                other => panic!("expected a protocol error, got {other:?}"),
            }
        });
    }
}
