use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::ScanError;
use crate::finding::Severity;
use crate::runtime_async;
use crate::runtime_async::PyFuture;
use crate::runtime_sync;

/// Mobile platform identifier.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MobilePlatformPy {
    Android,
    Ios,
}

#[pymethods]
impl MobilePlatformPy {
    fn __repr__(&self) -> String {
        format!("MobilePlatform.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl MobilePlatformPy {
    fn as_str(&self) -> &str {
        match self {
            MobilePlatformPy::Android => "Android",
            MobilePlatformPy::Ios => "Ios",
        }
    }

    fn from_engine(engine: eggsec::mobile::MobilePlatform) -> Self {
        match engine {
            eggsec::mobile::MobilePlatform::Android => MobilePlatformPy::Android,
            eggsec::mobile::MobilePlatform::Ios => MobilePlatformPy::Ios,
        }
    }
}

/// A single finding from mobile app static analysis.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileFindingPy {
    #[pyo3(get)]
    pub category: String,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub title: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub recommendation: String,
    #[pyo3(get)]
    pub evidence: Option<String>,
}

impl MobileFindingPy {
    fn from_engine(engine: eggsec::mobile::MobileFinding) -> Self {
        Self {
            category: engine.category,
            severity: Severity::from_engine(engine.severity),
            title: engine.title,
            description: engine.description,
            recommendation: engine.recommendation,
            evidence: engine.evidence,
        }
    }
}

#[pymethods]
impl MobileFindingPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("category", &self.category)?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("title", &self.title)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("recommendation", &self.recommendation)?;
        dict.set_item("evidence", &self.evidence)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "MobileFinding(category={}, severity={}, title={})",
            self.category,
            self.severity.as_str(),
            self.title
        )
    }

    fn __str__(&self) -> String {
        format!(
            "[{}] {} - {}",
            self.severity.as_str(),
            self.category,
            self.title
        )
    }
}

/// Full report from mobile app static analysis (APK or IPA).
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileScanReportPy {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub scan_type: String,
    #[pyo3(get)]
    pub platform: MobilePlatformPy,
    #[pyo3(get)]
    pub app_id: Option<String>,
    #[pyo3(get)]
    pub version: Option<String>,
    #[pyo3(get)]
    pub timestamp: String,
    findings: Vec<MobileFindingPy>,
    recommendations: Vec<String>,
    #[pyo3(get)]
    pub duration_ms: u64,
}

impl MobileScanReportPy {
    fn from_engine(engine: eggsec::mobile::MobileScanReport) -> Self {
        Self {
            target: engine.target,
            scan_type: engine.scan_type,
            platform: MobilePlatformPy::from_engine(engine.platform),
            app_id: engine.app_id,
            version: engine.version,
            timestamp: engine.timestamp,
            findings: engine
                .findings
                .into_iter()
                .map(MobileFindingPy::from_engine)
                .collect(),
            recommendations: engine.recommendations,
            duration_ms: engine.duration_ms,
        }
    }
}

#[pymethods]
impl MobileScanReportPy {
    #[getter]
    fn findings(&self) -> Vec<MobileFindingPy> {
        self.findings.clone()
    }

    #[getter]
    fn recommendations(&self) -> Vec<String> {
        self.recommendations.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("scan_type", &self.scan_type)?;
        dict.set_item("platform", self.platform.as_str())?;
        dict.set_item("app_id", &self.app_id)?;
        dict.set_item("version", &self.version)?;
        dict.set_item("timestamp", &self.timestamp)?;

        let findings_list = PyList::empty_bound(py);
        for f in &self.findings {
            findings_list.append(f.to_dict(py)?)?;
        }
        dict.set_item("findings", findings_list)?;

        let recs_list = PyList::new_bound(py, &self.recommendations);
        dict.set_item("recommendations", recs_list)?;

        dict.set_item("duration_ms", self.duration_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "MobileScanReport(target={}, platform={}, findings={}, duration_ms={})",
            self.target,
            self.platform.as_str(),
            self.findings.len(),
            self.duration_ms
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Mobile scan of '{}' ({}): {} findings, {}ms",
            self.target,
            self.platform.as_str(),
            self.findings.len(),
            self.duration_ms
        )
    }
}

/// Analyze an Android APK file (synchronous).
///
/// Performs static analysis on the APK, inspecting the manifest, permissions,
/// exported components, hardcoded secrets, and other security-relevant properties.
///
/// Args:
///     path: Filesystem path to the APK file.
///
/// Returns:
///     MobileScanReportPy: Full analysis report with findings and recommendations.
///
/// Raises:
///     ScanError: If the file cannot be read or analysis fails.
#[pyfunction]
pub fn analyze_apk(path: &str) -> PyResult<MobileScanReportPy> {
    let path_owned = path.to_string();

    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            let path_ref = Path::new(&path_owned);
            eggsec::mobile::analyze_apk(path_ref)
                .await
                .map_err(|e| ScanError::new_err(format!("Mobile analysis failed: {}", e)))
        })?;

        Ok(MobileScanReportPy::from_engine(result))
    })
}

/// Analyze an Android APK file (asynchronous).
///
/// Returns a PyFuture that can be awaited in Python.
#[pyfunction]
pub fn async_analyze_apk(path: &str) -> PyResult<PyFuture> {
    let path_owned = path.to_string();

    runtime_async::spawn_async(async move {
        let path_ref = Path::new(&path_owned);
        let result = eggsec::mobile::analyze_apk(path_ref)
            .await
            .map_err(|e| ScanError::new_err(format!("Mobile analysis failed: {}", e)))?;

        Ok(MobileScanReportPy::from_engine(result))
    })
}

/// Analyze an iOS IPA file (synchronous).
///
/// Performs static analysis on the IPA, inspecting the Info.plist, entitlements,
/// hardcoded secrets, transport security, and other security-relevant properties.
///
/// Args:
///     path: Filesystem path to the IPA file.
///
/// Returns:
///     MobileScanReportPy: Full analysis report with findings and recommendations.
///
/// Raises:
///     ScanError: If the file cannot be read or analysis fails.
#[pyfunction]
pub fn analyze_ipa(path: &str) -> PyResult<MobileScanReportPy> {
    let path_owned = path.to_string();

    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            let path_ref = Path::new(&path_owned);
            eggsec::mobile::analyze_ipa(path_ref)
                .await
                .map_err(|e| ScanError::new_err(format!("Mobile analysis failed: {}", e)))
        })?;

        Ok(MobileScanReportPy::from_engine(result))
    })
}

/// Analyze an iOS IPA file (asynchronous).
///
/// Returns a PyFuture that can be awaited in Python.
#[pyfunction]
pub fn async_analyze_ipa(path: &str) -> PyResult<PyFuture> {
    let path_owned = path.to_string();

    runtime_async::spawn_async(async move {
        let path_ref = Path::new(&path_owned);
        let result = eggsec::mobile::analyze_ipa(path_ref)
            .await
            .map_err(|e| ScanError::new_err(format!("Mobile analysis failed: {}", e)))?;

        Ok(MobileScanReportPy::from_engine(result))
    })
}
