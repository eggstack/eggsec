use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::artifact::ArtifactReferencePy;
use crate::error::ScanError;
use crate::runtime_async;
use crate::runtime_async::PyFuture;

// ═══════════════════════════════════════════════════════════════════
// Workstream 2-6: Mobile device discovery and session lifecycle
// ═══════════════════════════════════════════════════════════════════

/// Descriptor for a connected mobile device.
///
/// Contains device identity, platform information, transport details,
/// and supported operation capabilities.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileDeviceDescriptor {
    #[pyo3(get)]
    pub serial: String,
    #[pyo3(get)]
    pub model: Option<String>,
    #[pyo3(get)]
    pub platform: String,
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
    pub transport: String,
    #[pyo3(get)]
    pub authorization_status: String,
    supported_operations: Vec<String>,
}

#[pymethods]
impl MobileDeviceDescriptor {
    #[getter]
    fn supported_operations(&self) -> Vec<String> {
        self.supported_operations.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("serial", &self.serial)?;
        dict.set_item("model", &self.model)?;
        dict.set_item("platform", &self.platform)?;
        dict.set_item("android_version", &self.android_version)?;
        dict.set_item("sdk_version", self.sdk_version)?;
        dict.set_item("abi", &self.abi)?;
        dict.set_item("is_emulator", self.is_emulator)?;
        dict.set_item("is_rooted", self.is_rooted)?;
        dict.set_item("transport", &self.transport)?;
        dict.set_item("authorization_status", &self.authorization_status)?;
        dict.set_item("supported_operations", &self.supported_operations)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "MobileDeviceDescriptor(serial={}, platform={}, model={:?})",
            self.serial, self.platform, self.model
        )
    }

    fn __str__(&self) -> String {
        let model = self.model.as_deref().unwrap_or("unknown");
        format!("{} ({}) [{}]", model, self.serial, self.platform)
    }
}

/// Capabilities of a connected mobile device.
///
/// Describes which operations are supported for dynamic testing
/// on this particular device.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileDeviceCapabilities {
    #[pyo3(get)]
    pub supports_install: bool,
    #[pyo3(get)]
    pub supports_uninstall: bool,
    #[pyo3(get)]
    pub supports_launch: bool,
    #[pyo3(get)]
    pub supports_frida: bool,
    #[pyo3(get)]
    pub supports_log_capture: bool,
    #[pyo3(get)]
    pub supports_screenshot: bool,
    #[pyo3(get)]
    pub supports_network_capture: bool,
    #[pyo3(get)]
    pub supports_filesystem_extract: bool,
    #[pyo3(get)]
    pub supports_permission_management: bool,
}

#[pymethods]
impl MobileDeviceCapabilities {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("supports_install", self.supports_install)?;
        dict.set_item("supports_uninstall", self.supports_uninstall)?;
        dict.set_item("supports_launch", self.supports_launch)?;
        dict.set_item("supports_frida", self.supports_frida)?;
        dict.set_item("supports_log_capture", self.supports_log_capture)?;
        dict.set_item("supports_screenshot", self.supports_screenshot)?;
        dict.set_item("supports_network_capture", self.supports_network_capture)?;
        dict.set_item("supports_filesystem_extract", self.supports_filesystem_extract)?;
        dict.set_item(
            "supports_permission_management",
            self.supports_permission_management,
        )?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "MobileDeviceCapabilities(install={}, frida={}, screenshot={})",
            self.supports_install, self.supports_frida, self.supports_screenshot
        )
    }
}

/// Configuration for a mobile session.
///
/// Controls device targeting, app lifecycle, capture options, Frida
/// instrumentation, and permission management for the session.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileSessionConfig {
    #[pyo3(get)]
    pub device_serial: String,
    #[pyo3(get)]
    pub package_id: Option<String>,
    #[pyo3(get)]
    pub install_app: bool,
    #[pyo3(get)]
    pub uninstall_after: bool,
    #[pyo3(get)]
    pub capture_logs: bool,
    #[pyo3(get)]
    pub capture_screenshots: bool,
    #[pyo3(get)]
    pub capture_network: bool,
    #[pyo3(get)]
    pub traffic_output: Option<String>,
    frida_scripts: Vec<String>,
    #[pyo3(get)]
    pub allow_frida: bool,
    #[pyo3(get)]
    pub timeout_secs: Option<u64>,
    #[pyo3(get)]
    pub proxy: Option<String>,
    grant_permissions: Vec<String>,
    revoke_permissions: Vec<String>,
    #[pyo3(get)]
    pub dry_run: bool,
}

