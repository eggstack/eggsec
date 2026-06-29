use crate::types::{
    DbCorrelatedFinding, DbCorrelationResult, DbCorrelationSummary, DbCorrelationType, DbFinding,
    DbPentestReport,
};

/// A lightweight correlation note linking a db finding to a web SQLi finding.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DbCorrelationNote {
    pub db_finding_category: String,
    pub web_finding_category: String,
    pub correlation_type: String,
    pub note: String,
}

pub fn correlate_db_with_sqli(
    db_report: &DbPentestReport,
    web_findings: &[DbFinding],
) -> Vec<DbCorrelationNote> {
    let mut notes = Vec::new();

    let has_db_priv_excessive = db_report
        .findings
        .iter()
        .any(|f| f.category.contains("priv-excessive") || f.category.contains("file-priv"));
    let has_db_dangerous_ext = db_report
        .findings
        .iter()
        .any(|f| f.category.contains("dangerous-extension") || f.category.contains("xp-cmdshell"));
    let has_sqli = web_findings.iter().any(|f| f.category.contains("sqli"));

    if has_db_priv_excessive && has_sqli {
        notes.push(DbCorrelationNote {
            db_finding_category: "db-*-priv-excessive".to_string(),
            web_finding_category: "sqli-*".to_string(),
            correlation_type: "shared-privilege".to_string(),
            note: "Excessive DB privileges observed alongside SQLi vectors; privilege escalation risk if combined.".to_string(),
        });
    }

    if has_db_dangerous_ext && has_sqli {
        notes.push(DbCorrelationNote {
            db_finding_category: "db-*-dangerous-extension".to_string(),
            web_finding_category: "sqli-*".to_string(),
            correlation_type: "extension-sqli-vector".to_string(),
            note: "Dangerous extensions (file-read/xp_cmdshell) observed with SQLi vectors; server-side code execution risk.".to_string(),
        });
    }

    notes
}

#[derive(Debug, Clone)]
struct CorrelationRule {
    db_pattern: &'static str,
    web_pattern: &'static str,
    score: u8,
    correlation_type: DbCorrelationType,
    enrichment: &'static str,
}

