use crate::error::Result;
use crate::hunt::{HuntClient, HuntConfig};
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
#[allow(dead_code)]
pub enum RaceType {
    TimeOfCheckTimeOfUse,
    ConcurrentFundsTransfer,
    InventoryOverSale,
    SessionRace,
    CouponReuse,
    CommentRace,
    ResponseInconsistency,
    TimingAnomaly,
}

const STATE_CHANGING_PATHS: &[&str] = &[
    "/api/checkout",
    "/api/cart",
    "/api/transfer",
    "/api/payment",
    "/api/order",
    "/api/coupon",
    "/api/discount",
    "/api/vote",
    "/api/like",
    "/api/comment",
    "/api/purchase",
    "/api/redeem",
    "/api/claim",
    "/api/book",
    "/api/reserve",
];

pub async fn check_race_conditions(
    client: &HuntClient,
    config: &HuntConfig,
) -> Result<Vec<RaceCondition>> {
    let mut conditions = Vec::new();

    conditions.extend(check_concurrent_requests(client, config).await);
    conditions.extend(check_response_inconsistency(client, config).await);

    Ok(conditions)
}

async fn check_concurrent_requests(
    client: &HuntClient,
    config: &HuntConfig,
) -> Vec<RaceCondition> {
    let mut conditions = Vec::new();
    let concurrency = config.concurrency;

    for path in STATE_CHANGING_PATHS {
        let mut handles = Vec::new();
        let mut status_codes = Vec::new();

        for _ in 0..concurrency {
            let client = client.clone();
            let path = path.to_string();

            handles.push(tokio::spawn(async move {
                let body = serde_json::json!({
                    "action": "test",
                    "quantity": 1,
                    "amount": 100
                });
                client
                    .post_json(&path, &body)
                    .await
                    .map(|r| r.status().as_u16())
                    .unwrap_or(0)
            }));
        }

        for handle in handles {
            if let Ok(status) = handle.await {
                status_codes.push(status);
            }
        }

        if status_codes.is_empty() {
            continue;
        }

        let unique_statuses: std::collections::HashSet<u16> =
            status_codes.iter().copied().collect();
        let success_count = status_codes.iter().filter(|&&s| s == 200 || s == 201).count();
        let error_count = status_codes.iter().filter(|&&s| s >= 400).count();

        if unique_statuses.len() > 1 && success_count > 0 && error_count > 0 {
            let id = format!("rc-{}", &uuid::Uuid::new_v4().to_string()[..8]);
            conditions.push(RaceCondition {
                id,
                race_type: RaceType::ResponseInconsistency,
                severity: Severity::High,
                description: format!(
                    "Inconsistent responses under concurrent load at {} ({} success, {} error)",
                    path, success_count, error_count
                ),
                endpoint: format!("{}{}", client.base_url(), path),
                evidence: format!(
                    "Status codes: {:?}",
                    unique_statuses.into_iter().collect::<Vec<_>>()
                ),
                remediation:
                    "Implement proper locking or atomic operations for state-changing endpoints"
                        .to_string(),
                cvss_score: Some(7.0),
            });
        }

        if success_count > 1 {
            let id = format!("rc-{}", &uuid::Uuid::new_v4().to_string()[..8]);
            conditions.push(RaceCondition {
                id,
                race_type: RaceType::TimeOfCheckTimeOfUse,
                severity: Severity::Medium,
                description: format!(
                    "Multiple concurrent requests succeeded at {} (potential TOCTOU)",
                    path
                ),
                endpoint: format!("{}{}", client.base_url(), path),
                evidence: format!(
                    "{} out of {} concurrent requests returned success",
                    success_count, concurrency
                ),
                remediation: "Use database-level locking or idempotency keys for state-changing operations"
                    .to_string(),
                cvss_score: Some(6.0),
            });
        }
    }

    conditions
}

async fn check_response_inconsistency(
    client: &HuntClient,
    _config: &HuntConfig,
) -> Vec<RaceCondition> {
    let mut conditions = Vec::new();

    let endpoints = ["/api/user/profile", "/api/cart", "/api/balance"];

    for path in &endpoints {
        let mut timings = Vec::new();
        let mut statuses = Vec::new();

        for _ in 0..5 {
            let start = std::time::Instant::now();
            let resp = client.get(path).await;
            let elapsed = start.elapsed().as_millis() as u64;

            if let Ok(resp) = resp {
                timings.push(elapsed);
                statuses.push(resp.status().as_u16());
            }
        }

        if timings.len() < 2 {
            continue;
        }

        let avg: u64 = timings.iter().sum::<u64>() / timings.len() as u64;
        let max_deviation = timings
            .iter()
            .map(|t| (*t as i64 - avg as i64).unsigned_abs())
            .max()
            .unwrap_or(0);

        if max_deviation > avg * 3 && avg > 0 {
            let id = format!("rc-{}", &uuid::Uuid::new_v4().to_string()[..8]);
            conditions.push(RaceCondition {
                id,
                race_type: RaceType::TimingAnomaly,
                severity: Severity::Low,
                description: format!(
                    "Timing anomaly detected at {} (deviation: {}ms, avg: {}ms)",
                    path, max_deviation, avg
                ),
                endpoint: format!("{}{}", client.base_url(), path),
                evidence: format!("Timings: {:?}ms", timings),
                remediation: "Investigate timing inconsistencies that may indicate race conditions"
                    .to_string(),
                cvss_score: Some(3.0),
            });
        }

        let unique_statuses: std::collections::HashSet<u16> = statuses.iter().copied().collect();
        if unique_statuses.len() > 1 {
            let id = format!("rc-{}", &uuid::Uuid::new_v4().to_string()[..8]);
            conditions.push(RaceCondition {
                id,
                race_type: RaceType::ResponseInconsistency,
                severity: Severity::Medium,
                description: format!(
                    "Inconsistent status codes across repeated requests at {}",
                    path
                ),
                endpoint: format!("{}{}", client.base_url(), path),
                evidence: format!("Status codes: {:?}", statuses),
                remediation: "Ensure consistent responses for identical requests".to_string(),
                cvss_score: Some(5.0),
            });
        }
    }

    conditions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_race_condition_types() {
        assert_eq!(
            RaceType::TimeOfCheckTimeOfUse,
            RaceType::TimeOfCheckTimeOfUse
        );
        assert_eq!(
            RaceType::ConcurrentFundsTransfer,
            RaceType::ConcurrentFundsTransfer
        );
        assert_eq!(RaceType::TimingAnomaly, RaceType::TimingAnomaly);
    }
}
