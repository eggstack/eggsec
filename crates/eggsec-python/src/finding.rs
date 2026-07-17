use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

/// Severity level for findings.
#[pyclass(frozen, eq, hash)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[pymethods]
impl Severity {
    fn __repr__(&self) -> String {
        format!("Severity.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }

    #[staticmethod]
    fn from_str(s: &str) -> PyResult<Self> {
        match s.to_lowercase().as_str() {
            "critical" => Ok(Severity::Critical),
            "high" => Ok(Severity::High),
            "medium" => Ok(Severity::Medium),
            "low" => Ok(Severity::Low),
            "info" | "informational" => Ok(Severity::Info),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid severity: '{}'. Must be one of: critical, high, medium, low, info",
                s
            ))),
        }
    }
}

impl Severity {
    pub fn as_str(&self) -> &str {
        match self {
            Severity::Critical => "Critical",
            Severity::High => "High",
            Severity::Medium => "Medium",
            Severity::Low => "Low",
            Severity::Info => "Info",
        }
    }

    /// Convert from engine Severity.
    pub fn from_engine(engine: eggsec_core::types::Severity) -> Self {
        match engine {
            eggsec_core::types::Severity::Critical => Severity::Critical,
            eggsec_core::types::Severity::High => Severity::High,
            eggsec_core::types::Severity::Medium => Severity::Medium,
            eggsec_core::types::Severity::Low => Severity::Low,
            eggsec_core::types::Severity::Info => Severity::Info,
        }
    }

    /// Convert to engine Severity.
    pub fn to_engine(&self) -> eggsec_core::types::Severity {
        match self {
            Severity::Critical => eggsec_core::types::Severity::Critical,
            Severity::High => eggsec_core::types::Severity::High,
            Severity::Medium => eggsec_core::types::Severity::Medium,
            Severity::Low => eggsec_core::types::Severity::Low,
            Severity::Info => eggsec_core::types::Severity::Info,
        }
    }
}

/// Evidence supporting a finding.
#[pyclass(frozen)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    #[pyo3(get)]
    pub kind: String,
    #[pyo3(get)]
    pub value: String,
    #[pyo3(get)]
    pub source: String,
    #[pyo3(get)]
    pub confidence: f64,
    metadata: std::collections::HashMap<String, String>,
}

#[pymethods]
impl Evidence {
    #[new]
    #[pyo3(signature = (kind, value, source, *, confidence=1.0, metadata=None))]
    fn new(
        kind: String,
        value: String,
        source: String,
        confidence: f64,
        metadata: Option<std::collections::HashMap<String, String>>,
    ) -> Self {
        Self {
            kind,
            value,
            source,
            confidence,
            metadata: metadata.unwrap_or_default(),
        }
    }

    #[getter]
    fn metadata(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        for (k, v) in &self.metadata {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("kind", &self.kind)?;
        dict.set_item("value", "[REDACTED]")?;
        dict.set_item("source", &self.source)?;
        dict.set_item("confidence", self.confidence)?;
        let meta_dict = PyDict::new_bound(py);
        for (k, v) in &self.metadata {
            meta_dict.set_item(k, v)?;
        }
        dict.set_item("metadata", meta_dict)?;
        Ok(dict.into())
    }

    fn to_dict_raw(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("kind", &self.kind)?;
        dict.set_item("value", &self.value)?;
        dict.set_item("source", &self.source)?;
        dict.set_item("confidence", self.confidence)?;
        let meta_dict = PyDict::new_bound(py);
        for (k, v) in &self.metadata {
            meta_dict.set_item(k, v)?;
        }
        dict.set_item("metadata", meta_dict)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        let redacted = Evidence {
            kind: self.kind.clone(),
            value: "[REDACTED]".to_string(),
            source: self.source.clone(),
            confidence: self.confidence,
            metadata: self.metadata.clone(),
        };
        serde_json::to_string(&redacted)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn to_json_raw(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("Evidence(kind={}, source={})", self.kind, self.source)
    }
}

/// A security finding.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub title: String,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub category: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub recommendation: Option<String>,
    evidence_items: Vec<Evidence>,
    metadata: std::collections::HashMap<String, String>,
}

#[pymethods]
impl Finding {
    #[new]
    #[pyo3(signature = (id, title, severity, target, category, description, *, recommendation=None, evidence=None, metadata=None))]
    fn new(
        id: String,
        title: String,
        severity: Severity,
        target: String,
        category: String,
        description: String,
        recommendation: Option<String>,
        evidence: Option<Vec<Evidence>>,
        metadata: Option<std::collections::HashMap<String, String>>,
    ) -> Self {
        Self {
            id,
            title,
            severity,
            target,
            category,
            description,
            recommendation,
            evidence_items: evidence.unwrap_or_default(),
            metadata: metadata.unwrap_or_default(),
        }
    }

