use crate::config::{normalize_target, ApprovedOperation, OperationTarget};
use crate::error::EggsecError;
use crate::tool::response::{ResponseMetadata, ResponseStatus};
use crate::tool::{ToolRegistry, ToolRequest, ToolResponse};
use parking_lot::RwLock;
use std::sync::Arc;

/// Error returned by [`validate_request_binding`] when the request does not
/// match the approved operation binding.
#[derive(Debug, thiserror::Error)]
pub enum DispatchBindingError {
    /// The request tool name does not resolve to the approved canonical operation.
    #[error(
        "request tool '{request_tool}' does not match approved operation '{approved_operation}' \
         (alias resolution attempted)"
    )]
    OperationMismatch {
        request_tool: String,
        approved_operation: String,
    },

    /// A target-bearing approval was dispatched with no target in the request.
    #[error(
        "approved operation '{approved_operation}' requires target '{expected_target}' \
         but request has no target"
    )]
    MissingTarget {
        approved_operation: String,
        expected_target: String,
    },

    /// The request target does not match the approved target (after normalization).
    #[error(
        "request target '{request_raw}' normalizes to '{request_normalized}' which differs \
         from approved normalized target '{approved_normalized}'"
    )]
    NormalizedTargetMismatch {
        request_raw: String,
        request_normalized: OperationTarget,
        approved_normalized: OperationTarget,
    },

    /// A targetless approval received a request with a target that would alter scope.
    #[error(
        "approved operation '{approved_operation}' is targetless but request has target \
         '{request_target}' — scope-escaping target rejected"
    )]
    UnexpectedTarget {
        approved_operation: String,
        request_target: String,
    },

    /// The request contains conflicting typed and parameter targets.
    #[error(
        "conflicting targets: request target '{typed_target}' differs from \
         params['target'] '{param_target}'"
    )]
    ConflictingTargets {
        typed_target: String,
        param_target: String,
    },

    /// The approval surface does not match the context profile.
    #[error("dispatch binding: {reason}")]
    Other { reason: String },
}

/// Validate that a [`ToolRequest`] matches the binding in an [`ApprovedOperation`].
///
/// This is the single comparison function for dispatch-time binding checks.
/// It verifies:
///
/// 1. The request tool resolves to the approved canonical operation.
/// 2. A target-bearing approval has a non-empty target in the request.
/// 3. The request target normalizes to the same identity as the approved target.
/// 4. A targetless approval does not receive a target that would alter scope.
/// 5. Typed and parameter targets agree (no conflicting targets).
///
/// Returns `Ok(())` if the binding is valid, or [`DispatchBindingError`] on mismatch.
pub fn validate_request_binding(
    approval: &ApprovedOperation,
    request: &ToolRequest,
) -> Result<(), DispatchBindingError> {
    let descriptor = approval.descriptor();

    // 1. Operation ID match (with alias resolution).
    if !crate::config::operation_matches_tool_id(&request.tool, &descriptor.operation) {
        return Err(DispatchBindingError::OperationMismatch {
            request_tool: request.tool.clone(),
            approved_operation: descriptor.operation.clone(),
        });
    }

    let request_target = if request.target.value.is_empty() {
        None
    } else {
        Some(request.target.value.as_str())
    };
    let param_target = request
        .params
        .get("target")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());

    // 5. Check for conflicting targets (typed vs parameter).
    if let (Some(typed), Some(param)) = (request_target, param_target) {
        if typed != param {
            return Err(DispatchBindingError::ConflictingTargets {
                typed_target: typed.to_string(),
                param_target: param.to_string(),
            });
        }
    }

    let effective_target = request_target.or(param_target);

    match &descriptor.normalized_target {
        OperationTarget::None => {
            // 4. Targetless approval rejects scope-escaping targets.
            if let Some(actual) = effective_target {
                return Err(DispatchBindingError::UnexpectedTarget {
                    approved_operation: descriptor.operation.clone(),
                    request_target: actual.to_string(),
                });
            }
        }
        expected_normalized => {
            // 2. Target-bearing approval requires a target.
            let actual_raw =
                effective_target.ok_or_else(|| DispatchBindingError::MissingTarget {
                    approved_operation: descriptor.operation.clone(),
                    expected_target: descriptor.target.clone().unwrap_or_default(),
                })?;

            // 3. Normalize request target and compare against approved normalized target.
            let request_normalized = normalize_target(actual_raw, None);
            if request_normalized != *expected_normalized {
                return Err(DispatchBindingError::NormalizedTargetMismatch {
                    request_raw: actual_raw.to_string(),
                    request_normalized,
                    approved_normalized: expected_normalized.clone(),
                });
            }
        }
    }

    Ok(())
}

