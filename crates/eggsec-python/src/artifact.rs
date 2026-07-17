use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyMemoryView};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::c_void;
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
    #[serde(skip)]
    content: Option<Vec<u8>>,
}

#[pymethods]
impl ArtifactPy {
    #[new]
    #[pyo3(signature = (id, name, mime_type, size_bytes, content_hash, *, provenance=None, redacted=None, retention_policy=None, external_uri=None, content=None))]
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
        content: Option<Vec<u8>>,
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
            content,
        }
    }

    /// Create an artifact with embedded binary content.
    #[staticmethod]
    fn with_content(
        id: String,
        name: String,
        mime_type: String,
        content: Vec<u8>,
        content_hash: String,
    ) -> Self {
        let size_bytes = content.len() as u64;
        Self {
            id,
            name,
            mime_type,
            size_bytes,
            content_hash,
            provenance: "scan".to_string(),
            redacted: false,
            retention_policy: "session".to_string(),
            external_uri: None,
            content: Some(content),
        }
    }

    /// Whether this artifact has embedded content.
    fn has_content(&self) -> bool {
        self.content.is_some()
    }

    /// Get the embedded content as bytes, if available.
    fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyBytes>>> {
        match &self.content {
            Some(data) => Ok(Some(PyBytes::new_bound(py, data))),
            None => Ok(None),
        }
    }

    /// Get the embedded content as a BinaryBuffer.
    fn buffer(&self) -> PyResult<crate::buffer_support::BinaryBufferPy> {
        match &self.content {
            Some(data) => Ok(crate::buffer_support::BinaryBufferPy::from_slice(data)),
            None => Err(pyo3::exceptions::PyValueError::new_err(
                "Artifact has no embedded content",
            )),
        }
    }

    /// Get the embedded content as hex string.
    fn hex(&self) -> PyResult<Option<String>> {
        match &self.content {
            Some(data) => Ok(Some(hex_encode(data))),
            None => Ok(None),
        }
    }

    /// Get a PEP 3118 memoryview of the embedded content.
    fn memoryview<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
        match &self.content {
            Some(data) => {
                let bytes = PyBytes::new_bound(py, data);
                Ok(Some(PyMemoryView::from_bound(&bytes)?))
            }
            None => Ok(None),
        }
    }

    /// PEP 3118 buffer protocol: expose the embedded content.
    ///
    /// # Safety
    /// Fills a raw `ffi::Py_buffer` per PEP 3118.
    unsafe fn __getbuffer__(
        slf: PyRef<'_, Self>,
        view: *mut pyo3::ffi::Py_buffer,
        _flags: i32,
    ) -> PyResult<()> {
        if view.is_null() {
            return Err(pyo3::exceptions::PyBufferError::new_err("view is null"));
        }
        match &slf.content {
            Some(data) => {
                let ptr = data.as_ptr() as *const c_void;
                let len = data.len() as isize;

                (*view).obj = std::ptr::null_mut();
                (*view).buf = ptr as *mut c_void;
                (*view).len = len;
                (*view).readonly = 0;
                (*view).itemsize = 1;
                (*view).format = b"b\0".as_ptr() as *mut _;
                (*view).ndim = 1;
                (*view).shape = std::ptr::addr_of!(len) as *mut _;
                (*view).strides = std::ptr::null_mut();
                (*view).suboffsets = std::ptr::null_mut();
                (*view).internal = std::ptr::null_mut();
                Ok(())
            }
            None => Err(pyo3::exceptions::PyBufferError::new_err(
                "Artifact has no embedded content",
            )),
        }
    }

    /// PEP 3118 buffer protocol: release.
    ///
    /// # Safety
    /// Called by Python when the buffer is no longer needed.
    unsafe fn __releasebuffer__(&self, _view: *mut pyo3::ffi::Py_buffer) {}

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("name", "[REDACTED]")?;
        dict.set_item("mime_type", &self.mime_type)?;
        dict.set_item("size_bytes", self.size_bytes)?;
        dict.set_item("content_hash", &self.content_hash)?;
        dict.set_item("provenance", &self.provenance)?;
        dict.set_item("redacted", self.redacted)?;
        dict.set_item("retention_policy", &self.retention_policy)?;
        dict.set_item("external_uri", &self.external_uri)?;
        dict.set_item("has_content", self.content.is_some())?;
        Ok(dict.into())
    }

    fn to_dict_raw(&self, py: Python) -> PyResult<PyObject> {
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
        dict.set_item("has_content", self.content.is_some())?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        let redacted = ArtifactPy {
            id: self.id.clone(),
            name: "[REDACTED]".to_string(),
            mime_type: self.mime_type.clone(),
            size_bytes: self.size_bytes,
            content_hash: self.content_hash.clone(),
            provenance: self.provenance.clone(),
            redacted: self.redacted,
            retention_policy: self.retention_policy.clone(),
            external_uri: self.external_uri.clone(),
            content: None,
        };
        serde_json::to_string(&redacted)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn to_json_raw(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "Artifact(id={}, mime_type={}, has_content={})",
            self.id,
            self.mime_type,
            self.content.is_some()
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

const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

fn hex_encode(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len() * 2);
    for &byte in data {
        s.push(HEX_CHARS[(byte >> 4) as usize] as char);
        s.push(HEX_CHARS[(byte & 0x0f) as usize] as char);
    }
    s
}
