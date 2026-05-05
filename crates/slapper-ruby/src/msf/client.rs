use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use super::module::{ModuleExecutionResult, ModuleInfo};
use super::session::{Session, SessionInfo};
use super::types::{ModuleType, MsfError, MsfResponse};
use super::{MsfConfig, MsfConnection};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionCache {
    pub sessions: HashMap<String, SessionInfo>,
    pub last_updated: Option<String>,
}

pub struct MsfClient {
    client: Client,
    config: MsfConfig,
    connection: Option<MsfConnection>,
    session_cache: SessionCache,
}

impl MsfClient {
    pub fn new(config: MsfConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .danger_accept_invalid_certs(!config.verify_ssl)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            config,
            connection: None,
            session_cache: SessionCache::default(),
        })
    }

    pub fn from_url(url: &str) -> Result<Self> {
        Self::new(MsfConfig {
            url: url.to_string(),
            ..Default::default()
        })
    }

    pub async fn connect(&mut self) -> Result<()> {
        if let Some(ref token) = self.config.token {
            self.connection = Some(MsfConnection::new(self.config.url.clone(), token.clone()));
            return Ok(());
        }

        let username = self
            .config
            .username
            .as_ref()
            .ok_or_else(|| anyhow!("Username required for authentication"))?;
        let password = self
            .config
            .password
            .as_ref()
            .ok_or_else(|| anyhow!("Password required for authentication"))?;

        let response = self
            .client
            .post(format!("{}/api/v1/auth/login", self.config.url))
            .json(&serde_json::json!({
                "username": username,
                "password": password,
            }))
            .send()
            .await
            .context("Failed to connect to MSF RPC server")?;

        if !response.status().is_success() {
            anyhow::bail!("MSF RPC authentication failed: {}", response.status());
        }

        let result: MsfResponse = response
            .json()
            .await
            .context("Failed to parse MSF response")?;

        let token = result
            .token
            .ok_or_else(|| anyhow!("No token in MSF response"))?;

        self.connection = Some(MsfConnection::new(self.config.url.clone(), token));

        tracing::info!("Connected to MSF RPC at {}", self.config.url);

        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(ref conn) = self.connection {
            self.client
                .post(format!("{}/api/v1/auth/logout", conn.url))
                .json(&serde_json::json!({
                    "token": conn.token,
                }))
                .send()
                .await?;

            self.connection = None;
        }

        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    pub fn connection(&self) -> Option<&MsfConnection> {
        self.connection.as_ref()
    }

    pub fn get_cached_sessions(&self) -> &HashMap<String, SessionInfo> {
        &self.session_cache.sessions
    }

    pub fn get_session_cache(&self) -> &SessionCache {
        &self.session_cache
    }

    pub fn invalidate_session_cache(&mut self) {
        self.session_cache = SessionCache::default();
    }

    pub fn persist_session_cache(&self, path: &PathBuf) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.session_cache)
            .context("Failed to serialize session cache")?;
        std::fs::write(path, json).context("Failed to write session cache to file")?;
        tracing::info!("Session cache persisted to {:?}", path);
        Ok(())
    }

    pub fn load_session_cache(&mut self, path: &PathBuf) -> Result<()> {
        if !path.exists() {
            tracing::debug!("No session cache file found at {:?}", path);
            return Ok(());
        }
        let content = std::fs::read_to_string(path).context("Failed to read session cache file")?;
        let cache: SessionCache =
            serde_json::from_str(&content).context("Failed to parse session cache")?;
        tracing::info!("Loaded {} sessions from cache", cache.sessions.len());
        self.session_cache = cache;
        Ok(())
    }

    async fn request<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow!("Not connected to MSF RPC"))?;

        let response = self
            .client
            .post(format!("{}/api/v1/{}", conn.url, method))
            .json(&serde_json::json!({
                "token": conn.token,
                "params": params,
            }))
            .send()
            .await
            .context("MSF RPC request failed")?;

        let text = response.text().await?;

        if let Ok(error) = serde_json::from_str::<MsfError>(&text) {
            anyhow::bail!("MSF error: {}", error.error_message);
        }

        serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse MSF response: {}", text))
    }

    pub async fn get_version(&self) -> Result<MsfVersion> {
        self.request("core.version", serde_json::json!({})).await
    }

    pub async fn list_modules(&self, module_type: ModuleType) -> Result<Vec<String>> {
        let (endpoint, type_str) = match module_type {
            ModuleType::Exploit => ("module.exploits", "exploit"),
            ModuleType::Auxiliary => ("module.auxiliary", "auxiliary"),
            ModuleType::Post => ("module.post", "post"),
            ModuleType::Payload => ("module.payloads", "payload"),
            ModuleType::Encoder => ("module.encoders", "encoder"),
            ModuleType::Nop => ("module.nops", "nop"),
        };

        let response: ModulesResponse = self
            .request(endpoint, serde_json::json!({ "module_type": type_str }))
            .await?;

        Ok(response.modules)
    }

    pub async fn get_module_info(&self, module_type: ModuleType, name: &str) -> Result<ModuleInfo> {
        let type_str = match module_type {
            ModuleType::Exploit => "exploit",
            ModuleType::Auxiliary => "auxiliary",
            ModuleType::Post => "post",
            ModuleType::Payload => "payload",
            ModuleType::Encoder => "encoder",
            ModuleType::Nop => "nop",
        };

        self.request(
            "module.info",
            serde_json::json!({
                "module_type": type_str,
                "module_name": name,
            }),
        )
        .await
    }

    pub async fn execute_module(
        &self,
        module_type: ModuleType,
        module_name: &str,
        options: &HashMap<String, String>,
    ) -> Result<ModuleExecutionResult> {
        let type_str = match module_type {
            ModuleType::Exploit => "exploit",
            ModuleType::Auxiliary => "auxiliary",
            ModuleType::Post => "post",
            ModuleType::Payload => "payload",
            ModuleType::Encoder => "encoder",
            ModuleType::Nop => "nop",
        };

        self.request(
            "module.execute",
            serde_json::json!({
                "module_type": type_str,
                "module_name": module_name,
                "options": options,
            }),
        )
        .await
    }

    pub async fn list_sessions(&mut self) -> Result<HashMap<String, Session>> {
        let response: SessionsResponse =
            self.request("session.list", serde_json::json!({})).await?;

        self.session_cache.sessions.clear();
        for (id, session) in &response.sessions {
            self.session_cache
                .sessions
                .insert(id.clone(), session.to_info(id));
        }
        self.session_cache.last_updated = Some(chrono::Utc::now().to_rfc3339());

        Ok(response.sessions)
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Session> {
        self.request(
            "session.info",
            serde_json::json!({ "session_id": session_id }),
        )
        .await
    }

    pub async fn execute_session_command(&self, session_id: &str, command: &str) -> Result<String> {
        let response: CommandResponse = self
            .request(
                "session.shell_write",
                serde_json::json!({
                    "session_id": session_id,
                    "command": command,
                }),
            )
            .await?;

        Ok(response.output)
    }

    pub async fn read_session_output(&self, session_id: &str) -> Result<String> {
        let response: CommandResponse = self
            .request(
                "session.shell_read",
                serde_json::json!({ "session_id": session_id }),
            )
            .await?;

        Ok(response.output)
    }

    pub async fn kill_session(&mut self, session_id: &str) -> Result<()> {
        self.request::<()>(
            "session.stop",
            serde_json::json!({ "session_id": session_id }),
        )
        .await?;

        self.session_cache.sessions.remove(session_id);

        Ok(())
    }

    pub async fn generate_payload(
        &self,
        payload_name: &str,
        options: &HashMap<String, String>,
    ) -> Result<Vec<u8>> {
        let response: PayloadResponse = self
            .request(
                "module.execute",
                serde_json::json!({
                    "module_type": "payload",
                    "module_name": payload_name,
                    "options": options,
                }),
            )
            .await?;

        let bytes = general_purpose::STANDARD.decode(&response.payload)?;

        Ok(bytes)
    }

    pub async fn get_jobs(&self) -> Result<HashMap<String, JobInfo>> {
        self.request("job.list", serde_json::json!({})).await
    }

    pub async fn stop_job(&self, job_id: &str) -> Result<()> {
        self.request("job.stop", serde_json::json!({ "job_id": job_id }))
            .await
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MsfVersion {
    pub version: String,
    pub ruby: String,
    pub api: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ModulesResponse {
    modules: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SessionsResponse {
    sessions: HashMap<String, Session>,
}

#[derive(Debug, Clone, Deserialize)]
struct CommandResponse {
    output: String,
}

#[derive(Debug, Clone, Deserialize)]
struct PayloadResponse {
    payload: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JobInfo {
    pub job_id: String,
    pub name: String,
    pub start_time: i64,
    pub datastore: HashMap<String, String>,
}
