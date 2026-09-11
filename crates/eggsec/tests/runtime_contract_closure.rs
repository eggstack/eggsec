//! Phase 3 closure: runtime-contract integration guards.
//!
//! Proves the transport/session vs operation-semantics split at the public
//! boundary:
//!
//! - Runtime/daemon owns request IDs, serialization, session attachment, task
//!   lifecycle, cancellation transport, progress/outcome transport.
//! - Engine owns canonical operation identity, normalization/defaults, target
//!   semantics, policy descriptors, executor selection, result semantics.
//!
//! Specifically:
//! 1. `RuntimeSurface` ↔ `ExecutionSurface` is an explicit exhaustive wire
//!    conversion (`Unknown` rejected wire → engine; engine → wire infallible).
//! 2. `TaskKind` is a wire adapter: engine adapters delegate to the single
//!    wire-side match (no parallel operation-ID/target tables), and the
//!    canonical typed conversion (`CanonicalOperationRequest::from_task_kind`)
//!    agrees on identity/target/route for all 29 variants.
//! 3. Wire JSON stability: every `TaskKind` round-trips through serde with a
//!    stable `kind` tag; unknown tags are rejected.
//! 4. No-target/interface families carry `None` and fail descriptor resolution
//!    with `InvalidTarget` (never silent downgrade, never alias fall-through).
//! 5. Result/envelope conversion is single-owned
//!    (`dispatch::task_result_envelope`): stable wire kinds, empty payload,
//!    no artifacts by default.
//! 6. Embedded (TUI `from_task_kind` → `execute_canonical`) and daemon
//!    (bundle → `execute_approved`) paths resolve the same canonical
//!    operation/target for the same request.
//! 7. Approval binding still rejects approve-one-dispatch-another at the
//!    bundle boundary.

use eggsec::config::{metadata_for_tool_id, ExecutionSurface};
use eggsec::dispatch::{
    executor_route_for, task_result_envelope, CanonicalOperationRequest, TaskResult,
};
use eggsec::runtime_bridge::{
    descriptor_for_run_request, execution_surface_to_runtime_surface,
    runtime_surface_to_execution_surface, RuntimeBridgeError,
};
use eggsec_runtime::request::*;
use eggsec_runtime::RuntimeSurface;

fn all_task_kinds() -> Vec<TaskKind> {
    vec![
        TaskKind::LoadTest(LoadTestParams {
            target: "https://example.com".into(),
            method: "GET".into(),
            ..Default::default()
        }),
        TaskKind::StressTest(StressTestParams {
            target: "https://example.com".into(),
            flood_type: "syn".into(),
            ..Default::default()
        }),
        TaskKind::PortScan(PortScanParams {
            target: "10.0.0.1".into(),
            ..Default::default()
        }),
        TaskKind::EndpointScan(EndpointScanParams {
            target: "https://example.com".into(),
            ..Default::default()
        }),
        TaskKind::Fingerprint(FingerprintParams {
            target: "10.0.0.1".into(),
            ..Default::default()
        }),
        TaskKind::Fuzz(FuzzParams {
            target: "https://example.com".into(),
            ..Default::default()
        }),
        TaskKind::Waf(WafParams {
            target: "https://example.com".into(),
            ..Default::default()
        }),
        TaskKind::WafStress(WafStressParams {
            target: "https://example.com".into(),
            ..Default::default()
        }),
        TaskKind::Pipeline(PipelineParams {
            target: "https://example.com".into(),
            profile: None,
        }),
        TaskKind::Recon(ReconParams {
            target: "example.com".into(),
            modules: None,
        }),
        TaskKind::PacketCapture(PacketCaptureParams::default()),
        TaskKind::PacketTraceroute(PacketTracerouteParams {
            target: "10.0.0.1".into(),
            max_hops: None,
        }),
        TaskKind::PacketSend(PacketSendParams {
            target: "10.0.0.1".into(),
            protocol: "tcp".into(),
            ..Default::default()
        }),
        TaskKind::GraphQl(GraphQlParams {
            target: "https://example.com/graphql".into(),
            ..Default::default()
        }),
        TaskKind::OAuth(OAuthParams {
            target: "https://example.com".into(),
            ..Default::default()
        }),
        TaskKind::AuthTest(AuthTestParams {
            target: "https://example.com".into(),
            ..Default::default()
        }),
        TaskKind::Nse(NseParams {
            target: "10.0.0.1".into(),
            script: "default".into(),
            args: None,
        }),
        TaskKind::Hunt(HuntParams {
            target: "https://example.com".into(),
            hunt_type: None,
        }),
        TaskKind::Browser(BrowserParams {
            target: "https://example.com".into(),
            headless: None,
        }),
        TaskKind::Compliance(ComplianceParams {
            target: "https://example.com".into(),
            framework: None,
        }),
        TaskKind::Storage(StorageParams {
            storage_type: "findings".into(),
            path: None,
        }),
        TaskKind::Integrations(IntegrationsParams {
            integration_type: "jira".into(),
            config: None,
        }),
        TaskKind::Workflow(WorkflowParams {
            workflow_id: None,
            steps: None,
        }),
        TaskKind::Vuln(VulnParams {
            target: "https://example.com".into(),
            vuln_type: None,
        }),
        TaskKind::Wireless(WirelessParams {
            interface: None,
            duration_secs: None,
        }),
        TaskKind::WirelessActive(WirelessActiveParams {
            interface: None,
            target_bssid: None,
        }),
        TaskKind::DbPentest(DbPentestParams {
            db_type: "postgres".into(),
            target: "localhost".into(),
            ..Default::default()
        }),
        TaskKind::Intercept(InterceptParams::default()),
        TaskKind::C2(C2Params::default()),
    ]
}

