//! Phase 1 application-boundary tests: canonical dispatch ownership.
//!
//! Proves that representative operation families converge on one
//! engine-owned execution seam after producing the existing canonical typed
//! operation request and `ApprovedOperation`:
//!
//! - same canonical request normalization (CLI adapter vs runtime adapter);
//! - same canonical operation identity (no alias inference at the boundary);
//! - same target binding (canonical target → descriptor → approval);
//! - same executor route (single-owner `executor_route_for`);
//! - same terminal outcome/error classification (typed `ExecutionError` and
//!   `outcome_kind`, hermetic — no network execution);
//! - cancellation short-circuits before detached work starts (daemon-backed
//!   adapter; embedded TUI adapter covered in `eggsec-tui`).
//!
//! Families (per plan): scan-ports, recon, fuzz, waf-detect, load-test,
//! pipeline, packet family, one auth family (graphql/oauth/auth-test — all
//! three covered), one feature-gated domain operation (db-pentest), one
//! no-target/local-management operation where runtime-supported (storage).
//!
//! These tests run in the mandatory verification path
//! (`cargo test -p eggsec --features rest-api --tests`).

#![cfg(feature = "cli")]

use eggsec::commands::route::{route_for_command_id, route_for_commands};
use eggsec::config::{EnforcementContext, ExecutionPolicy, ExecutionSurface, LoadedScope};
use eggsec::dispatch::{
    execute_approved, executor_route_for, outcome_kind, CanonicalOperationRequest, ExecutionError,
    ExecutionSink,
};
use eggsec::operation_request::{self, validate_tool_request_params};
use eggsec_runtime::request::*;

// ── Helpers ──

fn manual_enforcement() -> EnforcementContext {
    EnforcementContext::for_surface(
        ExecutionSurface::CliManual,
        ExecutionPolicy::default(),
        LoadedScope::default_empty(),
    )
}

fn test_sink() -> (
    ExecutionSink,
    tokio::sync::mpsc::Receiver<eggsec::dispatch::ExecutionEvent>,
) {
    let (event_tx, event_rx) = tokio::sync::mpsc::channel(ExecutionSink::CAPACITY);
    let (legacy_tx, _legacy_rx) = tokio::sync::mpsc::channel(ExecutionSink::CAPACITY);
    (ExecutionSink::new(event_tx, Some(legacy_tx)), event_rx)
}

async fn approve_loopback(
    operation_id: &str,
    target: Option<&str>,
) -> eggsec::config::ApprovedOperation {
    let metadata = eggsec::config::metadata_for_tool_id(operation_id).expect("canonical operation");
    let descriptor = metadata
        .try_descriptor_for_target(target)
        .expect("loopback target must build descriptor");
    manual_enforcement()
        .approve_manual(ExecutionSurface::CliManual, descriptor, None)
        .expect("loopback manual approval must succeed")
}

// ── 1. scan-ports ──

#[test]
fn scan_ports_cli_and_runtime_normalize_identically() {
    let cli = operation_request::PortScanRequest {
        target: "  127.0.0.1  ".into(),
        ports: Some("22,80,443".into()),
        scan_type: Some("syn".into()),
        timeout_ms: None,
        concurrency: None,
    };
    let runtime = operation_request::runtime_adapters::port_scan_from_runtime(&PortScanParams {
        target: "  127.0.0.1  ".into(),
        ports: Some("22,80,443".into()),
        scan_type: Some("syn".into()),
        timeout_ms: None,
        concurrency: None,
    });
    assert_eq!(cli.normalize().unwrap(), runtime.normalize().unwrap());
}

