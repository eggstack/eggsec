use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::error::EggsecResultExt;
use crate::runtime_async;
use crate::runtime_sync;

/// HTTP load test result with latency percentiles and status code distribution.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestResultPy {
    #[pyo3(get)]
    pub target_url: String,
    #[pyo3(get)]
    pub total_requests: u64,
    #[pyo3(get)]
    pub successful_requests: u64,
    #[pyo3(get)]
    pub failed_requests: u64,
    #[pyo3(get)]
    pub total_duration_ms: u64,
    #[pyo3(get)]
    pub requests_per_second: f64,
    #[pyo3(get)]
    pub latency_min_ms: f64,
    #[pyo3(get)]
    pub latency_max_ms: f64,
    #[pyo3(get)]
    pub latency_mean_ms: f64,
    #[pyo3(get)]
    pub latency_p50_ms: f64,
    #[pyo3(get)]
    pub latency_p90_ms: f64,
    #[pyo3(get)]
    pub latency_p95_ms: f64,
    #[pyo3(get)]
    pub latency_p99_ms: f64,
    pub(crate) status_codes: HashMap<u16, u64>,
    #[pyo3(get)]
    pub errors: Vec<String>,
}

#[pymethods]
impl LoadTestResultPy {
    /// Status code to count mapping as a Python dict.
    #[getter]
    fn status_codes(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        for (&code, &count) in &self.status_codes {
            dict.set_item(code, count)?;
        }
        Ok(dict.into())
    }

    /// Convert to a Python dictionary.
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target_url", &self.target_url)?;
        dict.set_item("total_requests", self.total_requests)?;
        dict.set_item("successful_requests", self.successful_requests)?;
        dict.set_item("failed_requests", self.failed_requests)?;
        dict.set_item("total_duration_ms", self.total_duration_ms)?;
        dict.set_item("requests_per_second", self.requests_per_second)?;
        dict.set_item("latency_min_ms", self.latency_min_ms)?;
        dict.set_item("latency_max_ms", self.latency_max_ms)?;
        dict.set_item("latency_mean_ms", self.latency_mean_ms)?;
        dict.set_item("latency_p50_ms", self.latency_p50_ms)?;
        dict.set_item("latency_p90_ms", self.latency_p90_ms)?;
        dict.set_item("latency_p95_ms", self.latency_p95_ms)?;
        dict.set_item("latency_p99_ms", self.latency_p99_ms)?;
        let status_dict = PyDict::new_bound(py);
        for (&code, &count) in &self.status_codes {
            status_dict.set_item(code, count)?;
        }
        dict.set_item("status_codes", &status_dict)?;
        let error_list = PyList::new_bound(py, &self.errors);
        dict.set_item("errors", &error_list)?;
        Ok(dict.into())
    }

    /// Convert to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "LoadTestResult(url={}, total={}, successful={}, failed={}, rps={:.2})",
            self.target_url,
            self.total_requests,
            self.successful_requests,
            self.failed_requests,
            self.requests_per_second
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Load test: {} requests to {} in {:.2}s ({:.2} rps, {} errors)",
            self.total_requests,
            self.target_url,
            self.total_duration_ms as f64 / 1000.0,
            self.requests_per_second,
            self.failed_requests
        )
    }
}

/// Python-facing configuration for HTTP load testing.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub total_requests: u64,
    #[pyo3(get)]
    pub concurrency: usize,
    #[pyo3(get)]
    pub timeout_secs: u64,
    #[pyo3(get)]
    pub method: String,
    #[pyo3(get)]
    pub body: Option<String>,
    pub(crate) headers: Vec<(String, String)>,
}

#[pymethods]
impl LoadTestConfig {
    #[new]
    #[pyo3(signature = (url, total_requests, concurrency, timeout_secs, method="GET".to_string(), body=None, headers=None))]
    fn new(
        url: String,
        total_requests: u64,
        concurrency: usize,
        timeout_secs: u64,
        method: String,
        body: Option<String>,
        headers: Option<Vec<(String, String)>>,
    ) -> Self {
        Self {
            url,
            total_requests,
            concurrency,
            timeout_secs,
            method,
            body,
            headers: headers.unwrap_or_default(),
        }
    }

    /// Headers as a list of (key, value) tuples.
    #[getter]
    fn headers(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for (key, value) in &self.headers {
            let tuple = pyo3::types::PyTuple::new_bound(py, &[key.as_str(), value.as_str()]);
            list.append(tuple)?;
        }
        Ok(list.into())
    }