const CORRELATION_RULES: &[CorrelationRule] = &[
    CorrelationRule {
        db_pattern: "priv-excessive",
        web_pattern: "sqli",
        score: 85,
        correlation_type: DbCorrelationType::Direct,
        enrichment: "excessive DB privileges compound SQLi risk",
    },
    CorrelationRule {
        db_pattern: "file-priv",
        web_pattern: "sqli",
        score: 85,
        correlation_type: DbCorrelationType::Direct,
        enrichment: "excessive DB privileges compound SQLi risk",
    },
    CorrelationRule {
        db_pattern: "dangerous-extension",
        web_pattern: "sqli",
        score: 90,
        correlation_type: DbCorrelationType::Direct,
        enrichment: "server-side execution vector via extension + SQLi",
    },
    CorrelationRule {
        db_pattern: "xp-cmdshell",
        web_pattern: "sqli",
        score: 90,
        correlation_type: DbCorrelationType::Direct,
        enrichment: "server-side execution vector via extension + SQLi",
    },
    CorrelationRule {
        db_pattern: "public-schema",
        web_pattern: "sqli",
        score: 65,
        correlation_type: DbCorrelationType::Indirect,
        enrichment: "public schema misconfig amplifies SQLi impact",
    },
    CorrelationRule {
        db_pattern: "local-infile",
        web_pattern: "sqli",
        score: 75,
        correlation_type: DbCorrelationType::Direct,
        enrichment: "file access misconfig + SQLi enables data exfil",
    },
    CorrelationRule {
        db_pattern: "secure-file-priv",
        web_pattern: "sqli",
        score: 75,
        correlation_type: DbCorrelationType::Direct,
        enrichment: "file access misconfig + SQLi enables data exfil",
    },
    CorrelationRule {
        db_pattern: "superuser",
        web_pattern: "sqli",
        score: 80,
        correlation_type: DbCorrelationType::Direct,
        enrichment: "privileged account + SQLi = full compromise",
    },
    CorrelationRule {
        db_pattern: "sysadmin",
        web_pattern: "sqli",
        score: 80,
        correlation_type: DbCorrelationType::Direct,
        enrichment: "privileged account + SQLi = full compromise",
    },
    CorrelationRule {
        db_pattern: "logging",
        web_pattern: "*",
        score: 30,
        correlation_type: DbCorrelationType::Indirect,
        enrichment: "weak logging reduces detection of SQLi exploitation",
    },
    CorrelationRule {
        db_pattern: "version",
        web_pattern: "*",
        score: 25,
        correlation_type: DbCorrelationType::Indirect,
        enrichment: "known version vulnerability + attack surface",
    },
    CorrelationRule {
        db_pattern: "cve",
        web_pattern: "*",
        score: 25,
        correlation_type: DbCorrelationType::Indirect,
        enrichment: "known version vulnerability + attack surface",
    },
    CorrelationRule {
        db_pattern: "linked-servers",
        web_pattern: "sqli",
        score: 70,
        correlation_type: DbCorrelationType::CrossLayer,
        enrichment: "linked server + SQLi enables lateral movement",
    },
    CorrelationRule {
        db_pattern: "tde",
        web_pattern: "*",
        score: 20,
        correlation_type: DbCorrelationType::Indirect,
        enrichment: "no encryption at rest noted",
    },
    // Phase 5: Cross-DB correlation rules for MongoDB and Redis
    CorrelationRule {
        db_pattern: "mongodb-auth-noauth",
        web_pattern: "sqli",
        score: 85,
        correlation_type: DbCorrelationType::Direct,
        enrichment: "unauthenticated MongoDB + SQLi = full data exfil across layers",
    },
    CorrelationRule {
        db_pattern: "mongodb-misconfig-javascript",
        web_pattern: "sqli",
        score: 80,
        correlation_type: DbCorrelationType::Direct,
        enrichment: "server-side JS injection + SQLi enables RCE on MongoDB",
    },
    CorrelationRule {
        db_pattern: "redis-auth-noauth",
        web_pattern: "sqli",
        score: 80,
        correlation_type: DbCorrelationType::Direct,
        enrichment: "unauthenticated Redis + SQLi enables cache poisoning and data exfil",
    },
    CorrelationRule {
        db_pattern: "redis-misconfig-dangerous-command",
        web_pattern: "sqli",
        score: 85,
        correlation_type: DbCorrelationType::Direct,
        enrichment: "dangerous Redis commands + SQLi enables data destruction via FLUSHALL",
    },
    CorrelationRule {
        db_pattern: "mongodb-priv-excessive",
        web_pattern: "sqli",
        score: 80,
        correlation_type: DbCorrelationType::Direct,
        enrichment: "excessive MongoDB privileges compound SQLi risk",
    },
    CorrelationRule {
        db_pattern: "redis-priv-excessive",
        web_pattern: "sqli",
        score: 75,
        correlation_type: DbCorrelationType::Direct,
        enrichment: "excessive Redis ACL permissions compound SQLi risk",
    },
    // Cross-engine behavioral rules (findings from different DB types in same session)
    CorrelationRule {
        db_pattern: "auth-noauth",
        web_pattern: "*",
        score: 60,
        correlation_type: DbCorrelationType::Behavioral,
        enrichment: "no-auth detected across multiple DB engines indicates systemic access control gap",
    },
    CorrelationRule {
        db_pattern: "priv-excessive",
        web_pattern: "*",
        score: 55,
        correlation_type: DbCorrelationType::Behavioral,
        enrichment: "excessive privileges across multiple engines indicates systemic least-privilege failure",
    },
    CorrelationRule {
        db_pattern: "misconfig-dangerous-command",
        web_pattern: "*",
        score: 50,
        correlation_type: DbCorrelationType::Behavioral,
        enrichment: "dangerous command access across engines indicates systemic hardening gap",
    },
];

#[derive(Debug, Clone, Default)]
pub struct DbCorrelationEngine {
    pub min_score: u8,
}

impl DbCorrelationEngine {
    pub fn new() -> Self {
        Self { min_score: 40 }
    }

    pub fn with_min_score(mut self, min_score: u8) -> Self {
        self.min_score = min_score.min(100);
        self
    }

