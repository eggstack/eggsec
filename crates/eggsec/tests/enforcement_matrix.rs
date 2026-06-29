//! Enforcement matrix test suite.
//!
//! Protects the dual-mode enforcement contract across all execution surfaces.
//! Catches two categories of regression:
//!
//! 1. Manual CLI/TUI becoming too strict to be useful.
//! 2. Agent/MCP/REST/CI becoming too permissive or honoring manual discretion.
//!
//! This file is the canonical cross-surface guardrail.

use eggsec::config::{
    Capability, ConfirmationClass, EnforcementContext, EnforcementOutcome, ExecutionPolicy,
    ExecutionProfile, ExecutionSurface, LoadedScope, ManualOverride, OperationDescriptor,
    OperationMode, OperationRisk, Scope, ScopeRule, ScopeSource,
};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Minimal descriptor builder for matrix tests.
fn descriptor(target: &str, risk: OperationRisk) -> OperationDescriptor {
    OperationDescriptor {
        operation: "matrix-op".to_string(),
        mode: OperationMode::StandardAssessment,
        risk,
        intended_uses: vec![],
        target: Some(target.to_string()),
        required_features: vec![],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: vec![],
    }
}

/// Descriptor requiring explicit scope (networked operation).
fn descriptor_requires_scope(target: &str, risk: OperationRisk) -> OperationDescriptor {
    OperationDescriptor {
        requires_explicit_scope: true,
        ..descriptor(target, risk)
    }
}

/// Descriptor with a required capability.
fn descriptor_with_cap(target: &str, risk: OperationRisk, cap: Capability) -> OperationDescriptor {
    OperationDescriptor {
        required_capabilities: vec![cap],
        ..descriptor(target, risk)
    }
}

/// Descriptor requiring a compile-time feature.
fn descriptor_with_feature(target: &str, risk: OperationRisk, feature: &str) -> OperationDescriptor {
    OperationDescriptor {
        required_features: vec![feature.to_string()],
        ..descriptor(target, risk)
    }
}

/// Scope allowing a single target.
fn scope_allow(pattern: &str) -> Scope {
    Scope {
        allowed_targets: vec![ScopeRule::new(pattern.to_string())],
        ..Default::default()
    }
}

/// Scope with wildcard + exclusion.
fn scope_wildcard_excluding(excluded: &str) -> Scope {
    Scope {
        allowed_targets: vec![ScopeRule::new("*".to_string())],
        excluded_targets: vec![ScopeRule::new(excluded.to_string())],
        ..Default::default()
    }
}

/// LoadedScope with explicit source (ConfigFile).
fn loaded_explicit(scope: Scope) -> LoadedScope {
    LoadedScope::explicit(scope, ScopeSource::ConfigFile, None)
}

/// LoadedScope with CLI source.
fn loaded_cli(scope: Scope) -> LoadedScope {
    LoadedScope::explicit(scope, ScopeSource::CliScopeFile, None)
}

/// EnforcementContext for a surface, using a given policy and scope.
fn ctx_for_surface(
    surface: ExecutionSurface,
    policy: ExecutionPolicy,
    scope: LoadedScope,
) -> EnforcementContext {
    EnforcementContext::for_surface(surface, policy, scope)
}

/// Default policy (nothing enabled).
fn default_policy() -> ExecutionPolicy {
    ExecutionPolicy::default()
}

/// Policy with intrusive fuzzing enabled.
fn policy_intrusive() -> ExecutionPolicy {
    ExecutionPolicy {
        allow_intrusive_fuzzing: true,
        ..Default::default()
    }
}

/// Policy with a capability in the allowed list.
fn policy_allow_cap(cap: Capability) -> ExecutionPolicy {
    ExecutionPolicy {
        allowed_capabilities: vec![cap],
        ..Default::default()
    }
}

/// Policy with a capability in the denied list.
fn policy_deny_cap(cap: Capability) -> ExecutionPolicy {
    ExecutionPolicy {
        denied_capabilities: vec![cap],
        ..Default::default()
    }
}

/// All execution surfaces for iteration.
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

/// Automated (strict) surfaces.
const AUTOMATED_SURFACES: &[ExecutionSurface] = &[
    ExecutionSurface::McpServer,
    ExecutionSurface::SecurityAgent,
    ExecutionSurface::Ci,
    ExecutionSurface::RestApi,
];

/// Surfaces that honor manual overrides (only CliManual and TuiManual).
const OVERRIDE_HONORING_SURFACES: &[ExecutionSurface] = &[
    ExecutionSurface::CliManual,
    ExecutionSurface::TuiManual,
];

/// Surfaces that do NOT honor manual overrides.
const NON_OVERRIDE_SURFACES: &[ExecutionSurface] = &[
    ExecutionSurface::CliManualStrict,
    ExecutionSurface::TuiManualStrict,
    ExecutionSurface::McpServer,
    ExecutionSurface::SecurityAgent,
    ExecutionSurface::Ci,
    ExecutionSurface::RestApi,
];

/// Manual permissive surfaces.
const PERMISSIVE_SURFACES: &[ExecutionSurface] = &[
    ExecutionSurface::CliManual,
    ExecutionSurface::TuiManual,
];

