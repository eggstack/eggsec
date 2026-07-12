use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::sync::Mutex;

/// Current checkpoint schema version.
pub const CHECKPOINT_SCHEMA_VERSION: u32 = 2;

/// Version wrapper for checkpoint schema evolution.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CheckpointVersion(pub u32);

impl CheckpointVersion {
    pub fn current() -> Self {
        Self(CHECKPOINT_SCHEMA_VERSION)
    }

    pub fn needs_migration(&self) -> bool {
        self.0 < CHECKPOINT_SCHEMA_VERSION
    }
}

/// A versioned checkpoint capturing pipeline execution state for resumption.
///
/// Checkpoints are versioned to support schema evolution. When loaded,
/// stale versions are migrated forward to the current schema.
#[pyclass(frozen)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PipelineCheckpoint {
    /// Schema version of this checkpoint.
    pub version: CheckpointVersion,
    /// Unique pipeline identifier (not the human-readable name).
    #[pyo3(get)]
    pub pipeline_id: String,
    /// Human-readable pipeline name.
    #[pyo3(get)]
    pub pipeline_name: String,
    /// Names of steps that completed successfully before interruption.
    #[pyo3(get)]
    pub completed_steps: Vec<String>,
    /// The step that was in progress when the checkpoint was created, if any.
    #[pyo3(get)]
    pub current_step: Option<String>,
    /// Serialized results for completed steps (JSON values keyed by step name).
    pub step_results: HashMap<String, serde_json::Value>,
    /// Epoch milliseconds when the checkpoint was created.
    #[pyo3(get)]
    pub created_at_ms: u64,
    /// Epoch milliseconds when the checkpoint was last updated.
    #[pyo3(get)]
    pub updated_at_ms: u64,
}

#[pymethods]
impl PipelineCheckpoint {
    #[new]
    #[pyo3(signature = (pipeline_id, pipeline_name, *, completed_steps=None, current_step=None, step_results=None, created_at_ms=0, updated_at_ms=0))]
    fn py_new(
        py: Python,
        pipeline_id: String,
        pipeline_name: String,
        completed_steps: Option<Vec<String>>,
        current_step: Option<String>,
        step_results: Option<PyObject>,
        created_at_ms: u64,
        updated_at_ms: u64,
    ) -> Self {
        let now = current_epoch_ms();
        let parsed_results = step_results
            .map(|obj| {
                let json_mod = py.import_bound("json")?;
                let json_str = json_mod
                    .call_method1("dumps", (obj,))
                    .map_err(|e| {
                        pyo3::exceptions::PyTypeError::new_err(format!(
                            "step_results must be JSON-serializable: {}",
                            e
                        ))
                    })?
                    .extract::<String>()?;
                let map: HashMap<String, serde_json::Value> = serde_json::from_str(&json_str)
                    .map_err(|e| {
                        pyo3::exceptions::PyValueError::new_err(format!(
                            "step_results must be a JSON object: {}",
                            e
                        ))
                    })?;
                Ok::<_, PyErr>(map)
            })
            .transpose()
            .unwrap_or_default()
            .unwrap_or_default();
        Self {
            version: CheckpointVersion::current(),
            pipeline_id,
            pipeline_name,
            completed_steps: completed_steps.unwrap_or_default(),
            current_step,
            step_results: parsed_results,
            created_at_ms: if created_at_ms == 0 {
                now
            } else {
                created_at_ms
            },
            updated_at_ms: if updated_at_ms == 0 {
                now
            } else {
                updated_at_ms
            },
        }
    }

    /// Returns true if this checkpoint is at the current schema version.
    fn is_current_version(&self) -> bool {
        self.version == CheckpointVersion::current()
    }

    /// Returns the schema version number.
    #[getter]
    fn version(&self) -> u32 {
        self.version.0
    }

    /// Returns step results as a Python dict (manual getter since serde_json::Value
    /// can't be auto-converted by PyO3).
    #[getter]
    fn step_results(&self, py: Python) -> PyResult<PyObject> {
        let results_dict = PyDict::new_bound(py);
        for (k, v) in &self.step_results {
            let json_str = serde_json::to_string(v)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            let json_mod = py.import_bound("json")?;
            let py_val = json_mod.call_method1("loads", (json_str,))?;
            results_dict.set_item(k, py_val)?;
        }
        Ok(results_dict.into())
    }

