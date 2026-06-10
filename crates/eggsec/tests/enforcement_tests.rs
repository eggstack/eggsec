use eggsec::config::{
    evaluate_enforcement, Capability, DiscoveredTargetStatus, EnforcementOutcome, ExecutionPolicy,
    ExecutionProfile, IntendedUse, OperationDescriptor, OperationMode, OperationRisk,
    PolicyDecision, Scope, ScopeRule,
};

fn make_descriptor(target: &str, risk: OperationRisk) -> OperationDescriptor {
    OperationDescriptor {
        operation: "test-op".to_string(),
        mode: OperationMode::StandardAssessment,
        risk,
        intended_uses: vec![IntendedUse::WebAssessment],
        target: Some(target.to_string()),
        required_features: vec![],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: Vec::new(),
    }
}

fn scope_allow(target: &str) -> Scope {
    Scope {
        allowed_targets: vec![ScopeRule::new(target.to_string())],
        ..Default::default()
    }
}

fn scope_wildcard_with_exclusion(excluded: &str) -> Scope {
    Scope {
        allowed_targets: vec![ScopeRule::new("*".to_string())],
        excluded_targets: vec![ScopeRule::new(excluded.to_string())],
        ..Default::default()
    }
}

#[test]
fn in_scope_target_allowed_all_profiles() {
    let scope = scope_allow("127.0.0.1");
    let descriptor = make_descriptor("127.0.0.1", OperationRisk::SafeActive);
    let policy = ExecutionPolicy::default();

    let profiles = [
        ExecutionProfile::ManualPermissive,
        ExecutionProfile::ManualGuarded,
        ExecutionProfile::CiStrict,
        ExecutionProfile::McpStrict,
        ExecutionProfile::AgentStrict,
    ];

    for profile in &profiles {
        let outcome = evaluate_enforcement(&descriptor, &policy, Some(&scope), *profile);
        assert!(
            outcome.is_allowed(),
            "profile {:?} should allow in-scope target",
            profile
        );
    }
}

#[test]
fn out_of_scope_target_per_profile() {
    let scope = scope_allow("127.0.0.1");
    let descriptor = make_descriptor("93.184.216.34", OperationRisk::SafeActive);
    let policy = ExecutionPolicy::default();

    // ManualPermissive: out-of-scope target is denied at the policy level
    // because evaluate_operation_policy sets allowed=false for out-of-scope targets
    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        Some(&scope),
        ExecutionProfile::ManualPermissive,
    );
    assert!(outcome.is_denied());

    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        Some(&scope),
        ExecutionProfile::ManualGuarded,
    );
    assert!(outcome.is_denied());

    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        Some(&scope),
        ExecutionProfile::CiStrict,
    );
    assert!(outcome.is_denied());

    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        Some(&scope),
        ExecutionProfile::McpStrict,
    );
    assert!(outcome.is_denied());

    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        Some(&scope),
        ExecutionProfile::AgentStrict,
    );
    assert!(outcome.is_denied());
}

#[test]
fn excluded_target_denies_all_profiles() {
    let scope = scope_wildcard_with_exclusion("admin.example.com");
    let descriptor = make_descriptor("admin.example.com", OperationRisk::SafeActive);
    let policy = ExecutionPolicy::default();

    let profiles = [
        ExecutionProfile::ManualPermissive,
        ExecutionProfile::ManualGuarded,
        ExecutionProfile::CiStrict,
        ExecutionProfile::McpStrict,
        ExecutionProfile::AgentStrict,
    ];

    for profile in &profiles {
        let outcome = evaluate_enforcement(&descriptor, &policy, Some(&scope), *profile);
        assert!(
            outcome.is_denied(),
            "profile {:?} should deny excluded target",
            profile
        );
    }
}

#[test]
fn missing_scope_manual_safe_warns() {
    let descriptor = OperationDescriptor {
        operation: "scan".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![IntendedUse::WebAssessment],
        target: Some("127.0.0.1".to_string()),
        required_features: Vec::new(),
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: Vec::new(),
    };
    let mut policy = ExecutionPolicy::default();
    policy.require_explicit_scope = false;

    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        None,
        ExecutionProfile::ManualPermissive,
    );
    assert!(
        outcome.is_allowed(),
        "ManualPermissive with no scope and requires_explicit_scope=false should allow (may warn)"
    );
}

