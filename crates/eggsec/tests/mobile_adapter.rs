#![cfg(feature = "mobile")]

use eggsec::config::{
    metadata_for_tool_id, EnforcementContext, EnforcementOutcome, ExecutionPolicy,
    ExecutionSurface, LoadedScope, ManualOverride, OperationDescriptor, OperationMode,
    OperationRisk, Scope, ScopeSource,
};
use eggsec::mobile::{
    format_mobile_report, to_scan_report_data, MobileFinding, MobilePlatform, MobileScanReport,
};

fn default_policy() -> ExecutionPolicy {
    ExecutionPolicy::default()
}

fn scope_allow(pattern: &str) -> Scope {
    Scope {
        allowed_targets: vec![eggsec::config::ScopeRule::new(pattern.to_string())],
        ..Default::default()
    }
}

fn loaded_explicit(scope: Scope) -> LoadedScope {
    LoadedScope::explicit(scope, ScopeSource::ConfigFile, None)
}

fn ctx_for_surface(
    surface: ExecutionSurface,
    policy: ExecutionPolicy,
    scope: LoadedScope,
) -> EnforcementContext {
    EnforcementContext::for_surface(surface, policy, scope)
}

// ─── Metadata / Descriptor ──────────────────────────────────────────

#[test]
fn metadata_for_static_exists_and_matches_handler_descriptor() {
    let meta =
        metadata_for_tool_id("mobile-static").expect("mobile-static metadata should be registered");
    assert_eq!(meta.risk, OperationRisk::SafeActive);
    assert_eq!(meta.mode, OperationMode::StandardAssessment);
    assert!(meta.required_features.iter().any(|f| *f == "mobile"));
    assert!(!meta.mcp_exposable);
    assert!(!meta.agent_exposable);
    assert!(!meta.rest_exposable);
    assert!(!meta.grpc_exposable);
    let desc = meta.descriptor_for_target(Some("/tmp/test.apk".to_string()));
    assert_eq!(desc.operation, "mobile-static");
    assert_eq!(desc.mode, OperationMode::StandardAssessment);
    assert_eq!(desc.risk, OperationRisk::SafeActive);
    assert_eq!(desc.target, Some("/tmp/test.apk".to_string()));
}

#[test]
fn metadata_aliases_resolve() {
    let meta =
        metadata_for_tool_id("mobile").expect("alias 'mobile' should resolve to mobile-static");
    assert_eq!(meta.id, "mobile-static");
    let meta2 = metadata_for_tool_id("mobile-scan")
        .expect("alias 'mobile-scan' should resolve to mobile-static");
    assert_eq!(meta2.id, "mobile-static");
}

#[cfg(feature = "mobile-dynamic")]
#[test]
fn metadata_for_dynamic_exists() {
    let meta = metadata_for_tool_id("mobile-dynamic")
        .expect("mobile-dynamic metadata should be registered");
    assert_eq!(meta.mode, OperationMode::DefenseLab);
    assert!(meta
        .required_features
        .iter()
        .any(|f| *f == "mobile-dynamic"));
}

// ─── Enforcement before execution ────────────────────────────────────

#[test]
fn static_descriptor_allowed_in_scope() {
    let scope = loaded_explicit(scope_allow("/tmp/**"));
    let ctx = ctx_for_surface(ExecutionSurface::CliManual, default_policy(), scope);
    let desc = OperationDescriptor {
        operation: "mobile-static".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![],
        target: Some("/tmp/test.apk".to_string()),
        required_features: vec!["mobile".to_string()],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: vec![],
    };
    let outcome = ctx.evaluate(&desc);
    assert!(
        outcome.is_allowed(),
        "in-scope static should be allowed, got {:?}",
        outcome
    );
}

// ─── Denied policy prevents execution ────────────────────────────────

#[test]
fn static_descriptor_denied_out_of_scope() {
    // Use an IP-based target that can be scope-checked against CIDR rules.
    let scope = loaded_explicit(scope_allow("10.0.0.0/8"));
    let ctx = ctx_for_surface(ExecutionSurface::CliManualStrict, default_policy(), scope);
    let desc = OperationDescriptor {
        operation: "mobile-static".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![],
        target: Some("192.168.1.1".to_string()),
        required_features: vec!["mobile".to_string()],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: true,
        required_capabilities: vec![],
    };
    let outcome = ctx.evaluate(&desc);
    assert!(
        matches!(outcome, EnforcementOutcome::Deny { .. }),
        "out-of-scope IP should deny on strict surface, got {:?}",
        outcome
    );
}

#[test]
fn deny_prevents_domain_execution() {
    let scope = loaded_explicit(scope_allow("10.0.0.0/8"));
    let ctx = ctx_for_surface(ExecutionSurface::McpServer, default_policy(), scope);
    let desc = OperationDescriptor {
        operation: "mobile-static".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![],
        target: Some("192.168.1.1".to_string()),
        required_features: vec!["mobile".to_string()],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: true,
        required_capabilities: vec![],
    };
    let result = ctx.approve(ExecutionSurface::McpServer, desc);
    assert!(result.is_err(), "deny should prevent approve");
}

// ─── Manual override behavior ────────────────────────────────────────

const ALL_SURFACES: &[ExecutionSurface] = &[
    ExecutionSurface::CliManual,
    ExecutionSurface::CliManualStrict,
    ExecutionSurface::TuiManual,
    ExecutionSurface::TuiManualStrict,
    ExecutionSurface::McpServer,
    ExecutionSurface::SecurityAgent,
    ExecutionSurface::Ci,
    ExecutionSurface::RestApi,
];

