//! Runtime conformance for the daemon's `service` profile.
//!
//! The static side of the contract — the manifest, the documentation table,
//! the engine's feature registry — is checked in
//! `crates/eggsec/tests/feature_matrix.rs`, where it runs on every push.
//! This file checks the **compiled** side: the engine feature set the daemon
//! binary was actually built with, the executor the profile is built around,
//! and the platform prerequisites the profile's domains need.
//!
//! The whole file is gated on the `service` cargo feature: a daemon built
//! without the profile links no engine, so there is no compiled feature set
//! to compare against.
//!
//! The profile is a **build-side** contract. Nothing here reads or writes the
//! enabled-feature set over the socket — the wire-side publication belongs to
//! the protocol's capability negotiation, not to this milestone.

#![cfg(feature = "service")]

use std::sync::{Arc, Mutex};

use eggsec_daemon::client_registry::ClientKind;
use eggsec_daemon::config::DaemonConfig;
use eggsec_daemon::host::DaemonHost;
use eggsec_daemon::protocol::{ClientCommand, DaemonRequestContext, ServerMessage, TransportKind};
use eggsec_daemon::store::{DaemonStore, PersistedAuditEvent};
use eggsec_runtime::{ClientId, SessionId, SessionSnapshot};

use eggsec::config::daemon_profile::{
    profile_violations, required_profile_features, DAEMON_SERVICE_EXCLUDED,
    DAEMON_SERVICE_FORBIDDEN, DAEMON_SERVICE_PROFILE,
};
use eggsec::platform::{all_domain_ids, report_for, skip_reason_for};

/// The engine features the daemon process was compiled with.
fn compiled_features() -> Vec<String> {
    eggsec::policy_bridge::features::current_enabled_features()
        .iter()
        .map(str::to_owned)
        .collect()
}

// ─── The compiled feature set ──────────────────────────────────────────────

/// The compiled engine feature set equals the documented profile exactly.
#[test]
fn the_compiled_engine_feature_set_equals_the_documented_profile() {
    let mut compiled = compiled_features();
    compiled.sort_unstable();

    let mut documented: Vec<String> = DAEMON_SERVICE_PROFILE
        .iter()
        .map(|feature| (*feature).to_owned())
        .collect();
    documented.sort_unstable();

    assert_eq!(
        compiled, documented,
        "the daemon's compiled engine feature set must equal DAEMON_SERVICE_PROFILE"
    );
}

/// The engine's feature lookup fails closed on an unknown name.
#[test]
fn the_compiled_feature_set_is_fail_closed_on_unknown_names() {
    let features = eggsec::policy_bridge::features::current_enabled_features();
    assert!(!features.contains("not-a-real-feature"));
    assert!(!features.contains("totally-fake-feature-xyz"));
}

/// The compiled feature set covers every required feature.
#[test]
fn the_compiled_feature_set_covers_the_required_features() {
    let features = eggsec::policy_bridge::features::current_enabled_features();
    for feature in required_profile_features() {
        assert!(
            features.contains(feature),
            "the compiled feature set omits required feature '{feature}'"
        );
    }
}

/// The compiled feature set excludes every forbidden feature.
#[test]
fn the_compiled_feature_set_excludes_the_forbidden_features() {
    let features = eggsec::policy_bridge::features::current_enabled_features();
    for feature in DAEMON_SERVICE_FORBIDDEN {
        assert!(
            !features.contains(feature),
            "the compiled feature set must not contain forbidden feature '{feature}'"
        );
    }
}

/// The engine's registry snapshot agrees with the compiled set.
///
/// This is the engine's own invariant (`snapshot_matches_registry_oracle`);
/// it is asserted here too because the profile build is the build in which
/// a registry/snapshot divergence would silently change what the daemon can
/// dispatch.
#[test]
fn the_engine_snapshot_agrees_with_the_registry() {
    let features = eggsec::policy_bridge::features::current_enabled_features();
    for entry in eggsec::config::ALL_FEATURES {
        assert_eq!(
            features.contains(entry.name),
            entry.enabled,
            "feature '{}' mismatch between the registry and the compiled snapshot",
            entry.name
        );
    }
}