#[test]
fn missing_scope_mcp_networked_denies() {
    let descriptor = OperationDescriptor {
        operation: "scan".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![IntendedUse::WebAssessment],
        target: Some("127.0.0.1".to_string()),
        required_features: Vec::new(),
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: true,
        required_capabilities: Vec::new(),
    };
    let policy = ExecutionPolicy::default();

    let outcome = evaluate_enforcement(&descriptor, &policy, None, ExecutionProfile::McpStrict);
    assert!(
        outcome.is_denied(),
        "McpStrict with no scope and requires_explicit_scope=true should deny"
    );
}

#[test]
fn load_test_denied_without_policy_flag() {
    let descriptor = make_descriptor("127.0.0.1", OperationRisk::LoadTest);
    let policy = ExecutionPolicy::default();

    for profile in &[
        ExecutionProfile::ManualPermissive,
        ExecutionProfile::ManualGuarded,
        ExecutionProfile::CiStrict,
        ExecutionProfile::McpStrict,
        ExecutionProfile::AgentStrict,
    ] {
        let outcome = evaluate_enforcement(&descriptor, &policy, None, *profile);
        assert!(
            outcome.is_denied(),
            "profile {:?} should deny load test without policy flag",
            profile
        );
    }
}

#[test]
fn stress_test_allowed_with_policy_flag() {
    let descriptor = make_descriptor("127.0.0.1", OperationRisk::StressTest);
    let scope = scope_allow("127.0.0.1");
    let mut policy = ExecutionPolicy::default();
    policy.allow_stress_testing = true;

    for profile in &[
        ExecutionProfile::ManualPermissive,
        ExecutionProfile::ManualGuarded,
        ExecutionProfile::CiStrict,
        ExecutionProfile::McpStrict,
        ExecutionProfile::AgentStrict,
    ] {
        let outcome = evaluate_enforcement(&descriptor, &policy, Some(&scope), *profile);
        assert!(
            outcome.is_allowed(),
            "profile {:?} should allow stress test with policy flag",
            profile
        );
    }
}

#[test]
fn mcp_strict_denies_denied_capability() {
    let descriptor = OperationDescriptor {
        operation: "fuzz".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![IntendedUse::WebAssessment],
        target: Some("127.0.0.1".to_string()),
        required_features: vec![],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: vec![Capability::LoadTest],
    };
    let mut policy = ExecutionPolicy::default();
    policy.denied_capabilities = vec![Capability::LoadTest];

    let outcome = evaluate_enforcement(&descriptor, &policy, None, ExecutionProfile::McpStrict);
    assert!(
        outcome.is_denied(),
        "McpStrict should deny operation with denied capability"
    );
}

#[test]
fn agent_strict_denies_denied_capability() {
    let descriptor = OperationDescriptor {
        operation: "remote-exec".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![IntendedUse::WebAssessment],
        target: Some("127.0.0.1".to_string()),
        required_features: vec![],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: vec![Capability::RemoteExecution],
    };
    let mut policy = ExecutionPolicy::default();
    policy.denied_capabilities = vec![Capability::RemoteExecution];

    let outcome = evaluate_enforcement(&descriptor, &policy, None, ExecutionProfile::AgentStrict);
    assert!(
        outcome.is_denied(),
        "AgentStrict should deny operation with denied capability"
    );
}

#[test]
fn json_denial_output_has_required_fields() {
    let scope = scope_allow("127.0.0.1");
    let descriptor = make_descriptor("93.184.216.34", OperationRisk::SafeActive);
    let policy = ExecutionPolicy::default();

    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        Some(&scope),
        ExecutionProfile::McpStrict,
    );
    let json = serde_json::to_value(&outcome).unwrap();

    // EnforcementOutcome::Deny serializes as an object with "deny" variant
    assert!(json.is_object());
    // The inner PolicyDecision fields should be present
    let decision = outcome.decision();
    assert!(!decision.allowed);
    assert!(!decision.decision_id.is_empty());
    assert_eq!(decision.operation_risk, OperationRisk::SafeActive);
    assert!(!decision.denied_reasons.is_empty());
}

