use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use uuid::Uuid;
use crate::tool::agents::registry::{AgentRegistry, AgentInfo, AgentStatus};

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

#[derive(Debug, Clone)]
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
    health_status: Arc<RwLock<HashMap<Uuid, AgentHealth>>>,
    event_tx: mpsc::Sender<LifecycleEvent>,
}

impl LifecycleManager {
    pub fn new(
        agent_registry: AgentRegistry,
        config: LifecycleConfig,
    ) -> (Self, mpsc::Receiver<LifecycleEvent>) {
        let (event_tx, event_rx) = mpsc::channel(100);
        (
            Self {
                config,
                agent_registry,
                health_status: Arc::new(RwLock::new(HashMap::new())),
                event_tx,
            },
            event_rx,
        )
    }

    pub async fn start_health_monitor(&self) {
        let health_status = Arc::clone(&self.health_status);
        let agent_registry = self.agent_registry.clone();
        let config = self.config.clone();
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(config.health_check_interval_secs));

            loop {
                ticker.tick().await;
                Self::perform_health_check(
                    &health_status,
                    &agent_registry,
                    &config,
                    &event_tx,
                ).await;
            }
        });
    }

    async fn perform_health_check(
        health_status: &Arc<RwLock<HashMap<Uuid, AgentHealth>>>,
        agent_registry: &AgentRegistry,
        config: &LifecycleConfig,
        event_tx: &mpsc::Sender<LifecycleEvent>,
    ) {
        let agents = agent_registry.list().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for agent in agents {
            let is_stale = now - agent.last_heartbeat > config.stale_threshold_secs;

            let mut status = health_status.write().await;
            let agent_health = status.entry(agent.id).or_insert_with(|| AgentHealth {
                agent_id: agent.id,
                is_healthy: true,
                consecutive_failures: 0,
                last_health_check: now,
                issues: Vec::new(),
            });

            agent_health.last_health_check = now;

            if is_stale {
                agent_health.is_healthy = false;
                if !agent_health.issues.iter().any(|i| matches!(i, HealthIssue::MissedHeartbeat)) {
                    agent_health.issues.push(HealthIssue::MissedHeartbeat);

                    let _ = event_tx.send(LifecycleEvent {
                        event_type: LifecycleEventType::AgentMarkedStale,
                        agent_id: agent.id,
                        timestamp: now,
                        details: Some(format!(
                            "Agent {} missed heartbeat, last seen {}s ago",
                            agent.name,
                            now - agent.last_heartbeat
                        )),
                    }).await;
                }

                let _ = agent_registry.update_status(agent.id, AgentStatus::Idle).await;
            } else if agent_health.consecutive_failures >= config.max_consecutive_failures {
                agent_health.is_healthy = false;
                let _ = agent_registry.update_status(agent.id, AgentStatus::Offline).await;
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
                .unwrap()
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
                let _ = self.event_tx.send(LifecycleEvent {
                    event_type: LifecycleEventType::HealthCheckFailed,
                    agent_id,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    details: Some(format!(
                        "Agent exceeded max failures ({}) due to: {}",
                        self.config.max_consecutive_failures, reason
                    )),
                }).await;
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
        status.values()
            .filter(|h| !h.is_healthy)
            .cloned()
            .collect()
    }

    pub async fn initiate_graceful_shutdown(&self, agent_id: Uuid) -> bool {
        let _ = self.event_tx.send(LifecycleEvent {
            event_type: LifecycleEventType::GracefulShutdown,
            agent_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            details: None,
        }).await;

        let _ = self.agent_registry.unregister(agent_id).await;
        true
    }

    pub async fn force_shutdown(&self, agent_id: Uuid) {
        let _ = self.event_tx.send(LifecycleEvent {
            event_type: LifecycleEventType::ForcedShutdown,
            agent_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            details: Some("Agent force shutdown initiated".to_string()),
        }).await;

        let _ = self.agent_registry.unregister(agent_id).await;

        let mut status = self.health_status.write().await;
        status.remove(&agent_id);
    }
}