    pub fn correlate(
        &self,
        db_report: &DbPentestReport,
        web_findings: &[DbFinding],
    ) -> DbCorrelationResult {
        let mut correlations = Vec::new();

        for db_finding in &db_report.findings {
            for rule in CORRELATION_RULES {
                if !db_finding.category.contains(rule.db_pattern) {
                    continue;
                }

                let web_match = if rule.web_pattern == "*" {
                    !web_findings.is_empty()
                } else {
                    web_findings
                        .iter()
                        .any(|f| f.category.contains(rule.web_pattern))
                };

                if !web_match {
                    continue;
                }

                if rule.score < self.min_score {
                    continue;
                }

                let web_cat = if rule.web_pattern == "*" {
                    web_findings
                        .first()
                        .map(|f| f.category.clone())
                        .unwrap_or_default()
                } else {
                    web_findings
                        .iter()
                        .find(|f| f.category.contains(rule.web_pattern))
                        .map(|f| f.category.clone())
                        .unwrap_or_default()
                };

                correlations.push(DbCorrelatedFinding {
                    db_finding_category: db_finding.category.clone(),
                    web_finding_category: web_cat,
                    note: rule.enrichment.to_string(),
                    score: Some(rule.score),
                    correlation_type: Some(rule.correlation_type),
                    enrichment: Some(rule.enrichment.to_string()),
                });
            }
        }

        let timeline = build_timeline(db_report);
        let total = correlations.len();
        let avg_confidence = if total == 0 {
            0
        } else {
            let sum: u32 = correlations
                .iter()
                .map(|c| c.score.unwrap_or(0) as u32)
                .sum();
            (sum / total as u32) as u8
        };

        DbCorrelationResult {
            correlations,
            timeline,
            summary: DbCorrelationSummary {
                total_correlations: total,
                avg_confidence,
            },
        }
    }
}

pub fn correlate_reports(
    db_report: &DbPentestReport,
    web_findings: &[DbFinding],
) -> DbCorrelationResult {
    DbCorrelationEngine::new().correlate(db_report, web_findings)
}

/// Cross-DB correlation: correlate findings from multiple DB engines in the same lab session.
/// Finds patterns that span different database types (e.g., no-auth on both MongoDB and Redis).
pub fn correlate_cross_db(
    reports: &[DbPentestReport],
    web_findings: &[DbFinding],
) -> DbCorrelationResult {
    DbCorrelationEngine::new().correlate_cross_db(reports, web_findings)
}

