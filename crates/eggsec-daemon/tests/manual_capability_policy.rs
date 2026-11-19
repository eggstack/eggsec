//! The daemon's published capability list and its admitted capability list
//! are the **same object**.
//!
//! Milestone 010 (`ADR-0011`). The daemon used to publish a hand-written
//! "conservative" list while the runtime enforced a different one: the two
//! disagreed, so the daemon advertised capabilities it would refuse and
//! refused capabilities it advertised. Publication and admission are now one
//! list, computed once at construction.
//!
//! These tests assert the property at the daemon's own boundary: what the
//! daemon publishes over the wire is exactly what the runtime admits.

// The whole file is gated on the `service` cargo feature: a daemon built
// without the profile links no engine, so there is no engine executor to
// host.
#![cfg(feature = "service")]

use eggsec_daemon::config::DaemonConfig;
use eggsec_daemon::host::DaemonHost;
use eggsec_daemon::protocol::{ClientCommand, DaemonRequestContext, ServerMessage, TransportKind};
use eggsec_daemon::store::noop_store;
use eggsec_runtime::request::{
    DbPentestParams, InterceptParams, ResumeParams, StressTestParams, TaskKind,
    WirelessActiveParams,
};
use eggsec_runtime::{RunRequest, RuntimeSurface};

fn request_context() -> DaemonRequestContext {
    DaemonRequestContext {
        client_id: None,
        peer: None,
        transport: TransportKind::UnixSocket,
    }
}

/// The names the daemon publishes over the wire.
async fn published_names(host: &DaemonHost) -> Vec<String> {
    let response = host
        .handle_command(
            ClientCommand::Capabilities {
                request_id: "cap".into(),
            },
            request_context(),
        )
        .await;
    match response {
        ServerMessage::Capabilities { capabilities, .. } => capabilities
            .runtime
            .task_kinds
            .iter()
            .map(|capability| capability.name.clone())
            .collect(),
        other => panic!("expected Capabilities, got {other:?}"),
    }
}

/// The daemon publishes the canonical list, and the runtime enforces exactly
/// the list the daemon publishes.
///
/// This is the property the milestone exists for: the two are the same
/// object, so the daemon cannot advertise a capability it will refuse.
#[tokio::test]
async fn the_published_list_is_the_enforced_list() {
    let host = DaemonHost::new(
        DaemonConfig::default(),
        eggsec::runtime_bridge::EggsecRuntimeExecutor::new(),
        noop_store(),
    );

    let published = published_names(&host).await;
    let enforced: Vec<String> = host
        .runtime()
        .enforced_task_kinds()
        .iter()
        .map(|capability| capability.name.clone())
        .collect();

    assert_eq!(
        published, enforced,
        "the published list and the enforced list must be the same object"
    );
    assert!(
        !published.is_empty(),
        "the daemon must publish the canonical list"
    );
}

/// The canonical list covers every capability name the task-kind set
/// defines — `resume` and the hazard families included.
#[tokio::test]
async fn the_canonical_capabilities_are_published() {
    let host = DaemonHost::new(
        DaemonConfig::default(),
        eggsec::runtime_bridge::EggsecRuntimeExecutor::new(),
        noop_store(),
    );
    let published = published_names(&host).await;

    let canonical = eggsec_runtime::RuntimeCapabilities::canonical();
    assert_eq!(published.len(), canonical.task_kinds.len());
    for kind in TaskKind::all_variants() {
        let name = kind.capability_name();
        assert!(
            published.iter().any(|published| published == name),
            "the daemon must publish {name}"
        );
    }
}

/// The hazard families are **admitted**.
///
/// The daemon's admission check reads capability names; the operation's
/// authorisation is the enforcement layer's business. A submission the
/// operator's policy refuses is refused *by the policy* — and the daemon
/// still admits the kind, so the refusal is never mistaken for an unknown
/// capability.
#[tokio::test]
async fn the_hazard_families_are_admitted_by_the_daemon() {
    let host = DaemonHost::new(
        DaemonConfig::default(),
        eggsec::runtime_bridge::EggsecRuntimeExecutor::new(),
        noop_store(),
    );
    let session_id = match host
        .handle_command(
            ClientCommand::CreateSession {
                request_id: "s1".into(),
                surface: RuntimeSurface::CliManual,
                scope: None,
                labels: vec![],
            },
            request_context(),
        )
        .await
    {
        ServerMessage::SessionCreated { session_id, .. } => session_id,
        other => panic!("expected SessionCreated, got {other:?}"),
    };

    let kinds = vec![
        TaskKind::Resume(ResumeParams {
            session_path: "/tmp/does-not-exist.eggsec".into(),
        }),
        TaskKind::StressTest(StressTestParams {
            target: "10.0.0.1".into(),
            flood_type: "syn".into(),
            duration_secs: None,
            threads: None,
            rate_pps: None,
        }),
        TaskKind::WirelessActive(WirelessActiveParams {
            interface: None,
            target_bssid: None,
            ..Default::default()
        }),
        TaskKind::DbPentest(DbPentestParams {
            db_type: "postgres".into(),
            target: "10.0.0.1".into(),
            port: None,
            ..Default::default()
        }),
        TaskKind::Intercept(InterceptParams {
            listen_port: None,
            target: None,
            ..Default::default()
        }),
    ];

    for kind in kinds {
        let name = kind.capability_name();
        let request = RunRequest {
            task_kind: kind,
            requested_by: None,
            surface: RuntimeSurface::CliManual,
            labels: vec![],
        };
        let response = host
            .handle_command(
                ClientCommand::SubmitTask {
                    request_id: format!("submit-{name}"),
                    session_id,
                    request,
                },
                request_context(),
            )
            .await;
        match response {
            ServerMessage::TaskSubmitted { .. } => {}
            ServerMessage::Error { message, .. } => {
                assert!(
                    !message.contains("unsupported task kind"),
                    "{name} must be admitted by the daemon, not refused as unknown: {message}"
                );
            }
            other => panic!("expected TaskSubmitted for {name}, got {other:?}"),
        }
    }
}

/// A rejecting executor publishes nothing and admits nothing.
///
/// The executor declares the empty set explicitly; the trait's default is
/// the canonical list, so "says nothing" would mean "declares everything"
/// and the daemon would advertise capabilities it cannot execute.
#[tokio::test]
async fn a_rejecting_executor_publishes_nothing() {
    struct RejectingExecutor;

    impl eggsec_runtime::RuntimeTaskExecutor for RejectingExecutor {
        fn supported_task_kinds(&self) -> Vec<eggsec_runtime::TaskCapability> {
            Vec::new()
        }

        fn execute(
            &self,
            _task_id: eggsec_runtime::TaskId,
            _request: eggsec_runtime::RunRequest,
            _context: eggsec_runtime::RuntimeExecutionContext,
            _sink: eggsec_runtime::RuntimeEventSink,
            _cancel: tokio_util::sync::CancellationToken,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<eggsec_runtime::TaskOutcome, eggsec_runtime::RuntimeError>,
                    > + Send
                    + 'static,
            >,
        > {
            Box::pin(async { Err(eggsec_runtime::RuntimeError::UnsupportedTaskKind) })
        }
    }

    let host = DaemonHost::new(DaemonConfig::default(), RejectingExecutor, noop_store());
    assert!(
        published_names(&host).await.is_empty(),
        "a rejecting executor must publish nothing"
    );
    assert!(
        host.runtime().enforced_task_kinds().is_empty(),
        "a rejecting executor must admit nothing"
    );
}
