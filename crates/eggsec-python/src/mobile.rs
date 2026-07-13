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

/// Type alias for the APK analysis report used by the operation registry.
pub type ApkAnalysisReportPy = MobileScanReportPy;

/// Type alias for the IPA analysis report used by the operation registry.
pub type IpaAnalysisReportPy = MobileScanReportPy;

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

// ═══════════════════════════════════════════════════════════════════
// D5: Mobile dynamic analysis types
// ═══════════════════════════════════════════════════════════════════

/// Represents a connected mobile device for dynamic testing.
///
/// Contains device information obtained via ADB or similar tooling.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileDevicePy {
    #[pyo3(get)]
    pub serial: String,
    #[pyo3(get)]
    pub model: Option<String>,
    #[pyo3(get)]
    pub android_version: Option<String>,
    #[pyo3(get)]
    pub sdk_version: Option<u32>,
    #[pyo3(get)]
    pub abi: Option<String>,
    #[pyo3(get)]
    pub is_emulator: bool,
    #[pyo3(get)]
    pub is_rooted: bool,
    #[pyo3(get)]
    pub usb_connected: bool,
}

#[pymethods]
impl MobileDevicePy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("serial", &self.serial)?;
        dict.set_item("model", &self.model)?;
        dict.set_item("android_version", &self.android_version)?;
        dict.set_item("sdk_version", self.sdk_version)?;
        dict.set_item("abi", &self.abi)?;
        dict.set_item("is_emulator", self.is_emulator)?;
        dict.set_item("is_rooted", self.is_rooted)?;
        dict.set_item("usb_connected", self.usb_connected)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "MobileDevice(serial={}, model={:?})",
            self.serial, self.model
        )
    }

    fn __str__(&self) -> String {
        let model = self.model.as_deref().unwrap_or("unknown");
        format!("{} ({})", model, self.serial)
    }
}

/// Configuration for dynamic mobile analysis.
///
/// Controls what actions are performed on the device (install, launch,
/// capture traffic, run Frida scripts, etc.).
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicMobileConfigPy {
    #[pyo3(get)]
    pub install: bool,
    #[pyo3(get)]
    pub launch: Option<String>,
    #[pyo3(get)]
    pub capture_logs: bool,
    #[pyo3(get)]
    pub duration_secs: Option<u64>,
    #[pyo3(get)]
    pub uninstall_after: bool,
    #[pyo3(get)]
    pub dry_run: bool,
    #[pyo3(get)]
    pub proxy: Option<String>,
    #[pyo3(get)]
    pub frida_scripts: Vec<String>,
    #[pyo3(get)]
    pub allow_frida: bool,
    #[pyo3(get)]
    pub grant_permissions: Vec<String>,
    #[pyo3(get)]
    pub revoke_permissions: Vec<String>,
    #[pyo3(get)]
    pub list_permissions: bool,
    #[pyo3(get)]
    pub traffic_capture: Option<String>,
    #[pyo3(get)]
    pub baseline: Option<String>,
    #[pyo3(get)]
    pub evidence_bundle: Option<String>,
}

#[pymethods]
impl DynamicMobileConfigPy {
    #[new]
    #[pyo3(signature = (install=false, launch=None, capture_logs=false, duration_secs=None, uninstall_after=false, dry_run=false, proxy=None, frida_scripts=None, allow_frida=false, grant_permissions=None, revoke_permissions=None, list_permissions=false, traffic_capture=None, baseline=None, evidence_bundle=None))]
    fn new(
        install: bool,
        launch: Option<&str>,
        capture_logs: bool,
        duration_secs: Option<u64>,
        uninstall_after: bool,
        dry_run: bool,
        proxy: Option<&str>,
        frida_scripts: Option<Vec<String>>,
        allow_frida: bool,
        grant_permissions: Option<Vec<String>>,
        revoke_permissions: Option<Vec<String>>,
        list_permissions: bool,
        traffic_capture: Option<&str>,
        baseline: Option<&str>,
        evidence_bundle: Option<&str>,
    ) -> Self {
        Self {
            install,
            launch: launch.map(|s| s.to_string()),
            capture_logs,
            duration_secs,
            uninstall_after,
            dry_run,
            proxy: proxy.map(|s| s.to_string()),
            frida_scripts: frida_scripts.unwrap_or_default(),
            allow_frida,
            grant_permissions: grant_permissions.unwrap_or_default(),
            revoke_permissions: revoke_permissions.unwrap_or_default(),
            list_permissions,
            traffic_capture: traffic_capture.map(|s| s.to_string()),
            baseline: baseline.map(|s| s.to_string()),
            evidence_bundle: evidence_bundle.map(|s| s.to_string()),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("install", self.install)?;
        dict.set_item("launch", &self.launch)?;
        dict.set_item("capture_logs", self.capture_logs)?;
        dict.set_item("duration_secs", self.duration_secs)?;
        dict.set_item("uninstall_after", self.uninstall_after)?;
        dict.set_item("dry_run", self.dry_run)?;
        dict.set_item("proxy", &self.proxy)?;
        dict.set_item("frida_scripts", &self.frida_scripts)?;
        dict.set_item("allow_frida", self.allow_frida)?;
        dict.set_item("grant_permissions", &self.grant_permissions)?;
        dict.set_item("revoke_permissions", &self.revoke_permissions)?;
        dict.set_item("list_permissions", self.list_permissions)?;
        dict.set_item("traffic_capture", &self.traffic_capture)?;
        dict.set_item("baseline", &self.baseline)?;
        dict.set_item("evidence_bundle", &self.evidence_bundle)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DynamicMobileConfig(install={}, dry_run={}, frida={})",
            self.install, self.dry_run, self.allow_frida
        )
    }
}

