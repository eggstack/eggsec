mod client;
mod module;
mod payload;
mod session;
mod types;

pub use client::MsfClient;
pub use module::{ModuleExecutionResult, ModuleInfo};
pub use payload::PayloadInfo;
pub use session::{Session, SessionInfo};
pub use types::{ModuleType, MsfError, MsfResponse, SessionType};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsfConfig {
    pub url: String,
    pub token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub verify_ssl: bool,
    pub timeout_secs: u64,
}

impl Default for MsfConfig {
    fn default() -> Self {
        Self {
            url: "http://127.0.0.1:55553".to_string(),
            token: None,
            username: Some("msf".to_string()),
            password: Some("password".to_string()),
            verify_ssl: false,
            timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MsfConnection {
    pub url: String,
    pub token: String,
    pub connected_at: std::time::Instant,
}

impl MsfConnection {
    pub fn new(url: String, token: String) -> Self {
        Self {
            url,
            token,
            connected_at: std::time::Instant::now(),
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        self.connected_at.elapsed().as_secs()
    }
}
