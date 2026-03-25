use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::config::Scope;
use crate::output::agent::AgentFinding;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub session_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub context: ScanContext,
    pub findings: Vec<AgentFinding>,
    pub scope: Option<Scope>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanContext {
    pub target: Option<String>,
    pub target_type: Option<String>,
    pub scan_type: Option<String>,
    pub stages_completed: Vec<String>,
    pub discovered_endpoints: Vec<String>,
    pub discovered_technologies: Vec<String>,
    pub open_ports: Vec<u16>,
    pub authenticated: bool,
    pub auth_type: Option<String>,
    pub waf_detected: Option<String>,
    pub last_activity: Option<DateTime<Utc>>,
}

impl Default for AgentSession {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            session_id: Uuid::new_v4().to_string(),
            created_at: now,
            updated_at: now,
            context: ScanContext::default(),
            findings: Vec::new(),
            scope: None,
            metadata: HashMap::new(),
            status: SessionStatus::Active,
        }
    }
}

impl AgentSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_scope(scope: Scope) -> Self {
        Self {
            scope: Some(scope),
            ..Default::default()
        }
    }

    pub fn with_target(target: impl Into<String>) -> Self {
        let mut session = Self::default();
        session.context.target = Some(target.into());
        session
    }

    pub fn update_activity(&mut self) {
        self.updated_at = Utc::now();
        self.context.last_activity = Some(Utc::now());
    }

    pub fn add_finding(&mut self, finding: AgentFinding) {
        self.update_activity();
        self.findings.push(finding);
    }

    pub fn add_findings(&mut self, findings: Vec<AgentFinding>) {
        self.update_activity();
        self.findings.extend(findings);
    }

    pub fn complete_stage(&mut self, stage: impl Into<String>) {
        self.update_activity();
        let stage_str = stage.into();
        if !self.context.stages_completed.contains(&stage_str) {
            self.context.stages_completed.push(stage_str);
        }
    }

    pub fn is_expired(&self, ttl_seconds: i64) -> bool {
        let ttl = chrono::Duration::seconds(ttl_seconds);
        Utc::now() - self.updated_at > ttl
    }

    pub fn severity_summary(&self) -> HashMap<String, usize> {
        let mut summary = HashMap::new();
        for finding in &self.findings {
            let severity = format!("{:?}", finding.severity).to_lowercase();
            *summary.entry(severity).or_insert(0) += 1;
        }
        summary
    }
}

#[derive(Debug, Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, AgentSession>>>,
    storage_path: Arc<PathBuf>,
    default_ttl_seconds: i64,
    max_sessions: usize,
}

impl SessionManager {
    pub fn new(storage_path: PathBuf) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            storage_path: Arc::new(storage_path),
            default_ttl_seconds: 3600,
            max_sessions: 100,
        }
    }

    pub fn with_ttl(mut self, ttl_seconds: i64) -> Self {
        self.default_ttl_seconds = ttl_seconds;
        self
    }

    pub fn with_max_sessions(mut self, max: usize) -> Self {
        self.max_sessions = max;
        self
    }

    pub async fn init(&self) -> Result<()> {
        if !self.storage_path.exists() {
            fs::create_dir_all(self.storage_path.as_path())?;
        }
        
        self.load_from_disk().await?;
        self.cleanup_expired().await?;
        
        Ok(())
    }

    async fn load_from_disk(&self) -> Result<()> {
        let storage_path = self.storage_path.clone();
        let entries = fs::read_dir(storage_path.as_path())?;
        
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(session) = serde_json::from_str::<AgentSession>(&content) {
                        if !session.is_expired(self.default_ttl_seconds) {
                            self.sessions.write().await.insert(
                                session.session_id.clone(),
                                session,
                            );
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    async fn save_to_disk(&self, session: &AgentSession) -> Result<()> {
        let filename = format!("{}.json", session.session_id);
        let path = self.storage_path.join(filename);
        let content = serde_json::to_string_pretty(session)?;
        fs::write(path, content)?;
        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<()> {
        let storage_path = self.storage_path.clone();
        let mut sessions = self.sessions.write().await;
        let expired: Vec<String> = sessions
            .iter()
            .filter(|(_, s)| s.is_expired(self.default_ttl_seconds))
            .map(|(id, _)| id.clone())
            .collect();

        for id in &expired {
            sessions.remove(id);
            let path = storage_path.join(format!("{}.json", id));
            let _ = fs::remove_file(path);
        }

        Ok(())
    }

    pub async fn create_session(&self) -> Result<AgentSession> {
        let mut sessions = self.sessions.write().await;
        
        if sessions.len() >= self.max_sessions {
            self.cleanup_expired().await?;
            
            if sessions.len() >= self.max_sessions {
                anyhow::bail!("Maximum session limit reached");
            }
        }

        let session = AgentSession::new();
        let session_clone = session.clone();
        sessions.insert(session.session_id.clone(), session.clone());
        
        drop(sessions);
        self.save_to_disk(&session_clone).await?;
        
        Ok(session_clone)
    }

    pub async fn get_session(&self, session_id: &str) -> Option<AgentSession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    pub async fn update_session(&self, session: &AgentSession) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.session_id.clone(), session.clone());
        drop(sessions);
        self.save_to_disk(session).await
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        let storage_path = self.storage_path.clone();
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
        
        let path = storage_path.join(format!("{}.json", session_id));
        if path.exists() {
            fs::remove_file(path)?;
        }
        
        Ok(())
    }

    pub async fn list_sessions(&self) -> Vec<AgentSession> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }

    pub async fn list_active_session_ids(&self) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions
            .iter()
            .filter(|(_, s)| matches!(s.status, SessionStatus::Active))
            .map(|(id, _)| id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_session_creation() {
        let dir = tempdir().unwrap();
        let manager = SessionManager::new(dir.path().to_path_buf());
        
        let session = manager.create_session().await.unwrap();
        assert!(!session.session_id.is_empty());
        assert!(matches!(session.status, SessionStatus::Active));
    }

    #[tokio::test]
    async fn test_session_persistence() {
        let dir = tempdir().unwrap();
        let manager = SessionManager::new(dir.path().to_path_buf());
        manager.init().await.unwrap();
        
        let session = manager.create_session().await.unwrap();
        let session_id = session.session_id.clone();
        
        let loaded = manager.get_session(&session_id).await.unwrap();
        assert_eq!(loaded.session_id, session_id);
    }

    #[tokio::test]
    async fn test_session_update() {
        let dir = tempdir().unwrap();
        let manager = SessionManager::new(dir.path().to_path_buf());
        manager.init().await.unwrap();
        
        let mut session = manager.create_session().await.unwrap();
        session.context.target = Some("https://example.com".to_string());
        
        manager.update_session(&session).await.unwrap();
        
        let loaded = manager.get_session(&session.session_id).await.unwrap();
        assert_eq!(loaded.context.target, Some("https://example.com".to_string()));
    }

    #[tokio::test]
    async fn test_session_deletion() {
        let dir = tempdir().unwrap();
        let manager = SessionManager::new(dir.path().to_path_buf());
        manager.init().await.unwrap();
        
        let session = manager.create_session().await.unwrap();
        let session_id = session.session_id.clone();
        
        manager.delete_session(&session_id).await.unwrap();
        
        let loaded = manager.get_session(&session_id).await;
        assert!(loaded.is_none());
    }
}
