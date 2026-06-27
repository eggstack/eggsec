use crate::agent::constraints::{
    DoNotDoList, ForbiddenAction, OffPeakConfig, OperationalConstraints,
};
use crate::agent::portfolio::{ScanDepth, TargetConfig};
use crate::types::Severity;
use chrono::Timelike;
use rustc_hash::FxHashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const RATE_LIMIT_RESET_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintViolation {
    ActionForbidden {
        action: String,
        target: String,
        reason: String,
    },
    TargetForbidden {
        target: String,
        reason: String,
    },
    PayloadForbidden {
        payload: String,
        reason: String,
    },
    OutsideOffPeakWindow {
        current_hour: i32,
        allowed_windows: String,
    },
    ScanDepthNotAllowed {
        requested: ScanDepth,
        allowed: ScanDepth,
    },
    RateLimitExceeded {
        current: usize,
        limit: usize,
    },
    ApprovalRequired {
        action: String,
    },
}

impl ConstraintViolation {
    pub fn severity(&self) -> Severity {
        match self {
            ConstraintViolation::ActionForbidden { .. } => Severity::High,
            ConstraintViolation::TargetForbidden { .. } => Severity::High,
            ConstraintViolation::PayloadForbidden { .. } => Severity::Medium,
            ConstraintViolation::OutsideOffPeakWindow { .. } => Severity::Low,
            ConstraintViolation::ScanDepthNotAllowed { .. } => Severity::Low,
            ConstraintViolation::RateLimitExceeded { .. } => Severity::Medium,
            ConstraintViolation::ApprovalRequired { .. } => Severity::Medium,
        }
    }

    pub fn message(&self) -> String {
        match self {
            ConstraintViolation::ActionForbidden {
                action,
                target,
                reason,
            } => {
                format!(
                    "Action '{}' on '{}' is forbidden: {}",
                    action, target, reason
                )
            }
            ConstraintViolation::TargetForbidden { target, reason } => {
                format!("Target '{}' is forbidden: {}", target, reason)
            }
            ConstraintViolation::PayloadForbidden { payload, reason } => {
                format!("Payload '{}' is forbidden: {}", payload, reason)
            }
            ConstraintViolation::OutsideOffPeakWindow {
                current_hour,
                allowed_windows,
            } => {
                format!(
                    "Current hour {} is outside allowed off-peak windows: {}",
                    current_hour, allowed_windows
                )
            }
            ConstraintViolation::ScanDepthNotAllowed { requested, allowed } => {
                format!(
                    "Scan depth {:?} not allowed, falling back to {:?}",
                    requested, allowed
                )
            }
            ConstraintViolation::RateLimitExceeded { current, limit } => {
                format!(
                    "Rate limit exceeded: {} requests vs limit of {}",
                    current, limit
                )
            }
            ConstraintViolation::ApprovalRequired { action } => {
                format!("Action '{}' requires approval before execution", action)
            }
        }
    }
}

pub struct ConstraintChecker {
    constraints: OperationalConstraints,
    request_counts: Arc<Mutex<FxHashMap<String, usize>>>,
    last_reset_at: Arc<Mutex<Instant>>,
}

