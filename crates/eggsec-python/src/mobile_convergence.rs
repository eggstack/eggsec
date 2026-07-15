use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::error::ScanError;

// ═══════════════════════════════════════════════════════════════════
// WS4: Mobile static/dynamic convergence
// ═══════════════════════════════════════════════════════════════════

/// Summary of static analysis results that can seed a dynamic analysis plan.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticAnalysisSummary {
    #[pyo3(get)]
    pub package_id: String,
    #[pyo3(get)]
    pub package_name: Option<String>,
    #[pyo3(get)]
    pub version: Option<String>,
    #[pyo3(get)]
    pub min_sdk: Option<u32>,
    #[pyo3(get)]
    pub target_sdk: Option<u32>,
    #[pyo3(get)]
    pub permissions: Vec<String>,
    #[pyo3(get)]
    pub urls: Vec<String>,
    #[pyo3(get)]
    pub certificates: Vec<String>,
    #[pyo3(get)]
    pub exported_components: Vec<String>,
    #[pyo3(get)]
    pub activities: Vec<String>,
    #[pyo3(get)]
    pub services: Vec<String>,
    #[pyo3(get)]
    pub receivers: Vec<String>,
    #[pyo3(get)]
    pub providers: Vec<String>,
    #[pyo3(get)]
    pub deep_links: Vec<String>,
    #[pyo3(get)]
    pub intent_filters: Vec<String>,
    #[pyo3(get)]
    pub native_libraries: Vec<String>,
    #[pyo3(get)]
    pub hardcoded_secrets: Vec<String>,
    #[pyo3(get)]
    pub source_type: String,
}

#[pymethods]
impl StaticAnalysisSummary {
    #[new]
    #[pyo3(signature = (package_id, *, package_name=None, version=None, min_sdk=None, target_sdk=None, permissions=None, urls=None, certificates=None, exported_components=None, activities=None, services=None, receivers=None, providers=None, deep_links=None, intent_filters=None, native_libraries=None, hardcoded_secrets=None, source_type=None))]
    fn new(
        package_id: String,
        package_name: Option<String>,
        version: Option<String>,
        min_sdk: Option<u32>,
        target_sdk: Option<u32>,
        permissions: Option<Vec<String>>,
        urls: Option<Vec<String>>,
        certificates: Option<Vec<String>>,
        exported_components: Option<Vec<String>>,
        activities: Option<Vec<String>>,
        services: Option<Vec<String>>,
        receivers: Option<Vec<String>>,
        providers: Option<Vec<String>>,
        deep_links: Option<Vec<String>>,
        intent_filters: Option<Vec<String>>,
        native_libraries: Option<Vec<String>>,
        hardcoded_secrets: Option<Vec<String>>,
        source_type: Option<String>,
    ) -> Self {
        Self {
            package_id,
            package_name,
            version,
            min_sdk,
            target_sdk,
            permissions: permissions.unwrap_or_default(),
            urls: urls.unwrap_or_default(),
            certificates: certificates.unwrap_or_default(),
            exported_components: exported_components.unwrap_or_default(),
            activities: activities.unwrap_or_default(),
            services: services.unwrap_or_default(),
            receivers: receivers.unwrap_or_default(),
            providers: providers.unwrap_or_default(),
            deep_links: deep_links.unwrap_or_default(),
            intent_filters: intent_filters.unwrap_or_default(),
            native_libraries: native_libraries.unwrap_or_default(),
            hardcoded_secrets: hardcoded_secrets.unwrap_or_default(),
            source_type: source_type.unwrap_or_else(|| "apk".to_string()),
        }
    }

