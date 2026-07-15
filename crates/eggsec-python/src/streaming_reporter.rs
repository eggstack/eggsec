use md5::{Digest, Md5};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::time::Instant;

/// Configuration for a streaming report session.
#[pyclass(frozen, name = "StreamingReportConfig")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingReportConfigPy {
    #[pyo3(get)]
    pub format: String,
    #[pyo3(get)]
    pub output_path: Option<String>,
    #[pyo3(get)]
    pub buffer_size: usize,
    #[pyo3(get)]
    pub include_artifacts: bool,
    #[pyo3(get)]
    pub include_evidence: bool,
    #[pyo3(get)]
    pub redact_secrets: bool,
    #[pyo3(get)]
    pub timestamp_format: String,
}

#[pymethods]
impl StreamingReportConfigPy {
    #[new]
    #[pyo3(signature = (format, *, output_path=None, buffer_size=100, include_artifacts=false, include_evidence=false, redact_secrets=true, timestamp_format=None))]
    fn new(
        format: String,
        output_path: Option<String>,
        buffer_size: usize,
        include_artifacts: bool,
        include_evidence: bool,
        redact_secrets: bool,
        timestamp_format: Option<String>,
    ) -> Self {
        Self {
            format,
            output_path,
            buffer_size,
            include_artifacts,
            include_evidence,
            redact_secrets,
            timestamp_format: timestamp_format.unwrap_or_else(|| "rfc3339".to_string()),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("format", &self.format)?;
        dict.set_item("output_path", &self.output_path)?;
        dict.set_item("buffer_size", self.buffer_size)?;
        dict.set_item("include_artifacts", self.include_artifacts)?;
        dict.set_item("include_evidence", self.include_evidence)?;
        dict.set_item("redact_secrets", self.redact_secrets)?;
        dict.set_item("timestamp_format", &self.timestamp_format)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    fn __repr__(&self) -> String {
        format!(
            "StreamingReportConfig(format={}, buffer_size={}, redact_secrets={})",
            self.format, self.buffer_size, self.redact_secrets,
        )
    }
}

/// Summary produced when a streaming report is finalized.
#[pyclass(frozen, name = "ReportSummary")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummaryPy {
    #[pyo3(get)]
    pub format: String,
    #[pyo3(get)]
    pub total_findings: usize,
    #[pyo3(get)]
    pub findings_by_severity: Vec<(String, usize)>,
    #[pyo3(get)]
    pub output_path: Option<String>,
    #[pyo3(get)]
    pub output_size_bytes: u64,
    #[pyo3(get)]
    pub duration_ms: u64,
    #[pyo3(get)]
    pub content_hash: Option<String>,
}

#[pymethods]
impl ReportSummaryPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("format", &self.format)?;
        dict.set_item("total_findings", self.total_findings)?;

        let severity_list = PyList::empty_bound(py);
        for (severity, count) in &self.findings_by_severity {
            let item = PyDict::new_bound(py);
            item.set_item("severity", severity)?;
            item.set_item("count", count)?;
            severity_list.append(item)?;
        }
        dict.set_item("findings_by_severity", severity_list)?;

        dict.set_item("output_path", &self.output_path)?;
        dict.set_item("output_size_bytes", self.output_size_bytes)?;
        dict.set_item("duration_ms", self.duration_ms)?;
        dict.set_item("content_hash", &self.content_hash)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    fn __repr__(&self) -> String {
        format!(
            "ReportSummary(format={}, total_findings={}, output_size_bytes={}, duration_ms={})",
            self.format, self.total_findings, self.output_size_bytes, self.duration_ms,
        )
    }
}

/// Streaming report writer that buffers findings and writes incrementally.
#[pyclass(name = "StreamingReporter")]
pub struct StreamingReporterPy {
    config: StreamingReportConfigPy,
    buffer: Vec<String>,
    started: bool,
    start_time: Option<Instant>,
    output_file: Option<File>,
    output_size: u64,
    hasher: Md5,
}