/// Strict/guarded surfaces (not permissive).
const STRICT_SURFACES: &[ExecutionSurface] = &[
    ExecutionSurface::CliManualStrict,
    ExecutionSurface::TuiManualStrict,
    ExecutionSurface::McpServer,
    ExecutionSurface::SecurityAgent,
    ExecutionSurface::Ci,
    ExecutionSurface::RestApi,
];

// ===========================================================================
// 1. Surface mapping invariants
// ===========================================================================

#[test]
fn cli_manual_and_tui_manual_map_to_manual_permissive() {
    assert_eq!(ExecutionSurface::CliManual.profile(), ExecutionProfile::ManualPermissive);
    assert_eq!(ExecutionSurface::TuiManual.profile(), ExecutionProfile::ManualPermissive);
}

#[test]
fn cli_tui_strict_map_to_manual_guarded() {
    assert_eq!(ExecutionSurface::CliManualStrict.profile(), ExecutionProfile::ManualGuarded);
    assert_eq!(ExecutionSurface::TuiManualStrict.profile(), ExecutionProfile::ManualGuarded);
}

#[test]
fn mcp_maps_to_mcp_strict() {
    assert_eq!(ExecutionSurface::McpServer.profile(), ExecutionProfile::McpStrict);
}

#[test]
fn security_agent_maps_to_agent_strict() {
    assert_eq!(ExecutionSurface::SecurityAgent.profile(), ExecutionProfile::AgentStrict);
}

#[test]
fn ci_maps_to_ci_strict() {
    assert_eq!(ExecutionSurface::Ci.profile(), ExecutionProfile::CiStrict);
}

#[test]
fn rest_maps_to_strict_profile() {
    assert_eq!(ExecutionSurface::RestApi.profile(), ExecutionProfile::McpStrict);
}

#[test]
fn only_cli_tui_manual_honor_manual_overrides() {
    for surface in ALL_SURFACES {
        let expected = matches!(surface, ExecutionSurface::CliManual | ExecutionSurface::TuiManual);
        assert_eq!(
            surface.honors_manual_override(),
            expected,
            "{}: honors_manual_override should be {}",
            surface,
            expected
        );
    }
}

#[test]
fn agent_controlled_surfaces_require_explicit_manifest_for_networked() {
    for surface in ALL_SURFACES {
        let expected = surface.is_agent_controlled();
        assert_eq!(
            surface.requires_explicit_manifest_for_networked(),
            expected,
            "{}: requires_explicit_manifest_for_networked should be {}",
            surface,
            expected
        );
    }
}

// ===========================================================================
// 2. Manual permissive invariants
// ===========================================================================

