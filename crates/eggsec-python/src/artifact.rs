use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// A binary or text artifact produced during security testing.
#[pyclass(frozen, name = "MilestoneArtifact")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactPy {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub mime_type: String,
    #[pyo3(get)]
    pub size_bytes: u64,
    #[pyo3(get)]
    pub content_hash: String,
    #[pyo3(get)]
    pub provenance: String,
    #[pyo3(get)]
    pub redacted: bool,
    #[pyo3(get)]
    pub retention_policy: String,
    #[pyo3(get)]
    pub external_uri: Option<String>,
}

#[pymethods]
impl ArtifactPy {
    #[new]
    #[pyo3(signature = (id, name, mime_type, size_bytes, content_hash, *, provenance=None, redacted=None, retention_policy=None, external_uri=None))]
    fn new(
        id: String,
        name: String,
        mime_type: String,
        size_bytes: u64,
        content_hash: String,
        provenance: Option<String>,
        redacted: Option<bool>,
        retention_policy: Option<String>,
        external_uri: Option<String>,
    ) -> Self {
        Self {
            id,
            name,
            mime_type,
            size_bytes,
            content_hash,
            provenance: provenance.unwrap_or_else(|| "scan".to_string()),
            redacted: redacted.unwrap_or(false),
            retention_policy: retention_policy.unwrap_or_else(|| "session".to_string()),
            external_uri,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("name", &self.name)?;
        dict.set_item("mime_type", &self.mime_type)?;
        dict.set_item("size_bytes", self.size_bytes)?;
        dict.set_item("content_hash", &self.content_hash)?;
        dict.set_item("provenance", &self.provenance)?;
        dict.set_item("redacted", self.redacted)?;
        dict.set_item("retention_policy", &self.retention_policy)?;
        dict.set_item("external_uri", &self.external_uri)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "Artifact(id={}, name={}, mime_type={})",
            self.id, self.name, self.mime_type
        )
    }
}

/// Links an artifact to a finding with a specific role.
#[pyclass(frozen, name = "ArtifactReference")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactReferencePy {
    #[pyo3(get)]
    pub artifact_id: String,
    #[pyo3(get)]
    pub finding_id: String,
    #[pyo3(get)]
    pub role: String,
}

#[pymethods]
impl ArtifactReferencePy {
    #[new]
    fn new(artifact_id: String, finding_id: String, role: String) -> Self {
        Self {
            artifact_id,
            finding_id,
            role,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("artifact_id", &self.artifact_id)?;
        dict.set_item("finding_id", &self.finding_id)?;
        dict.set_item("role", &self.role)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ArtifactReference(artifact_id={}, finding_id={}, role={})",
            self.artifact_id, self.finding_id, self.role
        )
    }
}

/// Thread-safe store for artifacts produced during a scan.
#[pyclass(name = "ArtifactStore")]
pub struct ArtifactStorePy {
    artifacts: RwLock<HashMap<String, ArtifactPy>>,
}

#[pymethods]
impl ArtifactStorePy {
    #[new]
    fn new() -> Self {
        Self {
            artifacts: RwLock::new(HashMap::new()),
        }
    }

    /// Store an artifact and return its ID.
    fn store(&self, artifact: ArtifactPy) -> String {
        let id = artifact.id.clone();
        self.artifacts.write().unwrap().insert(id.clone(), artifact);
        id
    }

    /// Retrieve an artifact by ID.
    fn get(&self, artifact_id: &str) -> Option<ArtifactPy> {
        self.artifacts.read().unwrap().get(artifact_id).cloned()
    }

    /// Check if an artifact exists.
    fn contains(&self, artifact_id: &str) -> bool {
        self.artifacts.read().unwrap().contains_key(artifact_id)
    }

    /// Remove an artifact by ID. Returns true if it existed.
    fn remove(&self, artifact_id: &str) -> bool {
        self.artifacts
            .write()
            .unwrap()
            .remove(artifact_id)
            .is_some()
    }

    /// Number of stored artifacts.
    fn __len__(&self) -> usize {
        self.artifacts.read().unwrap().len()
    }

    /// Number of stored artifacts (method alias).
    fn len(&self) -> usize {
        self.artifacts.read().unwrap().len()
    }

    /// Whether the store is empty.
    fn is_empty(&self) -> bool {
        self.artifacts.read().unwrap().is_empty()
    }

    /// List all artifact IDs.
    fn list_ids(&self) -> Vec<String> {
        self.artifacts.read().unwrap().keys().cloned().collect()
    }

    /// Convert to a dictionary mapping artifact IDs to artifact dicts.
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        let artifacts = self.artifacts.read().unwrap();
        for (id, artifact) in artifacts.iter() {
            dict.set_item(id, artifact.to_dict(py)?)?;
        }
        Ok(dict.into())
    }

    /// Serialize all artifacts to JSON.
    fn to_json(&self) -> PyResult<String> {
        let artifacts = self.artifacts.read().unwrap();
        let map: HashMap<&str, &ArtifactPy> =
            artifacts.iter().map(|(k, v)| (k.as_str(), v)).collect();
        serde_json::to_string(&map)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        let len = self.artifacts.read().unwrap().len();
        format!("ArtifactStore(artifacts={len})")
    }
}
