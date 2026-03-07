mod bridge;
mod api;
mod loader;

pub use bridge::RubyBridge;
pub use api::SlapperApi;
pub use loader::PluginLoader;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubyPlugin {
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub path: PathBuf,
}

impl RubyPlugin {
    pub fn new(name: String, version: String, path: PathBuf) -> Self {
        Self {
            name,
            version,
            author: None,
            description: None,
            path,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubyPluginResult {
    pub success: bool,
    pub message: String,
    pub findings: Vec<RubyPluginFinding>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubyPluginFinding {
    pub severity: String,
    pub finding_type: String,
    pub description: String,
    pub location: String,
    pub evidence: Option<String>,
    pub references: Vec<String>,
}