fn run_request_for(kind: TaskKind) -> eggsec_runtime::RunRequest {
    eggsec_runtime::RunRequest {
        task_kind: kind,
        requested_by: None,
        surface: RuntimeSurface::CliManual,
        labels: vec![],
    }
}

// ─── 3.1 Surface ownership ────────────────────────────────────────────

#[test]
fn surface_wire_to_engine_covers_all_known_surfaces() {
    let known = [
        RuntimeSurface::CliManual,
        RuntimeSurface::CliManualStrict,
        RuntimeSurface::TuiManual,
        RuntimeSurface::TuiManualStrict,
        RuntimeSurface::Ci,
        RuntimeSurface::McpServer,
        RuntimeSurface::RestApi,
        RuntimeSurface::GrpcApi,
        RuntimeSurface::SecurityAgent,
    ];
    assert_eq!(known.len(), 9, "surface count changed — update the bridge");
    for rt in known {
        assert!(
            runtime_surface_to_execution_surface(rt.clone()).is_ok(),
            "known surface {rt:?} must map"
        );
    }
}

#[test]
fn surface_unknown_is_rejected_not_mapped() {
    let err = runtime_surface_to_execution_surface(RuntimeSurface::Unknown).unwrap_err();
    assert!(matches!(err, RuntimeBridgeError::UnknownSurface));
}

#[test]
fn surface_conversion_round_trips_without_loss() {
    let known = [
        (RuntimeSurface::CliManual, ExecutionSurface::CliManual),
        (
            RuntimeSurface::CliManualStrict,
            ExecutionSurface::CliManualStrict,
        ),
        (RuntimeSurface::TuiManual, ExecutionSurface::TuiManual),
        (
            RuntimeSurface::TuiManualStrict,
            ExecutionSurface::TuiManualStrict,
        ),
        (RuntimeSurface::Ci, ExecutionSurface::Ci),
        (RuntimeSurface::McpServer, ExecutionSurface::McpServer),
        (RuntimeSurface::RestApi, ExecutionSurface::RestApi),
        (RuntimeSurface::GrpcApi, ExecutionSurface::GrpcApi),
        (
            RuntimeSurface::SecurityAgent,
            ExecutionSurface::SecurityAgent,
        ),
    ];
    for (rt, exec) in known {
        let fwd = runtime_surface_to_execution_surface(rt.clone()).unwrap();
        assert_eq!(fwd, exec);
        let back = execution_surface_to_runtime_surface(&exec);
        assert_eq!(back, rt, "round-trip failed for {rt:?}");
    }
}

// ─── 3.2 TaskKind is a wire adapter, not a semantic registry ──────────

#[test]
fn task_kind_variant_count_is_pinned() {
    assert_eq!(
        all_task_kinds().len(),
        29,
        "TaskKind variant count changed — update the bridge, the canonical \
         conversion, capabilities, and this test together"
    );
}

#[cfg(feature = "cli")]
#[test]
fn engine_adapters_delegate_to_the_single_wire_match() {
    use eggsec::operation_request::runtime_adapters::{
        operation_id_for_task_kind, target_for_task_kind,
    };

    for kind in &all_task_kinds() {
        let wire = kind.operation_id();
        let engine = operation_id_for_task_kind(kind)
            .unwrap_or_else(|| panic!("engine adapter has no mapping for {kind:?}"));
        assert_eq!(
            wire, engine,
            "parallel operation-ID tables diverged for {kind:?}"
        );
        assert_eq!(
            kind.canonical_target(),
            target_for_task_kind(kind),
            "parallel target tables diverged for {kind:?}"
        );
    }
}

