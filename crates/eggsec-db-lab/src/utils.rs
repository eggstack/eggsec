//! Utilities: connection string parsing, redaction, dry-run population, target normalization,
//! and shared URL builders for all database engines.

use crate::types::{CheckType, DbFinding, DbPentestReport, DbTarget};
use eggsec_core::types::Severity;

/// Redact password from a connection URL, keeping the user and replacing the password with `***`.
pub fn redacted_conn_string(s: &str) -> String {
    if let Some(at) = s.find('@') {
        if let Some(scheme_end) = s.find("://") {
            let scheme = &s[..scheme_end + 3];
            let rest = &s[at + 1..];
            let user_part = &s[scheme_end + 3..at];
            // If there's a colon, there's a password to redact; otherwise just keep the user
            if user_part.contains(':') {
                let user = user_part.split(':').next().unwrap_or(user_part);
                return format!("{}{}:***@{}", scheme, user, rest);
            } else {
                return format!("{}{}@{}", scheme, user_part, rest);
            }
        }
    }
    s.to_string()
}

// ---------------------------------------------------------------------------
// Shared URL builders (consolidated from engine modules and advanced.rs)
// ---------------------------------------------------------------------------

/// Build a `postgres://` connection URL from a `DbTarget`.
pub fn build_postgres_url(t: &DbTarget) -> String {
    let db = t.database.as_deref().unwrap_or("postgres");
    if let Some(ref p) = t.password {
        format!("postgres://{}:{}@{}:{}/{}", t.user, p, t.host, t.port, db)
    } else {
        format!("postgres://{}@{}:{}/{}", t.user, t.host, t.port, db)
    }
}

/// Build a `mysql://` connection URL from a `DbTarget`.
pub fn build_mysql_url(t: &DbTarget) -> String {
    let db = t.database.as_deref().unwrap_or("mysql");
    if let Some(ref p) = t.password {
        format!("mysql://{}:{}@{}:{}/{}", t.user, p, t.host, t.port, db)
    } else {
        format!("mysql://{}@{}:{}/{}", t.user, t.host, t.port, db)
    }
}

/// Build a `mongodb://` connection URL from a `DbTarget` (authSource=admin).
pub fn build_mongodb_url(t: &DbTarget) -> String {
    let db = t.database.as_deref().unwrap_or("admin");
    let base = if let Some(ref p) = t.password {
        format!("mongodb://{}:{}", t.user, p)
    } else {
        format!("mongodb://{}", t.user)
    };
    format!("{}@{}:{}/{}?authSource=admin", base, t.host, t.port, db)
}

/// Build a `redis://` connection URL from a `DbTarget`.
pub fn build_redis_url(t: &DbTarget) -> String {
    if let Some(ref p) = t.password {
        format!("redis://{}:{}@{}:{}", t.user, p, t.host, t.port)
    } else {
        format!("redis://{}@{}:{}", t.user, t.host, t.port)
    }
}

/// Generate a simple hex timestamp-based identifier (for UDF names, etc.).
pub fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:x}", t.as_nanos())
}

