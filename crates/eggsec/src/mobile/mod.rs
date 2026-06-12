//! Mobile app security static analysis module (feature-gated behind `mobile`).
//!
//! Phase 1 (per plans/mobile-first-handoff-plan.md): reliable static analysis of
//! Android APKs and iOS IPAs in authorized lab/defense-validation environments.
//! Focus on high-signal manifest/config findings only (no dynamic instrumentation,
//! no Frida, no active exploitation, no full decompilation).
//!
//! Safety: pure-Rust ZIP + plist + bounded AXML extraction. No shelling out.
//! All operations are offline on user-supplied lab binaries. Explicit lab-only framing.
//!
//! Phase 1 dynamic (Android ADB core + high-signal runtime logcat analysis) is
//! available under the additional `mobile-dynamic` feature flag. See:
//!
//! - plans/mobile-dynamic-phase1-implementation-handoff-plan.md (deliverables 2,4,8,9)
//! - plans/dynamic-mobile-testing-loadout-design-plan.md (parent design)
//!
//! New modules: dynamic.rs (public API + run_dynamic_cli + report types + bridge),
//!   adb.rs (pure-Rust TCP primary + external adb convenience), runtime.rs (log parser).
//! Standalone defense-lab surface. Re-exports and types added under cfg(feature = "mobile-dynamic").

use crate::error::{EggsecError, Result};
use crate::types::Severity;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod apk;
pub mod ipa;

#[cfg(feature = "mobile-dynamic")]
pub mod dynamic;
#[cfg(feature = "mobile-dynamic")]
pub mod adb;
#[cfg(feature = "mobile-dynamic")]
pub mod runtime;
#[cfg(feature = "mobile-dynamic")]
pub mod traffic;