// ─── The executor the profile is built around ──────────────────────────────

fn request_context(client_id: Option<ClientId>) -> DaemonRequestContext {
    DaemonRequestContext {
        client_id,
        peer: None,
        transport: TransportKind::UnixSocket,
    }
}

/// The daemon hosts the engine executor under the `service` profile.
#[tokio::test]
async fn the_daemon_hosts_the_engine_executor_under_the_service_profile() {
    let host = DaemonHost::new(
        DaemonConfig::default(),
        eggsec::runtime_bridge::EggsecRuntimeExecutor::new(),
        eggsec_daemon::store::noop_store(),
    );

    let response = host
        .handle_command(
            ClientCommand::Capabilities {
                request_id: "cap-service".into(),
            },
            request_context(None),
        )
        .await;

    match response {
        ServerMessage::Capabilities { capabilities, .. } => {
            assert!(
                !capabilities.runtime.task_kinds.is_empty(),
                "the real executor must report a non-empty task-kind set"
            );
        }
        other => panic!("expected Capabilities, got {other:?}"),
    }
}

// ─── Platform prerequisites ────────────────────────────────────────────────

/// Every profile domain has a platform prerequisite row.
///
/// The engine's own matrix is the source of truth: each domain names the
/// cargo feature(s) it needs, and each of those features is either in the
/// profile or is a documented exclusion.
#[test]
fn every_profile_domain_has_a_platform_prerequisite_row() {
    for domain in all_domain_ids() {
        let report = report_for(&domain);
        assert!(
            !report.features.is_empty(),
            "domain '{domain}' must name the cargo feature(s) it needs"
        );
        for feature in &report.features {
            let in_profile = DAEMON_SERVICE_PROFILE.contains(&feature.as_str());
            let documented_exclusion = DAEMON_SERVICE_EXCLUDED.contains(&feature.as_str());
            assert!(
                in_profile || documented_exclusion,
                "domain '{domain}' needs feature '{feature}', which is neither in the profile nor a documented exclusion"
            );
        }
    }
}

/// The profile document's prerequisite table has one row per domain id.
#[test]
fn the_profile_platform_rows_are_documented() {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let doc_path = workspace_root.join("docs/DAEMON_FEATURE_PROFILE.md");
    let doc = std::fs::read_to_string(&doc_path).unwrap_or_else(|e| {
        panic!(
            "docs/DAEMON_FEATURE_PROFILE.md must exist at {}: {e}",
            doc_path.display()
        )
    });

    let documented = parse_table_column(&doc, "## System prerequisites");
    for domain in all_domain_ids() {
        assert!(
            documented.iter().any(|row| row == &domain),
            "the profile's prerequisite table must have a row for domain '{domain}' (found {documented:?})"
        );
    }
}

/// Parse the first markdown table under `heading` and return its first column.
fn parse_table_column(doc: &str, heading: &str) -> Vec<String> {
    let mut lines = doc.lines();
    let mut in_section = false;
    let mut column: Vec<String> = Vec::new();
    let mut seen_table = false;

    for line in lines.by_ref() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            if in_section {
                break;
            }
            if trimmed == heading {
                in_section = true;
            }
            continue;
        }
        if !in_section {
            continue;
        }
        if trimmed.starts_with('|') {
            let cells: Vec<&str> = trimmed.trim_matches('|').split('|').collect();
            let first = cells.first().copied().unwrap_or("").trim();
            if first.is_empty() {
                continue;
            }
            if first.starts_with('-') || first.starts_with("Feature") || first.starts_with("Domain")
            {
                seen_table = true;
                continue;
            }
            column.push(first.trim_matches('`').trim().to_string());
        } else if seen_table && !trimmed.is_empty() {
            break;
        }
    }

    column
}