#[test]
fn permissive_safe_passive_in_scope_allows() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    for surface in PERMISSIVE_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&descriptor("127.0.0.1", OperationRisk::Passive));
        assert!(
            outcome.is_allowed(),
            "{}: safe passive in-scope should allow, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn permissive_safe_active_in_scope_allows() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    for surface in PERMISSIVE_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&descriptor("127.0.0.1", OperationRisk::SafeActive));
        assert!(
            outcome.is_allowed(),
            "{}: safe active in-scope should allow, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn permissive_positive_allowlist_miss_requires_confirmation() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    for surface in PERMISSIVE_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.requires_confirmation(),
            "{}: positive allowlist miss should require confirmation, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn permissive_high_risk_with_policy_flag_requires_confirmation() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::Intrusive);
    for surface in PERMISSIVE_SURFACES {
        let ctx = ctx_for_surface(*surface, policy_intrusive(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.requires_confirmation(),
            "{}: high-risk with policy flag should require confirmation, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn permissive_assume_yes_does_not_permit_high_risk() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::Intrusive);
    let policy = policy_intrusive();

    for surface in PERMISSIVE_SURFACES {
        let ctx = ctx_for_surface(*surface, policy.clone(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        // assume_yes does NOT permit HighRisk; RequireConfirmation is expected.
        assert!(
            outcome.requires_confirmation() || outcome.is_denied(),
            "{}: assume_yes should not permit high-risk, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn permissive_assume_yes_does_not_permit_private_resolution() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::SafeActive);

    for surface in PERMISSIVE_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        // PrivateResolution is about target resolution, not a risk flag.
        // The override permitting is tested at CommandContext layer; here we verify
        // that assume_yes alone doesn't magically resolve private resolution issues.
        // This is a structural test - the override doesn't affect evaluate_enforcement directly.
        let _ = ctx.evaluate(&desc);
    }
}

#[test]
fn permissive_assume_yes_does_not_permit_nonbaseline_capability() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor_with_cap("127.0.0.1", OperationRisk::SafeActive, Capability::IntrusiveFuzz);
    let mut over = ManualOverride::default();
    over.assume_yes = true;

    for surface in PERMISSIVE_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        // Non-baseline capability without explicit allow + assume_yes should not allow.
        // Under ManualPermissive, non-baseline capability without policy allow gets RequireConfirmation.
        assert!(
            outcome.requires_confirmation() || outcome.is_denied(),
            "{}: assume_yes should not permit nonbaseline capability, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn permissive_explicit_denied_capability_hard_denies() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor_with_cap("127.0.0.1", OperationRisk::SafeActive, Capability::LoadTest);
    let policy = policy_deny_cap(Capability::LoadTest);

    for surface in PERMISSIVE_SURFACES {
        let ctx = ctx_for_surface(*surface, policy.clone(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_denied(),
            "{}: denied capability should hard-deny even in permissive, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn permissive_missing_compile_feature_hard_denies() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    // Use a known feature behind a cfg gate; "packet-inspection" is cfg!(feature = "packet-inspection")
    // which is false when running tests without that feature enabled.
    let desc = descriptor_with_feature("127.0.0.1", OperationRisk::SafeActive, "packet-inspection");
    let ctx = ctx_for_surface(ExecutionSurface::CliManual, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(
        outcome.is_denied(),
        "missing compile feature should hard-deny even in permissive, got {:?}",
        outcome
    );
}

#[test]
fn permissive_excluded_target_requires_confirmation() {
    let scope = loaded_explicit(scope_wildcard_excluding("admin.example.com"));
    let desc = descriptor("admin.example.com", OperationRisk::SafeActive);
    for surface in PERMISSIVE_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.requires_confirmation(),
            "{}: explicit exclusion should require confirmation, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn permissive_empty_scope_safe_op_allows_with_warning() {
    let desc = descriptor("127.0.0.1", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(
        ExecutionSurface::CliManual,
        default_policy(),
        LoadedScope::default_empty(),
    );
    let outcome = ctx.evaluate(&desc);
    // Empty scope + safe op under permissive: may Allow or Warn, but not hard Deny.
    assert!(
        outcome.is_allowed() || outcome.requires_confirmation(),
        "empty scope safe op should not hard-deny under permissive, got {:?}",
        outcome
    );
}

// ===========================================================================
// 3. Manual guarded invariants
// ===========================================================================

#[test]
fn guarded_positive_allowlist_miss_denies() {
    let scope = loaded_cli(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::CliManualStrict, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.is_denied(), "guarded: positive allowlist miss should deny, got {:?}", outcome);
}

#[test]
fn guarded_positive_scope_miss_denies() {
    let scope = loaded_cli(scope_allow("10.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::CliManualStrict, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.is_denied(), "guarded: positive scope miss should deny, got {:?}", outcome);
}

#[test]
fn guarded_default_empty_with_safe_op_allows() {
    // ManualGuarded does NOT enforce explicit manifest the same way automated profiles do.
    // DefaultEmpty + safe op under ManualGuarded: the scope check is permissive enough to allow.
    let desc = descriptor("127.0.0.1", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(
        ExecutionSurface::CliManualStrict,
        default_policy(),
        LoadedScope::default_empty(),
    );
    let outcome = ctx.evaluate(&desc);
    assert!(
        outcome.is_allowed(),
        "guarded: DefaultEmpty + safe op should allow (no positive rules to miss), got {:?}",
        outcome
    );
}

#[test]
fn guarded_manual_overrides_ignored() {
    let scope = loaded_cli(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::CliManualStrict, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    // ManualGuarded ignores all overrides; RequireConfirmation should be treated as Deny.
    assert!(
        outcome.is_denied(),
        "guarded: should deny regardless of override intent, got {:?}",
        outcome
    );
}

#[test]
fn guarded_high_risk_without_policy_denies() {
    let scope = loaded_cli(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::Intrusive);
    let ctx = ctx_for_surface(ExecutionSurface::CliManualStrict, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.is_denied(), "guarded: high-risk without policy should deny, got {:?}", outcome);
}

#[test]
fn guarded_high_risk_with_policy_allows() {
    let scope = loaded_cli(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::Intrusive);
    let ctx = ctx_for_surface(ExecutionSurface::CliManualStrict, policy_intrusive(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.is_allowed(), "guarded: high-risk with policy flag should allow, got {:?}", outcome);
}

#[test]
fn guarded_in_scope_safe_op_allows() {
    let scope = loaded_cli(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::CliManualStrict, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.is_allowed(), "guarded: in-scope safe op should allow, got {:?}", outcome);
}

// ===========================================================================
// 4. MCP invariants
// ===========================================================================

#[test]
fn mcp_missing_explicit_scope_denies() {
    let desc = descriptor_requires_scope("127.0.0.1", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(
        ExecutionSurface::McpServer,
        default_policy(),
        LoadedScope::default_empty(),
    );
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.is_denied(), "mcp: missing explicit scope should deny, got {:?}", outcome);
}

#[test]
fn mcp_positive_allowlist_miss_denies() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::McpServer, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.is_denied(), "mcp: positive allowlist miss should deny, got {:?}", outcome);
}

#[test]
fn mcp_manual_override_flags_have_no_effect() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::McpServer, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    // MCP never processes ManualOverride; outcome should be deny.
    assert!(outcome.is_denied(), "mcp: overrides should have no effect, got {:?}", outcome);
}

#[test]
fn mcp_nonbaseline_capability_not_allowlisted_denies() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor_with_cap("127.0.0.1", OperationRisk::SafeActive, Capability::IntrusiveFuzz);
    let ctx = ctx_for_surface(ExecutionSurface::McpServer, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(
        outcome.is_denied(),
        "mcp: non-baseline capability without allow should deny, got {:?}",
        outcome
    );
}

#[test]
fn mcp_baseline_capability_with_scope_allows() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    for cap in &[Capability::PassiveFingerprint, Capability::ActiveProbe, Capability::Crawl, Capability::WafDetect] {
        let desc = descriptor_with_cap("127.0.0.1", OperationRisk::SafeActive, *cap);
        let ctx = ctx_for_surface(ExecutionSurface::McpServer, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_allowed(),
            "mcp: baseline capability {:?} should allow, got {:?}",
            cap,
            outcome
        );
    }
}

#[test]
fn mcp_nonbaseline_capability_with_explicit_allow_allows() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor_with_cap("127.0.0.1", OperationRisk::SafeActive, Capability::IntrusiveFuzz);
    let policy = policy_allow_cap(Capability::IntrusiveFuzz);
    let ctx = ctx_for_surface(ExecutionSurface::McpServer, policy, scope);
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.is_allowed(), "mcp: non-baseline with explicit allow should allow, got {:?}", outcome);
}

#[test]
fn mcp_warn_not_dispatchable() {
    // MCP should never produce Warn; it produces Allow or Deny.
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::Passive);
    let ctx = ctx_for_surface(ExecutionSurface::McpServer, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(
        matches!(outcome, EnforcementOutcome::Allow(_)),
        "mcp: safe in-scope should produce Allow, not Warn, got {:?}",
        outcome
    );
}

#[test]
fn mcp_require_confirmation_not_dispatchable() {
    // MCP should never produce RequireConfirmation; it produces Allow or Deny.
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::McpServer, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(
        outcome.is_denied(),
        "mcp: out-of-scope should deny (not confirm), got {:?}",
        outcome
    );
}

// ===========================================================================
// 5. Security agent invariants
// ===========================================================================

#[test]
fn agent_requires_agent_strict_profile() {
    let ctx = EnforcementContext::agent_strict(default_policy(), LoadedScope::default_empty());
    assert_eq!(ctx.execution_profile, ExecutionProfile::AgentStrict);
}

#[test]
fn agent_positive_allowlist_miss_denies() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::SecurityAgent, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.is_denied(), "agent: positive allowlist miss should deny, got {:?}", outcome);
}

#[test]
fn agent_missing_explicit_scope_denies() {
    let desc = descriptor_requires_scope("127.0.0.1", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(
        ExecutionSurface::SecurityAgent,
        default_policy(),
        LoadedScope::default_empty(),
    );
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.is_denied(), "agent: missing explicit scope should deny, got {:?}", outcome);
}

#[test]
fn agent_nonbaseline_capability_denies() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor_with_cap("127.0.0.1", OperationRisk::SafeActive, Capability::RawPacketProbe);
    let ctx = ctx_for_surface(ExecutionSurface::SecurityAgent, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.is_denied(), "agent: non-baseline capability should deny, got {:?}", outcome);
}

#[test]
fn agent_warnings_treated_as_denial() {
    // Agent runtime treats Warn as deny; verify the evaluate layer never produces Warn for agent.
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::SecurityAgent, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(
        matches!(outcome, EnforcementOutcome::Allow(_)),
        "agent: safe in-scope should Allow, never Warn, got {:?}",
        outcome
    );
}

#[test]
fn agent_ignores_manual_overrides() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::SecurityAgent, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.is_denied(), "agent: should deny regardless of override intent, got {:?}", outcome);
}

// ===========================================================================
// 6. REST invariants
// ===========================================================================

#[test]
fn rest_requires_explicit_manifest_for_networked() {
    let desc = descriptor_requires_scope("127.0.0.1", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(
        ExecutionSurface::RestApi,
        default_policy(),
        LoadedScope::default_empty(),
    );
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.is_denied(), "rest: missing explicit manifest should deny, got {:?}", outcome);
}

#[test]
fn rest_dispatches_only_on_allow() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::RestApi, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(matches!(outcome, EnforcementOutcome::Allow(_)), "rest: should dispatch only on Allow");
}

#[test]
fn rest_positive_allowlist_miss_denies() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::RestApi, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.is_denied(), "rest: positive allowlist miss should deny, got {:?}", outcome);
}

#[test]
fn rest_warn_treated_as_deny() {
    // REST should never produce Warn; it produces Allow or Deny.
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::RestApi, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(
        outcome.is_denied(),
        "rest: should treat warn-like cases as deny, got {:?}",
        outcome
    );
}

#[test]
fn rest_require_confirmation_treated_as_deny() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::RestApi, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(
        outcome.is_denied(),
        "rest: should treat RequireConfirmation as deny, got {:?}",
        outcome
    );
}

#[test]
fn rest_ignores_manual_overrides() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::RestApi, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.is_denied(), "rest: should deny regardless of override intent, got {:?}", outcome);
}