// Re-export key dynamic types at crate::mobile level for handler/report bridge ergonomics (cfg-gated).
#[cfg(feature = "mobile-dynamic")]
pub use dynamic::{
    run_dynamic_cli, DynamicMobileArgs, DynamicMobileReport, DynamicMobileFinding, LabManifest,
    format_dynamic_report, to_scan_report_data_dynamic, correlate_findings, CorrelatedFinding,
};
#[cfg(feature = "mobile-dynamic")]
pub use traffic::{TrafficSummary, parse_traffic_capture};

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
///
/// Supports legacy direct path form (MobileArgs { path: Some(..), command: None, ... })
/// and new subcommand form (command: Some(MobileSubcommand::Static(MobileStaticArgs { path, .. })))
/// with common flags merged (subcommand flags take precedence for static).
pub async fn run_cli(args: crate::cli::MobileArgs, _config: &crate::config::EggsecConfig) -> Result<()> {
    let start = std::time::Instant::now();

    // Resolve effective static path + flags from legacy direct or 'static' subcommand.
    let (eff_path, eff_json, eff_output, eff_quiet) = match &args.command {
        Some(crate::cli::MobileSubcommand::Static(s)) => {
            (s.path.clone(), s.json, s.output.clone(), s.quiet)
        }
        _ => {
            let p = args.path.clone().ok_or_else(|| {
                EggsecError::Validation("mobile static: path required (legacy or via 'static' subcommand)".to_string())
            })?;
            (p, args.json, args.output.clone(), args.quiet)
        }
    };

    let path = Path::new(&eff_path);
    if !path.exists() {
        return Err(EggsecError::Validation(format!("Path does not exist: {}", eff_path)));
    }
    if !path.is_file() {
        return Err(EggsecError::Validation(format!("Path is not a file: {}", eff_path)));
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

    if !eff_quiet {
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

    let output = if eff_json {
        serde_json::to_string_pretty(&report)?
    } else {
        format_mobile_report(&report)
    };

    if let Some(ref out_path) = eff_output {
        tokio::fs::write(out_path, &output).await?;
        if !eff_quiet {
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
            category: format!("mobile-{}-{}", result.platform.as_str(), f.category),
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

    #[test]
    fn to_scan_report_data_produces_valid_bridge() {
        let mut r = MobileScanReport::new("test.apk", MobilePlatform::Android);
        r.app_id = Some("com.example".into());
        r.findings.push(MobileFinding {
            category: "manifest".into(),
            severity: Severity::High,
            title: "t".into(),
            description: "d".into(),
            recommendation: "r".into(),
            evidence: Some("e".into()),
        });
        let data = to_scan_report_data(&r);
        assert_eq!(data.target, "test.apk");
        assert_eq!(data.scan_type, "mobile-static");
        assert_eq!(data.findings.len(), 1);
        assert_eq!(data.findings[0].severity, "high");
        assert_eq!(data.findings[0].category, "mobile-android-manifest");
        assert_eq!(data.findings[0].remediation.as_deref(), Some("r"));
        assert_eq!(data.findings[0].evidence.as_deref(), Some("e"));
        assert!(data.wireless_networks.is_empty());
        assert!(data.policy_summary.is_none());
    }

    #[test]
    fn to_scan_report_data_ios_and_multiple_and_empty_and_roundtrip() {
        // iOS platform category
        let mut r = MobileScanReport::new("app.ipa", MobilePlatform::Ios);
        r.findings.push(MobileFinding {
            category: "transport".into(),
            severity: Severity::Medium,
            title: "weak transport".into(),
            description: "desc".into(),
            recommendation: "rec".into(),
            evidence: None,
        });
        r.findings.push(MobileFinding {
            category: "secret".into(),
            severity: Severity::High,
            title: "hardcoded".into(),
            description: "d2".into(),
            recommendation: "r2".into(),
            evidence: Some("key=...".into()),
        });
        let data = to_scan_report_data(&r);
        assert_eq!(data.target, "app.ipa");
        assert_eq!(data.scan_type, "mobile-static");
        assert_eq!(data.findings.len(), 2);
        assert_eq!(data.findings[0].category, "mobile-ios-transport");
        assert_eq!(data.findings[1].category, "mobile-ios-secret");
        assert_eq!(data.findings[1].evidence.as_deref(), Some("key=..."));
        assert!(data.wireless_networks.is_empty());

        // empty findings still produces valid bridge (0 findings)
        let r2 = MobileScanReport::new("empty.apk", MobilePlatform::Android);
        let d2 = to_scan_report_data(&r2);
        assert_eq!(d2.findings.len(), 0);
        assert_eq!(d2.target, "empty.apk");

        // serde roundtrip of bridged data
        let json = serde_json::to_string(&data).unwrap();
        let back: crate::output::convert::ScanReportData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.findings.len(), 2);
        assert_eq!(back.findings[0].category, "mobile-ios-transport");
    }

    #[test]
    fn to_scan_report_data_android_permission_evidence_roundtrip() {
        // targeted: android permission category + useful evidence (e.g. permission name)
        let mut r = MobileScanReport::new("vuln.apk", MobilePlatform::Android);
        r.findings.push(MobileFinding {
            category: "permission".into(),
            severity: Severity::Medium,
            title: "overprivileged permission".into(),
            description: "app requests READ_SMS".into(),
            recommendation: "remove if not needed".into(),
            evidence: Some("READ_SMS".into()),
        });
        let data = to_scan_report_data(&r);
        assert_eq!(data.findings.len(), 1);
        assert_eq!(data.findings[0].category, "mobile-android-permission");
        assert_eq!(data.findings[0].evidence.as_deref(), Some("READ_SMS"));
        assert_eq!(data.findings[0].severity, "medium");

        // serde roundtrip of bridged data (permission case)
        let json = serde_json::to_string(&data).unwrap();
        let back: crate::output::convert::ScanReportData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.findings.len(), 1);
        assert_eq!(back.findings[0].category, "mobile-android-permission");
        assert_eq!(back.findings[0].evidence.as_deref(), Some("READ_SMS"));
    }

    #[test]
    fn to_scan_report_data_ios_category_evidence() {
        // targeted iOS-specific category/evidence (e.g. secret pattern in bundle)
        let mut r = MobileScanReport::new("app.ipa", MobilePlatform::Ios);
        r.findings.push(MobileFinding {
            category: "secret".into(),
            severity: Severity::High,
            title: "hardcoded secret".into(),
            description: "api key in plist".into(),
            recommendation: "use keychain".into(),
            evidence: Some("api_key=sk_live_...".into()),
        });
        let data = to_scan_report_data(&r);
        assert_eq!(data.findings[0].category, "mobile-ios-secret");
        assert_eq!(data.findings[0].evidence.as_deref(), Some("api_key=sk_live_..."));
        assert!(data.wireless_networks.is_empty());
        assert!(data.policy_summary.is_none());
    }

    #[test]
    fn to_scan_report_data_android_exported_and_secret() {
        // covers two APK-specific categories via add_finding path + bridge
        let mut r = MobileScanReport::new("app.apk", MobilePlatform::Android);
        r.findings.push(MobileFinding {
            category: "exported-component".into(),
            severity: Severity::High,
            title: "Exported activity".into(),
            description: "MainActivity is exported".into(),
            recommendation: "Restrict export or add permission protection".into(),
            evidence: Some("com.example:MainActivity".into()),
        });
        r.findings.push(MobileFinding {
            category: "hardcoded-secret".into(),
            severity: Severity::High,
            title: "Hardcoded secret".into(),
            description: "api key in asset".into(),
            recommendation: "Remove secret".into(),
            evidence: Some("assets/config.json: ...api_key=...".into()),
        });
        let data = to_scan_report_data(&r);
        assert_eq!(data.findings.len(), 2);
        assert_eq!(data.findings[0].category, "mobile-android-exported-component");
        assert_eq!(data.findings[1].category, "mobile-android-hardcoded-secret");
        assert!(data.findings[0].remediation.is_some());
        assert!(data.findings[1].remediation.is_some());
        assert!(data.findings[0].evidence.is_some());
        assert!(data.findings[1].evidence.is_some());
        // roundtrip
        let j = serde_json::to_string(&data).unwrap();
        let back: crate::output::convert::ScanReportData = serde_json::from_str(&j).unwrap();
        assert_eq!(back.findings.len(), 2);
    }
}
