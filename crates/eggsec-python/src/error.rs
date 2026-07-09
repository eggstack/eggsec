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