#[test]
fn warning_outcome_preserves_warnings() {
    let descriptor = OperationDescriptor {
        operation: "scan".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![IntendedUse::WebAssessment],
        target: Some("127.0.0.1".to_string()),
        required_features: Vec::new(),
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: Vec::new(),
    };
    let policy = ExecutionPolicy::default();

    // ManualPermissive with target but no scope rules -> ambiguous scope -> warns
    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        None,
        ExecutionProfile::ManualPermissive,
    );
    match &outcome {
        EnforcementOutcome::Warn(d) => {
            assert!(
                !d.warnings.is_empty(),
                "Warn outcome should have warnings in the decision"
            );
        }
        _ => panic!("Expected Warn outcome for ambiguous scope under ManualPermissive"),
    }
}

#[test]
fn enforcement_outcome_allow_serializes() {
    let decision = PolicyDecision::allowed(
        "test",
        OperationMode::StandardAssessment,
        OperationRisk::SafeActive,
        vec![IntendedUse::WebAssessment],
    );
    let outcome = EnforcementOutcome::Allow(decision);
    let json = serde_json::to_string(&outcome).unwrap();
    assert!(json.contains("allow"));
}

#[test]
fn enforcement_outcome_deny_serializes() {
    let decision = PolicyDecision::denied(
        "test",
        OperationMode::StandardAssessment,
        OperationRisk::SafeActive,
        vec![IntendedUse::WebAssessment],
        "out of scope",
    );
    let outcome = EnforcementOutcome::Deny(decision);
    let json = serde_json::to_string(&outcome).unwrap();
    assert!(json.contains("deny"));
}

#[test]
fn enforcement_outcome_warn_serializes() {
    let decision = PolicyDecision::allowed(
        "test",
        OperationMode::StandardAssessment,
        OperationRisk::SafeActive,
        vec![IntendedUse::WebAssessment],
    );
    let outcome = EnforcementOutcome::Warn(decision);
    let json = serde_json::to_string(&outcome).unwrap();
    assert!(json.contains("warn"));
}

#[test]
fn execution_profile_display() {
    assert_eq!(
        format!("{}", ExecutionProfile::ManualPermissive),
        "manual-permissive"
    );
    assert_eq!(
        format!("{}", ExecutionProfile::ManualGuarded),
        "manual-guarded"
    );
    assert_eq!(format!("{}", ExecutionProfile::CiStrict), "ci-strict");
    assert_eq!(format!("{}", ExecutionProfile::McpStrict), "mcp-strict");
    assert_eq!(format!("{}", ExecutionProfile::AgentStrict), "agent-strict");
}

#[test]
fn execution_profile_is_automated() {
    assert!(!ExecutionProfile::ManualPermissive.is_automated());
    assert!(!ExecutionProfile::ManualGuarded.is_automated());
    assert!(ExecutionProfile::CiStrict.is_automated());
    assert!(ExecutionProfile::McpStrict.is_automated());
    assert!(ExecutionProfile::AgentStrict.is_automated());
}

#[test]
fn capability_display() {
    assert_eq!(format!("{}", Capability::ActiveProbe), "active-probe");
    assert_eq!(format!("{}", Capability::WafDetect), "waf-detect");
    assert_eq!(format!("{}", Capability::IntrusiveFuzz), "intrusive-fuzz");
    assert_eq!(format!("{}", Capability::LoadTest), "load-test");
    assert_eq!(format!("{}", Capability::NseSafe), "nse-safe");
}

#[test]
fn discovered_target_status_scannable() {
    assert!(DiscoveredTargetStatus::ApprovedInScope.is_scannable());
    assert!(!DiscoveredTargetStatus::Candidate.is_scannable());
    assert!(!DiscoveredTargetStatus::PendingApproval.is_scannable());
    assert!(!DiscoveredTargetStatus::RejectedOutOfScope.is_scannable());
}

#[test]
fn capability_serialization_roundtrip() {
    let cap = Capability::IntrusiveFuzz;
    let json = serde_json::to_string(&cap).unwrap();
    let deserialized: Capability = serde_json::from_str(&json).unwrap();
    assert_eq!(cap, deserialized);
}

#[test]
fn execution_profile_serialization_roundtrip() {
    let profile = ExecutionProfile::McpStrict;
    let json = serde_json::to_string(&profile).unwrap();
    let deserialized: ExecutionProfile = serde_json::from_str(&json).unwrap();
    assert_eq!(profile, deserialized);
}

