use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::{Path, PathBuf};

use super::scope::Scope;
use super::settings::EggsecConfig;
use crate::constants::{PROJECT_NAME, PROJECT_QUALIFIER};
use crate::types::check_config_file_permissions;

pub const DEFAULT_CONFIG_NAME: &str = "eggsec.toml";
pub const SCOPE_FILE_NAME: &str = "scope.toml";

pub fn load_config(config_path: Option<&str>) -> Result<EggsecConfig> {
    let path = config_path
        .map(PathBuf::from)
        .or_else(|| find_config_file(None))
        .unwrap_or_else(default_config_path);

    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!("No config file found at {:?}, using defaults", path);
            return Ok(EggsecConfig::default());
        }
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Failed to canonicalize config path '{}': {}",
                path.display(),
                e
            ));
        }
    };

    tracing::info!("Loading configuration from {:?}", canonical_path);

    let content = fs::read_to_string(&canonical_path)
        .with_context(|| format!("Failed to read config file: {:?}", canonical_path))?;

    let config: EggsecConfig = if canonical_path
        .extension()
        .map(|e| e == "yaml" || e == "yml")
        .unwrap_or(false)
    {
        serde_yaml_neo::from_str(&content)
            .with_context(|| format!("Failed to parse YAML config: {:?}", canonical_path))?
    } else {
        toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML config: {:?}", canonical_path))?
    };

    config.validate().map_err(|e| anyhow::anyhow!("{}", e))?;
    check_config_file_permissions(&canonical_path);

    Ok(config)
}

pub fn load_scope(scope_path: Option<&str>) -> Result<Scope> {
    let path = scope_path
        .map(PathBuf::from)
        .or_else(|| find_scope_file(None))
        .unwrap_or_else(default_scope_path);

    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!("No scope file found at {:?}, allowing all targets", path);
            return Ok(Scope::default());
        }
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Failed to canonicalize scope path '{}': {}",
                path.display(),
                e
            ));
        }
    };

    tracing::info!("Loading scope from {:?}", canonical_path);

    let path_str = canonical_path.to_str().ok_or_else(|| {
        anyhow::anyhow!(
            "Scope file path contains invalid UTF-8: {:?}",
            canonical_path
        )
    })?;
    let scope =
        Scope::from_file(path_str).map_err(|e| anyhow::anyhow!("Failed to load scope: {}", e))?;
    scope
        .validate()
        .map_err(|e| anyhow::anyhow!("Scope validation failed: {}", e))?;
    check_config_file_permissions(&canonical_path);
    Ok(scope)
}

