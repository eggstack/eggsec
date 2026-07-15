use md5::{Digest, Md5};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use crate::checkpoint_store::current_epoch_ms;

/// Metadata about a stored artifact.
#[pyclass(frozen, name = "ArtifactInfo")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactInfo {
    #[pyo3(get)]
    pub artifact_id: String,
    #[pyo3(get)]
    pub content_hash: String,
    #[pyo3(get)]
    pub content_type: String,
    #[pyo3(get)]
    pub size_bytes: u64,
    #[pyo3(get)]
    pub created_at_ms: u64,
    #[pyo3(get)]
    pub metadata: Option<String>,
    #[pyo3(get)]
    pub redacted: bool,
}

#[pymethods]
impl ArtifactInfo {
    #[new]
    #[pyo3(signature = (artifact_id, content_hash, content_type, size_bytes, *, created_at_ms=0, metadata=None, redacted=false))]
    fn new(
        artifact_id: String,
        content_hash: String,
        content_type: String,
        size_bytes: u64,
        created_at_ms: u64,
        metadata: Option<String>,
        redacted: bool,
    ) -> Self {
        let ts = if created_at_ms == 0 {
            current_epoch_ms()
        } else {
            created_at_ms
        };
        Self {
            artifact_id,
            content_hash,
            content_type,
            size_bytes,
            created_at_ms: ts,
            metadata,
            redacted,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("artifact_id", &self.artifact_id)?;
        dict.set_item("content_hash", &self.content_hash)?;
        dict.set_item("content_type", &self.content_type)?;
        dict.set_item("size_bytes", self.size_bytes)?;
        dict.set_item("created_at_ms", self.created_at_ms)?;
        dict.set_item("metadata", &self.metadata)?;
        dict.set_item("redacted", self.redacted)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ArtifactInfo(artifact_id={}, content_hash={}, content_type={}, size_bytes={})",
            self.artifact_id, self.content_hash, self.content_type, self.size_bytes,
        )
    }
}

/// An artifact with its full binary content.
#[pyclass(frozen, name = "ArtifactData")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactData {
    #[pyo3(get)]
    pub info: ArtifactInfo,
    #[serde(skip)]
    pub data: Vec<u8>,
}

#[pymethods]
impl ArtifactData {
    #[new]
    #[pyo3(signature = (info, data))]
    fn new(info: ArtifactInfo, data: Vec<u8>) -> Self {
        Self { info, data }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("info", self.info.to_dict(py)?)?;
        dict.set_item("data_len", self.data.len())?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ArtifactData(artifact_id={}, data_len={})",
            self.info.artifact_id,
            self.data.len(),
        )
    }
}

/// Result of a content-hash integrity verification.
#[pyclass(frozen, name = "IntegrityResult")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityResult {
    #[pyo3(get)]
    pub valid: bool,
    #[pyo3(get)]
    pub expected_hash: String,
    #[pyo3(get)]
    pub actual_hash: String,
    #[pyo3(get)]
    pub size_bytes: u64,
    #[pyo3(get)]
    pub verified_at_ms: u64,
}

#[pymethods]
impl IntegrityResult {
    #[new]
    #[pyo3(signature = (valid, expected_hash, actual_hash, size_bytes, *, verified_at_ms=0))]
    fn new(
        valid: bool,
        expected_hash: String,
        actual_hash: String,
        size_bytes: u64,
        verified_at_ms: u64,
    ) -> Self {
        let ts = if verified_at_ms == 0 {
            current_epoch_ms()
        } else {
            verified_at_ms
        };
        Self {
            valid,
            expected_hash,
            actual_hash,
            size_bytes,
            verified_at_ms: ts,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("valid", self.valid)?;
        dict.set_item("expected_hash", &self.expected_hash)?;
        dict.set_item("actual_hash", &self.actual_hash)?;
        dict.set_item("size_bytes", self.size_bytes)?;
        dict.set_item("verified_at_ms", self.verified_at_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "IntegrityResult(valid={}, expected={}, actual={})",
            self.valid, self.expected_hash, self.actual_hash,
        )
    }
}

/// Parameters for querying artifacts from a store.
#[pyclass(frozen, name = "ArtifactQuery")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactQuery {
    #[pyo3(get)]
    pub content_type: Option<String>,
    #[pyo3(get)]
    pub min_size: Option<u64>,
    #[pyo3(get)]
    pub max_size: Option<u64>,
    #[pyo3(get)]
    pub created_after_ms: Option<u64>,
    #[pyo3(get)]
    pub created_before_ms: Option<u64>,
    #[pyo3(get)]
    pub limit: u64,
    #[pyo3(get)]
    pub offset: u64,
}

