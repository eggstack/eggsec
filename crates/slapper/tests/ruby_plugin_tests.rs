#![cfg(feature = "ruby-plugins")]

use slapper::ruby::{PluginLoader, RubyPluginClient};
use std::path::PathBuf;

#[test]
fn test_ruby_client_creation() {
    let client = RubyPluginClient::new();
    assert!(client.is_ok());
}

#[test]
fn test_plugin_loader_creation() {
    let loader = PluginLoader::new(vec![]);
    assert!(loader.is_ok());
}

#[test]
fn test_discover_plugins_empty_dir() {
    let mut loader = PluginLoader::new(vec![PathBuf::from("/tmp/nonexistent_slapper_ruby_test")]);
    assert!(loader.is_ok());
    let mut loader = loader.unwrap();
    let discovered = loader.discover_plugins();
    assert!(discovered.is_ok());
    assert!(discovered.unwrap().is_empty());
}

#[test]
fn test_list_plugins_empty() {
    let loader = PluginLoader::new(vec![]).unwrap();
    assert!(loader.list_plugins().is_empty());
}
