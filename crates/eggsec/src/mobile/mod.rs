//! Mobile app security static analysis module (feature-gated behind `mobile`).
//!
//! Phase 1 (per plans/mobile-first-handoff-plan.md): reliable static analysis of
//! Android APKs and iOS IPAs in authorized lab/defense-validation environments.
//! Focus on high-signal manifest/config findings only (no dynamic instrumentation,
//! no Frida, no active exploitation, no full decompilation).
//!
//! Safety: pure-Rust ZIP + plist + bounded AXML extraction. No shelling out.
//! All operations are offline on user-supplied lab binaries. Explicit lab-only framing.

use crate::error::{EggsecError, Result};
use crate::types::Severity;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod apk;
pub mod ipa;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MobilePlatform {
    Android,
    Ios,
}

impl MobilePlatform {
    pub fn as_str(&self) -> &str {
        match self {
            MobilePlatform::Android => "android",
            MobilePlatform::Ios => "ios",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileFinding {
    pub category: String,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub recommendation: String,
    /// Optional structured evidence (e.g. permission name, component, key pattern)
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileScanReport {
    pub target: String,
    pub scan_type: String,
    pub platform: MobilePlatform,
    pub app_id: Option<String>,
    pub version: Option<String>,
    pub timestamp: String,
    pub findings: Vec<MobileFinding>,
    pub recommendations: Vec<String>,
    pub duration_ms: u64,
}

impl MobileScanReport {
    pub fn new(path: &str, platform: MobilePlatform) -> Self {
        Self {
            target: path.to_string(),
            scan_type: "mobile-static".to_string(),
            platform,
            app_id: None,
            version: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            findings: Vec::new(),
            recommendations: Vec::new(),
            duration_ms: 0,
        }
    }
}

/// High-level entry for CLI handlers. Validates path, dispatches to APK or IPA
/// parser, runs analysis, formats output (json/human), writes -o if provided.
pub async fn run_cli(args: crate::cli::MobileArgs, _config: &crate::config::EggsecConfig) -> Result<()> {
    let start = std::time::Instant::now();

    let path = Path::new(&args.path);
    if !path.exists() {
        return Err(EggsecError::Validation(format!("Path does not exist: {}", args.path)));
    }
    if !path.is_file() {
        return Err(EggsecError::Validation(format!("Path is not a file: {}", args.path)));
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if !matches!(ext.as_str(), "apk" | "ipa") {
        return Err(EggsecError::Validation(
            "Mobile static analysis supports only .apk (Android) or .ipa (iOS) files".to_string(),
        ));
    }

    // Size guard (defensive; static analysis should be bounded)
    if let Ok(meta) = std::fs::metadata(path) {
        const MAX: u64 = 200 * 1024 * 1024; // 200 MiB
        if meta.len() > MAX {
            return Err(EggsecError::Validation(format!(
                "Mobile artifact too large for static analysis ({} bytes > {} MiB limit)",
                meta.len(),
                MAX / 1024 / 1024
            )));
        }
    }

    if !args.quiet {
        eprintln!(
            "NOTE: Mobile static analysis is for authorized lab/defensive validation use only. \
             Provide your own test builds. No dynamic analysis or instrumentation is performed."
        );
    }

    let mut report = if ext == "apk" {
        let mut r = apk::analyze_apk(path).await?;
        r.scan_type = "mobile-static".to_string();
        r.timestamp = chrono::Utc::now().to_rfc3339();
        r.duration_ms = start.elapsed().as_millis() as u64;
        r
    } else {
        let mut r = ipa::analyze_ipa(path).await?;
        r.scan_type = "mobile-static".to_string();
        r.timestamp = chrono::Utc::now().to_rfc3339();
        r.duration_ms = start.elapsed().as_millis() as u64;
        r
    };

    // Always compute general recommendations (high-signal, lab-focused)
    report.recommendations = build_general_recommendations(&report);

    let output = if args.json {
        serde_json::to_string_pretty(&report)?
    } else {
        format_mobile_report(&report)
    };

    if let Some(ref out_path) = args.output {
        tokio::fs::write(out_path, &output).await?;
        if !args.quiet {
            eprintln!("Results written to {}", out_path);
        }
    } else {
        println!("{}", output);
    }

    Ok(())
}

fn build_general_recommendations(report: &MobileScanReport) -> Vec<String> {
    let mut recs = Vec::new();
    if report.findings.is_empty() {
        recs.push("No high-signal static issues detected in manifest/config surface. Expand testing with code review, dependency analysis, and (in lab) dynamic instrumentation under explicit authorization.".to_string());
    } else {
        recs.push("Review all findings in the context of the app's data classification and threat model.".to_string());
        recs.push("Prefer platform secure storage (Android Keystore / iOS Keychain) and strong transport (TLS 1.2+ with pinning where feasible).".to_string());
    }
    recs.push("This is static analysis only. Combine with SAST/dependency scanning, manual review, and authorized dynamic testing for comprehensive coverage.".to_string());
    recs.push("Ensure test builds are provenance-controlled and destroyed after lab use.".to_string());
    recs
}

fn format_mobile_report(report: &MobileScanReport) -> String {
    let mut buf = String::new();
    buf.push_str(&format!(
        "Mobile Static Analysis ({})\n",
        report.platform.as_str()
    ));
    buf.push_str(&format!("Target: {}\n", report.target));
    if let Some(ref id) = report.app_id {
        buf.push_str(&format!("App ID: {}\n", id));
    }
    if let Some(ref v) = report.version {
        buf.push_str(&format!("Version: {}\n", v));
    }
    buf.push_str(&format!("Findings: {}\n\n", report.findings.len()));

    if !report.findings.is_empty() {
        buf.push_str("Findings:\n");
        for (i, f) in report.findings.iter().enumerate() {
            buf.push_str(&format!(
                "  {}. [{}] {} ({})\n     {}\n     Rec: {}\n",
                i + 1,
                f.severity.as_str(),
                f.title,
                f.category,
                f.description,
                f.recommendation
            ));
            if let Some(ref ev) = f.evidence {
                buf.push_str(&format!("     Evidence: {}\n", ev));
            }
            buf.push('\n');
        }
    }

    if !report.recommendations.is_empty() {
        buf.push_str("Recommendations:\n");
        for r in &report.recommendations {
            buf.push_str(&format!("  - {}\n", r));
        }
        buf.push('\n');
    }

    buf.push_str(&format!("Duration: {} ms\n", report.duration_ms));
    buf
}

/// Convert a MobileScanReport into the unified ScanReportData for JSON/SARIF/JUnit/etc.
/// consumers (mirrors wireless::to_scan_report_data pattern).
pub fn to_scan_report_data(result: &MobileScanReport) -> crate::output::convert::ScanReportData {
    use crate::output::convert::FindingData;

    let findings: Vec<FindingData> = result
        .findings
        .iter()
        .map(|f| FindingData {
            title: f.title.clone(),
            severity: f.severity.as_str().to_string(),
            category: format!("mobile-{}", result.platform.as_str()),
            description: f.description.clone(),
            location: result.target.clone(),
            evidence: f.evidence.clone(),
            remediation: Some(f.recommendation.clone()),
            cwe_ids: Vec::new(),
        })
        .collect();

    crate::output::convert::ScanReportData {
        target: result.target.clone(),
        scan_type: result.scan_type.clone(),
        timestamp: result.timestamp.clone(),
        findings,
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: result.duration_ms,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_new_sets_defaults() {
        let r = MobileScanReport::new("/tmp/test.apk", MobilePlatform::Android);
        assert_eq!(r.target, "/tmp/test.apk");
        assert_eq!(r.scan_type, "mobile-static");
        assert!(r.findings.is_empty());
        assert_eq!(r.platform, MobilePlatform::Android);
    }

    #[test]
    fn format_empty_report_has_no_findings_section() {
        let mut r = MobileScanReport::new("x.apk", MobilePlatform::Android);
        r.recommendations = vec!["rec1".into()];
        let s = format_mobile_report(&r);
        assert!(s.contains("Findings: 0"));
        assert!(s.contains("rec1"));
    }
}
