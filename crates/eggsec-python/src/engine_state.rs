use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use pyo3::prelude::*;

use crate::config_model::PyEggsecConfig;
use crate::operation_registry::OperationExecutorRegistry;
use crate::scope::Scope;

/// Structured audit record emitted by the Python engine gate.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct DispatchAuditEvent {
    #[pyo3(get)]
    pub event_id: String,
    #[pyo3(get)]
    pub timestamp_ms: u64,
    #[pyo3(get)]
    pub operation_id: String,
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub surface: String,
    #[pyo3(get)]
    pub outcome: String,
    #[pyo3(get)]
    pub allowed: bool,
    #[pyo3(get)]
    pub decision_summary: String,
    #[pyo3(get)]
    pub redacted: bool,
}

#[pymethods]
impl DispatchAuditEvent {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = pyo3::types::PyDict::new_bound(py);
        dict.set_item("event_id", &self.event_id)?;
        dict.set_item("timestamp_ms", self.timestamp_ms)?;
        dict.set_item("operation_id", &self.operation_id)?;
        dict.set_item("target", &self.target)?;
        dict.set_item("surface", &self.surface)?;
        dict.set_item("outcome", &self.outcome)?;
        dict.set_item("allowed", self.allowed)?;
        dict.set_item("decision_summary", &self.decision_summary)?;
        dict.set_item("redacted", self.redacted)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&serde_json::json!({
            "event_id": self.event_id,
            "timestamp_ms": self.timestamp_ms,
            "operation_id": self.operation_id,
            "target": self.target,
            "surface": self.surface,
            "outcome": self.outcome,
            "allowed": self.allowed,
            "decision_summary": self.decision_summary,
            "redacted": self.redacted,
        }))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DispatchAuditEvent(operation_id={}, outcome={}, allowed={})",
            self.operation_id, self.outcome, self.allowed
        )
    }
}

/// The result of the mandatory pre-dispatch gate.
#[derive(Debug, Clone)]
pub struct DispatchDecision {
    pub audit_event_id: String,
    pub summary: String,
}

/// Optional event channel sender for emitting lifecycle events from operations.
///
/// The wrapper assigns stream sequence numbers and records failed non-blocking
/// deliveries, so callers cannot silently lose events without accounting.
#[derive(Clone)]
pub struct EventSender {
    inner: tokio::sync::mpsc::Sender<crate::event_protocol::EventEnvelope>,
    next_sequence: Arc<AtomicU64>,
    stats: Arc<Mutex<crate::backpressure::EventDeliveryStats>>,
}

impl EventSender {
    fn new(inner: tokio::sync::mpsc::Sender<crate::event_protocol::EventEnvelope>) -> Self {
        Self {
            inner,
            next_sequence: Arc::new(AtomicU64::new(1)),
            stats: Arc::new(Mutex::new(Default::default())),
        }
    }

    pub fn try_send(
        &self,
        mut event: crate::event_protocol::EventEnvelope,
    ) -> Result<(), tokio::sync::mpsc::error::TrySendError<crate::event_protocol::EventEnvelope>>
    {
        if event.sequence == 0 {
            event.sequence = self.next_sequence.fetch_add(1, Ordering::Relaxed);
        }
        if let Ok(mut stats) = self.stats.lock() {
            stats.emitted_count += 1;
        }
        match self.inner.try_send(event) {
            Ok(()) => {
                if let Ok(mut stats) = self.stats.lock() {
                    stats.delivered_count += 1;
                }
                Ok(())
            }
            Err(error) => {
                let kind = match &error {
                    tokio::sync::mpsc::error::TrySendError::Full(event)
                    | tokio::sync::mpsc::error::TrySendError::Closed(event) => {
                        event.event_type.clone()
                    }
                };
                let reliable = crate::backpressure::is_reliable(&kind);
                if let Ok(mut stats) = self.stats.lock() {
                    stats.dropped_count += 1;
                    *stats.dropped_by_kind.entry(kind).or_default() += 1;
                    if reliable {
                        stats.terminal_event_delivery_failures += 1;
                    }
                }
                tracing::warn!("event channel delivery failed; loss recorded");
                Err(error)
            }
        }
    }
}

