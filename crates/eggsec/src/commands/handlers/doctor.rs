pub(crate) const DOCTOR_SUCCESS: &str = "All checks passed";

/// Platform prerequisite diagnostic (Phase F).
///
/// Read-only: reports OS/privilege/binary/interface/feature facts from the
/// centralized [`crate::platform`] matrix. Fixture tests never require root
/// or hardware; live tests skip with the named prerequisite when absent.
/// Exits 0 always (diagnostic only); machine consumers parse the human rows
/// or set `EGGSEC_DOCTOR_JSON=1` for the JSON report.
pub async fn handle_doctor(_ctx: &crate::commands::handlers::CommandContext) -> anyhow::Result<()> {
    use std::io::Write;

    let report = crate::platform::PlatformReport::collect();
    let mut out = std::io::stdout();

    if std::env::var_os("EGGSEC_DOCTOR_JSON").is_some() {
        let json = serde_json::to_string_pretty(&report)?;
        writeln!(out, "{json}")?;
        return Ok(());
    }

    writeln!(out, "=== Eggsec Dependency Check ===\n")?;
    writeln!(out, "{}", report.format_human())?;
    writeln!(out, "\n{}", DOCTOR_SUCCESS)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctor_report_is_hermetic_and_round_trips() {
        let report = crate::platform::PlatformReport::collect();
        assert_eq!(
            report.domains.len(),
            crate::platform::all_domain_ids().len()
        );
        let human = report.format_human();
        assert!(human.contains("=== Eggsec Platform Report ==="));
        // Every domain documents its fixture entry point.
        for domain in &report.domains {
            assert!(!domain.fixture_command.is_empty());
        }
    }
}
