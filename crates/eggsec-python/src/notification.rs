use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::{Deserialize, Serialize};

/// Webhook event type.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WebhookEventPy {
    ScanStarted,
    ScanComplete,
    Findings,
    Error,
}

#[pymethods]
impl WebhookEventPy {
    fn __repr__(&self) -> String {
        format!("WebhookEvent.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl WebhookEventPy {
    fn as_str(&self) -> &str {
        match self {
            WebhookEventPy::ScanStarted => "ScanStarted",
            WebhookEventPy::ScanComplete => "ScanComplete",
            WebhookEventPy::Findings => "Findings",
            WebhookEventPy::Error => "Error",
        }
    }
}

/// A finding summary for notifications.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingSummaryPy {
    #[pyo3(get)]
    pub title: String,
    #[pyo3(get)]
    pub severity: String,
    #[pyo3(get)]
    pub description: String,
}

#[pymethods]
impl FindingSummaryPy {
    #[new]
    fn new(title: String, severity: String, description: String) -> Self {
        Self {
            title,
            severity,
            description,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("title", &self.title)?;
        dict.set_item("severity", &self.severity)?;
        dict.set_item("description", &self.description)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "FindingSummary(title={}, severity={})",
            self.title, self.severity
        )
    }
}

/// Scan statistics for notifications.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyScanStatsPy {
    #[pyo3(get)]
    pub total_findings: usize,
    #[pyo3(get)]
    pub critical_count: usize,
    #[pyo3(get)]
    pub high_count: usize,
    #[pyo3(get)]
    pub medium_count: usize,
    #[pyo3(get)]
    pub low_count: usize,
    #[pyo3(get)]
    pub duration_secs: u64,
}

#[pymethods]
impl NotifyScanStatsPy {
    #[new]
    fn new(
        total_findings: usize,
        critical_count: usize,
        high_count: usize,
        medium_count: usize,
        low_count: usize,
        duration_secs: u64,
    ) -> Self {
        Self {
            total_findings,
            critical_count,
            high_count,
            medium_count,
            low_count,
            duration_secs,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "NotifyScanStats(total={}, critical={})",
            self.total_findings, self.critical_count
        )
    }
}

/// Webhook notification configuration.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfigPy {
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub enabled: bool,
    #[pyo3(get)]
    pub events: Vec<String>,
}

#[pymethods]
impl WebhookConfigPy {
    #[new]
    #[pyo3(signature = (url, enabled=true, events=None))]
    fn new(url: String, enabled: bool, events: Option<Vec<String>>) -> Self {
        Self {
            url,
            enabled,
            events: events.unwrap_or_default(),
        }
    }

    fn __repr__(&self) -> String {
        format!("WebhookConfig(url={}, enabled={})", self.url, self.enabled)
    }
}

/// Notification manager for sending alerts.
///
/// Supports webhook-based notifications for scan events.
#[pyclass]
pub struct NotifyManagerPy {
    inner: Option<eggsec::notify::NotifyManager>,
}

#[pymethods]
impl NotifyManagerPy {
    #[new]
    fn new() -> Self {
        Self { inner: None }
    }

    /// Create a notification manager from EggsecConfig.
    #[staticmethod]
    fn from_config(_config: &crate::config_model::PyEggsecConfig) -> Self {
        // Simplified stub - full implementation would bridge config
        Self { inner: None }
    }

    /// Check if notifications are enabled.
    fn is_enabled(&self) -> bool {
        self.inner.as_ref().map(|m| m.is_enabled()).unwrap_or(false)
    }

    fn __repr__(&self) -> String {
        format!(
            "NotifyManager(enabled={})",
            self.inner.as_ref().map(|m| m.is_enabled()).unwrap_or(false)
        )
    }
}

/// Send a scan started notification.
///
/// Args:
///     scan_id: Unique scan identifier.
///     target: Scan target.
///     webhook_url: Optional webhook URL override.
#[pyfunction]
#[pyo3(signature = (scan_id, target, webhook_url=None))]
pub fn notify_scan_started(scan_id: &str, target: &str, webhook_url: Option<&str>) -> PyResult<()> {
    let _ = webhook_url;
    // Stub implementation - in production this would use the actual notifier
    tracing::info!(
        "Notification: scan_started scan_id={} target={}",
        scan_id,
        target
    );
    Ok(())
}

/// Send a scan complete notification.
///
/// Args:
///     scan_id: Unique scan identifier.
///     target: Scan target.
///     message: Completion message.
///     findings: Optional list of finding summaries.
///     webhook_url: Optional webhook URL override.
#[pyfunction]
#[pyo3(signature = (scan_id, target, message, findings=None, webhook_url=None))]
pub fn notify_scan_complete(
    scan_id: &str,
    target: &str,
    message: &str,
    findings: Option<Vec<FindingSummaryPy>>,
    webhook_url: Option<&str>,
) -> PyResult<()> {
    let _ = webhook_url;
    let _ = findings;
    tracing::info!(
        "Notification: scan_complete scan_id={} target={} message={}",
        scan_id,
        target,
        message
    );
    Ok(())
}

/// Send a findings notification.
///
/// Args:
///     scan_id: Unique scan identifier.
///     target: Scan target.
///     findings: List of finding summaries.
///     webhook_url: Optional webhook URL override.
#[pyfunction]
#[pyo3(signature = (scan_id, target, findings, webhook_url=None))]
pub fn notify_findings(
    scan_id: &str,
    target: &str,
    findings: Vec<FindingSummaryPy>,
    webhook_url: Option<&str>,
) -> PyResult<()> {
    let _ = webhook_url;
    tracing::info!(
        "Notification: findings scan_id={} target={} count={}",
        scan_id,
        target,
        findings.len()
    );
    Ok(())
}

/// Send an error notification.
///
/// Args:
///     scan_id: Unique scan identifier.
///     target: Scan target.
///     error: Error message.
///     webhook_url: Optional webhook URL override.
#[pyfunction]
#[pyo3(signature = (scan_id, target, error, webhook_url=None))]
pub fn notify_error(
    scan_id: &str,
    target: &str,
    error: &str,
    webhook_url: Option<&str>,
) -> PyResult<()> {
    let _ = webhook_url;
    tracing::warn!(
        "Notification: error scan_id={} target={} error={}",
        scan_id,
        target,
        error
    );
    Ok(())
}
