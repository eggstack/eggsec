use crate::error::Result;
use crate::findings::lifecycle::{FindingStatus, StoredFinding};
use crate::storage::models::StoredScan;
use crate::storage::StorageConfig;
use crate::Severity;

#[cfg(feature = "database")]
use crate::error::SlapperError;
#[cfg(feature = "database")]
use crate::storage::models::ScanStatus;

#[cfg(feature = "database")]
use sqlx::postgres::{PgPool, PgPoolOptions};
#[cfg(feature = "database")]
use sqlx::Row;

pub struct Database {
    #[cfg(feature = "database")]
    pool: PgPool,
    #[cfg(not(feature = "database"))]
    config: StorageConfig,
}

impl Database {
    pub async fn new(config: &StorageConfig) -> Result<Self> {
        #[cfg(feature = "database")]
        {
            let url = format!(
                "postgres://{}:{}@{}:{}/{}",
                config.username,
                config.password.expose_secret(),
                config.host,
                config.port,
                config.database
            );

            let pool = PgPoolOptions::new()
                .max_connections(config.max_connections)
                .connect(&url)
                .await
                .map_err(|e| {
                    SlapperError::Config(format!("Failed to connect to database: {}", e))
                })?;

            Ok(Self { pool })
        }
        #[cfg(not(feature = "database"))]
        {
            Ok(Self {
                config: config.clone(),
            })
        }
    }

    pub async fn insert_scan(&self, _scan: &StoredScan) -> Result<()> {
        #[cfg(feature = "database")]
        {
            sqlx::query(
                "INSERT INTO scans (id, target, scan_type, started_at, completed_at, status, findings_count)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (id) DO UPDATE SET
                     completed_at = EXCLUDED.completed_at,
                     status = EXCLUDED.status,
                     findings_count = EXCLUDED.findings_count"
            )
            .bind(&scan.id)
            .bind(&scan.target)
            .bind(&scan.scan_type)
            .bind(scan.started_at)
            .bind(scan.completed_at)
            .bind(format!("{:?}", scan.status))
            .bind(scan.findings_count as i64)
            .execute(&self.pool)
            .await
            .map_err(|e| SlapperError::Config(format!("Failed to insert scan: {}", e)))?;
        }
        Ok(())
    }

    pub async fn get_scan(&self, _id: &str) -> Result<Option<StoredScan>> {
        #[cfg(feature = "database")]
        {
            let row = sqlx::query("SELECT * FROM scans WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| SlapperError::Config(format!("Failed to get scan: {}", e)))?;

            Ok(row.map(|r| StoredScan {
                id: r.get("id"),
                target: r.get("target"),
                scan_type: r.get("scan_type"),
                started_at: r.get("started_at"),
                completed_at: r.get("completed_at"),
                status: parse_scan_status(r.get::<String, _>("status")),
                findings_count: r.get::<i64, _>("findings_count") as usize,
            }))
        }
        #[cfg(not(feature = "database"))]
        {
            Ok(None)
        }
    }

    pub async fn list_scans(&self, _limit: usize) -> Result<Vec<StoredScan>> {
        #[cfg(feature = "database")]
        {
            let rows = sqlx::query("SELECT * FROM scans ORDER BY started_at DESC LIMIT $1")
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| SlapperError::Config(format!("Failed to list scans: {}", e)))?;

            Ok(rows
                .iter()
                .map(|r| StoredScan {
                    id: r.get("id"),
                    target: r.get("target"),
                    scan_type: r.get("scan_type"),
                    started_at: r.get("started_at"),
                    completed_at: r.get("completed_at"),
                    status: parse_scan_status(r.get::<String, _>("status")),
                    findings_count: r.get::<i64, _>("findings_count") as usize,
                })
                .collect())
        }
        #[cfg(not(feature = "database"))]
        {
            Ok(vec![])
        }
    }

    pub async fn insert_finding(&self, _stored: &StoredFinding) -> Result<()> {
        #[cfg(feature = "database")]
        {
            let finding_json = serde_json::to_value(&stored.finding).map_err(|e| {
                SlapperError::Config(format!("Failed to serialize finding: {}", e))
            })?;
            let history_json = serde_json::to_value(&stored.status_history).map_err(|e| {
                SlapperError::Config(format!("Failed to serialize status history: {}", e))
            })?;

            sqlx::query(
                "INSERT INTO findings (id, scan_id, finding, status, created_at, updated_at, status_history)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (id) DO UPDATE SET
                     finding = EXCLUDED.finding,
                     status = EXCLUDED.status,
                     updated_at = EXCLUDED.updated_at,
                     status_history = EXCLUDED.status_history"
            )
            .bind(&stored.finding.id)
            .bind(&stored.finding.affected_asset.identifier)
            .bind(finding_json)
            .bind(stored.status.to_string())
            .bind(stored.created_at)
            .bind(stored.updated_at)
            .bind(history_json)
            .execute(&self.pool)
            .await
            .map_err(|e| SlapperError::Config(format!("Failed to insert finding: {}", e)))?;
        }
        Ok(())
    }

