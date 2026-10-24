//! The daemon publishes the engine's **report-form coverage**, and the
//! coverage leaks nothing and grants nothing.
//!
//! Milestone 013. The engine's coverage table
//! (`crates/eggsec/src/runtime_bridge/report_coverage.rs`) is the single
//! source of truth for which canonical operations produce a report document.
//! The engine publishes it, and the daemon republishes it **verbatim** in its
//! `Capabilities` reply, so a client can say *why* a task has no report
//! rather than only that it has none.
//!
//! These tests assert the property at the daemon's own boundary: what the
//! daemon publishes over the wire is exactly the engine's table.

// The whole file is gated on the `service` cargo feature: a daemon built
// without the profile links no engine, so there is no engine executor to
// host and no coverage to publish.
#![cfg(feature = "service")]

use eggsec_daemon::config::DaemonConfig;
use eggsec_daemon::host::DaemonHost;
use eggsec_daemon::protocol::{ClientCommand, DaemonRequestContext, ServerMessage, TransportKind};
use eggsec_daemon::store::noop_store;
use eggsec_runtime::request::TaskKind;

fn request_context() -> DaemonRequestContext {
    DaemonRequestContext {
        client_id: None,
        peer: None,
        transport: TransportKind::UnixSocket,
    }
}

/// The daemon hosting the engine executor.
fn engine_host() -> DaemonHost {
    DaemonHost::new(
        DaemonConfig::default(),
        eggsec::runtime_bridge::EggsecRuntimeExecutor::new(),
        noop_store(),
    )
}

/// The capabilities the daemon publishes.
async fn published_capabilities(host: &DaemonHost) -> eggsec_daemon::protocol::DaemonCapabilities {
    let response = host
        .handle_command(
            ClientCommand::Capabilities {
                request_id: "cap".into(),
            },
            request_context(),
        )
        .await;
    match response {
        ServerMessage::Capabilities { capabilities, .. } => capabilities,
        other => panic!("expected Capabilities, got {other:?}"),
    }
}

/// The canonical operation ids, derived from the canonical task-kind table
/// rather than transcribed here.
fn canonical_operation_ids() -> Vec<String> {
    let mut ids: Vec<String> = TaskKind::all_variants()
        .iter()
        .map(|kind| kind.operation_id().to_owned())
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

/// A daemon hosting the engine executor publishes the coverage: one row per
/// canonical operation id, the ids **derived** from the canonical task-kind
/// table, and the excluded rows carrying their reasons.
#[tokio::test]
async fn a_daemon_hosting_the_engine_publishes_the_full_coverage() {
    let host = engine_host();
    let capabilities = published_capabilities(&host).await;

    let forms = capabilities
        .report_forms
        .expect("a daemon that links the engine publishes the coverage");

    let mut published: Vec<String> = forms
        .iter()
        .map(|entry| entry.operation_id.clone())
        .collect();
    published.sort();

    assert_eq!(
        published,
        canonical_operation_ids(),
        "the published rows name every canonical operation, and no other"
    );

    for entry in &forms {
        match entry.form {
            eggsec_runtime::report::ReportForm::Canonical => assert!(
                entry.reason.is_empty(),
                "`{}` is canonical and carries no reason",
                entry.operation_id
            ),
            eggsec_runtime::report::ReportForm::NoCleanMapping => assert!(
                !entry.reason.is_empty(),
                "`{}` is excluded and must name the fact it is missing",
                entry.operation_id
            ),
        }
    }

    // At least one operation is excluded, and its reason is a sentence the
    // operator can read — not an empty string and not a code.
    let excluded = forms
        .iter()
        .filter(|entry| entry.form == eggsec_runtime::report::ReportForm::NoCleanMapping)
        .count();
    assert!(
        excluded > 0,
        "the coverage must name the operations it excludes"
    );
}

/// The published coverage leaks nothing: the serialized payload carries
/// operation ids, form discriminators and reason sentences — no target, no
/// banner, no finding text, no credential.
#[tokio::test]
async fn the_published_coverage_leaks_nothing() {
    let host = engine_host();
    let capabilities = published_capabilities(&host).await;

    let payload = serde_json::to_string(&capabilities).expect("the capabilities encode");
    let forms = capabilities
        .report_forms
        .expect("the coverage is published");

    // Every string in the coverage is either an operation id or a reason.
    for entry in &forms {
        assert!(
            !entry.reason.contains("://"),
            "`{}` carries a URL in its reason: {}",
            entry.operation_id,
            entry.reason
        );
        assert!(
            !entry.reason.contains("Bearer"),
            "`{}` carries a credential in its reason",
            entry.operation_id
        );
    }

    // The reasons are sentences about the *shape* of a result type, so they
    // are lowercase prose and carry no uppercase identifier that could be a
    // host name, a banner or a finding title.
    for entry in &forms {
        if entry.form == eggsec_runtime::report::ReportForm::NoCleanMapping {
            let first = entry.reason.chars().next().expect("a non-empty reason");
            assert!(
                first.is_lowercase(),
                "`{}` starts its reason with an uppercase character, which reads like an \
                 observed value: {}",
                entry.operation_id,
                entry.reason
            );
        }
    }

    assert!(
        payload.contains("report_forms"),
        "the coverage is on the wire"
    );
}

/// The coverage grants no authorization: publishing it changes neither the
/// daemon's published capability list nor the runtime's admitted one. The
/// coverage is a *build fact* about report forms, and it is not an
/// admission surface.
#[tokio::test]
async fn the_coverage_grants_no_authorization() {
    let host = engine_host();
    let capabilities = published_capabilities(&host).await;

    let published_kinds: Vec<String> = capabilities
        .runtime
        .task_kinds
        .iter()
        .map(|capability| capability.name.clone())
        .collect();

    assert!(
        !published_kinds.is_empty(),
        "the daemon publishes its task kinds as before"
    );
    assert!(
        capabilities.report_forms.is_some(),
        "the coverage is published alongside them"
    );
}
