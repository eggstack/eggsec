use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Active,
    Idle,
    Busy,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: Uuid,
    pub name: String,
    pub capabilities: Vec<String>,
    pub status: AgentStatus,
    pub last_heartbeat: u64,
    pub callback_url: Option<String>,
}

#[derive(Clone)]
pub struct AgentRegistry {
    agents: Arc<RwLock<HashMap<Uuid, AgentInfo>>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, agent: AgentInfo) {
        self.agents.write().await.insert(agent.id, agent);
    }

    pub async fn unregister(&self, id: Uuid) {
        self.agents.write().await.remove(&id);
    }

    pub async fn list(&self) -> Vec<AgentInfo> {
        self.agents.read().await.values().cloned().collect()
    }

    pub async fn get(&self, id: Uuid) -> Option<AgentInfo> {
        self.agents.read().await.get(&id).cloned()
    }

    pub async fn heartbeat(&self, id: Uuid) {
        if let Some(agent) = self.agents.write().await.get_mut(&id) {
            agent.last_heartbeat = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }
    }

    pub async fn update_status(&self, id: Uuid, status: AgentStatus) {
        if let Some(agent) = self.agents.write().await.get_mut(&id) {
            agent.status = status;
        }
    }

    pub async fn find_by_capability(&self, capability: &str) -> Vec<AgentInfo> {
        let capability_lower = capability.to_lowercase();
        self.agents
            .read()
            .await
            .values()
            .filter(|agent| {
                agent
                    .capabilities
                    .iter()
                    .any(|c| c.to_lowercase().contains(&capability_lower))
            })
            .cloned()
            .collect()
    }

    pub async fn find_by_status(&self, status: AgentStatus) -> Vec<AgentInfo> {
        self.agents
            .read()
            .await
            .values()
            .filter(|agent| agent.status == status)
            .cloned()
            .collect()
    }

    pub async fn list_active(&self) -> Vec<AgentInfo> {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(60);
        self.agents
            .read()
            .await
            .values()
            .filter(|agent| agent.last_heartbeat >= cutoff)
            .cloned()
            .collect()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
