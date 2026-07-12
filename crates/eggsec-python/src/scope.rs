use pyo3::prelude::*;
use std::path::PathBuf;

use crate::error::{EnforcementError, ScopeError};

/// PathLike helper — accepts str or pathlib.Path.
///
/// Tries str extraction first, then falls back to `path.__fspath__()`.
fn resolve_path_arg(path: &Bound<'_, PyAny>) -> PyResult<PathBuf> {
    // Try str first
    if let Ok(s) = path.extract::<String>() {
        return Ok(PathBuf::from(s));
    }
    // Try pathlib.Path via __fspath__()
    let fspath = path.getattr("__fspath__")?;
    let result = fspath.call0()?;
    let pystr = result.downcast::<pyo3::types::PyString>().map_err(|_| {
        pyo3::exceptions::PyTypeError::new_err("__fspath__() must return str or bytes")
    })?;
    let s = pystr
        .to_str()
        .map_err(|_| pyo3::exceptions::PyUnicodeDecodeError::new_err("path is not valid UTF-8"))?;
    Ok(PathBuf::from(s))
}

/// Python wrapper for Eggsec scope enforcement.
///
/// Controls which targets and ports are authorized for scanning.
/// Scope violations raise `EnforcementError`.
#[pyclass(frozen)]
#[derive(Clone)]
pub struct Scope {
    pub(crate) inner: eggsec::config::Scope,
}

#[pymethods]
impl Scope {
    /// Create a scope allowing specific hosts.
    ///
    /// Args:
    ///     hosts: List of hostnames or IPs to allow (e.g. ["example.com", "10.0.0.0/8"]).
    ///         Each entry may be a str or a pathlib.Path (for consistency).
    ///
    /// Returns:
    ///     Scope: A new scope allowing only the specified hosts.
    ///
    /// Raises:
    ///     ValueError: If hosts list is empty.
    #[staticmethod]
    fn allow_hosts(hosts: Vec<String>) -> PyResult<Self> {
        if hosts.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "hosts list must not be empty",
            ));
        }
        let rules: Vec<eggsec::config::ScopeRule> = hosts
            .into_iter()
            .map(|h| {
                if h.contains('/') {
                    eggsec::config::ScopeRule {
                        pattern: String::new(),
                        cidr: Some(h),
                        description: None,
                    }
                } else {
                    eggsec::config::ScopeRule {
                        pattern: h,
                        cidr: None,
                        description: None,
                    }
                }
            })
            .collect();
        let scope = eggsec::config::Scope {
            allowed_targets: rules,
            require_explicit_scope: true,
            ..Default::default()
        };
        Ok(Self { inner: scope })
    }

    /// Create a scope allowing specific CIDR ranges.
    ///
    /// Args:
    ///     cidrs: List of CIDR notation ranges (e.g. ["10.0.0.0/8", "192.168.0.0/16"]).
    ///
    /// Returns:
    ///     Scope: A new scope allowing only the specified CIDR ranges.
    ///
    /// Raises:
    ///     ValueError: If cidrs list is empty.
    #[staticmethod]
    fn allow_cidrs(cidrs: Vec<String>) -> PyResult<Self> {
        if cidrs.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "cidrs list must not be empty",
            ));
        }
        let rules: Vec<eggsec::config::ScopeRule> = cidrs
            .into_iter()
            .map(|c| eggsec::config::ScopeRule {
                pattern: String::new(),
                cidr: Some(c),
                description: None,
            })
            .collect();
        let scope = eggsec::config::Scope {
            allowed_targets: rules,
            require_explicit_scope: true,
            ..Default::default()
        };
        Ok(Self { inner: scope })
    }

    /// Create a scope that denies all targets.
    ///
    /// Returns:
    ///     Scope: A scope that denies all scanning targets.
    #[staticmethod]
    fn deny_all() -> Self {
        let scope = eggsec::config::Scope {
            require_explicit_scope: true,
            ..Default::default()
        };
        Self { inner: scope }
    }

    /// Load a scope from a TOML or YAML file.
    ///
    /// Args:
    ///     path: Path to the scope file. Accepts str or pathlib.Path.
    ///
    /// Returns:
    ///     Scope: The loaded scope.
    ///
    /// Raises:
    ///     ScopeError: If the file cannot be read or parsed.
    #[staticmethod]
    fn from_file(path: &Bound<'_, PyAny>) -> PyResult<Self> {
        let path_buf = resolve_path_arg(path)?;
        let path_str = path_buf
            .to_str()
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Path is not valid UTF-8"))?;
        let scope = eggsec::config::Scope::from_file(path_str)
            .map_err(|e| ScopeError::new_err(e.to_string()))?;
        Ok(Self { inner: scope })
    }

    /// Check if a target is allowed by this scope.
    ///
    /// Args:
    ///     target: Hostname or IP to check.
    ///
    /// Returns:
    ///     bool: True if the target is allowed.
    fn is_target_allowed(&self, target: &str) -> PyResult<bool> {
        self.inner
            .is_target_allowed(target)
            .map_err(|e| ScopeError::new_err(e.to_string()))
    }

    /// Check if a port is allowed by this scope.
    ///
    /// Args:
    ///     port: Port number to check.
    ///
    /// Returns:
    ///     bool: True if the port is allowed.
    fn is_port_allowed(&self, port: u16) -> bool {
        self.inner.is_port_allowed(port)
    }

    fn __repr__(&self) -> String {
        let targets: Vec<String> = self
            .inner
            .allowed_targets
            .iter()
            .map(|r| {
                if let Some(ref cidr) = r.cidr {
                    cidr.clone()
                } else {
                    r.pattern.clone()
                }
            })
            .collect();
        format!("Scope(allow_hosts={:?})", targets)
    }
}

impl Scope {
    /// Validate that a target is within scope, raising EnforcementError if denied.
    pub fn enforce_target(&self, target: &str) -> PyResult<()> {
        let allowed = self
            .inner
            .is_target_allowed(target)
            .map_err(|e| EnforcementError::new_err(e.to_string()))?;
        if !allowed {
            return Err(EnforcementError::new_err(format!(
                "Target '{}' is not within the allowed scope",
                target
            )));
        }
        Ok(())
    }

    /// Validate that a port is within scope, raising EnforcementError if denied.
    pub fn enforce_port(&self, port: u16) -> PyResult<()> {
        if !self.inner.is_port_allowed(port) {
            return Err(EnforcementError::new_err(format!(
                "Port {} is not within the allowed scope",
                port
            )));
        }
        Ok(())
    }
}
