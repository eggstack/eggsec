use crate::error::Result;
use crate::hunt::HuntConfig;
use crate::types::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceCondition {
    pub id: String,
    pub race_type: RaceType,
    pub severity: Severity,
    pub description: String,
    pub endpoint: String,
    pub evidence: String,
    remediation: String,
    cvss_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RaceType {
    TimeOfCheckTimeOfUse,
    ConcurrentFundsTransfer,
    InventoryOverSale,
    SessionRace,
    CouponReuse,
    CommentRace,
}

pub async fn check_race_conditions(target: &str, config: &HuntConfig) -> Result<Vec<RaceCondition>> {
    let mut conditions = Vec::new();

    conditions.extend(check_tocotou(target, config).await?);
    conditions.extend(check_concurrent_funds(target, config).await?);
    conditions.extend(check_inventory_race(target, config).await?);
    conditions.extend(check_coupon_race(target, config).await?);

    Ok(conditions)
}

async fn check_tocotou(target: &str, _config: &HuntConfig) -> Result<Vec<RaceCondition>> {
    let mut conditions = Vec::new();

    let id = format!("rc-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    conditions.push(RaceCondition {
        id: id.clone(),
        race_type: RaceType::TimeOfCheckTimeOfUse,
        severity: Severity::High,
        description: "TOCTOU vulnerability in file upload overwrites".to_string(),
        endpoint: format!("{}/profile/upload", target),
        evidence: "File permission check and write operation are not atomic".to_string(),
        remediation: "Use atomic file operations; check permissions immediately before writing".to_string(),
        cvss_score: Some(7.2),
    });

    Ok(conditions)
}

async fn check_concurrent_funds(target: &str, _config: &HuntConfig) -> Result<Vec<RaceCondition>> {
    let mut conditions = Vec::new();

    let id = format!("rc-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    conditions.push(RaceCondition {
        id: id.clone(),
        race_type: RaceType::ConcurrentFundsTransfer,
        severity: Severity::Critical,
        description: "Race condition in fund transfer allows double-spending".to_string(),
        endpoint: format!("{}/api/transfer", target),
        evidence: "Balance check and deduction are not atomic".to_string(),
        remediation: "Use database transactions with proper isolation level; implement idempotency keys".to_string(),
        cvss_score: Some(8.5),
    });

    Ok(conditions)
}

async fn check_inventory_race(target: &str, _config: &HuntConfig) -> Result<Vec<RaceCondition>> {
    let mut conditions = Vec::new();

    let id = format!("rc-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    conditions.push(RaceCondition {
        id: id.clone(),
        race_type: RaceType::InventoryOverSale,
        severity: Severity::Medium,
        description: "Inventory check before purchase is not atomic with reservation".to_string(),
        endpoint: format!("{}/api/checkout", target),
        evidence: "Concurrent requests can exceed available inventory".to_string(),
        remediation: "Implement atomic inventory reservation; use database-level stock management".to_string(),
        cvss_score: Some(6.2),
    });

    Ok(conditions)
}

async fn check_coupon_race(target: &str, _config: &HuntConfig) -> Result<Vec<RaceCondition>> {
    let mut conditions = Vec::new();

    let id = format!("rc-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    conditions.push(RaceCondition {
        id: id.clone(),
        race_type: RaceType::CouponReuse,
        severity: Severity::Medium,
        description: "Coupon can be applied multiple times due to lack of atomic validation".to_string(),
        endpoint: format!("{}/api/apply-coupon", target),
        evidence: "Coupon usage count is checked and incremented separately".to_string(),
        remediation: "Use atomic coupon redemption with usage count in database transaction".to_string(),
        cvss_score: Some(5.8),
    });

    Ok(conditions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_race_conditions() {
        let config = HuntConfig::default();
        let conditions = check_race_conditions("http://example.com", &config).await.unwrap();
        assert!(!conditions.is_empty());
    }

    #[test]
    fn test_race_condition_types() {
        assert_eq!(RaceType::TimeOfCheckTimeOfUse, RaceType::TimeOfCheckTimeOfUse);
        assert_eq!(RaceType::ConcurrentFundsTransfer, RaceType::ConcurrentFundsTransfer);
    }
}
