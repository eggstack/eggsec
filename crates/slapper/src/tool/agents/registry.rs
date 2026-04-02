use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