#[test]
fn enforcement_outcome_is_allowed_distinguishes_allow_and_warn() {
    let allow = EnforcementOutcome::Allow(PolicyDecision::allowed(
        "test",
        OperationMode::StandardAssessment,
        OperationRisk::SafeActive,
        vec![],
    ));
    let warn = EnforcementOutcome::Warn(PolicyDecision::allowed(
        "test",
        OperationMode::StandardAssessment,
        OperationRisk::SafeActive,
        vec![],
    ));
    let deny = EnforcementOutcome::Deny(PolicyDecision::denied(
        "test",
        OperationMode::StandardAssessment,
        OperationRisk::SafeActive,
        vec![],
        "blocked",
    ));

    assert!(allow.is_allowed());
    assert!(!allow.is_denied());
    assert!(warn.is_allowed());
    assert!(!warn.is_denied());
    assert!(!deny.is_allowed());
    assert!(deny.is_denied());
}

#[test]
fn manual_guarded_denies_missing_scope_for_networked() {
    let descriptor = OperationDescriptor {
        operation: "scan".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![IntendedUse::WebAssessment],
        target: Some("127.0.0.1".to_string()),
        required_features: Vec::new(),
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: true,
        required_capabilities: Vec::new(),
    };
    let policy = ExecutionPolicy::default();

    let outcome = evaluate_enforcement(&descriptor, &policy, None, ExecutionProfile::ManualGuarded);
    assert!(
        outcome.is_denied(),
        "ManualGuarded should deny when requires_explicit_scope=true and no scope"
    );
}

#[test]
fn ci_strict_denies_missing_scope_for_networked() {
    let descriptor = OperationDescriptor {
        operation: "scan".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![IntendedUse::WebAssessment],
        target: Some("127.0.0.1".to_string()),
        required_features: Vec::new(),
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: true,
        required_capabilities: Vec::new(),
    };
    let policy = ExecutionPolicy::default();

    let outcome = evaluate_enforcement(&descriptor, &policy, None, ExecutionProfile::CiStrict);
    assert!(
        outcome.is_denied(),
        "CiStrict should deny when requires_explicit_scope=true and no scope"
    );
}

#[test]
fn agent_strict_denies_missing_scope_for_networked() {
    let descriptor = OperationDescriptor {
        operation: "scan".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![IntendedUse::WebAssessment],
        target: Some("127.0.0.1".to_string()),
        required_features: Vec::new(),
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: true,
        required_capabilities: Vec::new(),
    };
    let policy = ExecutionPolicy::default();

    let outcome = evaluate_enforcement(&descriptor, &policy, None, ExecutionProfile::AgentStrict);
    assert!(
        outcome.is_denied(),
        "AgentStrict should deny when requires_explicit_scope=true and no scope"
    );
}

#[test]
fn strict_profiles_deny_ambiguous_scope() {
    let descriptor = OperationDescriptor {
        operation: "scan".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![IntendedUse::WebAssessment],
        target: Some("127.0.0.1".to_string()),
        required_features: Vec::new(),
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: Vec::new(),
    };
    let policy = ExecutionPolicy::default();

    // With scope provided and target matching, allow
    let scope = scope_allow("127.0.0.1");
    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        Some(&scope),
        ExecutionProfile::McpStrict,
    );
    assert!(
        outcome.is_allowed(),
        "McpStrict should allow when target matches scope"
    );

    // Without scope, target is ambiguous -> strict profiles deny
    let outcome = evaluate_enforcement(&descriptor, &policy, None, ExecutionProfile::McpStrict);
    assert!(outcome.is_denied(), "McpStrict should deny ambiguous scope");
}

#[test]
fn risk_policy_enforcement_all_risks() {
    let descriptor = make_descriptor("127.0.0.1", OperationRisk::Intrusive);
    let policy = ExecutionPolicy::default();

    // Intrusive not allowed by default
    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        None,
        ExecutionProfile::ManualPermissive,
    );
    assert!(outcome.is_denied());

    // Enable it
    let mut policy = ExecutionPolicy::default();
    policy.allow_intrusive_fuzzing = true;
    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        None,
        ExecutionProfile::ManualPermissive,
    );
    assert!(outcome.is_allowed());
}

#[test]
fn raw_packet_denied_without_policy() {
    let descriptor = make_descriptor("127.0.0.1", OperationRisk::RawPacket);
    let policy = ExecutionPolicy::default();
    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        None,
        ExecutionProfile::ManualPermissive,
    );
    assert!(outcome.is_denied());
}

