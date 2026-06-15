use super::PostexReport;

pub fn format_human_report(report: &PostexReport) -> String {
    let mut output = String::new();
    output.push_str("=== Post-Exploitation Simulation Report ===\n");
    output.push_str(&format!("Target: {}\n", report.target));
    output.push_str(&format!("Dry-run: {}\n", report.dry_run));
    output.push_str(&format!("Timestamp: {}\n", report.timestamp));
    output.push_str(&format!("Techniques: {}\n", report.summary.total));
    output.push_str(&format!("Simulated: {}\n", report.summary.simulated));
    output.push('\n');

    for d in &report.detections {
        let status = if d.simulated {
            "SIMULATED"
        } else {
            "NOT-SIMULATED"
        };
        output.push_str(&format!(
            "[{}] {} ({}) - confidence: {:.0}%\n",
            status,
            d.technique.name,
            d.technique.mitre_id,
            d.confidence * 100.0
        ));
        output.push_str(&format!("  Category: {}\n", d.technique.category));
        output.push_str(&format!("  Risk: {:?}\n", d.technique.risk));
        output.push_str(&format!("  Evidence: {}\n", d.evidence));
        output.push('\n');
    }

    output.push_str("Actions performed:\n");
    for action in &report.actions_performed {
        output.push_str(&format!("  - {}\n", action));
    }

    output
}

pub fn format_json_report(report: &PostexReport) -> Result<String, String> {
    serde_json::to_string_pretty(report).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::super::{PostexProfile, PostexScanner};
    use super::*;

    #[tokio::test]
    async fn test_format_human_report() {
        let scanner = PostexScanner::new(true, PostexProfile::Minimal);
        let report = scanner.scan("test-target").await.unwrap();
        let human = format_human_report(&report);
        assert!(human.contains("Post-Exploitation Simulation Report"));
        assert!(human.contains("test-target"));
        assert!(report.dry_run);
    }

    #[tokio::test]
    async fn test_format_json_report() {
        let scanner = PostexScanner::new(true, PostexProfile::Minimal);
        let report = scanner.scan("test-target").await.unwrap();
        let json = format_json_report(&report).unwrap();
        assert!(json.contains("test-target"));
        assert!(json.contains("dry_run"));
        assert!(json.contains("detections"));
    }
}
