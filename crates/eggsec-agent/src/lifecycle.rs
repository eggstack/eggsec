use crate::registry::{AgentRegistry, AgentStatus};
use reqwest::Client;
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

#[derive(Clone)]
pub struct LifecycleManager {
    config: LifecycleConfig,
    agent_registry: AgentRegistry,
    health_status: Arc<RwLock<FxHashMap<Uuid, AgentHealth>>>,
    event_tx: mpsc::Sender<LifecycleEvent>,
    client: Client,
}

impl LifecycleManager {
    pub fn new(
        agent_registry: AgentRegistry,
        config: LifecycleConfig,
    ) -> (Self, mpsc::Receiver<LifecycleEvent>) {
        let (event_tx, event_rx) = mpsc::channel(100);
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .pool_max_idle_per_host(eggsec_core::constants::DEFAULT_POOL_MAX_IDLE_PER_HOST)
            .pool_idle_timeout(Duration::from_secs(
                eggsec_core::constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS,
            ))
            .tcp_nodelay(true)
            .build()
            .unwrap_or_else(|_| Client::new());
        (
            Self {
                config,
                agent_registry,
                health_status: Arc::new(RwLock::new(FxHashMap::default())),
                event_tx,
                client,
            },
            event_rx,
        )
    }

    pub fn start_health_monitor(&self) -> tokio::task::JoinHandle<()> {
        let health_status = Arc::clone(&self.health_status);
        let agent_registry = self.agent_registry.clone();
        let config = self.config.clone();
        let event_tx = self.event_tx.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(config.health_check_interval_secs));

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        Self::perform_health_check(
                            &health_status,
                            &agent_registry,
                            &config,
                            &event_tx,
                            &client,
                        ).await;
                    }
                }
            }
        })
    }

    pub fn start_health_monitor_with_token(
        &self,
        token: CancellationToken,
    ) -> tokio::task::JoinHandle<()> {
        let health_status = Arc::clone(&self.health_status);
        let agent_registry = self.agent_registry.clone();
        let config = self.config.clone();
        let event_tx = self.event_tx.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(config.health_check_interval_secs));

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        Self::perform_health_check(
                            &health_status,
                            &agent_registry,
                            &config,
                            &event_tx,
                            &client,
                        ).await;
                    }
                    _ = token.cancelled() => {
                        break;
                    }
                }
            }
        })
    }

    async fn check_agent_callback_health_static(client: &Client, callback_url: &str) -> bool {
        client
            .get(callback_url)
            .send()
            .await
            .map(|resp| resp.status().is_success())
            .unwrap_or(false)
    }

    async fn perform_health_check(
        health_status: &Arc<RwLock<FxHashMap<Uuid, AgentHealth>>>,
        agent_registry: &AgentRegistry,
        config: &LifecycleConfig,
        event_tx: &mpsc::Sender<LifecycleEvent>,
        client: &Client,
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
                    !Self::check_agent_callback_health_static(client, callback_url).await
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
    use uuid::Uuid;

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

    fn make_healthy_agent(agent_id: Uuid) -> AgentHealth {
        AgentHealth {
            agent_id,
            is_healthy: true,
            consecutive_failures: 0,
            last_health_check: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                .as_secs(),
            issues: vec![],
        }
    }

    #[tokio::test]
    async fn test_health_issue_tracking_prevents_duplicate_events() {
        let registry = AgentRegistry::new();
        let config = LifecycleConfig::default();
        let (manager, _rx) = LifecycleManager::new(registry.clone(), config);

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
        let (manager, mut rx) = LifecycleManager::new(registry.clone(), config);

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
        let (manager, _rx) = LifecycleManager::new(registry.clone(), config);

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
        let (manager, _rx) = LifecycleManager::new(registry.clone(), config);

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
        let (manager, mut rx) = LifecycleManager::new(registry.clone(), config.clone());

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
            &manager.client,
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
    }

    #[tokio::test]
    async fn test_healthy_callback_after_failure_emits_recovery_event() {
        let registry = AgentRegistry::new();
        let config = LifecycleConfig::default();
        let (manager, mut rx) = LifecycleManager::new(registry.clone(), config.clone());

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
            &manager.client,
        )
        .await;

        let health = manager.get_agent_health(agent_id).await;
        assert!(health.is_some());
        let health = health.unwrap();
        assert!(!health.is_healthy);

        registry.update_status(agent_id, AgentStatus::Active).await;
        registry.heartbeat(agent_id).await;

        let old_callback_url = registry.get(agent_id).await.unwrap().callback_url.clone();
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
            &manager.client,
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
    async fn test_future_heartbeat_does_not_panic() {
        let registry = AgentRegistry::new();
        let config = LifecycleConfig::default();
        let (manager, _rx) = LifecycleManager::new(registry.clone(), config.clone());

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
            &manager.client,
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
        let (manager, _rx) = LifecycleManager::new(registry.clone(), config);

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
        let (manager, _rx) = LifecycleManager::new(registry.clone(), config.clone());

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
            &manager.client,
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