    /// Returns the name of the next step to execute (or None if all steps are done).
    fn next_step(&self, all_steps: Vec<String>) -> Option<String> {
        for step in &all_steps {
            if !self.completed_steps.contains(step) {
                return Some(step.clone());
            }
        }
        None
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("version", self.version.0)?;
        dict.set_item("pipeline_id", &self.pipeline_id)?;
        dict.set_item("pipeline_name", &self.pipeline_name)?;
        dict.set_item("completed_steps", &self.completed_steps)?;
        dict.set_item("current_step", &self.current_step)?;
        dict.set_item("created_at_ms", self.created_at_ms)?;
        dict.set_item("updated_at_ms", self.updated_at_ms)?;
        dict.set_item("step_results", self.step_results(py)?)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PipelineCheckpoint(version={}, pipeline_id={}, completed={}/{})",
            self.version.0,
            self.pipeline_id,
            self.completed_steps.len(),
            self.completed_steps.len() + if self.current_step.is_some() { 1 } else { 0 },
        )
    }
}

/// Result of loading a checkpoint, including migration status.
#[pyclass]
#[derive(Debug, Clone)]
pub struct CheckpointLoadResult {
    #[pyo3(get)]
    pub checkpoint: PipelineCheckpoint,
    #[pyo3(get)]
    pub migrated: bool,
    #[pyo3(get)]
    pub original_version: u32,
}

#[pymethods]
impl CheckpointLoadResult {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("checkpoint", self.checkpoint.to_dict(py)?)?;
        dict.set_item("migrated", self.migrated)?;
        dict.set_item("original_version", self.original_version)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "CheckpointLoadResult(migrated={}, original_version={})",
            self.migrated, self.original_version
        )
    }
}

/// Versioned checkpoint store with optional file persistence.
///
/// Checkpoints are keyed by pipeline ID. When persistence is enabled,
/// checkpoints are written to a JSON file after each save operation
/// and loaded from disk on startup.
#[pyclass]
#[derive(Debug)]
pub struct CheckpointStore {
    checkpoints: Mutex<HashMap<String, PipelineCheckpoint>>,
    persist_path: Option<String>,
}

impl Clone for CheckpointStore {
    fn clone(&self) -> Self {
        let data = self
            .checkpoints
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default();
        Self {
            checkpoints: Mutex::new(data),
            persist_path: self.persist_path.clone(),
        }
    }
}

#[pymethods]
impl CheckpointStore {
    /// Create an in-memory checkpoint store (no persistence).
    #[new]
    fn py_new() -> Self {
        Self::new()
    }

    /// Save a checkpoint (Python-facing). Overwrites any existing checkpoint for the same pipeline.
    fn save(&self, checkpoint: PipelineCheckpoint) -> PyResult<()> {
        self.save_inner(checkpoint)
    }

    /// Load a checkpoint by pipeline ID, performing version migration if needed.
    fn load(&self, pipeline_id: &str) -> PyResult<Option<CheckpointLoadResult>> {
        self.load_inner(pipeline_id)
    }

    /// Delete a checkpoint by pipeline ID. Returns true if found and deleted.
    fn delete(&self, pipeline_id: &str) -> PyResult<bool> {
        self.delete_inner(pipeline_id)
    }

    /// Load a checkpoint and return the step name to resume from.
    fn resume_from(
        &self,
        pipeline_id: &str,
        all_steps: Vec<String>,
    ) -> PyResult<Option<(CheckpointLoadResult, Option<String>)>> {
        self.resume_from_inner(pipeline_id, all_steps)
    }

    /// List all stored checkpoint pipeline IDs.
    fn list_pipeline_ids(&self) -> PyResult<Vec<String>> {
        let store = self.checkpoints.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
        })?;
        Ok(store.keys().cloned().collect())
    }

    /// Number of stored checkpoints.
    fn len(&self) -> PyResult<usize> {
        let store = self.checkpoints.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
        })?;
        Ok(store.len())
    }

    /// Returns true if no checkpoints are stored.
    fn is_empty(&self) -> PyResult<bool> {
        Ok(self.len()? == 0)
    }

    fn __repr__(&self) -> String {
        let len = self.len().unwrap_or(0);
        format!("CheckpointStore({} checkpoints)", len)
    }
}