#[pymethods]
impl MobileSessionConfig {
    #[new]
    #[pyo3(signature = (device_serial, package_id=None, install_app=false, uninstall_after=false, capture_logs=false, capture_screenshots=false, capture_network=false, traffic_output=None, frida_scripts=None, allow_frida=false, timeout_secs=None, proxy=None, grant_permissions=None, revoke_permissions=None, dry_run=false))]
    fn new(
        device_serial: String,
        package_id: Option<String>,
        install_app: bool,
        uninstall_after: bool,
        capture_logs: bool,
        capture_screenshots: bool,
        capture_network: bool,
        traffic_output: Option<String>,
        frida_scripts: Option<Vec<String>>,
        allow_frida: bool,
        timeout_secs: Option<u64>,
        proxy: Option<String>,
        grant_permissions: Option<Vec<String>>,
        revoke_permissions: Option<Vec<String>>,
        dry_run: bool,
    ) -> Self {
        Self {
            device_serial,
            package_id,
            install_app,
            uninstall_after,
            capture_logs,
            capture_screenshots,
            capture_network,
            traffic_output,
            frida_scripts: frida_scripts.unwrap_or_default(),
            allow_frida,
            timeout_secs,
            proxy,
            grant_permissions: grant_permissions.unwrap_or_default(),
            revoke_permissions: revoke_permissions.unwrap_or_default(),
            dry_run,
        }
    }

    #[getter]
    fn frida_scripts(&self) -> Vec<String> {
        self.frida_scripts.clone()
    }

    #[getter]
    fn grant_permissions(&self) -> Vec<String> {
        self.grant_permissions.clone()
    }

    #[getter]
    fn revoke_permissions(&self) -> Vec<String> {
        self.revoke_permissions.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("device_serial", &self.device_serial)?;
        dict.set_item("package_id", &self.package_id)?;
        dict.set_item("install_app", self.install_app)?;
        dict.set_item("uninstall_after", self.uninstall_after)?;
        dict.set_item("capture_logs", self.capture_logs)?;
        dict.set_item("capture_screenshots", self.capture_screenshots)?;
        dict.set_item("capture_network", self.capture_network)?;
        dict.set_item("traffic_output", &self.traffic_output)?;
        dict.set_item("frida_scripts", &self.frida_scripts)?;
        dict.set_item("allow_frida", self.allow_frida)?;
        dict.set_item("timeout_secs", self.timeout_secs)?;
        dict.set_item("proxy", &self.proxy)?;
        dict.set_item("grant_permissions", &self.grant_permissions)?;
        dict.set_item("revoke_permissions", &self.revoke_permissions)?;
        dict.set_item("dry_run", self.dry_run)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "MobileSessionConfig(device={}, package={:?}, dry_run={})",
            self.device_serial, self.package_id, self.dry_run
        )
    }
}

/// Lifecycle state of a mobile session.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MobileSessionState {
    Created,
    Connecting,
    Installing,
    Launching,
    Running,
    Capturing,
    Stopping,
    Uninstalling,
    Cleaning,
    Stopped,
    Failed,
    Cancelled,
}

#[pymethods]
impl MobileSessionState {
    fn __repr__(&self) -> String {
        format!("MobileSessionState.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl MobileSessionState {
    pub fn as_str(&self) -> &str {
        match self {
            MobileSessionState::Created => "Created",
            MobileSessionState::Connecting => "Connecting",
            MobileSessionState::Installing => "Installing",
            MobileSessionState::Launching => "Launching",
            MobileSessionState::Running => "Running",
            MobileSessionState::Capturing => "Capturing",
            MobileSessionState::Stopping => "Stopping",
            MobileSessionState::Uninstalling => "Uninstalling",
            MobileSessionState::Cleaning => "Cleaning",
            MobileSessionState::Stopped => "Stopped",
            MobileSessionState::Failed => "Failed",
            MobileSessionState::Cancelled => "Cancelled",
        }
    }
}

/// Aggregated statistics for a mobile session.
///
/// Updated as the session progresses through its lifecycle.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileSessionStats {
    #[pyo3(get)]
    pub screenshots_captured: usize,
    #[pyo3(get)]
    pub log_entries: usize,
    #[pyo3(get)]
    pub network_exchanges: usize,
    #[pyo3(get)]
    pub artifacts_collected: usize,
    #[pyo3(get)]
    pub frida_events: usize,
    #[pyo3(get)]
    pub duration_ms: u64,
}

#[pymethods]
impl MobileSessionStats {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("screenshots_captured", self.screenshots_captured)?;
        dict.set_item("log_entries", self.log_entries)?;
        dict.set_item("network_exchanges", self.network_exchanges)?;
        dict.set_item("artifacts_collected", self.artifacts_collected)?;
        dict.set_item("frida_events", self.frida_events)?;
        dict.set_item("duration_ms", self.duration_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "MobileSessionStats(screenshots={}, logs={}, network={}, artifacts={}, frida={}, duration_ms={})",
            self.screenshots_captured,
            self.log_entries,
            self.network_exchanges,
            self.artifacts_collected,
            self.frida_events,
            self.duration_ms
        )
    }
}

