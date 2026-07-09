use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

/// Evidence for a service fingerprint match.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintEvidence {
    #[pyo3(get)]
    pub probe: String,
    #[pyo3(get)]
    pub pattern: String,
    #[pyo3(get)]
    pub matched: bool,
}

#[pymethods]
impl FingerprintEvidence {
    fn __repr__(&self) -> String {
        format!(
            "FingerprintEvidence(probe={}, matched={})",
            self.probe, self.matched
        )
    }
}

/// Confidence level for a service fingerprint.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintConfidence {
    #[pyo3(get)]
    pub score: u8,
    #[pyo3(get)]
    pub level: String,
}

#[pymethods]
impl FingerprintConfidence {
    fn __repr__(&self) -> String {
        format!(
            "FingerprintConfidence(score={}, level={})",
            self.score, self.level
        )
    }

    fn __str__(&self) -> String {
        format!("{}% ({})", self.score, self.level)
    }
}

/// A single service fingerprint from a scan.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceFingerprintResult {
    #[pyo3(get)]
    pub port: u16,
    #[pyo3(get)]
    pub service: String,
    #[pyo3(get)]
    pub banner: Option<String>,
    #[pyo3(get)]
    pub version: Option<String>,
    #[pyo3(get)]
    pub product: Option<String>,
    #[pyo3(get)]
    pub extra: Option<String>,
    #[pyo3(get)]
    pub confidence: u8,
}

#[pymethods]
impl ServiceFingerprintResult {
    /// Convert to a Python dictionary.
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("port", self.port)?;
        dict.set_item("service", &self.service)?;
        dict.set_item("banner", &self.banner)?;
        dict.set_item("version", &self.version)?;
        dict.set_item("product", &self.product)?;
        dict.set_item("extra", &self.extra)?;
        dict.set_item("confidence", self.confidence)?;
        Ok(dict.into())
    }

    /// Convert to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ServiceFingerprintResult(port={}, service={})",
            self.port, self.service
        )
    }

    fn __str__(&self) -> String {
        let version_str = match (&self.product, &self.version) {
            (Some(p), Some(v)) => format!("{} {}", p, v),
            (Some(p), None) => p.clone(),
            (None, Some(v)) => v.clone(),
            (None, None) => String::new(),
        };
        if version_str.is_empty() {
            format!("{}/tcp - {}", self.port, self.service)
        } else {
            format!("{}/tcp - {} ({})", self.port, self.service, version_str)
        }
    }
}

/// Result of a service fingerprinting scan.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintScanResult {
    #[pyo3(get)]
    pub target: String,
    services: Vec<ServiceFingerprintResult>,
    #[pyo3(get)]
    pub services_identified: usize,
    #[pyo3(get)]
    pub elapsed_ms: u64,
}

#[pymethods]
impl FingerprintScanResult {
    /// Returns the list of service fingerprints found.
    #[getter]
    fn services(&self) -> Vec<ServiceFingerprintResult> {
        self.services.clone()
    }

    /// Convert result to a Python dictionary.
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("services_identified", self.services_identified)?;
        dict.set_item("elapsed_ms", self.elapsed_ms)?;

        let services_list = PyList::empty_bound(py);
        for s in &self.services {
            let s_dict = PyDict::new_bound(py);
            s_dict.set_item("port", s.port)?;
            s_dict.set_item("service", &s.service)?;
            s_dict.set_item("banner", &s.banner)?;
            s_dict.set_item("version", &s.version)?;
            s_dict.set_item("product", &s.product)?;
            s_dict.set_item("extra", &s.extra)?;
            s_dict.set_item("confidence", s.confidence)?;
            services_list.append(s_dict)?;
        }
        dict.set_item("services", services_list)?;

        Ok(dict.into())
    }

    /// Convert result to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "FingerprintScanResult(target={}, identified={})",
            self.target, self.services_identified
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Fingerprint of {}: {} services identified in {}ms",
            self.target, self.services_identified, self.elapsed_ms
        )
    }

    /// Convert service fingerprints to a list of row dicts suitable for tabular output.
    fn to_rows(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for s in &self.services {
            let dict = PyDict::new_bound(py);
            dict.set_item("target", &self.target)?;
            dict.set_item("port", s.port)?;
            dict.set_item("service", &s.service)?;
            dict.set_item("banner", &s.banner)?;
            dict.set_item("version", &s.version)?;
            dict.set_item("product", &s.product)?;
            dict.set_item("extra", &s.extra)?;
            dict.set_item("confidence", s.confidence)?;
            list.append(dict)?;
        }
        Ok(list.into())
    }
}

impl FingerprintScanResult {
    /// Convert from engine FingerprintResults.
    pub fn from_engine(engine: eggsec::scanner::FingerprintResults) -> Self {
        let services: Vec<ServiceFingerprintResult> = engine
            .results
            .into_iter()
            .map(|r| ServiceFingerprintResult {
                port: r.port,
                service: r.service,
                banner: r.banner,
                version: r.version,
                product: r.product,
                extra: r.extra,
                confidence: r.confidence,
            })
            .collect();

        Self {
            target: engine.host,
            services_identified: engine.services_identified,
            elapsed_ms: engine.duration_ms,
            services,
        }
    }
}
