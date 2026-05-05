use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::agent::alerts::AlertRoutingRules;
use crate::agent::portfolio::OffPeakWindow;
use crate::types::Severity;

pub mod checker;
pub use checker::{ConstraintChecker, ConstraintViolation};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoNotDoList {
    pub forbidden_actions: Vec<ForbiddenAction>,
    pub forbidden_targets: Vec<String>,
    pub forbidden_payloads: Vec<String>,
}

impl DoNotDoList {
    pub fn new() -> Self {
        Self {
            forbidden_actions: Vec::new(),
            forbidden_targets: Vec::new(),
            forbidden_payloads: Vec::new(),
        }
    }

    pub fn add_forbidden_action(&mut self, action: ForbiddenAction) {
        self.forbidden_actions.push(action);
    }

    pub fn add_forbidden_target(&mut self, target: impl Into<String>) {
        self.forbidden_targets.push(target.into());
    }

    pub fn add_forbidden_payload(&mut self, payload: impl Into<String>) {
        self.forbidden_payloads.push(payload.into());
    }

    pub fn is_action_allowed(&self, action_type: &str, target: &str) -> bool {
        for action in &self.forbidden_actions {
            if action.matches(action_type, target) {
                return false;
            }
        }
        true
    }

    pub fn is_target_allowed(&self, target: &str) -> bool {
        !self.forbidden_targets.iter().any(|t| {
            if t.starts_with('*') {
                target.contains(&t[1..])
            } else if t.ends_with('*') {
                target.starts_with(&t[..t.len() - 1])
            } else {
                target == t || target.starts_with(t)
            }
        })
    }

    pub fn is_payload_allowed(&self, payload: &str) -> bool {
        !self.forbidden_payloads.iter().any(|p| payload.contains(p))
    }
}

impl Default for DoNotDoList {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForbiddenAction {
    pub action_type: String,
    pub target_pattern: Option<String>,
    pub reason: String,
    pub severity: Option<Severity>,
}

impl ForbiddenAction {
    pub fn new(action_type: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            action_type: action_type.into(),
            target_pattern: None,
            reason: reason.into(),
            severity: None,
        }
    }

    pub fn with_target(mut self, target_pattern: impl Into<String>) -> Self {
        self.target_pattern = Some(target_pattern.into());
        self
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = Some(severity);
        self
    }

    pub fn matches(&self, action_type: &str, target: &str) -> bool {
        if self.action_type != action_type {
            return false;
        }

        if let Some(ref pattern) = self.target_pattern {
            if pattern.starts_with('*') {
                target.contains(&pattern[1..])
            } else {
                target == pattern || target.starts_with(pattern)
            }
        } else {
            true
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffPeakConfig {
    pub windows: Vec<OffPeakWindow>,
    pub allowed_scan_depths: Vec<crate::agent::portfolio::ScanDepth>,
    pub max_requests_per_hour: Option<usize>,
    pub timezone: String,
}

impl OffPeakConfig {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            allowed_scan_depths: vec![crate::agent::portfolio::ScanDepth::Shallow],
            max_requests_per_hour: None,
            timezone: "UTC".to_string(),
        }
    }

    pub fn with_window(mut self, window: OffPeakWindow) -> Self {
        self.windows.push(window);
        self
    }

    pub fn with_max_requests_per_hour(mut self, max: usize) -> Self {
        self.max_requests_per_hour = Some(max);
        self
    }

    pub fn is_in_any_window(&self, time: &DateTime<Utc>) -> bool {
        self.windows.iter().any(|w| w.is_in_window(time))
    }

    pub fn get_allowed_depth(
        &self,
        requested: crate::agent::portfolio::ScanDepth,
    ) -> crate::agent::portfolio::ScanDepth {
        if self.allowed_scan_depths.contains(&requested) {
            requested
        } else {
            crate::agent::portfolio::ScanDepth::Shallow
        }
    }
}

impl Default for OffPeakConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationalConstraints {
    pub off_peak_config: OffPeakConfig,
    pub alert_routing: AlertRoutingRules,
    pub do_not_do_list: DoNotDoList,
    pub rate_limit_budget: Option<usize>,
    pub require_approval_for: Vec<String>,
}

impl OperationalConstraints {
    pub fn new() -> Self {
        Self {
            off_peak_config: OffPeakConfig::new(),
            alert_routing: AlertRoutingRules::new(),
            do_not_do_list: DoNotDoList::new(),
            rate_limit_budget: None,
            require_approval_for: Vec::new(),
        }
    }

    pub fn with_off_peak_config(mut self, config: OffPeakConfig) -> Self {
        self.off_peak_config = config;
        self
    }

    pub fn with_alert_routing(mut self, routing: AlertRoutingRules) -> Self {
        self.alert_routing = routing;
        self
    }

    pub fn with_do_not_do_list(mut self, list: DoNotDoList) -> Self {
        self.do_not_do_list = list;
        self
    }

    pub fn is_action_allowed(&self, action_type: &str, target: &str) -> bool {
        self.do_not_do_list.is_action_allowed(action_type, target)
            && self.do_not_do_list.is_target_allowed(target)
    }

    pub fn requires_approval(&self, action_type: &str) -> bool {
        self.require_approval_for
            .iter()
            .any(|a| action_type.contains(a))
    }