    /// Convert to a Python dictionary.
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("url", &self.url)?;
        dict.set_item("total_requests", self.total_requests)?;
        dict.set_item("concurrency", self.concurrency)?;
        dict.set_item("timeout_secs", self.timeout_secs)?;
        dict.set_item("method", &self.method)?;
        dict.set_item("body", &self.body)?;
        let header_list = PyList::empty_bound(py);
        for (key, value) in &self.headers {
            let tuple = pyo3::types::PyTuple::new_bound(py, &[key.as_str(), value.as_str()]);
            header_list.append(tuple)?;
        }
        dict.set_item("headers", &header_list)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "LoadTestConfig(url={}, requests={}, concurrency={}, method={})",
            self.url, self.total_requests, self.concurrency, self.method
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Run an HTTP load test (synchronous).
///
/// Sends `total_requests` to `url` with up to `concurrency` parallel connections.
/// Returns aggregated latency metrics and status code distribution.
///
/// Args:
///     url: Target URL (e.g. "https://example.com/api").
///     total_requests: Total number of requests to send.
///     concurrency: Max concurrent connections.
///     timeout_secs: Per-request timeout in seconds.
///     method: HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS).
///
/// Returns:
///     LoadTestResultPy: Aggregated load test results with latency percentiles.
///
/// Raises:
///     ScanError: If the load test fails.
#[pyfunction]
#[pyo3(signature = (url, total_requests, concurrency, timeout_secs, method="GET"))]
pub fn load_test_http(
    py: Python<'_>,
    url: &str,
    total_requests: u64,
    concurrency: usize,
    timeout_secs: u64,
    method: &str,
) -> PyResult<LoadTestResultPy> {
    let url_owned = url.to_string();
    let method_owned = method.to_string();

    let result = runtime_sync::block_on(py, async move {
        let mut runner = eggsec::loadtest::LoadTestRunner::new(
            url_owned,
            total_requests,
            concurrency,
            Duration::from_secs(timeout_secs),
        )
        .map_pyerr()?;

        runner.set_method(method_owned);

        runner.run().await.map_pyerr()
    })?;

    Ok(LoadTestResultPy {
        target_url: result.target_url,
        total_requests: result.total_requests,
        successful_requests: result.successful_requests,
        failed_requests: result.failed_requests,
        total_duration_ms: result.total_duration_ms,
        requests_per_second: result.requests_per_second,
        latency_min_ms: result.latency_min_ms,
        latency_max_ms: result.latency_max_ms,
        latency_mean_ms: result.latency_mean_ms,
        latency_p50_ms: result.latency_p50_ms,
        latency_p90_ms: result.latency_p90_ms,
        latency_p95_ms: result.latency_p95_ms,
        latency_p99_ms: result.latency_p99_ms,
        status_codes: result.status_codes.into_iter().collect(),
        errors: result.errors,
    })
}

/// Run an HTTP load test (asynchronous).
///
/// Returns a PyFuture that can be awaited in Python.
#[pyfunction]
#[pyo3(signature = (url, total_requests, concurrency, timeout_secs, method="GET"))]
pub fn async_load_test_http(
    url: &str,
    total_requests: u64,
    concurrency: usize,
    timeout_secs: u64,
    method: &str,
) -> PyResult<runtime_async::PyFuture> {
    let url_owned = url.to_string();
    let method_owned = method.to_string();

    runtime_async::spawn_async(async move {
        let mut runner = eggsec::loadtest::LoadTestRunner::new(
            url_owned,
            total_requests,
            concurrency,
            Duration::from_secs(timeout_secs),
        )
        .map_pyerr()?;

        runner.set_method(method_owned);

        let result = runner.run().await.map_pyerr()?;

        Ok(LoadTestResultPy {
            target_url: result.target_url,
            total_requests: result.total_requests,
            successful_requests: result.successful_requests,
            failed_requests: result.failed_requests,
            total_duration_ms: result.total_duration_ms,
            requests_per_second: result.requests_per_second,
            latency_min_ms: result.latency_min_ms,
            latency_max_ms: result.latency_max_ms,
            latency_mean_ms: result.latency_mean_ms,
            latency_p50_ms: result.latency_p50_ms,
            latency_p90_ms: result.latency_p90_ms,
            latency_p95_ms: result.latency_p95_ms,
            latency_p99_ms: result.latency_p99_ms,
            status_codes: result.status_codes.into_iter().collect(),
            errors: result.errors,
        })
    })
}