#[test]
fn scan_ports_identity_binding_and_route_agree() {
    let kind = TaskKind::PortScan(PortScanParams {
        target: "127.0.0.1".into(),
        ports: Some("80".into()),
        ..Default::default()
    });
    let canonical = CanonicalOperationRequest::from_task_kind(&kind);
    assert_eq!(canonical.operation_id(), "scan-ports");
    assert_eq!(canonical.operation_id(), kind.operation_id());
    assert_eq!(canonical.canonical_target(), kind.canonical_target());
    assert_eq!(canonical.canonical_target(), Some("127.0.0.1".to_string()));
    // Tool surface resolves to the same canonical ID and validates.
    let params = serde_json::json!({"target": "127.0.0.1", "ports": "80"});
    assert_eq!(
        validate_tool_request_params("scan-ports", &params).unwrap(),
        "scan-ports"
    );
    // Alias `scan` resolves to the same canonical ID (before the boundary).
    let params_alias = serde_json::json!({"target": "127.0.0.1"});
    assert_eq!(
        validate_tool_request_params("scan-ports", &params_alias).unwrap(),
        "scan-ports"
    );
    assert!(eggsec::config::operation_matches_tool_id(
        "scan",
        "scan-ports"
    ));
    // Executor route is single-owned.
    assert_eq!(executor_route_for("scan-ports"), "scanner");
    assert_eq!(
        executor_route_for(canonical.operation_id()),
        executor_route_for(kind.operation_id())
    );
    // CLI route resolves to the canonical ID (not the alias).
    let route = route_for_command_id("scan-ports").expect("registered");
    assert_eq!(route.operation_id(), Some("scan-ports"));
}

// ── 2. recon ──

#[test]
fn recon_cli_and_runtime_normalize_identically() {
    let cli = operation_request::ReconRequest {
        target: "  127.0.0.1  ".into(),
        modules: None,
    };
    let runtime = operation_request::runtime_adapters::recon_from_runtime(&ReconParams {
        target: "  127.0.0.1  ".into(),
        modules: None,
    });
    assert_eq!(cli.normalize().unwrap(), runtime.normalize().unwrap());
}

#[test]
fn recon_identity_binding_and_route_agree() {
    let kind = TaskKind::Recon(ReconParams {
        target: "127.0.0.1".into(),
        modules: None,
    });
    let canonical = CanonicalOperationRequest::from_task_kind(&kind);
    assert_eq!(canonical.operation_id(), "recon");
    assert_eq!(canonical.canonical_target(), Some("127.0.0.1".to_string()));
    let params = serde_json::json!({"target": "127.0.0.1"});
    assert_eq!(
        validate_tool_request_params("recon", &params).unwrap(),
        "recon"
    );
    assert_eq!(executor_route_for("recon"), "recon");
}

// ── 3. fuzz ──

#[test]
fn fuzz_cli_and_runtime_normalize_identically() {
    let cli = operation_request::FuzzRequest {
        target: "http://127.0.0.1:8080".into(),
        payload_type: None,
        mode: None,
        method: None,
        param: None,
        threads: None,
        timeout_secs: None,
        mutations: None,
        mutation_count: None,
        graphql_introspection: None,
        graphql_depth_bypass: None,
        graphql_alias_overload: None,
        oauth_redirect_test: None,
        oauth_scope_test: None,
        oauth_state_test: None,
        oauth_grant_test: None,
    };
    let runtime = operation_request::runtime_adapters::fuzz_from_runtime(&FuzzParams {
        target: "http://127.0.0.1:8080".into(),
        ..Default::default()
    });
    assert_eq!(cli.normalize().unwrap(), runtime.normalize().unwrap());
}

#[test]
fn fuzz_identity_binding_and_route_agree() {
    let kind = TaskKind::Fuzz(FuzzParams {
        target: "http://127.0.0.1:8080".into(),
        ..Default::default()
    });
    let canonical = CanonicalOperationRequest::from_task_kind(&kind);
    assert_eq!(canonical.operation_id(), "fuzz");
    assert_eq!(executor_route_for("fuzz"), "fuzz-api");
}

// ── 4. waf-detect (alias `waf` resolved before boundary) ──

#[test]
fn waf_detect_alias_resolves_before_boundary() {
    // CLI `waf` alias → canonical `waf-detect`; the boundary never sees `waf`.
    let route = route_for_command_id("waf").expect("waf registered");
    assert_eq!(route.operation_id(), Some("waf-detect"));
    let kind = TaskKind::Waf(WafParams {
        target: "http://127.0.0.1:8080".into(),
        ..Default::default()
    });
    let canonical = CanonicalOperationRequest::from_task_kind(&kind);
    assert_eq!(canonical.operation_id(), "waf-detect");
    assert_ne!(canonical.operation_id(), "waf");
    assert_eq!(executor_route_for("waf-detect"), "waf");
}

// ── 5. load-test (alias `load` resolved before boundary) ──