#[derive(Clone)]
pub struct ToolDispatcher {
    registry: ToolRegistry,
    history: Arc<RwLock<Option<super::history::ExecutionHistory>>>,
}

impl ToolDispatcher {
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry,
            history: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_history(self, history: super::history::ExecutionHistory) -> Self {
        *self.history.write() = Some(history);
        Self {
            registry: self.registry,
            history: Arc::clone(&self.history),
        }
    }

    pub fn history(&self) -> Option<super::history::ExecutionHistory> {
        self.history.read().clone()
    }

    /// Raw dispatch — prefer `EnforcedDispatcher::dispatch_checked()` for strict surfaces.
    #[doc(hidden)]
    pub(crate) async fn dispatch(&self, request: ToolRequest) -> Result<ToolResponse, EggsecError> {
        if request.is_cancelled() {
            return Err(EggsecError::Cancelled);
        }

        let tool = self
            .registry
            .get(&request.tool)
            .ok_or_else(|| EggsecError::Config(format!("Tool '{}' not found", request.tool)))?;

        tool.validate(&request)?;

        let started_at = chrono::Utc::now();
        let result = tool.execute(request.clone()).await;
        let completed_at = chrono::Utc::now();

        let response = match &result {
            Ok(resp) => resp.clone(),
            Err(_) => ToolResponse {
                request_id: request.id.clone(),
                tool_id: request.tool.clone(),
                status: ResponseStatus::Failed,
                results: serde_json::json!({}),
                metadata: ResponseMetadata {
                    started_at,
                    completed_at,
                    duration_ms: (completed_at - started_at).num_milliseconds().max(0) as u64,
                    targets_scanned: 0,
                    findings_count: 0,
                },
                errors: vec![],
                findings: vec![],
            },
        };

        if let Some(ref history) = *self.history.read() {
            let capability = request
                .params
                .get("_capability")
                .and_then(|v| v.as_str())
                .map(String::from);
            history.record(&request, &response, capability);
        }

        result
    }

    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }
}

impl Default for ToolDispatcher {
    fn default() -> Self {
        Self::new(ToolRegistry::new())
    }
}

/// Wrapper around [`ToolDispatcher`] that requires an [`ApprovedOperation`]
/// token before dispatching. This enforces type-level access control so
/// strict programmatic surfaces cannot accidentally bypass policy.
#[derive(Clone)]
pub struct EnforcedDispatcher {
    inner: ToolDispatcher,
}

impl EnforcedDispatcher {
    pub fn new(inner: ToolDispatcher) -> Self {
        Self { inner }
    }

    /// Dispatch a tool request, verifying it matches the approved operation.
    ///
    /// Uses [`validate_request_binding`] to verify:
    /// - The tool name resolves to the approved canonical operation.
    /// - Target-bearing approvals have matching targets.
    /// - Targetless approvals don't receive scope-escaping targets.
    /// - Typed and parameter targets agree.
    ///
    /// Fails closed on any mismatch.
    pub async fn dispatch_checked(
        &self,
        approved: &ApprovedOperation,
        request: ToolRequest,
    ) -> Result<ToolResponse, EggsecError> {
        validate_request_binding(approved, &request)
            .map_err(|e| EggsecError::Config(format!("dispatch binding failed: {e}")))?;

        self.inner.dispatch(request).await
    }

    /// Access the underlying dispatcher (for cases where the caller has
    /// already obtained an approval token through another path).
    pub fn inner(&self) -> &ToolDispatcher {
        &self.inner
    }

    pub fn with_history(self, history: super::history::ExecutionHistory) -> Self {
        Self {
            inner: self.inner.with_history(history),
        }
    }

    pub fn history(&self) -> Option<super::history::ExecutionHistory> {
        self.inner.history()
    }

    pub fn registry(&self) -> &ToolRegistry {
        self.inner.registry()
    }
}