impl DbCorrelationEngine {
    /// Correlate findings across multiple heterogeneous DB reports.
    /// Each report's findings are checked against all rules; cross-engine behavioral rules
    /// fire when the same pattern appears in reports of different db_type.
    pub fn correlate_cross_db(
        &self,
        reports: &[DbPentestReport],
        web_findings: &[DbFinding],
    ) -> DbCorrelationResult {
        let mut correlations = Vec::new();

        // Single-report correlations (existing logic per report)
        for report in reports {
            let single = self.correlate(report, web_findings);
            correlations.extend(single.correlations);
        }

        // Cross-engine behavioral: find patterns appearing in multiple db_types
        if reports.len() > 1 {
            let all_findings: Vec<&DbFinding> =
                reports.iter().flat_map(|r| r.findings.iter()).collect();

            // Group findings by pattern family
            let mut pattern_families: std::collections::HashMap<String, Vec<&DbFinding>> =
                std::collections::HashMap::new();
            for f in &all_findings {
                for rule in CORRELATION_RULES {
                    if rule.correlation_type == DbCorrelationType::Behavioral
                        && f.category.contains(rule.db_pattern)
                    {
                        pattern_families
                            .entry(rule.db_pattern.to_string())
                            .or_default()
                            .push(f);
                    }
                }
            }

            for (pattern, findings) in &pattern_families {
                // Check if findings span multiple db_types
                let types: std::collections::HashSet<&str> =
                    findings.iter().map(|f| f.db_type.as_str()).collect();
                if types.len() > 1 {
                    // Find the enrichment text from rules
                    let enrichment = CORRELATION_RULES
                        .iter()
                        .find(|r| {
                            r.db_pattern == *pattern
                                && r.correlation_type == DbCorrelationType::Behavioral
                        })
                        .map(|r| r.enrichment)
                        .unwrap_or("cross-engine behavioral pattern");

                    let web_match = if web_findings.is_empty() {
                        false
                    } else {
                        web_findings.iter().any(|f| f.category.contains("sqli"))
                    };

                    let score = if web_match { 70 } else { 45 };
                    if score >= self.min_score {
                        correlations.push(DbCorrelatedFinding {
                            db_finding_category: format!("cross-db-{}", pattern),
                            web_finding_category: if web_match {
                                "sqli-*".to_string()
                            } else {
                                "*".to_string()
                            },
                            note: format!(
                                "Pattern '{}' found across {} engine types",
                                pattern,
                                types.len()
                            ),
                            score: Some(score),
                            correlation_type: Some(DbCorrelationType::Behavioral),
                            enrichment: Some(enrichment.to_string()),
                        });
                    }
                }
            }
        }

        // Deduplicate by (db_finding_category, web_finding_category)
        correlations.sort_by_key(|c| std::cmp::Reverse(c.score));
        correlations.dedup_by(|a, b| {
            a.db_finding_category == b.db_finding_category
                && a.web_finding_category == b.web_finding_category
        });

        let timeline = if let Some(first) = reports.first() {
            build_timeline(first)
        } else {
            Vec::new()
        };
        let total = correlations.len();
        let avg_confidence = if total == 0 {
            0
        } else {
            let sum: u32 = correlations
                .iter()
                .map(|c| c.score.unwrap_or(0) as u32)
                .sum();
            (sum / total as u32) as u8
        };

        DbCorrelationResult {
            correlations,
            timeline,
            summary: DbCorrelationSummary {
                total_correlations: total,
                avg_confidence,
            },
        }
    }
}