/// The live-capability probe reports the engine's own skip reasons.
///
/// The assertion is on the **shape** of the engine's wording, not on the
/// host's outcome, so the test is deterministic on any host.
#[test]
fn the_live_capability_probe_reports_engine_skip_reasons() {
    for domain in all_domain_ids() {
        if let Some(reason) = skip_reason_for(&domain) {
            assert!(
                reason.starts_with(&format!("SKIP {domain}:")),
                "the engine's skip reason for '{domain}' must use the engine's own wording, got '{reason}'"
            );
        }
    }
}

/// The profile document must not claim live capability for a domain that
/// needs a privilege the operator has not been told about.
#[test]
fn the_profile_does_not_claim_live_capability_for_unmet_prerequisites() {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let doc_path = workspace_root.join("docs/DAEMON_FEATURE_PROFILE.md");
    let doc = std::fs::read_to_string(&doc_path).unwrap_or_else(|e| {
        panic!(
            "docs/DAEMON_FEATURE_PROFILE.md must exist at {}: {e}",
            doc_path.display()
        )
    });

    for (domain, requirement) in [
        ("packet-inspection", "CAP_NET_RAW"),
        ("stress-testing", "CAP_NET_RAW"),
        ("wireless-advanced", "CAP_NET_ADMIN"),
    ] {
        assert!(
            doc.contains(domain),
            "the profile document must mention the '{domain}' domain"
        );
        assert!(
            doc.contains(requirement),
            "the profile document must name '{requirement}' for '{domain}'"
        );
    }
    assert!(
        doc.contains("Linux"),
        "the profile document must state the Linux-only constraint for the privileged domains"
    );
}

// ─── Security and negative checks ──────────────────────────────────────────

/// A profile that enables a forbidden feature fails the conformance check.
#[test]
fn a_profile_that_enables_a_forbidden_feature_fails_the_conformance_check() {
    let mut candidate: Vec<&str> = DAEMON_SERVICE_PROFILE.to_vec();
    candidate.push("insecure-tls");
    let violations = profile_violations(&candidate);
    assert!(
        violations.iter().any(|violation| {
            violation.to_string().contains("insecure-tls")
                && violation.to_string().contains("forbidden")
        }),
        "expected a forbidden-feature violation naming 'insecure-tls', got {violations:?}"
    );
}

/// A profile that enables a serving surface fails the conformance check.
#[test]
fn a_profile_that_enables_a_serving_surface_fails_the_conformance_check() {
    let mut candidate: Vec<&str> = DAEMON_SERVICE_PROFILE.to_vec();
    candidate.push("rest-api");
    let violations = profile_violations(&candidate);
    assert!(
        violations
            .iter()
            .any(|violation| violation.to_string().contains("rest-api")),
        "expected a violation naming 'rest-api', got {violations:?}"
    );
}

/// A profile that enables an exposure marker fails the conformance check.
#[test]
fn a_profile_that_enables_an_exposure_marker_fails_the_conformance_check() {
    let mut candidate: Vec<&str> = DAEMON_SERVICE_PROFILE.to_vec();
    candidate.push("db-pentest-mcp");
    let violations = profile_violations(&candidate);
    assert!(
        violations
            .iter()
            .any(|violation| violation.to_string().contains("db-pentest-mcp")),
        "expected a violation naming 'db-pentest-mcp', got {violations:?}"
    );
}

/// A profile that enables the aggregate fails the conformance check.
#[test]
fn a_profile_that_enables_the_aggregate_fails_the_conformance_check() {
    let mut candidate: Vec<&str> = DAEMON_SERVICE_PROFILE.to_vec();
    candidate.push("full");
    let violations = profile_violations(&candidate);
    assert!(
        violations
            .iter()
            .any(|violation| violation.to_string().contains("full")),
        "expected a violation naming 'full', got {violations:?}"
    );
}