impl ConstraintChecker {
    pub fn new(constraints: OperationalConstraints) -> Self {
        Self {
            constraints,
            request_counts: Arc::new(Mutex::new(FxHashMap::default())),
            last_reset_at: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub fn with_constraints(constraints: OperationalConstraints) -> Self {
        Self::new(constraints)
    }

    pub fn evaluate_action(
        &self,
        action_type: &str,
        target: &str,
    ) -> Result<(), ConstraintViolation> {
        if !self.constraints.is_action_allowed(action_type, target) {
            let reason = self
                .constraints
                .do_not_do_list
                .forbidden_actions
                .iter()
                .find(|a| a.action_type == action_type)
                .map(|a| a.reason.clone())
                .unwrap_or_else(|| "Action is in do-not-do list".to_string());

            return Err(ConstraintViolation::ActionForbidden {
                action: action_type.to_string(),
                target: target.to_string(),
                reason,
            });
        }
        Ok(())
    }

    pub fn evaluate_target(&self, target: &str) -> Result<(), ConstraintViolation> {
        if !self.constraints.do_not_do_list.is_target_allowed(target) {
            return Err(ConstraintViolation::TargetForbidden {
                target: target.to_string(),
                reason: "Target is in forbidden list".to_string(),
            });
        }
        Ok(())
    }

    pub fn evaluate_payload(&self, payload: &str) -> Result<(), ConstraintViolation> {
        if !self.constraints.do_not_do_list.is_payload_allowed(payload) {
            return Err(ConstraintViolation::PayloadForbidden {
                payload: payload.chars().take(50).collect(),
                reason: "Payload contains forbidden patterns".to_string(),
            });
        }
        Ok(())
    }

    pub fn evaluate_off_peak(&self, hour: i32) -> Result<(), ConstraintViolation> {
        let off_peak = &self.constraints.off_peak_config;

        if off_peak.windows.is_empty() {
            return Ok(());
        }

        let in_window = off_peak.windows.iter().any(|w| {
            let start = w.start_hour as i32;
            let end = w.end_hour as i32;
            if start <= end {
                hour >= start && hour < end
            } else {
                hour >= start || hour < end
            }
        });

        if !in_window {
            let windows_str = off_peak
                .windows
                .iter()
                .map(|w| format!("{:02}:00-{:02}:00", w.start_hour, w.end_hour))
                .collect::<Vec<_>>()
                .join(", ");

            return Err(ConstraintViolation::OutsideOffPeakWindow {
                current_hour: hour,
                allowed_windows: windows_str,
            });
        }

        Ok(())
    }

    pub fn evaluate_scan_depth(
        &self,
        requested: ScanDepth,
    ) -> Result<ScanDepth, ConstraintViolation> {
        let off_peak = &self.constraints.off_peak_config;

        if off_peak.allowed_scan_depths.contains(&requested) {
            return Ok(requested);
        }

        let allowed = if off_peak.allowed_scan_depths.is_empty() {
            ScanDepth::Shallow
        } else {
            off_peak.allowed_scan_depths[0]
        };

        Err(ConstraintViolation::ScanDepthNotAllowed { requested, allowed })
    }

    pub fn evaluate_rate_limit(&self, key: &str) -> Result<(), ConstraintViolation> {
        if let Some(limit) = self.constraints.rate_limit_budget {
            let mut request_counts = self.request_counts.lock().unwrap_or_else(|e| e.into_inner());
            let mut last_reset = self.last_reset_at.lock().unwrap_or_else(|e| e.into_inner());
            if last_reset.elapsed() >= RATE_LIMIT_RESET_INTERVAL {
                request_counts.clear();
                *last_reset = Instant::now();
            }
            let current = request_counts.entry(key.to_string()).or_insert(0);
            if *current >= limit {
                return Err(ConstraintViolation::RateLimitExceeded {
                    current: *current,
                    limit,
                });
            }
            *current += 1;
        }
        Ok(())
    }

    pub fn evaluate_approval(&self, action_type: &str) -> Result<(), ConstraintViolation> {
        if self.constraints.requires_approval(action_type) {
            return Err(ConstraintViolation::ApprovalRequired {
                action: action_type.to_string(),
            });
        }
        Ok(())
    }

    pub fn evaluate_all(
        &self,
        action_type: &str,
        target: &str,
        payload: Option<&str>,
    ) -> Vec<ConstraintViolation> {
        let mut violations = Vec::new();

        if let Err(v) = self.evaluate_action(action_type, target) {
            violations.push(v);
        }

        if let Err(v) = self.evaluate_target(target) {
            violations.push(v);
        }

        if let Some(p) = payload {
            if let Err(v) = self.evaluate_payload(p) {
                violations.push(v);
            }
        }

        if let Err(v) = self.evaluate_approval(action_type) {
            violations.push(v);
        }

        violations
    }

    pub fn check_target_config(&self, config: &TargetConfig) -> Vec<ConstraintViolation> {
        let mut violations = Vec::new();

        if let Err(v) = self.evaluate_target(&config.target) {
            violations.push(v);
        }

        if let Some(ref window) = config.off_peak_window {
            let now = chrono::Utc::now();
            if !window.is_in_window(&now) {
                violations.push(ConstraintViolation::OutsideOffPeakWindow {
                    current_hour: now.hour() as i32,
                    allowed_windows: format!(
                        "{:02}:00-{:02}:00",
                        window.start_hour, window.end_hour
                    ),
                });
            }
        }

        violations
    }

    pub fn reset_rate_limits(&self) {
        let mut request_counts = self.request_counts.lock().unwrap_or_else(|e| e.into_inner());
        request_counts.clear();
    }

    pub fn get_constraints(&self) -> &OperationalConstraints {
        &self.constraints
    }
}

impl Default for ConstraintChecker {
    fn default() -> Self {
        Self::new(OperationalConstraints::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::constraints::{
        DoNotDoList, ForbiddenAction, OffPeakConfig, OperationalConstraints,
    };
    use crate::agent::portfolio::{OffPeakWindow, ScanDepth};

    fn create_test_constraints() -> OperationalConstraints {
        let mut do_not_do = DoNotDoList::new();
        do_not_do.add_forbidden_action(
            ForbiddenAction::new("destructive", "Cannot run destructive scans")
                .with_severity(Severity::Critical),
        );
        do_not_do.add_forbidden_target("192.168.*");
        do_not_do.add_forbidden_payload("rm -rf");

        let mut off_peak = OffPeakConfig::new();
        off_peak.windows.push(OffPeakWindow {
            start_hour: 0,
            end_hour: 6,
            timezone: "UTC".to_string(),
        });

        OperationalConstraints {
            off_peak_config: off_peak,
            alert_routing: crate::agent::alerts::AlertRoutingRules::new(),
            do_not_do_list: do_not_do,
            rate_limit_budget: Some(100),
            require_approval_for: vec!["exploit".to_string()],
            max_concurrent_scans: None,
            per_target_cooldown_secs: None,
        }
    }

    #[test]
    fn test_evaluate_action_allowed() {
        let checker = ConstraintChecker::new(create_test_constraints());
        let result = checker.evaluate_action("scan", "https://example.com");
        assert!(result.is_ok());
    }

    #[test]
    fn test_evaluate_action_forbidden() {
        let checker = ConstraintChecker::new(create_test_constraints());
        let result = checker.evaluate_action("destructive", "https://example.com");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConstraintViolation::ActionForbidden { .. }
        ));
    }

    #[test]
    fn test_evaluate_target_forbidden() {
        let checker = ConstraintChecker::new(create_test_constraints());
        let result = checker.evaluate_target("192.168.1.1");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConstraintViolation::TargetForbidden { .. }
        ));
    }

