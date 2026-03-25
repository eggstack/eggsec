use serde::Deserialize;
use std::collections::HashMap;

use super::types::ModuleOption;

#[derive(Debug, Clone, Deserialize)]
pub struct ModuleInfo {
    pub name: String,
    pub fullname: String,
    pub aliases: Option<Vec<String>>,
    pub description: String,
    pub authors: Vec<String>,
    pub references: Vec<Vec<String>>,
    pub platform: Option<Vec<String>>,
    pub arch: Option<Vec<String>>,
    pub rank: Option<String>,
    pub privileged: Option<bool>,
    pub license: Option<String>,
    pub disclosure_date: Option<String>,
    pub default_target: Option<i32>,
    pub targets: Option<Vec<Target>>,
    pub options: HashMap<String, ModuleOption>,
    pub advanced_options: Option<HashMap<String, AdvancedOption>>,
    pub notes: Option<HashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Target {
    pub index: i32,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdvancedOption {
    pub default: Option<String>,
    pub desc: Option<String>,
    pub name: String,
    #[serde(rename = "type")]
    pub opt_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModuleExecutionResult {
    pub job_id: Option<String>,
    pub uuid: Option<String>,
    pub result: Option<String>,
    #[serde(default)]
    pub sessions: Vec<i32>,
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub message: String,
}

impl ModuleInfo {
    pub fn required_options(&self) -> Vec<(&str, &ModuleOption)> {
        self.options
            .iter()
            .filter(|(_, opt)| opt.required)
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    pub fn is_platform(&self, platform: &str) -> bool {
        self.platform
            .as_ref()
            .map(|p| {
                p.iter()
                    .any(|p| p.to_lowercase() == platform.to_lowercase())
            })
            .unwrap_or(false)
    }

    pub fn is_arch(&self, arch: &str) -> bool {
        self.arch
            .as_ref()
            .map(|a| a.iter().any(|a| a.to_lowercase() == arch.to_lowercase()))
            .unwrap_or(false)
    }
}