impl CheckpointStore {
    /// Create an in-memory checkpoint store.
    pub fn new() -> Self {
        Self {
            checkpoints: Mutex::new(HashMap::new()),
            persist_path: None,
        }
    }

    /// Create a file-backed checkpoint store.
    ///
    /// If the file exists, checkpoints are loaded from it on creation.
    /// Future saves are written through to disk.
    pub fn with_persistence(path: &str) -> PyResult<Self> {
        let store = Self {
            checkpoints: Mutex::new(HashMap::new()),
            persist_path: Some(path.to_string()),
        };

        // Load existing checkpoints from disk
        if std::path::Path::new(path).exists() {
            let data = std::fs::read_to_string(path).map_err(|e| {
                pyo3::exceptions::PyIOError::new_err(format!(
                    "Failed to read checkpoint file '{}': {}",
                    path, e
                ))
            })?;
            if !data.trim().is_empty() {
                let file_data: CheckpointFileData = serde_json::from_str(&data).map_err(|e| {
                    pyo3::exceptions::PyValueError::new_err(format!(
                        "Failed to parse checkpoint file '{}': {}",
                        path, e
                    ))
                })?;
                let mut checkpoints = HashMap::new();
                for mut cp in file_data.checkpoints {
                    let _ = migrate_checkpoint(&mut cp);
                    checkpoints.insert(cp.pipeline_id.clone(), cp);
                }
                let mut locked = store.checkpoints.lock().map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
                })?;
                *locked = checkpoints;
            }
        }

        Ok(store)
    }

    /// Save a checkpoint (Rust-facing). Overwrites any existing checkpoint for the same pipeline.
    pub fn save_inner(&self, checkpoint: PipelineCheckpoint) -> PyResult<()> {
        let mut store = self.checkpoints.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
        })?;
        store.insert(checkpoint.pipeline_id.clone(), checkpoint);
        drop(store);

        if let Some(ref path) = self.persist_path {
            self.flush_to_disk()?;
        }
        Ok(())
    }

    /// Load a checkpoint by pipeline ID, performing version migration if needed.
    pub fn load_inner(&self, pipeline_id: &str) -> PyResult<Option<CheckpointLoadResult>> {
        let store = self.checkpoints.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
        })?;
        match store.get(pipeline_id) {
            Some(cp) => {
                let original_version = cp.version.0;
                let mut migrated_cp = cp.clone();
                let migrated = migrate_checkpoint(&mut migrated_cp)?;
                Ok(Some(CheckpointLoadResult {
                    checkpoint: migrated_cp,
                    migrated,
                    original_version,
                }))
            }
            None => Ok(None),
        }
    }

    /// Delete a checkpoint by pipeline ID. Returns true if found and deleted.
    pub fn delete_inner(&self, pipeline_id: &str) -> PyResult<bool> {
        let mut store = self.checkpoints.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
        })?;
        let existed = store.remove(pipeline_id).is_some();
        drop(store);

        if existed && self.persist_path.is_some() {
            self.flush_to_disk()?;
        }
        Ok(existed)
    }

    /// Load a checkpoint and return the step name to resume from.
    pub fn resume_from_inner(
        &self,
        pipeline_id: &str,
        all_steps: Vec<String>,
    ) -> PyResult<Option<(CheckpointLoadResult, Option<String>)>> {
        let load_result = self.load_inner(pipeline_id)?;
        match load_result {
            Some(lr) => {
                let next = lr.checkpoint.next_step(all_steps);
                Ok(Some((lr, next)))
            }
            None => Ok(None),
        }
    }

    /// Write all checkpoints to disk.
    fn flush_to_disk(&self) -> PyResult<()> {
        let path = match &self.persist_path {
            Some(p) => p.clone(),
            None => return Ok(()),
        };

        let store = self.checkpoints.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
        })?;

        let data = CheckpointFileData {
            schema_version: CHECKPOINT_SCHEMA_VERSION,
            checkpoints: store.values().cloned().collect(),
        };
        drop(store);

        let json = serde_json::to_string_pretty(&data).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Failed to serialize checkpoints: {}",
                e
            ))
        })?;

        std::fs::write(&path, json).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!(
                "Failed to write checkpoint file '{}': {}",
                path, e
            ))
        })?;

        Ok(())
    }
}

