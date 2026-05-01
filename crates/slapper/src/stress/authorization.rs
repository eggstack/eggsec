#![allow(dead_code)]

use crate::error::{Result, SlapperError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::config::{load_scope, Scope};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressScope {
    #[serde(default)]
    pub allow_stress_test: bool,

    #[serde(default)]
    pub max_rate_pps: Option<u64>,

    #[serde(default)]
    pub max_duration_secs: Option<u64>,

    #[serde(default)]
    pub allowed_stress_types: Option<Vec<String>>,

    #[serde(default)]
    pub require_confirmation: bool,

    #[serde(default)]
    pub warning_message: Option<String>,
}

impl Default for StressScope {
    fn default() -> Self {
        Self {
            allow_stress_test: false,
            max_rate_pps: Some(100000),
            max_duration_secs: Some(300),
            allowed_stress_types: None,
            require_confirmation: true,
            warning_message: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StressAuthorization {
    scope: Scope,
    stress_scope: StressScope,
}

impl StressAuthorization {
    pub fn from_scope() -> Result<Self> {
        let scope = load_scope(None)?;
        let stress_scope = Self::load_stress_config()?;

        Ok(Self {
            scope,
            stress_scope,
        })
    }

    fn load_stress_config() -> Result<StressScope> {
        let config_path = crate::config::config_dir()
            .map(|d| d.join("stress.toml"))
            .unwrap_or_else(|| PathBuf::from("stress.toml"));

        if !config_path.exists() {
            tracing::debug!("No stress config found, using defaults");
            return Ok(StressScope::default());
        }

        let content = std::fs::read_to_string(&config_path).map_err(|e| {
            SlapperError::Runtime(format!(
                "Failed to read stress config: {:?}: {}",
                config_path, e
            ))
        })?;

        let config: StressScope = toml::from_str(&content).map_err(|e| {
            SlapperError::Runtime(format!(
                "Failed to parse stress config: {:?}: {}",
                config_path, e
            ))
        })?;

        Ok(config)
    }

    pub fn verify_target(&self, target: &str) -> Result<()> {
        if !self
            .scope
            .is_target_allowed(target)
            .map_err(|e| SlapperError::ScopeViolation(e.to_string()))?
        {
            return Err(SlapperError::ScopeViolation(format!(
                "Target '{}' is not in allowed scope",
                target
            )));
        }

        if !self.stress_scope.allow_stress_test {
            return Err(SlapperError::ScopeViolation(
                "Stress testing is not enabled for any targets. \
                 Add 'allow_stress_test = true' to your scope file for authorized targets."
                    .to_string(),
            ));
        }

        tracing::info!(
            target = %target,
            "Target authorized for stress testing"
        );

        Ok(())
    }

    pub fn verify_rate(&self, rate_pps: u64) -> Result<()> {
        if let Some(max_rate) = self.stress_scope.max_rate_pps {
            if rate_pps > max_rate {
                return Err(SlapperError::Validation(format!(
                    "Requested rate {} pps exceeds maximum allowed rate {} pps",
                    rate_pps, max_rate
                )));
            }
        }
        Ok(())
    }

    pub fn verify_duration(&self, duration_secs: u64) -> Result<()> {
        if let Some(max_duration) = self.stress_scope.max_duration_secs {
            if duration_secs > max_duration {
                return Err(SlapperError::Validation(format!(
                    "Requested duration {}s exceeds maximum allowed duration {}s",
                    duration_secs, max_duration
                )));
            }
        }
        Ok(())
    }

    pub fn verify_stress_type(&self, stress_type: &str) -> Result<()> {
        if let Some(ref allowed_types) = self.stress_scope.allowed_stress_types {
            let type_lower = stress_type.to_lowercase();
            if !allowed_types.iter().any(|t| t.to_lowercase() == type_lower) {
                return Err(SlapperError::Validation(format!(
                    "Stress type '{}' is not in allowed types: {:?}",
                    stress_type, allowed_types
                )));
            }
        }
        Ok(())
    }

    pub fn requires_confirmation(&self) -> bool {
        self.stress_scope.require_confirmation
    }

    pub fn get_warning_message(&self) -> Option<&str> {
        self.stress_scope.warning_message.as_deref()
    }

    pub fn max_rate(&self) -> Option<u64> {
        self.stress_scope.max_rate_pps
    }

    pub fn max_duration(&self) -> Option<u64> {
        self.stress_scope.max_duration_secs
    }
}

pub fn create_example_stress_config() -> String {
    toml::to_string_pretty(&StressScope {
        allow_stress_test: true,
        max_rate_pps: Some(50000),
        max_duration_secs: Some(300),
        allowed_stress_types: Some(vec![
            "syn".to_string(),
            "udp".to_string(),
            "http".to_string(),
        ]),
        require_confirmation: true,
        warning_message: Some(
            "Authorized penetration testing only. Unauthorized use is illegal.".to_string(),
        ),
    })
    .unwrap_or_default()
}
