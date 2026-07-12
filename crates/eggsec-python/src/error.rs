use pyo3::prelude::*;

pyo3::create_exception!(eggsec._core, EggsecError, pyo3::exceptions::PyException);
pyo3::create_exception!(eggsec._core, ConfigError, EggsecError);
pyo3::create_exception!(eggsec._core, ScopeError, EggsecError);
pyo3::create_exception!(eggsec._core, EnforcementError, EggsecError);
pyo3::create_exception!(eggsec._core, NetworkError, EggsecError);
pyo3::create_exception!(eggsec._core, ScanError, EggsecError);
pyo3::create_exception!(eggsec._core, TimeoutError, EggsecError);
pyo3::create_exception!(eggsec._core, FeatureUnavailableError, EggsecError);
pyo3::create_exception!(eggsec._core, SerializationError, EggsecError);
pyo3::create_exception!(eggsec._core, InternalError, EggsecError);
pyo3::create_exception!(eggsec._core, CancellationError, EggsecError);

/// Reconstruct the documented Python exception from a structured operation
/// error. This mapping is shared by sync, async, and daemon-facing results.
pub(crate) fn operation_error_to_pyerr(error: &crate::status::OperationError) -> PyErr {
    match error.kind.as_str() {
        "validation" | "configuration" => ConfigError::new_err(error.message.clone()),
        "scope_denial" => ScopeError::new_err(error.message.clone()),
        "policy_denial" | "capability_unavailable" | "privilege_missing" => {
            EnforcementError::new_err(error.message.clone())
        }
        "feature_unavailable" => FeatureUnavailableError::new_err(error.message.clone()),
        "network" | "daemon_transport" => NetworkError::new_err(error.message.clone()),
        "timeout" => TimeoutError::new_err(error.message.clone()),
        "cancellation" => CancellationError::new_err(error.message.clone()),
        "serialization" | "parsing" => SerializationError::new_err(error.message.clone()),
        "scan" => ScanError::new_err(error.message.clone()),
        _ => InternalError::new_err(error.message.clone()),
    }
}

/// Convert engine EggsecError to Python exception.
///
/// Since we can't implement `From` due to orphan rules, we use this as a helper.
pub(crate) fn engine_error_to_pyerr(err: eggsec::error::EggsecError) -> PyErr {
    use eggsec::error::EggsecError as E;
    match err {
        E::Config(msg) => ConfigError::new_err(msg),
        E::InvalidTarget(msg) => EnforcementError::new_err(msg),
        E::Network(msg) => NetworkError::new_err(msg),
        E::RequestFailed { method, url, error } => {
            NetworkError::new_err(format!("{} {} - {}", method, url, error))
        }
        E::Timeout {
            timeout_ms,
            operation,
        } => TimeoutError::new_err(format!("Timeout after {}ms: {}", timeout_ms, operation)),
        E::RateLimited(msg) => NetworkError::new_err(format!("Rate limited: {}", msg)),
        E::ScanFailed { stage, error } => ScanError::new_err(format!("{} - {}", stage, error)),
        E::Payload(msg) => ScanError::new_err(format!("Payload error: {}", msg)),
        E::Output(msg) => ScanError::new_err(format!("Output error: {}", msg)),
        E::Internal(msg) => InternalError::new_err(msg),
        E::ScopeViolation(msg) => EnforcementError::new_err(msg),
        E::Io(e) => ScanError::new_err(format!("IO error: {}", e)),
        E::HttpStatus { status, message } => {
            NetworkError::new_err(format!("HTTP {} - {}", status, message))
        }
        E::Http(msg) => NetworkError::new_err(msg),
        E::Parse(msg) => SerializationError::new_err(msg),
        E::Validation(msg) => ConfigError::new_err(msg),
        E::AddressParse(msg) => NetworkError::new_err(msg),
        E::Runtime(msg) => ScanError::new_err(msg),
        E::Cancelled => ScanError::new_err("Operation cancelled"),
        E::Proxy(msg) => ScanError::new_err(msg),
        E::Recon(msg) => ScanError::new_err(msg),
        E::LoadTest(msg) => ScanError::new_err(msg),
        E::Fingerprint(msg) => ScanError::new_err(msg),
    }
}

/// Extension trait for converting `Result<T, EggsecError>` to `PyResult<T>`.
pub(crate) trait EggsecResultExt<T> {
    fn map_pyerr(self) -> PyResult<T>;
}

impl<T> EggsecResultExt<T> for Result<T, eggsec::error::EggsecError> {
    fn map_pyerr(self) -> PyResult<T> {
        self.map_err(engine_error_to_pyerr)
    }
}