#[test]
fn load_test_legacy_connections_normalize_identically() {
    let cli = operation_request::LoadTestRequest {
        target: "http://127.0.0.1:8080".into(),
        method: Some("GET".into()),
        requests: None,
        connections: Some(25),
        duration_secs: None,
        rate_limit: None,
    };
    let runtime = operation_request::runtime_adapters::load_test_from_runtime(&LoadTestParams {
        target: "http://127.0.0.1:8080".into(),
        method: "GET".into(),
        requests: None,
        connections: Some(25),
        duration_secs: None,
        rate_limit: None,
    });
    let a = cli.normalize().unwrap();
    let b = runtime.normalize().unwrap();
    assert_eq!(a, b);
    // Legacy connections-only means 25 total requests, 25 concurrency.
    assert_eq!((a.requests, a.concurrency), (25, 25));
}

#[test]
fn load_test_identity_and_route_agree() {
    let kind = TaskKind::LoadTest(LoadTestParams {
        target: "http://127.0.0.1:8080".into(),
        method: "GET".into(),
        ..Default::default()
    });
    let canonical = CanonicalOperationRequest::from_task_kind(&kind);
    assert_eq!(canonical.operation_id(), "load-test");
    assert_eq!(executor_route_for("load-test"), "network");
    let route = route_for_command_id("load").expect("load registered");
    // Registry maps `load` → canonical `load-test`.
    assert_eq!(route.operation_id(), Some("load-test"));
}

// ── 6. pipeline (multiplexer `scan`/`resume` resolved before boundary) ──

#[test]
fn pipeline_multiplexer_resolves_before_boundary() {
    for cmd in ["scan", "resume"] {
        let route = route_for_command_id(cmd).expect("pipeline multiplexer registered");
        assert!(route.is_operation_backed());
        assert!(route.operations().contains(&"pipeline"));
    }
    let kind = TaskKind::Pipeline(PipelineParams {
        target: "127.0.0.1".into(),
        profile: None,
    });
    let canonical = CanonicalOperationRequest::from_task_kind(&kind);
    assert_eq!(canonical.operation_id(), "pipeline");
    assert_eq!(executor_route_for("pipeline"), "recon");
    // Unknown profiles fail closed at normalization (no silent downgrade).
    let bad = operation_request::PipelineRequest {
        target: "127.0.0.1".into(),
        profile: Some("bogus-profile".into()),
    };
    assert!(bad.normalize().is_err());
}

// ── 7. packet family ──

#[test]
fn packet_family_shares_operation_with_explicit_wire_kinds() {
    let cap = TaskKind::PacketCapture(PacketCaptureParams::default());
    let trace = TaskKind::PacketTraceroute(PacketTracerouteParams {
        target: "127.0.0.1".into(),
        max_hops: None,
    });
    let send = TaskKind::PacketSend(PacketSendParams {
        target: "127.0.0.1".into(),
        protocol: "tcp".into(),
        ..Default::default()
    });
    for kind in [&cap, &trace, &send] {
        assert_eq!(kind.operation_id(), "packet");
        let canonical = CanonicalOperationRequest::from_task_kind(kind);
        assert_eq!(canonical.operation_id(), "packet");
        assert_eq!(executor_route_for(canonical.operation_id()), "network");
    }
    // Interface-bound capture carries no target; traceroute/send do.
    assert_eq!(cap.canonical_target(), None);
    assert_eq!(trace.canonical_target(), Some("127.0.0.1".to_string()));
    assert_eq!(send.canonical_target(), Some("127.0.0.1".to_string()));
    // CLI multiplexers resolve to the packet family before approval.
    for cmd in ["packet", "icmp", "traceroute"] {
        let route = route_for_command_id(cmd).expect("packet multiplexer registered");
        assert!(route.is_operation_backed());
        assert!(route.operations().contains(&"packet"));
    }
}

// ── 8. auth family (graphql/oauth/auth-test) ──

#[test]
fn graphql_identity_and_route_agree() {
    let kind = TaskKind::GraphQl(GraphQlParams {
        target: "http://127.0.0.1:8080/graphql".into(),
        ..Default::default()
    });
    let canonical = CanonicalOperationRequest::from_task_kind(&kind);
    assert_eq!(canonical.operation_id(), "graphql");
    assert_eq!(executor_route_for("graphql"), "fuzz-api");
    let params = serde_json::json!({"target": "http://127.0.0.1:8080/graphql"});
    assert_eq!(
        validate_tool_request_params("graphql", &params).unwrap(),
        "graphql"
    );
}