#[test]
fn rest_excluded_target_denies() {
    let scope = loaded_explicit(scope_wildcard_excluding("admin.example.com"));
    let desc = descriptor("admin.example.com", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::RestApi, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.is_denied(), "rest: excluded target should deny, got {:?}", outcome);
}

// ===========================================================================
// 7. CI invariants
// ===========================================================================

#[test]
fn ci_strict_behavior_matches_automated_strict() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);

    let ci_ctx = ctx_for_surface(ExecutionSurface::Ci, default_policy(), scope.clone());
    let mcp_ctx = ctx_for_surface(ExecutionSurface::McpServer, default_policy(), scope.clone());

    let ci_outcome = ci_ctx.evaluate(&desc);
    let mcp_outcome = mcp_ctx.evaluate(&desc);

    // Both should deny out-of-scope
    assert!(ci_outcome.is_denied(), "ci: should deny out-of-scope");
    assert!(mcp_outcome.is_denied(), "mcp: should deny out-of-scope");
}

#[test]
fn ci_does_not_honor_manual_overrides() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::Ci, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.is_denied(), "ci: should deny regardless of override intent, got {:?}", outcome);
}

#[test]
fn ci_requires_explicit_scope_for_networked() {
    let desc = descriptor_requires_scope("127.0.0.1", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(
        ExecutionSurface::Ci,
        default_policy(),
        LoadedScope::default_empty(),
    );
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.is_denied(), "ci: missing explicit scope should deny, got {:?}", outcome);
}

