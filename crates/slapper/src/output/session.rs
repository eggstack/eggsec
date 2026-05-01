use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSession {
    pub version: String,
    pub created_at: String,
    pub last_modified: String,
    pub tab_states: HashMap<String, TabSessionState>,
    pub results: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabSessionState {
    pub inputs: Vec<InputFieldState>,
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputFieldState {
    pub label: String,
    pub value: String,
}

impl ScanSession {
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            created_at: chrono::Local::now().to_rfc3339(),
            last_modified: chrono::Local::now().to_rfc3339(),
            tab_states: HashMap::new(),
            results: HashMap::new(),
        }
    }

    pub fn save(&self, path: &str) -> Result<(), String> {
        let path = Path::new(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load(path: &str) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let session: ScanSession = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        Ok(session)
    }

    pub fn add_tab_state(&mut self, tab_name: &str, state: TabSessionState) {
        self.tab_states.insert(tab_name.to_string(), state);
        self.last_modified = chrono::Local::now().to_rfc3339();
    }

    pub fn add_result(&mut self, tab_name: &str, result: serde_json::Value) {
        self.results.insert(tab_name.to_string(), result);
        self.last_modified = chrono::Local::now().to_rfc3339();
    }

    pub fn list_sessions(directory: &str) -> Result<Vec<SessionInfo>, String> {
        let path = Path::new(directory);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                if let Ok(session) = Self::load(path.to_str().unwrap_or("")) {
                    sessions.push(SessionInfo {
                        path: path.to_string_lossy().to_string(),
                        created_at: session.created_at,
                        last_modified: session.last_modified,
                        tabs: session.tab_states.keys().cloned().collect(),
                    });
                }
            }
        }
        sessions.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
        Ok(sessions)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub path: String,
    pub created_at: String,
    pub last_modified: String,
    pub tabs: Vec<String>,
}

impl Default for ScanSession {
    fn default() -> Self {
        Self::new()
    }
}