#[test]
fn oauth_identity_and_route_agree() {
    let kind = TaskKind::OAuth(OAuthParams {
        target: "http://127.0.0.1:8080".into(),
        ..Default::default()
    });
    let canonical = CanonicalOperationRequest::from_task_kind(&kind);
    assert_eq!(canonical.operation_id(), "oauth");
    assert_eq!(executor_route_for("oauth"), "fuzz-api");
    // `o-auth` is a Clap input-boundary alias for the canonical `oauth`
    // command (cli/mod.rs `alias = "o-auth"`); it resolves to the same
    // canonical operation without creating a second identity (covered by the
    // matrix Clap-alias test). The execution boundary only ever sees `oauth`.
    {
        use clap::CommandFactory;
        let clap = eggsec::cli::Cli::command();
        let oauth = clap
            .get_subcommands()
            .find(|c| c.get_name() == "oauth")
            .expect("oauth must exist in the Clap tree");
        assert!(
            oauth.get_aliases().any(|a| a == "o-auth"),
            "oauth Clap command must keep the o-auth input alias"
        );
    }
}

#[test]
fn auth_test_identity_and_route_agree() {
    let kind = TaskKind::AuthTest(AuthTestParams {
        target: "http://127.0.0.1:8080".into(),
        ..Default::default()
    });
    let canonical = CanonicalOperationRequest::from_task_kind(&kind);
    assert_eq!(canonical.operation_id(), "auth-test");
    assert_eq!(executor_route_for("auth-test"), "network");
}

// ── 9. feature-gated domain operation (db-pentest) ──

#[test]
fn db_pentest_identity_and_route_agree() {
    let kind = TaskKind::DbPentest(DbPentestParams {
        db_type: "postgres".into(),
        target: "127.0.0.1".into(),
        ..Default::default()
    });
    let canonical = CanonicalOperationRequest::from_task_kind(&kind);
    assert_eq!(canonical.operation_id(), "db-pentest");
    assert_eq!(executor_route_for("db-pentest"), "db-pentest");
    let params = serde_json::json!({"target": "127.0.0.1", "db_type": "postgres"});
    assert_eq!(
        validate_tool_request_params("db-pentest", &params).unwrap(),
        "db-pentest"
    );
}

#[tokio::test]
async fn db_pentest_feature_gate_is_consistent_at_boundary() {
    // Hermetic: db-pentest without its feature must fail with the same typed
    // classification whether reached via the canonical boundary or the legacy
    // shim. With `--features rest-api` (no db-pentest), expect FeatureUnavailable.
    use eggsec::dispatch::ExecutionError;
    let kind = TaskKind::DbPentest(DbPentestParams {
        db_type: "postgres".into(),
        target: "127.0.0.1".into(),
        ..Default::default()
    });
    let canonical = CanonicalOperationRequest::from_task_kind(&kind);
    let (sink, _rx) = test_sink();
    let result = eggsec::dispatch::execute_canonical(canonical, &sink).await;
    #[cfg(feature = "db-pentest")]
    {
        // With the feature compiled, a dry-run localhost probe executes (may
        // succeed or fail on connection, but must not report FeatureUnavailable).
        match result {
            Err(ExecutionError::FeatureUnavailable { .. }) => {
                panic!("db-pentest feature is compiled; must not report FeatureUnavailable")
            }
            _ => {}
        }
    }
    #[cfg(not(feature = "db-pentest"))]
    {
        assert!(
            matches!(result, Err(ExecutionError::FeatureUnavailable { .. })),
            "expected typed FeatureUnavailable, got {result:?}"
        );
    }
}

// ── 10. no-target/local-management (storage) ──

#[test]
fn storage_identity_and_route_agree() {
    let kind = TaskKind::Storage(StorageParams {
        storage_type: "findings".into(),
        path: None,
    });
    let canonical = CanonicalOperationRequest::from_task_kind(&kind);
    assert_eq!(canonical.operation_id(), "storage");
    assert_eq!(canonical.canonical_target(), None);
    assert_eq!(kind.canonical_target(), None);
    assert_eq!(executor_route_for("storage"), "security");
    let params = serde_json::json!({"storage_type": "findings"});
    assert_eq!(
        validate_tool_request_params("storage", &params).unwrap(),
        "storage"
    );
}