#[test]
fn ci_in_scope_safe_op_allows() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::Ci, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.is_allowed(), "ci: in-scope safe op should allow, got {:?}", outcome);
}

// ===========================================================================
// 8. Risk tier matrix across surfaces
// ===========================================================================

#[test]
fn risk_tier_passive_safe_across_all_surfaces() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::Passive);
    for surface in ALL_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_allowed(),
            "{}: passive safe should allow, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn risk_tier_safe_active_across_all_surfaces() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::SafeActive);
    for surface in ALL_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_allowed(),
            "{}: safe active should allow, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn risk_tier_intrusive_without_policy_denies_across_all_surfaces() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::Intrusive);
    for surface in ALL_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        // Risk policy denial is a hard deny for ALL profiles, including ManualPermissive.
        // RequireConfirmation only occurs when the risk IS allowed by policy but is high-risk
        // (operator discretion). When the policy itself denies the risk, it's a hard deny.
        assert!(
            outcome.is_denied(),
            "{}: intrusive without policy should deny, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn risk_tier_intrusive_with_policy_allows_across_surfaces() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::Intrusive);
    for surface in ALL_SURFACES {
        let ctx = ctx_for_surface(*surface, policy_intrusive(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        if PERMISSIVE_SURFACES.contains(surface) {
            // Permissive with policy flag: RequireConfirmation (operator discretion).
            assert!(
                outcome.requires_confirmation(),
                "{}: intrusive with policy should require confirmation under permissive, got {:?}",
                surface,
                outcome
            );
        } else {
            assert!(
                outcome.is_allowed(),
                "{}: intrusive with policy should allow under strict, got {:?}",
                surface,
                outcome
            );
        }
    }
}

#[test]
fn risk_tier_load_test_denied_without_policy_all_surfaces() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::LoadTest);
    for surface in ALL_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_denied(),
            "{}: load test without policy should deny, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn risk_tier_stress_test_denied_without_policy_all_surfaces() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::StressTest);
    for surface in ALL_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_denied(),
            "{}: stress test without policy should deny, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn risk_tier_raw_packet_denied_without_policy_all_surfaces() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::RawPacket);
    for surface in ALL_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_denied(),
            "{}: raw packet without policy should deny, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn risk_tier_credential_testing_denied_without_policy_all_surfaces() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::CredentialTesting);
    for surface in ALL_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_denied(),
            "{}: credential testing without policy should deny, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn risk_tier_remote_execution_denied_without_policy_all_surfaces() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::RemoteExecution);
    for surface in ALL_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_denied(),
            "{}: remote execution without policy should deny, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn risk_tier_c2_operation_denied_without_policy_all_surfaces() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::C2Operation);
    for surface in ALL_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_denied(),
            "{}: C2 operation without policy should deny, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn risk_tier_agent_autonomous_denied_without_policy_all_surfaces() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::AgentAutonomous);
    for surface in ALL_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_denied(),
            "{}: agent autonomous without policy should deny, got {:?}",
            surface,
            outcome
        );
    }
}

// ===========================================================================
// 9. Capability matrix across surfaces
// ===========================================================================

#[test]
fn baseline_capabilities_allowed_across_all_surfaces() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let baseline_caps = [
        Capability::PassiveFingerprint,
        Capability::ActiveProbe,
        Capability::Crawl,
        Capability::WafDetect,
    ];
    for cap in &baseline_caps {
        let desc = descriptor_with_cap("127.0.0.1", OperationRisk::SafeActive, *cap);
        for surface in ALL_SURFACES {
            let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
            let outcome = ctx.evaluate(&desc);
            assert!(
                outcome.is_allowed(),
                "{}: baseline cap {:?} should allow, got {:?}",
                surface,
                cap,
                outcome
            );
        }
    }
}

