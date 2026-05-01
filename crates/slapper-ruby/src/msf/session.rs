use serde::{Deserialize, Serialize};

use super::types::SessionType;

#[derive(Debug, Clone, Deserialize)]
pub struct Session {
    #[serde(default)]
    pub id: String,

    #[serde(rename = "type")]
    pub session_type: SessionType,

    #[serde(default)]
    pub tunnel_local: Option<String>,

    #[serde(default)]
    pub tunnel_peer: Option<String>,

    #[serde(default)]
    pub via_exploit: Option<String>,

    #[serde(default)]
    pub via_payload: Option<String>,

    #[serde(default)]
    pub desc: Option<String>,

    #[serde(default)]
    pub info: Option<String>,

    #[serde(default)]
    pub workspace: Option<String>,

    #[serde(default)]
    pub session_host: Option<String>,

    #[serde(default)]
    pub session_port: Option<i32>,

    #[serde(default)]
    pub target_host: Option<String>,

    #[serde(default)]
    pub username: Option<String>,

    #[serde(default)]
    pub uuid: Option<String>,

    #[serde(default)]
    pub exploit_uuid: Option<String>,

    #[serde(default)]
    pub routes: Option<String>,

    #[serde(default)]
    pub platform: Option<String>,

    #[serde(default)]
    pub arch: Option<String>,

    #[serde(default)]
    pub last_checkin: Option<String>,

    #[serde(default)]
    pub is_connected: Option<bool>,

    #[serde(default)]
    pub exploit_name: Option<String>,

    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub session_type: SessionType,
    pub platform: String,
    pub arch: String,
    pub connected: bool,
    pub uptime_secs: u64,
}

impl Session {
    pub fn host(&self) -> Option<&str> {
        self.session_host.as_deref().or(self.target_host.as_deref())
    }

    pub fn port(&self) -> Option<u16> {
        self.session_port.map(|p| p as u16)
    }

    pub fn platform(&self) -> Option<&str> {
        self.platform.as_deref()
    }

    pub fn arch(&self) -> Option<&str> {
        self.arch.as_deref()
    }

    pub fn is_meterpreter(&self) -> bool {
        matches!(self.session_type, SessionType::Meterpreter)
    }

    pub fn is_shell(&self) -> bool {
        matches!(self.session_type, SessionType::Shell)
    }

    pub fn exploit(&self) -> Option<&str> {
        self.via_exploit.as_deref()
    }

    pub fn payload(&self) -> Option<&str> {
        self.via_payload.as_deref()
    }

    pub fn to_info(&self, id: &str) -> SessionInfo {
        let uptime_secs = self.calculate_uptime();

        SessionInfo {
            id: id.to_string(),
            host: self.host().unwrap_or("unknown").to_string(),
            port: self.port().unwrap_or(0),
            session_type: self.session_type,
            platform: self.platform().unwrap_or("unknown").to_string(),
            arch: self.arch().unwrap_or("unknown").to_string(),
            connected: self.is_connected.unwrap_or(false),
            uptime_secs,
        }
    }

    pub fn calculate_uptime(&self) -> u64 {
        if let Some(ref last_checkin) = self.last_checkin {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(last_checkin) {
                let now = chrono::Utc::now();
                let duration = now.signed_duration_since(dt.with_timezone(&chrono::Utc));
                return duration.num_seconds() as u64;
            }
        }

        if let Some(ref created_at) = self.created_at {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(created_at) {
                let now = chrono::Utc::now();
                let duration = now.signed_duration_since(dt.with_timezone(&chrono::Utc));
                return duration.num_seconds() as u64;
            }
        }

        0
    }
}