#[test]
fn canonical_typed_conversion_agrees_on_identity_and_target() {
    for kind in &all_task_kinds() {
        let canonical = CanonicalOperationRequest::from_task_kind(kind);
        assert_eq!(
            canonical.operation_id(),
            kind.operation_id(),
            "canonical identity diverged for {kind:?}"
        );
        assert_eq!(
            canonical.canonical_target(),
            kind.canonical_target(),
            "canonical target diverged for {kind:?}"
        );
        // Same executor family from either identity spelling.
        assert_eq!(
            executor_route_for(canonical.operation_id()),
            executor_route_for(kind.operation_id()),
            "executor route diverged for {kind:?}"
        );
    }
}

#[test]
fn every_mapped_operation_resolves_to_canonical_metadata() {
    for kind in &all_task_kinds() {
        let op = kind.operation_id();
        assert!(
            metadata_for_tool_id(op).is_some(),
            "{kind:?} maps to '{op}' with no canonical metadata"
        );
    }
}

#[test]
fn multiplexer_families_share_operations_explicitly() {
    let cap = TaskKind::PacketCapture(PacketCaptureParams::default());
    let trace = TaskKind::PacketTraceroute(PacketTracerouteParams {
        target: "10.0.0.1".into(),
        max_hops: None,
    });
    let send = TaskKind::PacketSend(PacketSendParams {
        target: "10.0.0.1".into(),
        protocol: "tcp".into(),
        ..Default::default()
    });
    for kind in [&cap, &trace, &send] {
        assert_eq!(kind.operation_id(), "packet", "{kind:?}");
        assert_eq!(
            CanonicalOperationRequest::from_task_kind(kind).operation_id(),
            "packet"
        );
    }
    assert_eq!(cap.canonical_target(), None);
    assert!(trace.canonical_target().is_some());
    assert!(send.canonical_target().is_some());

    let wifi = TaskKind::Wireless(WirelessParams {
        interface: None,
        duration_secs: None,
    });
    let active = TaskKind::WirelessActive(WirelessActiveParams {
        interface: None,
        target_bssid: None,
    });
    assert_eq!(wifi.operation_id(), "wireless");
    assert_eq!(active.operation_id(), "wireless");

    let waf = TaskKind::Waf(WafParams {
        target: "https://example.com".into(),
        ..Default::default()
    });
    assert_eq!(waf.operation_id(), "waf-detect");
    assert_eq!(
        CanonicalOperationRequest::from_task_kind(&waf).operation_id(),
        "waf-detect"
    );
}

// ─── 3.3 Wire stability + target families ─────────────────────────────

#[test]
fn task_kind_wire_json_is_stable_and_tagged() {
    for kind in &all_task_kinds() {
        let json = serde_json::to_value(kind).expect("TaskKind must serialize");
        let tag = json
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("TaskKind {kind:?} lost its serde tag"));
        assert!(!tag.is_empty());
        let back: TaskKind = serde_json::from_value(json).expect("round-trip");
        assert_eq!(&back, kind);
        assert_eq!(back.operation_id(), kind.operation_id());
    }
}

#[test]
fn unknown_wire_kind_tag_is_rejected() {
    let bad = serde_json::json!({"kind": "does-not-exist", "params": {}});
    let result: Result<TaskKind, _> = serde_json::from_value(bad);
    assert!(
        result.is_err(),
        "unknown wire kind must be rejected, not defaulted"
    );
}

#[test]
fn runtime_surface_wire_labels_are_stable() {
    assert_eq!(RuntimeSurface::CliManual.label(), "cli-manual");
    assert_eq!(RuntimeSurface::TuiManual.label(), "tui-manual");
    assert_eq!(RuntimeSurface::RestApi.label(), "rest-api");
    assert_eq!(RuntimeSurface::Unknown.label(), "unknown");
    let json = serde_json::to_string(&RuntimeSurface::McpServer).unwrap();
    let back: RuntimeSurface = serde_json::from_str(&json).unwrap();
    assert_eq!(back, RuntimeSurface::McpServer);
}

#[test]
fn no_target_and_interface_families_fail_descriptor_explicitly() {
    // These kinds carry no canonical target; the bridge must fail with
    // InvalidTarget (explicit) rather than UnsupportedTaskKind or a silent
    // downgrade.
    let kinds = [
        TaskKind::PacketCapture(PacketCaptureParams::default()),
        TaskKind::Storage(StorageParams {
            storage_type: "findings".into(),
            path: None,
        }),
        TaskKind::Integrations(IntegrationsParams {
            integration_type: "jira".into(),
            config: None,
        }),
        TaskKind::Workflow(WorkflowParams {
            workflow_id: None,
            steps: None,
        }),
        TaskKind::Wireless(WirelessParams {
            interface: None,
            duration_secs: None,
        }),
        TaskKind::WirelessActive(WirelessActiveParams {
            interface: None,
            target_bssid: None,
        }),
    ];
    for kind in &kinds {
        assert_eq!(kind.canonical_target(), None, "{kind:?}");
        let req = run_request_for(kind.clone());
        let err = descriptor_for_run_request(&req).unwrap_err();
        assert!(
            matches!(err, RuntimeBridgeError::InvalidTarget { .. }),
            "{kind:?}: expected InvalidTarget, got {err}"
        );
    }
}