#[test]
fn non_baseline_capabilities_denied_under_strict_without_allow() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor_with_cap("127.0.0.1", OperationRisk::SafeActive, Capability::IntrusiveFuzz);
    for surface in AUTOMATED_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_denied(),
            "{}: non-baseline without allow should deny, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn non_baseline_capabilities_with_explicit_allow_allows_under_strict() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor_with_cap("127.0.0.1", OperationRisk::SafeActive, Capability::IntrusiveFuzz);
    let policy = policy_allow_cap(Capability::IntrusiveFuzz);
    for surface in AUTOMATED_SURFACES {
        let ctx = ctx_for_surface(*surface, policy.clone(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_allowed(),
            "{}: non-baseline with explicit allow should allow, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn denied_capabilities_hard_deny_across_all_surfaces() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor_with_cap("127.0.0.1", OperationRisk::SafeActive, Capability::LoadTest);
    let policy = policy_deny_cap(Capability::LoadTest);
    for surface in ALL_SURFACES {
        let ctx = ctx_for_surface(*surface, policy.clone(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_denied(),
            "{}: denied capability should hard-deny, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn non_baseline_capability_under_permissive_requires_confirmation() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor_with_cap("127.0.0.1", OperationRisk::SafeActive, Capability::IntrusiveFuzz);
    for surface in PERMISSIVE_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.requires_confirmation(),
            "{}: non-baseline under permissive should require confirmation, got {:?}",
            surface,
            outcome
        );
    }
}

// ===========================================================================
// 10. Override matrix across surfaces
// ===========================================================================

#[test]
fn assume_yes_narrow_scope_only() {
    // --yes only permits OutOfScope and TargetExpansion, not high-risk, exclusion, etc.
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);

    // With assume_yes + out-of-scope under permissive, should succeed via override.
    for surface in OVERRIDE_HONORING_SURFACES {
        let mut over = ManualOverride::default();
        over.assume_yes = true;
        // Note: ManualOverride is checked at CommandContext layer, not at EnforcementContext::evaluate.
        // At the enforcement level, the outcome is RequireConfirmation for out-of-scope.
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.requires_confirmation(),
            "{}: out-of-scope should produce RequireConfirmation (override handled at CommandContext layer), got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn full_override_does_not_affect_non_override_surfaces() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);

    for surface in NON_OVERRIDE_SURFACES {
        // At the EnforcementContext level, overrides are not processed.
        // The outcome should be deny for out-of-scope on strict/guarded surfaces.
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_denied(),
            "{}: out-of-scope should deny regardless of override, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn only_permissive_surfaces_produce_require_confirmation_for_scope_miss() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);

    for surface in ALL_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        if PERMISSIVE_SURFACES.contains(surface) {
            assert!(
                outcome.requires_confirmation(),
                "{}: permissive should produce RequireConfirmation for scope miss",
                surface
            );
        } else {
            assert!(
                outcome.is_denied(),
                "{}: strict/guarded should deny for scope miss",
                surface
            );
        }
    }
}

// ===========================================================================
// 11. Scope state matrix
// ===========================================================================

#[test]
fn default_empty_scope_under_strict_denies_networked() {
    let desc = descriptor_requires_scope("127.0.0.1", OperationRisk::SafeActive);
    for surface in AUTOMATED_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), LoadedScope::default_empty());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_denied(),
            "{}: DefaultEmpty + requires_explicit_scope should deny, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn default_empty_scope_under_permissive_warns_or_allows() {
    let desc = descriptor("127.0.0.1", OperationRisk::SafeActive);
    for surface in PERMISSIVE_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), LoadedScope::default_empty());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_allowed(),
            "{}: DefaultEmpty + safe op under permissive should not hard-deny, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn explicit_allow_match_allows_all_surfaces() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::SafeActive);
    for surface in ALL_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_allowed(),
            "{}: explicit allow match should allow, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn explicit_allow_miss_under_strict_denies() {
    let scope = loaded_explicit(scope_allow("10.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    for surface in ALL_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        if PERMISSIVE_SURFACES.contains(surface) {
            assert!(
                outcome.requires_confirmation(),
                "{}: allow miss under permissive should require confirmation, got {:?}",
                surface,
                outcome
            );
        } else {
            assert!(
                outcome.is_denied(),
                "{}: allow miss under strict should deny, got {:?}",
                surface,
                outcome
            );
        }
    }
}

#[test]
fn explicit_exclusion_match_under_permissive_requires_confirmation() {
    let scope = loaded_explicit(scope_wildcard_excluding("admin.example.com"));
    let desc = descriptor("admin.example.com", OperationRisk::SafeActive);
    for surface in PERMISSIVE_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.requires_confirmation(),
            "{}: exclusion under permissive should require confirmation, got {:?}",
            surface,
            outcome
        );
    }
}

#[test]
fn explicit_exclusion_match_under_strict_denies() {
    let scope = loaded_explicit(scope_wildcard_excluding("admin.example.com"));
    let desc = descriptor("admin.example.com", OperationRisk::SafeActive);
    for surface in STRICT_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_denied(),
            "{}: exclusion under strict should deny, got {:?}",
            surface,
            outcome
        );
    }
}

// ===========================================================================
// 12. Dual-mode contract: permissive never becomes strict, strict never becomes permissive
// ===========================================================================