#[test]
fn only_permissive_surfaces_honor_manual_override() {
    for surface in ALL_SURFACES {
        let honors = matches!(
            surface,
            ExecutionSurface::CliManual | ExecutionSurface::TuiManual
        );
        assert_eq!(
            surface.honors_manual_override(),
            honors,
            "{:?} manual override honor mismatch",
            surface
        );
    }
}

#[test]
fn manual_override_permits_confirmation_on_permissive_surface() {
    let mut over = ManualOverride::default();
    over.allow_out_of_scope = true;
    let scope = loaded_explicit(scope_allow("10.0.0.0/8"));
    let ctx = ctx_for_surface(ExecutionSurface::CliManual, default_policy(), scope);
    let desc = OperationDescriptor {
        operation: "mobile-static".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![],
        target: Some("192.168.1.1".to_string()),
        required_features: vec!["mobile".to_string()],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: true,
        required_capabilities: vec![],
    };
    let result = ctx.approve_manual(ExecutionSurface::CliManual, desc, Some(&over));
    assert!(
        result.is_ok(),
        "manual override should permit on permissive surface"
    );
}

#[test]
fn manual_override_ignored_on_strict_surfaces() {
    let mut over = ManualOverride::default();
    over.allow_out_of_scope = true;
    let scope = loaded_explicit(scope_allow("10.0.0.0/8"));
    let desc = OperationDescriptor {
        operation: "mobile-static".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![],
        target: Some("192.168.1.1".to_string()),
        required_features: vec!["mobile".to_string()],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: true,
        required_capabilities: vec![],
    };
    for surface in &[
        ExecutionSurface::McpServer,
        ExecutionSurface::SecurityAgent,
        ExecutionSurface::Ci,
        ExecutionSurface::RestApi,
    ] {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let result = ctx.approve_manual(*surface, desc.clone(), Some(&over));
        assert!(
            result.is_err(),
            "manual override should be ignored on {:?}",
            surface
        );
    }
}

// ─── Strict surfaces deny without explicit scope ─────────────────────

#[test]
fn strict_surfaces_deny_out_of_scope() {
    let scope = loaded_explicit(scope_allow("192.168.1.0/24"));
    let desc = OperationDescriptor {
        operation: "mobile-static".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![],
        target: Some("10.0.0.1".to_string()),
        required_features: vec!["mobile".to_string()],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: true,
        required_capabilities: vec![],
    };
    for surface in &[
        ExecutionSurface::McpServer,
        ExecutionSurface::SecurityAgent,
        ExecutionSurface::Ci,
        ExecutionSurface::RestApi,
    ] {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            matches!(outcome, EnforcementOutcome::Deny { .. }),
            "{:?} strict surface should deny out-of-scope, got {:?}",
            surface,
            outcome
        );
    }
}

// ─── Report conversion (integration-level) ───────────────────────────

#[test]
fn report_conversion_bridge_produces_valid_output() {
    let mut r = MobileScanReport::new("/tmp/app.apk", MobilePlatform::Android);
    r.app_id = Some("com.example.app".to_string());
    r.findings.push(MobileFinding {
        category: "manifest".to_string(),
        severity: eggsec::types::Severity::High,
        title: "Debuggable application".to_string(),
        description: "Application has debuggable flag set".to_string(),
        recommendation: "Remove android:debuggable from manifest".to_string(),
        evidence: Some("android:debuggable=true".to_string()),
    });
    r.findings.push(MobileFinding {
        category: "permission".to_string(),
        severity: eggsec::types::Severity::Medium,
        title: "Excessive permissions".to_string(),
        description: "App requests unnecessary permissions".to_string(),
        recommendation: "Remove unused permissions".to_string(),
        evidence: Some("CAMERA, RECORD_AUDIO".to_string()),
    });
    let data = to_scan_report_data(&r);
    assert_eq!(data.target, "/tmp/app.apk");
    assert_eq!(data.scan_type, "mobile-static");
    assert_eq!(data.findings.len(), 2);
    assert_eq!(data.findings[0].category, "mobile-android-manifest");
    assert_eq!(data.findings[0].severity, "high");
    assert!(data.findings[0].remediation.is_some());
    assert_eq!(data.findings[1].category, "mobile-android-permission");
    assert!(data.wireless_networks.is_empty());
    assert!(data.policy_summary.is_none());
    // serde roundtrip
    let json = serde_json::to_string(&data).unwrap();
    let back: eggsec::output::convert::ScanReportData = serde_json::from_str(&json).unwrap();
    assert_eq!(back.findings.len(), 2);
    assert_eq!(back.target, "/tmp/app.apk");
}

#[test]
fn report_formatting_contains_all_sections() {
    let mut r = MobileScanReport::new("/tmp/app.apk", MobilePlatform::Android);
    r.findings.push(MobileFinding {
        category: "manifest".to_string(),
        severity: eggsec::types::Severity::High,
        title: "Issue".to_string(),
        description: "Desc".to_string(),
        recommendation: "Fix".to_string(),
        evidence: None,
    });
    r.recommendations = vec!["Use HTTPS".to_string()];
    let text = format_mobile_report(&r);
    assert!(text.contains("Findings: 1"));
    assert!(text.contains("Use HTTPS"));
    assert!(text.contains("android"));
    assert!(text.contains("/tmp/app.apk"));
}

// ─── Feature-gated builds ────────────────────────────────────────────

#[cfg(feature = "mobile-dynamic")]
#[test]
fn dynamic_types_accessible() {
    use eggsec::mobile::{DynamicMobileArgs, DynamicMobileReport};
    let _ = std::mem::size_of::<DynamicMobileArgs>();
    let _ = std::mem::size_of::<DynamicMobileReport>();
}