    /// Generate a dynamic analysis plan from the static findings.
    fn to_dynamic_plan(&self) -> DynamicAnalysisPlan {
        let mut targets = Vec::new();

        for url in &self.urls {
            targets.push(AnalysisTarget::new(
                url.clone(),
                "url".to_string(),
                "network".to_string(),
                5,
            ));
        }

        for component in &self.exported_components {
            targets.push(AnalysisTarget::new(
                component.clone(),
                "exported_component".to_string(),
                "component".to_string(),
                5,
            ));
        }

        for deep_link in &self.deep_links {
            targets.push(AnalysisTarget::new(
                deep_link.clone(),
                "deep_link".to_string(),
                "navigation".to_string(),
                5,
            ));
        }

        for secret in &self.hardcoded_secrets {
            targets.push(AnalysisTarget::new(
                secret.clone(),
                "hardcoded_secret".to_string(),
                "secret".to_string(),
                8,
            ));
        }

        DynamicAnalysisPlan {
            package_id: self.package_id.clone(),
            targets,
            permissions_to_test: self.permissions.clone(),
            components_to_inspect: self.exported_components.clone(),
            urls_to_probe: self.urls.clone(),
            use_frida: !self.native_libraries.is_empty(),
            instrumentation_focus: if self.native_libraries.is_empty() {
                "java".to_string()
            } else {
                "java+native".to_string()
            },
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("package_id", &self.package_id)?;
        dict.set_item("package_name", &self.package_name)?;
        dict.set_item("version", &self.version)?;
        dict.set_item("min_sdk", self.min_sdk)?;
        dict.set_item("target_sdk", self.target_sdk)?;
        dict.set_item("permissions", &self.permissions)?;
        dict.set_item("urls", &self.urls)?;
        dict.set_item("certificates", &self.certificates)?;
        dict.set_item("exported_components", &self.exported_components)?;
        dict.set_item("activities", &self.activities)?;
        dict.set_item("services", &self.services)?;
        dict.set_item("receivers", &self.receivers)?;
        dict.set_item("providers", &self.providers)?;
        dict.set_item("deep_links", &self.deep_links)?;
        dict.set_item("intent_filters", &self.intent_filters)?;
        dict.set_item("native_libraries", &self.native_libraries)?;
        dict.set_item("hardcoded_secrets", &self.hardcoded_secrets)?;
        dict.set_item("source_type", &self.source_type)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "StaticAnalysisSummary(pkg={}, perms={}, urls={})",
            self.package_id,
            self.permissions.len(),
            self.urls.len()
        )
    }
}

/// A single analysis target derived from static analysis.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisTarget {
    #[pyo3(get)]
    pub identifier: String,
    #[pyo3(get)]
    pub target_type: String,
    #[pyo3(get)]
    pub category: String,
    #[pyo3(get)]
    pub priority: u8,
}

#[pymethods]
impl AnalysisTarget {
    #[new]
    #[pyo3(signature = (identifier, target_type, category, priority=5))]
    fn new(identifier: String, target_type: String, category: String, priority: u8) -> Self {
        Self {
            identifier,
            target_type,
            category,
            priority,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("identifier", &self.identifier)?;
        dict.set_item("target_type", &self.target_type)?;
        dict.set_item("category", &self.category)?;
        dict.set_item("priority", self.priority)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "AnalysisTarget(id={}, type={}, cat={})",
            self.identifier, self.target_type, self.category
        )
    }
}

/// Plan for dynamic analysis derived from static analysis results.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicAnalysisPlan {
    #[pyo3(get)]
    pub package_id: String,
    targets: Vec<AnalysisTarget>,
    #[pyo3(get)]
    pub permissions_to_test: Vec<String>,
    #[pyo3(get)]
    pub components_to_inspect: Vec<String>,
    #[pyo3(get)]
    pub urls_to_probe: Vec<String>,
    #[pyo3(get)]
    pub use_frida: bool,
    #[pyo3(get)]
    pub instrumentation_focus: String,
}

