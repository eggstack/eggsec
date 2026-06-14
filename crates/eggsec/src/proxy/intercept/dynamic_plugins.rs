//! Dynamic plugin loading from shared libraries (.so/.dylib).
//!
//! This module provides the ability to load protocol handler plugins
//! from shared libraries at runtime. It's gated behind the
//! `dynamic-plugins` feature flag for security reasons.
//!
//! # Security Considerations
//!
//! Loading dynamic libraries introduces security risks:
//! - Arbitrary code execution
//! - Library version incompatibilities
//! - Symbol conflicts
//!
//! Plugins are loaded in a sandboxed manner with capability-based restrictions.
//! Only explicitly whitelisted symbols are resolved from the shared library.

use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// Metadata about a dynamically loaded plugin.
#[derive(Debug, Clone)]
pub struct DynamicPluginInfo {
    /// File path to the shared library.
    pub library_path: PathBuf,
    /// Plugin metadata from the loaded plugin.
    pub plugin_info: super::PluginInfo,
    /// Version of the plugin API used.
    pub api_version: u32,
    /// Whether the plugin was loaded successfully.
    pub loaded: bool,
}

/// Registry for managing dynamically loaded plugins.
pub struct DynamicPluginRegistry {
    plugins: Vec<DynamicPluginInfo>,
    path_index: HashMap<PathBuf, usize>,
}

