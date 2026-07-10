use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

/// Schema version constant.
pub const FINDING_SCHEMA_VERSION: &str = "1.0";

/// Confidence level for a finding.
#[pyclass(frozen, name = "Confidence")]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfidencePy {
    Confirmed,
    High,
    Medium,
    Low,
    Informational,
}

#[pymethods]
impl ConfidencePy {
    #[staticmethod]
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "confirmed" => Self::Confirmed,
            "high" => Self::High,
            "medium" => Self::Medium,
            "low" => Self::Low,
            "informational" | "info" => Self::Informational,
            _ => Self::Medium,
        }
    }

    fn score(&self) -> f64 {
        match self {
            Self::Confirmed => 1.0,
            Self::High => 0.8,
            Self::Medium => 0.6,
            Self::Low => 0.4,
            Self::Informational => 0.2,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Informational => "informational",
        }
    }

    fn __repr__(&self) -> String {
        format!("Confidence.{:?}", self)
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

/// Type classification for a finding.
#[pyclass(frozen, name = "FindingType")]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FindingTypePy {
    Vulnerability,
    Misconfiguration,
    InformationLeak,
    PolicyViolation,
    AssetDiscovery,
    ServiceDetection,
    WafDetection,
    FuzzResult,
    ScanResult,
}

#[pymethods]
impl FindingTypePy {
    #[staticmethod]
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "vulnerability" => Self::Vulnerability,
            "misconfiguration" => Self::Misconfiguration,
            "information_leak" | "informationleak" => Self::InformationLeak,
            "policy_violation" | "policyviolation" => Self::PolicyViolation,
            "asset_discovery" | "assetdiscovery" => Self::AssetDiscovery,
            "service_detection" | "servicedetection" => Self::ServiceDetection,
            "waf_detection" | "wafdetection" => Self::WafDetection,
            "fuzz_result" | "fuzzresult" => Self::FuzzResult,
            "scan_result" | "scanresult" => Self::ScanResult,
            _ => Self::ScanResult,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Vulnerability => "vulnerability",
            Self::Misconfiguration => "misconfiguration",
            Self::InformationLeak => "information_leak",
            Self::PolicyViolation => "policy_violation",
            Self::AssetDiscovery => "asset_discovery",
            Self::ServiceDetection => "service_detection",
            Self::WafDetection => "waf_detection",
            Self::FuzzResult => "fuzz_result",
            Self::ScanResult => "scan_result",
        }
    }

    fn __repr__(&self) -> String {
        format!("FindingType.{:?}", self)
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

/// Kind of evidence supporting a finding.
#[pyclass(frozen, name = "EvidenceKind")]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvidenceKindPy {
    HttpRequest,
    HttpResponse,
    Header,
    BodySnippet,
    Timing,
    Diff,
    Banner,
    DnsRecord,
    Certificate,
    PortState,
    Screenshot,
    FilePath,
    LogLine,
}

#[pymethods]
impl EvidenceKindPy {
    #[staticmethod]
    fn from_str(s: &str) -> Self {
        match s {
            "HttpRequest" | "http_request" | "httprequest" => Self::HttpRequest,
            "HttpResponse" | "http_response" | "httpresponse" => Self::HttpResponse,
            "Header" | "header" => Self::Header,
            "BodySnippet" | "body_snippet" | "bodysnippet" => Self::BodySnippet,
            "Timing" | "timing" => Self::Timing,
            "Diff" | "diff" => Self::Diff,
            "Banner" | "banner" => Self::Banner,
            "DnsRecord" | "dns_record" | "dnsrecord" => Self::DnsRecord,
            "Certificate" | "certificate" => Self::Certificate,
            "PortState" | "port_state" | "portstate" => Self::PortState,
            "Screenshot" | "screenshot" => Self::Screenshot,
            "FilePath" | "file_path" | "filepath" => Self::FilePath,
            "LogLine" | "log_line" | "logline" => Self::LogLine,
            _ => Self::BodySnippet,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::HttpRequest => "HttpRequest",
            Self::HttpResponse => "HttpResponse",
            Self::Header => "Header",
            Self::BodySnippet => "BodySnippet",
            Self::Timing => "Timing",
            Self::Diff => "Diff",
            Self::Banner => "Banner",
            Self::DnsRecord => "DnsRecord",
            Self::Certificate => "Certificate",
            Self::PortState => "PortState",
            Self::Screenshot => "Screenshot",
            Self::FilePath => "FilePath",
            Self::LogLine => "LogLine",
        }
    }

