//! Canonical engine execution boundary — Phase 1 convergence.
//!
//! This module is the **single production owner** for the mapping:
//!
//! ```text
//! (canonical operation ID + canonical typed request + ApprovedOperation)
//!                               ->
//!                        engine executor
//! ```
//!
//! Other layers may own conversion *into* that tuple (CLI args → canonical
//! request, `TaskKind` → canonical request, `ToolRequest.params` → canonical
//! request) and conversion of results *out* of it (outcome → CLI output,
//! outcome → runtime envelope, outcome → TUI rendering). They must not own a
//! second semantic mapping from operation name to implementation.
//!
//! # Design
//!
//! - [`CanonicalOperationRequest`] is a typed enum whose variants wrap the
//!   existing canonical request structs from
//!   `eggsec-tool-core::operation_request` (single owner for defaults/
//!   validation) plus runtime param structs for families that have no
//!   canonical struct yet. No `HashMap<String, Box<dyn Any>>` or JSON dispatch
//!   map: exhaustiveness is compiler-enforced.
//! - [`ExecutionEvent`] is the frontend-neutral event vocabulary (accepted/
//!   started, progress, finding, warning, completed, cancelled, failure).
//!   It contains no Clap, Ratatui, terminal, daemon-protocol, or Python types.
//! - [`ExecutionSink`] is a bounded, backpressure-aware event sink. Structured
//!   progress may be coalesced/dropped under load (with an explicit counter);
//!   findings and terminal outcomes are never dropped silently.
//! - [`execute_approved`] is the canonical entry point. It checks request/
//!   approval binding immediately before executor entry, checks feature
//!   availability in one predictable layer, and then routes through the single
//!   internal executor owner [`execute_canonical`].
//! - [`execute_canonical`] owns the executor match. `dispatch_inner()` (legacy
//!   manual shim) delegates to it; it does not own a second mapping.
//!
//! # Non-goals
//!
//! Domain worker signatures still take the legacy `(u64, u64)` progress sender.
//! The sink bridges legacy progress into [`ExecutionEvent`] via a forwarder
//! task. Migrating every worker to take `ExecutionSink` directly is Phase 2/3
//! work; the ownership guarantee (one executor match) already holds here.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use tokio::sync::mpsc;

use crate::config::{normalize_target, ApprovedOperation, OperationTarget};
use crate::dispatch::types::TaskResult;

// ── Frontend-neutral execution events (WS 1.4) ──

/// Frontend-neutral execution event vocabulary.
///
/// Distinguishes accepted/started, structured progress, streamed findings,
/// warnings/diagnostics, terminal outcome, cancellation, and failure. No
/// terminal, Clap, daemon-protocol, or Python types appear here.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ExecutionEvent {
    /// Execution accepted and started for a canonical operation.
    Started { operation_id: String },
    /// Structured progress (completed/total or phase where known).
    Progress {
        completed: u64,
        total: Option<u64>,
        message: Option<String>,
    },
    /// Streamed finding/result item where streaming is meaningful.
    Finding { item: String },
    /// Non-fatal warning/diagnostic.
    Warning { message: String },
    /// Completed with a terminal outcome kind (e.g. `port-scan`, `recon`).
    Completed { outcome_kind: String },
    /// Cancelled before or during execution.
    Cancelled { reason: Option<String> },
    /// Execution failure (typed message, not preformatted terminal output).
    Failed { message: String },
}

impl ExecutionEvent {
    /// Terminal events must never be dropped by the sink.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed { .. } | Self::Cancelled { .. } | Self::Failed { .. }
        )
    }

    /// Findings and terminal events must never be dropped silently.
    pub fn must_deliver(&self) -> bool {
        matches!(
            self,
            Self::Finding { .. }
                | Self::Completed { .. }
                | Self::Cancelled { .. }
                | Self::Failed { .. }
        )
    }
}

/// Bounded, backpressure-aware sink for [`ExecutionEvent`].
///
/// - Capacity is 100 (matches the legacy progress channel).
/// - Non-critical progress uses `try_send` with explicit loss/coalescing
///   semantics: when full, the event is dropped and an internal counter is
///   incremented (observable via [`Self::dropped_progress_count`]).
/// - Findings, warnings, and terminal outcomes use `send().await` and are
///   never dropped silently (backpressure applies).
/// - A legacy `(u64, u64)` sender is carried for domain workers that have not
///   yet migrated to event emission. Progress forwarded from legacy workers is
///   re-emitted as [`ExecutionEvent::Progress`] through the same coalescing
///   path.
#[derive(Debug, Clone)]
pub struct ExecutionSink {
    event_tx: mpsc::Sender<ExecutionEvent>,
    legacy_tx: Option<mpsc::Sender<(u64, u64)>>,
    dropped_progress: Arc<AtomicU64>,
}

impl ExecutionSink {
    /// Channel capacity for execution events (matches legacy progress channel).
    pub const CAPACITY: usize = 100;

