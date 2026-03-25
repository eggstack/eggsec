use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

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

    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Scope file path contains invalid UTF-8: {:?}", path))?;
    Scope::from_file(path_str).map_err(|e| anyhow::anyhow!("Failed to load scope: {}", e))
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
        assert_eq!(DEFAULT_CONFIG_NAME, "slapper.toml");
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
        assert!(!config.http.verify_tls || config.http.verify_tls);
    }

    #[test]
    fn test_load_config_valid_toml() {
        let dir = std::env::temp_dir().join("slapper_test_valid");
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
        let dir = std::env::temp_dir().join("slapper_test_invalid");
        let _ = std::fs::create_dir_all(&dir);
        let config_path = dir.join("invalid.toml");

        std::fs::write(&config_path, "[http\ninvalid toml").unwrap();

        let result = load_config(Some(config_path.to_str().unwrap()));
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_config_partial() {
        let dir = std::env::temp_dir().join("slapper_test_partial");
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
        let dir = std::env::temp_dir().join("slapper_test_yaml");
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
        let dir = std::env::temp_dir().join("slapper_test_empty");
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
        let temp_dir = std::env::temp_dir().join("slapper_test_no_config");
        let _ = std::fs::create_dir_all(&temp_dir);
        let _ = std::env::set_current_dir(&temp_dir);

        let result = find_config_file();
        let _ = std::env::set_current_dir(original_dir);
        let _ = std::fs::remove_dir_all(&temp_dir);

        assert!(result.is_none());
    }

    #[test]
    fn test_find_scope_file_returns_none_when_no_files() {
        let original_dir = std::env::current_dir().unwrap();
        let temp_dir = std::env::temp_dir().join("slapper_test_no_scope");
        let _ = std::fs::create_dir_all(&temp_dir);
        let _ = std::env::set_current_dir(&temp_dir);

        let result = find_scope_file();
        let _ = std::env::set_current_dir(original_dir);
        let _ = std::fs::remove_dir_all(&temp_dir);

        assert!(result.is_none());
    }

    #[test]
    fn test_slapper_config_default() {
        let config = SlapperConfig::default();
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