    #[getter]
    fn evidence(&self) -> Vec<Evidence> {
        self.evidence_items.clone()
    }

    #[getter]
    fn metadata(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        for (k, v) in &self.metadata {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    /// Convert to a Python dictionary (redacted by default).
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("title", "[REDACTED]")?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("target", &self.target)?;
        dict.set_item("category", &self.category)?;
        dict.set_item("description", "[REDACTED]")?;
        dict.set_item("recommendation", &self.recommendation)?;

        let evidence_list = PyList::empty_bound(py);
        for e in &self.evidence_items {
            let ev_dict = PyDict::new_bound(py);
            ev_dict.set_item("kind", &e.kind)?;
            ev_dict.set_item("value", "[REDACTED]")?;
            ev_dict.set_item("source", &e.source)?;
            ev_dict.set_item("confidence", e.confidence)?;
            let meta_dict = PyDict::new_bound(py);
            for (k, v) in &e.metadata {
                meta_dict.set_item(k, v)?;
            }
            ev_dict.set_item("metadata", meta_dict)?;
            evidence_list.append(ev_dict)?;
        }
        dict.set_item("evidence", evidence_list)?;

        let meta_dict = PyDict::new_bound(py);
        for (k, _v) in &self.metadata {
            meta_dict.set_item(k, "[REDACTED]")?;
        }
        dict.set_item("metadata", meta_dict)?;

        Ok(dict.into())
    }

    /// Convert to a Python dictionary with raw (unredacted) values.
    fn to_dict_raw(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("title", &self.title)?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("target", &self.target)?;
        dict.set_item("category", &self.category)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("recommendation", &self.recommendation)?;

        let evidence_list = PyList::empty_bound(py);
        for e in &self.evidence_items {
            evidence_list.append(e.to_dict_raw(py)?)?;
        }
        dict.set_item("evidence", evidence_list)?;

        let meta_dict = PyDict::new_bound(py);
        for (k, v) in &self.metadata {
            meta_dict.set_item(k, v)?;
        }
        dict.set_item("metadata", meta_dict)?;

        Ok(dict.into())
    }

    /// Convert to a JSON string (redacted by default).
    fn to_json(&self) -> PyResult<String> {
        let redacted_evidence: Vec<Evidence> = self
            .evidence_items
            .iter()
            .map(|e| Evidence {
                kind: e.kind.clone(),
                value: "[REDACTED]".to_string(),
                source: e.source.clone(),
                confidence: e.confidence,
                metadata: e.metadata.clone(),
            })
            .collect();
        let redacted_metadata: std::collections::HashMap<String, String> = self
            .metadata
            .keys()
            .map(|k| (k.clone(), "[REDACTED]".to_string()))
            .collect();
        let redacted = Finding {
            id: self.id.clone(),
            title: "[REDACTED]".to_string(),
            severity: self.severity,
            target: self.target.clone(),
            category: self.category.clone(),
            description: "[REDACTED]".to_string(),
            recommendation: self.recommendation.clone(),
            evidence_items: redacted_evidence,
            metadata: redacted_metadata,
        };
        serde_json::to_string(&redacted)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Convert to a JSON string with raw (unredacted) values.
    fn to_json_raw(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Convert to a row (list of key-value pairs) for tabular output.
    fn to_row(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("title", &self.title)?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("target", &self.target)?;
        dict.set_item("category", &self.category)?;
        dict.set_item("description", &self.description)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!("Finding(id={})", self.id)
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.id.hash(&mut hasher);
        self.title.hash(&mut hasher);
        self.target.hash(&mut hasher);
        hasher.finish()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.id == other.id && self.title == other.title && self.target == other.target
    }

    fn __bool__(&self) -> bool {
        true
    }
}

/// A collection of findings.
#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingSet {
    findings: Vec<Finding>,
}

/// Python iterator for FindingSet — yields items one at a time.
#[pyclass(name = "FindingSetIterator")]
pub struct FindingSetIteratorPy {
    findings: Vec<Finding>,
    index: usize,
}

#[pymethods]
impl FindingSetIteratorPy {
    #[new]
    fn new(findings: Vec<Finding>) -> Self {
        Self { findings, index: 0 }
    }

    fn __iter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __next__<'py>(mut slf: PyRefMut<'py, Self>, py: Python<'py>) -> PyResult<Option<Finding>> {
        if slf.index >= slf.findings.len() {
            return Ok(None);
        }
        let finding = slf.findings[slf.index].clone();
        slf.index += 1;
        Ok(Some(finding))
    }

    fn __len__(&self) -> usize {
        self.findings.len() - self.index
    }
}

#[pymethods]
impl FindingSet {
    #[new]
    fn new() -> Self {
        Self {
            findings: Vec::new(),
        }
    }