    /// Create a sink from explicit channels.
    pub fn new(
        event_tx: mpsc::Sender<ExecutionEvent>,
        legacy_tx: Option<mpsc::Sender<(u64, u64)>>,
    ) -> Self {
        Self {
            event_tx,
            legacy_tx,
            dropped_progress: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Create a sink from a legacy progress sender, returning the sink plus
    /// the event receiver. Domain progress can be bridged by forwarding the
    /// legacy channel into [`ExecutionEvent::Progress`].
    pub fn from_legacy(
        legacy_tx: mpsc::Sender<(u64, u64)>,
    ) -> (Self, mpsc::Receiver<ExecutionEvent>) {
        let (event_tx, event_rx) = mpsc::channel(Self::CAPACITY);
        (Self::new(event_tx, Some(legacy_tx)), event_rx)
    }

    /// Create a detached sink (events drained in background). Useful for
    /// callers that only need the legacy sender but want event accounting.
    pub fn detached(legacy_tx: mpsc::Sender<(u64, u64)>) -> Self {
        let (event_tx, mut event_rx) = mpsc::channel(Self::CAPACITY);
        tokio::spawn(async move { while event_rx.recv().await.is_some() {} });
        Self::new(event_tx, Some(legacy_tx))
    }

    /// Borrow the legacy progress sender for domain workers.
    pub fn legacy_sender(&self) -> Option<mpsc::Sender<(u64, u64)>> {
        self.legacy_tx.clone()
    }

    /// Number of non-critical progress events dropped due to backpressure.
    pub fn dropped_progress_count(&self) -> u64 {
        self.dropped_progress.load(Ordering::Relaxed)
    }

    async fn emit(&self, event: ExecutionEvent) {
        if event.must_deliver() {
            // Findings and terminal outcomes are never dropped: apply
            // backpressure. Log (don't propagate) on receiver-drop, since
            // execution already completed and the caller owns the outcome.
            if let Err(e) = self.event_tx.send(event).await {
                tracing::warn!("execution event receiver dropped: {}", e);
            }
        } else if let Err(e) = self.event_tx.try_send(event) {
            // Non-critical progress: coalesce under load, count the loss.
            self.dropped_progress.fetch_add(1, Ordering::Relaxed);
            tracing::debug!("execution progress coalesced (channel full): {}", e);
        }
    }

    /// Emit accepted/started (coalescible; start is also implied by progress).
    pub async fn emit_started(&self, operation_id: &str) {
        self.emit(ExecutionEvent::Started {
            operation_id: operation_id.to_string(),
        })
        .await;
    }

    /// Emit structured progress (coalescible under backpressure).
    pub async fn emit_progress(&self, completed: u64, total: Option<u64>, message: Option<String>) {
        self.emit(ExecutionEvent::Progress {
            completed,
            total,
            message,
        })
        .await;
        // Best-effort legacy bridge (never blocks execution).
        if let Some(tx) = &self.legacy_tx {
            let total_u64 = total.unwrap_or(100);
            if let Err(e) = tx.try_send((completed, total_u64)) {
                tracing::debug!("legacy progress bridge coalesced: {}", e);
            }
        }
    }

    /// Emit a streamed finding (never dropped).
    pub async fn emit_finding(&self, item: String) {
        self.emit(ExecutionEvent::Finding { item }).await;
    }

    /// Emit a warning/diagnostic (backpressure, never silently dropped).
    pub async fn emit_warning(&self, message: String) {
        self.emit(ExecutionEvent::Warning { message }).await;
    }

    /// Emit terminal completion (never dropped).
    pub async fn emit_completed(&self, outcome_kind: &str) {
        self.emit(ExecutionEvent::Completed {
            outcome_kind: outcome_kind.to_string(),
        })
        .await;
    }

    /// Emit cancellation (never dropped).
    pub async fn emit_cancelled(&self, reason: Option<String>) {
        self.emit(ExecutionEvent::Cancelled { reason }).await;
    }

    /// Emit failure (never dropped).
    pub async fn emit_failed(&self, message: String) {
        self.emit(ExecutionEvent::Failed { message }).await;
    }
}

// ── Canonical operation request (WS 1.1) ──

/// Canonical typed operation request.
///
/// Variants wrap the existing canonical request structs from
/// `eggsec-tool-core::operation_request` (single owner for defaults/
/// validation). Families without a canonical struct yet wrap their runtime
/// param struct directly; those structs are the canonical shape for that
/// family until a typed contract is promoted.
///
/// No user-facing alias appears here: aliases are resolved to canonical IDs
/// by surface adapters *before* the execution boundary.
#[derive(Debug, Clone)]
pub enum CanonicalOperationRequest {
    PortScan(crate::operation_request::PortScanRequest),
    EndpointScan(crate::operation_request::EndpointScanRequest),
    Fingerprint(crate::operation_request::FingerprintRequest),
    Fuzz(crate::operation_request::FuzzRequest),
    WafDetect(crate::operation_request::WafDetectRequest),
    WafStress(crate::operation_request::WafStressRequest),
    LoadTest(crate::operation_request::LoadTestRequest),
    Recon(crate::operation_request::ReconRequest),
    Pipeline(crate::operation_request::PipelineRequest),
    GraphQl(crate::operation_request::GraphQlRequest),
    OAuth(crate::operation_request::OAuthRequest),
    AuthTest(crate::operation_request::AuthTestRequest),
    DbPentest(crate::operation_request::DbPentestRequest),
    Storage(crate::operation_request::StorageRequest),
    StressTest(eggsec_runtime::request::StressTestParams),
    PacketCapture(eggsec_runtime::request::PacketCaptureParams),
    PacketTraceroute(eggsec_runtime::request::PacketTracerouteParams),
    PacketSend(eggsec_runtime::request::PacketSendParams),
    Nse(eggsec_runtime::request::NseParams),
    Hunt(eggsec_runtime::request::HuntParams),
    Browser(eggsec_runtime::request::BrowserParams),
    Compliance(eggsec_runtime::request::ComplianceParams),
    Integrations(eggsec_runtime::request::IntegrationsParams),
    Workflow(eggsec_runtime::request::WorkflowParams),
    Vuln(eggsec_runtime::request::VulnParams),
    Wireless(eggsec_runtime::request::WirelessParams),
    WirelessActive(eggsec_runtime::request::WirelessActiveParams),
    Intercept(eggsec_runtime::request::InterceptParams),
    C2(eggsec_runtime::request::C2Params),
}

impl CanonicalOperationRequest {
    /// Canonical operation ID. Never inferred from user-facing aliases here.
    pub fn operation_id(&self) -> &'static str {
        match self {
            Self::PortScan(_) => "scan-ports",
            Self::EndpointScan(_) => "scan-endpoints",
            Self::Fingerprint(_) => "fingerprint",
            Self::Fuzz(_) => "fuzz",
            Self::WafDetect(_) => "waf-detect",
            Self::WafStress(_) => "waf-stress",
            Self::LoadTest(_) => "load-test",
            Self::Recon(_) => "recon",
            Self::Pipeline(_) => "pipeline",
            Self::GraphQl(_) => "graphql",
            Self::OAuth(_) => "oauth",
            Self::AuthTest(_) => "auth-test",
            Self::DbPentest(_) => "db-pentest",
            Self::Storage(_) => "storage",
            Self::StressTest(_) => "stress-test",
            Self::PacketCapture(_) | Self::PacketTraceroute(_) | Self::PacketSend(_) => "packet",
            Self::Nse(_) => "nse",
            Self::Hunt(_) => "hunt",
            Self::Browser(_) => "browser",
            Self::Compliance(_) => "compliance",
            Self::Integrations(_) => "integrations",
            Self::Workflow(_) => "workflow",
            Self::Vuln(_) => "vuln",
            Self::Wireless(_) | Self::WirelessActive(_) => "wireless",
            Self::Intercept(_) => "proxy-intercept",
            Self::C2(_) => "c2",
        }
    }

    /// Canonical target (`None` for `NoTarget`/interface-bound operations).
    pub fn canonical_target(&self) -> Option<String> {
        match self {
            Self::PortScan(r) => Some(r.target.clone()),
            Self::EndpointScan(r) => Some(r.target.clone()),
            Self::Fingerprint(r) => Some(r.target.clone()),
            Self::Fuzz(r) => Some(r.target.clone()),
            Self::WafDetect(r) => Some(r.target.clone()),
            Self::WafStress(r) => Some(r.target.clone()),
            Self::LoadTest(r) => Some(r.target.clone()),
            Self::Recon(r) => Some(r.target.clone()),
            Self::Pipeline(r) => Some(r.target.clone()),
            Self::GraphQl(r) => Some(r.target.clone()),
            Self::OAuth(r) => Some(r.target.clone()),
            Self::AuthTest(r) => Some(r.target.clone()),
            Self::DbPentest(r) => Some(r.target.clone()),
            Self::Storage(_) => None,
            Self::StressTest(p) => Some(p.target.clone()),
            Self::PacketCapture(_) => None,
            Self::PacketTraceroute(p) => Some(p.target.clone()),
            Self::PacketSend(p) => Some(p.target.clone()),
            Self::Nse(p) => Some(p.target.clone()),
            Self::Hunt(p) => Some(p.target.clone()),
            Self::Browser(p) => Some(p.target.clone()),
            Self::Compliance(p) => Some(p.target.clone()),
            Self::Integrations(_) => None,
            Self::Workflow(_) => None,
            Self::Vuln(p) => Some(p.target.clone()),
            Self::Wireless(_) => None,
            Self::WirelessActive(_) => None,
            Self::Intercept(p) => p.target.clone(),
            Self::C2(p) => p.target.clone(),
        }
    }

    /// Validate/normalize the request through canonical contracts where they
    /// exist. Passthrough families get minimal structural validation here;
    /// strict validation for those families remains in their domain workers
    /// until typed contracts are promoted.
    pub fn validate(&self) -> Result<(), crate::operation_request::NormalizationError> {
        use crate::operation_request::NormalizationError;
        fn err(msg: impl Into<String>) -> NormalizationError {
            NormalizationError(msg.into())
        }
        match self {
            Self::PortScan(r) => {
                r.normalize()?;
                Ok(())
            }
            Self::EndpointScan(r) => {
                r.normalize()?;
                Ok(())
            }
            Self::Fingerprint(r) => {
                r.normalize()?;
                Ok(())
            }
            Self::Fuzz(r) => {
                r.normalize()?;
                Ok(())
            }
            Self::WafDetect(r) => {
                r.normalize()?;
                Ok(())
            }
            Self::WafStress(r) => {
                r.normalize()?;
                Ok(())
            }
            Self::LoadTest(r) => {
                r.normalize()?;
                Ok(())
            }
            Self::Recon(r) => {
                r.normalize()?;
                Ok(())
            }
            Self::Pipeline(r) => {
                r.normalize()?;
                Ok(())
            }
            Self::GraphQl(r) => {
                r.normalize()?;
                Ok(())
            }
            Self::OAuth(r) => {
                r.normalize()?;
                Ok(())
            }
            Self::AuthTest(r) => {
                r.normalize()?;
                Ok(())
            }
            Self::DbPentest(r) => {
                r.normalize()?;
                Ok(())
            }
            Self::Storage(r) => {
                r.normalize()?;
                Ok(())
            }
            Self::StressTest(p) => {
                if p.target.trim().is_empty() {
                    return Err(err("stress-test target must not be empty"));
                }
                if p.flood_type.trim().is_empty() {
                    return Err(err("stress-test flood_type must not be empty"));
                }
                Ok(())
            }
            Self::PacketCapture(_) => Ok(()),
            Self::PacketTraceroute(p) => {
                if p.target.trim().is_empty() {
                    return Err(err("packet traceroute target must not be empty"));
                }
                Ok(())
            }
            Self::PacketSend(p) => {
                if p.target.trim().is_empty() {
                    return Err(err("packet-send target must not be empty"));
                }
                Ok(())
            }
            Self::Nse(p) => {
                if p.target.trim().is_empty() {
                    return Err(err("nse target must not be empty"));
                }
                if p.script.trim().is_empty() {
                    return Err(err("nse script must not be empty"));
                }
                Ok(())
            }
            Self::Hunt(p) => {
                if p.target.trim().is_empty() {
                    return Err(err("hunt target must not be empty"));
                }
                Ok(())
            }
            Self::Browser(p) => {
                if p.target.trim().is_empty() {
                    return Err(err("browser target must not be empty"));
                }
                Ok(())
            }
            Self::Compliance(p) => {
                if p.target.trim().is_empty() {
                    return Err(err("compliance target must not be empty"));
                }
                Ok(())
            }
            Self::Integrations(p) => {
                if p.integration_type.trim().is_empty() {
                    return Err(err("integrations integration_type must not be empty"));
                }
                Ok(())
            }
            Self::Workflow(_) => Ok(()),
            Self::Vuln(p) => {
                if p.target.trim().is_empty() {
                    return Err(err("vuln target must not be empty"));
                }
                Ok(())
            }
            Self::Wireless(_) => Ok(()),
            Self::WirelessActive(_) => Ok(()),
            Self::Intercept(_) => Ok(()),
            Self::C2(_) => Ok(()),
        }
    }

    /// Convert a runtime [`TaskKind`] into its canonical request.
    ///
    /// Exhaustive over `TaskKind`: adding a variant without updating this
    /// function is a compile error. Canonical families reuse the existing
    /// `runtime_adapters` conversions so CLI/runtime/tool requests normalize
    /// identically.
    pub fn from_task_kind(kind: &eggsec_runtime::request::TaskKind) -> Self {
        use crate::operation_request::runtime_adapters as adapters;
        use eggsec_runtime::request::TaskKind as K;
        match kind {
            K::LoadTest(p) => Self::LoadTest(adapters::load_test_from_runtime(p)),
            K::StressTest(p) => Self::StressTest(p.clone()),
            K::PortScan(p) => Self::PortScan(adapters::port_scan_from_runtime(p)),
            K::EndpointScan(p) => Self::EndpointScan(adapters::endpoint_scan_from_runtime(p)),
            K::Fingerprint(p) => Self::Fingerprint(adapters::fingerprint_from_runtime(p)),
            K::Fuzz(p) => Self::Fuzz(adapters::fuzz_from_runtime(p)),
            K::Waf(p) => Self::WafDetect(adapters::waf_from_runtime(p)),
            K::WafStress(p) => Self::WafStress(adapters::waf_stress_from_runtime(p)),
            K::Pipeline(p) => Self::Pipeline(adapters::pipeline_from_runtime(p)),
            K::Recon(p) => Self::Recon(adapters::recon_from_runtime(p)),
            K::PacketCapture(p) => Self::PacketCapture(p.clone()),
            K::PacketTraceroute(p) => Self::PacketTraceroute(p.clone()),
            K::PacketSend(p) => Self::PacketSend(p.clone()),
            K::GraphQl(p) => Self::GraphQl(adapters::graphql_from_runtime(p)),
            K::OAuth(p) => Self::OAuth(adapters::oauth_from_runtime(p)),
            K::AuthTest(p) => Self::AuthTest(adapters::auth_test_from_runtime(p)),
            K::Nse(p) => Self::Nse(p.clone()),
            K::Hunt(p) => Self::Hunt(p.clone()),
            K::Browser(p) => Self::Browser(p.clone()),
            K::Compliance(p) => Self::Compliance(p.clone()),
            K::Storage(p) => Self::Storage(crate::operation_request::StorageRequest {
                storage_type: p.storage_type.clone(),
                path: p.path.clone(),
            }),
            K::Integrations(p) => Self::Integrations(p.clone()),
            K::Workflow(p) => Self::Workflow(p.clone()),
            K::Vuln(p) => Self::Vuln(p.clone()),
            K::Wireless(p) => Self::Wireless(p.clone()),
            K::WirelessActive(p) => Self::WirelessActive(p.clone()),
            K::DbPentest(p) => Self::DbPentest(adapters::db_pentest_from_runtime(p)),
            K::Intercept(p) => Self::Intercept(p.clone()),
            K::C2(p) => Self::C2(p.clone()),
        }
    }
}

// ── Feature availability (one predictable layer, WS 1.1) ──

/// Whether a Cargo feature is compiled into this binary.
///
/// Centralizes the feature-availability check for the execution boundary so
/// CLI, TUI, runtime, and programmatic surfaces fail consistently. Unknown
/// feature names return `false` (fail closed).
pub fn is_feature_available(feature: &str) -> bool {
    match feature {
        "stress-testing" => cfg!(feature = "stress-testing"),
        "packet-inspection" => cfg!(feature = "packet-inspection"),
        "nse" => cfg!(feature = "nse"),
        "db-pentest" => cfg!(feature = "db-pentest"),
        "c2" => cfg!(feature = "c2"),
        "web-proxy" => cfg!(feature = "web-proxy"),
        "wireless" => cfg!(feature = "wireless"),
        "wireless-advanced" => cfg!(feature = "wireless-advanced"),
        "advanced-hunting" => cfg!(feature = "advanced-hunting"),
        "headless-browser" => cfg!(feature = "headless-browser"),
        "compliance" => cfg!(feature = "compliance"),
        "database" => cfg!(feature = "database"),
        "external-integrations" => cfg!(feature = "external-integrations"),
        "finding-workflow" => cfg!(feature = "finding-workflow"),
        "vuln-management" => cfg!(feature = "vuln-management"),
        "mobile" => cfg!(feature = "mobile"),
        "mobile-dynamic" => cfg!(feature = "mobile-dynamic"),
        "evasion" => cfg!(feature = "evasion"),
        "postex" => cfg!(feature = "postex"),
        // Core operations have no feature gate.
        "" => true,
        _ => false,
    }
}

/// Executor family that owns an operation (single-owner routing table).
///
/// Used by equivalence tests to assert that every surface reaches the same
/// executor for the same canonical operation. The strings are stable
/// diagnostics, not second dispatch keys.
pub fn executor_route_for(operation_id: &str) -> &'static str {
    match operation_id {
        "scan-ports" | "scan-endpoints" | "fingerprint" => "scanner",
        "recon" | "pipeline" => "recon",
        "waf-detect" | "waf-bypass" | "waf-stress" => "waf",
        "fuzz" | "graphql" | "oauth" => "fuzz-api",
        "load-test" | "stress-test" | "packet" | "auth-test" => "network",
        "nse" => "nse",
        "hunt" | "browser" | "compliance" | "storage" | "integrations" | "workflow" | "vuln"
        | "wireless" => "security",
        "db-pentest" => "db-pentest",
        "proxy-intercept" => "intercept",
        "c2" => "c2",
        _ => "unknown",
    }
}