#[pymethods]
impl DynamicAnalysisPlan {
    #[getter]
    fn targets(&self) -> Vec<AnalysisTarget> {
        self.targets.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("package_id", &self.package_id)?;

        let targets_list = PyList::empty_bound(py);
        for t in &self.targets {
            targets_list.append(t.to_dict(py)?)?;
        }
        dict.set_item("targets", targets_list)?;
        dict.set_item("permissions_to_test", &self.permissions_to_test)?;
        dict.set_item("components_to_inspect", &self.components_to_inspect)?;
        dict.set_item("urls_to_probe", &self.urls_to_probe)?;
        dict.set_item("use_frida", self.use_frida)?;
        dict.set_item("instrumentation_focus", &self.instrumentation_focus)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DynamicAnalysisPlan(pkg={}, targets={}, frida={})",
            self.package_id,
            self.targets.len(),
            self.use_frida
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// WS5: Mobile instrumentation boundary
// ═══════════════════════════════════════════════════════════════════

/// Configuration for a mobile instrumentation session.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentationConfig {
    #[pyo3(get)]
    pub session_id: String,
    #[pyo3(get)]
    pub device_serial: String,
    #[pyo3(get)]
    pub package_id: String,
    #[pyo3(get)]
    pub timeout_secs: u64,
    #[pyo3(get)]
    pub max_output_bytes: u64,
    #[pyo3(get)]
    pub allow_system_hooks: bool,
    #[pyo3(get)]
    pub enable_logging: bool,
    scripts: Vec<InstrumentationScript>,
    hooks: Vec<String>,
}

#[pymethods]
impl InstrumentationConfig {
    #[new]
    #[pyo3(signature = (session_id, device_serial, package_id, *, timeout_secs=300, max_output_bytes=10485760, allow_system_hooks=false, enable_logging=true, scripts=None, hooks=None))]
    fn new(
        session_id: String,
        device_serial: String,
        package_id: String,
        timeout_secs: u64,
        max_output_bytes: u64,
        allow_system_hooks: bool,
        enable_logging: bool,
        scripts: Option<Vec<InstrumentationScript>>,
        hooks: Option<Vec<String>>,
    ) -> Self {
        Self {
            session_id,
            device_serial,
            package_id,
            timeout_secs,
            max_output_bytes,
            allow_system_hooks,
            enable_logging,
            scripts: scripts.unwrap_or_default(),
            hooks: hooks.unwrap_or_default(),
        }
    }

    #[getter]
    fn scripts(&self) -> Vec<InstrumentationScript> {
        self.scripts.clone()
    }

    #[getter]
    fn hooks(&self) -> Vec<String> {
        self.hooks.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("device_serial", &self.device_serial)?;
        dict.set_item("package_id", &self.package_id)?;
        dict.set_item("timeout_secs", self.timeout_secs)?;
        dict.set_item("max_output_bytes", self.max_output_bytes)?;
        dict.set_item("allow_system_hooks", self.allow_system_hooks)?;
        dict.set_item("enable_logging", self.enable_logging)?;

        let scripts_list = PyList::empty_bound(py);
        for s in &self.scripts {
            scripts_list.append(s.to_dict(py)?)?;
        }
        dict.set_item("scripts", scripts_list)?;
        dict.set_item("hooks", &self.hooks)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "InstrumentationConfig(session={}, pkg={}, scripts={}, hooks={})",
            self.session_id,
            self.package_id,
            self.scripts.len(),
            self.hooks.len()
        )
    }
}

/// A single instrumentation script with source identity.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentationScript {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub source_hash: String,
    #[pyo3(get)]
    pub script_type: String,
    #[pyo3(get)]
    pub built_in: bool,
    #[pyo3(get)]
    pub description: Option<String>,
    #[pyo3(get)]
    pub target_classes: Vec<String>,
    #[pyo3(get)]
    pub target_methods: Vec<String>,
}

#[pymethods]
impl InstrumentationScript {
    #[new]
    #[pyo3(signature = (name, source_hash, script_type, *, built_in=false, description=None, target_classes=None, target_methods=None))]
    fn new(
        name: String,
        source_hash: String,
        script_type: String,
        built_in: bool,
        description: Option<String>,
        target_classes: Option<Vec<String>>,
        target_methods: Option<Vec<String>>,
    ) -> Self {
        Self {
            name,
            source_hash,
            script_type,
            built_in,
            description,
            target_classes: target_classes.unwrap_or_default(),
            target_methods: target_methods.unwrap_or_default(),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("name", &self.name)?;
        dict.set_item("source_hash", &self.source_hash)?;
        dict.set_item("script_type", &self.script_type)?;
        dict.set_item("built_in", self.built_in)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("target_classes", &self.target_classes)?;
        dict.set_item("target_methods", &self.target_methods)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "InstrumentationScript(name={}, type={}, built_in={})",
            self.name, self.script_type, self.built_in
        )
    }
}