/// On-disk format wrapper.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CheckpointFileData {
    schema_version: u32,
    checkpoints: Vec<PipelineCheckpoint>,
}

/// Migrate a checkpoint from an older schema version to the current version.
///
/// Returns Ok(true) if migration was performed, Ok(false) if already current.
pub fn migrate_checkpoint(cp: &mut PipelineCheckpoint) -> PyResult<bool> {
    let original = cp.version.0;

    // v0/v1 → v2: Ensure updated_at_ms is set (was absent in v1).
    if cp.version.0 < 2 {
        cp.version = CheckpointVersion(2);
        if cp.updated_at_ms == 0 {
            cp.updated_at_ms = cp.created_at_ms;
        }
    }

    // Future migrations go here:
    // if cp.version.0 < 3 { ... cp.version = CheckpointVersion(3); }

    Ok(cp.version.0 != original)
}

/// Create a CheckpointStore backed by a file path.
#[pyfunction]
#[pyo3(signature = (path=None))]
pub fn create_checkpoint_store(path: Option<String>) -> PyResult<CheckpointStore> {
    match path {
        Some(p) => CheckpointStore::with_persistence(&p),
        None => Ok(CheckpointStore::new()),
    }
}

/// Current epoch time in milliseconds.
pub fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_version_current() {
        let v = CheckpointVersion::current();
        assert_eq!(v.0, CHECKPOINT_SCHEMA_VERSION);
        assert!(!v.needs_migration());
    }

    #[test]
    fn test_checkpoint_version_migration_needed() {
        let v = CheckpointVersion(1);
        assert!(v.needs_migration());
    }

    #[test]
    fn test_migrate_checkpoint_v1_to_v2() {
        let mut cp = PipelineCheckpoint {
            version: CheckpointVersion(1),
            pipeline_id: "test-pipeline".to_string(),
            pipeline_name: "Test".to_string(),
            completed_steps: vec!["step1".to_string()],
            current_step: Some("step2".to_string()),
            step_results: HashMap::new(),
            created_at_ms: 1000,
            updated_at_ms: 0,
        };

        let migrated = migrate_checkpoint(&mut cp).unwrap();
        assert!(migrated);
        assert_eq!(cp.version.0, 2);
        assert_eq!(cp.updated_at_ms, 1000);
    }

    #[test]
    fn test_migrate_checkpoint_already_current() {
        let mut cp = PipelineCheckpoint {
            version: CheckpointVersion::current(),
            pipeline_id: "test-pipeline".to_string(),
            pipeline_name: "Test".to_string(),
            completed_steps: vec![],
            current_step: None,
            step_results: HashMap::new(),
            created_at_ms: 1000,
            updated_at_ms: 2000,
        };

        let migrated = migrate_checkpoint(&mut cp).unwrap();
        assert!(!migrated);
    }

    #[test]
    fn test_in_memory_store_roundtrip() {
        let store = CheckpointStore::new();
        let mut results = HashMap::new();
        results.insert(
            "step1".to_string(),
            serde_json::json!({"status": "completed"}),
        );

        let cp = PipelineCheckpoint {
            version: CheckpointVersion::current(),
            pipeline_id: "p1".to_string(),
            pipeline_name: "Pipeline 1".to_string(),
            completed_steps: vec!["step1".to_string()],
            current_step: Some("step2".to_string()),
            step_results: results,
            created_at_ms: 1000,
            updated_at_ms: 1000,
        };

        store.save_inner(cp.clone()).unwrap();

        let loaded = store.load_inner("p1").unwrap().unwrap();
        assert!(!loaded.migrated);
        assert_eq!(loaded.original_version, CHECKPOINT_SCHEMA_VERSION);
        assert_eq!(loaded.checkpoint.pipeline_id, "p1");
        assert_eq!(loaded.checkpoint.completed_steps, vec!["step1"]);

        let not_found = store.load_inner("nonexistent").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_resume_from_returns_next_step() {
        let store = CheckpointStore::new();
        let cp = PipelineCheckpoint {
            version: CheckpointVersion::current(),
            pipeline_id: "p2".to_string(),
            pipeline_name: "Pipeline 2".to_string(),
            completed_steps: vec!["step1".to_string()],
            current_step: None,
            step_results: HashMap::new(),
            created_at_ms: 1000,
            updated_at_ms: 1000,
        };
        store.save_inner(cp).unwrap();

        let all_steps = vec![
            "step1".to_string(),
            "step2".to_string(),
            "step3".to_string(),
        ];
        let result = store.resume_from_inner("p2", all_steps).unwrap().unwrap();
        assert_eq!(result.1.unwrap(), "step2");
    }

    #[test]
    fn test_resume_from_complete_pipeline() {
        let store = CheckpointStore::new();
        let cp = PipelineCheckpoint {
            version: CheckpointVersion::current(),
            pipeline_id: "p3".to_string(),
            pipeline_name: "Pipeline 3".to_string(),
            completed_steps: vec!["step1".to_string(), "step2".to_string()],
            current_step: None,
            step_results: HashMap::new(),
            created_at_ms: 1000,
            updated_at_ms: 1000,
        };
        store.save_inner(cp).unwrap();

        let all_steps = vec!["step1".to_string(), "step2".to_string()];
        let result = store.resume_from_inner("p3", all_steps).unwrap().unwrap();
        assert!(result.1.is_none());
    }

    #[test]
    fn test_delete_checkpoint() {
        let store = CheckpointStore::new();
        let cp = PipelineCheckpoint {
            version: CheckpointVersion::current(),
            pipeline_id: "p4".to_string(),
            pipeline_name: "Pipeline 4".to_string(),
            completed_steps: vec![],
            current_step: None,
            step_results: HashMap::new(),
            created_at_ms: 1000,
            updated_at_ms: 1000,
        };
        store.save_inner(cp).unwrap();

        assert!(store.delete_inner("p4").unwrap());
        assert!(!store.delete_inner("p4").unwrap());
        assert!(store.load_inner("p4").unwrap().is_none());
    }

    #[test]
    fn test_file_persistence_roundtrip() {
        let dir = std::env::temp_dir().join("eggsec_checkpoint_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("checkpoints.json");

        // Create and save
        {
            let store = CheckpointStore::with_persistence(path.to_str().unwrap()).unwrap();
            let cp = PipelineCheckpoint {
                version: CheckpointVersion::current(),
                pipeline_id: "persist-test".to_string(),
                pipeline_name: "Persist Test".to_string(),
                completed_steps: vec!["a".to_string()],
                current_step: Some("b".to_string()),
                step_results: HashMap::new(),
                created_at_ms: 5000,
                updated_at_ms: 5000,
            };
            store.save_inner(cp).unwrap();
        }

        // Reload from disk
        {
            let store = CheckpointStore::with_persistence(path.to_str().unwrap()).unwrap();
            let loaded = store.load_inner("persist-test").unwrap().unwrap();
            assert_eq!(loaded.checkpoint.completed_steps, vec!["a"]);
            assert_eq!(loaded.checkpoint.current_step, Some("b".to_string()));
        }

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn test_list_pipeline_ids() {
        let store = CheckpointStore::new();
        for i in 0..3 {
            let cp = PipelineCheckpoint {
                version: CheckpointVersion::current(),
                pipeline_id: format!("p{}", i),
                pipeline_name: format!("Pipeline {}", i),
                completed_steps: vec![],
                current_step: None,
                step_results: HashMap::new(),
                created_at_ms: 1000,
                updated_at_ms: 1000,
            };
            store.save_inner(cp).unwrap();
        }

        let mut ids = store.list_pipeline_ids().unwrap();
        ids.sort();
        assert_eq!(ids, vec!["p0", "p1", "p2"]);
    }
}