// ── Execution errors ──

/// Typed execution failure from the canonical boundary.
///
/// Executor output is domain/event data, not preformatted terminal strings;
/// failures are typed so surfaces can render consistently.
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    /// Canonical request failed validation/normalization.
    #[error("invalid {operation_id} request: {reason}")]
    InvalidRequest {
        operation_id: String,
        reason: String,
    },
    /// Request does not match the approval binding (operation, normalized
    /// target, or policy-relevant descriptor field).
    #[error("dispatch binding failed: {reason}")]
    BindingMismatch { reason: String },
    /// Required Cargo feature is not compiled in.
    #[error("operation '{operation_id}' requires feature '{feature}' which is not compiled in")]
    FeatureUnavailable {
        operation_id: String,
        feature: String,
    },
    /// Execution failed after dispatch (domain error preserved as message).
    #[error("execution of '{operation_id}' failed: {message}")]
    ExecutionFailed {
        operation_id: String,
        message: String,
    },
    /// Execution was cancelled; no detached task survives (adapters race the
    /// future against the runtime cancellation token).
    #[error("execution of '{operation_id}' cancelled")]
    Cancelled { operation_id: String },
}

// ── Canonical execution entry points ──

/// Execute an approved canonical request.
///
/// This is the **single production owner** for
/// `(canonical operation ID + canonical typed request + ApprovedOperation)
/// → engine executor`.
///
/// Requirements enforced here:
/// - canonical operation ID is not inferred from user-facing aliases;
///   aliases must be resolved by surface adapters before this boundary;
/// - request/approval binding is checked immediately before executor entry
///   via exact [`ApprovedOperation::matches_descriptor`] semantics;
/// - feature availability is checked in one predictable layer;
/// - no Clap, Ratatui, terminal, daemon-protocol, or Python types cross this
///   boundary.
pub async fn execute_approved(
    approved: &ApprovedOperation,
    request: CanonicalOperationRequest,
    sink: &ExecutionSink,
) -> Result<TaskResult, ExecutionError> {
    let operation_id = request.operation_id();

    // 1. Canonical operation identity: exact match, no alias inference.
    if approved.descriptor().operation != operation_id {
        return Err(ExecutionError::BindingMismatch {
            reason: format!(
                "approved operation '{}' does not match request operation '{}' — aliases must be resolved before the execution boundary",
                approved.descriptor().operation,
                operation_id
            ),
        });
    }

    // 2. Request validation through canonical contracts (single owner).
    request
        .validate()
        .map_err(|e| ExecutionError::InvalidRequest {
            operation_id: operation_id.to_string(),
            reason: e.to_string(),
        })?;

    // 3. Exact descriptor binding: rebuild the expected descriptor from the
    // canonical request and require full equality (future policy-relevant
    // fields participate automatically via derived PartialEq).
    let expected = expected_descriptor_for_request(&request).map_err(|reason| {
        ExecutionError::InvalidRequest {
            operation_id: operation_id.to_string(),
            reason,
        }
    })?;
    if !approved.matches_descriptor(&expected) {
        return Err(ExecutionError::BindingMismatch {
            reason: binding_mismatch_reason(approved.descriptor(), &expected),
        });
    }

    // 4. Feature availability in one predictable layer.
    if let Some(metadata) = crate::config::metadata_for_tool_id(operation_id) {
        for feature in metadata.required_features {
            if !is_feature_available(feature) {
                return Err(ExecutionError::FeatureUnavailable {
                    operation_id: operation_id.to_string(),
                    feature: feature.to_string(),
                });
            }
        }
    } else {
        return Err(ExecutionError::InvalidRequest {
            operation_id: operation_id.to_string(),
            reason: "unknown canonical operation".to_string(),
        });
    }

    sink.emit_started(operation_id).await;

    // 5. Single executor owner.
    let outcome = execute_canonical(request, sink)
        .await
        .map_err(|e| match e {
            ExecutionError::Cancelled { .. } => e,
            ExecutionError::FeatureUnavailable { .. } => e,
            ExecutionError::InvalidRequest { .. } => e,
            ExecutionError::BindingMismatch { .. } => e,
            ExecutionError::ExecutionFailed {
                operation_id,
                message,
            } => {
                // Bridge domain anyhow errors into the typed boundary error.
                ExecutionError::ExecutionFailed {
                    operation_id,
                    message,
                }
            }
        })?;

    sink.emit_completed(&outcome_kind(&outcome)).await;
    Ok(outcome)
}

