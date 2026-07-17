use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::error::EggsecResultExt;
use crate::finding::Severity;
use crate::runtime_async;
use crate::runtime_sync;

/// SBOM output format.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SbomFormatPy {
    CycloneDx,
    Spdx,
}

#[pymethods]
impl SbomFormatPy {
    fn __repr__(&self) -> String {
        format!("SbomFormat.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }

    #[staticmethod]
    fn from_str(s: &str) -> PyResult<Self> {
        match s.to_lowercase().as_str() {
            "cyclonedx" | "cyclone-dx" | "cyclone_dx" => Ok(SbomFormatPy::CycloneDx),
            "spdx" => Ok(SbomFormatPy::Spdx),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid SBOM format: '{}'. Must be one of: cyclonedx, spdx",
                s
            ))),
        }
    }
}

impl SbomFormatPy {
    fn as_str(&self) -> &str {
        match self {
            SbomFormatPy::CycloneDx => "CycloneDx",
            SbomFormatPy::Spdx => "Spdx",
        }
    }

    pub fn from_engine(engine: eggsec::supply_chain::sbom::SbomFormat) -> Self {
        match engine {
            eggsec::supply_chain::sbom::SbomFormat::CycloneDx => SbomFormatPy::CycloneDx,
            eggsec::supply_chain::sbom::SbomFormat::Spdx => SbomFormatPy::Spdx,
        }
    }

    pub fn to_engine(&self) -> eggsec::supply_chain::sbom::SbomFormat {
        match self {
            SbomFormatPy::CycloneDx => eggsec::supply_chain::sbom::SbomFormat::CycloneDx,
            SbomFormatPy::Spdx => eggsec::supply_chain::sbom::SbomFormat::Spdx,
        }
    }

}

/// A single component in the SBOM.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbomComponentPy {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub version: String,
    #[pyo3(get)]
    pub ecosystem: String,
    #[pyo3(get)]
    pub purl: String,
    licenses: Vec<String>,
    #[pyo3(get)]
    pub is_direct: bool,
}

impl SbomComponentPy {
    pub fn from_engine(engine: eggsec::supply_chain::sbom::SbomComponent) -> Self {
        Self {
            name: engine.name,
            version: engine.version,
            ecosystem: engine.ecosystem,
            purl: engine.purl,
            licenses: engine.licenses,
            is_direct: engine.is_direct,
        }
    }
}

#[pymethods]
impl SbomComponentPy {
    #[getter]
    fn licenses(&self) -> Vec<String> {
        self.licenses.clone()
    }

    /// Convert to a Python dictionary.
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("name", &self.name)?;
        dict.set_item("version", &self.version)?;
        dict.set_item("ecosystem", &self.ecosystem)?;
        dict.set_item("purl", &self.purl)?;
        dict.set_item("licenses", &self.licenses)?;
        dict.set_item("is_direct", self.is_direct)?;
        Ok(dict.into())
    }

    /// Convert to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "SbomComponent(name={}, version={}, ecosystem={})",
            self.name, self.version, self.ecosystem
        )
    }

    fn __str__(&self) -> String {
        format!("{}@{} ({})", self.name, self.version, self.ecosystem)
    }
}

/// A vulnerability associated with an SBOM component.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbomVulnerabilityPy {
    #[pyo3(get)]
    pub component: String,
    #[pyo3(get)]
    pub cve_id: String,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub description: String,
}

impl SbomVulnerabilityPy {
    pub fn from_engine(engine: eggsec::supply_chain::sbom::SbomVulnerability) -> Self {
        Self {
            component: engine.component,
            cve_id: engine.cve_id,
            severity: Severity::from_engine(engine.severity),
            description: engine.description,
        }
    }
}

#[pymethods]
impl SbomVulnerabilityPy {
    /// Convert to a Python dictionary.
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("component", &self.component)?;
        dict.set_item("cve_id", &self.cve_id)?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("description", &self.description)?;
        Ok(dict.into())
    }

    /// Convert to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "SbomVulnerability(cve={}, severity={}, component={})",
            self.cve_id,
            self.severity.as_str(),
            self.component
        )
    }

    fn __str__(&self) -> String {
        format!(
            "[{}] {} - {}",
            self.severity.as_str(),
            self.cve_id,
            self.component
        )
    }
}

/// Full SBOM report with components and vulnerabilities.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbomReportPy {
    #[pyo3(get)]
    pub format: SbomFormatPy,
    #[pyo3(get)]
    pub project_name: String,
    #[pyo3(get)]
    pub version: String,
    #[pyo3(get)]
    pub generated_at: String,
    components: Vec<SbomComponentPy>,
    vulnerabilities: Vec<SbomVulnerabilityPy>,
}

#[pymethods]
impl SbomReportPy {
    #[getter]
    fn components(&self) -> Vec<SbomComponentPy> {
        self.components.clone()
    }

