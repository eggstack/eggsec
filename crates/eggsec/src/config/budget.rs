use serde::{Deserialize, Serialize};

/// Budget constraints for an execution run.
///
/// Ensures stress, load, and raw-packet operations always have finite limits.
/// At least one bound beyond duration must be specified.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionBudget {
    pub max_duration_secs: u64,
    pub max_requests: Option<u64>,
    pub max_packets: Option<u64>,
    pub max_bytes: Option<u64>,
    pub max_concurrency: usize,
    pub max_targets: usize,
    pub max_resolved_addresses_per_host: usize,
    pub max_payloads: Option<usize>,
    pub cooldown_secs: Option<u64>,
    pub per_target_rate_limit: Option<u32>,
}

impl Default for ExecutionBudget {
    fn default() -> Self {
        Self {
            max_duration_secs: 300,
            max_requests: Some(10_000),
            max_packets: None,
            max_bytes: None,
            max_concurrency: 10,
            max_targets: 1,
            max_resolved_addresses_per_host: 10,
            max_payloads: Some(500),
            cooldown_secs: None,
            per_target_rate_limit: None,
        }
    }
}

impl ExecutionBudget {
    /// Validate that the budget has at least one finite bound beyond duration.
    pub fn validate(&self) -> Result<(), BudgetError> {
        if self.max_duration_secs == 0 {
            return Err(BudgetError::ZeroDuration);
        }
        if self.max_requests.is_none()
            && self.max_packets.is_none()
            && self.max_bytes.is_none()
            && self.max_payloads.is_none()
        {
            return Err(BudgetError::NoFiniteBound);
        }
        if self.max_concurrency == 0 {
            return Err(BudgetError::ZeroConcurrency);
        }
        Ok(())
    }

    /// Create a conservative budget for defense-lab operations.
    pub fn defense_lab_default() -> Self {
        Self {
            max_duration_secs: 300,
            max_requests: Some(10_000),
            max_packets: None,
            max_bytes: None,
            max_concurrency: 10,
            max_targets: 1,
            max_resolved_addresses_per_host: 10,
            max_payloads: Some(500),
            cooldown_secs: None,
            per_target_rate_limit: None,
        }
    }

    /// Create a conservative budget for hazardous-lab operations.
    pub fn hazardous_lab_default() -> Self {
        Self {
            max_duration_secs: 120,
            max_requests: Some(5_000),
            max_packets: Some(10_000),
            max_bytes: Some(10 * 1024 * 1024),
            max_concurrency: 5,
            max_targets: 1,
            max_resolved_addresses_per_host: 5,
            max_payloads: Some(100),
            cooldown_secs: Some(10),
            per_target_rate_limit: Some(100),
        }
    }

    /// Create a budget from a defense-lab preset.
    pub fn from_preset(preset: &super::presets::DefenseLabPreset) -> Self {
        Self {
            max_duration_secs: preset.max_duration_secs,
            max_requests: preset.max_requests,
            max_packets: preset.max_packets,
            max_bytes: None,
            max_concurrency: preset.default_concurrency,
            max_targets: 1,
            max_resolved_addresses_per_host: 10,
            max_payloads: preset.max_payloads,
            cooldown_secs: None,
            per_target_rate_limit: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BudgetError {
    ZeroDuration,
    NoFiniteBound,
    ZeroConcurrency,
}

impl std::fmt::Display for BudgetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroDuration => write!(f, "budget duration must be greater than zero"),
            Self::NoFiniteBound => write!(
                f,
                "budget must have at least one finite bound (requests, packets, bytes, or payloads) in addition to duration"
            ),
            Self::ZeroConcurrency => write!(f, "concurrency must be greater than zero"),
        }
    }
}

impl std::error::Error for BudgetError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DefenseLabPreset;

    #[test]
    fn default_budget_is_valid() {
        let budget = ExecutionBudget::default();
        assert!(budget.validate().is_ok());
    }

    #[test]
    fn budget_without_any_bound_fails() {
        let budget = ExecutionBudget {
            max_requests: None,
            max_packets: None,
            max_bytes: None,
            max_payloads: None,
            ..Default::default()
        };
        assert_eq!(budget.validate(), Err(BudgetError::NoFiniteBound));
    }

    #[test]
    fn budget_with_zero_duration_fails() {
        let budget = ExecutionBudget {
            max_duration_secs: 0,
            max_requests: Some(100),
            ..Default::default()
        };
        assert_eq!(budget.validate(), Err(BudgetError::ZeroDuration));
    }

    #[test]
    fn budget_with_zero_concurrency_fails() {
        let budget = ExecutionBudget {
            max_concurrency: 0,
            max_requests: Some(100),
            ..Default::default()
        };
        assert_eq!(budget.validate(), Err(BudgetError::ZeroConcurrency));
    }

    #[test]
    fn defense_lab_default_budget() {
        let budget = ExecutionBudget::defense_lab_default();
        assert!(budget.validate().is_ok());
        assert!(budget.max_requests.is_some());
    }

    #[test]
    fn hazardous_lab_default_budget() {
        let budget = ExecutionBudget::hazardous_lab_default();
        assert!(budget.validate().is_ok());
        assert!(budget.max_packets.is_some());
        assert!(budget.max_bytes.is_some());
    }

    #[test]
    fn from_preset() {
        let preset = DefenseLabPreset::synvoid_local();
        let budget = ExecutionBudget::from_preset(&preset);
        assert!(budget.validate().is_ok());
        assert_eq!(budget.max_duration_secs, 300);
    }

    #[test]
    fn budget_serialization_roundtrip() {
        let budget = ExecutionBudget::defense_lab_default();
        let json = serde_json::to_string(&budget).unwrap();
        let deserialized: ExecutionBudget = serde_json::from_str(&json).unwrap();
        assert_eq!(budget.max_duration_secs, deserialized.max_duration_secs);
        assert_eq!(budget.max_requests, deserialized.max_requests);
    }

    #[test]
    fn budget_error_display() {
        assert_eq!(
            BudgetError::ZeroDuration.to_string(),
            "budget duration must be greater than zero"
        );
        assert!(BudgetError::NoFiniteBound
            .to_string()
            .contains("finite bound"));
        assert_eq!(
            BudgetError::ZeroConcurrency.to_string(),
            "concurrency must be greater than zero"
        );
    }
}