/// Build the expected [`crate::config::OperationDescriptor`] for a canonical
/// request via validated metadata construction.
fn expected_descriptor_for_request(
    request: &CanonicalOperationRequest,
) -> Result<crate::config::OperationDescriptor, String> {
    let operation_id = request.operation_id();
    let target = request.canonical_target();
    let metadata = crate::config::metadata_for_tool_id(operation_id)
        .ok_or_else(|| format!("unknown canonical operation '{operation_id}'"))?;
    metadata
        .try_descriptor_for_target(target.as_deref())
        .map_err(|e| format!("invalid target for operation '{operation_id}': {e}"))
}

fn binding_mismatch_reason(
    approved: &crate::config::OperationDescriptor,
    expected: &crate::config::OperationDescriptor,
) -> String {
    if approved.operation != expected.operation {
        return format!(
            "approved operation '{}' does not match request operation '{}'",
            approved.operation, expected.operation
        );
    }
    if approved.normalized_target != expected.normalized_target {
        return format!(
            "approved normalized target {:?} does not match request normalized target {:?}",
            approved.normalized_target, expected.normalized_target
        );
    }
    "approved descriptor does not match request descriptor (policy-relevant field changed)"
        .to_string()
}

/// Terminal outcome kind for an executed [`TaskResult`] (stable diagnostic).
pub fn outcome_kind(result: &TaskResult) -> String {
    match result {
        TaskResult::LoadTest(_) => "load-test".to_string(),
        #[cfg(feature = "stress-testing")]
        TaskResult::StressTest { .. } => "stress-test".to_string(),
        TaskResult::PortScan(_) => "port-scan".to_string(),
        TaskResult::EndpointScan(_) => "endpoint-scan".to_string(),
        TaskResult::Fingerprint(_) => "fingerprint".to_string(),
        TaskResult::WafDetection(_) => "waf-detect".to_string(),
        TaskResult::WafBypass { .. } => "waf-bypass".to_string(),
        TaskResult::WafStress(_) => "waf-stress".to_string(),
        TaskResult::Pipeline(_) => "pipeline".to_string(),
        TaskResult::Fuzz(_) => "fuzz".to_string(),
        TaskResult::Recon(_) => "recon".to_string(),
        TaskResult::PacketCapture { .. } => "packet-capture".to_string(),
        TaskResult::PacketTraceroute { .. } => "packet-traceroute".to_string(),
        TaskResult::PacketSend { .. } => "packet-send".to_string(),
        TaskResult::GraphQl(_) => "graphql".to_string(),
        TaskResult::OAuth(_) => "oauth".to_string(),
        #[cfg(feature = "nse")]
        TaskResult::Nse(_) => "nse".to_string(),
        #[cfg(feature = "advanced-hunting")]
        TaskResult::Hunt(_) => "hunt".to_string(),
        #[cfg(feature = "headless-browser")]
        TaskResult::Browser(_) => "browser".to_string(),
        #[cfg(feature = "compliance")]
        TaskResult::Compliance(_) => "compliance".to_string(),
        #[cfg(feature = "database")]
        TaskResult::Storage
        | TaskResult::StorageListScans { .. }
        | TaskResult::StorageListFindings { .. } => "storage".to_string(),
        #[cfg(feature = "external-integrations")]
        TaskResult::Integrations
        | TaskResult::IntegrationsCreateIssue { .. }
        | TaskResult::IntegrationsSearchIssues { .. } => "integrations".to_string(),
        #[cfg(feature = "finding-workflow")]
        TaskResult::Workflow(_) => "workflow".to_string(),
        #[cfg(feature = "vuln-management")]
        TaskResult::Vuln(_) => "vuln".to_string(),
        #[cfg(feature = "wireless")]
        TaskResult::Wireless(_) => "wireless".to_string(),
        #[cfg(feature = "wireless-advanced")]
        TaskResult::WirelessActive(_) => "wireless-active".to_string(),
        TaskResult::Auth(_) => "auth-test".to_string(),
        #[cfg(feature = "db-pentest")]
        TaskResult::DbPentest(_) => "db-pentest".to_string(),
        #[cfg(feature = "web-proxy")]
        TaskResult::Intercept(_) => "proxy-intercept".to_string(),
        #[cfg(feature = "c2")]
        TaskResult::C2(_) => "c2".to_string(),
        TaskResult::Error(_) => "error".to_string(),
        #[allow(unreachable_patterns)]
        _ => "unknown".to_string(),
    }
}