    /// Add a finding to the set.
    fn add_finding(&mut self, finding: Finding) {
        self.findings.push(finding);
    }

    /// Get the number of findings.
    fn __len__(&self) -> usize {
        self.findings.len()
    }

    /// Get findings by severity.
    fn by_severity(&self, severity: Severity) -> Vec<Finding> {
        self.findings
            .iter()
            .filter(|f| f.severity == severity)
            .cloned()
            .collect()
    }

    /// Get all findings.
    #[getter]
    fn findings(&self) -> Vec<Finding> {
        self.findings.clone()
    }

    /// Filter findings by severity, returning a new FindingSet.
    fn filter_by_severity(&self, severity: Severity) -> FindingSet {
        FindingSet {
            findings: self
                .findings
                .iter()
                .filter(|f| f.severity == severity)
                .cloned()
                .collect(),
        }
    }

    /// Filter findings by category/type, returning a new FindingSet.
    fn filter_by_type(&self, finding_type: &str) -> FindingSet {
        FindingSet {
            findings: self
                .findings
                .iter()
                .filter(|f| f.category == finding_type)
                .cloned()
                .collect(),
        }
    }

    /// Convert to a list of dicts (materializes all findings).
    fn to_dicts(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for f in &self.findings {
            list.append(f.to_dict(py)?)?;
        }
        Ok(list.into())
    }

    /// Convert to rows for tabular output.
    fn to_rows(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for f in &self.findings {
            list.append(f.to_row(py)?)?;
        }
        Ok(list.into())
    }

    /// Create a lazy iterator that yields findings one at a time.
    fn iter_lazy(&self) -> FindingSetIteratorPy {
        FindingSetIteratorPy::new(self.findings.clone())
    }

    fn __repr__(&self) -> String {
        format!("FindingSet(findings={})", self.findings.len())
    }

    /// Iterate over findings.
    fn __iter__<'py>(slf: PyRef<'py, Self>, py: Python<'py>) -> PyResult<PyObject> {
        let findings = slf.findings.clone();
        Ok(FindingSetIteratorPy::new(findings).into_py(py))
    }

    /// Check if a finding with the given id exists in the set.
    fn __contains__(&self, finding: &Finding) -> bool {
        self.findings.iter().any(|f| f.id == finding.id)
    }
}

/// A report aggregating multiple scan results.
#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    findings: Vec<Finding>,
    metadata: std::collections::HashMap<String, String>,
}

#[pymethods]
impl Report {
    #[new]
    #[pyo3(signature = (metadata=None))]
    fn new(metadata: Option<std::collections::HashMap<String, String>>) -> Self {
        Self {
            findings: Vec::new(),
            metadata: metadata.unwrap_or_default(),
        }
    }

    /// Add a finding to the report.
    fn add_finding(&mut self, finding: Finding) {
        self.findings.push(finding);
    }

    /// Add findings from a FindingSet.
    fn add_finding_set(&mut self, finding_set: FindingSet) {
        self.findings.extend(finding_set.findings);
    }