/// An event emitted during instrumentation.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentationEvent {
    #[pyo3(get)]
    pub event_id: String,
    #[pyo3(get)]
    pub session_id: String,
    #[pyo3(get)]
    pub event_type: String,
    #[pyo3(get)]
    pub timestamp_ms: u64,
    #[pyo3(get)]
    pub script_name: Option<String>,
    #[pyo3(get)]
    pub target_class: Option<String>,
    #[pyo3(get)]
    pub target_method: Option<String>,
    #[pyo3(get)]
    pub data: Option<String>,
    #[pyo3(get)]
    pub sequence: u64,
}

#[pymethods]
impl InstrumentationEvent {
    #[new]
    #[pyo3(signature = (event_id, session_id, event_type, timestamp_ms, *, script_name=None, target_class=None, target_method=None, data=None, sequence=0))]
    fn new(
        event_id: String,
        session_id: String,
        event_type: String,
        timestamp_ms: u64,
        script_name: Option<String>,
        target_class: Option<String>,
        target_method: Option<String>,
        data: Option<String>,
        sequence: u64,
    ) -> Self {
        Self {
            event_id,
            session_id,
            event_type,
            timestamp_ms,
            script_name,
            target_class,
            target_method,
            data,
            sequence,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("event_id", &self.event_id)?;
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("event_type", &self.event_type)?;
        dict.set_item("timestamp_ms", self.timestamp_ms)?;
        dict.set_item("script_name", &self.script_name)?;
        dict.set_item("target_class", &self.target_class)?;
        dict.set_item("target_method", &self.target_method)?;
        dict.set_item("data", &self.data)?;
        dict.set_item("sequence", self.sequence)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "InstrumentationEvent(id={}, type={}, seq={})",
            self.event_id, self.event_type, self.sequence
        )
    }
}

/// Result of an instrumentation session.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentationResult {
    #[pyo3(get)]
    pub session_id: String,
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub scripts_loaded: usize,
    #[pyo3(get)]
    pub hooks_installed: usize,
    #[pyo3(get)]
    pub events_captured: usize,
    #[pyo3(get)]
    pub output_bytes: u64,
    #[pyo3(get)]
    pub duration_ms: u64,
    #[pyo3(get)]
    pub timed_out: bool,
    #[pyo3(get)]
    pub error: Option<String>,
    #[pyo3(get)]
    pub output_truncated: bool,
}

#[pymethods]
impl InstrumentationResult {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("success", self.success)?;
        dict.set_item("scripts_loaded", self.scripts_loaded)?;
        dict.set_item("hooks_installed", self.hooks_installed)?;
        dict.set_item("events_captured", self.events_captured)?;
        dict.set_item("output_bytes", self.output_bytes)?;
        dict.set_item("duration_ms", self.duration_ms)?;
        dict.set_item("timed_out", self.timed_out)?;
        dict.set_item("error", &self.error)?;
        dict.set_item("output_truncated", self.output_truncated)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "InstrumentationResult(session={}, success={}, events={})",
            self.session_id, self.success, self.events_captured
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// WS6: Mobile evidence and artifacts
// ═══════════════════════════════════════════════════════════════════

/// Type of mobile evidence.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MobileEvidenceKind {
    Screenshot,
    Log,
    NetworkTrace,
    ApplicationFile,
    ProcessMetadata,
    RuntimePermission,
    DynamicApiObservation,
    CrashTrace,
    InstrumentationOutput,
}