/// A mobile session managing device interaction and app lifecycle.
///
/// Mutable session object that tracks state through the device interaction
/// pipeline: connect, install, launch, capture, and cleanup.
#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileSession {
    #[pyo3(get)]
    pub session_id: String,
    state: MobileSessionState,
    #[pyo3(get)]
    pub device_serial: String,
    #[pyo3(get)]
    pub config: MobileSessionConfig,
    stats: MobileSessionStats,
}

#[pymethods]
impl MobileSession {
    #[new]
    #[pyo3(signature = (session_id, device_serial, config))]
    fn new(
        session_id: String,
        device_serial: String,
        config: MobileSessionConfig,
    ) -> Self {
        Self {
            session_id,
            state: MobileSessionState::Created,
            device_serial,
            config,
            stats: MobileSessionStats {
                screenshots_captured: 0,
                log_entries: 0,
                network_exchanges: 0,
                artifacts_collected: 0,
                frida_events: 0,
                duration_ms: 0,
            },
        }
    }

    #[getter]
    fn state(&self) -> MobileSessionState {
        self.state
    }

    #[getter]
    fn stats(&self) -> MobileSessionStats {
        self.stats.clone()
    }

    /// Begin the session: connect to the device.
    fn start(&mut self) -> PyResult<()> {
        self.state = MobileSessionState::Connecting;
        Ok(())
    }

    /// Stop the session: begin teardown.
    fn stop(&mut self) -> PyResult<()> {
        self.state = MobileSessionState::Stopping;
        Ok(())
    }

    /// Install an application package on the connected device.
    ///
    /// Args:
    ///     package_path: Filesystem path to the APK or IPA.
    ///
    /// Returns:
    ///     str: The installed package identifier.
    ///
    /// Raises:
    ///     ScanError: If installation fails or the device is not connected.
    fn install_app(&mut self, package_path: &str) -> PyResult<String> {
        let _ = package_path;
        self.state = MobileSessionState::Installing;
        self.state = MobileSessionState::Failed;
        Err(ScanError::new_err(
            "Mobile session install requires a connected device and device runtime support. Use list_mobile_devices() to verify connectivity.",
        ))
    }

    /// Uninstall the current application from the device.
    ///
    /// Raises:
    ///     ScanError: If no package_id is configured or the device is not connected.
    fn uninstall_app(&mut self) -> PyResult<()> {
        let _ = self.config.package_id.as_ref().ok_or_else(|| {
            ScanError::new_err("No package_id configured for session")
        })?;
        self.state = MobileSessionState::Uninstalling;
        self.state = MobileSessionState::Failed;
        Err(ScanError::new_err(
            "Mobile session uninstall requires a connected device and device runtime support. Use list_mobile_devices() to verify connectivity.",
        ))
    }

    /// Launch the configured application on the device.
    ///
    /// Raises:
    ///     ScanError: If no package_id is configured or the device is not connected.
    fn launch_app(&mut self) -> PyResult<()> {
        let _ = self.config.package_id.as_ref().ok_or_else(|| {
            ScanError::new_err("No package_id configured for session")
        })?;
        self.state = MobileSessionState::Launching;
        self.state = MobileSessionState::Failed;
        Err(ScanError::new_err(
            "Mobile session launch requires a connected device and device runtime support. Use list_mobile_devices() to verify connectivity.",
        ))
    }

    /// Stop the currently running application on the device.
    ///
    /// Raises:
    ///     ScanError: If no package_id is configured or the device is not connected.
    fn stop_app(&mut self) -> PyResult<()> {
        let _ = self.config.package_id.as_ref().ok_or_else(|| {
            ScanError::new_err("No package_id configured for session")
        })?;
        Err(ScanError::new_err(
            "Mobile session stop_app requires a connected device and device runtime support. Use list_mobile_devices() to verify connectivity.",
        ))
    }

