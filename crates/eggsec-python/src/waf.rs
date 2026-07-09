use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::{Deserialize, Serialize};

use crate::error::EggsecResultExt;
use crate::runtime_sync;

/// WAF detection result from HTTP response analysis.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafDetectionResultPy {
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub detected: bool,
    #[pyo3(get)]
    pub vendor: Option<String>,
    #[pyo3(get)]
    pub waf_name: Option<String>,
    #[pyo3(get)]
    pub confidence: u8,
    pub(crate) matched_headers: Vec<String>,
    pub(crate) matched_cookies: Vec<String>,
    pub(crate) matched_patterns: Vec<String>,
    #[pyo3(get)]
    pub server_header: Option<String>,
    #[pyo3(get)]
    pub status_code: u16,
    #[pyo3(get)]
    pub request_error: Option<String>,
}

#[pymethods]
impl WafDetectionResultPy {
    #[getter]
    fn matched_headers(&self) -> Vec<String> {
        self.matched_headers.clone()
    }

    #[getter]
    fn matched_cookies(&self) -> Vec<String> {
        self.matched_cookies.clone()
    }

    #[getter]
    fn matched_patterns(&self) -> Vec<String> {
        self.matched_patterns.clone()
    }

    /// Convert to a Python dictionary.
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("url", &self.url)?;
        dict.set_item("detected", self.detected)?;
        dict.set_item("vendor", &self.vendor)?;
        dict.set_item("waf_name", &self.waf_name)?;
        dict.set_item("confidence", self.confidence)?;
        dict.set_item("matched_headers", &self.matched_headers)?;
        dict.set_item("matched_cookies", &self.matched_cookies)?;
        dict.set_item("matched_patterns", &self.matched_patterns)?;
        dict.set_item("server_header", &self.server_header)?;
        dict.set_item("status_code", self.status_code)?;
        dict.set_item("request_error", &self.request_error)?;
        Ok(dict.into())
    }

    /// Convert to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        let vendor_str = self
            .vendor
            .as_deref()
            .unwrap_or("Unknown");
        format!(
            "WafDetectionResult(url={}, detected={}, vendor={}, confidence={})",
            self.url, self.detected, vendor_str, self.confidence
        )
    }

    fn __str__(&self) -> String {
        if self.detected {
            let vendor = self.vendor.as_deref().unwrap_or("Unknown");
            format!(
                "WAF detected: {} (confidence: {}%)",
                vendor, self.confidence
            )
        } else {
            "No WAF detected".to_string()
        }
    }
}

/// Detect WAF by making an HTTP request to the target URL.
///
/// This performs a passive detection only - no bypass or validation testing.
/// The detection analyzes HTTP response headers, cookies, status codes, and
/// body patterns against known WAF signatures.
///
/// Args:
///     url: Target URL to test (e.g. "https://example.com").
///
/// Returns:
///     WafDetectionResultPy: WAF detection result with vendor, confidence, and evidence.
///
/// Raises:
///     NetworkError: If the HTTP request fails.
#[pyfunction]
pub fn detect_waf(url: &str) -> PyResult<WafDetectionResultPy> {
    Python::with_gil(|py| {
        let url_owned = url.to_string();
        let result = runtime_sync::block_on(py, async move {
            let detector = eggsec::waf::WafDetector::new().map_pyerr()?;
            detector.detect(&url_owned).await.map_pyerr()
        })?;

        Ok(WafDetectionResultPy {
            url: result.status_code.to_string(), // placeholder, overwritten below
            detected: result.waf_name.is_some(),
            vendor: result.server_header.clone(), // placeholder, overwritten below
            waf_name: result.waf_name,
            confidence: result.confidence,
            matched_headers: result.matched_headers,
            matched_cookies: result.matched_cookies,
            matched_patterns: result.matched_patterns,
            server_header: result.server_header,
            status_code: result.status_code,
            request_error: result.request_error,
        })
    })
}

/// Detect WAF by making an async HTTP request to the target URL.
#[pyfunction]
pub fn async_detect_waf(url: &str) -> PyResult<crate::runtime_async::PyFuture> {
    let url_owned = url.to_string();

    crate::runtime_async::spawn_async(async move {
        let detector = eggsec::waf::WafDetector::new().map_pyerr()?;
        let result = detector.detect(&url_owned).await.map_pyerr()?;

        Ok(WafDetectionResultPy {
            url: url_owned,
            detected: result.waf_name.is_some(),
            vendor: result.waf_name.clone(),
            waf_name: result.waf_name,
            confidence: result.confidence,
            matched_headers: result.matched_headers,
            matched_cookies: result.matched_cookies,
            matched_patterns: result.matched_patterns,
            server_header: result.server_header,
            status_code: result.status_code,
            request_error: result.request_error,
        })
    })
}
