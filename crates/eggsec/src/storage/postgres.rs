use crate::error::EggsecError;
use crate::error::Result;
use crate::findings::lifecycle::{FindingStatus, StoredFinding};
use crate::storage::models::StoredScan;
use crate::storage::StorageConfig;
use crate::Severity;

#[cfg(feature = "database")]
use crate::storage::models::ScanStatus;

#[cfg(feature = "database")]
use sqlx::postgres::{PgPool, PgPoolOptions};
#[cfg(feature = "database")]
use sqlx::Row;

pub struct Database {
    #[cfg(feature = "database")]
    pool: PgPool,
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
                    EggsecError::Config(format!("Failed to connect to database: {}", e))
                })?;

            Ok(Self { pool })
        }
        #[cfg(not(feature = "database"))]
        {
            let _ = config;
            Err(EggsecError::Config(
                "database feature not enabled".to_string(),
            ))
        }
    }

    #[cfg(feature = "database")]
    pub fn pool_ref(&self) -> &PgPool {
        &self.pool
    }

    pub async fn insert_scan(&self, scan: &StoredScan) -> Result<()> {
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
            .bind(scan.status.to_string())
            .bind(scan.findings_count as i64)
            .execute(&self.pool)
            .await
            .map_err(|e| EggsecError::Config(format!("Failed to insert scan: {}", e)))?;
        }
        Ok(())
    }

    pub async fn get_scan(&self, id: &str) -> Result<Option<StoredScan>> {
        #[cfg(feature = "database")]
        {
            let row = sqlx::query("SELECT * FROM scans WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| EggsecError::Config(format!("Failed to get scan: {}", e)))?;

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
            let _ = id;
            Ok(None)
        }
    }

    pub async fn list_scans(&self, limit: usize) -> Result<Vec<StoredScan>> {
        #[cfg(feature = "database")]
        {
            let rows = sqlx::query("SELECT * FROM scans ORDER BY started_at DESC LIMIT $1")
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| EggsecError::Config(format!("Failed to list scans: {}", e)))?;

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
            let _ = limit;
            Ok(vec![])
        }
    }

    pub async fn insert_finding(&self, stored: &StoredFinding) -> Result<()> {
        #[cfg(feature = "database")]
        {
            let finding_json = serde_json::to_value(&stored.finding)
                .map_err(|e| EggsecError::Config(format!("Failed to serialize finding: {}", e)))?;
            let history_json = serde_json::to_value(&stored.status_history).map_err(|e| {
                EggsecError::Config(format!("Failed to serialize status history: {}", e))
            })?;

            sqlx::query(
                "INSERT INTO findings (id, scan_id, finding, status, created_at, updated_at, status_history)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (id) DO UPDATE SET
                     scan_id = EXCLUDED.scan_id,
                     finding = EXCLUDED.finding,
                     status = EXCLUDED.status,
                     updated_at = EXCLUDED.updated_at,
                     status_history = EXCLUDED.status_history"
            )
            .bind(&stored.finding.id)
            .bind(&stored.scan_id)
            .bind(finding_json)
            .bind(stored.status.to_string())
            .bind(stored.created_at)
            .bind(stored.updated_at)
            .bind(history_json)
            .execute(&self.pool)
            .await
            .map_err(|e| EggsecError::Config(format!("Failed to insert finding: {}", e)))?;
        }
        Ok(())
    }

    pub async fn get_finding(&self, id: &str) -> Result<Option<StoredFinding>> {
        #[cfg(feature = "database")]
        {
            let row = sqlx::query("SELECT * FROM findings WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| EggsecError::Config(format!("Failed to get finding: {}", e)))?;

            Ok(row.map(|r| row_to_stored_finding(&r)).transpose()?)
        }
        #[cfg(not(feature = "database"))]
        {
            let _ = id;
            Ok(None)
        }
    }

    pub async fn update_finding_status(&self, id: &str, status: FindingStatus) -> Result<()> {
        #[cfg(feature = "database")]
        {
            sqlx::query("UPDATE findings SET status = $1, updated_at = NOW() WHERE id = $2")
                .bind(status.to_string())
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    EggsecError::Config(format!("Failed to update finding status: {}", e))
                })?;
        }
        Ok(())
    }

    pub async fn list_findings(
        &self,
        scan_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<StoredFinding>> {
        #[cfg(feature = "database")]
        {
            let rows = sqlx::query(
                "SELECT * FROM findings WHERE scan_id = $1 ORDER BY created_at DESC OFFSET $2 LIMIT $3",
            )
            .bind(scan_id)
            .bind(offset as i64)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| EggsecError::Config(format!("Failed to list findings: {}", e)))?;

            rows.iter()
                .map(|r| row_to_stored_finding(r))
                .collect::<Result<Vec<_>>>()
        }
        #[cfg(not(feature = "database"))]
        {
            let _ = (scan_id, offset, limit);
            Ok(vec![])
        }
    }

    pub async fn list_all_findings(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<StoredFinding>> {
        #[cfg(feature = "database")]
        {
            let rows =
                sqlx::query("SELECT * FROM findings ORDER BY created_at DESC OFFSET $1 LIMIT $2")
                    .bind(offset as i64)
                    .bind(limit as i64)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|e| {
                        EggsecError::Config(format!("Failed to list all findings: {}", e))
                    })?;

            rows.iter()
                .map(|r| row_to_stored_finding(r))
                .collect::<Result<Vec<_>>>()
        }
        #[cfg(not(feature = "database"))]
        {
            let _ = (offset, limit);
            Ok(vec![])
        }
    }

    pub async fn get_findings_by_severity(&self, severity: Severity) -> Result<Vec<StoredFinding>> {
        #[cfg(feature = "database")]
        {
            let rows = sqlx::query(
                "SELECT * FROM findings WHERE finding->>'severity' = $1 ORDER BY created_at DESC",
            )
            .bind(severity.as_str().to_lowercase())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                EggsecError::Config(format!("Failed to get findings by severity: {}", e))
            })?;

            rows.iter()
                .map(|r| row_to_stored_finding(r))
                .collect::<Result<Vec<_>>>()
        }
        #[cfg(not(feature = "database"))]
        {
            let _ = severity;
            Ok(vec![])
        }
    }

    pub async fn update_scan_findings_count(&self, scan_id: &str) -> Result<()> {
        #[cfg(feature = "database")]
        {
            sqlx::query(
                "UPDATE scans SET findings_count = (SELECT COUNT(*)::int FROM findings WHERE scan_id = $1) WHERE id = $1",
            )
            .bind(scan_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                EggsecError::Config(format!("Failed to update scan findings count: {}", e))
            })?;
        }
        Ok(())
    }
}