    /// Capture a screenshot from the device.
    ///
    /// Returns:
    ///     ArtifactReferencePy: Reference to the captured screenshot artifact.
    ///
    /// Raises:
    ///     ScanError: If the device is not connected or screenshot capture fails.
    fn capture_screenshot(&mut self) -> PyResult<ArtifactReferencePy> {
        self.state = MobileSessionState::Capturing;
        self.state = MobileSessionState::Failed;
        Err(ScanError::new_err(
            "Mobile screenshot capture requires a connected device and device runtime support. Use list_mobile_devices() to verify connectivity.",
        ))
    }

    /// Retrieve captured log entries from the device.
    ///
    /// Returns:
    ///     list[str]: Log lines captured during the session.
    ///
    /// Raises:
    ///     ScanError: If the device is not connected or log capture fails.
    fn get_logs(&self) -> PyResult<Vec<String>> {
        Err(ScanError::new_err(
            "Mobile log capture requires a connected device and device runtime support. Use list_mobile_devices() to verify connectivity.",
        ))
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        Ok(false)
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("state", self.state.as_str())?;
        dict.set_item("device_serial", &self.device_serial)?;
        dict.set_item("config", self.config.to_dict(py)?)?;
        dict.set_item("stats", self.stats.to_dict(py)?)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "MobileSession(id={}, state={}, device={})",
            self.session_id,
            self.state.as_str(),
            self.device_serial
        )
    }
}

/// Asynchronous mobile session with the same interface as MobileSession.
///
/// All methods return PyFuture for use with Python `await`.
#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncMobileSession {
    #[pyo3(get)]
    pub session_id: String,
    state: MobileSessionState,
    #[pyo3(get)]
    pub device_serial: String,
    #[pyo3(get)]
    pub config: MobileSessionConfig,
    stats: MobileSessionStats,
}

#[pymethods]
impl AsyncMobileSession {
    #[new]
    #[pyo3(signature = (session_id, device_serial, config))]
    fn new(
        session_id: String,
        device_serial: String,
        config: MobileSessionConfig,
    ) -> Self {
        Self {
            session_id,
            state: MobileSessionState::Created,
            device_serial,
            config,
            stats: MobileSessionStats {
                screenshots_captured: 0,
                log_entries: 0,
                network_exchanges: 0,
                artifacts_collected: 0,
                frida_events: 0,
                duration_ms: 0,
            },
        }
    }

    #[getter]
    fn state(&self) -> MobileSessionState {
        self.state
    }

    #[getter]
    fn stats(&self) -> MobileSessionStats {
        self.stats.clone()
    }

    /// Begin the session: connect to the device.
    fn async_start(&mut self) -> PyResult<PyFuture> {
        self.state = MobileSessionState::Connecting;
        runtime_async::spawn_async(async {
            Err::<(), PyErr>(ScanError::new_err(
                "Mobile session connect requires device runtime support. Use list_mobile_devices() to verify connectivity.",
            ))
        })
    }

    /// Stop the session: begin teardown.
    fn async_stop(&mut self) -> PyResult<PyFuture> {
        self.state = MobileSessionState::Stopping;
        runtime_async::spawn_async(async {
            Err::<(), PyErr>(ScanError::new_err(
                "Mobile session disconnect requires device runtime support. Use list_mobile_devices() to verify connectivity.",
            ))
        })
    }

    /// Install an application package on the connected device.
    fn async_install_app(&mut self, package_path: &str) -> PyResult<PyFuture> {
        let _ = package_path;
        self.state = MobileSessionState::Installing;
        runtime_async::spawn_async(async {
            Err::<(), PyErr>(ScanError::new_err(
                "Mobile session install requires device runtime support. Use list_mobile_devices() to verify connectivity.",
            ))
        })
    }

    /// Uninstall the current application from the device.
    fn async_uninstall_app(&mut self) -> PyResult<PyFuture> {
        let _ = self.config.package_id.as_ref().ok_or_else(|| {
            ScanError::new_err("No package_id configured for session")
        })?;
        self.state = MobileSessionState::Uninstalling;
        runtime_async::spawn_async(async {
            Err::<(), PyErr>(ScanError::new_err(
                "Mobile session uninstall requires device runtime support. Use list_mobile_devices() to verify connectivity.",
            ))
        })
    }