    fn __repr__(&self) -> String {
        format!("EvidenceKind.{:?}", self)
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

/// Asset affected by a finding.
#[pyclass(frozen, name = "AffectedAsset")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedAssetPy {
    #[pyo3(get)]
    pub asset_type: String,
    #[pyo3(get)]
    pub identifier: String,
    #[pyo3(get)]
    pub host: Option<String>,
    #[pyo3(get)]
    pub port: Option<u16>,
    #[pyo3(get)]
    pub protocol: Option<String>,
}

#[pymethods]
impl AffectedAssetPy {
    #[new]
    #[pyo3(signature = (asset_type, identifier, *, host=None, port=None, protocol=None))]
    fn new(
        asset_type: String,
        identifier: String,
        host: Option<String>,
        port: Option<u16>,
        protocol: Option<String>,
    ) -> Self {
        Self {
            asset_type,
            identifier,
            host,
            port,
            protocol,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("asset_type", &self.asset_type)?;
        dict.set_item("identifier", &self.identifier)?;
        dict.set_item("host", &self.host)?;
        dict.set_item("port", &self.port)?;
        dict.set_item("protocol", &self.protocol)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "AffectedAsset(asset_type={}, identifier={})",
            self.asset_type, self.identifier
        )
    }
}

/// Precise location within the affected asset.
#[pyclass(frozen, name = "FindingLocation")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingLocationPy {
    #[pyo3(get)]
    pub url: Option<String>,
    #[pyo3(get)]
    pub path: Option<String>,
    #[pyo3(get)]
    pub parameter: Option<String>,
    #[pyo3(get)]
    pub header: Option<String>,
    #[pyo3(get)]
    pub method: Option<String>,
    #[pyo3(get)]
    pub line: Option<u32>,
    #[pyo3(get)]
    pub file: Option<String>,
}

#[pymethods]
impl FindingLocationPy {
    #[new]
    #[pyo3(signature = (*, url=None, path=None, parameter=None, header=None, method=None, line=None, file=None))]
    fn new(
        url: Option<String>,
        path: Option<String>,
        parameter: Option<String>,
        header: Option<String>,
        method: Option<String>,
        line: Option<u32>,
        file: Option<String>,
    ) -> Self {
        Self {
            url,
            path,
            parameter,
            header,
            method,
            line,
            file,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("url", &self.url)?;
        dict.set_item("path", &self.path)?;
        dict.set_item("parameter", &self.parameter)?;
        dict.set_item("header", &self.header)?;
        dict.set_item("method", &self.method)?;
        dict.set_item("line", &self.line)?;
        dict.set_item("file", &self.file)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        let parts: Vec<String> = [
            self.url.as_ref().map(|v| format!("url={v}")),
            self.path.as_ref().map(|v| format!("path={v}")),
            self.parameter.as_ref().map(|v| format!("parameter={v}")),
            self.header.as_ref().map(|v| format!("header={v}")),
            self.method.as_ref().map(|v| format!("method={v}")),
            self.line.map(|v| format!("line={v}")),
            self.file.as_ref().map(|v| format!("file={v}")),
        ]
        .into_iter()
        .flatten()
        .collect();
        format!("FindingLocation({})", parts.join(", "))
    }
}

/// Evidence attached to a versioned finding.
#[pyclass(frozen, name = "VersionedEvidence")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedEvidencePy {
    #[pyo3(get)]
    pub kind: EvidenceKindPy,
    #[pyo3(get)]
    pub redacted: bool,
    #[pyo3(get)]
    pub summary: String,
    #[pyo3(get)]
    pub data: String,
}

#[pymethods]
impl VersionedEvidencePy {
    #[new]
    #[pyo3(signature = (kind, summary, *, data=None, redacted=None))]
    fn new(
        kind: EvidenceKindPy,
        summary: String,
        data: Option<String>,
        redacted: Option<bool>,
    ) -> Self {
        Self {
            kind,
            summary,
            data: data.unwrap_or_default(),
            redacted: redacted.unwrap_or(false),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("kind", self.kind.as_str())?;
        dict.set_item("summary", &self.summary)?;
        dict.set_item("data", &self.data)?;
        dict.set_item("redacted", self.redacted)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "VersionedEvidence(kind={}, summary={})",
            self.kind.as_str(),
            self.summary
        )
    }
}