#[pymethods]
impl StreamingReporterPy {
    #[new]
    fn new(config: StreamingReportConfigPy) -> Self {
        Self {
            config,
            buffer: Vec::new(),
            started: false,
            start_time: None,
            output_file: None,
            output_size: 0,
            hasher: Md5::new(),
        }
    }

    /// Initialize the streaming reporter, opening the output file if configured.
    fn start(&mut self) -> PyResult<()> {
        if self.started {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "StreamingReporter already started",
            ));
        }

        self.start_time = Some(Instant::now());

        if let Some(ref path) = self.config.output_path {
            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(path)
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            self.output_file = Some(file);
        }

        self.started = true;
        Ok(())
    }

    /// Append a single finding (JSON string) to the streaming buffer.
    fn write_finding(&mut self, finding_json: &str) -> PyResult<()> {
        if !self.started {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "StreamingReporter not started; call start() first",
            ));
        }

        self.hasher.update(finding_json.as_bytes());
        self.buffer.push(finding_json.to_string());

        if self.buffer.len() >= self.config.buffer_size {
            self.flush_buffer()?;
        }

        Ok(())
    }

    /// Append multiple findings (JSON array string) to the streaming buffer.
    fn write_findings_batch(&mut self, findings_json: &str) -> PyResult<()> {
        if !self.started {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "StreamingReporter not started; call start() first",
            ));
        }

        let parsed: serde_json::Value = serde_json::from_str(findings_json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        match parsed {
            serde_json::Value::Array(items) => {
                for item in &items {
                    let line = serde_json::to_string(item)
                        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
                    self.hasher.update(line.as_bytes());
                    self.buffer.push(line);
                }
            }
            _ => {
                self.hasher.update(findings_json.as_bytes());
                self.buffer.push(findings_json.to_string());
            }
        }

        if self.buffer.len() >= self.config.buffer_size {
            self.flush_buffer()?;
        }

        Ok(())
    }

    /// Return the number of findings currently held in the buffer.
    fn get_buffered_count(&self) -> usize {
        self.buffer.len()
    }

    /// Flush the internal buffer to the output file (if configured).
    fn flush(&mut self) -> PyResult<()> {
        self.flush_buffer()
    }

    /// Finalize the report and return a summary of the streaming session.
    fn finish(&mut self) -> PyResult<ReportSummaryPy> {
        if !self.started {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "StreamingReporter not started",
            ));
        }

        self.flush_buffer()?;

        let total_findings = self.buffer.len();

        // Count severities by re-parsing buffered findings
        let mut severity_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for line in &self.buffer {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(sev) = val.get("severity").and_then(|v| v.as_str()) {
                    *severity_counts.entry(sev.to_lowercase()).or_insert(0) += 1;
                }
            }
        }

        let mut findings_by_severity: Vec<(String, usize)> =
            severity_counts.into_iter().collect::<Vec<_>>();
        findings_by_severity.sort_by(|a, b| b.1.cmp(&a.1));

        let duration_ms = self
            .start_time
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);

        let content_hash = format!("{:x}", self.hasher.finalize_reset());

        // Close the file handle explicitly
        self.output_file = None;

        Ok(ReportSummaryPy {
            format: self.config.format.clone(),
            total_findings,
            findings_by_severity,
            output_path: self.config.output_path.clone(),
            output_size_bytes: self.output_size,
            duration_ms,
            content_hash: Some(content_hash),
        })
    }

    /// Return the configured output path, if any.
    fn get_output_path(&self) -> Option<String> {
        self.config.output_path.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("format", &self.config.format)?;
        dict.set_item("output_path", &self.config.output_path)?;
        dict.set_item("buffered_count", self.buffer.len())?;
        dict.set_item("started", self.started)?;
        dict.set_item("output_size_bytes", self.output_size)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(&self.config).unwrap_or_default()
    }

    fn __repr__(&self) -> String {
        format!(
            "StreamingReporter(format={}, started={}, buffered={})",
            self.config.format,
            self.started,
            self.buffer.len(),
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl StreamingReporterPy {
    fn flush_buffer(&mut self) -> PyResult<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        if let Some(ref mut file) = self.output_file {
            for line in &self.buffer {
                file.write_all(line.as_bytes())
                    .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
                file.write_all(b"\n")
                    .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            }
            file.flush()
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        }

        self.output_size += self.buffer.iter().map(|l| l.len() as u64 + 1).sum::<u64>();
        self.buffer.clear();
        Ok(())
    }
}

/// Result of diffing a single finding against a baseline.
#[pyclass(frozen, name = "FindingDiffResult")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingDiffResultPy {
    #[pyo3(get)]
    pub finding_id: String,
    #[pyo3(get)]
    pub diff_status: String,
    #[pyo3(get)]
    pub baseline_finding_id: Option<String>,
    #[pyo3(get)]
    pub changes: Vec<String>,
}