    pub async fn get_finding(&self, _id: &str) -> Result<Option<StoredFinding>> {
        #[cfg(feature = "database")]
        {
            let row = sqlx::query("SELECT * FROM findings WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| SlapperError::Config(format!("Failed to get finding: {}", e)))?;

            Ok(row.map(|r| row_to_stored_finding(&r)))
        }
        #[cfg(not(feature = "database"))]
        {
            Ok(None)
        }
    }

    pub async fn update_finding_status(&self, _id: &str, _status: FindingStatus) -> Result<()> {
        #[cfg(feature = "database")]
        {
            sqlx::query("UPDATE findings SET status = $1, updated_at = NOW() WHERE id = $2")
                .bind(status.to_string())
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    SlapperError::Config(format!("Failed to update finding status: {}", e))
                })?;
        }
        Ok(())
    }

    pub async fn list_findings(&self, _scan_id: &str) -> Result<Vec<StoredFinding>> {
        #[cfg(feature = "database")]
        {
            let rows = if scan_id == "all" {
                sqlx::query("SELECT * FROM findings ORDER BY created_at DESC")
                    .fetch_all(&self.pool)
                    .await
            } else {
                sqlx::query("SELECT * FROM findings WHERE scan_id = $1 ORDER BY created_at DESC")
                    .bind(scan_id)
                    .fetch_all(&self.pool)
                    .await
            }
            .map_err(|e| SlapperError::Config(format!("Failed to list findings: {}", e)))?;

            Ok(rows.iter().map(|r| row_to_stored_finding(r)).collect())
        }
        #[cfg(not(feature = "database"))]
        {
            Ok(vec![])
        }
    }

    pub async fn get_findings_by_severity(
        &self,
        _severity: Severity,
    ) -> Result<Vec<StoredFinding>> {
        #[cfg(feature = "database")]
        {
            let rows = sqlx::query(
                "SELECT * FROM findings WHERE finding->>'severity' = $1 ORDER BY created_at DESC",
            )
            .bind(severity.as_str().to_lowercase())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                SlapperError::Config(format!("Failed to get findings by severity: {}", e))
            })?;

            Ok(rows.iter().map(|r| row_to_stored_finding(r)).collect())
        }
        #[cfg(not(feature = "database"))]
        {
            Ok(vec![])
        }
    }
}

#[cfg(feature = "database")]
fn parse_scan_status(s: String) -> ScanStatus {
    match s.as_str() {
        "Running" => ScanStatus::Running,
        "Completed" => ScanStatus::Completed,
        "Failed" => ScanStatus::Failed,
        "Cancelled" => ScanStatus::Cancelled,
        _ => ScanStatus::Running,
    }
}

#[cfg(feature = "database")]
fn row_to_stored_finding(row: &sqlx::postgres::PgRow) -> StoredFinding {
    use sqlx::Row;

    let finding_json: serde_json::Value = row.get("finding");
    let history_json: serde_json::Value = row.get("status_history");
    let status_str: String = row.get("status");

    let finding: crate::findings::Finding = serde_json::from_value(finding_json).unwrap_or_else(
        |_| crate::findings::Finding {
            id: uuid::Uuid::new_v4().to_string(),
            fingerprint: String::new(),
            title: "Deserialization failed".to_string(),
            description: String::new(),
            severity: Severity::Info,
            confidence: crate::findings::Confidence::Informational,
            finding_type: crate::findings::FindingType::ScanResult,
            cwe: None,
            owasp: None,
            cve: None,
            affected_asset: crate::findings::AffectedAsset {
                asset_type: "unknown".to_string(),
                identifier: "unknown".to_string(),
                host: None,
                port: None,
                protocol: None,
            },
            location: crate::findings::FindingLocation::default(),
            evidence: vec![],
            reproduction: None,
            remediation: None,
            discovered_at: chrono::Utc::now(),
            source: crate::findings::FindingSource {
                tool: "unknown".to_string(),
                module: "storage".to_string(),
                run_id: None,
            },
            tags: vec![],
            metadata: serde_json::Value::Null,
        },
    );

    let status = match status_str.as_str() {
        "new" => FindingStatus::New,
        "confirmed" => FindingStatus::Confirmed,
        "accepted_risk" => FindingStatus::AcceptedRisk,
        "false_positive" => FindingStatus::FalsePositive,
        "remediated" => FindingStatus::Remediated,
        "reopened" => FindingStatus::Reopened,
        _ => FindingStatus::New,
    };

    let status_history: Vec<crate::findings::lifecycle::StatusChange> =
        serde_json::from_value(history_json).unwrap_or_default();

    let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
    let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");

    StoredFinding {
        finding,
        status,
        created_at,
        updated_at,
        status_history,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_creation() {
        let config = StorageConfig::default();
        let db = Database::new(&config).await.unwrap();
        assert_eq!(db.config.port, 5432);
    }
}
