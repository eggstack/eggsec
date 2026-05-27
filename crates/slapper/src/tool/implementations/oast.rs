//! OAST (Out-of-Band Application Security Testing) integration.
//!
//! This module provides blind vulnerability detection using out-of-band
//! techniques via the Interactsh API.

use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::SlapperError;
use crate::tool::traits::{SecurityTool, ToolCapability, ToolCategory};
use crate::tool::{ToolRequest, ToolResponse, ToolResult};

const INTERACTSH_URL: &str = "https://interactsh.com";
const DEFAULT_POLL_INTERVAL_MS: u64 = 2000;
const DEFAULT_TIMEOUT_SECS: u64 = 300;

#[derive(Clone)]
pub struct OastTool {
    client: Client,
    interactions: Arc<RwLock<Vec<Interaction>>>,
    server_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    pub id: String,
    pub timestamp: chrono::DateTime<Utc>,
    pub interaction_type: String,
    pub full_url: String,
    pub remote_address: Option<String>,
    pub raw_request: Option<String>,
    pub payload: Option<String>,
}

impl OastTool {
    pub fn new() -> Self {
        let client = Client::builder()
            .pool_max_idle_per_host(20)
            .pool_idle_timeout(Duration::from_secs(30))
            .tcp_nodelay(true)
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            interactions: Arc::new(RwLock::new(Vec::new())),
            server_url: INTERACTSH_URL.to_string(),
        }
    }

    pub fn with_server_url(server_url: &str) -> Self {
        let mut tool = Self::new();
        tool.server_url = server_url.to_string();
        tool
    }

    pub async fn register_session(&self) -> Result<String, SlapperError> {
        let response = self
            .client
            .get(&format!("{}/register", self.server_url))
            .send()
            .await
            .map_err(|e| SlapperError::Network(e.to_string()))?;

        let body = response
            .text()
            .await
            .map_err(|e| SlapperError::Network(e.to_string()))?;

        Ok(body.trim().to_string())
    }

    pub async fn poll_interactions(
        &self,
        session_id: &str,
    ) -> Result<Vec<Interaction>, SlapperError> {
        let response = self
            .client
            .get(&format!(
                "{}/poll?id={}&token={}",
                self.server_url, session_id, session_id
            ))
            .send()
            .await
            .map_err(|e| SlapperError::Network(e.to_string()))?;

        let body = response
            .text()
            .await
            .map_err(|e| SlapperError::Network(e.to_string()))?;

        if body.trim().is_empty() || body.trim() == "null" {
            return Ok(Vec::new());
        }

        let interactions: Vec<InteractshInteraction> =
            serde_json::from_str(&body).map_err(|e| SlapperError::Parse(e.to_string()))?;

        let parsed: Vec<Interaction> = interactions
            .into_iter()
            .map(|i| Interaction {
                id: Uuid::new_v4().to_string(),
                timestamp: chrono::Utc::now(),
                interaction_type: i
                    .interactions
                    .first()
                    .map(|x| x.type_field.clone())
                    .unwrap_or_default(),
                full_url: i
                    .interactions
                    .first()
                    .map(|x| x.full_url.clone())
                    .unwrap_or_default(),
                remote_address: i
                    .interactions
                    .first()
                    .and_then(|x| x.remote_address.clone()),
                raw_request: i.interactions.first().and_then(|x| x.raw_request.clone()),
                payload: None,
            })
            .collect();

        let mut interactions_guard = self.interactions.write().await;
        interactions_guard.extend(parsed.clone());

        Ok(parsed)
    }

    pub async fn get_all_interactions(&self) -> Vec<Interaction> {
        self.interactions.read().await.clone()
    }

    pub async fn clear_interactions(&self) {
        self.interactions.write().await.clear();
    }

    pub fn generate_oast_url(&self, session_id: &str, interaction_id: &str) -> String {
        format!("{}/{}", session_id.replace("-", ""), interaction_id)
    }

    pub async fn correlate_interactions(&self, payloads: &[String]) -> Vec<(String, Interaction)> {
        let interactions = self.interactions.read().await;
        let mut correlations = Vec::new();

        for interaction in interactions.iter() {
            for payload in payloads {
                if interaction.full_url.contains(payload) {
                    correlations.push((payload.clone(), interaction.clone()));
                }
            }
        }

        correlations
    }
}

#[derive(Debug, Deserialize)]
struct InteractshInteraction {
    #[serde(rename = "interactions")]
    interactions: Vec<InteractshInteractionData>,
}

#[derive(Debug, Deserialize)]
struct InteractshInteractionData {
    #[serde(rename = "type")]
    #[serde(alias = "type")]
    type_field: String,
    #[serde(rename = "full-url")]
    full_url: String,
    #[serde(rename = "remote-address")]
    remote_address: Option<String>,
    #[serde(rename = "raw-request")]
    raw_request: Option<String>,
}

