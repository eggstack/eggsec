//! Redis checks for db-pentest Phase 5.
//! All operations are read-only or use safe INFO/ACL/DEBUG DRYRUN commands.
//! Bounded enumeration with configurable limits.
//!
//! Compiled ONLY when both `db-pentest` AND the marker `db-pentest-redis` are enabled.
//! When the marker is absent, real-path runs fall back to synthetic population
//! (caller in mod.rs + utils::populate_dry_run_findings). Dry-run for Redis is
//! always rich and driver-independent.

use crate::types::{CheckType, DbPentestReport, DbTarget};
use anyhow::Result;

#[cfg(feature = "redis")]
mod real {
    use super::*;
    use crate::types::DbFinding;
    use crate::utils;
    use eggsec_core::types::Severity;

    /// Run Redis security checks.
    ///
    /// If `client` is provided, reuses the existing `redis::Client` (connection reuse).
    /// If `client` is `None`, creates a new client for this call only.
    pub async fn run_redis_checks_real(
        target: &DbTarget,
        report: &mut DbPentestReport,
        checks: &[CheckType],
        max_queries: u64,
        client: Option<&::redis::Client>,
    ) -> Result<()> {
        let created_client;
        let client_ref = match client {
            Some(c) => c,
            None => {
                let url = utils::build_redis_url(target);
                created_client = ::redis::Client::open(url.as_str())?;
                &created_client
            }
        };

        // Connect using the redis crate (blocking, wrapped for async context)
        let cc = client_ref.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = cc.get_connection()?;
            // Quick connectivity test
            let _: String = ::redis::cmd("PING").query(&mut conn)?;
            Ok::<_, anyhow::Error>(conn)
        })
        .await??;

        report
            .actions_performed
            .push("redis: connected (PING succeeded)".to_string());

        // Connection / PING
        if checks.iter().any(|c| matches!(c, CheckType::Connection)) {
            report.findings.push(DbFinding {
                category: "db-redis-connection-ok".to_string(),
                severity: Severity::Info,
                title: "Redis connection successful".to_string(),
                description: "PING command returned PONG; basic connectivity confirmed."
                    .to_string(),
                recommendation: "Ensure Redis is only accessible from trusted networks."
                    .to_string(),
                evidence: Some("PING -> PONG".to_string()),
                db_type: "redis".to_string(),
                target_host: target.host.clone(),
            });
        }

        // Version / INFO server
        if checks.iter().any(|c| {
            matches!(
                c,
                CheckType::Connection | CheckType::Version | CheckType::Cve
            )
        }) {
            let cc = client_ref.clone();
            let info_result = tokio::task::spawn_blocking(move || -> Result<String> {
                let mut conn = cc.get_connection()?;
                let info: String = ::redis::cmd("INFO").arg("server").query(&mut conn)?;
                Ok(info)
            })
            .await??;
            report.queries_executed += 1;
            report
                .actions_performed
                .push("redis: fetched INFO server".to_string());

            let mut version_str = "unknown".to_string();
            for line in info_result.lines() {
                if let Some(v) = line.strip_prefix("redis_version:") {
                    version_str = v.trim().to_string();
                    break;
                }
            }

            if checks
                .iter()
                .any(|c| matches!(c, CheckType::Version | CheckType::Cve))
            {
                report.findings.push(DbFinding {
                    category: "db-redis-version".to_string(),
                    severity: Severity::Info,
                    title: format!("Redis version: {}", version_str),
                    description:
                        "Version string obtained via INFO server (safe read-only command)."
                            .to_string(),
                    recommendation: "Keep Redis patched; review release notes for security fixes."
                        .to_string(),
                    evidence: Some(format!("redis_version={}", version_str)),
                    db_type: "redis".to_string(),
                    target_host: target.host.clone(),
                });

                if let Some(major) = version_str
                    .split('.')
                    .next()
                    .and_then(|s| s.parse::<u32>().ok())
                {
                    if major < 6 {
                        report.findings.push(DbFinding {
                            category: "db-redis-cve".to_string(),
                            severity: Severity::High,
                            title: "Redis version < 6.0 — lacks ACL support, known CVEs".to_string(),
                            description: "Versions before 6.0 do not have ACL commands and have historical CVEs (e.g. Lua sandbox escape, module loading). Upgrade recommended.".to_string(),
                            recommendation: "Upgrade to Redis 7.x; if stuck on < 6.0, ensure no untrusted code/clients connect.".to_string(),
                            evidence: Some(format!("redis_version={}", version_str)),
                            db_type: "redis".to_string(),
                            target_host: target.host.clone(),
                        });
                    } else if major < 7 {
                        report.findings.push(DbFinding {
                            category: "db-redis-cve".to_string(),
                            severity: Severity::Low,
                            title: "Redis 6.x — review known CVEs for exact patch level".to_string(),
                            description: "Redis 6.x line has had several CVEs (SSRF via Lua, ACL bypass). Verify current patch level.".to_string(),
                            recommendation: "Cross-reference Redis changelog and CVE databases for your exact version.".to_string(),
                            evidence: Some(format!("redis_version={}", version_str)),
                            db_type: "redis".to_string(),
                            target_host: target.host.clone(),
                        });
                    }
                }
            }
        }

        // Auth: requirepass / ACL check
        if checks
            .iter()
            .any(|c| matches!(c, CheckType::Auth | CheckType::Privs))
        {
            // Check requirepass via CONFIG GET
            let cc = client_ref.clone();
            let requirepass_result =
                tokio::task::spawn_blocking(move || -> Result<Option<String>> {
                    let mut conn = cc.get_connection()?;
                    let val: Option<String> = ::redis::cmd("CONFIG")
                        .arg("GET")
                        .arg("requirepass")
                        .arg("1")
                        .query(&mut conn)?;
                    Ok(val)
                })
                .await??;
            report.queries_executed += 1;

            let has_password = requirepass_result.as_ref().and_then(|v| {
                let trimmed = v.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            });

            if has_password.is_none() {
                report.findings.push(DbFinding {
                    category: "db-redis-auth-noauth".to_string(),
                    severity: Severity::High,
                    title: "Redis has no password configured (requirepass empty)".to_string(),
                    description: "Redis is running without authentication. Any client can connect and execute commands.".to_string(),
                    recommendation: "Set requirepass via CONFIG SET requirepass or redis.conf; use ACL for fine-grained control (Redis 6+).".to_string(),
                    evidence: Some("requirepass not set (empty)".to_string()),
                    db_type: "redis".to_string(),
                    target_host: target.host.clone(),
                });
            }

            // ACL user count (Redis 6+)
            let cc = client_ref.clone();
            let acl_users_result = tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
                let mut conn = cc.get_connection()?;
                let users: Vec<String> = ::redis::cmd("ACL").arg("USERS").query(&mut conn)?;
                Ok(users)
            })
            .await??;
            report.queries_executed += 1;

            report.actions_performed.push(format!(
                "redis: ACL USERS returned {} user(s)",
                acl_users_result.len()
            ));

            if acl_users_result.len() <= 1 && acl_users_result.iter().any(|u| u == "default") {
                report.findings.push(DbFinding {
                    category: "db-redis-auth-default-only".to_string(),
                    severity: Severity::Medium,
                    title: "Only default ACL user exists — ACL not configured".to_string(),
                    description: "Redis ACL system is not configured beyond the default user. All clients share the same permission set.".to_string(),
                    recommendation: "Create dedicated ACL users with minimal command/key permissions for each client role.".to_string(),
                    evidence: Some(format!("ACL users: {:?}", acl_users_result)),
                    db_type: "redis".to_string(),
                    target_host: target.host.clone(),
                });
            }
        }

        // Misconfig: bind address + protected-mode
        if checks.iter().any(|c| matches!(c, CheckType::Misconfig)) {
            // Bind address
            let cc = client_ref.clone();
            let bind_result = tokio::task::spawn_blocking(move || -> Result<Option<String>> {
                let mut conn = cc.get_connection()?;
                let val: Option<String> = ::redis::cmd("CONFIG")
                    .arg("GET")
                    .arg("bind")
                    .arg("1")
                    .query(&mut conn)?;
                Ok(val)
            })
            .await??;
            report.queries_executed += 1;

            if let Some(ref bind_val) = bind_result {
                let bind_str = bind_val.trim();
                if bind_str.is_empty() || bind_str.contains("0.0.0.0") || bind_str.contains('*') {
                    report.findings.push(DbFinding {
                        category: "db-redis-misconfig-bind-all".to_string(),
                        severity: Severity::High,
                        title: "Redis bind address is wildcard (0.0.0.0 or *)".to_string(),
                        description: "Redis is listening on all network interfaces, exposing it to any reachable client.".to_string(),
                        recommendation: "Bind to 127.0.0.1 or specific lab network interface(s) via redis.conf or CONFIG SET bind.".to_string(),
                        evidence: Some(format!("bind={}", bind_str)),
                        db_type: "redis".to_string(),
                        target_host: target.host.clone(),
                    });
                }
            }

            // Protected mode
            let cc = client_ref.clone();
            let protected_result =
                tokio::task::spawn_blocking(move || -> Result<Option<String>> {
                    let mut conn = cc.get_connection()?;
                    let val: Option<String> = ::redis::cmd("CONFIG")
                        .arg("GET")
                        .arg("protected-mode")
                        .arg("1")
                        .query(&mut conn)?;
                    Ok(val)
                })
                .await??;
            report.queries_executed += 1;

            if let Some(ref prot_val) = protected_result {
                if prot_val.trim() == "0" || prot_val.trim().eq_ignore_ascii_case("off") {
                    report.findings.push(DbFinding {
                        category: "db-redis-misconfig-protected-mode-off".to_string(),
                        severity: Severity::Medium,
                        title: "Redis protected-mode is disabled".to_string(),
                        description: "protected-mode prevents Redis from accepting connections from non-loopback addresses when no password is set. Disabling it without auth is dangerous.".to_string(),
                        recommendation: "Enable protected-mode (CONFIG SET protected-mode yes) or set requirepass for non-loopback access.".to_string(),
                        evidence: Some(format!("protected-mode={}", prot_val.trim())),
                        db_type: "redis".to_string(),
                        target_host: target.host.clone(),
                    });
                }
            }
        }

        // Privs: enumerate ACL user permissions (Redis 6+)
        if checks
            .iter()
            .any(|c| matches!(c, CheckType::Privs | CheckType::Enum))
        {
            let cc = client_ref.clone();
            let acl_list_result = tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
                let mut conn = cc.get_connection()?;
                let entries: Vec<String> = ::redis::cmd("ACL").arg("LIST").query(&mut conn)?;
                Ok(entries)
            })
            .await??;
            report.queries_executed += 1;

            let mut total_users = 0usize;
            let mut broad_users = Vec::new();
            for entry in &acl_list_result {
                let parts: Vec<&str> = entry.split_whitespace().collect();
                if parts.len() < 2 {
                    continue;
                }
                let username = parts[1];
                total_users += 1;

                let has_all_keys = entry.contains("~*");
                let has_all_commands = entry.contains("+@all") || entry.contains("+@admin");
                if has_all_keys && has_all_commands {
                    broad_users.push(username.to_string());
                }
            }

            if total_users > 0 {
                report
                    .actions_performed
                    .push(format!("redis: ACL LIST returned {} user(s)", total_users));
            }

            if !broad_users.is_empty() {
                report.findings.push(DbFinding {
                    category: "db-redis-priv-excessive".to_string(),
                    severity: Severity::High,
                    title: format!("ACL user(s) with broad permissions: {:?}", broad_users),
                    description: "One or more ACL users have both ~* (all keys) and +@all/+@admin (all commands), providing unrestricted access.".to_string(),
                    recommendation: "Restrict each ACL user to only the commands and key patterns required for their role.".to_string(),
                    evidence: Some(format!("broad users: {:?}", broad_users)),
                    db_type: "redis".to_string(),
                    target_host: target.host.clone(),
                });
            }
        }

        // Enum: database count + key sampling per DB (SCAN with bounded iteration)
        if checks.iter().any(|c| matches!(c, CheckType::Enum))
            && report.queries_executed < max_queries
        {
            let cc = client_ref.clone();
            let keyspace_result = tokio::task::spawn_blocking(move || -> Result<String> {
                let mut conn = cc.get_connection()?;
                let info: String = ::redis::cmd("INFO").arg("keyspace").query(&mut conn)?;
                Ok(info)
            })
            .await??;
            report.queries_executed += 1;

            let mut db_keys: Vec<(String, u64)> = Vec::new();
            for line in keyspace_result.lines() {
                if let Some(db_line) = line.strip_prefix("db") {
                    if let Some((db_num, rest)) = db_line.split_once(':') {
                        if let Some(keys_part) = rest.split(',').next() {
                            if let Some(k) = keys_part.strip_prefix("keys=") {
                                if let Ok(count) = k.parse::<u64>() {
                                    db_keys.push((format!("db{}", db_num), count));
                                }
                            }
                        }
                    }
                }
            }

            if !db_keys.is_empty() {
                report.actions_performed.push(format!(
                    "redis: INFO keyspace returned {} database(s) with keys",
                    db_keys.len()
                ));

                for (db_name, key_count) in &db_keys {
                    report.findings.push(DbFinding {
                        category: "db-redis-enum-db-keys".to_string(),
                        severity: Severity::Info,
                        title: format!("{}: {} key(s)", db_name, key_count),
                        description: format!("Database {} contains {} key(s) visible via INFO keyspace.", db_name, key_count),
                        recommendation: "Review data stored in Redis; consider key-level ACL restrictions per database.".to_string(),
                        evidence: Some(format!("{} keys={}", db_name, key_count)),
                        db_type: "redis".to_string(),
                        target_host: target.host.clone(),
                    });
                }

                let total_keys: u64 = db_keys.iter().map(|(_, c)| c).sum();
                if total_keys > 10000 {
                    report.findings.push(DbFinding {
                        category: "db-redis-enum-many-keys".to_string(),
                        severity: Severity::Info,
                        title: format!("{} total keys across {} database(s)", total_keys, db_keys.len()),
                        description: "Large key counts may indicate broad data exposure surface; review data classification and TTL policies.".to_string(),
                        recommendation: "Audit key naming conventions, set appropriate TTLs, and consider key-pattern ACL restrictions.".to_string(),
                        evidence: Some(format!("total_keys={} databases={}", total_keys, db_keys.len())),
                        db_type: "redis".to_string(),
                        target_host: target.host.clone(),
                    });
                }
            } else {
                report
                    .actions_performed
                    .push("redis: INFO keyspace returned no database entries".to_string());
            }

            // Bounded SCAN sampling (one pass over db0)
            if report.queries_executed < max_queries {
                let cc = client_ref.clone();
                let scan_result = tokio::task::spawn_blocking(
                    move || -> Result<(::redis::Value, Vec<String>)> {
                        let mut conn = cc.get_connection()?;
                        let (cursor, sample_keys): (::redis::Value, Vec<String>) =
                            ::redis::cmd("SCAN")
                                .arg(0u64)
                                .arg("COUNT")
                                .arg(20u64)
                                .query(&mut conn)?;
                        Ok((cursor, sample_keys))
                    },
                )
                .await??;
                report.queries_executed += 1;

                report.actions_performed.push(format!(
                    "redis: SCAN returned {} sample key(s) from db0",
                    scan_result.1.len()
                ));

                if !scan_result.1.is_empty() {
                    report.findings.push(DbFinding {
                        category: "db-redis-enum-scan-sample".to_string(),
                        severity: Severity::Info,
                        title: format!("SCAN returned {} sample key(s) from db0", scan_result.1.len()),
                        description: "Key enumeration via SCAN confirms readable key space. In production, key names may expose internal data structures.".to_string(),
                        recommendation: "Review key naming conventions; use ACL key patterns to restrict which clients can enumerate keys.".to_string(),
                        evidence: Some(format!("sampled {} key(s) via SCAN", scan_result.1.len())),
                        db_type: "redis".to_string(),
                        target_host: target.host.clone(),
                    });
                }
            }
        }

        // Misconfig: dangerous commands (FLUSHALL, FLUSHDB, CONFIG SET, DEBUG, SHUTDOWN)
        if checks.iter().any(|c| matches!(c, CheckType::Misconfig)) {
            let dangerous_commands = ["FLUSHALL", "FLUSHDB", "CONFIG", "DEBUG", "SHUTDOWN"];
            for cmd_name in &dangerous_commands {
                if report.queries_executed >= max_queries {
                    break;
                }

                let cc = client_ref.clone();
                let cmd_owned = cmd_name.to_string();
                let cmd_result = tokio::task::spawn_blocking(move || -> Result<bool> {
                    let mut conn = cc.get_connection()?;
                    // DRYRUN executes the command without side effects; if the command
                    // is blocked via rename-command or ACL, this returns an error.
                    let result: ::redis::RedisResult<String> = ::redis::cmd("CMD")
                        .arg("DRYRUN")
                        .arg(&cmd_owned)
                        .query(&mut conn);
                    Ok(result.is_ok())
                })
                .await??;
                report.queries_executed += 1;

                if cmd_result {
                    let (severity, category_suffix) = match *cmd_name {
                        "FLUSHALL" | "FLUSHDB" => (Severity::High, "dangerous-command-flush"),
                        "CONFIG" | "SHUTDOWN" => (Severity::Medium, "dangerous-command-config"),
                        "DEBUG" => (Severity::Low, "dangerous-command-debug"),
                        _ => (Severity::Low, "dangerous-command-other"),
                    };

                    report.findings.push(DbFinding {
                        category: format!("db-redis-misconfig-{}", category_suffix),
                        severity,
                        title: format!("Dangerous command {} is available", cmd_name),
                        description: format!(
                            "CMD DRYRUN {} succeeded — the command is not renamed or ACL-blocked. {}",
                            cmd_name,
                            match *cmd_name {
                                "FLUSHALL" => "FLUSHALL can delete all keys across all databases.",
                                "FLUSHDB" => "FLUSHDB can delete all keys in the current database.",
                                "CONFIG" => "CONFIG SET can modify runtime parameters including bind, requirepass, and save paths.",
                                "DEBUG" => "DEBUG can crash or destabilize the server; some subcommands have side effects.",
                                "SHUTDOWN" => "SHUTDOWN stops the Redis server process.",
                                _ => "",
                            }
                        ),
                        recommendation: format!(
                            "Consider renaming or disabling {} via rename-command in redis.conf or ACL command restrictions.",
                            cmd_name
                        ),
                        evidence: Some(format!("CMD DRYRUN {} -> available", cmd_name)),
                        db_type: "redis".to_string(),
                        target_host: target.host.clone(),
                    });
                }
            }
        }

        report.actions_performed.push(format!(
            "redis: completed with ~{} queries executed (budget {})",
            report.queries_executed, max_queries
        ));

        Ok(())
    }
}

#[cfg(feature = "redis")]
pub async fn run_redis_checks(
    target: &DbTarget,
    report: &mut DbPentestReport,
    checks: &[CheckType],
    max_queries: u64,
    client: Option<&::redis::Client>,
) -> Result<()> {
    real::run_redis_checks_real(target, report, checks, max_queries, client).await
}

#[cfg(not(feature = "redis"))]
pub async fn run_redis_checks(
    target: &DbTarget,
    report: &mut DbPentestReport,
    checks: &[CheckType],
    max_queries: u64,
    _client: Option<&()>,
) -> Result<()> {
    // Fallback: no redis driver in this build — synthesize representative findings
    // (real dry-run path in utils::populate_dry_run_findings covers all categories)
    report.actions_performed.push("redis: real execution prepared (redis driver behind db-pentest-redis marker; enable when dep resolves). Used synthetic for this run.".to_string());
    crate::db_pentest::utils::populate_dry_run_findings(report, target, checks, max_queries);
    Ok(())
}
