use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use pyo3::prelude::*;
use pyo3::types::PyDict;

const JSONL_REPOSITORY_SCHEMA_VERSION: u32 = 1;

fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// JsonlFindingRepository
// ---------------------------------------------------------------------------

#[pyclass(name = "JsonlFindingRepository")]
pub struct JsonlFindingRepository {
    findings: Arc<Mutex<HashMap<String, String>>>,
    dedup_index: Arc<Mutex<HashMap<String, String>>>,
    next_id: Arc<Mutex<u64>>,
    path: String,
    initialized: bool,
}

impl Clone for JsonlFindingRepository {
    fn clone(&self) -> Self {
        Self {
            findings: Arc::clone(&self.findings),
            dedup_index: Arc::clone(&self.dedup_index),
            next_id: Arc::clone(&self.next_id),
            path: self.path.clone(),
            initialized: self.initialized,
        }
    }
}

#[pymethods]
impl JsonlFindingRepository {
    #[new]
    fn new(path: String) -> Self {
        Self {
            findings: Arc::new(Mutex::new(HashMap::new())),
            dedup_index: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
            path,
            initialized: false,
        }
    }

    fn initialize(&mut self) -> PyResult<()> {
        let p = Path::new(&self.path);
        if p.exists() {
            let file =
                File::open(p).map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            let reader = BufReader::new(file);
            let mut findings = self
                .findings
                .lock()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            let mut dedup = self
                .dedup_index
                .lock()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            let mut max_id: u64 = 0;
            for line_result in reader.lines() {
                let line =
                    line_result.map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    if let Some(id) = val.get("id").and_then(|v| v.as_str()) {
                        findings.insert(id.to_string(), trimmed.to_string());
                        if let Some(dk) = val.get("dedup_key").and_then(|v| v.as_str()) {
                            dedup.insert(dk.to_string(), id.to_string());
                        }
                        if id.starts_with("find-") {
                            if let Ok(n) = id[5..].parse::<u64>() {
                                if n >= max_id {
                                    max_id = n + 1;
                                }
                            }
                        }
                    }
                }
            }
            let mut next = self
                .next_id
                .lock()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            if max_id > *next {
                *next = max_id;
            }
        } else {
            File::create(p).map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        }
        self.initialized = true;
        Ok(())
    }

    fn insert_finding(&self, finding_json: &str) -> PyResult<String> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }

        let parsed: serde_json::Value = serde_json::from_str(finding_json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        let id = if let Some(id_val) = parsed.get("id").and_then(|v| v.as_str()) {
            id_val.to_string()
        } else {
            let mut next = self
                .next_id
                .lock()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            let id = format!("find-{}", *next);
            *next += 1;
            id
        };

        if let Some(dedup_key) = parsed.get("dedup_key").and_then(|v| v.as_str()) {
            let dedup = self
                .dedup_index
                .lock()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            if dedup.contains_key(dedup_key) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Duplicate finding with dedup_key '{}'",
                    dedup_key
                )));
            }
        }

        let mut findings = self
            .findings
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        findings.insert(id.clone(), finding_json.to_string());

        if let Some(dedup_key) = parsed.get("dedup_key").and_then(|v| v.as_str()) {
            let mut dedup = self
                .dedup_index
                .lock()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            dedup.insert(dedup_key.to_string(), id.clone());
        }

        Ok(id)
    }

    fn get_finding(&self, finding_id: &str) -> PyResult<Option<String>> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }
        let findings = self
            .findings
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(findings.get(finding_id).cloned())
    }

    fn update_finding(&self, finding_id: &str, finding_json: &str) -> PyResult<bool> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }

        let _parsed: serde_json::Value = serde_json::from_str(finding_json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        let mut findings = self
            .findings
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        match findings.get_mut(finding_id) {
            Some(existing) => {
                *existing = finding_json.to_string();
                Ok(true)
            }
            None => Ok(false),
        }
    }

    fn delete_finding(&self, finding_id: &str) -> PyResult<bool> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }
        let mut findings = self
            .findings
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(findings.remove(finding_id).is_some())
    }

    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (assessment_id=None, severity=None, state=None, finding_type=None, limit=100, offset=0))]
    fn query_findings(
        &self,
        assessment_id: Option<&str>,
        severity: Option<&str>,
        state: Option<&str>,
        finding_type: Option<&str>,
        limit: u64,
        offset: u64,
    ) -> PyResult<Vec<String>> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }

        let findings = self
            .findings
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let mut results: Vec<String> = findings
            .values()
            .filter(|json| {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(json) {
                    if let Some(s) = severity {
                        if val.get("severity").and_then(|v| v.as_str()) != Some(s) {
                            return false;
                        }
                    }
                    if let Some(s) = state {
                        if val.get("state").and_then(|v| v.as_str()) != Some(s) {
                            return false;
                        }
                    }
                    if let Some(ft) = finding_type {
                        if val.get("finding_type").and_then(|v| v.as_str()) != Some(ft) {
                            return false;
                        }
                    }
                    if let Some(aid) = assessment_id {
                        if val.get("assessment_id").and_then(|v| v.as_str()) != Some(aid) {
                            return false;
                        }
                    }
                    true
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        let start = offset as usize;
        let end = start + limit as usize;
        if start >= results.len() {
            return Ok(Vec::new());
        }
        results.drain(..start);
        results.truncate((end - start) as usize);

        Ok(results)
    }

    #[pyo3(signature = (assessment_id=None, severity=None))]
    fn count_findings(&self, assessment_id: Option<&str>, severity: Option<&str>) -> PyResult<u64> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }

        let findings = self
            .findings
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let count = findings
            .values()
            .filter(|json| {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(json) {
                    if let Some(s) = severity {
                        if val.get("severity").and_then(|v| v.as_str()) != Some(s) {
                            return false;
                        }
                    }
                    if let Some(aid) = assessment_id {
                        if val.get("assessment_id").and_then(|v| v.as_str()) != Some(aid) {
                            return false;
                        }
                    }
                    true
                } else {
                    false
                }
            })
            .count() as u64;

        Ok(count)
    }

    fn deduplicate(&self, dedup_key: &str) -> PyResult<Option<String>> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }
        let dedup = self
            .dedup_index
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(dedup.get(dedup_key).cloned())
    }

    fn flush(&self) -> PyResult<()> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }
        let findings = self
            .findings
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let dir = Path::new(&self.path)
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let tmp_path = dir.join(format!(".jsonl_finding_tmp_{}", current_epoch_ms()));
        let final_path = Path::new(&self.path);

        {
            let mut tmp_file = File::create(&tmp_path)
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            for json in findings.values() {
                writeln!(tmp_file, "{}", json)
                    .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            }
            tmp_file
                .sync_all()
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        }

        std::fs::rename(&tmp_path, final_path)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(())
    }

    fn close(&mut self) {
        self.initialized = false;
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
    ) -> bool {
        false
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let findings = self
            .findings
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let dict = PyDict::new_bound(py);
        let list = pyo3::types::PyList::empty_bound(py);
        for (id, json) in findings.iter() {
            let item_dict = PyDict::new_bound(py);
            item_dict.set_item("id", id)?;
            item_dict.set_item("data", json)?;
            list.append(item_dict)?;
        }
        dict.set_item("findings", list)?;
        dict.set_item("count", findings.len())?;
        dict.set_item("path", &self.path)?;
        dict.set_item("initialized", self.initialized)?;
        dict.set_item("schema_version", JSONL_REPOSITORY_SCHEMA_VERSION)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        let findings = self
            .findings
            .lock()
            .map(|f| f.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        serde_json::to_string_pretty(&findings).unwrap_or_else(|_| "[]".to_string())
    }

    fn __repr__(&self) -> String {
        let count = self.findings.lock().map(|f| f.len()).unwrap_or(0);
        format!(
            "JsonlFindingRepository(path={}, count={}, initialized={})",
            self.path, count, self.initialized
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

// ---------------------------------------------------------------------------
// JsonlAssessmentRepository
// ---------------------------------------------------------------------------

#[pyclass(name = "JsonlAssessmentRepository")]
pub struct JsonlAssessmentRepository {
    assessments: Arc<Mutex<HashMap<String, String>>>,
    assessment_findings: Arc<Mutex<HashMap<String, Vec<String>>>>,
    next_id: Arc<Mutex<u64>>,
    path: String,
    initialized: bool,
}

impl Clone for JsonlAssessmentRepository {
    fn clone(&self) -> Self {
        Self {
            assessments: Arc::clone(&self.assessments),
            assessment_findings: Arc::clone(&self.assessment_findings),
            next_id: Arc::clone(&self.next_id),
            path: self.path.clone(),
            initialized: self.initialized,
        }
    }
}

#[pymethods]
impl JsonlAssessmentRepository {
    #[new]
    fn new(path: String) -> Self {
        Self {
            assessments: Arc::new(Mutex::new(HashMap::new())),
            assessment_findings: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
            path,
            initialized: false,
        }
    }

    fn initialize(&mut self) -> PyResult<()> {
        let p = Path::new(&self.path);
        if p.exists() {
            let file =
                File::open(p).map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            let reader = BufReader::new(file);
            let mut assessments = self
                .assessments
                .lock()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            let mut links = self
                .assessment_findings
                .lock()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            let mut max_id: u64 = 0;
            for line_result in reader.lines() {
                let line =
                    line_result.map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    if let Some(id) = val.get("id").and_then(|v| v.as_str()) {
                        if let Some(fids) = val.get("finding_ids").and_then(|v| v.as_array()) {
                            links.insert(
                                id.to_string(),
                                fids.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect(),
                            );
                        }
                        assessments.insert(id.to_string(), trimmed.to_string());
                        if id.starts_with("assess-") {
                            if let Ok(n) = id[7..].parse::<u64>() {
                                if n >= max_id {
                                    max_id = n + 1;
                                }
                            }
                        }
                    }
                }
            }
            let mut next = self
                .next_id
                .lock()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            if max_id > *next {
                *next = max_id;
            }
        } else {
            File::create(p).map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        }
        self.initialized = true;
        Ok(())
    }

    fn create_assessment(
        &self,
        name: &str,
        target: &str,
        assessment_type: &str,
    ) -> PyResult<String> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }

        let mut next = self
            .next_id
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let id = format!("assess-{}", *next);
        *next += 1;

        let assessment = serde_json::json!({
            "id": id,
            "name": name,
            "target": target,
            "assessment_type": assessment_type,
            "state": "created",
            "created_at_ms": current_epoch_ms(),
            "finding_ids": [],
            "artifacts": [],
        });

        let mut assessments = self
            .assessments
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        assessments.insert(id.clone(), assessment.to_string());

        Ok(id)
    }

    fn get_assessment(&self, assessment_id: &str) -> PyResult<Option<String>> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }
        let assessments = self
            .assessments
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(assessments.get(assessment_id).cloned())
    }

    fn update_assessment_state(&self, assessment_id: &str, state: &str) -> PyResult<bool> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }

        let mut assessments = self
            .assessments
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        match assessments.get_mut(assessment_id) {
            Some(json_str) => {
                let mut val: serde_json::Value = serde_json::from_str(json_str)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
                val["state"] = serde_json::Value::String(state.to_string());
                *json_str = val.to_string();
                Ok(true)
            }
            None => Ok(false),
        }
    }

    fn attach_finding(&self, assessment_id: &str, finding_id: &str) -> PyResult<bool> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }

        {
            let assessments = self
                .assessments
                .lock()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            if !assessments.contains_key(assessment_id) {
                return Ok(false);
            }
        }

        let mut links = self
            .assessment_findings
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let finding_ids = links
            .entry(assessment_id.to_string())
            .or_insert_with(Vec::new);
        if !finding_ids.contains(&finding_id.to_string()) {
            finding_ids.push(finding_id.to_string());
        }

        let mut assessments = self
            .assessments
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        if let Some(json_str) = assessments.get_mut(assessment_id) {
            let mut val: serde_json::Value = serde_json::from_str(json_str)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            if let Some(arr) = val.get_mut("finding_ids").and_then(|v| v.as_array_mut()) {
                let fid = finding_id.to_string();
                if !arr.iter().any(|v| v.as_str() == Some(&fid)) {
                    arr.push(serde_json::Value::String(fid));
                }
            }
            *json_str = val.to_string();
        }

        Ok(true)
    }

    fn attach_artifact(&self, assessment_id: &str, artifact_json: &str) -> PyResult<bool> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }

        let _parsed: serde_json::Value = serde_json::from_str(artifact_json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        let mut assessments = self
            .assessments
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        match assessments.get_mut(assessment_id) {
            Some(json_str) => {
                let mut val: serde_json::Value = serde_json::from_str(json_str)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
                let artifact_val: serde_json::Value = serde_json::from_str(artifact_json)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
                if let Some(arr) = val.get_mut("artifacts").and_then(|v| v.as_array_mut()) {
                    arr.push(artifact_val);
                }
                *json_str = val.to_string();
                Ok(true)
            }
            None => Ok(false),
        }
    }

    #[pyo3(signature = (limit=100, offset=0))]
    fn list_assessments(&self, limit: u64, offset: u64) -> PyResult<Vec<String>> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }

        let assessments = self
            .assessments
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let mut results: Vec<String> = assessments.values().cloned().collect();

        let start = offset as usize;
        let end = start + limit as usize;
        if start >= results.len() {
            return Ok(Vec::new());
        }
        results.drain(..start);
        results.truncate((end - start) as usize);

        Ok(results)
    }

    fn delete_assessment(&self, assessment_id: &str) -> PyResult<bool> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }

        let mut assessments = self
            .assessments
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let removed = assessments.remove(assessment_id).is_some();

        if removed {
            let mut links = self
                .assessment_findings
                .lock()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            links.remove(assessment_id);
        }

        Ok(removed)
    }

    fn flush(&self) -> PyResult<()> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }
        let assessments = self
            .assessments
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let dir = Path::new(&self.path)
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let tmp_path = dir.join(format!(".jsonl_assessment_tmp_{}", current_epoch_ms()));
        let final_path = Path::new(&self.path);

        {
            let mut tmp_file = File::create(&tmp_path)
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            for json in assessments.values() {
                writeln!(tmp_file, "{}", json)
                    .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            }
            tmp_file
                .sync_all()
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        }

        std::fs::rename(&tmp_path, final_path)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(())
    }

    fn close(&mut self) {
        self.initialized = false;
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
    ) -> bool {
        false
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let assessments = self
            .assessments
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let dict = PyDict::new_bound(py);
        let list = pyo3::types::PyList::empty_bound(py);
        for (id, json) in assessments.iter() {
            let item_dict = PyDict::new_bound(py);
            item_dict.set_item("id", id)?;
            item_dict.set_item("data", json)?;
            list.append(item_dict)?;
        }
        dict.set_item("assessments", list)?;
        dict.set_item("count", assessments.len())?;
        dict.set_item("path", &self.path)?;
        dict.set_item("initialized", self.initialized)?;
        dict.set_item("schema_version", JSONL_REPOSITORY_SCHEMA_VERSION)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        let assessments = self
            .assessments
            .lock()
            .map(|a| a.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        serde_json::to_string_pretty(&assessments).unwrap_or_else(|_| "[]".to_string())
    }

    fn __repr__(&self) -> String {
        let count = self.assessments.lock().map(|a| a.len()).unwrap_or(0);
        format!(
            "JsonlAssessmentRepository(path={}, count={}, initialized={})",
            self.path, count, self.initialized
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_path(name: &str) -> String {
        let dir = std::env::temp_dir().join("eggsec_jsonl_test");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(format!("{}.jsonl", name));
        let _ = fs::remove_file(&path);
        path.to_string_lossy().to_string()
    }

    #[test]
    fn test_jsonl_finding_insert_and_get() {
        let path = tmp_path("insert_get");
        let mut repo = JsonlFindingRepository::new(path.clone());
        repo.initialize().unwrap();

        let finding = r#"{"id":"f1","title":"Test finding","severity":"high"}"#;
        let id = repo.insert_finding(finding).unwrap();
        assert_eq!(id, "f1");

        let got = repo.get_finding("f1").unwrap().unwrap();
        assert!(got.contains("Test finding"));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_finding_generated_id() {
        let path = tmp_path("gen_id");
        let mut repo = JsonlFindingRepository::new(path.clone());
        repo.initialize().unwrap();

        let id = repo
            .insert_finding(r#"{"title":"No ID","severity":"low"}"#)
            .unwrap();
        assert_eq!(id, "find-1");

        let id2 = repo
            .insert_finding(r#"{"title":"Second","severity":"medium"}"#)
            .unwrap();
        assert_eq!(id2, "find-2");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_finding_query_filters() {
        let path = tmp_path("query_filters");
        let mut repo = JsonlFindingRepository::new(path.clone());
        repo.initialize().unwrap();

        repo.insert_finding(r#"{"id":"f1","severity":"high","state":"open"}"#)
            .unwrap();
        repo.insert_finding(r#"{"id":"f2","severity":"low","state":"closed"}"#)
            .unwrap();
        repo.insert_finding(
            r#"{"id":"f3","severity":"high","state":"open","finding_type":"vuln"}"#,
        )
        .unwrap();

        let high = repo
            .query_findings(None, Some("high"), None, None, 100, 0)
            .unwrap();
        assert_eq!(high.len(), 2);

        let low = repo
            .query_findings(None, Some("low"), None, None, 100, 0)
            .unwrap();
        assert_eq!(low.len(), 1);

        let open = repo
            .query_findings(None, None, Some("open"), None, 100, 0)
            .unwrap();
        assert_eq!(open.len(), 2);

        let vuln = repo
            .query_findings(None, None, None, Some("vuln"), 100, 0)
            .unwrap();
        assert_eq!(vuln.len(), 1);

        let count = repo.count_findings(None, Some("high")).unwrap();
        assert_eq!(count, 2);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_finding_pagination() {
        let path = tmp_path("pagination");
        let mut repo = JsonlFindingRepository::new(path.clone());
        repo.initialize().unwrap();

        for i in 0..10 {
            repo.insert_finding(&format!(r#"{{"id":"f{}","severity":"info"}}"#, i))
                .unwrap();
        }

        let page1 = repo.query_findings(None, None, None, None, 3, 0).unwrap();
        assert_eq!(page1.len(), 3);

        let page2 = repo.query_findings(None, None, None, None, 3, 3).unwrap();
        assert_eq!(page2.len(), 3);

        let page_end = repo.query_findings(None, None, None, None, 3, 9).unwrap();
        assert_eq!(page_end.len(), 1);

        let page_past = repo.query_findings(None, None, None, None, 3, 10).unwrap();
        assert!(page_past.is_empty());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_finding_update_and_delete() {
        let path = tmp_path("update_delete");
        let mut repo = JsonlFindingRepository::new(path.clone());
        repo.initialize().unwrap();

        repo.insert_finding(r#"{"id":"f1","title":"Original"}"#)
            .unwrap();

        let updated = repo
            .update_finding("f1", r#"{"id":"f1","title":"Updated"}"#)
            .unwrap();
        assert!(updated);

        let got = repo.get_finding("f1").unwrap().unwrap();
        assert!(got.contains("Updated"));

        assert!(repo.delete_finding("f1").unwrap());
        assert!(!repo.delete_finding("f1").unwrap());
        assert!(repo.get_finding("f1").unwrap().is_none());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_finding_deduplication() {
        let path = tmp_path("dedup");
        let mut repo = JsonlFindingRepository::new(path.clone());
        repo.initialize().unwrap();

        repo.insert_finding(r#"{"id":"f1","dedup_key":"dk1"}"#)
            .unwrap();

        let dup = repo.deduplicate("dk1").unwrap();
        assert_eq!(dup, Some("f1".to_string()));

        let no_dup = repo.deduplicate("dk2").unwrap();
        assert!(no_dup.is_none());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_finding_not_initialized() {
        let path = tmp_path("not_init");
        let repo = JsonlFindingRepository::new(path.clone());
        let result = repo.insert_finding(r#"{"id":"f1"}"#);
        assert!(result.is_err());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_finding_flush_and_reload() {
        let path = tmp_path("flush_reload");
        {
            let mut repo = JsonlFindingRepository::new(path.clone());
            repo.initialize().unwrap();
            repo.insert_finding(r#"{"id":"f1","title":"Persisted"}"#)
                .unwrap();
            repo.flush().unwrap();
        }

        let mut repo2 = JsonlFindingRepository::new(path.clone());
        repo2.initialize().unwrap();
        assert_eq!(repo2.findings.lock().unwrap().len(), 1);
        let got = repo2.get_finding("f1").unwrap().unwrap();
        assert!(got.contains("Persisted"));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_finding_delete_flush() {
        let path = tmp_path("delete_flush");
        {
            let mut repo = JsonlFindingRepository::new(path.clone());
            repo.initialize().unwrap();
            repo.insert_finding(r#"{"id":"f1"}"#).unwrap();
            repo.insert_finding(r#"{"id":"f2"}"#).unwrap();
            repo.delete_finding("f1").unwrap();
            repo.flush().unwrap();
        }

        let mut repo2 = JsonlFindingRepository::new(path.clone());
        repo2.initialize().unwrap();
        assert_eq!(repo2.findings.lock().unwrap().len(), 1);
        assert!(repo2.get_finding("f1").unwrap().is_none());
        assert!(repo2.get_finding("f2").unwrap().is_some());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_assessment_create_and_get() {
        let path = tmp_path("assess_create");
        let mut repo = JsonlAssessmentRepository::new(path.clone());
        repo.initialize().unwrap();

        let id = repo
            .create_assessment("Test", "10.0.0.1", "full-scan")
            .unwrap();
        assert_eq!(id, "assess-1");

        let got = repo.get_assessment(&id).unwrap().unwrap();
        assert!(got.contains("Test"));
        assert!(got.contains("10.0.0.1"));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_assessment_state_update() {
        let path = tmp_path("assess_state");
        let mut repo = JsonlAssessmentRepository::new(path.clone());
        repo.initialize().unwrap();

        let id = repo
            .create_assessment("Test", "10.0.0.1", "port-scan")
            .unwrap();
        assert!(repo.update_assessment_state(&id, "running").unwrap());

        let got = repo.get_assessment(&id).unwrap().unwrap();
        assert!(got.contains("running"));

        assert!(!repo
            .update_assessment_state("nonexistent", "running")
            .unwrap());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_assessment_attach_finding() {
        let path = tmp_path("assess_attach");
        let mut repo = JsonlAssessmentRepository::new(path.clone());
        repo.initialize().unwrap();

        let id = repo
            .create_assessment("Test", "10.0.0.1", "full-scan")
            .unwrap();
        assert!(repo.attach_finding(&id, "find-1").unwrap());
        assert!(repo.attach_finding(&id, "find-2").unwrap());
        assert!(repo.attach_finding(&id, "find-1").unwrap());

        assert!(!repo.attach_finding("nonexistent", "find-1").unwrap());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_assessment_attach_artifact() {
        let path = tmp_path("assess_artifact");
        let mut repo = JsonlAssessmentRepository::new(path.clone());
        repo.initialize().unwrap();

        let id = repo
            .create_assessment("Test", "10.0.0.1", "full-scan")
            .unwrap();
        let artifact = r#"{"type":"pcap","path":"/tmp/capture.pcap"}"#;
        assert!(repo.attach_artifact(&id, artifact).unwrap());

        let got = repo.get_assessment(&id).unwrap().unwrap();
        assert!(got.contains("pcap"));

        assert!(!repo.attach_artifact("nonexistent", artifact).unwrap());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_assessment_list_and_delete() {
        let path = tmp_path("assess_list");
        let mut repo = JsonlAssessmentRepository::new(path.clone());
        repo.initialize().unwrap();

        for i in 0..5 {
            repo.create_assessment(&format!("Assessment {}", i), "10.0.0.1", "port-scan")
                .unwrap();
        }

        let all = repo.list_assessments(100, 0).unwrap();
        assert_eq!(all.len(), 5);

        let page = repo.list_assessments(2, 0).unwrap();
        assert_eq!(page.len(), 2);

        assert!(repo.delete_assessment("assess-1").unwrap());
        assert!(!repo.delete_assessment("assess-1").unwrap());

        let after_delete = repo.list_assessments(100, 0).unwrap();
        assert_eq!(after_delete.len(), 4);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_assessment_not_initialized() {
        let path = tmp_path("assess_not_init");
        let repo = JsonlAssessmentRepository::new(path.clone());
        let result = repo.create_assessment("Test", "10.0.0.1", "scan");
        assert!(result.is_err());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_assessment_flush_and_reload() {
        let path = tmp_path("assess_flush");
        {
            let mut repo = JsonlAssessmentRepository::new(path.clone());
            repo.initialize().unwrap();
            repo.create_assessment("Persistent", "10.0.0.1", "full-scan")
                .unwrap();
            repo.flush().unwrap();
        }

        let mut repo2 = JsonlAssessmentRepository::new(path.clone());
        repo2.initialize().unwrap();
        assert_eq!(repo2.assessments.lock().unwrap().len(), 1);
        let got = repo2.get_assessment("assess-1").unwrap().unwrap();
        assert!(got.contains("Persistent"));

        let _ = fs::remove_file(&path);
    }
}
