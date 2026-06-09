use crate::cli::ScanProfile;
use crate::config::EggsecConfig;
use crate::error::Result;
use serde::{Deserialize, Serialize};

use super::context::PipelineContext;
use super::stage::Stage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSession {
    pub target: String,
    pub profile: ScanProfile,
    pub completed_stages: Vec<Stage>,
    pub remaining_stages: Vec<Stage>,
    pub context: PipelineContext,
    pub spoof_config: crate::scanner::spoof::SpoofConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrent_stages: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<EggsecConfig>,
}

pub async fn save(path: &str, session: &PipelineSession) -> Result<()> {
    use tokio::io::AsyncWriteExt;
    let json = serde_json::to_string_pretty(session)?;
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .await?;
    file.write_all(json.as_bytes()).await?;
    Ok(())
}

pub async fn load(path: &str) -> Result<PipelineSession> {
    let json = tokio::fs::read_to_string(path).await?;
    let session: PipelineSession = serde_json::from_str(&json)?;
    Ok(session)
}
