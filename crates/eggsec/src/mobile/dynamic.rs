//! Mobile dynamic runtime analysis module (feature-gated behind `mobile-dynamic`).
//!
//! Phase 1 per plans/mobile-dynamic-phase1-implementation-handoff-plan.md (and parent
//! plans/dynamic-mobile-testing-loadout-design-plan.md): Android ADB core + high-signal
//! runtime log analysis for lab/defense validation.
//!
//! This file provides the public API surface, report types (DynamicMobileReport / Finding,
//! LabManifest), the run_dynamic_cli dispatcher, human/JSON formatting, and the
//! to_scan_report_data_dynamic bridge stub.
//!
//! Key behaviors (P1):
//! - dry_run: simulate everything, produce full valid report, zero device/net touch.
//! - real: load optional --lab-manifest (TOML, advisory), connect via adb, conditional
//!   install/launch/capture-logs/uninstall, always best-effort cleanup, parse via runtime,
//!   audit all actions.
//! - Platform limited to Android in Phase 1.
//! - Standalone defense-lab (MCP/agent exposure absent).
//!
//! See also: adb.rs (pure-Rust TCP primary + external adb convenience), runtime.rs (log parser),
//! mobile/mod.rs reexports, and the handoff plan for full context + safety model.

use crate::error::{EggsecError, Result};
use crate::types::Severity;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;

use super::MobilePlatform;

/// CLI args struct for the dynamic entry point (P1 skeleton; real CLI struct
/// will live in cli/ and be mapped by handler in later integration).
#[derive(Debug, Clone, Default)]
pub struct DynamicMobileArgs {
    pub target: String,
    pub device: Option<String>,
    pub install: bool,
    pub launch: Option<String>,
    pub capture_logs: bool,
    pub duration: Option<u64>,
    pub uninstall_after: bool,
    pub dry_run: bool,
    pub json: bool,
    pub output: Option<String>,
    pub quiet: bool,
    pub allow_dynamic_mobile: bool,
    pub lab_manifest: Option<String>,
}

/// Lab device/app allowlist manifest (loaded from --lab-manifest TOML if provided).
/// Default = empty (advisory only in Phase 1; no hard block).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LabManifest {
    pub allowed_device_serials: Vec<String>,
    pub allowed_packages: Vec<String>,
}

impl LabManifest {
    /// Load from TOML file (advisory semantics in P1).
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| EggsecError::Validation(format!("failed to read lab manifest {}: {}", path.display(), e)))?;
        toml::from_str(&content)
            .map_err(|e| EggsecError::Validation(format!("invalid lab manifest TOML: {}", e)))
    }
}

/// Runtime finding from dynamic execution (logcat, observed behavior).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicMobileFinding {
    pub category: String,           // "runtime-permission", "crash-log", "cleartext-observed", "log-secret-leak", ...
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub recommendation: String,
    pub evidence: Option<String>,
    /// Optional link back to a static finding (future correlation).
    pub static_correlation: Option<String>,
}

/// Full report from a dynamic mobile run (install/launch/observe/uninstall cycle).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicMobileReport {
    pub target: String,                 // APK path or package name
    pub scan_type: String,              // "mobile-dynamic"
    pub platform: MobilePlatform,       // Android only in P1
    pub device_serial: Option<String>,
    pub app_id: Option<String>,
    pub version: Option<String>,
    pub timestamp: String,
    pub findings: Vec<DynamicMobileFinding>,
    pub recommendations: Vec<String>,
    pub duration_ms: u64,
    /// Audit trail of every action taken (or simulated).
    pub actions_performed: Vec<String>,
    pub dry_run: bool,
}

impl DynamicMobileReport {
    pub fn new(target: &str) -> Self {
        Self {
            target: target.to_string(),
            scan_type: "mobile-dynamic".to_string(),
            platform: MobilePlatform::Android,
            device_serial: None,
            app_id: None,
            version: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            findings: Vec::new(),
            recommendations: Vec::new(),
            duration_ms: 0,
            actions_performed: Vec::new(),
            dry_run: false,
        }
    }
}