#[pymethods]
impl ArtifactQuery {
    #[new]
    #[pyo3(signature = (*, content_type=None, min_size=None, max_size=None, created_after_ms=None, created_before_ms=None, limit=100, offset=0))]
    fn new(
        content_type: Option<String>,
        min_size: Option<u64>,
        max_size: Option<u64>,
        created_after_ms: Option<u64>,
        created_before_ms: Option<u64>,
        limit: u64,
        offset: u64,
    ) -> Self {
        Self {
            content_type,
            min_size,
            max_size,
            created_after_ms,
            created_before_ms,
            limit,
            offset,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("content_type", &self.content_type)?;
        dict.set_item("min_size", &self.min_size)?;
        dict.set_item("max_size", &self.max_size)?;
        dict.set_item("created_after_ms", &self.created_after_ms)?;
        dict.set_item("created_before_ms", &self.created_before_ms)?;
        dict.set_item("limit", self.limit)?;
        dict.set_item("offset", self.offset)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ArtifactQuery(content_type={:?}, limit={}, offset={})",
            self.content_type, self.limit, self.offset,
        )
    }
}

fn compute_md5_hex(data: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

// ---------------------------------------------------------------------------
// ContentAddressedArtifactStore
// ---------------------------------------------------------------------------

type CausalEntry = (Vec<u8>, String, u64, u64, Option<String>);

/// Content-addressed artifact store.
///
/// Artifacts are stored and retrieved by their content hash. Identical content
/// is deduplicated automatically.
#[pyclass(name = "ContentAddressedArtifactStore")]
pub struct ContentAddressedArtifactStore {
    base_dir: String,
    entries: Mutex<HashMap<String, CausalEntry>>,
}

impl Clone for ContentAddressedArtifactStore {
    fn clone(&self) -> Self {
        let entries = self.entries.lock().map(|g| g.clone()).unwrap_or_default();
        Self {
            base_dir: self.base_dir.clone(),
            entries: Mutex::new(entries),
        }
    }
}

#[pymethods]
impl ContentAddressedArtifactStore {
    #[new]
    fn new(base_dir: String) -> Self {
        Self {
            base_dir,
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Create the base directory on disk if it does not already exist.
    fn initialize(&self) -> PyResult<()> {
        fs::create_dir_all(&self.base_dir).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!(
                "Failed to create base directory '{}': {}",
                self.base_dir, e,
            ))
        })
    }

    /// Store data and return its artifact info. The content hash is used as the
    /// storage key, so identical payloads are deduplicated.
    fn put(
        &self,
        py: Python<'_>,
        data: &Bound<'_, PyBytes>,
        content_type: &str,
        metadata_json: Option<&str>,
    ) -> PyResult<ArtifactInfo> {
        let bytes = data.as_bytes();
        let hash = compute_md5_hex(bytes);
        let size = bytes.len() as u64;
        let now = current_epoch_ms();

        let artifact_id = hash.clone();

        // Persist to disk
        let file_path = Path::new(&self.base_dir).join(&hash);
        fs::write(&file_path, bytes).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!(
                "Failed to write artifact '{}': {}",
                hash, e,
            ))
        })?;

        let info = ArtifactInfo {
            artifact_id: artifact_id.clone(),
            content_hash: hash.clone(),
            content_type: content_type.to_string(),
            size_bytes: size,
            created_at_ms: now,
            metadata: metadata_json.map(|s| s.to_string()),
            redacted: false,
        };

        let mut entries = self.entries.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
        })?;
        entries.insert(
            hash,
            (
                bytes.to_vec(),
                content_type.to_string(),
                size,
                now,
                metadata_json.map(|s| s.to_string()),
            ),
        );

        Ok(info)
    }

    /// Retrieve an artifact by its content hash.
    fn get<'py>(&self, py: Python<'py>, hash: &str) -> PyResult<Option<ArtifactData>> {
        let entries = self.entries.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
        })?;
        match entries.get(hash) {
            Some((data, content_type, size, created_at_ms, metadata)) => {
                let info = ArtifactInfo {
                    artifact_id: hash.to_string(),
                    content_hash: hash.to_string(),
                    content_type: content_type.clone(),
                    size_bytes: *size,
                    created_at_ms: *created_at_ms,
                    metadata: metadata.clone(),
                    redacted: false,
                };
                Ok(Some(ArtifactData {
                    info,
                    data: data.clone(),
                }))
            }
            None => Ok(None),
        }
    }

    /// Check whether an artifact with the given hash exists.
    fn has(&self, hash: &str) -> PyResult<bool> {
        let entries = self.entries.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
        })?;
        Ok(entries.contains_key(hash))
    }

    /// Delete an artifact by hash. Returns true if the artifact existed.
    fn delete(&self, hash: &str) -> PyResult<bool> {
        let removed = {
            let mut entries = self.entries.lock().map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
            })?;
            entries.remove(hash).is_some()
        };
        if removed {
            let file_path = Path::new(&self.base_dir).join(hash);
            let _ = fs::remove_file(&file_path);
        }
        Ok(removed)
    }

    /// Verify that stored content matches the expected hash.
    fn verify(&self, hash: &str) -> PyResult<IntegrityResult> {
        let entries = self.entries.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
        })?;
        match entries.get(hash) {
            Some((data, _, size, _, _)) => {
                let actual = compute_md5_hex(data);
                let valid = actual == hash;
                Ok(IntegrityResult {
                    valid,
                    expected_hash: hash.to_string(),
                    actual_hash: actual,
                    size_bytes: *size,
                    verified_at_ms: current_epoch_ms(),
                })
            }
            None => Ok(IntegrityResult {
                valid: false,
                expected_hash: hash.to_string(),
                actual_hash: String::new(),
                size_bytes: 0,
                verified_at_ms: current_epoch_ms(),
            }),
        }
    }

    /// List stored artifacts with pagination.
    fn list_artifacts(&self, limit: u64, offset: u64) -> PyResult<Vec<ArtifactInfo>> {
        let entries = self.entries.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
        })?;
        let mut infos: Vec<ArtifactInfo> = entries
            .iter()
            .map(
                |(hash, (_, content_type, size, created_at_ms, metadata))| ArtifactInfo {
                    artifact_id: hash.clone(),
                    content_hash: hash.clone(),
                    content_type: content_type.clone(),
                    size_bytes: *size,
                    created_at_ms: *created_at_ms,
                    metadata: metadata.clone(),
                    redacted: false,
                },
            )
            .collect();
        // Sort by created_at_ms descending for stable ordering
        infos.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));
        let start = offset as usize;
        let end = (start + limit as usize).min(infos.len());
        if start >= infos.len() {
            Ok(vec![])
        } else {
            Ok(infos[start..end].to_vec())
        }
    }

    /// Return the size in bytes of the artifact with the given hash.
    fn size_bytes(&self, hash: &str) -> PyResult<Option<u64>> {
        let entries = self.entries.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
        })?;
        Ok(entries.get(hash).map(|(_, _, size, _, _)| *size))
    }

    /// Total size in bytes across all stored artifacts.
    fn total_size_bytes(&self) -> PyResult<u64> {
        let entries = self.entries.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
        })?;
        Ok(entries.values().map(|(_, _, size, _, _)| *size).sum())
    }

    /// Remove artifacts matching age and/or size constraints.
    /// Returns the number of pruned artifacts.
    fn prune(&self, max_age_secs: Option<u64>, max_size_bytes: Option<u64>) -> PyResult<u64> {
        let now = current_epoch_ms();
        let mut removed_hashes: Vec<String> = Vec::new();

        {
            let entries = self.entries.lock().map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
            })?;
            for (hash, (_, _, size, created_at_ms, _)) in entries.iter() {
                let mut prune = false;
                if let Some(max_age) = max_age_secs {
                    let age_ms = max_age * 1000;
                    if now.saturating_sub(*created_at_ms) > age_ms {
                        prune = true;
                    }
                }
                if let Some(max_size) = max_size_bytes {
                    if *size > max_size {
                        prune = true;
                    }
                }
                if prune {
                    removed_hashes.push(hash.clone());
                }
            }
        }

        let count = removed_hashes.len() as u64;
        {
            let mut entries = self.entries.lock().map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
            })?;
            for hash in &removed_hashes {
                entries.remove(hash);
                let file_path = Path::new(&self.base_dir).join(hash);
                let _ = fs::remove_file(&file_path);
            }
        }

        Ok(count)
    }

    /// Context manager entry point.
    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Context manager exit point — performs no cleanup.
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        false
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("base_dir", &self.base_dir)?;
        let entries = self.entries.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
        })?;
        dict.set_item("count", entries.len())?;
        dict.set_item(
            "total_size_bytes",
            entries
                .values()
                .map(|(_, _, size, _, _)| *size)
                .sum::<u64>(),
        )?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        let snapshot = {
            let entries = self.entries.lock().map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
            })?;
            entries
                .iter()
                .map(|(hash, (_, ct, sz, ts, meta))| {
                    (
                        hash.clone(),
                        ArtifactInfo {
                            artifact_id: hash.clone(),
                            content_hash: hash.clone(),
                            content_type: ct.clone(),
                            size_bytes: *sz,
                            created_at_ms: *ts,
                            metadata: meta.clone(),
                            redacted: false,
                        },
                    )
                })
                .collect::<Vec<_>>()
        };
        serde_json::to_string(&snapshot)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        let count = self.entries.lock().map(|g| g.len()).unwrap_or(0);
        format!(
            "ContentAddressedArtifactStore(base_dir={}, artifacts={})",
            self.base_dir, count,
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

// ---------------------------------------------------------------------------
// DirectoryArtifactStore
// ---------------------------------------------------------------------------

type DirEntry = (Vec<u8>, String, u64, u64);

/// Directory-backed artifact store where artifacts are keyed by name.
#[pyclass(name = "DirectoryArtifactStore")]
pub struct DirectoryArtifactStore {
    base_dir: String,
    flat: bool,
    entries: Mutex<HashMap<String, DirEntry>>,
}

impl Clone for DirectoryArtifactStore {
    fn clone(&self) -> Self {
        let entries = self.entries.lock().map(|g| g.clone()).unwrap_or_default();
        Self {
            base_dir: self.base_dir.clone(),
            flat: self.flat,
            entries: Mutex::new(entries),
        }
    }
}

#[pymethods]
impl DirectoryArtifactStore {
    #[new]
    #[pyo3(signature = (base_dir, *, flat=true))]
    fn new(base_dir: String, flat: bool) -> Self {
        Self {
            base_dir,
            flat,
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Create the base directory on disk if it does not already exist.
    fn initialize(&self) -> PyResult<()> {
        fs::create_dir_all(&self.base_dir).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!(
                "Failed to create base directory '{}': {}",
                self.base_dir, e,
            ))
        })
    }

    /// Store data under the given name and return artifact info.
    fn put<'py>(
        &self,
        py: Python<'py>,
        name: &str,
        data: &Bound<'py, PyBytes>,
        content_type: &str,
    ) -> PyResult<ArtifactInfo> {
        let bytes = data.as_bytes();
        let hash = compute_md5_hex(bytes);
        let size = bytes.len() as u64;
        let now = current_epoch_ms();

        // Resolve the on-disk path
        let file_path = self.resolve_file_path(name);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                pyo3::exceptions::PyIOError::new_err(format!(
                    "Failed to create parent directory for '{}': {}",
                    name, e,
                ))
            })?;
        }
        fs::write(&file_path, bytes).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!(
                "Failed to write artifact '{}': {}",
                name, e,
            ))
        })?;

        let info = ArtifactInfo {
            artifact_id: name.to_string(),
            content_hash: hash,
            content_type: content_type.to_string(),
            size_bytes: size,
            created_at_ms: now,
            metadata: None,
            redacted: false,
        };

        let mut entries = self.entries.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
        })?;
        entries.insert(
            name.to_string(),
            (bytes.to_vec(), content_type.to_string(), size, now),
        );

        Ok(info)
    }

    /// Retrieve an artifact by name.
    fn get<'py>(&self, py: Python<'py>, name: &str) -> PyResult<Option<ArtifactData>> {
        let entries = self.entries.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
        })?;
        match entries.get(name) {
            Some((data, content_type, size, created_at_ms)) => {
                let hash = compute_md5_hex(data);
                let info = ArtifactInfo {
                    artifact_id: name.to_string(),
                    content_hash: hash,
                    content_type: content_type.clone(),
                    size_bytes: *size,
                    created_at_ms: *created_at_ms,
                    metadata: None,
                    redacted: false,
                };
                Ok(Some(ArtifactData {
                    info,
                    data: data.clone(),
                }))
            }
            None => Ok(None),
        }
    }

    /// Check whether an artifact with the given name exists.
    fn has(&self, name: &str) -> PyResult<bool> {
        let entries = self.entries.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
        })?;
        Ok(entries.contains_key(name))
    }

    /// Delete an artifact by name. Returns true if it existed.
    fn delete(&self, name: &str) -> PyResult<bool> {
        let removed = {
            let mut entries = self.entries.lock().map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
            })?;
            entries.remove(name).is_some()
        };
        if removed {
            let file_path = self.resolve_file_path(name);
            let _ = fs::remove_file(&file_path);
        }
        Ok(removed)
    }

    /// List stored artifacts with pagination.
    fn list_artifacts(&self, limit: u64, offset: u64) -> PyResult<Vec<ArtifactInfo>> {
        let entries = self.entries.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
        })?;
        let mut infos: Vec<ArtifactInfo> = entries
            .iter()
            .map(
                |(name, (_, content_type, size, created_at_ms))| ArtifactInfo {
                    artifact_id: name.clone(),
                    content_hash: String::new(),
                    content_type: content_type.clone(),
                    size_bytes: *size,
                    created_at_ms: *created_at_ms,
                    metadata: None,
                    redacted: false,
                },
            )
            .collect();
        infos.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));
        let start = offset as usize;
        let end = (start + limit as usize).min(infos.len());
        if start >= infos.len() {
            Ok(vec![])
        } else {
            Ok(infos[start..end].to_vec())
        }
    }

    /// Resolve the on-disk file path for a given artifact name.
    fn resolve_path(&self, name: &str) -> Option<String> {
        let path = self.resolve_file_path(name);
        if path.exists() {
            path.to_str().map(|s| s.to_string())
        } else {
            None
        }
    }

    /// Context manager entry point.
    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Context manager exit point — performs no cleanup.
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        false
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("base_dir", &self.base_dir)?;
        dict.set_item("flat", self.flat)?;
        let entries = self.entries.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
        })?;
        dict.set_item("count", entries.len())?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        let snapshot: Vec<(String, u64)> = {
            let entries = self.entries.lock().map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Lock poisoned: {}", e))
            })?;
            entries
                .iter()
                .map(|(name, (_, _, size, _))| (name.clone(), *size))
                .collect()
        };
        serde_json::to_string(&snapshot)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        let count = self.entries.lock().map(|g| g.len()).unwrap_or(0);
        format!(
            "DirectoryArtifactStore(base_dir={}, flat={}, artifacts={})",
            self.base_dir, self.flat, count,
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl DirectoryArtifactStore {
    fn resolve_file_path(&self, name: &str) -> std::path::PathBuf {
        let base = Path::new(&self.base_dir);
        if self.flat {
            // Use the name directly under base_dir (flat layout)
            base.join(name)
        } else {
            // Use the name as a relative path (hierarchical layout)
            base.join(name)
        }
    }
}
