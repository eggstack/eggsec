//! Integration tests for the agent module.
//!
//! Tests portfolio management and related functionality.

#[cfg(test)]
#[cfg(feature = "rest-api")]
mod tests {
    use slapper::agent::{Priority, ScanRecord, TargetConfig, TargetPortfolio};

    #[test]
    fn test_portfolio_target_crud() {
        let portfolio = TargetPortfolio::new();

        let config = TargetConfig {
            target: "https://example.com".to_string(),
            target_type: "url".to_string(),
            priority: Priority::High,
            schedule: Some("0 0 * * *".to_string()),
            alert_channels: vec!["webhook".to_string()],
            last_scan: None,
            scan_history: vec![],
            baseline_findings: vec![],
            enabled: true,
        };

        portfolio.add_target("example.com".to_string(), config.clone());

        assert_eq!(portfolio.targets_count(), 1);
        assert_eq!(portfolio.enabled_count(), 1);

        let retrieved = portfolio.get_target("example.com");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().priority, Priority::High);

        let removed = portfolio.remove_target("example.com");
        assert!(removed);
        assert_eq!(portfolio.targets_count(), 0);
    }

    #[test]
    fn test_portfolio_get_all_targets() {
        let portfolio = TargetPortfolio::new();

        portfolio.add_target(
            "target1.com".to_string(),
            TargetConfig {
                target: "https://target1.com".to_string(),
                enabled: true,
                ..Default::default()
            },
        );

        portfolio.add_target(
            "target2.com".to_string(),
            TargetConfig {
                target: "https://target2.com".to_string(),
                enabled: false,
                ..Default::default()
            },
        );

        portfolio.add_target(
            "target3.com".to_string(),
            TargetConfig {
                target: "https://target3.com".to_string(),
                enabled: true,
                ..Default::default()
            },
        );

        let all = portfolio.get_all_targets();
        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|(id, _)| id == "target1.com"));
        assert!(all.iter().any(|(id, _)| id == "target3.com"));
        assert!(!all.iter().any(|(id, _)| id == "target2.com"));
    }

    #[test]
    fn test_portfolio_update_last_scan() {
        let portfolio = TargetPortfolio::new();
        let timestamp = chrono::Utc::now();

        portfolio.add_target(
            "example.com".to_string(),
            TargetConfig {
                target: "https://example.com".to_string(),
                last_scan: None,
                ..Default::default()
            },
        );

        portfolio.update_last_scan("example.com", &timestamp);

        let target = portfolio.get_target("example.com").unwrap();
        assert!(target.last_scan.is_some());
    }

    #[test]
    fn test_portfolio_add_scan_record() {
        let portfolio = TargetPortfolio::new();

        portfolio.add_target(
            "example.com".to_string(),
            TargetConfig {
                target: "https://example.com".to_string(),
                scan_history: vec![],
                ..Default::default()
            },
        );

        let record = slapper::agent::ScanRecord {
            scan_id: "scan-001".to_string(),
            scan_type: "fuzz".to_string(),
            timestamp: chrono::Utc::now(),
            findings_count: 5,
            severity_counts: std::collections::HashMap::new(),
        };

        portfolio.add_scan_record("example.com", record);

        let target = portfolio.get_target("example.com").unwrap();
        assert_eq!(target.scan_history.len(), 1);
        assert_eq!(target.scan_history[0].scan_id, "scan-001");
    }

    #[test]
    fn test_portfolio_set_baseline() {
        let portfolio = TargetPortfolio::new();

        portfolio.add_target(
            "example.com".to_string(),
            TargetConfig {
                target: "https://example.com".to_string(),
                baseline_findings: vec![],
                ..Default::default()
            },
        );

        portfolio.set_baseline("example.com", vec!["finding-1".to_string(), "finding-2".to_string()]);

        let target = portfolio.get_target("example.com").unwrap();
        assert_eq!(target.baseline_findings.len(), 2);
    }

    #[test]
    fn test_portfolio_nonexistent_target_operations() {
        let portfolio = TargetPortfolio::new();

        let retrieved = portfolio.get_target("nonexistent.com");
        assert!(retrieved.is_none());

        let removed = portfolio.remove_target("nonexistent.com");
        assert!(!removed);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical.as_int() > Priority::High.as_int());
        assert!(Priority::High.as_int() > Priority::Normal.as_int());
        assert!(Priority::Normal.as_int() > Priority::Low.as_int());
        assert!(Priority::Critical.as_int() > Priority::Low.as_int());
    }

    #[test]
    fn test_priority_default() {
        let default = Priority::default();
        assert_eq!(default, Priority::Normal);
    }
}