pub fn build_timeline(report: &DbPentestReport) -> Vec<String> {
    let mut timeline = Vec::new();
    timeline.push(format!("scan-start: {}", report.timestamp));
    for action in &report.actions_performed {
        timeline.push(format!("action: {}", action));
    }
    if report.dry_run {
        timeline.push("mode: dry-run".to_string());
    }
    timeline.push(format!(
        "scan-end: findings={}, duration={}ms",
        report.findings.len(),
        report.duration_ms
    ));
    timeline
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggsec_core::types::Severity;

    #[test]
    fn correlation_produces_notes_for_priv_and_sqli() {
        let mut db_report = DbPentestReport::new("postgres://u@h/db", "postgres");
        db_report.findings.push(DbFinding {
            category: "db-postgres-priv-excessive".to_string(),
            severity: Severity::Medium,
            title: "Excessive privs".to_string(),
            description: "desc".to_string(),
            recommendation: "fix".to_string(),
            evidence: None,
            db_type: "postgres".to_string(),
            target_host: "h".to_string(),
        });
        let web_findings = vec![DbFinding {
            category: "sqli-error-based".to_string(),
            severity: Severity::High,
            title: "SQLi".to_string(),
            description: "desc".to_string(),
            recommendation: "fix".to_string(),
            evidence: None,
            db_type: "web".to_string(),
            target_host: "h".to_string(),
        }];
        let notes = correlate_db_with_sqli(&db_report, &web_findings);
        assert!(!notes.is_empty());
        assert!(notes
            .iter()
            .any(|n| n.correlation_type == "shared-privilege"));
    }

    #[test]
    fn correlation_empty_when_no_match() {
        let db_report = DbPentestReport::new("postgres://u@h/db", "postgres");
        let notes = correlate_db_with_sqli(&db_report, &[]);
        assert!(notes.is_empty());
    }

    #[test]
    fn engine_produces_scored_results_for_priv_excessive_and_sqli() {
        let mut db_report = DbPentestReport::new("postgres://u@h/db", "postgres");
        db_report.findings.push(DbFinding {
            category: "db-postgres-priv-excessive".to_string(),
            severity: Severity::Medium,
            title: "Excessive privs".to_string(),
            description: "desc".to_string(),
            recommendation: "fix".to_string(),
            evidence: None,
            db_type: "postgres".to_string(),
            target_host: "h".to_string(),
        });
        let web_findings = vec![DbFinding {
            category: "sqli-error-based".to_string(),
            severity: Severity::High,
            title: "SQLi".to_string(),
            description: "desc".to_string(),
            recommendation: "fix".to_string(),
            evidence: None,
            db_type: "web".to_string(),
            target_host: "h".to_string(),
        }];
        let result = DbCorrelationEngine::new().correlate(&db_report, &web_findings);
        // Direct rule (85) + behavioral rule (55) both match
        assert!(result.correlations.len() >= 1);
        assert_eq!(result.correlations[0].score, Some(85));
        assert_eq!(
            result.correlations[0].correlation_type,
            Some(DbCorrelationType::Direct)
        );
        assert!(result.summary.total_correlations >= 1);
        // avg_confidence includes both direct (85) and behavioral (55) rules
        assert!(result.summary.avg_confidence >= 55);
    }

    #[test]
    fn engine_filters_by_min_score() {
        let mut db_report = DbPentestReport::new("postgres://u@h/db", "postgres");
        db_report.findings.push(DbFinding {
            category: "db-postgres-logging-low".to_string(),
            severity: Severity::Low,
            title: "Weak logging".to_string(),
            description: "desc".to_string(),
            recommendation: "fix".to_string(),
            evidence: None,
            db_type: "postgres".to_string(),
            target_host: "h".to_string(),
        });
        let web_findings = vec![DbFinding {
            category: "sqli-error-based".to_string(),
            severity: Severity::High,
            title: "SQLi".to_string(),
            description: "desc".to_string(),
            recommendation: "fix".to_string(),
            evidence: None,
            db_type: "web".to_string(),
            target_host: "h".to_string(),
        }];
        let result = DbCorrelationEngine::new()
            .with_min_score(50)
            .correlate(&db_report, &web_findings);
        assert!(result.correlations.is_empty());
        assert_eq!(result.summary.total_correlations, 0);
    }

    #[test]
    fn engine_empty_case_returns_empty() {
        let db_report = DbPentestReport::new("postgres://u@h/db", "postgres");
        let result = DbCorrelationEngine::new().correlate(&db_report, &[]);
        assert!(result.correlations.is_empty());
        assert_eq!(result.summary.total_correlations, 0);
        assert_eq!(result.summary.avg_confidence, 0);
    }

    #[test]
    fn build_timeline_entries() {
        let mut report = DbPentestReport::new("postgres://u@h/db", "postgres");
        report.actions_performed.push("loaded manifest".to_string());
        report.actions_performed.push("ran checks".to_string());
        report.dry_run = true;
        report.duration_ms = 150;
        let timeline = build_timeline(&report);
        assert!(timeline[0].contains("scan-start"));
        assert!(timeline.iter().any(|t| t.contains("loaded manifest")));
        assert!(timeline.iter().any(|t| t.contains("ran checks")));
        assert!(timeline.iter().any(|t| t.contains("dry-run")));
        assert!(timeline.iter().any(|t| t.contains("scan-end")));
    }

    #[test]
    fn engine_builder_with_min_score() {
        let engine = DbCorrelationEngine::new().with_min_score(75);
        assert_eq!(engine.min_score, 75);

        let engine = DbCorrelationEngine::new().with_min_score(200);
        assert_eq!(engine.min_score, 100);
    }

    #[test]
    fn engine_dangerous_extension_with_sqli_scores_90() {
        let mut db_report = DbPentestReport::new("mssql://u@h/db", "mssql");
        db_report.findings.push(DbFinding {
            category: "db-mssql-dangerous-extension".to_string(),
            severity: Severity::Critical,
            title: "Dangerous ext".to_string(),
            description: "desc".to_string(),
            recommendation: "fix".to_string(),
            evidence: None,
            db_type: "mssql".to_string(),
            target_host: "h".to_string(),
        });
        let web_findings = vec![DbFinding {
            category: "sqli-blind".to_string(),
            severity: Severity::High,
            title: "SQLi".to_string(),
            description: "desc".to_string(),
            recommendation: "fix".to_string(),
            evidence: None,
            db_type: "web".to_string(),
            target_host: "h".to_string(),
        }];
        let result = correlate_reports(&db_report, &web_findings);
        assert_eq!(result.correlations.len(), 1);
        assert_eq!(result.correlations[0].score, Some(90));
    }

    #[test]
    fn cross_db_correlates_mongodb_and_redis_noauth() {
        let mut mongo_report = DbPentestReport::new("mongodb://u@h/db", "mongodb");
        mongo_report.findings.push(DbFinding {
            category: "db-mongodb-auth-noauth".to_string(),
            severity: Severity::High,
            title: "No auth".to_string(),
            description: "desc".to_string(),
            recommendation: "fix".to_string(),
            evidence: None,
            db_type: "mongodb".to_string(),
            target_host: "h".to_string(),
        });
        let mut redis_report = DbPentestReport::new("redis://u@h/0", "redis");
        redis_report.findings.push(DbFinding {
            category: "db-redis-auth-noauth".to_string(),
            severity: Severity::High,
            title: "No auth".to_string(),
            description: "desc".to_string(),
            recommendation: "fix".to_string(),
            evidence: None,
            db_type: "redis".to_string(),
            target_host: "h".to_string(),
        });
        let result = correlate_cross_db(&[mongo_report, redis_report], &[]);
        // Should find cross-engine behavioral correlation for auth-noauth
        let cross_findings: Vec<_> = result
            .correlations
            .iter()
            .filter(|c| c.db_finding_category.starts_with("cross-db-"))
            .collect();
        assert!(
            !cross_findings.is_empty(),
            "expected cross-db behavioral correlation for auth-noauth across mongodb+redis"
        );
        assert!(cross_findings
            .iter()
            .any(|c| c.correlation_type == Some(DbCorrelationType::Behavioral)));
    }

    #[test]
    fn cross_db_no_correlation_single_engine() {
        let mut mongo_report = DbPentestReport::new("mongodb://u@h/db", "mongodb");
        mongo_report.findings.push(DbFinding {
            category: "db-mongodb-auth-noauth".to_string(),
            severity: Severity::High,
            title: "No auth".to_string(),
            description: "desc".to_string(),
            recommendation: "fix".to_string(),
            evidence: None,
            db_type: "mongodb".to_string(),
            target_host: "h".to_string(),
        });
        let result = correlate_cross_db(&[mongo_report], &[]);
        let cross_findings: Vec<_> = result
            .correlations
            .iter()
            .filter(|c| c.db_finding_category.starts_with("cross-db-"))
            .collect();
        assert!(
            cross_findings.is_empty(),
            "single engine should not produce cross-db behavioral correlations"
        );
    }

    #[test]
    fn cross_db_with_sqli_boosts_score() {
        let mut mongo_report = DbPentestReport::new("mongodb://u@h/db", "mongodb");
        mongo_report.findings.push(DbFinding {
            category: "db-mongodb-auth-noauth".to_string(),
            severity: Severity::High,
            title: "No auth".to_string(),
            description: "desc".to_string(),
            recommendation: "fix".to_string(),
            evidence: None,
            db_type: "mongodb".to_string(),
            target_host: "h".to_string(),
        });
        let mut redis_report = DbPentestReport::new("redis://u@h/0", "redis");
        redis_report.findings.push(DbFinding {
            category: "db-redis-auth-noauth".to_string(),
            severity: Severity::High,
            title: "No auth".to_string(),
            description: "desc".to_string(),
            recommendation: "fix".to_string(),
            evidence: None,
            db_type: "redis".to_string(),
            target_host: "h".to_string(),
        });
        let web_findings = vec![DbFinding {
            category: "sqli-error-based".to_string(),
            severity: Severity::High,
            title: "SQLi".to_string(),
            description: "desc".to_string(),
            recommendation: "fix".to_string(),
            evidence: None,
            db_type: "web".to_string(),
            target_host: "h".to_string(),
        }];
        let result = correlate_cross_db(&[mongo_report, redis_report], &web_findings);
        let cross_findings: Vec<_> = result
            .correlations
            .iter()
            .filter(|c| c.db_finding_category.starts_with("cross-db-"))
            .collect();
        assert!(!cross_findings.is_empty());
        // Score should be 70 when web findings present (boost from sqli)
        assert!(cross_findings.iter().any(|c| c.score == Some(70)));
    }
}