pub fn find_config_file(base_dir: Option<&Path>) -> Option<PathBuf> {
    let base: &Path = base_dir.unwrap_or_else(|| Path::new("."));
    let candidates: Vec<PathBuf> = vec![
        base.join(DEFAULT_CONFIG_NAME),
        base.join(".eggsec").join(DEFAULT_CONFIG_NAME),
        base.join("config").join(DEFAULT_CONFIG_NAME),
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

pub fn find_scope_file(base_dir: Option<&Path>) -> Option<PathBuf> {
    let base: &Path = base_dir.unwrap_or_else(|| Path::new("."));
    let candidates: Vec<PathBuf> = vec![
        base.join(SCOPE_FILE_NAME),
        base.join(".eggsec").join(SCOPE_FILE_NAME),
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

#[allow(dead_code)]
pub fn data_dir() -> Option<PathBuf> {
    ProjectDirs::from(PROJECT_QUALIFIER, "", PROJECT_NAME).map(|dirs| dirs.data_dir().to_path_buf())
}

#[allow(dead_code)]
pub fn cache_dir() -> Option<PathBuf> {
    ProjectDirs::from(PROJECT_QUALIFIER, "", PROJECT_NAME)
        .map(|dirs| dirs.cache_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_name() {
        assert_eq!(DEFAULT_CONFIG_NAME, "eggsec.toml");
    }

    #[test]
    fn test_scope_file_name() {
        assert_eq!(SCOPE_FILE_NAME, "scope.toml");
    }

    #[test]
    fn test_config_dir_returns_some() {
        let dir = config_dir();
        assert!(dir.is_some());
    }

    #[test]
    fn test_default_config_path_ends_with_name() {
        let path = default_config_path();
        assert!(path.to_string_lossy().ends_with(DEFAULT_CONFIG_NAME));
    }

    #[test]
    fn test_default_scope_path_ends_with_name() {
        let path = default_scope_path();
        assert!(path.to_string_lossy().ends_with(SCOPE_FILE_NAME));
    }

    #[test]
    fn test_data_dir_returns_some() {
        let dir = data_dir();
        assert!(dir.is_some());
    }

    #[test]
    fn test_cache_dir_returns_some() {
        let dir = cache_dir();
        assert!(dir.is_some());
    }

    #[test]
    fn test_load_config_default_when_no_file() {
        let config = load_config(Some("/nonexistent/path/config.toml"));
        assert!(config.is_ok());
    }

    #[test]
    fn test_load_config_default_values() {
        let config = load_config(Some("/nonexistent/path/config.toml")).unwrap();
        assert_eq!(config.http.timeout_secs, 30);
        assert_eq!(config.scan.default_concurrency, 10);
        assert_eq!(config.scan.port_timeout_secs, 2);
        assert!(config.http.verify_tls);
    }

    #[test]
    fn test_load_config_valid_toml() {
        let dir = std::env::temp_dir().join("eggsec_test_valid");
        let _ = std::fs::create_dir_all(&dir);
        let config_path = dir.join("valid.toml");

        let toml_content = r#"
[http]
timeout_secs = 15
max_retries = 5
verify_tls = false

[scan]
default_concurrency = 50
port_timeout_secs = 5
stealth_mode = true
"#;
        std::fs::write(&config_path, toml_content).unwrap();

        let config = load_config(Some(config_path.to_str().unwrap())).unwrap();
        assert_eq!(config.http.timeout_secs, 15);
        assert_eq!(config.http.max_retries, 5);
        assert!(!config.http.verify_tls);
        assert_eq!(config.scan.default_concurrency, 50);
        assert_eq!(config.scan.port_timeout_secs, 5);
        assert!(config.scan.stealth_mode);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_config_invalid_toml() {
        let dir = std::env::temp_dir().join("eggsec_test_invalid");
        let _ = std::fs::create_dir_all(&dir);
        let config_path = dir.join("invalid.toml");

        std::fs::write(&config_path, "[http\ninvalid toml").unwrap();

        let result = load_config(Some(config_path.to_str().unwrap()));
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_config_partial() {
        let dir = std::env::temp_dir().join("eggsec_test_partial");
        let _ = std::fs::create_dir_all(&dir);
        let config_path = dir.join("partial.toml");

        let toml_content = r#"
[http]
timeout_secs = 60
"#;
        std::fs::write(&config_path, toml_content).unwrap();

        let config = load_config(Some(config_path.to_str().unwrap())).unwrap();
        assert_eq!(config.http.timeout_secs, 60);
        assert_eq!(config.scan.default_concurrency, 10);
        assert_eq!(config.scan.port_timeout_secs, 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_config_yaml_format() {
        let dir = std::env::temp_dir().join("eggsec_test_yaml");
        let _ = std::fs::create_dir_all(&dir);
        let config_path = dir.join("config.yaml");

        let yaml_content = r#"
http:
  timeout_secs: 25
  max_retries: 2
scan:
  default_concurrency: 15
"#;
        std::fs::write(&config_path, yaml_content).unwrap();

        let config = load_config(Some(config_path.to_str().unwrap())).unwrap();
        assert_eq!(config.http.timeout_secs, 25);
        assert_eq!(config.http.max_retries, 2);
        assert_eq!(config.scan.default_concurrency, 15);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_config_empty_toml() {
        let dir = std::env::temp_dir().join("eggsec_test_empty");
        let _ = std::fs::create_dir_all(&dir);
        let config_path = dir.join("empty.toml");

        std::fs::write(&config_path, "").unwrap();

        let config = load_config(Some(config_path.to_str().unwrap())).unwrap();
        assert_eq!(config.http.timeout_secs, 30);
        assert_eq!(config.scan.default_concurrency, 10);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_scope_nonexistent() {
        let scope = load_scope(Some("/nonexistent/scope.toml"));
        assert!(scope.is_ok());
        let scope = scope.unwrap();
        assert!(scope.allowed_targets.is_empty());
        assert!(!scope.require_explicit_scope);
    }

    #[test]
    fn test_find_config_file_returns_none_when_no_files() {
        let original_dir = std::env::current_dir().unwrap();
        let temp_dir = std::env::temp_dir().join("eggsec_test_no_config");
        let _ = std::fs::create_dir_all(&temp_dir);
        let _ = std::env::set_current_dir(&temp_dir);

        let result = find_config_file(None);
        let _ = std::env::set_current_dir(original_dir);
        let _ = std::fs::remove_dir_all(&temp_dir);

        assert!(result.is_none());
    }

    #[test]
    fn test_find_scope_file_returns_none_when_no_files() {
        let original_dir = std::env::current_dir().unwrap();
        let temp_dir = std::env::temp_dir().join("eggsec_test_no_scope");
        let _ = std::fs::create_dir_all(&temp_dir);
        let _ = std::env::set_current_dir(&temp_dir);

        let result = find_scope_file(None);
        let _ = std::env::set_current_dir(original_dir);
        let _ = std::fs::remove_dir_all(&temp_dir);

        assert!(result.is_none());
    }

    #[test]
    fn test_eggsec_config_default() {
        let config = EggsecConfig::default();
        assert_eq!(config.http.timeout_secs, 30);
        assert_eq!(config.http.max_retries, 3);
        assert_eq!(config.http.retry_delay_ms, 1000);
        assert!(config.http.verify_tls);
        assert!(config.http.follow_redirects);
        assert_eq!(config.http.max_redirects, 10);
        assert_eq!(config.scan.default_concurrency, 10);
        assert!(!config.scan.stealth_mode);
        assert_eq!(config.scan.port_timeout_secs, 2);
        assert!(!config.scan.save_session);
        assert!(config.output.color);
        assert!(config.output.progress_bars);
        assert!(!config.output.save_results);
    }
}
