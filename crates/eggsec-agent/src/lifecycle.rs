use crate::registry::{AgentRegistry, AgentStatus};
use eggsec_transport::{
    HttpTransport, Method, NetworkAuthority, RedirectPolicy, ScopedHttpRequest, TimeoutPolicy,
};
use rustc_hash::FxHashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct LifecycleConfig {
    pub health_check_interval_secs: u64,
    pub stale_threshold_secs: u64,
    pub max_consecutive_failures: usize,
    pub graceful_shutdown_timeout_secs: u64,
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            health_check_interval_secs: 30,
            stale_threshold_secs: 120,
            max_consecutive_failures: 5,
            graceful_shutdown_timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentHealth {
    pub agent_id: Uuid,
    pub is_healthy: bool,
    pub consecutive_failures: usize,
    pub last_health_check: u64,
    pub issues: Vec<HealthIssue>,
}

#[derive(Debug, Clone)]
pub enum HealthIssue {
    MissedHeartbeat,
    CallbackUnhealthy(String),
    HighLatency(u64),
    TaskTimeout,
    ResourceExhaustion(String),
}

#[derive(Debug, Clone)]
pub struct LifecycleEvent {
    pub event_type: LifecycleEventType,
    pub agent_id: Uuid,
    pub timestamp: u64,
    pub details: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleEventType {
    AgentRegistered,
    AgentUnregistered,
    HealthCheckPassed,
    HealthCheckFailed,
    AgentMarkedStale,
    AgentRecovered,
    GracefulShutdown,
    ForcedShutdown,
}

pub struct LifecycleManager<T: HttpTransport> {
    config: LifecycleConfig,
    agent_registry: AgentRegistry,
    health_status: Arc<RwLock<FxHashMap<Uuid, AgentHealth>>>,
    event_tx: mpsc::Sender<LifecycleEvent>,
    transport: Arc<T>,
    authority: Arc<dyn NetworkAuthority>,
}

impl<T: HttpTransport> Clone for LifecycleManager<T> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            agent_registry: self.agent_registry.clone(),
            health_status: Arc::clone(&self.health_status),
            event_tx: self.event_tx.clone(),
            transport: Arc::clone(&self.transport),
            authority: Arc::clone(&self.authority),
        }
    }
}

