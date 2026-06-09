use eggsec::config::*;
use eggsec::probe::*;

#[test]
fn default_policy_blocks_intrusive_load_stress_raw_credential_exploit_remote_autonomous() {
    let policy = ExecutionPolicy::default();

    assert!(OperationRisk::Passive.is_allowed_by(&policy));
    assert!(OperationRisk::SafeActive.is_allowed_by(&policy));

    assert!(!OperationRisk::Intrusive.is_allowed_by(&policy));
    assert!(!OperationRisk::LoadTest.is_allowed_by(&policy));
    assert!(!OperationRisk::StressTest.is_allowed_by(&policy));
    assert!(!OperationRisk::RawPacket.is_allowed_by(&policy));
    assert!(!OperationRisk::CredentialTesting.is_allowed_by(&policy));
    assert!(!OperationRisk::ExploitAdjacent.is_allowed_by(&policy));
    assert!(!OperationRisk::RemoteExecution.is_allowed_by(&policy));
    assert!(!OperationRisk::AgentAutonomous.is_allowed_by(&policy));
}

#[test]
fn operation_risk_serialization_is_stable() {
    let cases: &[(OperationRisk, &str)] = &[
        (OperationRisk::Passive, "\"passive\""),
        (OperationRisk::SafeActive, "\"safe_active\""),
        (OperationRisk::Intrusive, "\"intrusive\""),
        (OperationRisk::LoadTest, "\"load_test\""),
        (OperationRisk::StressTest, "\"stress_test\""),
        (OperationRisk::RawPacket, "\"raw_packet\""),
        (OperationRisk::CredentialTesting, "\"credential_testing\""),
        (OperationRisk::ExploitAdjacent, "\"exploit_adjacent\""),
        (OperationRisk::RemoteExecution, "\"remote_execution\""),
        (OperationRisk::AgentAutonomous, "\"agent_autonomous\""),
    ];
    for (variant, expected) in cases {
        let json = serde_json::to_string(variant).unwrap();
        assert_eq!(
            json, *expected,
            "OperationRisk::{:?} serialization mismatch",
            variant
        );
    }
}

#[test]
fn operation_mode_default_max_risk_is_correct() {
    assert_eq!(
        OperationMode::StandardAssessment.default_max_risk(),
        OperationRisk::SafeActive
    );
    assert_eq!(
        OperationMode::DefenseLab.default_max_risk(),
        OperationRisk::Intrusive
    );
    assert_eq!(
        OperationMode::HazardousLab.default_max_risk(),
        OperationRisk::AgentAutonomous
    );
}

#[test]
fn intended_use_serialization_is_stable() {
    let cases: &[(IntendedUse, &str)] = &[
        (IntendedUse::WebAssessment, "\"web-assessment\""),
        (IntendedUse::ApiAssessment, "\"api-assessment\""),
        (IntendedUse::WafRegression, "\"waf-regression\""),
        (IntendedUse::SynvoidRegression, "\"synvoid-regression\""),
        (
            IntendedUse::DistributedSystemStress,
            "\"distributed-system-stress\"",
        ),
        (
            IntendedUse::ProtocolEdgeValidation,
            "\"protocol-edge-validation\"",
        ),
        (IntendedUse::CiRegression, "\"ci-regression\""),
        (
            IntendedUse::CodingAgentVerification,
            "\"coding-agent-verification\"",
        ),
    ];
    for (variant, expected) in cases {
        let json = serde_json::to_string(variant).unwrap();
        assert_eq!(
            json, *expected,
            "IntendedUse::{:?} serialization mismatch",
            variant
        );
    }
}

