use crate::types::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaPolicy {
    pub severity: Severity,
    pub target_hours: u32,
}

impl SlaPolicy {
    pub fn default_policies() -> Vec<SlaPolicy> {
        vec![
            SlaPolicy {
                severity: Severity::Critical,
                target_hours: 24,
            },
            SlaPolicy {
                severity: Severity::High,
                target_hours: 168,
            },
            SlaPolicy {
                severity: Severity::Medium,
                target_hours: 720,
            },
            SlaPolicy {
                severity: Severity::Low,
                target_hours: 2160,
            },
            SlaPolicy {
                severity: Severity::Info,
                target_hours: 8760,
            },
        ]
    }

    pub fn get_policy(severity: Severity) -> Self {
        let target_hours = match severity {
            Severity::Critical => 24,
            Severity::High => 168,
            Severity::Medium => 720,
            Severity::Low => 2160,
            Severity::Info => 8760,
        };
        SlaPolicy {
            severity,
            target_hours,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaStatus {
    pub finding_id: String,
    pub severity: Severity,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub due_at: chrono::DateTime<chrono::Utc>,
    pub is_violated: bool,
    pub hours_remaining: i64,
}

pub fn calculate_sla(
    finding_id: &str,
    severity: Severity,
    created_at: chrono::DateTime<chrono::Utc>,
) -> SlaStatus {
    let policy = SlaPolicy::get_policy(severity);
    let due_at = created_at + chrono::Duration::hours(policy.target_hours as i64);
    let now = chrono::Utc::now();
    let hours_remaining = (due_at - now).num_hours();

    SlaStatus {
        finding_id: finding_id.to_string(),
        severity,
        created_at,
        due_at,
        is_violated: hours_remaining < 0,
        hours_remaining,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sla_policy() {
        let policy = SlaPolicy::get_policy(Severity::Critical);
        assert_eq!(policy.target_hours, 24);
    }

    #[test]
    fn test_sla_calculation() {
        let created = chrono::Utc::now() - chrono::Duration::hours(200);
        let status = calculate_sla("finding-1", Severity::High, created);
        assert!(status.is_violated);
    }

    #[test]
    fn test_sla_policy_all_severities() {
        assert_eq!(SlaPolicy::get_policy(Severity::Critical).target_hours, 24);
        assert_eq!(SlaPolicy::get_policy(Severity::High).target_hours, 168);
        assert_eq!(SlaPolicy::get_policy(Severity::Medium).target_hours, 720);
        assert_eq!(SlaPolicy::get_policy(Severity::Low).target_hours, 2160);
        assert_eq!(SlaPolicy::get_policy(Severity::Info).target_hours, 8760);
    }

    #[test]
    fn test_sla_not_violated_when_just_created() {
        let created = chrono::Utc::now();
        let status = calculate_sla("finding-1", Severity::Critical, created);
        assert!(!status.is_violated);
        assert!(status.hours_remaining > 0);
    }

    #[test]
    fn test_sla_violated_when_past_due() {
        let created = chrono::Utc::now() - chrono::Duration::hours(25);
        let status = calculate_sla("finding-1", Severity::Critical, created);
        assert!(status.is_violated);
        assert!(status.hours_remaining < 0);
    }

    #[test]
    fn test_sla_hours_remaining_calculation() {
        let created = chrono::Utc::now() - chrono::Duration::hours(10);
        let status = calculate_sla("finding-1", Severity::Critical, created);
        assert!(!status.is_violated);
        assert!(status.hours_remaining >= 13 && status.hours_remaining <= 15);
    }

    #[test]
    fn test_default_policies_count() {
        let policies = SlaPolicy::default_policies();
        assert_eq!(policies.len(), 5);
    }
}
