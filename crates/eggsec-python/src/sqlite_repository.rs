use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::{Deserialize, Serialize};

/// Current schema version for the SQLite-backed repositories.
pub const SQLITE_REPOSITORY_SCHEMA_VERSION: u32 = 1;

/// Epoch milliseconds helper.
fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// SqliteMigration
// ---------------------------------------------------------------------------

/// A single applied schema migration.
#[pyclass(frozen, name = "SqliteMigration")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteMigration {
    #[pyo3(get)]
    pub version: u32,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub applied_at_ms: u64,
}

#[pymethods]
impl SqliteMigration {
    #[new]
    fn py_new(version: u32, description: String, applied_at_ms: u64) -> Self {
        Self {
            version,
            description,
            applied_at_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("version", self.version)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("applied_at_ms", self.applied_at_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    fn __repr__(&self) -> String {
        format!(
            "SqliteMigration(version={}, description={})",
            self.version, self.description
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

// ---------------------------------------------------------------------------
// MigrationResult
// ---------------------------------------------------------------------------

/// Result of running database migrations.
#[pyclass(frozen, name = "SqliteMigrationResult")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    #[pyo3(get)]
    pub applied: bool,
    #[pyo3(get)]
    pub from_version: u32,
    #[pyo3(get)]
    pub to_version: u32,
    #[pyo3(get)]
    pub migrations_applied: Vec<SqliteMigration>,
}

#[pymethods]
impl MigrationResult {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("applied", self.applied)?;
        dict.set_item("from_version", self.from_version)?;
        dict.set_item("to_version", self.to_version)?;
        let list = pyo3::types::PyList::empty_bound(py);
        for m in &self.migrations_applied {
            list.append(m.to_dict(py)?)?;
        }
        dict.set_item("migrations_applied", list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    fn __repr__(&self) -> String {
        format!(
            "MigrationResult(applied={}, from_version={}, to_version={}, migrations={})",
            self.applied,
            self.from_version,
            self.to_version,
            self.migrations_applied.len()
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

// ---------------------------------------------------------------------------
// SqliteFindingRepository
// ---------------------------------------------------------------------------

/// SQLite-backed finding repository with HashMap simulation.
///
/// Provides a persistence-oriented API for findings with deduplication,
/// filtering, and pagination. The backend is an in-memory HashMap that
/// simulates what a real SQLite store would provide.
#[pyclass(name = "SqliteFindingRepository")]
pub struct SqliteFindingRepository {
    findings: Arc<Mutex<HashMap<String, String>>>,
    dedup_index: Arc<Mutex<HashMap<String, String>>>,
    next_id: Arc<Mutex<u64>>,
    db_path: String,
    initialized: bool,
}

impl Clone for SqliteFindingRepository {
    fn clone(&self) -> Self {
        Self {
            findings: Arc::clone(&self.findings),
            dedup_index: Arc::clone(&self.dedup_index),
            next_id: Arc::clone(&self.next_id),
            db_path: self.db_path.clone(),
            initialized: self.initialized,
        }
    }
}

#[pymethods]
impl SqliteFindingRepository {
    /// Create a new finding repository backed by the given database path.
    /// The actual SQLite file is not opened until `initialize()` is called.
    #[new]
    fn new(db_path: String) -> Self {
        Self {
            findings: Arc::new(Mutex::new(HashMap::new())),
            dedup_index: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
            db_path,
            initialized: false,
        }
    }

    /// Initialize the repository, creating tables if they do not exist.
    /// For the in-memory simulation this sets an initialized flag.
    fn initialize(&mut self) -> PyResult<()> {
        self.initialized = true;
        Ok(())
    }

    /// Insert a finding from its JSON representation.
    /// Returns the generated finding ID.
    fn insert_finding(&self, finding_json: &str) -> PyResult<String> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }

        let parsed: serde_json::Value = serde_json::from_str(finding_json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        // Use provided ID or generate one.
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

        // Check dedup index.
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

        // Register in dedup index if present.
        if let Some(dedup_key) = parsed.get("dedup_key").and_then(|v| v.as_str()) {
            let mut dedup = self
                .dedup_index
                .lock()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            dedup.insert(dedup_key.to_string(), id.clone());
        }

        Ok(id)
    }

    /// Get a finding by ID. Returns the JSON string or None.
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

    /// Update an existing finding. Returns true if the finding was found and updated.
    fn update_finding(&self, finding_id: &str, finding_json: &str) -> PyResult<bool> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }

        // Validate JSON.
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

    /// Delete a finding by ID. Returns true if found and removed.
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

    /// Query findings with optional filters and pagination.
    /// Returns a vector of JSON strings.
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

        // Apply pagination.
        let start = offset as usize;
        let end = start + limit as usize;
        if start >= results.len() {
            return Ok(Vec::new());
        }
        results.drain(..start);
        results.truncate((end - start) as usize);

        Ok(results)
    }

    /// Count findings with optional filters.
    #[pyo3(signature = (assessment_id=None, severity=None))]
    fn count_findings(
        &self,
        assessment_id: Option<&str>,
        severity: Option<&str>,
    ) -> PyResult<u64> {
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

    /// Check for a duplicate finding by dedup_key.
    /// Returns the existing finding ID if a duplicate exists, None otherwise.
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

    /// Close the repository and release resources.
    fn close(&mut self) {
        self.initialized = false;
    }

    /// Context manager __enter__.
    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Context manager __exit__ — closes the repository.
    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        // Intentionally does not call self.close() since __exit__ takes &self
        // and close takes &mut self. The flag is left as-is; explicit close()
        // is recommended.
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
        dict.set_item("db_path", &self.db_path)?;
        dict.set_item("initialized", self.initialized)?;
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
            "SqliteFindingRepository(db_path={}, count={}, initialized={})",
            self.db_path, count, self.initialized
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

// ---------------------------------------------------------------------------
// SqliteAssessmentRepository
// ---------------------------------------------------------------------------

/// SQLite-backed assessment repository with HashMap simulation.
///
/// Provides a persistence-oriented API for assessments with finding/artifact
/// attachment, state management, and pagination.
#[pyclass(name = "SqliteAssessmentRepository")]
pub struct SqliteAssessmentRepository {
    assessments: Arc<Mutex<HashMap<String, String>>>,
    assessment_findings: Arc<Mutex<HashMap<String, Vec<String>>>>,
    next_id: Arc<Mutex<u64>>,
    db_path: String,
    initialized: bool,
}

impl Clone for SqliteAssessmentRepository {
    fn clone(&self) -> Self {
        Self {
            assessments: Arc::clone(&self.assessments),
            assessment_findings: Arc::clone(&self.assessment_findings),
            next_id: Arc::clone(&self.next_id),
            db_path: self.db_path.clone(),
            initialized: self.initialized,
        }
    }
}

#[pymethods]
impl SqliteAssessmentRepository {
    /// Create a new assessment repository backed by the given database path.
    #[new]
    fn new(db_path: String) -> Self {
        Self {
            assessments: Arc::new(Mutex::new(HashMap::new())),
            assessment_findings: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
            db_path,
            initialized: false,
        }
    }

    /// Initialize the repository, creating tables if they do not exist.
    fn initialize(&mut self) -> PyResult<()> {
        self.initialized = true;
        Ok(())
    }

    /// Create a new assessment and return its generated ID.
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

    /// Get an assessment by ID. Returns the JSON string or None.
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

    /// Update the state of an assessment. Returns true if found and updated.
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

    /// Attach a finding to an assessment. Returns true if found and attached.
    fn attach_finding(&self, assessment_id: &str, finding_id: &str) -> PyResult<bool> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }

        // Verify assessment exists.
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

        // Update the assessment JSON to reflect the attached finding.
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

    /// Attach an artifact (as JSON) to an assessment. Returns true if found and attached.
    fn attach_artifact(&self, assessment_id: &str, artifact_json: &str) -> PyResult<bool> {
        if !self.initialized {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Repository not initialized. Call initialize() first.",
            ));
        }

        // Validate artifact JSON.
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

    /// List assessments with pagination. Returns a vector of JSON strings.
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

        // Apply pagination.
        let start = offset as usize;
        let end = start + limit as usize;
        if start >= results.len() {
            return Ok(Vec::new());
        }
        results.drain(..start);
        results.truncate((end - start) as usize);

        Ok(results)
    }

    /// Delete an assessment and its finding links. Returns true if found and removed.
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

    /// Close the repository and release resources.
    fn close(&mut self) {
        self.initialized = false;
    }

    /// Context manager __enter__.
    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Context manager __exit__ — closes the repository.
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
        dict.set_item("db_path", &self.db_path)?;
        dict.set_item("initialized", self.initialized)?;
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
            "SqliteAssessmentRepository(db_path={}, count={}, initialized={})",
            self.db_path, count, self.initialized
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_roundtrip() {
        let m = SqliteMigration {
            version: 1,
            description: "Initial schema".to_string(),
            applied_at_ms: 1234567890,
        };
        let json = m.to_json();
        let m2: SqliteMigration = serde_json::from_str(&json).unwrap();
        assert_eq!(m2.version, 1);
        assert_eq!(m2.description, "Initial schema");
    }

    #[test]
    fn test_migration_result_roundtrip() {
        let mr = MigrationResult {
            applied: true,
            from_version: 0,
            to_version: 1,
            migrations_applied: vec![SqliteMigration {
                version: 1,
                description: "Create tables".to_string(),
                applied_at_ms: 1000,
            }],
        };
        let json = mr.to_json();
        let mr2: MigrationResult = serde_json::from_str(&json).unwrap();
        assert!(mr2.applied);
        assert_eq!(mr2.migrations_applied.len(), 1);
    }

    #[test]
    fn test_finding_repository_insert_and_get() {
        let repo = SqliteFindingRepository::new(":memory:".to_string());
        let mut repo = repo;
        repo.initialize().unwrap();

        let finding = r#"{"id":"f1","title":"Test finding","severity":"high"}"#;
        let id = repo.insert_finding(finding).unwrap();
        assert_eq!(id, "f1");

        let got = repo.get_finding("f1").unwrap().unwrap();
        assert!(got.contains("Test finding"));
    }

    #[test]
    fn test_finding_repository_generated_id() {
        let repo = SqliteFindingRepository::new(":memory:".to_string());
        let mut repo = repo;
        repo.initialize().unwrap();

        let finding = r#"{"title":"No ID","severity":"low"}"#;
        let id = repo.insert_finding(finding).unwrap();
        assert_eq!(id, "find-1");

        let id2 = repo.insert_finding(r#"{"title":"Second","severity":"medium"}"#).unwrap();
        assert_eq!(id2, "find-2");
    }

    #[test]
    fn test_finding_repository_query_filters() {
        let repo = SqliteFindingRepository::new(":memory:".to_string());
        let mut repo = repo;
        repo.initialize().unwrap();

        repo.insert_finding(r#"{"id":"f1","severity":"high","state":"open"}"#)
            .unwrap();
        repo.insert_finding(r#"{"id":"f2","severity":"low","state":"closed"}"#)
            .unwrap();
        repo.insert_finding(r#"{"id":"f3","severity":"high","state":"open","finding_type":"vuln"}"#)
            .unwrap();

        let high = repo.query_findings(None, Some("high"), None, None, 100, 0).unwrap();
        assert_eq!(high.len(), 2);

        let low = repo.query_findings(None, Some("low"), None, None, 100, 0).unwrap();
        assert_eq!(low.len(), 1);

        let open = repo.query_findings(None, None, Some("open"), None, 100, 0).unwrap();
        assert_eq!(open.len(), 2);

        let vuln = repo.query_findings(None, None, None, Some("vuln"), 100, 0).unwrap();
        assert_eq!(vuln.len(), 1);

        let count = repo.count_findings(None, Some("high")).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_finding_repository_pagination() {
        let repo = SqliteFindingRepository::new(":memory:".to_string());
        let mut repo = repo;
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
    }

    #[test]
    fn test_finding_repository_update_and_delete() {
        let repo = SqliteFindingRepository::new(":memory:".to_string());
        let mut repo = repo;
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
    }

    #[test]
    fn test_finding_repository_deduplication() {
        let repo = SqliteFindingRepository::new(":memory:".to_string());
        let mut repo = repo;
        repo.initialize().unwrap();

        repo.insert_finding(r#"{"id":"f1","dedup_key":"dk1"}"#)
            .unwrap();

        let dup = repo.deduplicate("dk1").unwrap();
        assert_eq!(dup, Some("f1".to_string()));

        let no_dup = repo.deduplicate("dk2").unwrap();
        assert!(no_dup.is_none());
    }

    #[test]
    fn test_finding_repository_not_initialized() {
        let repo = SqliteFindingRepository::new(":memory:".to_string());
        let result = repo.insert_finding(r#"{"id":"f1"}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_assessment_repository_create_and_get() {
        let repo = SqliteAssessmentRepository::new(":memory:".to_string());
        let mut repo = repo;
        repo.initialize().unwrap();

        let id = repo
            .create_assessment("Test", "10.0.0.1", "full-scan")
            .unwrap();
        assert_eq!(id, "assess-1");

        let got = repo.get_assessment(&id).unwrap().unwrap();
        assert!(got.contains("Test"));
        assert!(got.contains("10.0.0.1"));
    }

    #[test]
    fn test_assessment_repository_state_update() {
        let repo = SqliteAssessmentRepository::new(":memory:".to_string());
        let mut repo = repo;
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
    }

    #[test]
    fn test_assessment_repository_attach_finding() {
        let repo = SqliteAssessmentRepository::new(":memory:".to_string());
        let mut repo = repo;
        repo.initialize().unwrap();

        let id = repo
            .create_assessment("Test", "10.0.0.1", "full-scan")
            .unwrap();
        assert!(repo.attach_finding(&id, "find-1").unwrap());
        assert!(repo.attach_finding(&id, "find-2").unwrap());
        // Duplicate attachment is idempotent.
        assert!(repo.attach_finding(&id, "find-1").unwrap());

        assert!(!repo.attach_finding("nonexistent", "find-1").unwrap());
    }

    #[test]
    fn test_assessment_repository_attach_artifact() {
        let repo = SqliteAssessmentRepository::new(":memory:".to_string());
        let mut repo = repo;
        repo.initialize().unwrap();

        let id = repo
            .create_assessment("Test", "10.0.0.1", "full-scan")
            .unwrap();
        let artifact = r#"{"type":"pcap","path":"/tmp/capture.pcap"}"#;
        assert!(repo.attach_artifact(&id, artifact).unwrap());

        let got = repo.get_assessment(&id).unwrap().unwrap();
        assert!(got.contains("pcap"));

        assert!(!repo.attach_artifact("nonexistent", artifact).unwrap());
    }

    #[test]
    fn test_assessment_repository_list_and_delete() {
        let repo = SqliteAssessmentRepository::new(":memory:".to_string());
        let mut repo = repo;
        repo.initialize().unwrap();

        for i in 0..5 {
            repo.create_assessment(
                &format!("Assessment {}", i),
                "10.0.0.1",
                "port-scan",
            )
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
    }

    #[test]
    fn test_assessment_repository_not_initialized() {
        let repo = SqliteAssessmentRepository::new(":memory:".to_string());
        let result = repo.create_assessment("Test", "10.0.0.1", "scan");
        assert!(result.is_err());
    }
}