// ── Binding: approval is exact (operation + normalized target) ──

#[tokio::test]
async fn boundary_rejects_operation_mismatch() {
    let approved = approve_loopback("scan-ports", Some("127.0.0.1")).await;
    let request = CanonicalOperationRequest::Recon(operation_request::ReconRequest {
        target: "127.0.0.1".into(),
        modules: None,
    });
    let (sink, _rx) = test_sink();
    let result = execute_approved(&approved, request, &sink).await;
    assert!(
        matches!(result, Err(ExecutionError::BindingMismatch { .. })),
        "expected BindingMismatch, got {result:?}"
    );
}

#[tokio::test]
async fn boundary_rejects_target_mismatch() {
    let approved = approve_loopback("scan-ports", Some("127.0.0.1")).await;
    let request = CanonicalOperationRequest::PortScan(operation_request::PortScanRequest {
        target: "127.0.0.2".into(),
        ports: Some("80".into()),
        scan_type: None,
        timeout_ms: None,
        concurrency: None,
    });
    let (sink, _rx) = test_sink();
    let result = execute_approved(&approved, request, &sink).await;
    assert!(
        matches!(result, Err(ExecutionError::BindingMismatch { .. })),
        "expected BindingMismatch, got {result:?}"
    );
}

#[tokio::test]
async fn boundary_rejects_unresolved_alias() {
    // Even though `scan` is a valid tool alias for `scan-ports`, the boundary
    // requires the canonical ID: an approval for `scan-ports` presented with
    // a request whose operation ID is not exactly `scan-ports` must fail.
    // (Canonical requests always carry canonical IDs; this test pins the
    // exact-equality check against future alias-tolerant regressions.)
    let approved = approve_loopback("scan-ports", Some("127.0.0.1")).await;
    assert_eq!(approved.descriptor().operation, "scan-ports");
    // A recon request under a scan-ports approval is the mismatch probe.
    let request = CanonicalOperationRequest::Recon(operation_request::ReconRequest {
        target: "127.0.0.1".into(),
        modules: None,
    });
    assert_ne!(request.operation_id(), approved.descriptor().operation);
    let (sink, _rx) = test_sink();
    assert!(matches!(
        execute_approved(&approved, request, &sink).await,
        Err(ExecutionError::BindingMismatch { .. })
    ));
}

// ── Terminal outcome classes (hermetic: typed errors + outcome kinds) ──

#[test]
fn invalid_requests_share_error_classification_across_surfaces() {
    // CLI-shaped, runtime-shaped, and tool-shaped invalid inputs must all
    // fail at canonical normalization with the same classification.
    let cli = operation_request::PortScanRequest {
        target: "   ".into(),
        ports: None,
        scan_type: None,
        timeout_ms: None,
        concurrency: None,
    };
    assert!(cli.normalize().is_err());
    let runtime = operation_request::runtime_adapters::port_scan_from_runtime(&PortScanParams {
        target: "   ".into(),
        ..Default::default()
    });
    assert!(runtime.normalize().is_err());
    let bad = serde_json::json!({"target": "   "});
    assert!(validate_tool_request_params("scan-ports", &bad).is_err());
}

#[test]
fn outcome_kinds_are_frontend_neutral() {
    use eggsec::dispatch::TaskResult;
    assert_eq!(
        outcome_kind(&TaskResult::Error("x".into())),
        "error".to_string()
    );
    assert_eq!(
        outcome_kind(&TaskResult::PacketCapture {
            packets_captured: 0,
            output_file: None,
        }),
        "packet-capture".to_string()
    );
}

// ── CommandRoute: single coherent owner with registry ──

#[test]
fn command_route_covers_operation_backed_commands() {
    use eggsec::cli::Commands;
    // Exhaustiveness of route_for_commands is compiler-enforced (no wildcard).
    // Here we pin representative alias resolutions.
    let ids = [
        ("scan-ports", Some("scan-ports")),
        ("waf", Some("waf-detect")),
        ("load", Some("load-test")),
        ("oauth", Some("oauth")),
    ];
    for (cmd_id, expected_op) in ids {
        let route = route_for_command_id(cmd_id).expect("registered");
        assert_eq!(route.operation_id(), expected_op);
    }
    // Helper/lifecycle remain explicit non-operation routes.
    for cmd_id in ["plan", "config", "doctor", "serve", "cluster"] {
        if let Some(route) = route_for_command_id(cmd_id) {
            assert!(route.is_non_operation(), "{cmd_id} must stay non-operation");
        }
    }
    let _ = route_for_commands as fn(&Commands) -> eggsec::commands::route::CommandRoute;
}