#[pymethods]
impl FindingDiffResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("finding_id", &self.finding_id)?;
        dict.set_item("diff_status", &self.diff_status)?;
        dict.set_item("baseline_finding_id", &self.baseline_finding_id)?;
        dict.set_item("changes", &self.changes)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    fn __repr__(&self) -> String {
        format!(
            "FindingDiffResult(id={}, status={})",
            self.finding_id, self.diff_status,
        )
    }
}

/// Summary produced when a diff report is finalized.
#[pyclass(frozen, name = "DiffReportSummary")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffReportSummaryPy {
    #[pyo3(get)]
    pub total_findings: usize,
    #[pyo3(get)]
    pub new_findings: usize,
    #[pyo3(get)]
    pub resolved_findings: usize,
    #[pyo3(get)]
    pub changed_findings: usize,
    #[pyo3(get)]
    pub unchanged_findings: usize,
    #[pyo3(get)]
    pub duration_ms: u64,
    #[pyo3(get)]
    pub output_path: Option<String>,
}

#[pymethods]
impl DiffReportSummaryPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("total_findings", self.total_findings)?;
        dict.set_item("new_findings", self.new_findings)?;
        dict.set_item("resolved_findings", self.resolved_findings)?;
        dict.set_item("changed_findings", self.changed_findings)?;
        dict.set_item("unchanged_findings", self.unchanged_findings)?;
        dict.set_item("duration_ms", self.duration_ms)?;
        dict.set_item("output_path", &self.output_path)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    fn __repr__(&self) -> String {
        format!(
            "DiffReportSummary(total={}, new={}, resolved={}, changed={}, unchanged={})",
            self.total_findings,
            self.new_findings,
            self.resolved_findings,
            self.changed_findings,
            self.unchanged_findings,
        )
    }
}

/// Streaming reporter that compares findings against a baseline.
#[pyclass(name = "StreamingDiffReporter")]
pub struct StreamingDiffReporterPy {
    config: StreamingReportConfigPy,
    baseline: Option<serde_json::Value>,
    diff_results: Vec<FindingDiffResultPy>,
    started: bool,
    start_time: Option<Instant>,
    output_file: Option<File>,
    output_size: u64,
    hasher: Md5,
}

#[pymethods]
impl StreamingDiffReporterPy {
    #[new]
    #[pyo3(signature = (config, *, baseline_json=None))]
    fn new(config: StreamingReportConfigPy, baseline_json: Option<&str>) -> PyResult<Self> {
        let baseline = baseline_json
            .map(|s| {
                serde_json::from_str(s)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
            })
            .transpose()?;

        Ok(Self {
            config,
            baseline,
            diff_results: Vec::new(),
            started: false,
            start_time: None,
            output_file: None,
            output_size: 0,
            hasher: Md5::new(),
        })
    }