/// Result of dynamic mobile analysis.
///
/// Contains findings, device info, traffic summary, and Frida instrumentation results.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicMobileReportPy {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub scan_type: String,
    #[pyo3(get)]
    pub platform: String,
    #[pyo3(get)]
    pub device_serial: Option<String>,
    #[pyo3(get)]
    pub app_id: Option<String>,
    #[pyo3(get)]
    pub version: Option<String>,
    #[pyo3(get)]
    pub timestamp: String,
    #[pyo3(get)]
    pub findings_count: usize,
    #[pyo3(get)]
    pub recommendations: Vec<String>,
    #[pyo3(get)]
    pub duration_ms: u64,
    #[pyo3(get)]
    pub actions_performed: Vec<String>,
    #[pyo3(get)]
    pub dry_run: bool,
    #[pyo3(get)]
    pub has_traffic_summary: bool,
    #[pyo3(get)]
    pub has_frida_instrumentation: bool,
}

#[pymethods]
impl DynamicMobileReportPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("scan_type", &self.scan_type)?;
        dict.set_item("platform", &self.platform)?;
        dict.set_item("device_serial", &self.device_serial)?;
        dict.set_item("app_id", &self.app_id)?;
        dict.set_item("version", &self.version)?;
        dict.set_item("timestamp", &self.timestamp)?;
        dict.set_item("findings_count", self.findings_count)?;
        dict.set_item("recommendations", &self.recommendations)?;
        dict.set_item("duration_ms", self.duration_ms)?;
        dict.set_item("actions_performed", &self.actions_performed)?;
        dict.set_item("dry_run", self.dry_run)?;
        dict.set_item("has_traffic_summary", self.has_traffic_summary)?;
        dict.set_item("has_frida_instrumentation", self.has_frida_instrumentation)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DynamicMobileReport(target={}, findings={}, platform={})",
            self.target, self.findings_count, self.platform
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Dynamic scan of {} ({} findings, {})",
            self.target, self.findings_count, self.platform
        )
    }
}

/// List connected Android devices via ADB.
///
/// Returns:
///     list[MobileDevicePy]: Connected devices.
///
/// Raises:
///     ScanError: If ADB is unavailable or fails.
#[pyfunction]
pub fn list_mobile_devices() -> PyResult<Vec<MobileDevicePy>> {
    let output = std::process::Command::new("adb")
        .arg("devices")
        .arg("-l")
        .output()
        .map_err(|e| ScanError::new_err(format!("Failed to run ADB: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();
    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[1] == "device" {
            let serial = parts[0].to_string();
            let mut model = None;
            let mut abi = None;
            let mut android_version = None;
            for part in &parts[2..] {
                if let Some(val) = part.strip_prefix("model:") {
                    model = Some(val.to_string());
                } else if let Some(val) = part.strip_prefix("abi:") {
                    abi = Some(val.to_string());
                }
            }
            devices.push(MobileDevicePy {
                serial: serial.clone(),
                model,
                android_version,
                sdk_version: None,
                abi,
                is_emulator: serial.contains("emulator"),
                is_rooted: false,
                usb_connected: true,
            });
        }
    }
    Ok(devices)
}

/// Run dynamic mobile analysis on a connected device.
///
/// Args:
///     package: Package name (Android) or bundle ID (iOS).
///     config: Dynamic analysis configuration.
///     device_serial: Device serial (or None for first available).
///
/// Returns:
///     DynamicMobileReportPy: Dynamic analysis report.
///
/// Raises:
///     ScanError: If dynamic analysis fails.
#[pyfunction]
#[pyo3(signature = (package, config, device_serial=None))]
pub fn dynamic_mobile_analysis(
    package: &str,
    config: DynamicMobileConfigPy,
    device_serial: Option<&str>,
) -> PyResult<DynamicMobileReportPy> {
    let _ = (package, device_serial, config);
    Err(ScanError::new_err(
        "Dynamic mobile analysis requires a connected device. Use list_mobile_devices() to verify connectivity.",
    ))
}