    #[test]
    fn test_evaluate_target_allowed() {
        let checker = ConstraintChecker::new(create_test_constraints());
        let result = checker.evaluate_target("https://example.com");
        assert!(result.is_ok());
    }

    #[test]
    fn test_evaluate_payload_forbidden() {
        let checker = ConstraintChecker::new(create_test_constraints());
        let result = checker.evaluate_payload("rm -rf /");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConstraintViolation::PayloadForbidden { .. }
        ));
    }

    #[test]
    fn test_evaluate_payload_allowed() {
        let checker = ConstraintChecker::new(create_test_constraints());
        let result = checker.evaluate_payload("'; SELECT * FROM users;--");
        assert!(result.is_ok());
    }

    #[test]
    fn test_evaluate_off_peak_in_window() {
        let checker = ConstraintChecker::new(create_test_constraints());
        let result = checker.evaluate_off_peak(3);
        assert!(result.is_ok());
    }

    #[test]
    fn test_evaluate_off_peak_outside_window() {
        let checker = ConstraintChecker::new(create_test_constraints());
        let result = checker.evaluate_off_peak(12);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConstraintViolation::OutsideOffPeakWindow { .. }
        ));
    }

    #[test]
    fn test_evaluate_scan_depth_allowed() {
        let checker = ConstraintChecker::new(create_test_constraints());
        let result = checker.evaluate_scan_depth(ScanDepth::Shallow);
        assert!(result.is_ok());
    }

    #[test]
    fn test_evaluate_scan_depth_not_allowed() {
        let checker = ConstraintChecker::new(create_test_constraints());
        let result = checker.evaluate_scan_depth(ScanDepth::Deep);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConstraintViolation::ScanDepthNotAllowed { .. }
        ));
    }

    #[test]
    fn test_evaluate_rate_limit_within_budget() {
        let mut checker = ConstraintChecker::new(create_test_constraints());
        let result = checker.evaluate_rate_limit("test_target");
        assert!(result.is_ok());
    }

    #[test]
    fn test_evaluate_rate_limit_exceeded() {
        let mut checker = ConstraintChecker::new(create_test_constraints());
        for _ in 0..100 {
            checker.evaluate_rate_limit("test_target").unwrap();
        }
        let result = checker.evaluate_rate_limit("test_target");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConstraintViolation::RateLimitExceeded { .. }
        ));
    }

    #[test]
    fn test_evaluate_approval_required() {
        let checker = ConstraintChecker::new(create_test_constraints());
        let result = checker.evaluate_approval("run_exploit");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConstraintViolation::ApprovalRequired { .. }
        ));
    }

    #[test]
    fn test_evaluate_approval_not_required() {
        let checker = ConstraintChecker::new(create_test_constraints());
        let result = checker.evaluate_approval("normal_scan");
        assert!(result.is_ok());
    }

    #[test]
    fn test_evaluate_all_no_violations() {
        let checker = ConstraintChecker::new(create_test_constraints());
        let violations = checker.evaluate_all("scan", "https://example.com", Some("test"));
        assert!(violations.is_empty());
    }

    #[test]
    fn test_evaluate_all_multiple_violations() {
        let checker = ConstraintChecker::new(create_test_constraints());
        let violations = checker.evaluate_all("destructive", "192.168.1.1", None);
        assert!(violations.len() >= 2);
    }

    #[test]
    fn test_constraint_violation_severity() {
        let action_violation = ConstraintViolation::ActionForbidden {
            action: "test".to_string(),
            target: "test".to_string(),
            reason: "test".to_string(),
        };
        assert_eq!(action_violation.severity(), Severity::High);

        let payload_violation = ConstraintViolation::PayloadForbidden {
            payload: "test".to_string(),
            reason: "test".to_string(),
        };
        assert_eq!(payload_violation.severity(), Severity::Medium);
    }

    #[test]
    fn test_constraint_violation_message() {
        let violation = ConstraintViolation::ActionForbidden {
            action: "scan".to_string(),
            target: "test.com".to_string(),
            reason: "Not allowed".to_string(),
        };
        let msg = violation.message();
        assert!(msg.contains("scan"));
        assert!(msg.contains("test.com"));
        assert!(msg.contains("Not allowed"));
    }

    #[test]
    fn test_reset_rate_limits() {
        let mut checker = ConstraintChecker::new(create_test_constraints());
        for _ in 0..10 {
            checker.evaluate_rate_limit("test_target").unwrap();
        }
        checker.reset_rate_limits();
        let result = checker.evaluate_rate_limit("test_target");
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_constraints() {
        let constraints = create_test_constraints();
        let checker = ConstraintChecker::new(constraints.clone());
        let checker_constraints = checker.get_constraints();
        assert_eq!(
            checker_constraints.rate_limit_budget,
            constraints.rate_limit_budget
        );
        assert_eq!(
            checker_constraints.require_approval_for,
            constraints.require_approval_for
        );
    }
}
