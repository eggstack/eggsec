use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn validate_plugin_path(base: &Path, user_path: &Path) -> Result<PathBuf> {
    let canonical = user_path
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Failed to canonicalize path: {}", e))?;
    let base_canonical = base
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Failed to canonicalize base path: {}", e))?;
    if !canonical.starts_with(&base_canonical) {
        return Err(anyhow::anyhow!("Path traversal detected"));
    }
    Ok(canonical)
}
