use crate::cli::storage::{StorageArgs, StorageCommand};
use crate::commands::handlers::CommandContext;
use anyhow::Result;

pub async fn handle_storage(_ctx: &CommandContext, args: StorageArgs) -> Result<()> {
    match args.command {
        StorageCommand::Query(args) => handle_storage_query(args).await,
        StorageCommand::Export(args) => handle_storage_export(args).await,
        StorageCommand::Stats(args) => handle_storage_stats(args).await,
        StorageCommand::Init(args) => handle_storage_init(args).await,
    }
}

async fn handle_storage_query(args: crate::cli::storage::StorageQueryArgs) -> Result<()> {
    #[cfg(feature = "database")]
    {
        use crate::storage::{init_storage, StorageConfig};
        use sqlx::Row;

        let config = StorageConfig::default();
        let db = init_storage(&config).await?;

        if let Some(ref sql) = args.sql {
            let rows = sqlx::query(sql)
                .fetch_all(db.pool_ref())
                .await
                .map_err(|e| anyhow::anyhow!("Query failed: {}", e))?;

            println!("Rows returned: {}", rows.len());
            for (i, row) in rows.iter().enumerate() {
                println!("  Row {}: {:?}", i + 1, row.columns());
            }
        } else {
            let query_type = args.query.as_deref().unwrap_or("recent_scans");
            let limit = args.limit.unwrap_or(10);
            match query_type {
                "recent_scans" => {
                    let scans = db.list_scans(limit).await?;
                    println!("Recent scans ({}):", scans.len());
                    for scan in &scans {
                        println!(
                            "  {} - {} - {} ({} findings)",
                            scan.id, scan.target, scan.status, scan.findings_count
                        );
                    }
                }
                "all_findings" => {
                    let findings = db.list_all_findings(0, limit).await?;
                    println!("Findings ({}):", findings.len());
                    for f in &findings {
                        println!(
                            "  [{}] {} - {:?}",
                            f.finding.severity, f.finding.title, f.status
                        );
                    }
                }
                _ => {
                    anyhow::bail!(
                        "Unknown query type: {}. Valid types: recent_scans, all_findings",
                        query_type
                    );
                }
            }
        }
        Ok(())
    }
    #[cfg(not(feature = "database"))]
    {
        let _ = args;
        println!("Database storage requires the 'database' feature to be enabled.");
        println!("Rebuild with: cargo build --features database");
        Ok(())
    }
}

async fn handle_storage_export(args: crate::cli::storage::StorageExportArgs) -> Result<()> {
    #[cfg(feature = "database")]
    {
        use crate::storage::{init_storage, StorageConfig};

        let config = StorageConfig::default();
        let db = init_storage(&config).await?;

        let output_path = args
            .output
            .as_deref()
            .unwrap_or("storage_export.json");

        if let Some(ref scan_id) = args.scan_id {
            let findings = db.list_findings(scan_id, 0, 10000).await?;
            let json = serde_json::to_string_pretty(&findings)
                .map_err(|e| anyhow::anyhow!("Serialization failed: {}", e))?;
            std::fs::write(output_path, &json)?;
            println!(
                "Exported {} findings for scan {} to {}",
                findings.len(),
                scan_id,
                output_path
            );
        } else if let Some(ref finding_id) = args.finding_id {
            match db.get_finding(finding_id).await? {
                Some(finding) => {
                    let json = serde_json::to_string_pretty(&finding)
                        .map_err(|e| anyhow::anyhow!("Serialization failed: {}", e))?;
                    std::fs::write(output_path, &json)?;
                    println!("Exported finding {} to {}", finding_id, output_path);
                }
                None => {
                    anyhow::bail!("Finding not found: {}", finding_id);
                }
            }
        } else {
            let findings = db.list_all_findings(0, 10000).await?;
            let json = serde_json::to_string_pretty(&findings)
                .map_err(|e| anyhow::anyhow!("Serialization failed: {}", e))?;
            std::fs::write(output_path, &json)?;
            println!("Exported {} findings to {}", findings.len(), output_path);
        }
        Ok(())
    }
    #[cfg(not(feature = "database"))]
    {
        let _ = args;
        println!("Database storage requires the 'database' feature to be enabled.");
        println!("Rebuild with: cargo build --features database");
        Ok(())
    }
}