    pub fn get_off_peak_config(&self) -> &OffPeakConfig {
        &self.off_peak_config
    }

    pub fn get_alert_routing(&self) -> &AlertRoutingRules {
        &self.alert_routing
    }

    pub fn get_do_not_do_list(&self) -> &DoNotDoList {
        &self.do_not_do_list
    }
}

impl Default for OperationalConstraints {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_do_not_do_list_creation() {
        let list = DoNotDoList::new();
        assert!(list.forbidden_actions.is_empty());
        assert!(list.forbidden_targets.is_empty());
        assert!(list.forbidden_payloads.is_empty());
    }

    #[test]
    fn test_do_not_do_list_add_forbidden_action() {
        let mut list = DoNotDoList::new();
        list.add_forbidden_action(ForbiddenAction::new("scan", "Not allowed"));
        assert_eq!(list.forbidden_actions.len(), 1);
    }

    #[test]
    fn test_do_not_do_list_is_action_allowed() {
        let mut list = DoNotDoList::new();
        list.add_forbidden_action(
            ForbiddenAction::new("scan", "Not allowed").with_target("*.local"),
        );
        assert!(!list.is_action_allowed("scan", "test.local"));
        assert!(list.is_action_allowed("scan", "example.com"));
    }

    #[test]
    fn test_do_not_do_list_is_target_allowed() {
        let mut list = DoNotDoList::new();
        list.add_forbidden_target("192.168.*");
        list.add_forbidden_target("*.local");

        assert!(!list.is_target_allowed("192.168.1.1"));
        assert!(!list.is_target_allowed("test.local"));
        assert!(list.is_target_allowed("example.com"));
    }

    #[test]
    fn test_do_not_do_list_is_payload_allowed() {
        let mut list = DoNotDoList::new();
        list.add_forbidden_payload("rm -rf");
        list.add_forbidden_payload("drop table");

        assert!(!list.is_payload_allowed("rm -rf /"));
        assert!(!list.is_payload_allowed("'; drop table users;--"));
        assert!(list.is_payload_allowed("' OR 1=1"));
    }

    #[test]
    fn test_forbidden_action_matches() {
        let action = ForbiddenAction::new("exploit", "Dangerous")
            .with_target("*.gov")
            .with_severity(Severity::Critical);

        assert!(action.matches("exploit", "test.gov"));
        assert!(action.matches("exploit", "admin.gov"));
        assert!(!action.matches("scan", "test.gov"));
        assert!(!action.matches("exploit", "example.com"));
    }

    #[test]
    fn test_off_peak_config_creation() {
        let config = OffPeakConfig::new();
        assert!(config.windows.is_empty());
        assert!(config.max_requests_per_hour.is_none());
    }

    #[test]
    fn test_off_peak_config_with_window() {
        let config = OffPeakConfig::new().with_window(OffPeakWindow {
            start_hour: 0,
            end_hour: 6,
            timezone: "UTC".to_string(),
        });

        assert_eq!(config.windows.len(), 1);
    }

    #[test]
    fn test_off_peak_config_get_allowed_depth() {
        let config = OffPeakConfig::new().with_window(OffPeakWindow {
            start_hour: 0,
            end_hour: 6,
            timezone: "UTC".to_string(),
        });

        let allowed = config.get_allowed_depth(crate::agent::portfolio::ScanDepth::Shallow);
        assert_eq!(allowed, crate::agent::portfolio::ScanDepth::Shallow);

        let deep = config.get_allowed_depth(crate::agent::portfolio::ScanDepth::Deep);
        assert_eq!(deep, crate::agent::portfolio::ScanDepth::Shallow);
    }

    #[test]
    fn test_operational_constraints_creation() {
        let constraints = OperationalConstraints::new();
        assert!(constraints.rate_limit_budget.is_none());
        assert!(constraints.require_approval_for.is_empty());
    }

    #[test]
    fn test_operational_constraints_is_action_allowed() {
        let mut constraints = OperationalConstraints::new();
        constraints
            .do_not_do_list
            .add_forbidden_action(ForbiddenAction::new("destructive", "Too dangerous"));

        assert!(!constraints.is_action_allowed("destructive", "test.com"));
        assert!(constraints.is_action_allowed("scan", "test.com"));
    }

    #[test]
    fn test_operational_constraints_requires_approval() {
        let mut constraints = OperationalConstraints::new();
        constraints.require_approval_for = vec!["exploit".to_string(), "destructive".to_string()];

        assert!(constraints.requires_approval("run_exploit"));
        assert!(constraints.requires_approval("destructive_scan"));
        assert!(!constraints.requires_approval("normal_scan"));
    }

    #[test]
    fn test_operational_constraints_off_peak_config() {
        let constraints = OperationalConstraints::new();
        let config = constraints.get_off_peak_config();
        assert!(config.windows.is_empty());
    }

    #[test]
    fn test_operational_constraints_alert_routing() {
        let constraints = OperationalConstraints::new();
        let routing = constraints.get_alert_routing();
        assert!(routing.by_severity.is_empty());
    }

    #[test]
    fn test_operational_constraints_do_not_do_list() {
        let constraints = OperationalConstraints::new();
        let list = constraints.get_do_not_do_list();
        assert!(list.forbidden_actions.is_empty());
    }
}