#[test]
fn raw_packet_allowed_with_policy() {
    let descriptor = make_descriptor("127.0.0.1", OperationRisk::RawPacket);
    let mut policy = ExecutionPolicy::default();
    policy.allow_raw_packets = true;
    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        None,
        ExecutionProfile::ManualPermissive,
    );
    assert!(outcome.is_allowed());
}

#[test]
fn credential_testing_denied_without_policy() {
    let descriptor = make_descriptor("127.0.0.1", OperationRisk::CredentialTesting);
    let policy = ExecutionPolicy::default();
    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        None,
        ExecutionProfile::ManualPermissive,
    );
    assert!(outcome.is_denied());
}

#[test]
fn credential_testing_allowed_with_policy() {
    let descriptor = make_descriptor("127.0.0.1", OperationRisk::CredentialTesting);
    let mut policy = ExecutionPolicy::default();
    policy.allow_credential_testing = true;
    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        None,
        ExecutionProfile::ManualPermissive,
    );
    assert!(outcome.is_allowed());
}

#[test]
fn exploit_adjacent_denied_without_policy() {
    let descriptor = make_descriptor("127.0.0.1", OperationRisk::ExploitAdjacent);
    let policy = ExecutionPolicy::default();
    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        None,
        ExecutionProfile::ManualPermissive,
    );
    assert!(outcome.is_denied());
}

#[test]
fn exploit_adjacent_allowed_with_policy() {
    let descriptor = make_descriptor("127.0.0.1", OperationRisk::ExploitAdjacent);
    let mut policy = ExecutionPolicy::default();
    policy.allow_exploit_adjacent = true;
    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        None,
        ExecutionProfile::ManualPermissive,
    );
    assert!(outcome.is_allowed());
}

#[test]
fn remote_execution_denied_without_policy() {
    let descriptor = make_descriptor("127.0.0.1", OperationRisk::RemoteExecution);
    let policy = ExecutionPolicy::default();
    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        None,
        ExecutionProfile::ManualPermissive,
    );
    assert!(outcome.is_denied());
}

#[test]
fn remote_execution_allowed_with_policy() {
    let descriptor = make_descriptor("127.0.0.1", OperationRisk::RemoteExecution);
    let mut policy = ExecutionPolicy::default();
    policy.allow_remote_execution = true;
    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        None,
        ExecutionProfile::ManualPermissive,
    );
    assert!(outcome.is_allowed());
}

#[test]
fn agent_autonomous_denied_without_policy() {
    let descriptor = make_descriptor("127.0.0.1", OperationRisk::AgentAutonomous);
    let policy = ExecutionPolicy::default();
    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        None,
        ExecutionProfile::ManualPermissive,
    );
    assert!(outcome.is_denied());
}

#[test]
fn agent_autonomous_allowed_with_policy() {
    let descriptor = make_descriptor("127.0.0.1", OperationRisk::AgentAutonomous);
    let mut policy = ExecutionPolicy::default();
    policy.allow_agent_autonomous = true;
    let outcome = evaluate_enforcement(
        &descriptor,
        &policy,
        None,
        ExecutionProfile::ManualPermissive,
    );
    assert!(outcome.is_allowed());
}

#[test]
fn execution_profile_serialization_contains_correct_variant() {
    let json = serde_json::to_string(&ExecutionProfile::AgentStrict).unwrap();
    assert!(json.contains("agent-strict"));
    let json = serde_json::to_string(&ExecutionProfile::CiStrict).unwrap();
    assert!(json.contains("ci-strict"));
}

#[test]
fn capability_all_variants_serialize() {
    let variants = [
        Capability::PassiveFingerprint,
        Capability::ActiveProbe,
        Capability::Crawl,
        Capability::HttpFuzzLowImpact,
        Capability::IntrusiveFuzz,
        Capability::WafDetect,
        Capability::WafBypassSimulation,
        Capability::WafStressTest,
        Capability::LoadTest,
        Capability::RawPacketProbe,
        Capability::CredentialTesting,
        Capability::RemoteExecution,
        Capability::NseSafe,
        Capability::NseIntrusive,
    ];
    for cap in &variants {
        let json = serde_json::to_string(cap).unwrap();
        let deserialized: Capability = serde_json::from_str(&json).unwrap();
        assert_eq!(*cap, deserialized, "roundtrip failed for {:?}", cap);
    }
}