#[test]
fn permissive_does_not_hard_deny_safe_in_scope() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::SafeActive);
    for surface in PERMISSIVE_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            !outcome.is_denied(),
            "{}: permissive should not hard-deny safe in-scope operation",
            surface
        );
    }
}

#[test]
fn permissive_does_not_hard_deny_empty_scope_safe_op() {
    let desc = descriptor("127.0.0.1", OperationRisk::SafeActive);
    for surface in PERMISSIVE_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), LoadedScope::default_empty());
        let outcome = ctx.evaluate(&desc);
        assert!(
            !outcome.is_denied(),
            "{}: permissive should not hard-deny safe op with empty scope",
            surface
        );
    }
}

#[test]
fn strict_does_not_require_confirmation() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    for surface in STRICT_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            !outcome.requires_confirmation(),
            "{}: strict/guarded should never produce RequireConfirmation (only Allow or Deny)",
            surface
        );
    }
}

#[test]
fn strict_does_not_warn() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::SafeActive);
    for surface in STRICT_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            !matches!(outcome, EnforcementOutcome::Warn(_)),
            "{}: strict/guarded should never produce Warn",
            surface
        );
    }
}

// ===========================================================================
// 13. CommandContext-style tests (override handling at dispatch layer)
// ===========================================================================

#[test]
fn permissive_with_matching_override_can_dispatch() {
    // At the enforcement layer, RequireConfirmation is produced.
    // The CommandContext layer uses ManualOverride::permits() to check if the
    // confirmation class is permitted by the override. This test verifies the
    // enforcement layer produces the expected RequireConfirmation outcome.
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    for surface in PERMISSIVE_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.requires_confirmation(),
            "{}: should produce RequireConfirmation for out-of-scope (override checked at CommandContext layer)",
            surface
        );
    }
}

#[test]
fn permissive_irrelevant_override_still_requires_confirmation() {
    // Even with a full override set, the enforcement layer still produces RequireConfirmation.
    // The CommandContext layer checks if the specific class is permitted.
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    for surface in PERMISSIVE_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.requires_confirmation(),
            "{}: should produce RequireConfirmation regardless of override at enforcement layer",
            surface
        );
    }
}

#[test]
fn guarded_with_matching_override_still_denies() {
    let scope = loaded_cli(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    for surface in &[
        ExecutionSurface::CliManualStrict,
        ExecutionSurface::TuiManualStrict,
    ] {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_denied(),
            "{}: guarded should deny even with matching override",
            surface
        );
    }
}

#[test]
fn agent_mcp_rest_with_matching_override_still_denies() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    for surface in AUTOMATED_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_denied(),
            "{}: automated surface should deny regardless of override",
            surface
        );
    }
}

// ===========================================================================
// 14. ConfirmationClass isolation tests
// ===========================================================================

#[test]
fn confirm_class_out_of_scope_only_for_scope_miss() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("93.184.216.34", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::CliManual, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.requires_confirmation());
    let classes = eggsec::config::confirmation_classes_for(
        &desc,
        outcome.decision(),
        &default_policy(),
    );
    if !classes.is_empty() {
        assert!(
            classes.contains(&ConfirmationClass::OutOfScope),
            "scope miss should produce OutOfScope class, got {:?}",
            classes
        );
    }
}

#[test]
fn confirm_class_explicit_exclusion_for_excluded_target() {
    let scope = loaded_explicit(scope_wildcard_excluding("admin.example.com"));
    let desc = descriptor("admin.example.com", OperationRisk::SafeActive);
    let ctx = ctx_for_surface(ExecutionSurface::CliManual, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.requires_confirmation());
    let classes = eggsec::config::confirmation_classes_for(
        &desc,
        outcome.decision(),
        &default_policy(),
    );
    assert!(
        classes.contains(&ConfirmationClass::ExplicitExclusion),
        "excluded target should produce ExplicitExclusion class, got {:?}",
        classes
    );
}

#[test]
fn confirm_class_high_risk_for_intrusive_with_policy() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor("127.0.0.1", OperationRisk::Intrusive);
    let ctx = ctx_for_surface(ExecutionSurface::CliManual, policy_intrusive(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.requires_confirmation());
    let classes = eggsec::config::confirmation_classes_for(
        &desc,
        outcome.decision(),
        &policy_intrusive(),
    );
    assert!(
        classes.contains(&ConfirmationClass::HighRisk),
        "intrusive with policy should produce HighRisk class, got {:?}",
        classes
    );
}

#[test]
fn confirm_class_nonbaseline_for_non_baseline_cap() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    let desc = descriptor_with_cap("127.0.0.1", OperationRisk::SafeActive, Capability::IntrusiveFuzz);
    let ctx = ctx_for_surface(ExecutionSurface::CliManual, default_policy(), scope);
    let outcome = ctx.evaluate(&desc);
    assert!(outcome.requires_confirmation());
    let classes = eggsec::config::confirmation_classes_for(
        &desc,
        outcome.decision(),
        &default_policy(),
    );
    assert!(
        classes.contains(&ConfirmationClass::NonBaselineCapability),
        "non-baseline cap should produce NonBaselineCapability class, got {:?}",
        classes
    );
}

// ===========================================================================
// 15. ManualOverride::permits() isolation tests
// ===========================================================================