// ── Cancellation: shared primitive + daemon-bundle race ──
//
// Both adapters (`TuiExecutor` embedded, `EggsecRuntimeExecutor`
// daemon-backed) race their dispatch future against the runtime cancellation
// token with identical semantics (pre-cancel never starts work; mid-execution
// cancel drops the future, releasing senders so forwarders drain instead of
// leaking). The shared primitive lives in `eggsec-runtime::cancel`; these
// hermetic tests pin its contract plus the daemon bundle's raced future.
// The embedded adapter's identical race is pinned by the TUI-side test
// (`task_runtime::tests::embedded_adapter_cancels_before_detached_work`).

#[tokio::test]
async fn shared_cancel_primitive_never_starts_pre_cancelled_work() {
    use eggsec_runtime::{race_with_cancel, RuntimeError};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let started = Arc::new(AtomicBool::new(false));
    let started_clone = started.clone();
    let cancel = eggsec_runtime::CancellationToken::new();
    cancel.cancel();

    let result = race_with_cancel(
        async move {
            started_clone.store(true, Ordering::SeqCst);
            Ok::<_, RuntimeError>("must not run")
        },
        cancel,
    )
    .await;
    assert!(matches!(result, Err(RuntimeError::DispatchFailed(_))));
    assert!(
        !started.load(Ordering::SeqCst),
        "pre-cancelled work must never start"
    );
}

#[tokio::test]
async fn shared_cancel_primitive_releases_senders_on_cancel() {
    use eggsec_runtime::{race_with_cancel, RuntimeError};

    let (tx, mut rx) = tokio::sync::mpsc::channel::<u32>(4);
    let cancel = eggsec_runtime::CancellationToken::new();
    let cancel_clone = cancel.clone();
    let dispatch = async move {
        let _held = tx;
        std::future::pending::<()>().await;
        #[allow(unreachable_code)]
        Ok::<_, RuntimeError>("never")
    };
    tokio::spawn(async move {
        tokio::task::yield_now().await;
        cancel_clone.cancel();
    });
    let result = race_with_cancel(dispatch, cancel).await;
    assert!(matches!(result, Err(RuntimeError::DispatchFailed(_))));
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
            .await
            .map(|v| v.is_none())
            .unwrap_or(false),
        "channel must close after cancellation (no detached sender)"
    );
}

#[tokio::test]
async fn daemon_bundle_race_resolves_cancel_without_execution() {
    // Mirror of the daemon adapter's race: an approved bundle's dispatch
    // future raced against a pre-cancelled token must resolve to cancel
    // without producing a TaskResult. No network executes because the cancel
    // branch wins before the dispatch future completes.
    use eggsec::runtime_bridge::approve_run_request_bundle;

    let request = eggsec_runtime::RunRequest {
        task_kind: TaskKind::PortScan(PortScanParams {
            target: "127.0.0.1".into(),
            ports: Some("80".into()),
            ..Default::default()
        }),
        requested_by: None,
        surface: eggsec_runtime::RuntimeSurface::CliManual,
        labels: vec![],
    };
    let bundle = approve_run_request_bundle(
        eggsec_runtime::RuntimeSurface::CliManual,
        eggsec::config::ExecutionPolicy::default(),
        eggsec::config::LoadedScope::default_empty(),
        request,
        None,
    )
    .expect("loopback bundle must approve");
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(16);
    // Drain progress so a started dispatch cannot block on delivery.
    let drain = tokio::spawn(async move { while progress_rx.recv().await.is_some() {} });
    let cancel = eggsec_runtime::CancellationToken::new();
    cancel.cancel();

    let outcome = tokio::select! {
        result = eggsec::runtime_bridge::dispatch_approved_runtime_request(bundle, progress_tx) => {
            format!("dispatched: {result:?}")
        }
        _ = cancel.cancelled() => "cancelled".to_string(),
    };
    assert_eq!(outcome, "cancelled");
    drain.abort();
}