/// Shared execution configuration for Engine and AsyncEngine.
///
/// Both sync and async engines hold `Arc<EngineState>` so every operation
/// passes through common validation, scope enforcement, feature gating,
/// and audit logging before execution.
pub struct EngineState {
    /// The execution scope defining authorized targets and ports.
    pub scope: Scope,
    /// Execution mode ("manual" or "automation").
    pub mode: String,
    /// Max concurrent connections.
    pub concurrency: usize,
    /// Connection timeout in milliseconds.
    pub timeout_ms: u64,
    /// Full engine configuration snapshot.
    pub config: PyEggsecConfig,
    /// Operation registry mapping IDs to metadata and feature requirements.
    pub registry: OperationExecutorRegistry,
    /// Optional event channel for emitting lifecycle events.
    pub event_tx: Option<EventSender>,
    /// The canonical Rust policy context used by every engine dispatch path.
    pub enforcement: eggsec::config::EnforcementContext,
    /// In-process structured audit sink for allow and deny decisions.
    pub audit_events: Arc<Mutex<Vec<DispatchAuditEvent>>>,
}

impl EngineState {
    /// Create an EngineState from individual parameters (backward-compatible path).
    pub fn from_params(
        scope: Scope,
        mode: &str,
        concurrency: usize,
        timeout_ms: u64,
    ) -> PyResult<Arc<Self>> {
        if mode != "manual" && mode != "automation" {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid mode '{}'. Must be 'manual' or 'automation'.",
                mode
            )));
        }
        let loaded_scope = eggsec::config::LoadedScope::explicit(
            scope.inner.clone(),
            eggsec::config::ScopeSource::GeneratedPreset,
            None,
        );
        let enforcement = eggsec::config::EnforcementContext::for_surface(
            if mode == "automation" {
                eggsec::config::ExecutionSurface::SecurityAgent
            } else {
                eggsec::config::ExecutionSurface::CliManual
            },
            eggsec::config::ExecutionPolicy::default(),
            loaded_scope,
        );
        Ok(Arc::new(Self {
            scope,
            mode: mode.to_string(),
            concurrency,
            timeout_ms,
            config: PyEggsecConfig::new_default(),
            registry: OperationExecutorRegistry::default_stable(),
            event_tx: None,
            enforcement,
            audit_events: Arc::new(Mutex::new(Vec::new())),
        }))
    }

    /// Create an EngineState from a full EggsecConfig and scope.
    pub fn from_config(
        scope: Scope,
        config: PyEggsecConfig,
        mode: &str,
        concurrency: Option<usize>,
        timeout_ms: Option<u64>,
    ) -> PyResult<Arc<Self>> {
        if mode != "manual" && mode != "automation" {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid mode '{}'. Must be 'manual' or 'automation'.",
                mode
            )));
        }
        let default_concurrency = config.default_concurrency();
        let loaded_scope = eggsec::config::LoadedScope::explicit(
            scope.inner.clone(),
            eggsec::config::ScopeSource::GeneratedPreset,
            None,
        );
        let enforcement = eggsec::config::EnforcementContext::for_surface(
            if mode == "automation" {
                eggsec::config::ExecutionSurface::SecurityAgent
            } else {
                eggsec::config::ExecutionSurface::CliManual
            },
            eggsec::config::ExecutionPolicy::default(),
            loaded_scope,
        );
        Ok(Arc::new(Self {
            scope,
            mode: mode.to_string(),
            concurrency: concurrency.unwrap_or(default_concurrency),
            timeout_ms: timeout_ms.unwrap_or(5000),
            config,
            registry: OperationExecutorRegistry::default_stable(),
            event_tx: None,
            enforcement,
            audit_events: Arc::new(Mutex::new(Vec::new())),
        }))
    }

    /// Set the event channel sender for lifecycle event emission.
    pub fn set_event_tx(
        &mut self,
        tx: tokio::sync::mpsc::Sender<crate::event_protocol::EventEnvelope>,
    ) {
        self.event_tx = Some(EventSender::new(tx));
    }

    /// Emit a lifecycle event if an event channel is configured.
    ///
    /// Non-blocking: drops the event if the channel is full (backpressure).
    pub fn emit_event(&self, event: crate::event_protocol::EventEnvelope) {
        if let Some(ref tx) = self.event_tx {
            // Non-blocking send. EventSender records any loss by event kind.
            if let Err(error) = tx.try_send(event) {
                tracing::debug!(?error, "non-blocking lifecycle event delivery failed");
            }
        }
    }

    // -- Accessors (mirrors old Engine field accessors) --

    pub fn enforce_target(&self, target: &str) -> PyResult<()> {
        self.scope.enforce_target(target)
    }

    pub fn enforce_port(&self, port: u16) -> PyResult<()> {
        self.scope.enforce_port(port)
    }

    pub fn scope_ref(&self) -> &Scope {
        &self.scope
    }

    pub fn get_concurrency(&self) -> usize {
        self.concurrency
    }

    pub fn get_timeout_ms(&self) -> u64 {
        self.timeout_ms
    }

    pub fn get_mode(&self) -> &str {
        &self.mode
    }

    // -- Pre-dispatch validation --

    /// Evaluate the mandatory structured policy gate before dispatch.
    pub fn pre_dispatch_validate(
        &self,
        operation: &str,
        target: &str,
    ) -> PyResult<DispatchDecision> {
        let canonical =
            crate::operation_registry::StableOperation::parse(operation).ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!("Unknown operation: {}", operation))
            })?;
        let descriptor = eggsec::config::OperationDescriptor {
            operation: canonical.id().replace('_', "-"),
            mode: eggsec::config::OperationMode::StandardAssessment,
            risk: match canonical {
                crate::operation_registry::StableOperation::ScanPorts
                | crate::operation_registry::StableOperation::FingerprintServices
                | crate::operation_registry::StableOperation::ReconDns
                | crate::operation_registry::StableOperation::InspectTls => {
                    eggsec::config::OperationRisk::Passive
                }
                crate::operation_registry::StableOperation::LoadTest => {
                    eggsec::config::OperationRisk::LoadTest
                }
                _ => eggsec::config::OperationRisk::SafeActive,
            },
            intended_uses: vec![eggsec::config::IntendedUse::WebAssessment],
            target: Some(target.to_string()),
            required_features: self
                .registry
                .get(operation)
                .and_then(|info| info.feature_required)
                .into_iter()
                .collect(),
            required_policy_flags: vec!["require_explicit_scope".to_string()],
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: Vec::new(),
        };
        let outcome = self.enforcement.evaluate(&descriptor);
        let (outcome_name, allowed) = match &outcome {
            eggsec::config::EnforcementOutcome::Allow(_) => ("allow", true),
            eggsec::config::EnforcementOutcome::Warn(_) => ("warn", true),
            eggsec::config::EnforcementOutcome::RequireConfirmation(_) => ("confirm", false),
            eggsec::config::EnforcementOutcome::Deny(_) => ("deny", false),
        };
        let decision = outcome.decision();
        let summary = decision.to_human_readable();
        let event = DispatchAuditEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp_ms: chrono::Utc::now().timestamp_millis().max(0) as u64,
            operation_id: canonical.id().to_string(),
            target: target.to_string(),
            surface: self.enforcement.execution_profile.to_string(),
            outcome: outcome_name.to_string(),
            allowed,
            decision_summary: summary.clone(),
            redacted: true,
        };
        let event_id = event.event_id.clone();
        if let Ok(mut events) = self.audit_events.lock() {
            events.push(event);
        }
        tracing::info!(
            operation,
            target,
            outcome = outcome_name,
            allowed,
            "structured dispatch decision"
        );
        if !allowed {
            return Err(crate::error::EnforcementError::new_err(summary));
        }
        Ok(DispatchDecision {
            audit_event_id: event_id,
            summary,
        })
    }

    pub fn audit_events(&self) -> Vec<DispatchAuditEvent> {
        self.audit_events
            .lock()
            .map(|events| events.clone())
            .unwrap_or_default()
    }
}