    /// Initialize the diff reporter.
    fn start(&mut self) -> PyResult<()> {
        if self.started {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "StreamingDiffReporter already started",
            ));
        }

        self.start_time = Some(Instant::now());

        if let Some(ref path) = self.config.output_path {
            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(path)
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            self.output_file = Some(file);
        }

        self.started = true;
        Ok(())
    }

    /// Append a finding (JSON string), comparing it against the baseline.
    fn write_finding(&mut self, finding_json: &str) -> PyResult<Option<FindingDiffResultPy>> {
        if !self.started {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "StreamingDiffReporter not started; call start() first",
            ));
        }

        self.hasher.update(finding_json.as_bytes());

        let finding: serde_json::Value = serde_json::from_str(finding_json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        let finding_id = finding
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let severity = finding
            .get("severity")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let title = finding
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let diff_result = match &self.baseline {
            Some(base) => {
                let baseline_findings = base.get("findings").and_then(|v| v.as_array());

                match baseline_findings {
                    Some(findings) => {
                        let matched = findings.iter().find(|bf| {
                            bf.get("id")
                                .and_then(|v| v.as_str())
                                .map(|id| id == finding_id)
                                .unwrap_or(false)
                        });

                        match matched {
                            Some(bf) => {
                                let mut changes = Vec::new();

                                let base_severity =
                                    bf.get("severity").and_then(|v| v.as_str()).unwrap_or("");
                                if base_severity != severity {
                                    changes.push(format!(
                                        "severity: {} -> {}",
                                        base_severity, severity
                                    ));
                                }

                                let base_title =
                                    bf.get("title").and_then(|v| v.as_str()).unwrap_or("");
                                if base_title != title {
                                    changes.push(format!("title: {} -> {}", base_title, title));
                                }

                                let base_desc =
                                    bf.get("description").and_then(|v| v.as_str()).unwrap_or("");
                                let cur_desc = finding
                                    .get("description")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                if base_desc != cur_desc {
                                    changes.push("description changed".to_string());
                                }

                                if changes.is_empty() {
                                    FindingDiffResultPy {
                                        finding_id,
                                        diff_status: "unchanged".to_string(),
                                        baseline_finding_id: bf
                                            .get("id")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string()),
                                        changes: Vec::new(),
                                    }
                                } else {
                                    FindingDiffResultPy {
                                        finding_id,
                                        diff_status: "changed".to_string(),
                                        baseline_finding_id: bf
                                            .get("id")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string()),
                                        changes,
                                    }
                                }
                            }
                            None => FindingDiffResultPy {
                                finding_id,
                                diff_status: "new".to_string(),
                                baseline_finding_id: None,
                                changes: Vec::new(),
                            },
                        }
                    }
                    None => FindingDiffResultPy {
                        finding_id,
                        diff_status: "new".to_string(),
                        baseline_finding_id: None,
                        changes: Vec::new(),
                    },
                }
            }
            None => FindingDiffResultPy {
                finding_id,
                diff_status: "new".to_string(),
                baseline_finding_id: None,
                changes: Vec::new(),
            },
        };

        let result = diff_result.clone();
        self.diff_results.push(diff_result);

        // Flush if buffer threshold reached
        if self.diff_results.len() >= self.config.buffer_size {
            self.flush_to_disk()?;
        }

        Ok(Some(result))
    }

    /// Finalize the diff report and return a summary.
    fn finish(&mut self) -> PyResult<DiffReportSummaryPy> {
        if !self.started {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "StreamingDiffReporter not started",
            ));
        }

        self.flush_to_disk()?;

        let total = self.diff_results.len();
        let new_findings = self
            .diff_results
            .iter()
            .filter(|d| d.diff_status == "new")
            .count();
        let resolved_findings = self
            .diff_results
            .iter()
            .filter(|d| d.diff_status == "resolved")
            .count();
        let changed_findings = self
            .diff_results
            .iter()
            .filter(|d| d.diff_status == "changed")
            .count();
        let unchanged_findings = self
            .diff_results
            .iter()
            .filter(|d| d.diff_status == "unchanged")
            .count();

        let duration_ms = self
            .start_time
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);

        self.output_file = None;

        Ok(DiffReportSummaryPy {
            total_findings: total,
            new_findings,
            resolved_findings,
            changed_findings,
            unchanged_findings,
            duration_ms,
            output_path: self.config.output_path.clone(),
        })
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("format", &self.config.format)?;
        dict.set_item("output_path", &self.config.output_path)?;
        dict.set_item("started", self.started)?;
        dict.set_item("diff_count", self.diff_results.len())?;
        dict.set_item("has_baseline", self.baseline.is_some())?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(&self.config).unwrap_or_default()
    }

    fn __repr__(&self) -> String {
        format!(
            "StreamingDiffReporter(format={}, started={}, diffs={})",
            self.config.format,
            self.started,
            self.diff_results.len(),
        )
    }
}