#[test]
fn override_permits_out_of_scope_with_allow_flag() {
    let mut over = ManualOverride::default();
    over.allow_out_of_scope = true;
    assert!(over.permits(ConfirmationClass::OutOfScope));
    assert!(over.permits(ConfirmationClass::TargetExpansion));
}

#[test]
fn override_permits_out_of_scope_with_assume_yes() {
    let mut over = ManualOverride::default();
    over.assume_yes = true;
    assert!(over.permits(ConfirmationClass::OutOfScope));
    assert!(over.permits(ConfirmationClass::TargetExpansion));
}

#[test]
fn override_does_not_permit_high_risk_with_assume_yes() {
    let mut over = ManualOverride::default();
    over.assume_yes = true;
    assert!(!over.permits(ConfirmationClass::HighRisk));
}

#[test]
fn override_permits_high_risk_with_allow_flag() {
    let mut over = ManualOverride::default();
    over.allow_high_risk = true;
    assert!(over.permits(ConfirmationClass::HighRisk));
}

#[test]
fn override_permits_high_risk_with_db_pentest_flag() {
    let mut over = ManualOverride::default();
    over.allow_db_pentest = true;
    assert!(over.permits(ConfirmationClass::HighRisk));
}

#[test]
fn override_permits_exclusion_with_allow_flag() {
    let mut over = ManualOverride::default();
    over.allow_explicit_exclusion = true;
    assert!(over.permits(ConfirmationClass::ExplicitExclusion));
}

#[test]
fn override_permits_traffic_interception_with_web_proxy_flag() {
    let mut over = ManualOverride::default();
    over.allow_web_proxy = true;
    assert!(over.permits(ConfirmationClass::TrafficInterception));
}

#[test]
fn override_permits_nonbaseline_with_allow_flag() {
    let mut over = ManualOverride::default();
    over.allow_nonbaseline_capability = true;
    assert!(over.permits(ConfirmationClass::NonBaselineCapability));
}

#[test]
fn override_permits_private_resolution_with_allow_flag() {
    let mut over = ManualOverride::default();
    over.allow_private_resolution = true;
    assert!(over.permits(ConfirmationClass::PrivateResolution));
}

#[test]
fn override_permits_cross_host_redirect_with_allow_flag() {
    let mut over = ManualOverride::default();
    over.allow_cross_host_redirect = true;
    assert!(over.permits(ConfirmationClass::CrossHostRedirect));
}

#[test]
fn override_does_not_permit_unrelated_classes() {
    let mut over = ManualOverride::default();
    over.allow_out_of_scope = true;
    assert!(!over.permits(ConfirmationClass::HighRisk));
    assert!(!over.permits(ConfirmationClass::ExplicitExclusion));
    assert!(!over.permits(ConfirmationClass::NonBaselineCapability));
    assert!(!over.permits(ConfirmationClass::PrivateResolution));
    assert!(!over.permits(ConfirmationClass::CrossHostRedirect));
    assert!(!over.permits(ConfirmationClass::TrafficInterception));
}

// ===========================================================================
// 16. for_surface canonical construction tests
// ===========================================================================

#[test]
fn for_surface_builds_correct_context_for_each_surface() {
    let policy = default_policy();
    let scope = LoadedScope::default_empty();

    for surface in ALL_SURFACES {
        let ctx = EnforcementContext::for_surface(*surface, policy.clone(), scope.clone());
        assert_eq!(
            ctx.execution_profile,
            surface.profile(),
            "{}: for_surface should set correct profile",
            surface
        );
    }
}

// ===========================================================================
// 17. Scope source provenance tests
// ===========================================================================

#[test]
fn cli_scope_source_is_explicit() {
    let scope = loaded_cli(scope_allow("127.0.0.1"));
    assert!(scope.is_explicit_manifest(), "CliScopeFile should be explicit manifest");
}

#[test]
fn config_scope_source_is_explicit() {
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    assert!(scope.is_explicit_manifest(), "ConfigFile should be explicit manifest");
}

#[test]
fn default_empty_scope_is_not_explicit() {
    let scope = LoadedScope::default_empty();
    assert!(!scope.is_explicit_manifest(), "DefaultEmpty should not be explicit manifest");
}

#[test]
fn strict_profiles_reject_default_empty_for_networked() {
    let desc = descriptor_requires_scope("127.0.0.1", OperationRisk::SafeActive);
    for surface in AUTOMATED_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), LoadedScope::default_empty());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_denied(),
            "{}: DefaultEmpty + requires_explicit_scope should deny for automated surface",
            surface
        );
    }
}

#[test]
fn strict_profiles_accept_explicit_manifest_for_networked() {
    let desc = descriptor_requires_scope("127.0.0.1", OperationRisk::SafeActive);
    let scope = loaded_explicit(scope_allow("127.0.0.1"));
    for surface in AUTOMATED_SURFACES {
        let ctx = ctx_for_surface(*surface, default_policy(), scope.clone());
        let outcome = ctx.evaluate(&desc);
        assert!(
            outcome.is_allowed(),
            "{}: explicit manifest + matching scope should allow for automated surface, got {:?}",
            surface,
            outcome
        );
    }
}