/// A finding with full schema versioning and structured fields.
#[pyclass(frozen, name = "VersionedFinding")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedFindingPy {
    #[pyo3(get)]
    pub schema_version: String,
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub fingerprint: String,
    #[pyo3(get)]
    pub title: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub severity: String,
    #[pyo3(get)]
    pub confidence: ConfidencePy,
    #[pyo3(get)]
    pub finding_type: FindingTypePy,
    #[pyo3(get)]
    pub cwe: Option<String>,
    #[pyo3(get)]
    pub owasp: Option<String>,
    #[pyo3(get)]
    pub cve: Option<String>,
    #[pyo3(get)]
    pub affected_asset: AffectedAssetPy,
    #[pyo3(get)]
    pub location: FindingLocationPy,
    #[pyo3(get)]
    pub evidence: Vec<VersionedEvidencePy>,
    #[pyo3(get)]
    pub remediation: Option<String>,
    #[pyo3(get)]
    pub tags: Vec<String>,
    #[pyo3(get)]
    pub discovered_at: String,
    #[pyo3(get)]
    pub source_tool: String,
    #[pyo3(get)]
    pub source_module: String,
    #[pyo3(get)]
    pub metadata: String,
}

#[pymethods]
impl VersionedFindingPy {
    #[new]
    #[pyo3(signature = (id, title, description, severity, finding_type, affected_asset, source_tool, source_module, *, fingerprint=None, confidence=None, cwe=None, owasp=None, cve=None, location=None, evidence=None, remediation=None, tags=None, discovered_at=None, metadata=None))]
    fn new(
        id: String,
        title: String,
        description: String,
        severity: String,
        finding_type: FindingTypePy,
        affected_asset: AffectedAssetPy,
        source_tool: String,
        source_module: String,
        fingerprint: Option<String>,
        confidence: Option<ConfidencePy>,
        cwe: Option<String>,
        owasp: Option<String>,
        cve: Option<String>,
        location: Option<FindingLocationPy>,
        evidence: Option<Vec<VersionedEvidencePy>>,
        remediation: Option<String>,
        tags: Option<Vec<String>>,
        discovered_at: Option<String>,
        metadata: Option<String>,
    ) -> Self {
        let discovered_at = discovered_at
            .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());
        let location = location.unwrap_or_default();

        Self {
            schema_version: FINDING_SCHEMA_VERSION.to_string(),
            id,
            fingerprint: fingerprint.unwrap_or_default(),
            title,
            description,
            severity,
            confidence: confidence.unwrap_or(ConfidencePy::Medium),
            finding_type,
            cwe,
            owasp,
            cve,
            affected_asset,
            location,
            evidence: evidence.unwrap_or_default(),
            remediation,
            tags: tags.unwrap_or_default(),
            discovered_at,
            source_tool,
            source_module,
            metadata: metadata.unwrap_or_else(|| "{}".to_string()),
        }
    }

    /// Compute a deterministic fingerprint from key finding fields.
    fn compute_fingerprint(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.title.hash(&mut hasher);
        self.severity.hash(&mut hasher);
        self.affected_asset.identifier.hash(&mut hasher);
        self.location.url.hash(&mut hasher);
        self.location.path.hash(&mut hasher);
        self.location.parameter.hash(&mut hasher);
        self.source_tool.hash(&mut hasher);
        self.source_module.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("schema_version", &self.schema_version)?;
        dict.set_item("id", &self.id)?;
        dict.set_item("fingerprint", &self.fingerprint)?;
        dict.set_item("title", &self.title)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("severity", &self.severity)?;
        dict.set_item("confidence", self.confidence.as_str())?;
        dict.set_item("finding_type", self.finding_type.as_str())?;
        dict.set_item("cwe", &self.cwe)?;
        dict.set_item("owasp", &self.owasp)?;
        dict.set_item("cve", &self.cve)?;
        dict.set_item("affected_asset", self.affected_asset.to_dict(py)?)?;
        dict.set_item("location", self.location.to_dict(py)?)?;

        let evidence_list = PyList::empty_bound(py);
        for e in &self.evidence {
            evidence_list.append(e.to_dict(py)?)?;
        }
        dict.set_item("evidence", evidence_list)?;

        dict.set_item("remediation", &self.remediation)?;
        dict.set_item("tags", &self.tags)?;
        dict.set_item("discovered_at", &self.discovered_at)?;
        dict.set_item("source_tool", &self.source_tool)?;
        dict.set_item("source_module", &self.source_module)?;
        dict.set_item("metadata", &self.metadata)?;

        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    #[staticmethod]
    fn schema_version() -> &'static str {
        FINDING_SCHEMA_VERSION
    }

    fn __repr__(&self) -> String {
        format!(
            "VersionedFinding(id={}, severity={}, title={})",
            self.id, self.severity, self.title
        )
    }
}

impl Default for FindingLocationPy {
    fn default() -> Self {
        Self {
            url: None,
            path: None,
            parameter: None,
            header: None,
            method: None,
            line: None,
            file: None,
        }
    }
}
