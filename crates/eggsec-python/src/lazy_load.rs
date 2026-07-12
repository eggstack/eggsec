use pyo3::prelude::*;
use std::path::{Path, PathBuf};

use crate::buffer_support::BinaryBufferPy;

/// Metadata about an artifact that can be read without loading its content.
#[pyclass(name = "ArtifactMeta")]
#[derive(Debug, Clone)]
pub struct ArtifactMetaPy {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub kind: String,
    #[pyo3(get)]
    pub mime_type: String,
    #[pyo3(get)]
    pub size: u64,
    #[pyo3(get)]
    pub content_hash: Option<String>,
}

#[pymethods]
impl ArtifactMetaPy {
    #[new]
    #[pyo3(signature = (name, kind, mime_type, size, *, content_hash=None))]
    fn new(
        name: String,
        kind: String,
        mime_type: String,
        size: u64,
        content_hash: Option<String>,
    ) -> Self {
        Self {
            name,
            kind,
            mime_type,
            size,
            content_hash,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ArtifactMeta(name={}, kind={}, size={})",
            self.name, self.kind, self.size
        )
    }
}

/// Lazy-loaded artifact — defers file read until accessed.
///
/// Holds a path and metadata without reading the file content.
/// Call `load()` to read the data into memory, returning a `BinaryBuffer`.
#[pyclass(name = "LazyArtifact")]
pub struct LazyArtifactPy {
    path: PathBuf,
    metadata: ArtifactMetaPy,
    loaded: Option<Vec<u8>>,
}

#[pymethods]
impl LazyArtifactPy {
    #[new]
    #[pyo3(signature = (path, metadata))]
    fn new(path: PathBuf, metadata: ArtifactMetaPy) -> Self {
        Self {
            path,
            metadata,
            loaded: None,
        }
    }

    /// Artifact name (without loading content).
    fn name(&self) -> &str {
        &self.metadata.name
    }

    /// Artifact kind (e.g. "pcap", "screenshot", "log").
    fn kind(&self) -> &str {
        &self.metadata.kind
    }

    /// MIME type.
    fn mime_type(&self) -> &str {
        &self.metadata.mime_type
    }

    /// Size in bytes (from metadata, not necessarily actual file size).
    fn size_bytes(&self) -> u64 {
        self.metadata.size
    }

    /// Content hash (may be None).
    #[getter]
    fn content_hash(&self) -> Option<&str> {
        self.metadata.content_hash.as_deref()
    }

    /// File path on disk.
    fn path(&self) -> &Path {
        &self.path
    }

    /// Read the file into memory and return a `BinaryBuffer`.
    fn load(&mut self) -> PyResult<BinaryBufferPy> {
        if let Some(ref data) = self.loaded {
            return Ok(BinaryBufferPy::from_slice(data));
        }
        let data = std::fs::read(&self.path)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        let buf = BinaryBufferPy::from_slice(&data);
        self.loaded = Some(data);
        Ok(buf)
    }

    /// Drop loaded data to free memory.
    fn unload(&mut self) {
        self.loaded = None;
    }

    /// Whether the content is currently loaded in memory.
    fn is_loaded(&self) -> bool {
        self.loaded.is_some()
    }

    /// Get loaded data as bytes without re-reading the file.
    /// Returns None if not loaded.
    fn loaded_bytes(&self) -> Option<Vec<u8>> {
        self.loaded.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "LazyArtifact(path={}, loaded={})",
            self.path.display(),
            self.loaded.is_some()
        )
    }
}
