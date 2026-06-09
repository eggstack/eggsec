# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Fixed broken import paths in test files (`recon_tests.rs`)
- Added missing feature flag guards for NSE integration tests
- Removed unused imports in stress module exports
- Fixed invalid `SynFlooder` import reference in TUI workers
- Added LICENSE files (MIT and Apache-2.0)
- Added `#![allow(dead_code)]` to stress metrics module

### Removed

- Python plugin runtime and all Python plugin support (`python-plugins` feature)
- Ruby plugin runtime and all Ruby plugin support (`ruby-plugins` feature)
- Metasploit RPC integration (`eggsec-ruby` crate)
- `eggsec-plugin` crate (Python plugin manager, AST scanner, security validation)
- `eggsec-ruby` crate (Ruby plugin bridge, loader, MSF client)
- `eggsec plugin list` and `eggsec plugin run` CLI commands
- TUI plugin tab for Python/Ruby plugin discovery
- Plugin-related configuration fields (`plugins_dir`)
- Plugin development documentation (`PLUGIN_DEVELOPMENT.md`, `PLUGINS.md`)

NSE support remains available as an optional Nmap NSE compatibility layer via the `nse`, `nse-sandbox`, and `nse-ssh2` features.

### Added

#### Configuration System
- Configuration file support (TOML/YAML) at `~/.config/eggsec/eggsec.toml`
- Environment variable support with `EGGSEC_` prefix
- Scope file support for target authorization (`scope.toml`)
- Multiple scan profiles (quick, deep, waf-test, full)
- Custom scan profile definitions

#### Output Formats
- SARIF output format for GitHub Code Scanning integration
- JUnit XML output for CI/CD integration
- HTML report generation
- CSV export format

#### Notifications
- Webhook notifications for scan events
- Slack webhook integration
- Discord webhook integration
- Configurable severity thresholds for notifications

#### Logging & Observability
- Structured logging with `tracing`
- JSON log format support
- Configurable log levels (trace, debug, info, warn, error)
- Request/response logging with timing

#### Security
- Scope-based target authorization
- CIDR-based allow/block lists
- Port exclusion rules
- Secret handling with `secrecy` crate
- Rate limiting with configurable limits

#### Plugin System
- Python plugin support (optional feature)
- Custom payload directories
- Plugin configuration schema

#### Infrastructure
- Dockerfile for containerized deployment
- docker-compose.yml with optional services
- Multi-stage Docker build for smaller images

### Changed

- Improved error handling across all modules
- Better error messages with context
- Removed `.unwrap()` and `.expect()` in favor of proper error propagation
- Enhanced TUI with better error display

### Fixed

- Various race conditions in concurrent operations
- Memory leaks in long-running scans
- Proper cleanup of resources on interruption

## [0.1.0] - 2024-01-15

### Added

- Load testing module with concurrent request support
- Port scanner with service detection
- Endpoint discovery scanner
- Service fingerprinting (20+ protocols)
- WAF detection (30+ WAFs)
- WAF bypass techniques
  - Header manipulation
  - HTTP smuggling
  - Evasion techniques (homoglyphs, zero-width, encoding)
- Security fuzzing
  - SQL injection payloads
  - XSS payloads
  - Path traversal
  - SSRF
  - Open redirect
  - ReDoS
  - Header expansion
  - Compression bombs
- Pipeline mode for chained assessments
- Session persistence and resume capability
- Interactive TUI
- JSON output
- Progress bars with indicatif

### Security

- Initial security controls for responsible testing
- TLS certificate verification (configurable)

[Unreleased]: https://github.com/eggstack/eggsec/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/eggstack/eggsec/releases/tag/v0.1.0
