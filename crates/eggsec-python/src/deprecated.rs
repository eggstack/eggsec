use pyo3::prelude::*;

/// A custom DeprecationWarning class for eggsec deprecations.
///
/// Subclass of Python's built-in DeprecationWarning. Used to mark
/// deprecated APIs with eggsec-specific messaging.
#[pyclass(name = "DeprecatedWarning")]
pub struct DeprecatedWarning {
    #[pyo3(get)]
    message: String,
}

#[pymethods]
impl DeprecatedWarning {
    #[new]
    #[pyo3(signature = (msg=None))]
    fn new(msg: Option<String>) -> Self {
        DeprecatedWarning {
            message: msg.unwrap_or_else(|| "This API is deprecated".to_string()),
        }
    }

    fn __repr__(&self) -> String {
        format!("DeprecatedWarning({})", self.message)
    }

    fn __str__(&self) -> String {
        self.message.clone()
    }
}

/// Emits a DeprecationWarning with an optional replacement suggestion.
///
/// This is the Rust-side helper; Python code should use `warnings.warn()` directly
/// for most deprecations.
#[pyfunction]
pub fn deprecated_warning(msg: String, py: Python) -> PyResult<()> {
    let warnings = py.import_bound("warnings")?;
    let exc_type = py.get_type_bound::<DeprecatedWarning>();
    warnings.call_method1("warn", (msg, exc_type))?;
    Ok(())
}