impl Default for OastTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecurityTool for OastTool {
    fn id(&self) -> &'static str {
        "oast"
    }

    fn name(&self) -> &'static str {
        "OAST Scanner"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Scanning
    }

    fn description(&self) -> &'static str {
        "Out-of-Band Application Security Testing for blind vulnerability detection"
    }

    async fn execute(&self, request: ToolRequest) -> ToolResult<ToolResponse> {
        let started_at = Utc::now();

        let params = &request.params;
        let timeout_ms = params
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_TIMEOUT_SECS * 1000);
        let poll_interval_ms = params
            .get("poll_interval_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_POLL_INTERVAL_MS);
        let payloads: Vec<String> = params
            .get("payloads")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let session_id = match self.register_session().await {
            Ok(id) => id,
            Err(e) => {
                return Ok(ToolResponse {
                    request_id: request.id,
                    tool_id: "oast".to_string(),
                    status: crate::tool::ResponseStatus::Failed,
                    results: serde_json::json!({}),
                    metadata: crate::tool::ResponseMetadata {
                        started_at,
                        completed_at: Utc::now(),
                        duration_ms: 0,
                        targets_scanned: 0,
                        findings_count: 0,
                    },
                    errors: vec![crate::tool::ToolError::new("OAST_ERROR", e.to_string())],
                    findings: vec![],
                });
            }
        };
        let interaction_id = Uuid::new_v4().to_string();

        let oast_url = self.generate_oast_url(&session_id, &interaction_id);

        let mut findings = Vec::new();

        for payload in &payloads {
            if payload.contains("$OAST$") {
                let modified_payload = payload.replace("$OAST$", &oast_url);
                let mut metadata = FxHashMap::default();
                metadata.insert("oast_url".to_string(), serde_json::json!(oast_url));
                metadata.insert("session_id".to_string(), serde_json::json!(session_id));
                metadata.insert("payload".to_string(), serde_json::json!(payload));
                findings.push(crate::tool::response::Finding {
                    id: Uuid::new_v4().to_string(),
                    finding_type: crate::tool::response::FindingType::Vulnerability,
                    severity: crate::tool::response::ResponseSeverity::High,
                    title: "Potential SSRF via OAST".to_string(),
                    description: format!(
                        "OAST payload generated: {}. Waiting for interaction...",
                        modified_payload
                    ),
                    location: oast_url.clone(),
                    evidence: None,
                    cve_ids: vec![],
                    remediation: Some("Validate and sanitize all user-supplied URLs".to_string()),
                    references: vec![],
                    metadata,
                });
            }
        }

        let poll_duration = Duration::from_millis(poll_interval_ms);
        let max_polls = (timeout_ms / poll_interval_ms) as usize;
        let mut poll_count = 0;

        while poll_count < max_polls {
            tokio::time::sleep(poll_duration).await;

            match self.poll_interactions(&session_id).await {
                Ok(interactions) => {
                    for interaction in &interactions {
                        for payload in &payloads {
                            if interaction.full_url.contains(payload) {
                                let mut metadata = FxHashMap::default();
                                metadata.insert(
                                    "interaction".to_string(),
                                    serde_json::json!(interaction.full_url),
                                );
                                metadata.insert(
                                    "triggered_payload".to_string(),
                                    serde_json::json!(payload),
                                );
                                findings.push(crate::tool::response::Finding {
                                    id: Uuid::new_v4().to_string(),
                                    finding_type: crate::tool::response::FindingType::Vulnerability,
                                    severity: crate::tool::response::ResponseSeverity::Critical,
                                    title: "Blind SSRF Confirmed via OAST".to_string(),
                                    description: format!(
                                        "Out-of-band interaction detected. The server made a request to {} triggered by payload in URL parameter.",
                                        interaction.full_url
                                    ),
                                    location: interaction.full_url.clone(),
                                    evidence: None,
                                    cve_ids: vec![],
                                    remediation: Some("Validate and sanitize all user-supplied URLs".to_string()),
                                    references: vec![],
                                    metadata,
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("OAST poll failed: {}", e);
                }
            }

            poll_count += 1;
        }

        let completed_at = Utc::now();
        let duration_ms = (completed_at - started_at).num_milliseconds() as u64;

        Ok(ToolResponse {
            request_id: request.id,
            tool_id: "oast".to_string(),
            status: crate::tool::ResponseStatus::Success,
            results: serde_json::json!({
                "session_id": session_id,
                "oast_url": oast_url,
                "interactions_count": self.interactions.read().await.len(),
            }),
            metadata: crate::tool::ResponseMetadata {
                started_at,
                completed_at,
                duration_ms,
                targets_scanned: 1,
                findings_count: findings.len(),
            },
            errors: vec![],
            findings,
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability {
            name: "ssrf_detection".to_string(),
            description: "Detect Server-Side Request Forgery via OAST".to_string(),
            parameters: vec![],
            examples: vec![],
            attack_surface: vec![crate::tool::traits::AttackSurface::Internal],
            severity_potential: vec![crate::output::AgentSeverity::Critical],
            prerequisites: vec![],
            estimated_duration_ms: 60000,
        }]
    }

    fn validate(&self, request: &ToolRequest) -> ToolResult<()> {
        if request.target.value.is_empty() {
            return Err(SlapperError::Validation("Target is required".to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oast_url_generation() {
        let tool = OastTool::new();
        let url = tool.generate_oast_url("abc123def456", "interaction-1");
        assert!(url.contains("abc123def456"));
        assert!(url.contains("interaction-1"));
    }

    #[test]
    fn test_oast_tool_id() {
        let tool = OastTool::new();
        assert_eq!(tool.id(), "oast");
    }

    #[test]
    fn test_oast_tool_name() {
        let tool = OastTool::new();
        assert_eq!(tool.name(), "OAST Scanner");
    }

    #[tokio::test]
    async fn test_oast_interactions_storage() {
        let tool = OastTool::new();
        assert!(tool.get_all_interactions().await.is_empty());

        tool.clear_interactions().await;
        assert!(tool.get_all_interactions().await.is_empty());
    }
}
