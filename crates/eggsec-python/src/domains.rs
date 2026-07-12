use pyo3::prelude::*;
use std::collections::HashMap;

/// Python binding for [`eggsec::domain::DomainDescriptor`].
///
/// Describes a capability domain — what it can do, how it integrates with
/// CLI/TUI/MCP/tool surfaces, and what feature gates control its availability.
#[pyclass(frozen, name = "DomainDescriptorPy")]
#[derive(Clone)]
pub struct DomainDescriptorPy {
    id: String,
    display_name: String,
    description: String,
    category: String,
    required_feature: Option<String>,
    operations: Vec<String>,
    is_available: bool,
}

#[pymethods]
impl DomainDescriptorPy {
    /// Unique domain identifier (e.g. "db-pentest", "mobile-static").
    #[getter]
    fn id(&self) -> String {
        self.id.clone()
    }

    /// Human-readable display name.
    #[getter]
    fn display_name(&self) -> String {
        self.display_name.clone()
    }

    /// Brief description of the domain's purpose.
    #[getter]
    fn description(&self) -> String {
        self.description.clone()
    }

    /// Classification category (e.g. "standard-assessment", "defense-lab").
    #[getter]
    fn category(&self) -> String {
        self.category.clone()
    }

    /// Cargo feature flag required to compile this domain (None if always available).
    #[getter]
    fn required_feature(&self) -> Option<String> {
        self.required_feature.clone()
    }

    /// Operation IDs provided by this domain.
    #[getter]
    fn operations(&self) -> Vec<String> {
        self.operations.clone()
    }

    /// Whether this domain is available given the current compile-time feature set.
    #[getter]
    fn is_available(&self) -> bool {
        self.is_available
    }

    /// Serialize all fields to a Python dict.
    fn to_dict(&self) -> HashMap<String, pyo3::PyObject> {
        Python::with_gil(|py| {
            let mut d = HashMap::new();
            d.insert("id".into(), self.id.clone().into_py(py));
            d.insert("display_name".into(), self.display_name.clone().into_py(py));
            d.insert("description".into(), self.description.clone().into_py(py));
            d.insert("category".into(), self.category.clone().into_py(py));
            d.insert(
                "required_feature".into(),
                self.required_feature.clone().into_py(py),
            );
            d.insert("operations".into(), self.operations.clone().into_py(py));
            d.insert("is_available".into(), self.is_available.into_py(py));
            d
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "DomainDescriptorPy(id={}, name={:?})",
            self.id, self.display_name
        )
    }

    fn __str__(&self) -> String {
        self.display_name.clone()
    }
}

/// Build a `DomainDescriptorPy` from an engine `DomainDescriptor` reference.
fn build_domain_view(domain: &eggsec::domain::DomainDescriptor) -> DomainDescriptorPy {
    let operations = domain
        .operations
        .iter()
        .map(|op| op.operation_id.to_string())
        .collect();

    DomainDescriptorPy {
        id: domain.id.to_string(),
        display_name: domain.display_name.to_string(),
        description: domain.description.to_string(),
        category: domain.category.to_string(),
        required_feature: domain.required_feature.map(|s| s.to_string()),
        operations,
        is_available: domain.is_available(),
    }
}

/// Registry of domain descriptors.
///
/// Provides static methods to query the canonical domain metadata registry.
#[pyclass]
pub struct DomainRegistry;

#[pymethods]
impl DomainRegistry {
    /// All known domain descriptors, regardless of feature availability.
    ///
    /// Returns:
    ///     list[DomainDescriptorPy]: All domain descriptors.
    #[staticmethod]
    fn all_domains() -> Vec<DomainDescriptorPy> {
        eggsec::domain::all_domain_descriptors()
            .iter()
            .map(build_domain_view)
            .collect()
    }

    /// Only domains whose required feature is currently compiled.
    ///
    /// Returns:
    ///     list[DomainDescriptorPy]: Available domain descriptors.
    #[staticmethod]
    fn available_domains() -> Vec<DomainDescriptorPy> {
        eggsec::domain::available_domain_descriptors()
            .iter()
            .map(|d| build_domain_view(d))
            .collect()
    }

    /// Find a domain descriptor by its ID.
    ///
    /// Args:
    ///     domain_id: The domain identifier (e.g. "db-pentest", "mobile-static").
    ///
    /// Returns:
    ///     DomainDescriptorPy | None: The domain descriptor, or None if not found.
    #[staticmethod]
    fn find(domain_id: &str) -> Option<DomainDescriptorPy> {
        let d = eggsec::domain::domain_descriptor_by_id(domain_id)?;
        Some(build_domain_view(d))
    }

    fn __repr__(&self) -> String {
        "DomainRegistry".to_string()
    }
}