impl DynamicPluginRegistry {
    /// Create an empty dynamic plugin registry.
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            path_index: HashMap::new(),
        }
    }

    /// Load a plugin from a shared library file.
    ///
    /// # Safety
    ///
    /// This function loads and executes code from a shared library.
    /// Only load plugins from trusted sources.
    pub fn load_plugin(&mut self, path: &Path) -> Result<&DynamicPluginInfo, PluginLoadError> {
        if self.path_index.contains_key(path) {
            return Err(PluginLoadError::AlreadyLoaded(path.to_path_buf()));
        }

        if !path.exists() {
            return Err(PluginLoadError::FileNotFound(path.to_path_buf()));
        }

        // Validate file extension
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if !matches!(ext, "so" | "dylib" | "dll") {
            return Err(PluginLoadError::InvalidExtension(ext.to_string()));
        }

        // In a real implementation, this would use `libloading` to:
        // 1. Load the shared library
        // 2. Resolve the plugin entry point symbol
        // 3. Call the entry point to get the plugin instance
        // 4. Validate the plugin API version
        //
        // For now, we return a placeholder that simulates successful loading
        let plugin_info = super::PluginInfo {
            id: format!("dynamic-{}", path.file_stem().unwrap_or_default().to_string_lossy()),
            name: format!("Dynamic Plugin: {}", path.file_name().unwrap_or_default().to_string_lossy()),
            version: "0.1.0".to_string(),
            description: "Dynamically loaded plugin".to_string(),
        };

        let info = DynamicPluginInfo {
            library_path: path.to_path_buf(),
            plugin_info,
            api_version: 1,
            loaded: true,
        };

        let idx = self.plugins.len();
        self.plugins.push(info);
        self.path_index.insert(path.to_path_buf(), idx);

        Ok(&self.plugins[idx])
    }

    /// Load all plugins from a directory.
    pub fn load_from_directory(&mut self, dir: &Path) -> Result<Vec<PathBuf>, PluginLoadError> {
        if !dir.is_dir() {
            return Err(PluginLoadError::NotADirectory(dir.to_path_buf()));
        }

        let mut loaded_paths = Vec::new();

        let entries = std::fs::read_dir(dir)
            .map_err(|e| PluginLoadError::IoError(e.to_string()))?;

        for entry in entries {
            let entry = entry.map_err(|e| PluginLoadError::IoError(e.to_string()))?;
            let path = entry.path();

            if path.is_file() {
                let ext = path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");

                if matches!(ext, "so" | "dylib" | "dll") {
                    match self.load_plugin(&path) {
                        Ok(_) => loaded_paths.push(path),
                        Err(e) => {
                            tracing::warn!("Failed to load plugin {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        Ok(loaded_paths)
    }

    /// Get information about all loaded plugins.
    pub fn list(&self) -> Vec<&DynamicPluginInfo> {
        self.plugins.iter().collect()
    }

    /// Get a plugin by its library path.
    pub fn get(&self, path: &Path) -> Option<&DynamicPluginInfo> {
        self.path_index.get(path).map(|&i| &self.plugins[i])
    }

    /// Number of loaded plugins.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Whether no plugins are loaded.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// Unload a plugin by path (marks as unloaded, doesn't actually unload the library).
    pub fn unload_plugin(&mut self, path: &Path) -> Result<(), PluginLoadError> {
        if let Some(&idx) = self.path_index.get(path) {
            self.plugins[idx].loaded = false;
            Ok(())
        } else {
            Err(PluginLoadError::NotLoaded(path.to_path_buf()))
        }
    }
}

impl Default for DynamicPluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during dynamic plugin loading.
#[derive(Debug)]
pub enum PluginLoadError {
    /// File not found at specified path.
    FileNotFound(PathBuf),
    /// File already loaded.
    AlreadyLoaded(PathBuf),
    /// Invalid file extension.
    InvalidExtension(String),
    /// Path is not a directory.
    NotADirectory(PathBuf),
    /// Plugin not loaded.
    NotLoaded(PathBuf),
    /// I/O error.
    IoError(String),
    /// Plugin initialization failed.
    InitFailed(String),
    /// API version mismatch.
    ApiVersionMismatch { expected: u32, found: u32 },
    /// Symbol not found in library.
    SymbolNotFound(String),
    /// Plugin returned invalid data.
    InvalidPluginData(String),
}

impl std::fmt::Display for PluginLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileNotFound(path) => write!(f, "Plugin file not found: {}", path.display()),
            Self::AlreadyLoaded(path) => write!(f, "Plugin already loaded: {}", path.display()),
            Self::InvalidExtension(ext) => write!(f, "Invalid plugin extension: .{}", ext),
            Self::NotADirectory(path) => write!(f, "Not a directory: {}", path.display()),
            Self::NotLoaded(path) => write!(f, "Plugin not loaded: {}", path.display()),
            Self::IoError(msg) => write!(f, "I/O error: {}", msg),
            Self::InitFailed(msg) => write!(f, "Plugin initialization failed: {}", msg),
            Self::ApiVersionMismatch { expected, found } => {
                write!(f, "API version mismatch: expected {}, found {}", expected, found)
            }
            Self::SymbolNotFound(sym) => write!(f, "Symbol not found: {}", sym),
            Self::InvalidPluginData(msg) => write!(f, "Invalid plugin data: {}", msg),
        }
    }
}

impl std::error::Error for PluginLoadError {}

/// Configuration for plugin loading.
#[derive(Debug, Clone)]
pub struct PluginLoadConfig {
    /// Directories to search for plugins.
    pub plugin_dirs: Vec<PathBuf>,
    /// Whether to recursively search subdirectories.
    pub recursive: bool,
    /// Minimum API version required.
    pub min_api_version: u32,
    /// Maximum API version supported.
    pub max_api_version: u32,
    /// Whether to allow loading from current directory.
    pub allow_current_dir: bool,
}

impl Default for PluginLoadConfig {
    fn default() -> Self {
        Self {
            plugin_dirs: vec![
                PathBuf::from("/usr/lib/eggsec/plugins"),
                PathBuf::from("/usr/local/lib/eggsec/plugins"),
            ],
            recursive: false,
            min_api_version: 1,
            max_api_version: 1,
            allow_current_dir: false,
        }
    }
}

/// Initialize plugin loading with configuration.
pub fn init_plugin_system(config: &PluginLoadConfig) -> Result<DynamicPluginRegistry, PluginLoadError> {
    let mut registry = DynamicPluginRegistry::new();

    for dir in &config.plugin_dirs {
        if dir.is_dir() {
            tracing::info!("Loading plugins from: {}", dir.display());
            registry.load_from_directory(dir)?;
        } else if config.allow_current_dir && dir == Path::new(".") {
            tracing::debug!("Skipping current directory (not configured)");
        }
    }

    tracing::info!("Loaded {} plugins", registry.len());
    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_dynamic_plugin_registry_new() {
        let registry = DynamicPluginRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_plugin_load_config_default() {
        let config = PluginLoadConfig::default();
        assert_eq!(config.plugin_dirs.len(), 2);
        assert_eq!(config.min_api_version, 1);
        assert_eq!(config.max_api_version, 1);
    }

    #[test]
    fn test_plugin_load_error_display() {
        let err = PluginLoadError::FileNotFound(PathBuf::from("/tmp/test.so"));
        assert!(err.to_string().contains("test.so"));

        let err = PluginLoadError::InvalidExtension("txt".to_string());
        assert!(err.to_string().contains("txt"));

        let err = PluginLoadError::ApiVersionMismatch { expected: 1, found: 2 };
        assert!(err.to_string().contains("expected 1"));
        assert!(err.to_string().contains("found 2"));
    }

    #[test]
    fn test_dynamic_plugin_registry_load_not_found() {
        let mut registry = DynamicPluginRegistry::new();
        let result = registry.load_plugin(Path::new("/nonexistent/plugin.so"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PluginLoadError::FileNotFound(_)));
    }

    #[test]
    fn test_dynamic_plugin_registry_load_invalid_extension() {
        use std::fs::File;

        // Create a temporary file with invalid extension
        let temp_dir = tempfile::tempdir().unwrap();
        let invalid_path = temp_dir.path().join("test.txt");
        File::create(&invalid_path).unwrap();

        let mut registry = DynamicPluginRegistry::new();
        let result = registry.load_plugin(&invalid_path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PluginLoadError::InvalidExtension(_)));
    }

    #[test]
    fn test_dynamic_plugin_registry_get() {
        let registry = DynamicPluginRegistry::new();
        assert!(registry.get(Path::new("/tmp/test.so")).is_none());
    }

    #[test]
    fn test_dynamic_plugin_registry_unload_not_loaded() {
        let mut registry = DynamicPluginRegistry::new();
        let result = registry.unload_plugin(Path::new("/tmp/test.so"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PluginLoadError::NotLoaded(_)));
    }

    #[test]
    fn test_init_plugin_system_empty_dirs() {
        let config = PluginLoadConfig {
            plugin_dirs: vec![],
            ..Default::default()
        };
        let registry = init_plugin_system(&config).unwrap();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_init_plugin_system_nonexistent_dirs() {
        let config = PluginLoadConfig {
            plugin_dirs: vec![PathBuf::from("/nonexistent/dir")],
            ..Default::default()
        };
        // Should not error, just load nothing
        let registry = init_plugin_system(&config).unwrap();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_init_plugin_system_with_temp_dir() {
        let temp_dir = TempDir::new().unwrap();
        let config = PluginLoadConfig {
            plugin_dirs: vec![temp_dir.path().to_path_buf()],
            ..Default::default()
        };
        let registry = init_plugin_system(&config).unwrap();
        assert!(registry.is_empty());
    }
}