pub fn parse_target_or_components(
    target: Option<&str>,
    db_type_hint: Option<&str>,
    host: Option<&str>,
    port: Option<u16>,
    user: Option<&str>,
    password: Option<&str>,
    database: Option<&str>,
) -> anyhow::Result<DbTarget> {
    if let Some(t) = target {
        // postgres://user:pass@host:5432/dbname or mysql://... or mssql://... / sqlserver://...
        if let Some(rest) = t
            .strip_prefix("postgres://")
            .or_else(|| t.strip_prefix("postgresql://"))
        {
            return parse_urlish(rest, "postgres", port, user, password, database);
        }
        if let Some(rest) = t.strip_prefix("mysql://") {
            return parse_urlish(rest, "mysql", port, user, password, database);
        }
        if let Some(rest) = t
            .strip_prefix("mssql://")
            .or_else(|| t.strip_prefix("sqlserver://"))
        {
            return parse_urlish(rest, "mssql", port, user, password, database);
        }
        if let Some(rest) = t
            .strip_prefix("mongodb://")
            .or_else(|| t.strip_prefix("mongodb+srv://"))
        {
            return parse_urlish(rest, "mongodb", port, user, password, database);
        }
        if let Some(rest) = t
            .strip_prefix("redis://")
            .or_else(|| t.strip_prefix("rediss://"))
        {
            return parse_urlish(rest, "redis", port, user, password, database);
        }
        // fallback: treat as host or host:port (default postgres for backward compat in components path)
        let (h, p) = split_host_port(t, 5432);
        return Ok(DbTarget {
            db_type: db_type_hint.unwrap_or("postgres").to_string(),
            host: h,
            port: p,
            user: user.unwrap_or("postgres").to_string(),
            database: database.map(|s| s.to_string()),
            password: password.map(|s| s.to_string()),
        });
    }

    // components path
    let h = host.unwrap_or("127.0.0.1").to_string();
    let p = port.unwrap_or(5432);
    let u = user.unwrap_or("postgres").to_string();
    let dt = db_type_hint.unwrap_or("postgres").to_string();
    Ok(DbTarget {
        db_type: dt,
        host: h,
        port: p,
        user: u,
        database: database.map(|s| s.to_string()),
        password: password.map(|s| s.to_string()),
    })
}

fn parse_urlish(
    rest: &str,
    db_type: &str,
    port_override: Option<u16>,
    user_override: Option<&str>,
    pass_override: Option<&str>,
    db_override: Option<&str>,
) -> anyhow::Result<DbTarget> {
    // user:pass@host:port/db?...
    let userinfo_and_host = rest;
    let (userinfo, hostport_db) = if let Some(at) = userinfo_and_host.find('@') {
        (&userinfo_and_host[..at], &userinfo_and_host[at + 1..])
    } else {
        ("", userinfo_and_host)
    };
    let (user, pass) = if !userinfo.is_empty() {
        if let Some(col) = userinfo.find(':') {
            let u = userinfo[..col].to_string();
            let p = Some(userinfo[col + 1..].to_string());
            if u.is_empty() {
                let default_user = if db_type == "mssql" {
                    "sa"
                } else if db_type == "mongodb" {
                    "admin"
                } else if db_type == "redis" {
                    "default"
                } else {
                    "postgres"
                };
                (default_user.to_string(), p)
            } else {
                (u, p)
            }
        } else {
            (userinfo.to_string(), None)
        }
    } else {
        let default_user = if db_type == "mssql" {
            "sa"
        } else if db_type == "mongodb" {
            "admin"
        } else if db_type == "redis" {
            "default"
        } else {
            "postgres"
        };
        (default_user.to_string(), None)
    };
    let (hostport, dbq) = if let Some(sl) = hostport_db.find('/') {
        (&hostport_db[..sl], Some(&hostport_db[sl + 1..]))
    } else {
        (hostport_db, None)
    };
    let default_port = if db_type == "mysql" {
        3306
    } else if db_type == "mssql" {
        1433
    } else if db_type == "mongodb" {
        27017
    } else if db_type == "redis" {
        6379
    } else {
        5432
    };
    let (host, port) = split_host_port(hostport, default_port);
    let db = if let Some(dq) = dbq {
        dq.split('?').next().unwrap_or(dq).to_string()
    } else {
        String::new()
    };

    Ok(DbTarget {
        db_type: db_type.to_string(),
        host,
        port: port_override.unwrap_or(port),
        user: user_override.unwrap_or(&user).to_string(),
        database: db_override.map(|s| s.to_string()).or(if db.is_empty() {
            None
        } else {
            Some(db)
        }),
        password: pass_override.map(|s| s.to_string()).or(pass),
    })
}