/// A profile that omits a required feature fails the conformance check.
#[test]
fn a_profile_missing_a_required_feature_fails_the_conformance_check() {
    let candidate: Vec<&str> = DAEMON_SERVICE_PROFILE
        .iter()
        .copied()
        .filter(|feature| *feature != "nse")
        .collect();
    let violations = profile_violations(&candidate);
    assert!(
        violations.iter().any(|violation| {
            violation.to_string().contains("nse") && violation.to_string().contains("missing")
        }),
        "expected a missing-required-feature violation naming 'nse', got {violations:?}"
    );
}

/// A profile that names an unknown feature fails the conformance check.
#[test]
fn a_profile_with_an_unknown_feature_fails_the_conformance_check() {
    let mut candidate: Vec<&str> = DAEMON_SERVICE_PROFILE.to_vec();
    candidate.push("totally-fake-feature-xyz");
    let violations = profile_violations(&candidate);
    assert!(
        violations.iter().any(|violation| {
            violation.to_string().contains("totally-fake-feature-xyz")
                && violation.to_string().contains("unknown")
        }),
        "expected an unknown-feature violation, got {violations:?}"
    );
}

/// The daemon's compiled feature set never contains `insecure-tls`.
#[test]
fn the_daemon_profile_does_not_enable_insecure_tls() {
    let features = eggsec::policy_bridge::features::current_enabled_features();
    assert!(
        !features.contains("insecure-tls"),
        "the daemon must never be built with insecure-tls"
    );
}

// ─── Build-time property, not a runtime one ────────────────────────────────

/// A store that records everything the daemon persists.
#[derive(Default)]
struct RecordingStore {
    audit_events: Mutex<Vec<PersistedAuditEvent>>,
    snapshots: Mutex<Vec<SessionSnapshot>>,
}

#[async_trait::async_trait]
impl DaemonStore for RecordingStore {
    async fn save_session_snapshot(&self, snapshot: &SessionSnapshot) -> anyhow::Result<()> {
        self.snapshots
            .lock()
            .map_err(|e| anyhow::anyhow!("recording store poisoned: {e}"))?
            .push(snapshot.clone());
        Ok(())
    }

    async fn load_session_snapshot(
        &self,
        _session_id: SessionId,
    ) -> anyhow::Result<Option<SessionSnapshot>> {
        Ok(None)
    }

    async fn load_all_sessions(&self) -> anyhow::Result<Vec<SessionSnapshot>> {
        Ok(Vec::new())
    }

    async fn record_audit_event(&self, event: &PersistedAuditEvent) -> anyhow::Result<()> {
        self.audit_events
            .lock()
            .map_err(|e| anyhow::anyhow!("recording store poisoned: {e}"))?
            .push(event.clone());
        Ok(())
    }

    async fn delete_session(&self, _session_id: SessionId) -> anyhow::Result<()> {
        Ok(())
    }

    fn blocking_list_sessions(&self) -> anyhow::Result<Vec<eggsec_runtime::SessionSummary>> {
        Ok(Vec::new())
    }

    fn blocking_get_snapshot(
        &self,
        _session_id: &SessionId,
    ) -> anyhow::Result<Option<SessionSnapshot>> {
        Ok(None)
    }
}

/// Whether `text` names `feature` as a standalone token.
///
/// The daemon's audit vocabulary legitimately contains a feature name inside
/// a longer word — "declare-client" contains "cli" — so a bare substring
/// test would report a leak that is not there. The name must be delimited by
/// non-alphanumeric characters on both sides.
fn names_feature(text: &str, feature: &str) -> bool {
    let bytes = text.as_bytes();
    let needle = feature.as_bytes();
    if needle.is_empty() || bytes.len() < needle.len() {
        return false;
    }
    for start in 0..=(bytes.len() - needle.len()) {
        if &bytes[start..start + needle.len()] != needle {
            continue;
        }
        let end = start + needle.len();
        let delimited_before = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let delimited_after = end == bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if delimited_before && delimited_after {
            return true;
        }
    }
    false
}