    #[getter]
    fn vulnerabilities(&self) -> Vec<SbomVulnerabilityPy> {
        self.vulnerabilities.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("format", self.format.as_str())?;
        dict.set_item("project_name", &self.project_name)?;
        dict.set_item("version", &self.version)?;
        dict.set_item("generated_at", &self.generated_at)?;

        let components_list = PyList::empty_bound(py);
        for c in &self.components {
            components_list.append(c.to_dict(py)?)?;
        }
        dict.set_item("components", components_list)?;

        let vulns_list = PyList::empty_bound(py);
        for v in &self.vulnerabilities {
            vulns_list.append(v.to_dict(py)?)?;
        }
        dict.set_item("vulnerabilities", vulns_list)?;
        Ok(dict.into())
    }

    /// Convert to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "SbomReport(project={}, format={}, components={}, vulns={})",
            self.project_name,
            self.format.as_str(),
            self.components.len(),
            self.vulnerabilities.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "SBOM for '{}' v{}: {} components, {} vulnerabilities ({})",
            self.project_name,
            self.version,
            self.components.len(),
            self.vulnerabilities.len(),
            self.format.as_str()
        )
    }
}

/// Generate a Software Bill of Materials for a project.
///
/// Supports three ecosystems:
/// - "cargo": Rust projects (reads Cargo.toml / Cargo.lock)
/// - "npm": Node.js projects (reads package.json / package-lock.json)
/// - "pip": Python projects (reads requirements.txt)
///
/// Args:
///     project_path: Path to the project root directory.
///     ecosystem: Package ecosystem ("cargo", "npm", or "pip").
///     format: Output format ("cyclonedx" or "spdx").
///
/// Returns:
///     SbomReportPy: Full SBOM report with components and vulnerabilities.
///
/// Raises:
///     ConfigError: If the ecosystem is unsupported or project files are missing.
///     ScanError: If generation fails.
#[pyfunction]
#[pyo3(signature = (project_path, *, ecosystem="cargo", format="cyclonedx"))]
pub fn generate_sbom(project_path: &str, ecosystem: &str, format: &str) -> PyResult<SbomReportPy> {
    let sbom_format = SbomFormatPy::from_str(format)?;
    let project_path_owned = project_path.to_string();
    let ecosystem_owned = ecosystem.to_string();

    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            let gen = eggsec::supply_chain::sbom::SbomGenerator::new();
            let engine_format = sbom_format.to_engine();
            Ok(match ecosystem_owned.as_str() {
                "cargo" => gen.generate_from_cargo(&project_path_owned, engine_format),
                "npm" => gen.generate_from_npm(&project_path_owned, engine_format),
                "pip" => gen.generate_from_requirements(&project_path_owned, engine_format),
                other => {
                    return Err(eggsec::error::EggsecError::Config(format!(
                        "Unsupported ecosystem: '{}'. Must be one of: cargo, npm, pip",
                        other
                    )))
                }
            })
        })?;

        let result = result.map_pyerr()?;

        Ok(SbomReportPy {
            format: SbomFormatPy::from_engine(result.format),
            project_name: result.project_name,
            version: result.version,
            generated_at: result.generated_at,
            components: result
                .components
                .into_iter()
                .map(SbomComponentPy::from_engine)
                .collect(),
            vulnerabilities: result
                .vulnerabilities
                .into_iter()
                .map(SbomVulnerabilityPy::from_engine)
                .collect(),
        })
    })
}

/// Perform async SBOM generation.
#[pyfunction]
#[pyo3(signature = (project_path, *, ecosystem="cargo", format="cyclonedx"))]
pub fn async_generate_sbom(
    project_path: &str,
    ecosystem: &str,
    format: &str,
) -> PyResult<crate::runtime_async::PyFuture> {
    let sbom_format = SbomFormatPy::from_str(format)?;
    let project_path_owned = project_path.to_string();
    let ecosystem_owned = ecosystem.to_string();

    runtime_async::spawn_async(async move {
        let gen = eggsec::supply_chain::sbom::SbomGenerator::new();
        let engine_format = sbom_format.to_engine();
        let result = match ecosystem_owned.as_str() {
            "cargo" => gen.generate_from_cargo(&project_path_owned, engine_format),
            "npm" => gen.generate_from_npm(&project_path_owned, engine_format),
            "pip" => gen.generate_from_requirements(&project_path_owned, engine_format),
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Unsupported ecosystem: '{}'. Must be one of: cargo, npm, pip",
                    other
                )))
            }
        }
        .map_pyerr()?;

        Ok(SbomReportPy {
            format: SbomFormatPy::from_engine(result.format),
            project_name: result.project_name,
            version: result.version,
            generated_at: result.generated_at,
            components: result
                .components
                .into_iter()
                .map(SbomComponentPy::from_engine)
                .collect(),
            vulnerabilities: result
                .vulnerabilities
                .into_iter()
                .map(SbomVulnerabilityPy::from_engine)
                .collect(),
        })
    })
}
