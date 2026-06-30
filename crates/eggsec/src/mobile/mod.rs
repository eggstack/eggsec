//! Mobile app security analysis module (feature-gated behind `mobile`).
//!
//! Domain logic is owned by the `eggsec-mobile-lab` crate. This module provides
//! a thin adapter layer that re-exports types for backward compatibility and
//! bridges the handler to the domain crate.
//!
//! Key entry points (delegated to domain crate):
//! - `run_cli(args, config)` for static analysis dispatch.
//! - `run_dynamic_cli(args, config)` for dynamic analysis dispatch.
//! - `to_scan_report_data(report)` for static report bridge.
//! - `to_scan_report_data_dynamic(report)` for dynamic report bridge.

use crate::config::EggsecConfig;
use crate::error::{EggsecError, Result};

// Re-export all domain types from eggsec-mobile-lab
pub use eggsec_mobile_lab::{
    // Static analysis functions
    analyze_apk,
    analyze_ipa,
    // Static analysis entry points and formatting
    format_mobile_report,
    to_scan_report_data,
    // Dynamic types (cfg-gated in domain crate)
    // Static analysis types
    MobileFinding,
    MobilePlatform,
    MobileScanReport,
};

// Re-export dynamic submodule for backward compatibility
pub use eggsec_mobile_lab::apk;
pub use eggsec_mobile_lab::ipa;

// Re-export dynamic types (cfg-gated)
#[cfg(feature = "mobile-dynamic")]
pub use eggsec_mobile_lab::frida::{
    basic_method_trace, connect, execute_script, resolve_frida_script_spec, run_frida_spec,
    FridaInstrumentation, FridaScriptResult, FridaSession, FRIDA_LIB_COMMON_HOOKS,
};
#[cfg(feature = "mobile-dynamic")]
pub use eggsec_mobile_lab::traffic::{parse_traffic_capture, TrafficSummary};
#[cfg(feature = "mobile-dynamic")]
pub use eggsec_mobile_lab::{
    capture_baseline, compare_to_baseline, correlate_findings, correlate_reports,
    export_evidence_bundle, format_dynamic_report, run_baseline_compare_workflow,
    to_scan_report_data_dynamic, CorrelatedFinding, CorrelationEngine, CorrelationResult,
    CorrelationSummary, CorrelationType, DynamicMobileArgs, DynamicMobileFinding,
    DynamicMobileReport, LabManifest, MobileBaseline,
};

// Re-export submodules for backward compatibility
#[cfg(feature = "mobile-dynamic")]
pub use eggsec_mobile_lab::adb;
#[cfg(feature = "mobile-dynamic")]
pub use eggsec_mobile_lab::dynamic;
#[cfg(feature = "mobile-dynamic")]
pub use eggsec_mobile_lab::frida;
#[cfg(feature = "mobile-dynamic")]
pub use eggsec_mobile_lab::runtime;
#[cfg(feature = "mobile-dynamic")]
pub use eggsec_mobile_lab::traffic;

/// High-level entry for CLI handlers (static analysis).
/// Delegates to the domain crate's `run_static_cli`.
pub async fn run_cli(args: crate::cli::MobileArgs, _config: &EggsecConfig) -> Result<()> {
    let _start = std::time::Instant::now();

    // Resolve effective static path + flags from legacy direct or 'static' subcommand.
    let (eff_path, eff_json, eff_output, eff_quiet) = match &args.command {
        Some(crate::cli::MobileSubcommand::Static(s)) => {
            (s.path.clone(), s.json, s.output.clone(), s.quiet)
        }
        _ => {
            let p = args.path.clone().ok_or_else(|| {
                EggsecError::Validation(
                    "mobile static: path required (legacy or via 'static' subcommand)".to_string(),
                )
            })?;
            (p, args.json, args.output.clone(), args.quiet)
        }
    };

    let path = std::path::Path::new(&eff_path);
    eggsec_mobile_lab::run_static_cli(path, eff_json, eff_output.as_deref(), eff_quiet)
        .await
        .map_err(|e| EggsecError::Validation(e.to_string()))
}

/// High-level entry for CLI handlers (dynamic analysis).
/// Delegates to the domain crate's `run_dynamic_cli`.
#[cfg(feature = "mobile-dynamic")]
pub async fn run_dynamic_cli(args: DynamicMobileArgs, _config: &EggsecConfig) -> Result<()> {
    eggsec_mobile_lab::run_dynamic_cli(args)
        .await
        .map_err(|e| EggsecError::Validation(e.to_string()))
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
            severity: crate::types::Severity::High,
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
            severity: crate::types::Severity::Medium,
            title: "weak transport".into(),
            description: "desc".into(),
            recommendation: "rec".into(),
            evidence: None,
        });
        r.findings.push(MobileFinding {
            category: "secret".into(),
            severity: crate::types::Severity::High,
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
            severity: crate::types::Severity::Medium,
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
            severity: crate::types::Severity::High,
            title: "hardcoded secret".into(),
            description: "api key in plist".into(),
            recommendation: "use keychain".into(),
            evidence: Some("api_key=sk_live_...".into()),
        });
        let data = to_scan_report_data(&r);
        assert_eq!(data.findings[0].category, "mobile-ios-secret");
        assert_eq!(
            data.findings[0].evidence.as_deref(),
            Some("api_key=sk_live_...")
        );
        assert!(data.wireless_networks.is_empty());
        assert!(data.policy_summary.is_none());
    }

    #[test]
    fn to_scan_report_data_android_exported_and_secret() {
        // covers two APK-specific categories via add_finding path + bridge
        let mut r = MobileScanReport::new("app.apk", MobilePlatform::Android);
        r.findings.push(MobileFinding {
            category: "exported-component".into(),
            severity: crate::types::Severity::High,
            title: "Exported activity".into(),
            description: "MainActivity is exported".into(),
            recommendation: "Restrict export or add permission protection".into(),
            evidence: Some("com.example:MainActivity".into()),
        });
        r.findings.push(MobileFinding {
            category: "hardcoded-secret".into(),
            severity: crate::types::Severity::High,
            title: "Hardcoded secret".into(),
            description: "api key in asset".into(),
            recommendation: "Remove secret".into(),
            evidence: Some("assets/config.json: ...api_key=...".into()),
        });
        let data = to_scan_report_data(&r);
        assert_eq!(data.findings.len(), 2);
        assert_eq!(
            data.findings[0].category,
            "mobile-android-exported-component"
        );
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