/// High-level dispatcher for `eggsec mobile dynamic ...` (and future TUI/automation).
/// Mirrors the structure and UX of static `run_cli` in parent mod.
///
/// - Loads lab manifest (if --lab-manifest) — advisory in P1.
/// - Dry-run: never touches devices/network; always produces complete, serializable report
///   (with simulated actions + optional sample findings).
/// - Real path: requires --device, uses adb crate, performs requested ops, always attempts
///   cleanup on exit path, feeds captured logs to runtime::parse_logcat_findings.
/// - Output: --json or pretty human; -o writes to file; --quiet suppresses notes.
/// - --allow-dynamic-mobile is accepted (policy/enforcement checked in caller/handler).
pub async fn run_dynamic_cli(args: DynamicMobileArgs, _config: &crate::config::EggsecConfig) -> Result<()> {
    let start = Instant::now();

    let target = if args.target.trim().is_empty() {
        if args.dry_run {
            "sample-dynamic.apk".to_string()
        } else {
            return Err(EggsecError::Validation(
                "target APK path or package required for dynamic mobile (use --dry-run for simulation without target)".to_string(),
            ));
        }
    } else {
        args.target.clone()
    };

    let mut actions: Vec<String> = Vec::new();
    let mut findings: Vec<DynamicMobileFinding> = Vec::new();

    // Manifest load (advisory — recorded in actions; enforcement is policy + handler layer)
    if let Some(manifest_path) = &args.lab_manifest {
        match LabManifest::load(Path::new(manifest_path)) {
            Ok(m) => {
                actions.push(format!(
                    "loaded lab-manifest ({} allowed devices, {} allowed packages; advisory in P1)",
                    m.allowed_device_serials.len(),
                    m.allowed_packages.len()
                ));
            }
            Err(e) => {
                if !args.quiet {
                    eprintln!("warning: failed to load --lab-manifest {}: {}", manifest_path, e);
                }
                actions.push(format!("lab-manifest load failed (treated as advisory): {}", e));
            }
        }
    }

    if !args.quiet {
        eprintln!(
            "NOTE: Mobile *dynamic* analysis is for authorized lab/defensive validation use only. \
             Supply your own test builds and lab devices. All actions are logged and best-effort reversible. \
             See docs/MOBILE.md and docs/SAFETY.md."
        );
    }

    if args.dry_run {
        actions.push("dry-run: no device or network actions performed".to_string());
        if args.install {
            actions.push(format!("dry-run: would install {}", target));
        }
        if let Some(ref act) = args.launch {
            actions.push(format!("dry-run: would launch {}", act));
        }
        if args.capture_logs {
            let secs = args.duration.unwrap_or(30);
            actions.push(format!("dry-run: would capture-logs for {}s", secs));
        }
        if args.uninstall_after {
            actions.push("dry-run: would uninstall-after".to_string());
        }
        // P1: include one simulated high-signal finding so report is non-empty and bridge-exercised
        findings.push(DynamicMobileFinding {
            category: "runtime-permission".to_string(),
            severity: Severity::Low,
            title: "Simulated runtime permission grant (dry-run)".to_string(),
            description: "In a real run, logcat would show permission grant/denial events.".to_string(),
            recommendation: "Correlate runtime grants with static manifest analysis.".to_string(),
            evidence: Some("dry-run: simulated CAMERA grant".to_string()),
            static_correlation: None,
        });
    } else {
        // Real execution path — device required
        let device = args
            .device
            .as_ref()
            .ok_or_else(|| EggsecError::Validation("--device (serial or host:port) is required for non-dry-run dynamic mobile".to_string()))?;

        if args.install {
            let p = Path::new(&target);
            if !p.exists() || !p.is_file() {
                return Err(EggsecError::Validation(format!(
                    "--install requires a readable .apk file as target, got: {}",
                    target
                )));
            }
        }

        // Connect / validate reachability (adb module handles pure-Rust TCP for emulator-XXXX or host:port).
        // We do not retain the connection here; later steps re-connect per operation for simplicity in P1.
        // This produces the audit "connected" entry and fails fast if device is unreachable.
        let _conn = crate::mobile::adb::AdbClient::connect(device)
            .await
            .map_err(|e| EggsecError::Validation(format!("adb connect to {} failed: {}", device, e)))?;
        actions.push(format!("connected to device {}", device));

        // Derive package for launch/uninstall (P1 heuristic; real would parse manifest or require --package)
        let is_apk_like = target.ends_with(".apk") || Path::new(&target).exists();
        let package: String = if is_apk_like {
            if let Some(ref launch) = args.launch {
                launch.split_once('/').map(|(p, _)| p.to_string()).unwrap_or_else(|| "com.example.dynamic".to_string())
            } else {
                "com.example.dynamic".to_string()
            }
        } else {
            target.clone()
        };

        if args.install {
            let data = tokio::fs::read(&target)
                .await
                .map_err(|e| EggsecError::Validation(format!("failed to read apk for install: {}", e)))?;
            let mut conn_i = crate::mobile::adb::AdbClient::connect(device)
                .await
                .map_err(|e| EggsecError::Validation(format!("adb connect for install failed: {}", e)))?;
            let install_out = conn_i.install_apk(&data)
                .await
                .map_err(|e| EggsecError::Validation(format!("install failed: {}", e)))?;
            actions.push(format!("install: {}", install_out.trim()));
        }

        if let Some(ref activity) = args.launch {
            let mut conn_l = crate::mobile::adb::AdbClient::connect(device)
                .await
                .map_err(|e| EggsecError::Validation(e.to_string()))?;
            conn_l.launch_app(&package, Some(activity))
                .await
                .map_err(|e| EggsecError::Validation(format!("launch failed: {}", e)))?;
            actions.push(format!("launched {}", activity));
        }

        if args.capture_logs {
            let dur = std::time::Duration::from_secs(args.duration.unwrap_or(30));
            let mut conn_c = crate::mobile::adb::AdbClient::connect(device)
                .await
                .map_err(|e| EggsecError::Validation(e.to_string()))?;
            let captured_logs = conn_c
                .capture_logcat(dur, Some(&package))
                .await
                .map_err(|e| EggsecError::Validation(format!("logcat capture failed: {}", e)))?;
            actions.push(format!("captured {} bytes of logcat ({}s)", captured_logs.len(), dur.as_secs()));
            let parsed = crate::mobile::runtime::parse_logcat_findings(&captured_logs);
            findings.extend(parsed);
        }

        // uninstall if requested
        let mut cleaned = false;
        if args.uninstall_after {
            let mut conn_u = crate::mobile::adb::AdbClient::connect(device)
                .await
                .map_err(|e| EggsecError::Validation(e.to_string()))?;
            match conn_u.uninstall(&package, false).await {
                Ok(_) => {
                    actions.push(format!("uninstalled {}", package));
                    cleaned = true;
                }
                Err(e) => {
                    actions.push(format!("uninstall failed (best-effort cleanup will retry): {}", e));
                }
            }
        }

        // ALWAYS attempt cleanup for any install we performed (even without --uninstall-after)
        if args.install && !cleaned {
            let mut conn_cl = crate::mobile::adb::AdbClient::connect(device)
                .await
                .map_err(|e| EggsecError::Validation(e.to_string()))?;
            let _ = conn_cl.uninstall(&package, false).await;
            actions.push(format!("post-run cleanup: uninstall attempted for {} (best effort)", package));
        }
    }

    // Build report
    let mut report = DynamicMobileReport {
        target: target.clone(),
        scan_type: "mobile-dynamic".to_string(),
        platform: MobilePlatform::Android,
        device_serial: args.device.clone(),
        app_id: if target.ends_with(".apk") { None } else { Some(target.clone()) },
        version: None,
        timestamp: chrono::Utc::now().to_rfc3339(),
        findings,
        recommendations: Vec::new(),
        duration_ms: start.elapsed().as_millis() as u64,
        actions_performed: actions,
        dry_run: args.dry_run,
    };
    report.recommendations = build_dynamic_recommendations(&report);

    // Output
    let output = if args.json {
        serde_json::to_string_pretty(&report)?
    } else {
        format_dynamic_report(&report)
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

fn build_dynamic_recommendations(report: &DynamicMobileReport) -> Vec<String> {
    let mut recs = Vec::new();
    recs.push(
        "Dynamic mobile testing is for authorized lab/defense-validation use only. Supply your own test builds. \
         Securely destroy test artifacts and devices after use.".to_string(),
    );
    if report.findings.is_empty() {
        recs.push(
            "No high-signal runtime events (permission, crash, cleartext, secret patterns) observed in captured logs. \
             Expand with static baseline + proxy MITM for deeper coverage.".to_string(),
        );
    } else {
        recs.push("Review all runtime findings in context of the app's data classification, manifest claims, and threat model.".to_string());
        recs.push("Correlate dynamic observations (e.g. actual permission grants or log leaks) back to static manifest results.".to_string());
    }
    recs.push("This is ADB + logcat observation only (Phase 1). Future phases add proxy correlation and gated instrumentation.".to_string());
    if report.dry_run {
        recs.push("Report generated in --dry-run mode — no device actions were executed.".to_string());
    }
    recs
}

pub fn format_dynamic_report(report: &DynamicMobileReport) -> String {
    let mut buf = String::new();
    buf.push_str(&format!("Mobile Dynamic Analysis ({})\n", report.platform.as_str()));
    buf.push_str(&format!("Target: {}\n", report.target));
    if let Some(ref d) = report.device_serial {
        buf.push_str(&format!("Device: {}\n", d));
    }
    if let Some(ref id) = report.app_id {
        buf.push_str(&format!("App ID / Package: {}\n", id));
    }
    buf.push_str(&format!("Scan type: {}  |  dry_run: {}\n", report.scan_type, report.dry_run));
    buf.push_str(&format!("Findings: {}  |  Actions logged: {}\n\n", report.findings.len(), report.actions_performed.len()));

    if !report.actions_performed.is_empty() {
        buf.push_str("Actions performed:\n");
        for a in &report.actions_performed {
            buf.push_str(&format!("  - {}\n", a));
        }
        buf.push('\n');
    }

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

/// Convert DynamicMobileReport into unified ScanReportData (stub but produces valid structure).
/// Categories follow the documented convention: mobile-dynamic-android-*
pub fn to_scan_report_data_dynamic(result: &DynamicMobileReport) -> crate::output::convert::ScanReportData {
    use crate::output::convert::FindingData;

    let findings: Vec<FindingData> = result
        .findings
        .iter()
        .map(|f| FindingData {
            title: f.title.clone(),
            severity: f.severity.as_str().to_string(),
            category: format!("mobile-dynamic-android-{}", f.category),
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
    fn lab_manifest_default_is_empty_advisory() {
        let m = LabManifest::default();
        assert!(m.allowed_device_serials.is_empty());
        assert!(m.allowed_packages.is_empty());
    }

    #[test]
    fn lab_manifest_serde_and_load_smoke() {
        let m = LabManifest {
            allowed_device_serials: vec!["emulator-5554".into(), "ABCD1234".into()],
            allowed_packages: vec!["com.example.test".into()],
        };
        let toml = toml::to_string(&m).unwrap();
        assert!(toml.contains("emulator-5554"));
        let back: LabManifest = toml::from_str(&toml).unwrap();
        assert_eq!(back.allowed_device_serials.len(), 2);
    }

    #[test]
    fn dynamic_finding_and_report_serde_roundtrips() {
        let mut f = DynamicMobileFinding {
            category: "runtime-permission".into(),
            severity: Severity::Medium,
            title: "runtime perm".into(),
            description: "d".into(),
            recommendation: "r".into(),
            evidence: Some("CAMERA granted".into()),
            static_correlation: None,
        };
        let jf = serde_json::to_string(&f).unwrap();
        let bf: DynamicMobileFinding = serde_json::from_str(&jf).unwrap();
        assert_eq!(bf.category, "runtime-permission");

        let mut r = DynamicMobileReport::new("test.apk");
        r.dry_run = true;
        r.actions_performed.push("simulated install".into());
        r.findings.push(f);
        let jr = serde_json::to_string(&r).unwrap();
        let br: DynamicMobileReport = serde_json::from_str(&jr).unwrap();
        assert_eq!(br.findings.len(), 1);
        assert!(br.dry_run);
        assert_eq!(br.scan_type, "mobile-dynamic");
        assert_eq!(br.platform, MobilePlatform::Android);
    }

    #[test]
    fn dry_run_path_produces_consistent_full_structure() {
        // Construct a representative dry-run report directly (exercises the types + formatting + bridge
        // without executing the async dispatcher which would require real args).
        let mut r = DynamicMobileReport::new("dry.apk");
        r.dry_run = true;
        r.device_serial = Some("emulator-5554".into());
        r.actions_performed = vec![
            "dry-run: no device or network actions performed".into(),
            "dry-run: would install dry.apk".into(),
        ];
        r.findings.push(DynamicMobileFinding {
            category: "runtime-permission".into(),
            severity: Severity::Low,
            title: "sim".into(),
            description: "sim".into(),
            recommendation: "sim".into(),
            evidence: None,
            static_correlation: None,
        });
        assert!(r.dry_run);
        assert_eq!(r.actions_performed.len(), 2);
        assert_eq!(r.findings.len(), 1);

        let pretty = format_dynamic_report(&r);
        assert!(pretty.contains("dry_run: true"));
        assert!(pretty.contains("Actions performed:"));
        assert!(pretty.contains("would install"));

        let data = to_scan_report_data_dynamic(&r);
        assert_eq!(data.target, "dry.apk");
        assert_eq!(data.scan_type, "mobile-dynamic");
        assert_eq!(data.findings.len(), 1);
        assert_eq!(data.findings[0].category, "mobile-dynamic-android-runtime-permission");
        assert!(data.wireless_networks.is_empty());
        assert!(data.policy_summary.is_none());
    }

    #[test]
    fn to_scan_report_data_dynamic_categories_and_bridge() {
        let mut r = DynamicMobileReport::new("vuln.apk");
        r.findings.push(DynamicMobileFinding {
            category: "crash-log".into(),
            severity: Severity::High,
            title: "crash".into(),
            description: "boom".into(),
            recommendation: "fix".into(),
            evidence: Some("NullPointer at ...".into()),
            static_correlation: None,
        });
        r.findings.push(DynamicMobileFinding {
            category: "log-secret-leak".into(),
            severity: Severity::High,
            title: "secret".into(),
            description: "leaked".into(),
            recommendation: "never log".into(),
            evidence: Some("api_key=[REDACTED]".into()),
            static_correlation: None,
        });
        let data = to_scan_report_data_dynamic(&r);
        assert_eq!(data.findings.len(), 2);
        assert_eq!(data.findings[0].category, "mobile-dynamic-android-crash-log");
        assert_eq!(data.findings[1].category, "mobile-dynamic-android-log-secret-leak");
        // roundtrip
        let j = serde_json::to_string(&data).unwrap();
        let back: crate::output::convert::ScanReportData = serde_json::from_str(&j).unwrap();
        assert_eq!(back.findings.len(), 2);
    }

    #[test]
    fn parser_integration_on_synthetic_fixtures() {
        // Exercises runtime parser through the re-export / sibling path + categories we care about
        let synthetic = r#"
I/ActivityManager: permission android.permission.CAMERA granted for com.example.vuln
E/AndroidRuntime: FATAL EXCEPTION: main
E/AndroidRuntime: java.lang.NullPointerException: ...
E/AndroidRuntime: at com.example.vuln.MainActivity.onCreate(MainActivity.java:87)
D/NetworkClient: http://api.vuln.example.com/data?token=sk_live_ABC123
W/PackageManager: permission denied: READ_SMS
"#;
        let fs = crate::mobile::runtime::parse_logcat_findings(synthetic);
        assert!(fs.iter().any(|f| f.category == "runtime-permission" && f.title.contains("granted")));
        assert!(fs.iter().any(|f| f.category == "runtime-permission" && f.title.contains("denied")));
        assert!(fs.iter().any(|f| f.category == "crash-log"));
        assert!(fs.iter().any(|f| f.category == "cleartext-observed"));
        assert!(fs.iter().any(|f| f.category == "log-secret-leak"));
        // evidence for secret should be redacted
        let secret = fs.iter().find(|f| f.category == "log-secret-leak").unwrap();
        assert!(secret.evidence.as_ref().unwrap().contains("[REDACTED]"));
    }
}