impl<T: HttpTransport + 'static> LifecycleManager<T> {
    /// Canonical constructor (Phase D): the owning process injects a
    /// scope-aware [`HttpTransport`] plus the operation's
    /// [`NetworkAuthority`]. The manager never constructs an unrestricted
    /// HTTP client itself; every callback probe executes through the
    /// mandatory authority checkpoints (initial-url → host → dns → socket →
    /// tls-consistency → proxy → dispatch → redirect).
    ///
    /// Timeouts stay explicit: each callback probe carries a 5s
    /// per-request timeout (parity with the pre-migration client) and each
    /// monitor pass is bounded by 60s. Redirects follow same-host targets
    /// only (up to 5 hops); cross-host 3xx responses surface as unhealthy
    /// rather than following out-of-scope hosts.
    pub fn new(
        agent_registry: AgentRegistry,
        config: LifecycleConfig,
        transport: Arc<T>,
        authority: Arc<dyn NetworkAuthority>,
    ) -> (Self, mpsc::Receiver<LifecycleEvent>) {
        let (event_tx, event_rx) = mpsc::channel(100);
        (
            Self {
                config,
                agent_registry,
                health_status: Arc::new(RwLock::new(FxHashMap::default())),
                event_tx,
                transport,
                authority,
            },
            event_rx,
        )
    }

    /// Start the health monitor loop WITHOUT a cancellation token.
    ///
    /// This is a convenience for tests and fire-and-forget embeddings that
    /// manage shutdown via the returned [`tokio::task::JoinHandle::abort`].
    /// Production code that needs graceful shutdown should prefer
    /// [`Self::start_health_monitor_with_token`].
    /// Each pass is bounded by a 60s timeout; the task ends when the caller
    /// aborts the handle or (for the token variant) cancels the token.
    pub fn start_health_monitor(&self) -> tokio::task::JoinHandle<()> {
        self.start_health_monitor_with_token(CancellationToken::new())
    }

    /// Start the health monitor loop with a [`CancellationToken`] for graceful shutdown.
    ///
    /// Cancelling the token stops the loop after the in-flight pass finishes.
    pub fn start_health_monitor_with_token(
        &self,
        token: CancellationToken,
    ) -> tokio::task::JoinHandle<()> {
        let health_status = Arc::clone(&self.health_status);
        let agent_registry = self.agent_registry.clone();
        let config = self.config.clone();
        let event_tx = self.event_tx.clone();
        let transport = Arc::clone(&self.transport);
        let authority = Arc::clone(&self.authority);

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(config.health_check_interval_secs));

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        // Bound each pass so a stalled registry/network cannot
                        // wedge the monitor task indefinitely.
                        if tokio::time::timeout(
                            Duration::from_secs(60),
                            Self::perform_health_check(
                                &health_status,
                                &agent_registry,
                                &config,
                                &event_tx,
                                transport.as_ref(),
                                authority.as_ref(),
                            ),
                        )
                        .await
                        .is_err()
                        {
                            tracing::warn!("Agent health check pass timed out after 60s");
                        }
                    }
                    _ = token.cancelled() => {
                        break;
                    }
                }
            }
        })
    }

    async fn check_agent_callback_health_static(
        transport: &T,
        authority: &dyn NetworkAuthority,
        callback_url: &str,
    ) -> bool {
        let request = match ScopedHttpRequest::new_with_url(Method::GET, callback_url) {
            Ok(req) => req
                .with_timeout(TimeoutPolicy::with_request_timeout(5))
                .with_redirect(RedirectPolicy::SameHostOnly { max_redirects: 5 }),
            Err(e) => {
                tracing::debug!(url = %callback_url, error = %e, "callback URL rejected");
                return false;
            }
        };
        match transport.execute(authority, request).await {
            Ok(resp) => resp.status.is_success(),
            Err(e) => {
                tracing::debug!(url = %callback_url, error = %e, "callback health probe failed");
                false
            }
        }
    }

    async fn perform_health_check(
        health_status: &Arc<RwLock<FxHashMap<Uuid, AgentHealth>>>,
        agent_registry: &AgentRegistry,
        config: &LifecycleConfig,
        event_tx: &mpsc::Sender<LifecycleEvent>,
        transport: &T,
        authority: &dyn NetworkAuthority,
    ) {
        let agents = agent_registry.list().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs();

        #[derive(Debug)]
        struct AgentCheckState {
            agent_id: Uuid,
            agent_name: String,
            is_stale: bool,
            callback_unhealthy: bool,
            was_healthy: bool,
            has_missed_heartbeat: bool,
            has_callback_issue: bool,
        }

        let mut check_states: Vec<AgentCheckState> = Vec::new();

        {
            let status = health_status.read().await;
            for agent in &agents {
                let is_stale =
                    now.saturating_sub(agent.last_heartbeat) > config.stale_threshold_secs;

                let agent_health = status.get(&agent.id);
                let (was_healthy, has_missed_heartbeat, has_callback_issue) =
                    if let Some(health) = agent_health {
                        (
                            health.is_healthy,
                            health
                                .issues
                                .iter()
                                .any(|i| matches!(i, HealthIssue::MissedHeartbeat)),
                            health
                                .issues
                                .iter()
                                .any(|i| matches!(i, HealthIssue::CallbackUnhealthy(_))),
                        )
                    } else {
                        (true, false, false)
                    };

                check_states.push(AgentCheckState {
                    agent_id: agent.id,
                    agent_name: agent.name.clone(),
                    is_stale,
                    callback_unhealthy: false,
                    was_healthy,
                    has_missed_heartbeat,
                    has_callback_issue,
                });
            }
        }

        let callback_results: Vec<(Uuid, bool)> = {
            let mut results = Vec::new();
            for agent in &agents {
                let callback_unhealthy = if let Some(ref callback_url) = agent.callback_url {
                    !Self::check_agent_callback_health_static(transport, authority, callback_url)
                        .await
                } else {
                    false
                };
                results.push((agent.id, callback_unhealthy));
            }
            results
        };

        for check in check_states.iter_mut() {
            check.callback_unhealthy = callback_results
                .iter()
                .find(|(id, _)| *id == check.agent_id)
                .map(|(_, r)| *r)
                .unwrap_or(false);
        }

        let mut pending_events: Vec<LifecycleEvent> = Vec::new();
        let mut mark_offline: Vec<Uuid> = Vec::new();
        let mut stale_heartbeat_agents: Vec<(Uuid, String)> = Vec::new();

        {
            let mut status = health_status.write().await;
            for check in check_states {
                let is_stale = check.is_stale;
                let callback_unhealthy = check.callback_unhealthy;
                let was_healthy = check.was_healthy;
                let has_missed_heartbeat = check.has_missed_heartbeat;
                let has_callback_issue = check.has_callback_issue;

                let agent_health = status.entry(check.agent_id).or_insert_with(|| AgentHealth {
                    agent_id: check.agent_id,
                    is_healthy: true,
                    consecutive_failures: 0,
                    last_health_check: now,
                    issues: Vec::new(),
                });

                agent_health.last_health_check = now;

                if is_stale || callback_unhealthy {
                    agent_health.is_healthy = false;

                    if is_stale && !has_missed_heartbeat {
                        agent_health.issues.push(HealthIssue::MissedHeartbeat);
                        stale_heartbeat_agents.push((check.agent_id, check.agent_name.clone()));
                    }

                    if callback_unhealthy && !has_callback_issue {
                        agent_health.issues.push(HealthIssue::CallbackUnhealthy(
                            "Callback health check failed".to_string(),
                        ));
                        pending_events.push(LifecycleEvent {
                            event_type: LifecycleEventType::AgentMarkedStale,
                            agent_id: check.agent_id,
                            timestamp: now,
                            details: Some(format!(
                                "Agent {} failed callback health check",
                                check.agent_name
                            )),
                        });
                    }

                    mark_offline.push(check.agent_id);
                } else if !was_healthy && !is_stale && !callback_unhealthy {
                    if !agent_health.issues.is_empty() || !agent_health.is_healthy {
                        agent_health.issues.clear();
                        agent_health.is_healthy = true;
                        pending_events.push(LifecycleEvent {
                            event_type: LifecycleEventType::AgentRecovered,
                            agent_id: check.agent_id,
                            timestamp: now,
                            details: Some("Agent health restored".to_string()),
                        });
                    }
                } else if agent_health.consecutive_failures >= config.max_consecutive_failures {
                    agent_health.is_healthy = false;
                    mark_offline.push(check.agent_id);
                }
            }
        }

        for (agent_id, agent_name) in stale_heartbeat_agents {
            let last_seen = agent_registry
                .get(agent_id)
                .await
                .map(|a| a.last_heartbeat)
                .unwrap_or(now);
            pending_events.push(LifecycleEvent {
                event_type: LifecycleEventType::AgentMarkedStale,
                agent_id,
                timestamp: now,
                details: Some(format!(
                    "Agent {} missed heartbeat, last seen {}s ago",
                    agent_name,
                    now.saturating_sub(last_seen)
                )),
            });
        }

        for agent_id in mark_offline {
            agent_registry
                .update_status(agent_id, AgentStatus::Offline)
                .await;
        }

        for event in pending_events {
            if let Err(e) = event_tx.send(event).await {
                tracing::warn!("Failed to send pending event: {:?}", e);
            }
        }
    }

    pub async fn record_task_start(&self, agent_id: Uuid) {
        let mut status = self.health_status.write().await;
        let health = status.entry(agent_id).or_insert_with(|| AgentHealth {
            agent_id,
            is_healthy: true,
            consecutive_failures: 0,
            last_health_check: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                .as_secs(),
            issues: Vec::new(),
        });

        if !health.is_healthy {
            health.consecutive_failures = 0;
            health.is_healthy = true;
            health.issues.clear();
        }
    }

    pub async fn record_task_success(&self, agent_id: Uuid) {
        let mut status = self.health_status.write().await;
        if let Some(health) = status.get_mut(&agent_id) {
            health.consecutive_failures = 0;
        }
    }

    pub async fn record_task_failure(&self, agent_id: Uuid, reason: &str) {
        let mut status = self.health_status.write().await;
        if let Some(health) = status.get_mut(&agent_id) {
            health.consecutive_failures += 1;

            if health.consecutive_failures >= self.config.max_consecutive_failures {
                health.is_healthy = false;
                if let Err(e) = self
                    .event_tx
                    .send(LifecycleEvent {
                        event_type: LifecycleEventType::HealthCheckFailed,
                        agent_id,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                            .as_secs(),
                        details: Some(format!(
                            "Agent exceeded max failures ({}) due to: {}",
                            self.config.max_consecutive_failures, reason
                        )),
                    })
                    .await
                {
                    tracing::warn!(
                        "Failed to send HealthCheckFailed event for agent {}: {:?}",
                        agent_id,
                        e
                    );
                }
            }
        }
    }

    pub async fn get_agent_health(&self, agent_id: Uuid) -> Option<AgentHealth> {
        let status = self.health_status.read().await;
        status.get(&agent_id).cloned()
    }

    pub async fn get_all_health_status(&self) -> Vec<AgentHealth> {
        let status = self.health_status.read().await;
        status.values().cloned().collect()
    }

    pub async fn get_unhealthy_agents(&self) -> Vec<AgentHealth> {
        let status = self.health_status.read().await;
        status.values().filter(|h| !h.is_healthy).cloned().collect()
    }

    pub async fn initiate_graceful_shutdown(&self, agent_id: Uuid) -> bool {
        if let Err(e) = self
            .event_tx
            .send(LifecycleEvent {
                event_type: LifecycleEventType::GracefulShutdown,
                agent_id,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                    .as_secs(),
                details: None,
            })
            .await
        {
            tracing::warn!(
                "Failed to send GracefulShutdown event for agent {}: {:?}",
                agent_id,
                e
            );
        }

        self.agent_registry.unregister(agent_id).await;
        true
    }

    pub async fn force_shutdown(&self, agent_id: Uuid) {
        if let Err(e) = self
            .event_tx
            .send(LifecycleEvent {
                event_type: LifecycleEventType::ForcedShutdown,
                agent_id,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                    .as_secs(),
                details: Some("Agent force shutdown initiated".to_string()),
            })
            .await
        {
            tracing::warn!(
                "Failed to send ForcedShutdown event for agent {}: {:?}",
                agent_id,
                e
            );
        }

        self.agent_registry.unregister(agent_id).await;

        let mut status = self.health_status.write().await;
        status.remove(&agent_id);
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use crate::AgentInfo;
    use eggsec_transport::{
        CannedResponse, InMemoryResolver, PolicyCheckpoint, RecordingFakeTransport, StatusCode,
        TransportError,
    };
    use std::net::IpAddr;
    use url::Url;
    use uuid::Uuid;

    /// Test-only permissive authority: approves every checkpoint.
    ///
    /// Production callers must supply the operation's real authority
    /// (canonically `eggsec::config::ScopeAuthority`); this helper exists
    /// only so unit tests can exercise health logic deterministically
    /// through the recording fake without network I/O.
    #[derive(Debug, Default)]
    struct AllowAllAuthority;

    impl NetworkAuthority for AllowAllAuthority {
        fn authorize_initial_url(&self, _url: &Url) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_host(
            &self,
            _host: &str,
            _port: Option<u16>,
            _is_ip_literal: bool,
        ) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_resolved(
            &self,
            _host: &str,
            candidates: &[IpAddr],
        ) -> Result<Vec<IpAddr>, TransportError> {
            Ok(candidates.to_vec())
        }
        fn authorize_socket(
            &self,
            _host: &str,
            _addr: IpAddr,
            _port: u16,
        ) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_redirect(&self, _from: &Url, _to: &Url) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_proxy(
            &self,
            _proxy_endpoint: &Url,
            _ultimate: &Url,
        ) -> Result<(), TransportError> {
            Ok(())
        }
        fn check_tls_consistency(
            &self,
            _request_host: &str,
            _sni_override: Option<&str>,
            _host_override: Option<&str>,
        ) -> Result<(), TransportError> {
            Ok(())
        }
    }

    /// Denying authority for failure-path tests (callback probes fail closed
    /// with no hop recorded, mirroring unreachable-host behavior).
    #[derive(Debug, Default)]
    struct DenyAllAuthority;

    impl NetworkAuthority for DenyAllAuthority {
        fn authorize_initial_url(&self, _url: &Url) -> Result<(), TransportError> {
            Err(TransportError::denied(
                PolicyCheckpoint::InitialUrl,
                "deny-all test authority",
            ))
        }
        fn authorize_host(
            &self,
            _host: &str,
            _port: Option<u16>,
            _is_ip_literal: bool,
        ) -> Result<(), TransportError> {
            Err(TransportError::denied(
                PolicyCheckpoint::Host,
                "deny-all test authority",
            ))
        }
        fn authorize_resolved(
            &self,
            _host: &str,
            _candidates: &[IpAddr],
        ) -> Result<Vec<IpAddr>, TransportError> {
            Err(TransportError::denied(
                PolicyCheckpoint::Dns,
                "deny-all test authority",
            ))
        }
        fn authorize_socket(
            &self,
            _host: &str,
            _addr: IpAddr,
            _port: u16,
        ) -> Result<(), TransportError> {
            Err(TransportError::denied(
                PolicyCheckpoint::Socket,
                "deny-all test authority",
            ))
        }
        fn authorize_redirect(&self, _from: &Url, _to: &Url) -> Result<(), TransportError> {
            Err(TransportError::denied(
                PolicyCheckpoint::Redirect,
                "deny-all test authority",
            ))
        }
        fn authorize_proxy(
            &self,
            _proxy_endpoint: &Url,
            _ultimate: &Url,
        ) -> Result<(), TransportError> {
            Err(TransportError::denied(
                PolicyCheckpoint::Proxy,
                "deny-all test authority",
            ))
        }
        fn check_tls_consistency(
            &self,
            _request_host: &str,
            _sni_override: Option<&str>,
            _host_override: Option<&str>,
        ) -> Result<(), TransportError> {
            Err(TransportError::denied(
                PolicyCheckpoint::TlsConsistency,
                "deny-all test authority",
            ))
        }
    }

    fn test_transport() -> (Arc<RecordingFakeTransport>, Arc<AllowAllAuthority>) {
        let resolver = InMemoryResolver::new()
            .with("example.com", vec!["93.184.216.34"])
            .shared();
        let fake = Arc::new(RecordingFakeTransport::new(resolver));
        (fake, Arc::new(AllowAllAuthority))
    }

    fn make_test_agent(
        id: Uuid,
        name: &str,
        callback_url: Option<String>,
        last_heartbeat: u64,
    ) -> AgentInfo {
        AgentInfo {
            id,
            name: name.to_string(),
            capabilities: vec!["scan".to_string()],
            status: AgentStatus::Active,
            last_heartbeat,
            callback_url,
        }
    }

    fn make_health_with_callback_issue(agent_id: Uuid) -> AgentHealth {
        AgentHealth {
            agent_id,
            is_healthy: false,
            consecutive_failures: 0,
            last_health_check: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                .as_secs(),
            issues: vec![HealthIssue::CallbackUnhealthy("test".to_string())],
        }
    }

    #[tokio::test]
    async fn test_health_issue_tracking_prevents_duplicate_events() {
        let registry = AgentRegistry::new();
        let config = LifecycleConfig::default();
        let (transport, authority) = test_transport();
        let (manager, _rx) = LifecycleManager::new(registry.clone(), config, transport, authority);

        let agent_id = Uuid::new_v4();

        {
            let mut status = manager.health_status.write().await;
            status.insert(agent_id, make_health_with_callback_issue(agent_id));
        }

        let health = manager.get_agent_health(agent_id).await;
        assert!(health.is_some());
        let health = health.unwrap();

        assert!(!health.is_healthy);
        assert!(health
            .issues
            .iter()
            .any(|i| matches!(i, HealthIssue::CallbackUnhealthy(_))));

        let has_callback = health
            .issues
            .iter()
            .any(|i| matches!(i, HealthIssue::CallbackUnhealthy(_)));
        assert!(has_callback, "CallbackUnhealthy issue should be tracked");
    }

    #[tokio::test]
    async fn test_recovery_clears_callback_issue() {
        let registry = AgentRegistry::new();
        let config = LifecycleConfig::default();
        let (transport, authority) = test_transport();
        let (manager, _rx) = LifecycleManager::new(registry.clone(), config, transport, authority);

        let agent_id = Uuid::new_v4();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs();

        registry
            .register(make_test_agent(agent_id, "test-agent", None, now))
            .await;

        {
            let mut status = manager.health_status.write().await;
            status.insert(agent_id, make_health_with_callback_issue(agent_id));
        }

        registry.heartbeat(agent_id).await;

        {
            let mut status = manager.health_status.write().await;
            if let Some(health) = status.get_mut(&agent_id) {
                health.issues.clear();
                health.is_healthy = true;
            }
        }

        let health = manager.get_agent_health(agent_id).await;
        assert!(health.is_some());
        let health = health.unwrap();

        assert!(health.is_healthy, "Agent should be healthy after recovery");
        assert!(
            health.issues.is_empty(),
            "Issues should be cleared on recovery"
        );
    }

    #[tokio::test]
    async fn test_record_task_start_not_blocked_by_slow_callback() {
        let registry = AgentRegistry::new();
        let config = LifecycleConfig::default();
        let (transport, authority) = test_transport();
        let (manager, _rx) = LifecycleManager::new(registry.clone(), config, transport, authority);

        let agent_id = Uuid::new_v4();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs();

        registry
            .register(make_test_agent(agent_id, "test-agent", None, now))
            .await;

        manager.record_task_start(agent_id).await;

        let health = manager.get_agent_health(agent_id).await;
        assert!(health.is_some());
        assert!(health.unwrap().is_healthy);
    }

    #[tokio::test]
    async fn test_record_task_success_not_blocked_by_slow_callback() {
        let registry = AgentRegistry::new();
        let config = LifecycleConfig::default();
        let (transport, authority) = test_transport();
        let (manager, _rx) = LifecycleManager::new(registry.clone(), config, transport, authority);

        let agent_id = Uuid::new_v4();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs();

        registry
            .register(make_test_agent(agent_id, "test-agent", None, now))
            .await;

        manager.record_task_start(agent_id).await;
        manager.record_task_success(agent_id).await;

        let health = manager.get_agent_health(agent_id).await;
        assert!(health.is_some());
        assert!(health.unwrap().consecutive_failures == 0);
    }

    #[tokio::test]
    async fn test_callback_failure_emits_one_stale_event() {
        let registry = AgentRegistry::new();
        let config = LifecycleConfig::default();
        // Denying authority models the pre-migration unreachable-host case
        // (connection refused → probe fails → exactly one stale event).
        // No hop is recorded on denial (fail-closed before dispatch).
        let resolver = InMemoryResolver::new().shared();
        let transport = Arc::new(RecordingFakeTransport::new(resolver));
        let authority: Arc<dyn NetworkAuthority> = Arc::new(DenyAllAuthority);
        let (manager, mut rx) = LifecycleManager::new(
            registry.clone(),
            config.clone(),
            transport.clone(),
            authority.clone(),
        );

        let agent_id = Uuid::new_v4();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs();

        registry
            .register(make_test_agent(
                agent_id,
                "test-agent",
                Some("http://127.0.0.1:9999".to_string()),
                now,
            ))
            .await;

        LifecycleManager::perform_health_check(
            &manager.health_status,
            &registry,
            &config,
            &manager.event_tx,
            transport.as_ref(),
            authority.as_ref(),
        )
        .await;

        let mut stale_count = 0;
        while let Ok(event) = rx.try_recv() {
            if event.event_type == LifecycleEventType::AgentMarkedStale
                && event.agent_id == agent_id
            {
                stale_count += 1;
            }
        }

        assert_eq!(
            stale_count, 1,
            "Should emit exactly one AgentMarkedStale event for callback failure"
        );

        let health = manager.get_agent_health(agent_id).await;
        assert!(health.is_some());
        let health = health.unwrap();
        assert!(!health.is_healthy);
        assert!(health
            .issues
            .iter()
            .any(|i| matches!(i, HealthIssue::CallbackUnhealthy(_))));
        assert_eq!(transport.hop_count(), 0, "denied probes must not dispatch");
    }

    #[tokio::test]
    async fn test_healthy_callback_after_failure_emits_recovery_event() {
        let registry = AgentRegistry::new();
        let config = LifecycleConfig::default();
        let resolver = InMemoryResolver::new().shared();
        let transport = Arc::new(RecordingFakeTransport::new(resolver));
        let authority: Arc<dyn NetworkAuthority> = Arc::new(DenyAllAuthority);
        let (manager, mut rx) = LifecycleManager::new(
            registry.clone(),
            config.clone(),
            transport.clone(),
            authority.clone(),
        );

        let agent_id = Uuid::new_v4();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs();

        registry
            .register(make_test_agent(
                agent_id,
                "test-agent",
                Some("http://127.0.0.1:9999".to_string()),
                now,
            ))
            .await;

        LifecycleManager::perform_health_check(
            &manager.health_status,
            &registry,
            &config,
            &manager.event_tx,
            transport.as_ref(),
            authority.as_ref(),
        )
        .await;

        let health = manager.get_agent_health(agent_id).await;
        assert!(health.is_some());
        let health = health.unwrap();
        assert!(!health.is_healthy);

        registry.update_status(agent_id, AgentStatus::Active).await;
        registry.heartbeat(agent_id).await;

        let _old_callback_url = registry.get(agent_id).await.unwrap().callback_url.clone();
        registry.update_status(agent_id, AgentStatus::Idle).await;

        {
            let mut status = manager.health_status.write().await;
            if let Some(h) = status.get_mut(&agent_id) {
                h.issues.clear();
                h.is_healthy = true;
            }
        }

        LifecycleManager::perform_health_check(
            &manager.health_status,
            &registry,
            &config,
            &manager.event_tx,
            transport.as_ref(),
            authority.as_ref(),
        )
        .await;

        let mut recovery_count = 0;
        while let Ok(event) = rx.try_recv() {
            if event.event_type == LifecycleEventType::AgentRecovered && event.agent_id == agent_id
            {
                recovery_count += 1;
            }
        }

        assert_eq!(
            recovery_count, 0,
            "Recovery should not emit event when already healthy in status map"
        );

        let health = manager.get_agent_health(agent_id).await;
        assert!(health.is_some());
    }

    #[tokio::test]
    async fn test_transport_success_marks_callback_healthy() {
        let registry = AgentRegistry::new();
        let config = LifecycleConfig::default();
        let resolver = InMemoryResolver::new()
            .with("callbacks.example", vec!["93.184.216.34"])
            .shared();
        let transport = Arc::new(RecordingFakeTransport::new(resolver).with_canned(
            "http://callbacks.example/health",
            CannedResponse::ok(b"ok".to_vec()),
        ));
        let authority: Arc<dyn NetworkAuthority> = Arc::new(AllowAllAuthority);
        let (manager, _rx) = LifecycleManager::new(
            registry.clone(),
            config.clone(),
            transport.clone(),
            authority.clone(),
        );

        let agent_id = Uuid::new_v4();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs();
        registry
            .register(make_test_agent(
                agent_id,
                "test-agent",
                Some("http://callbacks.example/health".to_string()),
                now,
            ))
            .await;

        LifecycleManager::perform_health_check(
            &manager.health_status,
            &registry,
            &config,
            &manager.event_tx,
            transport.as_ref(),
            authority.as_ref(),
        )
        .await;

        assert_eq!(transport.hop_count(), 1, "healthy probe dispatches once");
        let hop = transport.hops().pop().expect("hop");
        assert_eq!(hop.host, "callbacks.example");
        assert_eq!(hop.request_timeout_secs, 5);
        assert_eq!(hop.redirect_max, 5);
        assert!(!hop.uses_proxy);
        assert!(hop.tls_verified);
        let health = manager.get_agent_health(agent_id).await.expect("health");
        assert!(health.is_healthy);
    }

    #[tokio::test]
    async fn test_transport_non_success_marks_callback_unhealthy() {
        let registry = AgentRegistry::new();
        let config = LifecycleConfig::default();
        let resolver = InMemoryResolver::new()
            .with("callbacks.example", vec!["93.184.216.34"])
            .shared();
        let transport = Arc::new(RecordingFakeTransport::new(resolver).with_canned(
            "http://callbacks.example/health",
            CannedResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                location: None,
                body: b"boom".to_vec(),
            },
        ));
        let authority: Arc<dyn NetworkAuthority> = Arc::new(AllowAllAuthority);
        let (manager, _rx) = LifecycleManager::new(
            registry.clone(),
            config.clone(),
            transport.clone(),
            authority.clone(),
        );

        let agent_id = Uuid::new_v4();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs();
        registry
            .register(make_test_agent(
                agent_id,
                "test-agent",
                Some("http://callbacks.example/health".to_string()),
                now,
            ))
            .await;

        LifecycleManager::perform_health_check(
            &manager.health_status,
            &registry,
            &config,
            &manager.event_tx,
            transport.as_ref(),
            authority.as_ref(),
        )
        .await;

        let health = manager.get_agent_health(agent_id).await.expect("health");
        assert!(!health.is_healthy);
        assert!(health
            .issues
            .iter()
            .any(|i| matches!(i, HealthIssue::CallbackUnhealthy(_))));
    }

    #[tokio::test]
    async fn test_future_heartbeat_does_not_panic() {
        let registry = AgentRegistry::new();
        let config = LifecycleConfig::default();
        let (transport, authority) = test_transport();
        let (manager, _rx) = LifecycleManager::new(
            registry.clone(),
            config.clone(),
            transport.clone(),
            authority.clone(),
        );

        let agent_id = Uuid::new_v4();
        let future_time = u64::MAX;

        registry
            .register(make_test_agent(agent_id, "test-agent", None, future_time))
            .await;

        LifecycleManager::perform_health_check(
            &manager.health_status,
            &registry,
            &config,
            &manager.event_tx,
            transport.as_ref(),
            authority.as_ref(),
        )
        .await;

        let health = manager.get_agent_health(agent_id).await;
        assert!(health.is_some());
    }

    #[tokio::test]
    async fn test_health_monitor_can_be_stopped() {
        use tokio_util::sync::CancellationToken;

        let registry = AgentRegistry::new();
        let config = LifecycleConfig {
            health_check_interval_secs: 60,
            ..LifecycleConfig::default()
        };
        let (transport, authority) = test_transport();
        let (manager, _rx) = LifecycleManager::new(registry.clone(), config, transport, authority);

        let token = CancellationToken::new();
        let handle = manager.start_health_monitor_with_token(token.clone());

        token.cancel();

        let result = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        assert!(
            result.is_ok(),
            "Health monitor should stop when token is cancelled"
        );
    }

    #[tokio::test]
    async fn test_stale_agent_status_is_offline_not_idle() {
        let registry = AgentRegistry::new();
        let config = LifecycleConfig::default();
        let (transport, authority) = test_transport();
        let (manager, _rx) = LifecycleManager::new(
            registry.clone(),
            config.clone(),
            transport.clone(),
            authority.clone(),
        );

        let agent_id = Uuid::new_v4();
        let old_heartbeat = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs()
            .saturating_sub(config.stale_threshold_secs + 100);

        registry
            .register(make_test_agent(agent_id, "test-agent", None, old_heartbeat))
            .await;

        LifecycleManager::perform_health_check(
            &manager.health_status,
            &registry,
            &config,
            &manager.event_tx,
            transport.as_ref(),
            authority.as_ref(),
        )
        .await;

        let agent = registry.get(agent_id).await;
        assert!(agent.is_some());
        assert_eq!(
            agent.unwrap().status,
            AgentStatus::Offline,
            "Stale agent should have Offline status, not Idle"
        );
    }
}
