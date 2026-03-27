# Plugin Development Overview

Slapper supports plugins in three languages: Python, Ruby, and Lua (NSE scripts). This document provides a unified overview. See language-specific guides for details.

## Plugin Types

| Language | File Extension | Interface | Runtime |
|----------|---------------|-----------|---------|
| Python | `.py` | Class-based (`PLUGINS = [MyPlugin]`) or function-based | PyO3 |
| Ruby | `.rb` | `Slapper::Plugin` class | Magnus |
| Lua/NSE | `.nse` | Port/Host rules + action function | mlua |

## Plugin Discovery

Slapper searches for plugins in these directories (in order):

1. `./plugins/` (project directory)
2. `~/.config/slapper/plugins/` (user config)
3. `~/.slapper/plugins/` (legacy location)

## Unified Plugin Trait

All plugin backends implement the `Plugin` trait from `slapper-plugin`:

```rust
#[async_trait]
pub trait Plugin: Send + Sync {
    fn info(&self) -> &PluginInfo;
    fn language(&self) -> PluginLanguage;
    fn list_checks(&self) -> Vec<PluginCheck>;
    async fn run_check(&self, check_name: &str, target: &str) -> Result<PluginResult>;
    async fn run(&self, target: &str, config: &PluginConfig) -> Result<PluginResult>;
}
```

## Plugin Registry

The `PluginRegistry` manages all loaded plugins:

```rust
use slapper_plugin::PluginRegistry;

let mut registry = PluginRegistry::new();
// Register plugins from each backend
registry.register(python_plugin);
registry.register(ruby_plugin);

// Run all checks
let results = registry.run_check("vuln_scan", "https://example.com").await?;
```

## Language-Specific Guides

- [Python Plugins](PLUGINS.md#python-plugins) - Class-based and function-based interfaces
- [Ruby Plugins](PLUGINS.md#ruby-plugins) - Slapper::Plugin class with Metasploit integration
- [NSE Scripts](NSE_SCRIPTS.md) - Lua scripts compatible with Nmap Scripting Engine

## Configuration

Add plugin configuration to `slapper.toml`:

```toml
[plugins.my_python_plugin]
enabled = true
timeout = 30

[plugins.my_ruby_plugin]
enabled = true
```

## Output Format

All plugins return findings in a consistent format:

```json
{
    "title": "Finding Title",
    "severity": "critical|high|medium|low|info",
    "description": "Detailed description",
    "location": "https://example.com/path",
    "evidence": "Optional evidence string",
    "cve_ids": ["CVE-2024-1234"]
}
```
