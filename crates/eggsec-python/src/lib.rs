mod error;
mod features;
mod version;

pub use error::*;
use pyo3::prelude::*;

/// The eggsec Python module.
///
/// Python bindings for the Eggsec security assessment engine.
/// This is a host-language binding over the Rust engine, not an internal plugin runtime.
#[pymodule]
pub fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__version_info__", (0, 1, 0))?;

    // Exceptions
    m.add("EggsecError", m.py().get_type_bound::<EggsecError>())?;
    m.add("ConfigError", m.py().get_type_bound::<ConfigError>())?;
    m.add("ScopeError", m.py().get_type_bound::<ScopeError>())?;
    m.add(
        "EnforcementError",
        m.py().get_type_bound::<EnforcementError>(),
    )?;
    m.add("NetworkError", m.py().get_type_bound::<NetworkError>())?;
    m.add("ScanError", m.py().get_type_bound::<ScanError>())?;
    m.add("TimeoutError", m.py().get_type_bound::<TimeoutError>())?;
    m.add(
        "FeatureUnavailableError",
        m.py().get_type_bound::<FeatureUnavailableError>(),
    )?;
    m.add(
        "SerializationError",
        m.py().get_type_bound::<SerializationError>(),
    )?;
    m.add("InternalError", m.py().get_type_bound::<InternalError>())?;

    // Functions
    m.add_function(wrap_pyfunction!(features::features, m)?)?;
    m.add_function(wrap_pyfunction!(features::has_feature, m)?)?;
    m.add_function(wrap_pyfunction!(version::build_info, m)?)?;

    Ok(())
}