/// The enabled-feature set is never persisted.
///
/// The profile is a build-time property: the daemon does not record which
/// features it was compiled with, negotiate them, or report them. This is
/// what keeps the profile a build recipe rather than a protocol change.
#[tokio::test]
async fn the_profile_is_a_build_time_property_not_a_runtime_one() {
    let store = Arc::new(RecordingStore::default());
    let host = DaemonHost::new(
        DaemonConfig::default(),
        eggsec::runtime_bridge::EggsecRuntimeExecutor::new(),
        store.clone(),
    );

    let response = host
        .handle_command(
            ClientCommand::DeclareClient {
                request_id: "declare-profile".into(),
                kind: ClientKind::Tui,
                label: Some("profile-probe".into()),
            },
            request_context(None),
        )
        .await;
    let client_id = match response {
        ServerMessage::ClientDeclared { client_id, .. } => client_id,
        other => panic!("expected ClientDeclared, got {other:?}"),
    };

    let response = host
        .handle_command(
            ClientCommand::CreateSession {
                request_id: "session-profile".into(),
                surface: eggsec_runtime::RuntimeSurface::CliManual,
                scope: None,
                labels: vec![],
            },
            request_context(Some(client_id)),
        )
        .await;
    let session_id = match response {
        ServerMessage::SessionCreated { session_id, .. } => session_id,
        other => panic!("expected SessionCreated, got {other:?}"),
    };

    let response = host
        .handle_command(
            ClientCommand::SubmitTask {
                request_id: "task-profile".into(),
                session_id,
                request: eggsec_runtime::RunRequest {
                    task_kind: eggsec_runtime::TaskKind::Fingerprint(
                        eggsec_runtime::request::FingerprintParams {
                            target: "127.0.0.1".into(),
                            ports: Some("80".into()),
                            timeout_secs: None,
                            concurrency: None,
                        },
                    ),
                    requested_by: None,
                    surface: eggsec_runtime::RuntimeSurface::CliManual,
                    labels: vec![],
                },
            },
            request_context(Some(client_id)),
        )
        .await;
    assert!(
        !matches!(response, ServerMessage::Error { .. }),
        "the probe task must be accepted, got {response:?}"
    );

    let audit = store.audit_events.lock().expect("recording store poisoned");
    let snapshots = store.snapshots.lock().expect("recording store poisoned");
    for feature in DAEMON_SERVICE_PROFILE {
        assert!(
            !audit
                .iter()
                .any(|event| names_feature(&event.action, feature)),
            "the daemon persisted a record naming the enabled feature '{feature}'"
        );
    }
    for snapshot in snapshots.iter() {
        let rendered = format!("{snapshot:?}");
        for feature in DAEMON_SERVICE_PROFILE {
            assert!(
                !names_feature(&rendered, feature),
                "the daemon persisted a session snapshot naming the enabled feature '{feature}'"
            );
        }
    }
}

// ─── Migration and compatibility ───────────────────────────────────────────

/// The daemon's runtime `--full-executor` flag is unchanged.
///
/// The flag's *name* and *meaning* are unchanged: it selects the engine
/// executor at runtime. It is a different mechanism from the cargo feature
/// of the same name, which is now a deprecated alias for `service`.
#[test]
fn the_daemon_runtime_flag_is_unchanged() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_eggsec-daemon"))
        .arg("--help")
        .output()
        .expect("failed to run the daemon binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--full-executor"),
        "the daemon's --help must still advertise --full-executor, got: {stdout}"
    );
}

/// The daemon protocol version is unchanged.
///
/// The profile is a build-side contract: it adds no protocol field, changes
/// no capability, and does not bump the version.
#[test]
fn the_protocol_version_is_unchanged() {
    assert_eq!(eggsec_daemon_protocol::protocol::DAEMON_PROTOCOL_VERSION, 2);
}