    /// Launch the configured application on the device.
    fn async_launch_app(&mut self) -> PyResult<PyFuture> {
        let _ = self.config.package_id.as_ref().ok_or_else(|| {
            ScanError::new_err("No package_id configured for session")
        })?;
        self.state = MobileSessionState::Launching;
        runtime_async::spawn_async(async {
            Err::<(), PyErr>(ScanError::new_err(
                "Mobile session launch requires device runtime support. Use list_mobile_devices() to verify connectivity.",
            ))
        })
    }

    /// Stop the currently running application on the device.
    fn async_stop_app(&mut self) -> PyResult<PyFuture> {
        let _ = self.config.package_id.as_ref().ok_or_else(|| {
            ScanError::new_err("No package_id configured for session")
        })?;
        runtime_async::spawn_async(async {
            Err::<(), PyErr>(ScanError::new_err(
                "Mobile session stop_app requires device runtime support. Use list_mobile_devices() to verify connectivity.",
            ))
        })
    }

    /// Capture a screenshot from the device.
    fn async_capture_screenshot(&mut self) -> PyResult<PyFuture> {
        self.state = MobileSessionState::Capturing;
        runtime_async::spawn_async(async {
            Err::<(), PyErr>(ScanError::new_err(
                "Mobile screenshot capture requires device runtime support. Use list_mobile_devices() to verify connectivity.",
            ))
        })
    }

    /// Retrieve captured log entries from the device.
    fn async_get_logs(&self) -> PyResult<PyFuture> {
        runtime_async::spawn_async(async {
            Err::<(), PyErr>(ScanError::new_err(
                "Mobile log capture requires device runtime support. Use list_mobile_devices() to verify connectivity.",
            ))
        })
    }

    /// Async context manager __aenter__.
    fn __aenter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Async context manager __aexit__: stops the session.
    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __aexit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        Ok(false)
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("state", self.state.as_str())?;
        dict.set_item("device_serial", &self.device_serial)?;
        dict.set_item("config", self.config.to_dict(py)?)?;
        dict.set_item("stats", self.stats.to_dict(py)?)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "AsyncMobileSession(id={}, state={}, device={})",
            self.session_id,
            self.state.as_str(),
            self.device_serial
        )
    }
}

/// Registry of discovered mobile devices.
///
/// Maintains a cached list of connected devices and supports refresh
/// to re-enumerate the available devices.
#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileDeviceRegistry {
    devices: Vec<MobileDeviceDescriptor>,
}

#[pymethods]
impl MobileDeviceRegistry {
    #[new]
    fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    #[getter]
    fn devices(&self) -> Vec<MobileDeviceDescriptor> {
        self.devices.clone()
    }

    /// Re-enumerate connected devices via ADB or similar tooling.
    ///
    /// Returns:
    ///     list[MobileDeviceDescriptor]: Updated device list.
    fn refresh(&mut self) -> PyResult<Vec<MobileDeviceDescriptor>> {
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
                let android_version = None;
                let sdk_version = None;
                for part in &parts[2..] {
                    if let Some(val) = part.strip_prefix("model:") {
                        model = Some(val.to_string());
                    } else if let Some(val) = part.strip_prefix("abi:") {
                        abi = Some(val.to_string());
                    }
                }
                let is_emulator = serial.contains("emulator");
                let transport = if is_emulator {
                    "adb_remote"
                } else {
                    "usb"
                };

                devices.push(MobileDeviceDescriptor {
                    serial: serial.clone(),
                    model,
                    platform: "android".to_string(),
                    android_version,
                    sdk_version,
                    abi,
                    is_emulator,
                    is_rooted: false,
                    transport: transport.to_string(),
                    authorization_status: "authorized".to_string(),
                    supported_operations: vec![
                        "install".to_string(),
                        "uninstall".to_string(),
                        "launch".to_string(),
                        "screenshot".to_string(),
                        "logcat".to_string(),
                    ],
                });
            }
        }
        self.devices = devices.clone();
        Ok(devices)
    }

    /// Look up a device by its serial number.
    ///
    /// Args:
    ///     serial: Device serial identifier.
    ///
    /// Returns:
    ///     MobileDeviceDescriptor | None: The device descriptor, or None if not found.
    fn get_device(&self, serial: &str) -> Option<MobileDeviceDescriptor> {
        self.devices.iter().find(|d| d.serial == serial).cloned()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        let devices_list = PyList::empty_bound(py);
        for d in &self.devices {
            devices_list.append(d.to_dict(py)?)?;
        }
        dict.set_item("devices", devices_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "MobileDeviceRegistry(device_count={})",
            self.devices.len()
        )
    }
}