fn split_host_port(s: &str, default_port: u16) -> (String, u16) {
    if let Some(col) = s.rfind(':') {
        // crude ipv6 guard: if more than one :, treat as bare host
        if s.matches(':').count() > 1 {
            return (s.to_string(), default_port);
        }
        let h = s[..col].to_string();
        if let Ok(p) = s[col + 1..].parse::<u16>() {
            return (h, p);
        }
        (s.to_string(), default_port)
    } else {
        (s.to_string(), default_port)
    }
}

pub fn populate_dry_run_findings(
    report: &mut DbPentestReport,
    target: &DbTarget,
    checks: &[CheckType],
    _max_q: u64,
) {
    report
        .actions_performed
        .push("dry-run: synthesizing representative findings for all requested checks".to_string());

    if checks
        .iter()
        .any(|c| matches!(c, CheckType::Connection | CheckType::Auth))
    {
        report.findings.push(DbFinding {
            category: format!("db-{}-auth-ok", target.db_type),
            severity: Severity::Info,
            title: "Connection and basic auth successful (dry-run synthetic)".to_string(),
            description: "In a real run this would confirm supported auth mechanisms and detect weak/default accounts via safe probes.".to_string(),
            recommendation: "Ensure strong passwords, disable trust authentication, and prefer SCRAM/IDENT over cleartext where possible.".to_string(),
            evidence: Some(format!("host={} port={} user={}", target.host, target.port, target.user)),
            db_type: target.db_type.clone(),
            target_host: target.host.clone(),
        });
    }

    if checks.iter().any(|c| matches!(c, CheckType::Misconfig)) {
        if target.db_type == "postgres" {
            report.findings.push(DbFinding {
                category: "db-postgres-misconfig-dangerous-extension".to_string(),
                severity: Severity::High,
                title: "Dangerous extension 'pg_read_server_files' present and grantable".to_string(),
                description: "The extension allows reading server files via SQL; if granted to non-superuser roles this is a high-risk vector.".to_string(),
                recommendation: "Revoke from non-superusers; only install extensions required by the workload.".to_string(),
                evidence: Some("dry-run: would have queried pg_available_extensions and pg_extension".to_string()),
                db_type: target.db_type.clone(),
                target_host: target.host.clone(),
            });
        } else if target.db_type == "mysql" {
            report.findings.push(DbFinding {
                category: "db-mysql-misconfig-local-infile".to_string(),
                severity: Severity::Medium,
                title: "MySQL global 'local_infile' is enabled".to_string(),
                description: "Enables LOAD DATA LOCAL INFILE which can be abused for file reads on client or server depending on configuration.".to_string(),
                recommendation: "Set local_infile=OFF in my.cnf or via SET PERSIST; review secure_file_priv.".to_string(),
                evidence: Some("dry-run: would have queried @@local_infile and @@secure_file_priv".to_string()),
                db_type: target.db_type.clone(),
                target_host: target.host.clone(),
            });
        } else if target.db_type == "mssql" {
            report.findings.push(DbFinding {
                category: "db-mssql-misconfig-xp-cmdshell".to_string(),
                severity: Severity::High,
                title: "xp_cmdshell enabled (dry-run synthetic)".to_string(),
                description: "xp_cmdshell allows OS command execution from T-SQL. High risk surface if accessible beyond controlled lab accounts.".to_string(),
                recommendation: "Disable xp_cmdshell (sp_configure) for production-equivalent lab baselines.".to_string(),
                evidence: Some("dry-run: would have queried sys.configurations for xp_cmdshell".to_string()),
                db_type: target.db_type.clone(),
                target_host: target.host.clone(),
            });
        } else if target.db_type == "mongodb" {
            report.findings.push(DbFinding {
                category: "db-mongodb-misconfig-javascript".to_string(),
                severity: Severity::Medium,
                title: "Server-side JavaScript enabled (dry-run synthetic)".to_string(),
                description: "Enables $where and mapReduce JS expressions; increases RCE surface if injection is possible.".to_string(),
                recommendation: "Disable security.javascriptEnabled unless required; prefer Aggregation Pipeline.".to_string(),
                evidence: Some("dry-run: would have queried serverStatus for security.javascriptEnabled".to_string()),
                db_type: target.db_type.clone(),
                target_host: target.host.clone(),
            });
            report.findings.push(DbFinding {
                category: "db-mongodb-auth-noauth".to_string(),
                severity: Severity::High,
                title: "No authenticated users detected (dry-run synthetic)".to_string(),
                description: "Connection established without authenticated users; server may have security.authorization disabled.".to_string(),
                recommendation: "Enable security.authorization in mongod.conf and create dedicated users.".to_string(),
                evidence: Some("dry-run: would have queried connectionStatus".to_string()),
                db_type: target.db_type.clone(),
                target_host: target.host.clone(),
            });
        } else if target.db_type == "redis" {
            report.findings.push(DbFinding {
                category: "db-redis-misconfig-dangerous-command".to_string(),
                severity: Severity::High,
                title: "Dangerous command FLUSHALL accessible (dry-run synthetic)".to_string(),
                description: "FLUSHALL can delete all data across all databases. If accessible without ACL restriction, risk of data loss.".to_string(),
                recommendation: "Restrict FLUSHALL via ACL or rename-command in redis.conf.".to_string(),
                evidence: Some("dry-run: would have tested CMD DRYRUN FLUSHALL".to_string()),
                db_type: target.db_type.clone(),
                target_host: target.host.clone(),
            });
            report.findings.push(DbFinding {
                category: "db-redis-auth-noauth".to_string(),
                severity: Severity::High,
                title: "No authentication required (dry-run synthetic)".to_string(),
                description: "Redis instance accessible without password; any network-reachable client can execute commands.".to_string(),
                recommendation: "Set requirepass or configure ACL users with appropriate permissions.".to_string(),
                evidence: Some("dry-run: would have checked CONFIG GET requirepass".to_string()),
                db_type: target.db_type.clone(),
                target_host: target.host.clone(),
            });
        }
    }

    if checks
        .iter()
        .any(|c| matches!(c, CheckType::Privs | CheckType::Enum))
    {
        let priv_desc = if target.db_type == "mssql" {
            "Role had broad grants (e.g. sysadmin or CONTROL SERVER) not required for workload."
        } else {
            "Role had broad grants (e.g. pg_read_all_data or FILE) not required for workload."
        };
        report.findings.push(DbFinding {
            category: format!("db-{}-priv-excessive", target.db_type),
            severity: Severity::Medium,
            title: "Excessive privileges detected on application role (dry-run)".to_string(),
            description: priv_desc.to_string(),
            recommendation:
                "Apply principle of least privilege; use read-only roles for reporting users."
                    .to_string(),
            evidence: Some(format!(
                "dry-run: would have sampled {} privilege views with row limit",
                target.db_type
            )),
            db_type: target.db_type.clone(),
            target_host: target.host.clone(),
        });
    }

    if checks
        .iter()
        .any(|c| matches!(c, CheckType::Version | CheckType::Cve))
    {
        let example = if target.db_type == "mssql" {
            "Microsoft SQL Server 2019 (RTM-CU27) ..."
        } else if target.db_type == "mysql" {
            "8.0.36-0ubuntu0.22.04.1"
        } else if target.db_type == "mongodb" {
            "7.0.4"
        } else if target.db_type == "redis" {
            "7.2.4"
        } else {
            "PostgreSQL 14.5 on x86_64-pc-linux-gnu"
        };
        report.findings.push(DbFinding {
            category: format!("db-{}-version", target.db_type),
            severity: Severity::Info,
            title: "Version fingerprint (dry-run synthetic)".to_string(),
            description:
                "Version string would be obtained via safe method (e.g. version() or @@version)."
                    .to_string(),
            recommendation:
                "Keep DB engine patched; cross-reference disclosed CVEs against your exposure."
                    .to_string(),
            evidence: Some(format!("dry-run: e.g. {}", example)),
            db_type: target.db_type.clone(),
            target_host: target.host.clone(),
        });
    }

    // Always add a positive control finding so report shape is exercised
    if report.findings.is_empty() {
        report.findings.push(DbFinding {
            category: "db-info-dry-run-baseline".to_string(),
            severity: Severity::Info,
            title: "Dry-run baseline executed successfully".to_string(),
            description:
                "All requested check categories were exercised without any database interaction."
                    .to_string(),
            recommendation:
                "Use this report shape to validate downstream consumers (bridge, SARIF, CI)."
                    .to_string(),
            evidence: None,
            db_type: target.db_type.clone(),
            target_host: target.host.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction_basic() {
        let s = "postgres://alice:secret@db.internal:5432/app";
        let r = redacted_conn_string(s);
        assert!(r.contains("alice:***@"));
        assert!(!r.contains("secret"));
    }

    #[test]
    fn target_parse_postgres_url() {
        let t = parse_target_or_components(
            Some("postgres://u:p@h:5433/d"),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(t.db_type, "postgres");
        assert_eq!(t.host, "h");
        assert_eq!(t.port, 5433);
        assert_eq!(t.user, "u");
        assert_eq!(t.database.as_deref(), Some("d"));
    }

    #[test]
    fn dry_run_populates_report() {
        let mut r = DbPentestReport::new("postgres://u@h/db", "postgres");
        let tgt = DbTarget {
            db_type: "postgres".into(),
            host: "h".into(),
            port: 5432,
            user: "u".into(),
            database: Some("db".into()),
            password: None,
        };
        populate_dry_run_findings(&mut r, &tgt, &[CheckType::Misconfig], 10);
        assert!(!r.findings.is_empty());
        assert!(r.dry_run == false); // caller sets it
    }

    #[test]
    fn target_parse_mssql_url() {
        let t = parse_target_or_components(
            Some("mssql://sa:pass@sqlserver.lab:1433/master"),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(t.db_type, "mssql");
        assert_eq!(t.host, "sqlserver.lab");
        assert_eq!(t.port, 1433);
        assert_eq!(t.user, "sa");
        assert_eq!(t.database.as_deref(), Some("master"));
    }

    #[test]
    fn dry_run_mssql_populates_mssql_categories() {
        let mut r = DbPentestReport::new("mssql://sa@sql:1433/master", "mssql");
        let tgt = DbTarget {
            db_type: "mssql".into(),
            host: "sql".into(),
            port: 1433,
            user: "sa".into(),
            database: Some("master".into()),
            password: None,
        };
        populate_dry_run_findings(
            &mut r,
            &tgt,
            &[CheckType::Misconfig, CheckType::Privs, CheckType::Version],
            50,
        );
        let cats: Vec<_> = r.findings.iter().map(|f| f.category.as_str()).collect();
        assert!(
            cats.iter().any(|c| c.contains("mssql")),
            "expected at least one db-mssql-* category in dry-run mssql"
        );
        assert!(r.findings.iter().any(|f| f.category.contains("xp-cmdshell")
            || f.category.contains("priv-excessive")
            || f.category.contains("version")));
    }

    #[test]
    fn target_parse_redis_url() {
        let t = parse_target_or_components(
            Some("redis://:pass@redis.lab:6379/0"),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(t.db_type, "redis");
        assert_eq!(t.host, "redis.lab");
        assert_eq!(t.port, 6379);
        assert_eq!(t.user, "default");
        assert_eq!(t.database.as_deref(), Some("0"));
    }

    #[test]
    fn target_parse_redis_url_no_password() {
        let t = parse_target_or_components(
            Some("redis://redis.host"),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(t.db_type, "redis");
        assert_eq!(t.host, "redis.host");
        assert_eq!(t.port, 6379);
    }

    #[test]
    fn dry_run_redis_populates_redis_categories() {
        let mut r = DbPentestReport::new("redis://r:6379/0", "redis");
        let tgt = DbTarget {
            db_type: "redis".into(),
            host: "r".into(),
            port: 6379,
            user: "default".into(),
            database: Some("0".into()),
            password: None,
        };
        populate_dry_run_findings(
            &mut r,
            &tgt,
            &[CheckType::Misconfig, CheckType::Auth, CheckType::Version],
            50,
        );
        let cats: Vec<_> = r.findings.iter().map(|f| f.category.as_str()).collect();
        assert!(
            cats.iter().any(|c| c.contains("redis")),
            "expected at least one db-redis-* category in dry-run redis"
        );
        assert!(r
            .findings
            .iter()
            .any(|f| f.category.contains("dangerous-command")
                || f.category.contains("noauth")
                || f.category.contains("version")));
    }

    #[test]
    fn target_parse_mongodb_url() {
        let t = parse_target_or_components(
            Some("mongodb://admin:pass@mongo.lab:27017/admin"),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(t.db_type, "mongodb");
        assert_eq!(t.host, "mongo.lab");
        assert_eq!(t.port, 27017);
        assert_eq!(t.user, "admin");
        assert_eq!(t.database.as_deref(), Some("admin"));
    }

    // --- Shared URL builder tests ---

    fn target(
        db_type: &str,
        user: &str,
        pass: Option<&str>,
        host: &str,
        port: u16,
        db: Option<&str>,
    ) -> DbTarget {
        DbTarget {
            db_type: db_type.to_string(),
            host: host.to_string(),
            port,
            user: user.to_string(),
            database: db.map(|s| s.to_string()),
            password: pass.map(|s| s.to_string()),
        }
    }

    #[test]
    fn build_postgres_url_with_password() {
        let t = target(
            "postgres",
            "admin",
            Some("secret"),
            "db.local",
            5433,
            Some("mydb"),
        );
        assert_eq!(
            build_postgres_url(&t),
            "postgres://admin:secret@db.local:5433/mydb"
        );
    }

    #[test]
    fn build_postgres_url_without_password() {
        let t = target("postgres", "admin", None, "db.local", 5432, None);
        assert_eq!(
            build_postgres_url(&t),
            "postgres://admin@db.local:5432/postgres"
        );
    }

    #[test]
    fn build_mysql_url_with_password() {
        let t = target("mysql", "root", Some("pw"), "db.local", 3306, Some("app"));
        assert_eq!(build_mysql_url(&t), "mysql://root:pw@db.local:3306/app");
    }

    #[test]
    fn build_mysql_url_without_password() {
        let t = target("mysql", "root", None, "db.local", 3306, None);
        assert_eq!(build_mysql_url(&t), "mysql://root@db.local:3306/mysql");
    }

    #[test]
    fn build_mongodb_url_with_password() {
        let t = target(
            "mongodb",
            "admin",
            Some("secret"),
            "mongo.lab",
            27017,
            Some("admin"),
        );
        let url = build_mongodb_url(&t);
        assert!(url.starts_with("mongodb://admin:secret@mongo.lab:27017/admin?authSource=admin"));
    }

    #[test]
    fn build_mongodb_url_without_password() {
        let t = target("mongodb", "admin", None, "mongo.lab", 27017, None);
        let url = build_mongodb_url(&t);
        assert!(url.starts_with("mongodb://admin@mongo.lab:27017/admin?authSource=admin"));
    }

    #[test]
    fn build_redis_url_with_password() {
        let t = target("redis", "default", Some("rpass"), "redis.lab", 6379, None);
        assert_eq!(build_redis_url(&t), "redis://default:rpass@redis.lab:6379");
    }

    #[test]
    fn build_redis_url_without_password() {
        let t = target("redis", "default", None, "redis.lab", 6379, None);
        assert_eq!(build_redis_url(&t), "redis://default@redis.lab:6379");
    }

    #[test]
    fn uuid_simple_produces_hex() {
        let id = uuid_simple();
        assert!(!id.is_empty());
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn redaction_no_password() {
        let s = "postgres://alice@db.internal:5432/app";
        assert_eq!(redacted_conn_string(s), s);
    }

    #[test]
    fn redaction_no_scheme() {
        let s = "just-a-host";
        assert_eq!(redacted_conn_string(s), s);
    }
}