impl StreamingDiffReporterPy {
    fn flush_to_disk(&mut self) -> PyResult<()> {
        if self.diff_results.is_empty() {
            return Ok(());
        }

        if let Some(ref mut file) = self.output_file {
            for result in &self.diff_results {
                let line = serde_json::to_string(result)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
                file.write_all(line.as_bytes())
                    .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
                file.write_all(b"\n")
                    .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            }
            file.flush()
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        }

        self.output_size += self
            .diff_results
            .iter()
            .map(|d| {
                serde_json::to_string(d)
                    .map(|s| s.len() as u64 + 1)
                    .unwrap_or(1)
            })
            .sum::<u64>();
        self.diff_results.clear();
        Ok(())
    }
}

/// Manifest describing a completed streaming report.
#[pyclass(frozen, name = "ReportManifest")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportManifestPy {
    #[pyo3(get)]
    pub report_id: String,
    #[pyo3(get)]
    pub format: String,
    #[pyo3(get)]
    pub schema_version: String,
    #[pyo3(get)]
    pub tool_version: String,
    #[pyo3(get)]
    pub created_at_ms: u64,
    #[pyo3(get)]
    pub finding_count: usize,
    #[pyo3(get)]
    pub artifact_ids: Vec<String>,
    #[pyo3(get)]
    pub content_hash: String,
}

#[pymethods]
impl ReportManifestPy {
    #[new]
    #[pyo3(signature = (report_id, format, created_at_ms, finding_count, content_hash, *, schema_version=None, tool_version=None, artifact_ids=None))]
    fn new(
        report_id: String,
        format: String,
        created_at_ms: u64,
        finding_count: usize,
        content_hash: String,
        schema_version: Option<String>,
        tool_version: Option<String>,
        artifact_ids: Option<Vec<String>>,
    ) -> Self {
        Self {
            report_id,
            format,
            schema_version: schema_version.unwrap_or_else(|| "1.0.0".to_string()),
            tool_version: tool_version.unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string()),
            created_at_ms,
            finding_count,
            artifact_ids: artifact_ids.unwrap_or_default(),
            content_hash,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("report_id", &self.report_id)?;
        dict.set_item("format", &self.format)?;
        dict.set_item("schema_version", &self.schema_version)?;
        dict.set_item("tool_version", &self.tool_version)?;
        dict.set_item("created_at_ms", self.created_at_ms)?;
        dict.set_item("finding_count", self.finding_count)?;
        dict.set_item("artifact_ids", &self.artifact_ids)?;
        dict.set_item("content_hash", &self.content_hash)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    fn __repr__(&self) -> String {
        let hash_display = if self.content_hash.len() >= 8 {
            &self.content_hash[..8]
        } else {
            &self.content_hash
        };
        format!(
            "ReportManifest(id={}, format={}, findings={}, hash={})",
            self.report_id, self.format, self.finding_count, hash_display,
        )
    }
}

/// Compute an MD5 content hash from a string. Used for manifest generation.
pub fn compute_content_hash(values: &[String]) -> String {
    let mut hasher = Md5::new();
    for v in values {
        hasher.update(v.as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}
