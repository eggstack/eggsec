#![cfg(feature = "python-plugins")]

use slapper::plugin::PythonPluginManager;
use std::path::Path;

#[test]
fn test_python_plugin_manager_creation() {
    let manager = PythonPluginManager::new();
    let checks = manager.get_checks();
    assert!(checks.is_empty());
}

#[test]
fn test_discover_plugins_empty_dir() {
    let mut manager = PythonPluginManager::new();
    let dir = Path::new("/tmp/nonexistent_slapper_test_dir");
    let result = manager.load_plugins(dir);
    assert!(result.is_ok());
}

#[test]
fn test_load_python_plugin_metadata() {
    use slapper::plugin::PluginManager;

    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let test_plugin = fixtures_dir.join("test_plugin.py");
    if test_plugin.exists() {
        let mut manager = PluginManager::new();
        manager.add_plugin_dir(fixtures_dir);
        let plugins = manager.discover_plugins();
        assert!(!plugins.is_empty());
        let plugin = plugins.iter().find(|p| p.name == "test_plugin");
        assert!(plugin.is_some());
        let plugin = plugin.unwrap();
        assert_eq!(plugin.version, "0.1.0");
        assert_eq!(plugin.author, "Test");
    }
}

#[test]
fn test_get_checks_from_plugin() {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let test_plugin = fixtures_dir.join("test_plugin.py");
    if test_plugin.exists() {
        let mut manager = PythonPluginManager::new();
        let result = manager.load_plugins(&fixtures_dir);
        assert!(result.is_ok());
        let checks = manager.get_checks();
        assert!(!checks.is_empty());
        assert_eq!(checks[0].name, "test_check");
    }
}

#[test]
fn test_run_check_from_plugin() {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let test_plugin = fixtures_dir.join("test_plugin.py");
    if test_plugin.exists() {
        let mut manager = PythonPluginManager::new();
        let _ = manager.load_plugins(&fixtures_dir);
        let results = manager.run_check("test_check", "http://example.com");
        assert!(results.is_ok());
        let results = results.unwrap();
        assert!(!results.is_empty());
    }
}

#[test]
fn test_default_plugin_dirs() {
    use slapper::plugin::PluginManager;

    let dirs = PluginManager::default_plugin_dirs(None);
    assert!(!dirs.is_empty());
    assert!(dirs.iter().any(|d| d.ends_with("plugins")));

    let custom = std::path::PathBuf::from("/custom/plugins");
    let dirs = PluginManager::default_plugin_dirs(Some(custom.clone()));
    assert_eq!(dirs[0], custom);
}