#[test]
fn target_bearing_kinds_resolve_descriptors() {
    let req = run_request_for(TaskKind::PortScan(PortScanParams {
        target: "10.0.0.1".into(),
        ..Default::default()
    }));
    let desc = descriptor_for_run_request(&req).unwrap();
    assert_eq!(desc.operation, "scan-ports");
    assert_eq!(desc.target, Some("10.0.0.1".to_string()));
}

// ─── 3.5 Single-owned envelope mapping ────────────────────────────────

#[test]
fn envelope_kinds_are_stable_wire_discriminators() {
    let cases: Vec<(TaskResult, &str)> = vec![
        (
            TaskResult::PortScan(eggsec::scanner::PortScanResults {
                host: "10.0.0.1".into(),
                ports_scanned: 1000,
                open_ports: vec![],
                total_open_ports: 0,
                results_truncated: false,
                duration_ms: 500,
                spoof_stats: None,
            }),
            "port-scan",
        ),
        (
            TaskResult::PacketCapture {
                packets_captured: 42,
                output_file: None,
            },
            "packet-capture",
        ),
        (TaskResult::PacketTraceroute { hops: vec![] }, "traceroute"),
        (
            TaskResult::PacketSend {
                packets_sent: 10,
                bytes_sent: 640,
            },
            "packet-send",
        ),
        (TaskResult::WafStress(vec![]), "waf-stress"),
        (TaskResult::Error("boom".into()), "error"),
    ];
    for (result, expected_kind) in &cases {
        let envelope = task_result_envelope(result);
        assert_eq!(&envelope.kind, expected_kind);
        assert!(envelope.summary.is_some());
        assert_eq!(envelope.payload, serde_json::json!({}));
        assert!(envelope.artifacts.is_empty());
    }
}

// ─── 3.4 Embedded/daemon seam equivalence ─────────────────────────────

#[test]
fn embedded_and_daemon_paths_resolve_the_same_canonical_operation() {
    // Embedded TUI: TaskKind → CanonicalOperationRequest (from_task_kind).
    // Daemon: RunRequest → OperationDescriptor (descriptor_for_run_request).
    // Both must agree on operation identity and target for the same request.
    for kind in &all_task_kinds() {
        let req = run_request_for(kind.clone());
        let canonical = CanonicalOperationRequest::from_task_kind(&req.task_kind);
        match descriptor_for_run_request(&req) {
            Ok(desc) => {
                assert_eq!(desc.operation, canonical.operation_id());
                assert_eq!(desc.target, canonical.canonical_target());
            }
            Err(RuntimeBridgeError::InvalidTarget { .. }) => {
                assert_eq!(
                    canonical.canonical_target(),
                    None,
                    "{kind:?}: bridge says no target but canonical has one"
                );
            }
            Err(other) => panic!("{kind:?}: unexpected bridge error: {other}"),
        }
    }
}

// ─── Approval binding regression ──────────────────────────────────────

#[test]
fn bundle_binding_predicate_rejects_approve_one_dispatch_another() {
    use eggsec::config::{ExecutionPolicy, LoadedScope};
    use eggsec::runtime_bridge::approve_run_request;

    let port_req = run_request_for(TaskKind::PortScan(PortScanParams {
        target: "10.0.0.1".into(),
        ..Default::default()
    }));
    let approved = approve_run_request(
        RuntimeSurface::CliManual,
        ExecutionPolicy::default(),
        LoadedScope::default_empty(),
        &port_req,
        None,
    )
    .unwrap();

    let fuzz_req = run_request_for(TaskKind::Fuzz(FuzzParams {
        target: "https://example.com".into(),
        ..Default::default()
    }));
    let fuzz_desc = descriptor_for_run_request(&fuzz_req).unwrap();
    // The dispatch wrapper gates on this exact predicate
    // (`ApprovedOperation::matches_descriptor`, derived PartialEq over all
    // policy-relevant fields — never operation-name comparison alone).
    assert!(!approved.matches_descriptor(&fuzz_desc));

    // Same-operation/different-target requests are also rejected.
    let other_req = run_request_for(TaskKind::PortScan(PortScanParams {
        target: "10.0.0.2".into(),
        ..Default::default()
    }));
    let other_desc = descriptor_for_run_request(&other_req).unwrap();
    assert!(!approved.matches_descriptor(&other_desc));
}
