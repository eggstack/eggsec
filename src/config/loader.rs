#![allow(dead_code)]

use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::{Path, PathBuf};

use super::scope::Scope;
use super::settings::SlapperConfig;

pub const DEFAULT_CONFIG_NAME: &str = "slapper.toml";
pub const SCOPE_FILE_NAME: &str = "scope.toml";
const PROJECT_NAME: &str = "slapper";
const PROJECT_QUALIFIER: &str = "tools";

pub fn load_config(config_path: Option<&str>) -> Result<SlapperConfig> {
    let path = config_path
        .map(PathBuf::from)
        .or_else(find_config_file)
        .unwrap_or_else(default_config_path);

    if !path.exists() {
        tracing::debug!("No config file found at {:?}, using defaults", path);
        return Ok(SlapperConfig::default());
    }

    tracing::info!("Loading configuration from {:?}", path);

    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config file: {:?}", path))?;

    let config: SlapperConfig = if path
        .extension()
        .map(|e| e == "yaml" || e == "yml")
        .unwrap_or(false)
    {
        serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML config: {:?}", path))?
    } else {
        toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML config: {:?}", path))?
    };

    Ok(config)
}

pub fn load_scope(scope_path: Option<&str>) -> Result<Scope> {
    let path = scope_path
        .map(PathBuf::from)
        .or_else(find_scope_file)
        .unwrap_or_else(default_scope_path);

    if !path.exists() {
        tracing::debug!("No scope file found at {:?}, allowing all targets", path);
        return Ok(Scope::default());
    }

    tracing::info!("Loading scope from {:?}", path);

    Scope::from_file(path.to_str().unwrap())
        .map_err(|e| anyhow::anyhow!("Failed to load scope: {}", e))
}

pub fn find_config_file() -> Option<PathBuf> {
    let candidates = vec![
        PathBuf::from(DEFAULT_CONFIG_NAME),
        PathBuf::from(".slapper").join(DEFAULT_CONFIG_NAME),
        PathBuf::from("config").join(DEFAULT_CONFIG_NAME),
    ];

    for candidate in candidates {
        if candidate.exists() {
            return Some(candidate);
        }
    }

    if let Some(config_dir) = config_dir() {
        let config_file = config_dir.join(DEFAULT_CONFIG_NAME);
        if config_file.exists() {
            return Some(config_file);
        }
    }

    None
}

pub fn find_scope_file() -> Option<PathBuf> {
    let candidates = vec![
        PathBuf::from(SCOPE_FILE_NAME),
        PathBuf::from(".slapper").join(SCOPE_FILE_NAME),
    ];

    candidates.into_iter().find(|candidate| candidate.exists())
}

pub fn default_config_path() -> PathBuf {
    config_dir()
        .map(|d| d.join(DEFAULT_CONFIG_NAME))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_NAME))
}

pub fn default_scope_path() -> PathBuf {
    config_dir()
        .map(|d| d.join(SCOPE_FILE_NAME))
        .unwrap_or_else(|| PathBuf::from(SCOPE_FILE_NAME))
}

pub fn config_dir() -> Option<PathBuf> {
    ProjectDirs::from(PROJECT_QUALIFIER, "", PROJECT_NAME)
        .map(|dirs| dirs.config_dir().to_path_buf())
}

pub fn data_dir() -> Option<PathBuf> {
    ProjectDirs::from(PROJECT_QUALIFIER, "", PROJECT_NAME).map(|dirs| dirs.data_dir().to_path_buf())
}

pub fn cache_dir() -> Option<PathBuf> {
    ProjectDirs::from(PROJECT_QUALIFIER, "", PROJECT_NAME)
        .map(|dirs| dirs.cache_dir().to_path_buf())
}

pub fn ensure_directories() -> Result<()> {
    if let Some(dir) = config_dir() {
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create config directory: {:?}", dir))?;
    }

    if let Some(dir) = data_dir() {
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create data directory: {:?}", dir))?;
    }

    if let Some(dir) = cache_dir() {
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create cache directory: {:?}", dir))?;
    }

    Ok(())
}

pub fn write_example_config(path: &Path) -> Result<()> {
    let config = SlapperConfig::default();
    let content = toml::to_string_pretty(&config).context("Failed to serialize example config")?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {:?}", parent))?;
    }

    fs::write(path, content).with_context(|| format!("Failed to write config file: {:?}", path))?;

    Ok(())
}
