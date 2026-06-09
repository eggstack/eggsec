//! Inter-agent communication for multi-agent coordination.
//!
//! Provides message passing between agents for capability advertising,
//! health tracking, and collaborative task execution.

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::delegation::{DelegationRequest, DelegationResponse};
use crate::registry::{AgentInfo, AgentRegistry, AgentStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityAdvertisement {
    pub agent_id: Uuid,
    pub capabilities: Vec<AgentCapability>,
    pub advertised_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    pub name: String,
    pub description: String,
    pub version: String,
    pub parameters: Vec<CapabilityParam>,
    pub max_concurrent_tasks: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityParam {
    pub name: String,
    pub param_type: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    pub agent_id: Uuid,
    pub uptime_seconds: u64,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub current_load: f32,
    pub memory_usage_mb: Option<u64>,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub health_status: HealthStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl HealthMetrics {
    pub fn new(agent_id: Uuid) -> Self {
        Self {
            agent_id,
            uptime_seconds: 0,
            tasks_completed: 0,
            tasks_failed: 0,
            current_load: 0.0,
            memory_usage_mb: None,
            last_heartbeat: chrono::Utc::now(),
            health_status: HealthStatus::Unknown,
        }
    }

    pub fn update_load(&mut self, load: f32) {
        self.current_load = load.clamp(0.0, 1.0);
        self.update_health_status();
    }

    pub fn record_completion(&mut self) {
        self.tasks_completed += 1;
        self.update_health_status();
    }

    pub fn record_failure(&mut self) {
        self.tasks_failed += 1;
        self.update_health_status();
    }

    fn update_health_status(&mut self) {
        let failure_rate = if self.tasks_completed + self.tasks_failed > 0 {
            self.tasks_failed as f32 / (self.tasks_completed + self.tasks_failed) as f32
        } else {
            0.0
        };

        self.health_status = if self.current_load > 0.9 || failure_rate > 0.3 {
            HealthStatus::Unhealthy
        } else if self.current_load > 0.7 || failure_rate > 0.1 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub recipient_id: Option<Uuid>,
    pub message_type: MessageType,
    pub payload: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageType {
    Delegation(DelegationRequest),
    DelegationResponse(DelegationResponse),
    CapabilityAdvertisement(CapabilityAdvertisement),
    HealthUpdate(HealthMetrics),
    TaskStatus(TaskStatusUpdate),
    Heartbeat,
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatusUpdate {
    pub task_id: Uuid,
    pub agent_id: Uuid,
    pub status: TaskStatus,
    pub progress_percent: Option<f32>,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

pub struct InterAgentChannel {
    messages: Arc<RwLock<Vec<AgentMessage>>>,
    subscriptions: Arc<RwLock<FxHashMap<Uuid, Vec<MessageSubscription>>>>,
    registry: AgentRegistry,
}

struct MessageSubscription {
    subscriber_id: Uuid,
    message_types: Vec<String>,
    callback_url: Option<String>,
}

impl InterAgentChannel {
    pub fn new(registry: AgentRegistry) -> Self {
        Self {
            messages: Arc::new(RwLock::new(Vec::new())),
            subscriptions: Arc::new(RwLock::new(FxHashMap::default())),
            registry,
        }
    }

    pub async fn send_message(&self, message: AgentMessage) -> Result<(), InterAgentError> {
        if let Some(recipient_id) = message.recipient_id {
            let subscriptions = self.subscriptions.read().await;
            if let Some(agent_subscriptions) = subscriptions.get(&recipient_id) {
                let message_type = message_type_name(&message.message_type);
                let should_deliver = agent_subscriptions.iter().any(|sub| {
                    let _has_callback = sub.callback_url.is_some();
                    sub.subscriber_id == recipient_id
                        && sub
                            .message_types
                            .iter()
                            .any(|t| t.eq_ignore_ascii_case(message_type))
                });
                if !should_deliver {
                    return Ok(());
                }
            }
        }

        let mut messages = self.messages.write().await;
        messages.push(message);

        self.cleanup_expired_messages(&mut messages).await;

        Ok(())
    }

    pub async fn broadcast(&self, message: AgentMessage) -> Result<(), InterAgentError> {
        let mut messages = self.messages.write().await;
        messages.push(message);
        self.cleanup_expired_messages(&mut messages).await;
        Ok(())
    }

    pub async fn subscribe(
        &self,
        subscriber_id: Uuid,
        message_types: Vec<String>,
        callback_url: Option<String>,
    ) {
        let subscription = MessageSubscription {
            subscriber_id,
            message_types,
            callback_url,
        };

        let mut subscriptions = self.subscriptions.write().await;
        subscriptions
            .entry(subscriber_id)
            .or_insert_with(Vec::new)
            .push(subscription);
    }

    pub async fn unsubscribe(&self, subscriber_id: Uuid) {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.remove(&subscriber_id);
    }

    pub async fn get_messages_for_agent(&self, agent_id: &Uuid) -> Vec<AgentMessage> {
        let messages = self.messages.read().await;
        messages
            .iter()
            .filter(|m| m.recipient_id.map_or(true, |r| &r == agent_id) || m.recipient_id.is_none())
            .cloned()
            .collect()
    }

    pub async fn find_agents_by_capability(&self, capability_name: &str) -> Vec<AgentInfo> {
        let capability_name = capability_name.to_lowercase();
        let agents = self.registry.list().await;
        agents
            .into_iter()
            .filter(|a| {
                a.capabilities
                    .iter()
                    .any(|c| c.to_lowercase().contains(&capability_name))
            })
            .collect()
    }

    pub async fn find_agents_by_health(&self, min_status: AgentStatus) -> Vec<AgentInfo> {
        let agents = self.registry.list().await;
        let status_order = |s: &AgentStatus| match s {
            AgentStatus::Active => 0,
            AgentStatus::Idle => 1,
            AgentStatus::Busy => 2,
            AgentStatus::Offline => 3,
        };

        agents
            .into_iter()
            .filter(|a| status_order(&a.status) <= status_order(&min_status))
            .collect()
    }

    pub async fn find_available_agent(&self, capability_name: &str) -> Option<AgentInfo> {
        let capability_name = capability_name.to_lowercase();
        let agents = self.registry.list().await;
        agents
            .into_iter()
            .filter(|a| a.status == AgentStatus::Active || a.status == AgentStatus::Idle)
            .filter(|a| {
                a.capabilities
                    .iter()
                    .any(|c| c.to_lowercase().contains(&capability_name))
            })
            .min_by_key(|a| match a.status {
                AgentStatus::Idle => 0,
                AgentStatus::Active => 1,
                _ => 2,
            })
    }

    async fn cleanup_expired_messages(&self, messages: &mut Vec<AgentMessage>) {
        let now = chrono::Utc::now();
        messages.retain(|m| {
            let age = now - m.timestamp;
            age.num_seconds() < m.ttl_seconds as i64
        });
    }
}

fn message_type_name(message_type: &MessageType) -> &'static str {
    match message_type {
        MessageType::Delegation(_) => "delegation",
        MessageType::DelegationResponse(_) => "delegation_response",
        MessageType::CapabilityAdvertisement(_) => "capability_advertisement",
        MessageType::HealthUpdate(_) => "health_update",
        MessageType::TaskStatus(_) => "task_status",
        MessageType::Heartbeat => "heartbeat",
        MessageType::Shutdown => "shutdown",
    }
}

#[derive(Debug, Clone)]
pub struct InterAgentError {
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for InterAgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for InterAgentError {}

pub struct MultiAgentCoordinator {
    channel: InterAgentChannel,
    local_agent_id: Uuid,
}

impl MultiAgentCoordinator {
    pub fn new(registry: AgentRegistry, local_agent_id: Uuid) -> Self {
        Self {
            channel: InterAgentChannel::new(registry),
            local_agent_id,
        }
    }

    pub async fn delegate_task(&self, request: DelegationRequest) -> Result<Uuid, InterAgentError> {
        let message = AgentMessage {
            id: Uuid::new_v4(),
            sender_id: self.local_agent_id,
            recipient_id: None,
            message_type: MessageType::Delegation(request),
            payload: serde_json::Value::Null,
            timestamp: chrono::Utc::now(),
            ttl_seconds: 300,
        };
        let message_id = message.id;

        self.channel
            .send_message(message)
            .await
            .map_err(|e| InterAgentError {
                code: "SEND_FAILED".to_string(),
                message: e.to_string(),
            })?;

        Ok(message_id)
    }

    pub async fn advertise_capabilities(
        &self,
        capabilities: Vec<AgentCapability>,
    ) -> Result<(), InterAgentError> {
        let advertisement = CapabilityAdvertisement {
            agent_id: self.local_agent_id,
            capabilities,
            advertised_at: chrono::Utc::now(),
        };

        let message = AgentMessage {
            id: Uuid::new_v4(),
            sender_id: self.local_agent_id,
            recipient_id: None,
            message_type: MessageType::CapabilityAdvertisement(advertisement),
            payload: serde_json::Value::Null,
            timestamp: chrono::Utc::now(),
            ttl_seconds: 600,
        };

        self.channel
            .broadcast(message)
            .await
            .map_err(|e| InterAgentError {
                code: "BROADCAST_FAILED".to_string(),
                message: e.to_string(),
            })
    }

    pub async fn update_health(&self, metrics: HealthMetrics) -> Result<(), InterAgentError> {
        let message = AgentMessage {
            id: Uuid::new_v4(),
            sender_id: self.local_agent_id,
            recipient_id: None,
            message_type: MessageType::HealthUpdate(metrics),
            payload: serde_json::Value::Null,
            timestamp: chrono::Utc::now(),
            ttl_seconds: 60,
        };

        self.channel
            .broadcast(message)
            .await
            .map_err(|e| InterAgentError {
                code: "BROADCAST_FAILED".to_string(),
                message: e.to_string(),
            })
    }

    pub async fn get_pending_messages(&self) -> Vec<AgentMessage> {
        self.channel
            .get_messages_for_agent(&self.local_agent_id)
            .await
    }

    pub async fn find_agent_for_task(&self, capability_required: &str) -> Option<AgentInfo> {
        self.channel.find_available_agent(capability_required).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_metrics_new() {
        let id = Uuid::new_v4();
        let metrics = HealthMetrics::new(id);
        assert_eq!(metrics.agent_id, id);
        assert_eq!(metrics.tasks_completed, 0);
        assert_eq!(metrics.health_status, HealthStatus::Unknown);
    }

    #[test]
    fn test_health_metrics_update_load() {
        let mut metrics = HealthMetrics::new(Uuid::new_v4());
        metrics.update_load(0.8);
        assert_eq!(metrics.current_load, 0.8);
    }

    #[test]
    fn test_health_metrics_record_completion() {
        let mut metrics = HealthMetrics::new(Uuid::new_v4());
        metrics.record_completion();
        assert_eq!(metrics.tasks_completed, 1);
        assert_eq!(metrics.health_status, HealthStatus::Healthy);
    }

    #[test]
    fn test_health_metrics_record_failure() {
        let mut metrics = HealthMetrics::new(Uuid::new_v4());
        metrics.tasks_completed = 9;
        metrics.record_failure();
        metrics.record_failure();
        assert_eq!(metrics.tasks_failed, 2);
        assert_eq!(
            metrics.health_status,
            HealthStatus::Degraded,
            "11%% failure rate → Degraded"
        );
    }

    #[tokio::test]
    async fn test_inter_agent_channel_send() {
        let registry = AgentRegistry::new();
        let channel = InterAgentChannel::new(registry);

        let message = AgentMessage {
            id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            recipient_id: Some(Uuid::new_v4()),
            message_type: MessageType::Heartbeat,
            payload: serde_json::json!({}),
            timestamp: chrono::Utc::now(),
            ttl_seconds: 60,
        };

        let result = channel.send_message(message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_inter_agent_channel_subscribe() {
        let registry = AgentRegistry::new();
        let channel = InterAgentChannel::new(registry);

        channel
            .subscribe(Uuid::new_v4(), vec!["Heartbeat".to_string()], None)
            .await;
    }

    #[tokio::test]
    async fn test_subscribed_message_type_is_delivered() {
        let registry = AgentRegistry::new();
        let channel = InterAgentChannel::new(registry);
        let recipient = Uuid::new_v4();

        channel
            .subscribe(recipient, vec!["heartbeat".to_string()], None)
            .await;

        let message = AgentMessage {
            id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            recipient_id: Some(recipient),
            message_type: MessageType::Heartbeat,
            payload: serde_json::json!({}),
            timestamp: chrono::Utc::now(),
            ttl_seconds: 60,
        };

        channel.send_message(message).await.unwrap();
        let messages = channel.get_messages_for_agent(&recipient).await;
        assert_eq!(messages.len(), 1);
    }

    #[tokio::test]
    async fn test_unsubscribed_message_type_is_not_delivered() {
        let registry = AgentRegistry::new();
        let channel = InterAgentChannel::new(registry);
        let recipient = Uuid::new_v4();

        channel
            .subscribe(recipient, vec!["heartbeat".to_string()], None)
            .await;

        let message = AgentMessage {
            id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            recipient_id: Some(recipient),
            message_type: MessageType::Shutdown,
            payload: serde_json::json!({}),
            timestamp: chrono::Utc::now(),
            ttl_seconds: 60,
        };

        channel.send_message(message).await.unwrap();
        let messages = channel.get_messages_for_agent(&recipient).await;
        assert_eq!(messages.len(), 0);
    }

    #[tokio::test]
    async fn test_find_available_agent_case_insensitive_capability_match() {
        let registry = AgentRegistry::new();
        let agent = AgentInfo {
            id: Uuid::new_v4(),
            name: "agent".to_string(),
            capabilities: vec!["WebScan".to_string()],
            status: AgentStatus::Idle,
            last_heartbeat: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            callback_url: None,
        };
        registry.register(agent.clone()).await;

        let channel = InterAgentChannel::new(registry);
        let found = channel.find_available_agent("webscan").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, agent.id);
    }

    #[tokio::test]
    async fn test_multi_agent_coordinator_delegate() {
        let registry = AgentRegistry::new();
        let coordinator = MultiAgentCoordinator::new(registry, Uuid::new_v4());

        let request = DelegationRequest {
            id: Uuid::new_v4(),
            task_type: "scan".to_string(),
            target: "https://example.com".to_string(),
            parameters: serde_json::json!({}),
            callback_url: None,
        };

        let result = coordinator.delegate_task(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_multi_agent_coordinator_delegate_returns_message_id() {
        let registry = AgentRegistry::new();
        let local_id = Uuid::new_v4();
        let coordinator = MultiAgentCoordinator::new(registry, local_id);

        let request = DelegationRequest {
            id: Uuid::new_v4(),
            task_type: "scan".to_string(),
            target: "https://example.com".to_string(),
            parameters: serde_json::json!({}),
            callback_url: None,
        };

        let returned_id = coordinator.delegate_task(request).await.unwrap();
        let messages = coordinator.get_pending_messages().await;
        let delegated = messages
            .iter()
            .find(|m| matches!(m.message_type, MessageType::Delegation(_)))
            .expect("delegation message should exist");

        assert_eq!(returned_id, delegated.id);
    }

    #[tokio::test]
    async fn test_broadcast_cleans_up_expired_messages() {
        let registry = AgentRegistry::new();
        let channel = InterAgentChannel::new(registry);
        let now = chrono::Utc::now();

        let expired = AgentMessage {
            id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            recipient_id: None,
            message_type: MessageType::Heartbeat,
            payload: serde_json::Value::Null,
            timestamp: now - chrono::Duration::seconds(120),
            ttl_seconds: 1,
        };
        let expired_id = expired.id;
        channel.broadcast(expired).await.unwrap();

        let fresh = AgentMessage {
            id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            recipient_id: None,
            message_type: MessageType::Heartbeat,
            payload: serde_json::Value::Null,
            timestamp: now,
            ttl_seconds: 60,
        };
        channel.broadcast(fresh.clone()).await.unwrap();

        let all = channel.get_messages_for_agent(&Uuid::new_v4()).await;
        assert!(all.iter().any(|m| m.id == fresh.id));
        assert!(all.iter().all(|m| m.id != expired_id));
    }
}