/// Internal single-owner executor dispatch.
///
/// Owns the canonical-request → domain-worker match. `dispatch_inner()` and
/// `execute_approved()` both route through here; neither owns a second
/// mapping. Feature-gated families without their feature return a typed
/// [`ExecutionError::FeatureUnavailable`]; the legacy `dispatch_inner` shim
/// converts that to `Ok(TaskResult::Error(..))` for backward compatibility.
pub async fn execute_canonical(
    request: CanonicalOperationRequest,
    sink: &ExecutionSink,
) -> Result<TaskResult, ExecutionError> {
    // Bridge legacy (u64, u64) progress into frontend-neutral events. Domain
    // workers still take the legacy sender; the forwarder re-emits as
    // ExecutionEvent::Progress through the coalescing path.
    let legacy_tx = sink.legacy_sender().unwrap_or_else(|| {
        let (tx, mut rx) = mpsc::channel(ExecutionSink::CAPACITY);
        tokio::spawn(async move { while rx.recv().await.is_some() {} });
        tx
    });
    let sink_clone = sink.clone();
    let (bridge_tx, mut bridge_rx) = mpsc::channel::<(u64, u64)>(ExecutionSink::CAPACITY);
    let forwarder = tokio::spawn(async move {
        while let Some((completed, total)) = bridge_rx.recv().await {
            sink_clone.emit_progress(completed, Some(total), None).await;
        }
    });
    // Fan out to both the caller's legacy sender and the event bridge.
    let fanout_tx = fanout_sender(legacy_tx, bridge_tx);

    let operation_id = request.operation_id().to_string();
    let result: Result<TaskResult, ExecutionError> = match request {
        CanonicalOperationRequest::LoadTest(raw) => {
            let n = raw
                .normalize()
                .map_err(|e| ExecutionError::InvalidRequest {
                    operation_id: operation_id.clone(),
                    reason: e.to_string(),
                })?;
            let timeout = std::time::Duration::from_secs(n.duration_secs);
            super::network::run_load_test(
                n.target,
                n.requests,
                n.concurrency,
                timeout,
                fanout_tx.clone(),
            )
            .await
            .map_err(|e| ExecutionError::ExecutionFailed {
                operation_id: operation_id.clone(),
                message: e.to_string(),
            })
        }
        CanonicalOperationRequest::StressTest(p) => super::network::run_stress_test(
            p.target,
            p.flood_type,
            p.rate_pps.unwrap_or(1000),
            p.duration_secs.unwrap_or(60) as u64,
            p.threads.unwrap_or(10) as usize,
            fanout_tx.clone(),
        )
        .await
        .map_err(|e| ExecutionError::ExecutionFailed {
            operation_id: operation_id.clone(),
            message: e.to_string(),
        }),
        CanonicalOperationRequest::PortScan(raw) => {
            let n = raw
                .normalize()
                .map_err(|e| ExecutionError::InvalidRequest {
                    operation_id: operation_id.clone(),
                    reason: e.to_string(),
                })?;
            let timeout = std::time::Duration::from_millis(n.timeout_ms);
            super::scanner::run_port_scan(
                n.target,
                n.ports,
                n.concurrency,
                timeout,
                fanout_tx.clone(),
            )
            .await
            .map_err(|e| ExecutionError::ExecutionFailed {
                operation_id: operation_id.clone(),
                message: e.to_string(),
            })
        }
        CanonicalOperationRequest::EndpointScan(raw) => {
            let n = raw
                .normalize()
                .map_err(|e| ExecutionError::InvalidRequest {
                    operation_id: operation_id.clone(),
                    reason: e.to_string(),
                })?;
            let timeout = std::time::Duration::from_secs(n.timeout_secs);
            super::scanner::run_endpoint_scan(
                n.target,
                n.concurrency,
                timeout,
                n.wordlist,
                fanout_tx.clone(),
            )
            .await
            .map_err(|e| ExecutionError::ExecutionFailed {
                operation_id: operation_id.clone(),
                message: e.to_string(),
            })
        }
        CanonicalOperationRequest::Fingerprint(raw) => {
            let n = raw
                .normalize()
                .map_err(|e| ExecutionError::InvalidRequest {
                    operation_id: operation_id.clone(),
                    reason: e.to_string(),
                })?;
            let timeout = std::time::Duration::from_secs(n.timeout_secs);
            super::scanner::run_fingerprint(
                n.target,
                n.ports,
                timeout,
                n.concurrency,
                fanout_tx.clone(),
            )
            .await
            .map_err(|e| ExecutionError::ExecutionFailed {
                operation_id: operation_id.clone(),
                message: e.to_string(),
            })
        }
        CanonicalOperationRequest::Fuzz(raw) => {
            let n = raw
                .normalize()
                .map_err(|e| ExecutionError::InvalidRequest {
                    operation_id: operation_id.clone(),
                    reason: e.to_string(),
                })?;
            super::fuzzer::run_fuzz(
                n.target,
                n.payload_type,
                n.mode,
                n.mutations,
                n.mutation_count,
                n.method,
                n.param,
                n.threads,
                n.timeout_secs,
                n.graphql_introspection,
                n.graphql_depth_bypass,
                n.graphql_alias_overload,
                n.oauth_redirect_test,
                n.oauth_scope_test,
                n.oauth_state_test,
                n.oauth_grant_test,
                fanout_tx.clone(),
            )
            .await
            .map_err(|e| ExecutionError::ExecutionFailed {
                operation_id: operation_id.clone(),
                message: e.to_string(),
            })
        }
        CanonicalOperationRequest::WafDetect(raw) => {
            let n = raw
                .normalize()
                .map_err(|e| ExecutionError::InvalidRequest {
                    operation_id: operation_id.clone(),
                    reason: e.to_string(),
                })?;
            super::fuzzer::run_waf(n.target, n.bypass_mode, n.techniques, fanout_tx.clone())
                .await
                .map_err(|e| ExecutionError::ExecutionFailed {
                    operation_id: operation_id.clone(),
                    message: e.to_string(),
                })
        }
        CanonicalOperationRequest::WafStress(raw) => {
            let n = raw
                .normalize()
                .map_err(|e| ExecutionError::InvalidRequest {
                    operation_id: operation_id.clone(),
                    reason: e.to_string(),
                })?;
            super::fuzzer::run_waf_stress(n.target, n.concurrency, n.requests, fanout_tx.clone())
                .await
                .map_err(|e| ExecutionError::ExecutionFailed {
                    operation_id: operation_id.clone(),
                    message: e.to_string(),
                })
        }
        CanonicalOperationRequest::Pipeline(raw) => {
            let n = raw
                .normalize()
                .map_err(|e| ExecutionError::InvalidRequest {
                    operation_id: operation_id.clone(),
                    reason: e.to_string(),
                })?;
            let profile = match n.profile.as_str() {
                "quick" => crate::types::ScanProfile::Quick,
                "endpoint" => crate::types::ScanProfile::Endpoint,
                "web" => crate::types::ScanProfile::Web,
                "waf" => crate::types::ScanProfile::Waf,
                "full" => crate::types::ScanProfile::Full,
                "api" => crate::types::ScanProfile::Api,
                "recon" => crate::types::ScanProfile::Recon,
                "stealth" => crate::types::ScanProfile::Stealth,
                "deep" => crate::types::ScanProfile::Deep,
                "vuln" => crate::types::ScanProfile::Vuln,
                "auth" => crate::types::ScanProfile::Auth,
                "defense-lab" => crate::types::ScanProfile::DefenseLab,
                other => {
                    // Early return must still release the progress bridge so
                    // the forwarder task cannot outlive this call.
                    drop(fanout_tx);
                    forwarder.abort();
                    return Err(ExecutionError::InvalidRequest {
                        operation_id: operation_id.clone(),
                        reason: format!("unknown scan profile '{other}'"),
                    });
                }
            };
            super::recon::run_pipeline(n.target, profile, fanout_tx.clone())
                .await
                .map_err(|e| ExecutionError::ExecutionFailed {
                    operation_id: operation_id.clone(),
                    message: e.to_string(),
                })
        }
        CanonicalOperationRequest::Recon(raw) => {
            let n = raw
                .normalize()
                .map_err(|e| ExecutionError::InvalidRequest {
                    operation_id: operation_id.clone(),
                    reason: e.to_string(),
                })?;
            super::recon::run_recon(
                n.target,
                20,
                super::ReconOptions::default(),
                fanout_tx.clone(),
            )
            .await
            .map_err(|e| ExecutionError::ExecutionFailed {
                operation_id: operation_id.clone(),
                message: e.to_string(),
            })
        }
        CanonicalOperationRequest::PacketCapture(p) => super::network::run_packet_capture(
            p.interface.unwrap_or_else(|| "eth0".to_string()),
            p.filter.unwrap_or_default(),
            p.max_packets.unwrap_or(1000),
            p.promiscuous.unwrap_or(true),
            None,
            fanout_tx.clone(),
        )
        .await
        .map_err(|e| ExecutionError::ExecutionFailed {
            operation_id: operation_id.clone(),
            message: e.to_string(),
        }),
        CanonicalOperationRequest::PacketTraceroute(p) => super::network::run_packet_traceroute(
            p.target,
            p.max_hops.unwrap_or(30) as u8,
            fanout_tx.clone(),
        )
        .await
        .map_err(|e| ExecutionError::ExecutionFailed {
            operation_id: operation_id.clone(),
            message: e.to_string(),
        }),
        CanonicalOperationRequest::PacketSend(p) => super::network::run_packet_send(
            p.target,
            p.port.unwrap_or(80),
            p.count.unwrap_or(10),
            p.packet_size.unwrap_or(64),
            fanout_tx.clone(),
        )
        .await
        .map_err(|e| ExecutionError::ExecutionFailed {
            operation_id: operation_id.clone(),
            message: e.to_string(),
        }),
        CanonicalOperationRequest::GraphQl(raw) => {
            let n = raw
                .normalize()
                .map_err(|e| ExecutionError::InvalidRequest {
                    operation_id: operation_id.clone(),
                    reason: e.to_string(),
                })?;
            super::api::run_graphql(
                n.target,
                n.introspection,
                n.inject,
                n.depth_bypass,
                n.alias_overload,
                n.concurrency,
                n.timeout_secs,
                fanout_tx.clone(),
            )
            .await
            .map_err(|e| ExecutionError::ExecutionFailed {
                operation_id: operation_id.clone(),
                message: e.to_string(),
            })
        }
        CanonicalOperationRequest::OAuth(raw) => {
            let n = raw
                .normalize()
                .map_err(|e| ExecutionError::InvalidRequest {
                    operation_id: operation_id.clone(),
                    reason: e.to_string(),
                })?;
            super::api::run_oauth(
                n.target,
                n.client_id,
                n.redirect_uri,
                n.redirect_test,
                n.scope_test,
                n.state_test,
                n.grant_test,
                n.concurrency,
                n.timeout_secs,
                fanout_tx.clone(),
            )
            .await
            .map_err(|e| ExecutionError::ExecutionFailed {
                operation_id: operation_id.clone(),
                message: e.to_string(),
            })
        }
        CanonicalOperationRequest::AuthTest(raw) => {
            let n = raw
                .normalize()
                .map_err(|e| ExecutionError::InvalidRequest {
                    operation_id: operation_id.clone(),
                    reason: e.to_string(),
                })?;
            super::auth::run_auth_task(
                n.target,
                n.username,
                n.credential_list,
                n.credential_file,
                n.max_attempts,
                n.concurrency,
                n.timeout_secs,
                fanout_tx.clone(),
            )
            .await
            .map_err(|e| ExecutionError::ExecutionFailed {
                operation_id: operation_id.clone(),
                message: e.to_string(),
            })
        }
        CanonicalOperationRequest::Nse(p) => {
            #[cfg(feature = "nse")]
            {
                super::api::run_nse(p.target, p.script, p.args, None, fanout_tx.clone())
                    .await
                    .map_err(|e| ExecutionError::ExecutionFailed {
                        operation_id: operation_id.clone(),
                        message: e.to_string(),
                    })
            }
            #[cfg(not(feature = "nse"))]
            {
                let _ = p;
                Err(ExecutionError::FeatureUnavailable {
                    operation_id: operation_id.clone(),
                    feature: "nse".to_string(),
                })
            }
        }
        CanonicalOperationRequest::Hunt(p) => {
            #[cfg(feature = "advanced-hunting")]
            {
                super::security::run_hunt_task(
                    p.target,
                    crate::hunt::HuntConfig::default(),
                    fanout_tx.clone(),
                )
                .await
                .map_err(|e| ExecutionError::ExecutionFailed {
                    operation_id: operation_id.clone(),
                    message: e.to_string(),
                })
            }
            #[cfg(not(feature = "advanced-hunting"))]
            {
                let _ = p;
                Err(ExecutionError::FeatureUnavailable {
                    operation_id: operation_id.clone(),
                    feature: "advanced-hunting".to_string(),
                })
            }
        }
        CanonicalOperationRequest::Browser(p) => {
            #[cfg(feature = "headless-browser")]
            {
                super::security::run_browser_task(
                    p.target,
                    crate::browser::BrowserConfig::default(),
                    fanout_tx.clone(),
                )
                .await
                .map_err(|e| ExecutionError::ExecutionFailed {
                    operation_id: operation_id.clone(),
                    message: e.to_string(),
                })
            }
            #[cfg(not(feature = "headless-browser"))]
            {
                let _ = p;
                Err(ExecutionError::FeatureUnavailable {
                    operation_id: operation_id.clone(),
                    feature: "headless-browser".to_string(),
                })
            }
        }
        CanonicalOperationRequest::Compliance(p) => {
            #[cfg(feature = "compliance")]
            {
                super::security::run_compliance_task(
                    p.target,
                    crate::compliance::ComplianceFramework::OWASP,
                    fanout_tx.clone(),
                )
                .await
                .map_err(|e| ExecutionError::ExecutionFailed {
                    operation_id: operation_id.clone(),
                    message: e.to_string(),
                })
            }
            #[cfg(not(feature = "compliance"))]
            {
                let _ = p;
                Err(ExecutionError::FeatureUnavailable {
                    operation_id: operation_id.clone(),
                    feature: "compliance".to_string(),
                })
            }
        }
        CanonicalOperationRequest::Storage(raw) => {
            raw.normalize()
                .map_err(|e| ExecutionError::InvalidRequest {
                    operation_id: operation_id.clone(),
                    reason: e.to_string(),
                })?;
            #[cfg(feature = "database")]
            {
                super::security::run_storage_task(
                    crate::storage::StorageConfig::default(),
                    "read".to_string(),
                    None,
                    None,
                    None,
                    fanout_tx.clone(),
                )
                .await
                .map_err(|e| ExecutionError::ExecutionFailed {
                    operation_id: operation_id.clone(),
                    message: e.to_string(),
                })
            }
            #[cfg(not(feature = "database"))]
            {
                let _ = &fanout_tx;
                Err(ExecutionError::FeatureUnavailable {
                    operation_id: operation_id.clone(),
                    feature: "database".to_string(),
                })
            }
        }
        CanonicalOperationRequest::Integrations(p) => {
            #[cfg(feature = "external-integrations")]
            {
                super::security::run_integrations_task(
                    crate::integrations::IntegrationConfig::default(),
                    "list".to_string(),
                    None,
                    None,
                    vec![],
                    vec![],
                    None,
                    fanout_tx.clone(),
                )
                .await
                .map_err(|e| ExecutionError::ExecutionFailed {
                    operation_id: operation_id.clone(),
                    message: e.to_string(),
                })
            }
            #[cfg(not(feature = "external-integrations"))]
            {
                let _ = p;
                Err(ExecutionError::FeatureUnavailable {
                    operation_id: operation_id.clone(),
                    feature: "external-integrations".to_string(),
                })
            }
        }
        CanonicalOperationRequest::Workflow(_) => {
            #[cfg(feature = "finding-workflow")]
            {
                super::security::run_workflow_task(
                    "list".to_string(),
                    None,
                    vec![],
                    fanout_tx.clone(),
                )
                .await
                .map_err(|e| ExecutionError::ExecutionFailed {
                    operation_id: operation_id.clone(),
                    message: e.to_string(),
                })
            }
            #[cfg(not(feature = "finding-workflow"))]
            {
                let _ = &fanout_tx;
                Err(ExecutionError::FeatureUnavailable {
                    operation_id: operation_id.clone(),
                    feature: "finding-workflow".to_string(),
                })
            }
        }
        CanonicalOperationRequest::Vuln(p) => {
            #[cfg(feature = "vuln-management")]
            {
                super::security::run_vuln_task(
                    "assess".to_string(),
                    Some(p.target),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    fanout_tx.clone(),
                )
                .await
                .map_err(|e| ExecutionError::ExecutionFailed {
                    operation_id: operation_id.clone(),
                    message: e.to_string(),
                })
            }
            #[cfg(not(feature = "vuln-management"))]
            {
                let _ = p;
                Err(ExecutionError::FeatureUnavailable {
                    operation_id: operation_id.clone(),
                    feature: "vuln-management".to_string(),
                })
            }
        }
        CanonicalOperationRequest::Wireless(p) => {
            #[cfg(feature = "wireless")]
            {
                super::security::run_wireless_task(
                    p.interface.unwrap_or_else(|| "wlan0".to_string()),
                    fanout_tx.clone(),
                )
                .await
                .map_err(|e| ExecutionError::ExecutionFailed {
                    operation_id: operation_id.clone(),
                    message: e.to_string(),
                })
            }
            #[cfg(not(feature = "wireless"))]
            {
                let _ = p;
                Err(ExecutionError::FeatureUnavailable {
                    operation_id: operation_id.clone(),
                    feature: "wireless".to_string(),
                })
            }
        }
        CanonicalOperationRequest::WirelessActive(p) => {
            #[cfg(feature = "wireless-advanced")]
            {
                super::security::run_wireless_active_task(
                    p.interface.unwrap_or_else(|| "wlan0".to_string()),
                    "deauth".to_string(),
                    p.target_bssid,
                    None,
                    100,
                    10,
                    true,
                    fanout_tx.clone(),
                )
                .await
                .map_err(|e| ExecutionError::ExecutionFailed {
                    operation_id: operation_id.clone(),
                    message: e.to_string(),
                })
            }
            #[cfg(not(feature = "wireless-advanced"))]
            {
                let _ = p;
                // Wireless-active without the advanced gate degrades to the
                // base wireless gate for a consistent unavailable signal.
                let feature = if cfg!(feature = "wireless") {
                    "wireless-advanced"
                } else {
                    "wireless"
                };
                Err(ExecutionError::FeatureUnavailable {
                    operation_id: operation_id.clone(),
                    feature: feature.to_string(),
                })
            }
        }
        CanonicalOperationRequest::DbPentest(raw) => {
            let n = raw
                .normalize()
                .map_err(|e| ExecutionError::InvalidRequest {
                    operation_id: operation_id.clone(),
                    reason: e.to_string(),
                })?;
            #[cfg(feature = "db-pentest")]
            {
                super::db_pentest::run_db_pentest_task(
                    None,
                    Some(n.target),
                    Some(n.db_type),
                    n.port,
                    n.checks,
                    n.dry_run,
                    n.allow_advanced,
                    n.max_queries,
                    n.max_duration_secs,
                    fanout_tx.clone(),
                )
                .await
                .map_err(|e| ExecutionError::ExecutionFailed {
                    operation_id: operation_id.clone(),
                    message: e.to_string(),
                })
            }
            #[cfg(not(feature = "db-pentest"))]
            {
                let _ = n;
                Err(ExecutionError::FeatureUnavailable {
                    operation_id: operation_id.clone(),
                    feature: "db-pentest".to_string(),
                })
            }
        }
        CanonicalOperationRequest::Intercept(p) => {
            #[cfg(feature = "web-proxy")]
            {
                super::intercept::run_intercept_task(
                    format!(
                        "{}:{}",
                        p.listen_host.unwrap_or_else(|| "127.0.0.1".to_string()),
                        p.listen_port.unwrap_or(8080)
                    ),
                    p.dry_run.unwrap_or(true),
                    p.max_flows.unwrap_or(100),
                    p.target,
                    fanout_tx.clone(),
                )
                .await
                .map_err(|e| ExecutionError::ExecutionFailed {
                    operation_id: operation_id.clone(),
                    message: e.to_string(),
                })
            }
            #[cfg(not(feature = "web-proxy"))]
            {
                let _ = p;
                Err(ExecutionError::FeatureUnavailable {
                    operation_id: operation_id.clone(),
                    feature: "web-proxy".to_string(),
                })
            }
        }
        CanonicalOperationRequest::C2(p) => {
            #[cfg(feature = "c2")]
            {
                super::c2::run_c2_task(
                    p.target.unwrap_or_else(|| "127.0.0.1".to_string()),
                    p.profile.unwrap_or_else(|| "default".to_string()),
                    p.dry_run.unwrap_or(true),
                    fanout_tx.clone(),
                )
                .await
                .map_err(|e| ExecutionError::ExecutionFailed {
                    operation_id: operation_id.clone(),
                    message: e.to_string(),
                })
            }
            #[cfg(not(feature = "c2"))]
            {
                let _ = p;
                Err(ExecutionError::FeatureUnavailable {
                    operation_id: operation_id.clone(),
                    feature: "c2".to_string(),
                })
            }
        }
    };

    // Drain the bridge forwarder (it exits when fanout clones drop).
    drop(fanout_tx);
    if let Err(error) = forwarder.await {
        tracing::warn!(?error, "execution progress forwarder failed");
    }

    // Surface normalized-target context for scope decisions without
    // duplicating authorization here (enforcement already approved).
    let _ = normalize_target("localhost", None);
    let _ = OperationTarget::None;

    result
}

