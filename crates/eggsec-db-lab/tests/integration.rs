//! Integration tests for eggsec-db-lab domain crate.
//!
//! These tests verify that the domain crate's public API works correctly
//! and that types are properly exported.

use eggsec_db_lab::{
    CheckType, DbCorrelationEngine, DbFinding, DbPentestReport, DbPentestRunArgs, DbTarget,
    LabDbManifest,
};

#[test]
fn test_db_pentest_report_creation() {
    let target = DbTarget {
        db_type: "postgres".to_string(),
        host: "127.0.0.1".to_string(),
        port: 5432,
        user: "test".to_string(),
        database: Some("testdb".to_string()),
        password: Some("pass".to_string()),
    };

    let report = DbPentestReport::new(target.redacted_target(), &target.db_type);

    assert_eq!(report.db_type, "postgres");
    assert!(report.findings.is_empty());
    assert!(report.recommendations.is_empty());
    assert!(!report.dry_run);
    assert!(report.correlation.is_none());
    assert!(report.compliance.is_none());
}

#[test]
fn test_check_type_parse() {
    assert!(CheckType::parse("connection").is_some());
    assert!(CheckType::parse("auth").is_some());
    assert!(CheckType::parse("misconfig").is_some());
    assert!(CheckType::parse("privs").is_some());
    assert!(CheckType::parse("enum").is_some());
    assert!(CheckType::parse("version").is_some());
    assert!(CheckType::parse("cve").is_some());
    assert!(CheckType::parse("all").is_none()); // "all" is handled separately
    assert!(CheckType::parse("invalid").is_none());
}

#[test]
fn test_db_finding_creation() {
    let finding = DbFinding {
        category: "test-category".to_string(),
        severity: eggsec_core::types::Severity::High,
        title: "Test Finding".to_string(),
        description: "A test finding".to_string(),
        recommendation: "Fix it".to_string(),
        evidence: Some("evidence here".to_string()),
        db_type: "postgres".to_string(),
        target_host: "127.0.0.1".to_string(),
    };

    assert_eq!(finding.category, "test-category");
    assert_eq!(finding.severity, eggsec_core::types::Severity::High);
    assert_eq!(finding.db_type, "postgres");
}

#[test]
fn test_correlation_engine_empty() {
    let target = DbTarget {
        db_type: "postgres".to_string(),
        host: "127.0.0.1".to_string(),
        port: 5432,
        user: "test".to_string(),
        database: Some("testdb".to_string()),
        password: Some("pass".to_string()),
    };

    let report = DbPentestReport::new(target.redacted_target(), &target.db_type);
    let engine = DbCorrelationEngine::new();
    let result = engine.correlate(&report, &[]);

    assert!(result.correlations.is_empty());
    assert_eq!(result.summary.total_correlations, 0);
}

#[test]
fn test_manifest_empty_rules_permits_all() {
    let manifest = LabDbManifest::default();

    assert!(manifest.allows("127.0.0.1", 5432, "testdb"));
}

#[tokio::test]
async fn test_dry_run_produces_complete_report() {
    let args = DbPentestRunArgs {
        target: Some("postgres://test:pass@127.0.0.1:5432/testdb".to_string()),
        db_type: None,
        checks: "all".to_string(),
        max_queries: 100,
        max_duration: 60,
        lab_manifest: None,
        dry_run: true,
        json: false,
        output: None,
        allow_db_pentest: true,
        allow_db_pentest_advanced: false,
        manual_override_reason: None,
        quiet: true,
        evidence_bundle: None,
        baseline: None,
        baseline_label: None,
        capture_baseline: false,
        baseline_output: None,
        host: None,
        port: None,
        user: None,
        password: None,
        database: None,
    };

    let report = eggsec_db_lab::run_db_pentest(args).await.unwrap();

    assert!(report.dry_run);
    assert!(
        !report.findings.is_empty(),
        "dry-run should produce findings"
    );
    assert!(!report.actions_performed.is_empty());
    assert!(
        report.correlation.is_some(),
        "dry-run should produce correlation"
    );
    assert!(
        report.compliance.is_some(),
        "dry-run should produce compliance"
    );
}