#[cfg(feature = "database")]
fn parse_scan_status(s: String) -> ScanStatus {
    match s.to_lowercase().as_str() {
        "running" => ScanStatus::Running,
        "completed" => ScanStatus::Completed,
        "failed" => ScanStatus::Failed,
        "cancelled" => ScanStatus::Cancelled,
        other => {
            tracing::warn!(status = other, "Unknown scan status, defaulting to Running");
            ScanStatus::Running
        }
    }
}

#[cfg(feature = "database")]
fn row_to_stored_finding(row: &sqlx::postgres::PgRow) -> Result<StoredFinding> {
    use sqlx::Row;

    let finding_json: serde_json::Value = row.get("finding");
    let history_json: serde_json::Value = row.get("status_history");
    let status_str: String = row.get("status");
    let scan_id: String = row.get("scan_id");

    let finding: crate::findings::Finding = serde_json::from_value(finding_json).map_err(|e| {
        EggsecError::Config(format!(
            "Failed to deserialize finding JSON for id {}: {}",
            row.get::<String, _>("id"),
            e
        ))
    })?;

    let status = match status_str.as_str() {
        "new" => FindingStatus::New,
        "confirmed" => FindingStatus::Confirmed,
        "accepted_risk" => FindingStatus::AcceptedRisk,
        "false_positive" => FindingStatus::FalsePositive,
        "remediated" => FindingStatus::Remediated,
        "reopened" => FindingStatus::Reopened,
        other => {
            tracing::warn!(
                finding_id = row.get::<String, _>("id"),
                status = other,
                "Unknown finding status, defaulting to New"
            );
            FindingStatus::New
        }
    };

    let status_history: Vec<crate::findings::lifecycle::StatusChange> =
        serde_json::from_value(history_json).unwrap_or_else(|e| {
            tracing::warn!(
                finding_id = row.get::<String, _>("id"),
                error = %e,
                "Failed to deserialize status_history, using empty history"
            );
            vec![]
        });

    let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
    let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");

    Ok(StoredFinding {
        finding,
        scan_id,
        status,
        created_at,
        updated_at,
        status_history,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_config_defaults() {
        let config = StorageConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5432);
        assert_eq!(config.database, "eggsec");
        assert_eq!(config.username, "postgres");
        assert_eq!(config.max_connections, 10);
    }

    #[cfg(feature = "database")]
    mod database_tests {
        use super::*;
        use crate::storage::models::ScanStatus;

        #[test]
        fn test_scan_status_parse_roundtrip() {
            let statuses = [
                ScanStatus::Running,
                ScanStatus::Completed,
                ScanStatus::Failed,
                ScanStatus::Cancelled,
            ];
            for status in &statuses {
                let s = status.to_string();
                let parsed = parse_scan_status(s.clone());
                assert_eq!(*status, parsed, "Roundtrip failed for {}", s);
            }
        }

        #[test]
        fn test_scan_status_parse_unknown() {
            let parsed = parse_scan_status("unknown_status".to_string());
            assert_eq!(parsed, ScanStatus::Running);
        }

        #[test]
        fn test_scan_status_parse_case_insensitive() {
            assert_eq!(
                parse_scan_status("running".to_string()),
                ScanStatus::Running
            );
            assert_eq!(
                parse_scan_status("Running".to_string()),
                ScanStatus::Running
            );
            assert_eq!(
                parse_scan_status("RUNNING".to_string()),
                ScanStatus::Running
            );
            assert_eq!(
                parse_scan_status("completed".to_string()),
                ScanStatus::Completed
            );
            assert_eq!(
                parse_scan_status("COMPLETED".to_string()),
                ScanStatus::Completed
            );
        }
    }
}