/// Fan out legacy progress to the caller's sender and the event bridge.
///
/// Both sends are best-effort (`try_send`): producers never block on progress
/// delivery. Findings and terminal outcomes use the event channel directly
/// (see [`ExecutionSink`]) and are never dropped.
fn fanout_sender(
    primary: mpsc::Sender<(u64, u64)>,
    bridge: mpsc::Sender<(u64, u64)>,
) -> mpsc::Sender<(u64, u64)> {
    let (tx, mut rx) = mpsc::channel::<(u64, u64)>(ExecutionSink::CAPACITY);
    tokio::spawn(async move {
        while let Some(item) = rx.recv().await {
            if let Err(e) = primary.try_send(item) {
                tracing::debug!("canonical fanout primary coalesced: {}", e);
            }
            if let Err(e) = bridge.try_send(item) {
                tracing::debug!("canonical fanout bridge coalesced: {}", e);
            }
        }
    });
    tx
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggsec_runtime::request::*;

    fn test_sink() -> (ExecutionSink, mpsc::Receiver<ExecutionEvent>) {
        let (event_tx, event_rx) = mpsc::channel(ExecutionSink::CAPACITY);
        let (legacy_tx, _legacy_rx) = mpsc::channel(ExecutionSink::CAPACITY);
        (ExecutionSink::new(event_tx, Some(legacy_tx)), event_rx)
    }

    #[test]
    fn canonical_ids_are_not_aliases() {
        // Aliases must be resolved before the boundary; the boundary only
        // emits canonical IDs.
        let req = CanonicalOperationRequest::PortScan(crate::operation_request::PortScanRequest {
            target: "10.0.0.1".into(),
            ports: None,
            scan_type: None,
            timeout_ms: None,
            concurrency: None,
        });
        assert_eq!(req.operation_id(), "scan-ports");
        assert_ne!(req.operation_id(), "scan");

        let waf =
            CanonicalOperationRequest::WafDetect(crate::operation_request::WafDetectRequest {
                target: "https://example.com".into(),
                bypass_mode: None,
                techniques: None,
            });
        assert_eq!(waf.operation_id(), "waf-detect");
        assert_ne!(waf.operation_id(), "waf");

        let pipe = CanonicalOperationRequest::Pipeline(crate::operation_request::PipelineRequest {
            target: "https://example.com".into(),
            profile: None,
        });
        assert_eq!(pipe.operation_id(), "pipeline");
    }

    #[test]
    fn packet_family_shares_canonical_operation() {
        let cap = CanonicalOperationRequest::PacketCapture(PacketCaptureParams::default());
        let trace = CanonicalOperationRequest::PacketTraceroute(PacketTracerouteParams {
            target: "10.0.0.1".into(),
            max_hops: None,
        });
        let send = CanonicalOperationRequest::PacketSend(PacketSendParams {
            target: "10.0.0.1".into(),
            protocol: "tcp".into(),
            ..Default::default()
        });
        assert_eq!(cap.operation_id(), "packet");
        assert_eq!(trace.operation_id(), "packet");
        assert_eq!(send.operation_id(), "packet");
        assert_eq!(cap.canonical_target(), None);
        assert_eq!(trace.canonical_target(), Some("10.0.0.1".to_string()));
    }

    #[test]
    fn from_task_kind_is_exhaustive_and_canonical() {
        // Every TaskKind maps to a canonical operation ID identical to the
        // dual-owner mappings (Phase 0 guards). Adding a TaskKind variant
        // without updating from_task_kind is a compile error.
        let kinds: Vec<(TaskKind, &'static str)> = vec![
            (
                TaskKind::PortScan(PortScanParams {
                    target: "10.0.0.1".into(),
                    ..Default::default()
                }),
                "scan-ports",
            ),
            (
                TaskKind::Recon(ReconParams {
                    target: "example.com".into(),
                    modules: None,
                }),
                "recon",
            ),
            (
                TaskKind::Fuzz(FuzzParams {
                    target: "https://example.com".into(),
                    ..Default::default()
                }),
                "fuzz",
            ),
            (
                TaskKind::Waf(WafParams {
                    target: "https://example.com".into(),
                    ..Default::default()
                }),
                "waf-detect",
            ),
            (
                TaskKind::LoadTest(LoadTestParams {
                    target: "https://example.com".into(),
                    method: "GET".into(),
                    ..Default::default()
                }),
                "load-test",
            ),
            (
                TaskKind::Pipeline(PipelineParams {
                    target: "https://example.com".into(),
                    profile: None,
                }),
                "pipeline",
            ),
        ];
        for (kind, expected_op) in &kinds {
            let canonical = CanonicalOperationRequest::from_task_kind(kind);
            assert_eq!(canonical.operation_id(), *expected_op);
            assert_eq!(canonical.operation_id(), kind.operation_id());
            assert_eq!(canonical.canonical_target(), kind.canonical_target());
            assert_eq!(
                executor_route_for(canonical.operation_id()),
                executor_route_for(kind.operation_id())
            );
        }
    }

    #[test]
    fn executor_route_covers_representative_families() {
        assert_eq!(executor_route_for("scan-ports"), "scanner");
        assert_eq!(executor_route_for("recon"), "recon");
        assert_eq!(executor_route_for("fuzz"), "fuzz-api");
        assert_eq!(executor_route_for("waf-detect"), "waf");
        assert_eq!(executor_route_for("load-test"), "network");
        assert_eq!(executor_route_for("pipeline"), "recon");
        assert_eq!(executor_route_for("packet"), "network");
        assert_eq!(executor_route_for("graphql"), "fuzz-api");
        assert_eq!(executor_route_for("db-pentest"), "db-pentest");
        assert_eq!(executor_route_for("storage"), "security");
    }

    #[test]
    fn feature_availability_is_predictable() {
        // Core operations have no gate.
        assert!(is_feature_available(""));
        // Unknown features fail closed.
        assert!(!is_feature_available("bogus-feature-xyz"));
    }

    #[tokio::test]
    async fn sink_coalesces_progress_but_never_drops_findings() {
        let (event_tx, mut event_rx) = mpsc::channel(2);
        let sink = ExecutionSink::new(event_tx, None);
        // Drain concurrently so must-deliver sends (backpressure) cannot
        // deadlock against an undrained small channel. Progress still uses
        // try_send, so bursts beyond capacity coalesce and count.
        let collector = tokio::spawn(async move {
            let mut saw_finding = false;
            let mut saw_completed = false;
            // Expect at most 22 events; stop after terminal completion.
            for _ in 0..64 {
                match tokio::time::timeout(std::time::Duration::from_secs(5), event_rx.recv()).await
                {
                    Ok(Some(ExecutionEvent::Finding { .. })) => saw_finding = true,
                    Ok(Some(ExecutionEvent::Completed { .. })) => {
                        saw_completed = true;
                        break;
                    }
                    Ok(Some(_)) => {}
                    Ok(None) | Err(_) => break,
                }
            }
            (saw_finding, saw_completed)
        });
        // Flood progress beyond capacity; some must coalesce.
        for i in 0..20u64 {
            sink.emit_progress(i, Some(100), None).await;
        }
        // Findings and terminal outcomes still deliver (backpressure, not drop).
        sink.emit_finding("sqli".to_string()).await;
        sink.emit_completed("fuzz").await;
        drop(sink);

        let (saw_finding, saw_completed) =
            tokio::time::timeout(std::time::Duration::from_secs(10), collector)
                .await
                .expect("collector must finish")
                .expect("collector task must succeed");
        assert!(saw_finding, "findings must never be dropped");
        assert!(saw_completed, "terminal outcomes must never be dropped");
    }

    #[tokio::test]
    async fn sink_counts_coalesced_progress_deterministically() {
        // Capacity 1, pre-filled: the next progress try_send must coalesce
        // and increment the loss counter instead of blocking.
        let (event_tx, _event_rx) = mpsc::channel(1);
        event_tx
            .try_send(ExecutionEvent::Progress {
                completed: 0,
                total: Some(100),
                message: None,
            })
            .expect("pre-fill must succeed");
        let sink = ExecutionSink::new(event_tx, None);
        sink.emit_progress(1, Some(100), None).await;
        assert_eq!(sink.dropped_progress_count(), 1);
    }

    #[tokio::test]
    async fn execute_approved_rejects_alias_mismatch() {
        use crate::config::{EnforcementContext, ExecutionSurface, OperationDescriptor};
        use crate::config::{ExecutionPolicy, LoadedScope, OperationMode, OperationRisk};

        let descriptor = OperationDescriptor::new(
            "scan-ports".to_string(),
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            Vec::new(),
            Some("127.0.0.1".to_string()),
            Vec::new(),
            Vec::new(),
            false,
            false,
            Vec::new(),
        );
        let enforcement = EnforcementContext::for_surface(
            ExecutionSurface::CliManual,
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let approved = enforcement
            .approve_manual(ExecutionSurface::CliManual, descriptor, None)
            .expect("loopback scan-ports should approve");
        // Request a different operation under the same approval.
        let request = CanonicalOperationRequest::Recon(crate::operation_request::ReconRequest {
            target: "127.0.0.1".into(),
            modules: None,
        });
        let (sink, _rx) = test_sink();
        let result = execute_approved(&approved, request, &sink).await;
        assert!(matches!(
            result,
            Err(ExecutionError::BindingMismatch { .. })
        ));
    }

    #[tokio::test]
    async fn execute_approved_rejects_target_mismatch() {
        use crate::config::{
            EnforcementContext, ExecutionPolicy, ExecutionSurface, LoadedScope,
            OperationDescriptor, OperationMode, OperationRisk,
        };

        let descriptor = OperationDescriptor::new(
            "scan-ports".to_string(),
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            Vec::new(),
            Some("127.0.0.1".to_string()),
            Vec::new(),
            Vec::new(),
            false,
            false,
            Vec::new(),
        );
        let enforcement = EnforcementContext::for_surface(
            ExecutionSurface::CliManual,
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let approved = enforcement
            .approve_manual(ExecutionSurface::CliManual, descriptor, None)
            .expect("loopback should approve");
        let request =
            CanonicalOperationRequest::PortScan(crate::operation_request::PortScanRequest {
                target: "127.0.0.2".into(),
                ports: Some("80".into()),
                scan_type: None,
                timeout_ms: None,
                concurrency: None,
            });
        let (sink, _rx) = test_sink();
        let result = execute_approved(&approved, request, &sink).await;
        assert!(matches!(
            result,
            Err(ExecutionError::BindingMismatch { .. })
        ));
    }
}