    /// Add results from a PortScanResult, converting open ports to findings.
    fn add_result(&mut self, result: &Bound<'_, PyAny>) -> PyResult<()> {
        // Try to extract findings from various result types
        if let Ok(port_result) = result.extract::<crate::dto::PortScanResult>() {
            // Access open_ports via Python getter since it's a private field with #[getter]
            let open_ports_py = result.getattr("open_ports")?;
            let open_ports: Vec<crate::dto::OpenPort> = open_ports_py.extract()?;
            for port in open_ports {
                let finding = Finding::new(
                    format!("port-open-{}", port.port),
                    format!("Open port {}/{}", port.port, port.protocol),
                    Severity::Info,
                    port_result.target.clone(),
                    "port-scan".to_string(),
                    format!("Port {} is open (service: {})", port.port, port.service),
                    None,
                    None,
                    None,
                );
                self.findings.push(finding);
            }
        } else if let Ok(endpoint_result) = result.extract::<crate::endpoint::EndpointScanResult>()
        {
            // Access findings via Python getter since it's a private field with #[getter]
            let findings_py = result.getattr("findings")?;
            let endpoint_findings: Vec<crate::endpoint::EndpointFinding> = findings_py.extract()?;
            for f in endpoint_findings {
                let finding = Finding::new(
                    format!("endpoint-{}", f.path),
                    format!("Endpoint found: {}", f.path),
                    Severity::Info,
                    endpoint_result.base_url.clone(),
                    "endpoint-discovery".to_string(),
                    format!("HTTP {} at {}", f.status_code, f.path),
                    None,
                    None,
                    None,
                );
                self.findings.push(finding);
            }
        } else if let Ok(fp_result) = result.extract::<crate::fingerprint::FingerprintScanResult>()
        {
            // Access services via Python getter since it's a private field with #[getter]
            let services_py = result.getattr("services")?;
            let services: Vec<crate::fingerprint::ServiceFingerprintResult> =
                services_py.extract()?;
            for svc in services {
                let finding = Finding::new(
                    format!("service-{}", svc.port),
                    format!("Service detected: {}", svc.service),
                    Severity::Info,
                    fp_result.target.clone(),
                    "service-detection".to_string(),
                    format!(
                        "Service '{}' detected on port {} (confidence: {})",
                        svc.service, svc.port, svc.confidence
                    ),
                    None,
                    None,
                    None,
                );
                self.findings.push(finding);
            }
        } else if let Ok(waf_result) = result.extract::<crate::waf::WafDetectionResultPy>() {
            if waf_result.detected {
                let severity = match waf_result.confidence {
                    80..=100 => Severity::High,
                    50..=79 => Severity::Medium,
                    _ => Severity::Low,
                };
                let vendor_str = waf_result
                    .vendor
                    .clone()
                    .unwrap_or_else(|| "Unknown".to_string());
                let finding = Finding::new(
                    format!("waf-{}", waf_result.url),
                    format!("WAF detected: {}", vendor_str),
                    severity,
                    waf_result.url.clone(),
                    "waf-detection".to_string(),
                    format!(
                        "WAF '{}' detected with {}% confidence",
                        vendor_str, waf_result.confidence
                    ),
                    None,
                    None,
                    None,
                );
                self.findings.push(finding);
            }
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "Unsupported result type. Expected PortScanResult, EndpointScanResult, FingerprintScanResult, or WafDetectionResult.",
            ));
        }
        Ok(())
    }

    /// Get the number of findings.
    fn __len__(&self) -> usize {
        self.findings.len()
    }

    /// Get all findings.
    #[getter]
    fn findings(&self) -> Vec<Finding> {
        self.findings.clone()
    }

    /// Get report metadata.
    #[getter]
    fn metadata(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        for (k, v) in &self.metadata {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    /// Convert to a Python dictionary (redacted by default).
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        let findings_list = PyList::empty_bound(py);
        for f in &self.findings {
            findings_list.append(f.to_dict(py)?)?;
        }
        dict.set_item("findings", findings_list)?;
        let meta_dict = PyDict::new_bound(py);
        for (k, _v) in &self.metadata {
            meta_dict.set_item(k, "[REDACTED]")?;
        }
        dict.set_item("metadata", meta_dict)?;
        Ok(dict.into())
    }

    /// Convert to a Python dictionary with raw (unredacted) values.
    fn to_dict_raw(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        let findings_list = PyList::empty_bound(py);
        for f in &self.findings {
            findings_list.append(f.to_dict_raw(py)?)?;
        }
        dict.set_item("findings", findings_list)?;
        let meta_dict = PyDict::new_bound(py);
        for (k, v) in &self.metadata {
            meta_dict.set_item(k, v)?;
        }
        dict.set_item("metadata", meta_dict)?;
        Ok(dict.into())
    }

    /// Convert to a JSON string (redacted by default).
    fn to_json(&self) -> PyResult<String> {
        let redacted_findings: Vec<Finding> = self
            .findings
            .iter()
            .map(|f| {
                let redacted_evidence: Vec<Evidence> = f
                    .evidence_items
                    .iter()
                    .map(|e| Evidence {
                        kind: e.kind.clone(),
                        value: "[REDACTED]".to_string(),
                        source: e.source.clone(),
                        confidence: e.confidence,
                        metadata: e.metadata.clone(),
                    })
                    .collect();
                let redacted_metadata: std::collections::HashMap<String, String> = f
                    .metadata
                    .keys()
                    .map(|k| (k.clone(), "[REDACTED]".to_string()))
                    .collect();
                Finding {
                    id: f.id.clone(),
                    title: "[REDACTED]".to_string(),
                    severity: f.severity,
                    target: f.target.clone(),
                    category: f.category.clone(),
                    description: "[REDACTED]".to_string(),
                    recommendation: f.recommendation.clone(),
                    evidence_items: redacted_evidence,
                    metadata: redacted_metadata,
                }
            })
            .collect();
        let redacted_metadata: std::collections::HashMap<String, String> = self
            .metadata
            .keys()
            .map(|k| (k.clone(), "[REDACTED]".to_string()))
            .collect();
        let redacted = Report {
            findings: redacted_findings,
            metadata: redacted_metadata,
        };
        serde_json::to_string(&redacted)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Convert to a JSON string with raw (unredacted) values.
    fn to_json_raw(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Convert to rows for tabular output (suitable for pandas).
    fn to_rows(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for f in &self.findings {
            list.append(f.to_row(py)?)?;
        }
        Ok(list.into())
    }

    /// Write report to a JSON file.
    fn write_json(&self, path: &str) -> PyResult<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        std::fs::write(path, json)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(())
    }

    /// Write report to a Markdown file.
    fn write_markdown(&self, path: &str) -> PyResult<()> {
        let mut md = String::from("# Eggsec Report\n\n");

        // Summary table
        let mut severity_counts = std::collections::HashMap::new();
        for f in &self.findings {
            *severity_counts.entry(f.severity.as_str()).or_insert(0u32) += 1;
        }

        md.push_str("## Summary\n\n");
        md.push_str("| Severity | Count |\n");
        md.push_str("|----------|-------|\n");
        for sev in &["Critical", "High", "Medium", "Low", "Info"] {
            let count = severity_counts.get(*sev).unwrap_or(&0);
            md.push_str(&format!("| {} | {} |\n", sev, count));
        }
        md.push('\n');

        // Findings table
        if !self.findings.is_empty() {
            md.push_str("## Findings\n\n");
            md.push_str("| ID | Severity | Title | Target | Category |\n");
            md.push_str("|-----|----------|-------|--------|----------|\n");
            for f in &self.findings {
                md.push_str(&format!(
                    "| {} | {} | {} | {} | {} |\n",
                    f.id,
                    f.severity.as_str(),
                    f.title,
                    f.target,
                    f.category
                ));
            }
            md.push('\n');

            // Detailed findings
            md.push_str("## Detailed Findings\n\n");
            for f in &self.findings {
                md.push_str(&format!("### {} [{}]\n\n", f.title, f.severity.as_str()));
                md.push_str(&format!("**ID:** {}\n\n", f.id));
                md.push_str(&format!("**Target:** {}\n\n", f.target));
                md.push_str(&format!("**Category:** {}\n\n", f.category));
                md.push_str(&format!("{}\n\n", f.description));
                if let Some(ref rec) = f.recommendation {
                    md.push_str(&format!("**Recommendation:** {}\n\n", rec));
                }
                if !f.evidence_items.is_empty() {
                    md.push_str("**Evidence:**\n\n");
                    for e in &f.evidence_items {
                        md.push_str(&format!(
                            "- [{}] {} (source: {}, confidence: {:.0}%)\n",
                            e.kind,
                            e.value,
                            e.source,
                            e.confidence * 100.0
                        ));
                    }
                    md.push('\n');
                }
            }
        }

        std::fs::write(path, md)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!("Report(findings={})", self.findings.len())
    }
}
