use std::sync::Arc;

use pyo3::prelude::*;

use crate::config_model::PyEggsecConfig;
use crate::features;
use crate::operation_registry::OperationExecutorRegistry;
use crate::scope::Scope;

/// Optional event channel sender for emitting lifecycle events from operations.
pub type EventSender = tokio::sync::mpsc::Sender<crate::event_protocol::EventEnvelope>;

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
        Ok(Arc::new(Self {
            scope,
            mode: mode.to_string(),
            concurrency,
            timeout_ms,
            config: PyEggsecConfig::new_default(),
            registry: OperationExecutorRegistry::default_stable(),
            event_tx: None,
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
        Ok(Arc::new(Self {
            scope,
            mode: mode.to_string(),
            concurrency: concurrency.unwrap_or(default_concurrency),
            timeout_ms: timeout_ms.unwrap_or(5000),
            config,
            registry: OperationExecutorRegistry::default_stable(),
            event_tx: None,
        }))
    }

    /// Set the event channel sender for lifecycle event emission.
    pub fn set_event_tx(&mut self, tx: EventSender) {
        self.event_tx = Some(tx);
    }

    /// Emit a lifecycle event if an event channel is configured.
    ///
    /// Non-blocking: drops the event if the channel is full (backpressure).
    pub fn emit_event(&self, event: crate::event_protocol::EventEnvelope) {
        if let Some(ref tx) = self.event_tx {
            // Non-blocking send — drop event on backpressure rather than blocking
            let _ = tx.try_send(event);
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

    /// Validate an operation before dispatch.
    ///
    /// Checks:
    /// 1. Target is within the allowed scope.
    /// 2. Feature requirements are satisfied.
    /// 3. Logs an audit event for the operation attempt.
    ///
    /// Returns `Ok(())` on success, or `Err(PyErr)` with a descriptive message.
    pub fn pre_dispatch_validate(&self, operation: &str, target: &str) -> PyResult<()> {
        // 1. Scope enforcement — target must be in allowed scope.
        self.scope.enforce_target(target).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Scope violation for operation '{}': {}",
                operation, e
            ))
        })?;

        // 2. Feature gate — check if the operation's required feature is compiled.
        if let Some(info) = self.registry.get(operation) {
            if let Some(ref required_feature) = info.feature_required {
                if !features::has_feature(required_feature) {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Operation '{}' requires feature '{}' which is not compiled",
                        operation, required_feature
                    )));
                }
            }
        }

        // 3. Audit event — log the pre-dispatch validation.
        tracing::info!(
            operation = operation,
            target = target,
            mode = %self.mode,
            concurrency = self.concurrency,
            timeout_ms = self.timeout_ms,
            "pre_dispatch_validate: operation approved for execution"
        );

        Ok(())
    }
}