async fn handle_storage_stats(args: crate::cli::storage::StorageStatsArgs) -> Result<()> {
    #[cfg(feature = "database")]
    {
        use crate::storage::{init_storage, StorageConfig};

        let config = StorageConfig::default();
        let db = init_storage(&config).await?;

        if let Some(ref scan_id) = args.scan_id {
            match db.get_scan(scan_id).await? {
                Some(scan) => {
                    println!("Scan: {}", scan.id);
                    println!("  Target: {}", scan.target);
                    println!("  Type: {}", scan.scan_type);
                    println!("  Status: {}", scan.status);
                    println!("  Started: {}", scan.started_at);
                    if let Some(completed) = scan.completed_at {
                        println!("  Completed: {}", completed);
                    }
                    println!("  Findings: {}", scan.findings_count);
                }
                None => {
                    anyhow::bail!("Scan not found: {}", scan_id);
                }
            }
        } else {
            let scans = db.list_scans(100).await?;
            println!("Database Statistics:");
            println!("  Total scans: {}", scans.len());
            let running = scans.iter().filter(|s| s.status == crate::storage::models::ScanStatus::Running).count();
            let completed = scans.iter().filter(|s| s.status == crate::storage::models::ScanStatus::Completed).count();
            let failed = scans.iter().filter(|s| s.status == crate::storage::models::ScanStatus::Failed).count();
            println!("  Running: {}", running);
            println!("  Completed: {}", completed);
            println!("  Failed: {}", failed);
            println!("  (Note: total findings count requires a COUNT query)");
        }
        Ok(())
    }
    #[cfg(not(feature = "database"))]
    {
        let _ = args;
        println!("Database storage requires the 'database' feature to be enabled.");
        println!("Rebuild with: cargo build --features database");
        Ok(())
    }
}

async fn handle_storage_init(args: crate::cli::storage::StorageInitArgs) -> Result<()> {
    #[cfg(feature = "database")]
    {
        use crate::storage::{init_storage, StorageConfig};

        let config = StorageConfig::default();
        let db = init_storage(&config).await?;

        if args.force {
            println!("Dropping existing tables...");
            sqlx::query("DROP TABLE IF EXISTS findings CASCADE")
                .execute(db.pool_ref())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to drop findings: {}", e))?;
            sqlx::query("DROP TABLE IF EXISTS scans CASCADE")
                .execute(db.pool_ref())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to drop scans: {}", e))?;
            sqlx::query("DROP TABLE IF EXISTS users CASCADE")
                .execute(db.pool_ref())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to drop users: {}", e))?;
            println!("Tables dropped.");
        }

        println!("Running migrations...");

        sqlx::query(include_str!("../../../migrations/001_create_scans.sql"))
            .execute(db.pool_ref())
            .await
            .map_err(|e| anyhow::anyhow!("Migration 001 failed: {}", e))?;
        println!("  001_create_scans.sql - OK");

        sqlx::query(include_str!("../../../migrations/002_create_findings.sql"))
            .execute(db.pool_ref())
            .await
            .map_err(|e| anyhow::anyhow!("Migration 002 failed: {}", e))?;
        println!("  002_create_findings.sql - OK");

        sqlx::query(include_str!("../../../migrations/003_create_users.sql"))
            .execute(db.pool_ref())
            .await
            .map_err(|e| anyhow::anyhow!("Migration 003 failed: {}", e))?;
        println!("  003_create_users.sql - OK");

        println!("Database initialized successfully.");
        Ok(())
    }
    #[cfg(not(feature = "database"))]
    {
        let _ = args;
        println!("Database storage requires the 'database' feature to be enabled.");
        println!("Rebuild with: cargo build --features database");
        Ok(())
    }
}