#[test]
fn probe_risk_to_operation_risk_mapping() {
    let cases: &[(ProbeRisk, OperationRisk)] = &[
        (ProbeRisk::Passive, OperationRisk::Passive),
        (ProbeRisk::SafeActive, OperationRisk::SafeActive),
        (ProbeRisk::Intrusive, OperationRisk::Intrusive),
        (ProbeRisk::Credentialed, OperationRisk::CredentialTesting),
        (ProbeRisk::Stress, OperationRisk::StressTest),
        (ProbeRisk::ExploitAdjacent, OperationRisk::ExploitAdjacent),
    ];
    for (probe_risk, expected_operation_risk) in cases {
        let mapped = probe_risk.to_operation_risk();
        assert_eq!(
            mapped, *expected_operation_risk,
            "ProbeRisk::{:?} should map to OperationRisk::{:?}",
            probe_risk, expected_operation_risk
        );
    }
}

#[test]
fn policy_decision_golden_json() {
    let decision = PolicyDecision::allowed(
        "waf-detect",
        OperationMode::StandardAssessment,
        OperationRisk::SafeActive,
        vec![IntendedUse::WafRegression],
    )
    .with_target("127.0.0.1", "127.0.0.1")
    .with_required_feature("packet-inspection")
    .with_warning("private IP");

    let json = serde_json::to_value(&decision).unwrap();

    assert!(json.is_object());
    assert!(json.get("decision_id").is_some());
    assert_eq!(json["allowed"], true);
    assert_eq!(json["operation"], "waf-detect");
    assert_eq!(json["operation_mode"], "standard-assessment");
    assert_eq!(json["operation_risk"], "safe_active");
    assert!(json["intended_uses"].is_array());
    assert_eq!(json["intended_uses"][0], "waf-regression");
    assert_eq!(json["target_original"], "127.0.0.1");
    assert_eq!(json["target_normalized"], "127.0.0.1");
    assert!(json["resolved_addresses"].is_array());
    assert!(json["matched_scope_rules"].is_array());
    assert!(json["matched_exclusion_rules"].is_array());
    assert!(json["required_features"].is_array());
    assert_eq!(json["required_features"][0], "packet-inspection");
    assert!(json["missing_features"].is_array());
    assert!(json["required_policy_flags"].is_array());
    assert!(json["denied_reasons"].is_array());
    assert!(json["warnings"].is_array());
    assert_eq!(json["warnings"][0], "private IP");
}

#[test]
fn evaluate_operation_policy_allowed_localhost() {
    let scope = Scope {
        allowed_targets: vec![ScopeRule::new("127.0.0.1".to_string())],
        ..Default::default()
    };
    let descriptor = OperationDescriptor {
        operation: "scan-ports".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![IntendedUse::WebAssessment],
        target: Some("127.0.0.1".to_string()),
        required_features: vec![],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
    };
    let policy = ExecutionPolicy::default();
    let decision = evaluate_operation_policy(&descriptor, &policy, Some(&scope));
    assert!(decision.allowed);
    assert!(decision
        .matched_scope_rules
        .iter()
        .any(|r| r.contains("target in scope")));
}

#[test]
fn evaluate_operation_policy_denied_by_risk() {
    let descriptor = OperationDescriptor {
        operation: "stress".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::StressTest,
        intended_uses: vec![IntendedUse::DistributedSystemStress],
        target: Some("127.0.0.1".to_string()),
        required_features: vec![],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
    };
    let policy = ExecutionPolicy::default();
    let decision = evaluate_operation_policy(&descriptor, &policy, None);
    assert!(!decision.allowed);
    assert!(decision
        .denied_reasons
        .iter()
        .any(|r| r.contains("not allowed by current execution policy")));
}

#[test]
fn evaluate_operation_policy_denied_missing_scope() {
    let descriptor = OperationDescriptor {
        operation: "fuzz".to_string(),
        mode: OperationMode::DefenseLab,
        risk: OperationRisk::Intrusive,
        intended_uses: vec![IntendedUse::WafRegression],
        target: Some("127.0.0.1".to_string()),
        required_features: vec![],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: true,
    };
    let policy = ExecutionPolicy::default();
    let decision = evaluate_operation_policy(&descriptor, &policy, None);
    assert!(!decision.allowed);
    assert!(decision
        .denied_reasons
        .iter()
        .any(|r| r.contains("scope file required")));
}