#[pymethods]
impl MobileEvidenceKind {
    fn __repr__(&self) -> String {
        format!("MobileEvidenceKind.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl MobileEvidenceKind {
    fn as_str(&self) -> &str {
        match self {
            MobileEvidenceKind::Screenshot => "Screenshot",
            MobileEvidenceKind::Log => "Log",
            MobileEvidenceKind::NetworkTrace => "NetworkTrace",
            MobileEvidenceKind::ApplicationFile => "ApplicationFile",
            MobileEvidenceKind::ProcessMetadata => "ProcessMetadata",
            MobileEvidenceKind::RuntimePermission => "RuntimePermission",
            MobileEvidenceKind::DynamicApiObservation => "DynamicApiObservation",
            MobileEvidenceKind::CrashTrace => "CrashTrace",
            MobileEvidenceKind::InstrumentationOutput => "InstrumentationOutput",
        }
    }
}

/// A piece of evidence collected during mobile dynamic analysis.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileEvidence {
    #[pyo3(get)]
    pub evidence_id: String,
    #[pyo3(get)]
    pub session_id: String,
    #[pyo3(get)]
    pub kind: MobileEvidenceKind,
    #[pyo3(get)]
    pub device_serial: String,
    #[pyo3(get)]
    pub package_id: String,
    #[pyo3(get)]
    pub timestamp_ms: u64,
    #[pyo3(get)]
    pub content_type: String,
    #[pyo3(get)]
    pub content_hash: Option<String>,
    #[pyo3(get)]
    pub size_bytes: u64,
    #[pyo3(get)]
    pub redacted: bool,
    #[pyo3(get)]
    pub description: Option<String>,
    #[pyo3(get)]
    pub linked_static_evidence: Option<String>,
}

#[pymethods]
impl MobileEvidence {
    #[new]
    #[pyo3(signature = (evidence_id, session_id, kind, device_serial, package_id, timestamp_ms, content_type, *, content_hash=None, size_bytes=0, redacted=false, description=None, linked_static_evidence=None))]
    fn new(
        evidence_id: String,
        session_id: String,
        kind: MobileEvidenceKind,
        device_serial: String,
        package_id: String,
        timestamp_ms: u64,
        content_type: String,
        content_hash: Option<String>,
        size_bytes: u64,
        redacted: bool,
        description: Option<String>,
        linked_static_evidence: Option<String>,
    ) -> Self {
        Self {
            evidence_id,
            session_id,
            kind,
            device_serial,
            package_id,
            timestamp_ms,
            content_type,
            content_hash,
            size_bytes,
            redacted,
            description,
            linked_static_evidence,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("evidence_id", &self.evidence_id)?;
        dict.set_item("session_id", &self.session_id)?;
        dict.set_item("kind", self.kind.as_str())?;
        dict.set_item("device_serial", &self.device_serial)?;
        dict.set_item("package_id", &self.package_id)?;
        dict.set_item("timestamp_ms", self.timestamp_ms)?;
        dict.set_item("content_type", &self.content_type)?;
        dict.set_item("content_hash", &self.content_hash)?;
        dict.set_item("size_bytes", self.size_bytes)?;
        dict.set_item("redacted", self.redacted)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("linked_static_evidence", &self.linked_static_evidence)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "MobileEvidence(id={}, kind={}, pkg={})",
            self.evidence_id,
            self.kind.as_str(),
            self.package_id
        )
    }
}

/// Aggregated evidence collection for a mobile session.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileEvidenceCollection {
    #[pyo3(get)]
    pub session_id: String,
    evidence: Vec<MobileEvidence>,
    #[pyo3(get)]
    pub total_size_bytes: u64,
    #[pyo3(get)]
    pub redacted_count: usize,
}

#[pymethods]
impl MobileEvidenceCollection {
    #[new]
    fn new(session_id: String) -> Self {
        Self {
            session_id,
            evidence: Vec::new(),
            total_size_bytes: 0,
            redacted_count: 0,
        }
    }

    #[getter]
    fn evidence(&self) -> Vec<MobileEvidence> {
        self.evidence.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("session_id", &self.session_id)?;

        let evidence_list = PyList::empty_bound(py);
        for e in &self.evidence {
            evidence_list.append(e.to_dict(py)?)?;
        }
        dict.set_item("evidence", evidence_list)?;
        dict.set_item("total_size_bytes", self.total_size_bytes)?;
        dict.set_item("redacted_count", self.redacted_count)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "MobileEvidenceCollection(session={}, count={}, size={})",
            self.session_id,
            self.evidence.len(),
            self.total_size_bytes
        )
    }
}
