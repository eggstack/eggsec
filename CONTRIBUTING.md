# Contributing to Slapper

Thank you for your interest in contributing to Slapper! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Documentation](#documentation)
- [TUI Development](#tui-development)
- [Pull Request Process](#pull-request-process)

## Code of Conduct

This project adheres to a Code of Conduct. By participating, you are expected to uphold this code. Please report unacceptable behavior to conduct@slapper-tool.org.

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git
- A GitHub account

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/slapper.git
   cd slapper
   ```
3. Add the upstream remote:
   ```bash
   git remote add upstream https://github.com/slapper-tool/slapper.git
   ```

## Development Setup

### Build

```bash
# Debug build
cargo build

# Release build
cargo build --release

# With all features
cargo build --all-features
```

### Run Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with all features
cargo test --all-features

# Run with verbose output
cargo test -- --nocapture
```

### Linting

```bash
# Run clippy
cargo clippy --all-features -- -D warnings

# Format check
cargo fmt --check

# Auto-format
cargo fmt
```

### Security Audit

```bash
# Install cargo-audit
cargo install cargo-audit

# Run audit
cargo audit
```

### Feature Flags

Slapper uses Cargo feature flags to enable optional capabilities. This allows building with only the features you need.

#### Available Features

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `tool-api` | Base abstraction layer | None |
| `rest-api` | REST API server with MCP | axum, tower, async-stream |
| `grpc-api` | gRPC API server | tonic, prost |
| `stress-testing` | Network stress testing | pnet, socket2, nix, surge-ping |
| `packet-inspection` | Live packet capture | pnet, libc |
| `nse` | Nmap Scripting Engine | mlua, openssl, ssh2 |
| `full` | All features combined | All of the above |

#### Feature Propagation

Features are propagated between workspace crates:

```
slapper (parent)
├── nse → slapper-nse/nse
└── stress-testing → slapper-nse?/stress-testing (if nse enabled)
```

The `?` syntax means "if the dependency is enabled, also enable this feature on it".

#### Testing Feature Combinations

```bash
# Test default build
cargo build -p slapper

# Test specific feature
cargo build -p slapper --features stress-testing

# Test all features
cargo build -p slapper --features full

# CI tests all feature combinations via matrix strategy
```

#### CI Matrix

The CI runs checks and tests across multiple feature combinations:

- **Default**: Workspace check with no optional features
- **Minimal CLI**: Core CLI without API servers
- **rest-api**: REST API + MCP server
- **grpc-api**: gRPC API server
- **packet-inspection**: Packet capture and traceroute
- **stress-testing**: Raw sockets and stress testing
- **nse**: Nmap NSE script support
- **nse-sandbox**: NSE with sandbox mode
- **api-schema**: OpenAPI schema support
- **sbom**: SBOM generation
- **full**: All features combined

This catches undeclared or miswired features early.

#### Adding a New Feature

1. Add the feature to `crates/slapper/Cargo.toml`:
   ```toml
   [features]
   my-feature = ["dep:my-dependency"]
   ```

2. Gate code with `#[cfg(feature = "my-feature")]`:
   ```rust
   #[cfg(feature = "my-feature")]
   mod my_module;
   ```

3. Add to CI matrix in `.github/workflows/test.yml`

4. Update the `full` feature if it should be included

## Making Changes

### Branch Naming

Use descriptive branch names:

- `feature/add-graphql-support` - New features
- `fix/rate-limiter-race-condition` - Bug fixes
- `docs/improve-plugin-guide` - Documentation
- `refactor/simplify-error-handling` - Code refactoring

### Code Style

1. **Follow Rust conventions**
   - Use `cargo fmt` for formatting
   - Address all `clippy` warnings
   - Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

2. **Documentation**
   - All public items must have doc comments
   - Include examples in doc comments where helpful
   - Update CHANGELOG.md for notable changes

3. **Error Handling**
   - Use `thiserror` for library errors
   - Use `anyhow` for application errors
   - Provide context with errors

4. **Logging**
   - Use `tracing` macros for logging
   - Use appropriate log levels
   - Include relevant context

### Example: Adding a New Feature

```rust
/// Brief description of the function.
///
/// More detailed explanation if needed.
///
/// # Arguments
///
/// * `param` - Description of parameter
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// Description of when errors are returned
///
/// # Example
///
/// ```
/// use slapper::module::function;
/// let result = function("example");
/// ```
pub fn new_function(param: &str) -> Result<Output, Error> {
    tracing::debug!("Processing: {}", param);
    
    // Implementation
    Ok(Output::default())
}
```

## Testing

### Unit Tests

Place unit tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_new_function() {
        let result = new_function("test").unwrap();
        assert!(result.is_valid());
    }
    
    #[test]
    fn test_error_case() {
        let result = new_function("");
        assert!(result.is_err());
    }
}
```

### Integration Tests

Place integration tests in the `tests/` directory:

```rust
// tests/integration_test.rs
use slapper::*;

#[tokio::test]
async fn test_full_workflow() {
    // Test with mock server
}
```

### Property-Based Testing

Use `proptest` for property-based testing:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_parse_url_doesnt_crash(url in ".*") {
        let _ = parse_url(&url);
    }
}
```

## Documentation

### Code Documentation

- Use `///` for doc comments
- Include examples, panics, errors, and safety notes
- Keep line length under 100 characters

### User Documentation

Update documentation in:

- `README.md` - Overview and quick start
- `docs/` - Detailed guides
- Example files in `examples/`

## TUI Development

### Overview

The TUI is built with [ratatui](https://github.com/ratatui-org/ratatui) and uses crossterm for terminal handling. All TUI code is in `src/tui/`.

### Directory Structure

```
src/tui/
├── mod.rs           # Entry point, App struct, event loop
├── ui.rs            # Layout rendering, tabs, status bar
├── components/      # Reusable UI widgets
│   ├── input.rs     # InputField, InputGroup
│   ├── popup.rs     # Popup, help_popup_for_tab
│   ├── progress.rs  # ProgressGauge, StatusBar
│   ├── scrollable.rs # ScrollableText, ScrollableTable
│   └── selector.rs  # Selector, Checkbox, RadioGroup
├── tabs/            # Tab implementations
│   ├── mod.rs       # Tab enum, traits (TabState, TabRender, TabInput)
│   ├── load.rs      # Example: Load testing tab
│   └── ...
├── state/           # State management
│   └── history.rs   # History management
└── workers/         # Async task execution
    └── runner.rs    # TaskRunner, TaskConfig, TaskResult
```

### Input Modes

The TUI uses VIM-style input modes:

- **Normal Mode [NOR]**: Navigation with `h/j/k/l`, tab switching with numbers, `?` for help
- **Insert Mode [INS]**: Press `i` to enter, type to input text, `Esc` to return to Normal mode

When adding new features, ensure they respect the current input mode:

```rust
// Example: Only handle character input in Insert mode
(KeyModifiers::NONE, KeyCode::Char(c)) if app.mode == InputMode::Insert => {
    app.handle_char(c);
}
```

### Adding a New Tab

1. **Create the tab struct** in `src/tui/tabs/new_feature.rs`:

```rust
use crate::tui::components::{InputField, InputGroup, ProgressGauge, ScrollableText};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    Frame,
};

pub struct NewFeatureTab {
    pub inputs: InputGroup,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub results_view: ScrollableText,
}

impl NewFeatureTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("Target"))
            .add(InputField::new("Option").with_value("default"));

        Self {
            inputs,
            progress: ProgressGauge::new("Processing..."),
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
        }
    }

    pub fn target(&self) -> &str {
        self.inputs.fields.get(0).map(|f| f.value.as_str()).unwrap_or("")
    }

    pub fn start(&mut self) {
        if !self.target().is_empty() {
            self.state = AppState::Running;
            self.results_view.clear();
        }
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

    pub fn update_progress(&mut self, completed: u64, total: u64) {
        self.progress.current = completed;
        self.progress.total = total;
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.results_view.page_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.results_view.page_down(page_size);
    }
}

impl TabState for NewFeatureTab {
    fn state(&self) -> AppState { self.state.clone() }
    fn progress(&self) -> f64 { self.progress.percent() as f64 }
    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.progress.current = 0;
        self.results_view.clear();
        for field in &mut self.inputs.fields {
            field.clear();
        }
    }
}

impl TabRender for NewFeatureTab {
    fn render(&self, f: &mut Frame, area: Rect) {
        // Layout and render widgets
    }
}

impl TabInput for NewFeatureTab {
    fn handle_focus_next(&mut self) { self.inputs.focus_next(); }
    fn handle_focus_prev(&mut self) { self.inputs.focus_prev(); }
    fn handle_char(&mut self, c: char) {
        if !self.is_running() { self.inputs.insert(c); }
    }
    fn handle_backspace(&mut self) {
        if !self.is_running() { self.inputs.backspace(); }
    }
    fn handle_enter(&mut self) {
        if self.inputs.is_focused() {
            self.inputs.blur();
        } else if self.is_running() {
            self.stop();
        } else {
            self.start();
        }
    }
    fn handle_escape(&mut self) { self.inputs.blur(); }
    fn handle_up(&mut self) { self.inputs.focus_prev(); }
    fn handle_down(&mut self) { self.inputs.focus_next(); }
    fn handle_left(&mut self) { self.inputs.move_left(); }
    fn handle_right(&mut self) { self.inputs.move_right(); }
    fn is_input_focused(&self) -> bool { self.inputs.is_focused() }
}
```

2. **Register the tab** in `src/tui/tabs/mod.rs`:

```rust
mod new_feature;
pub use new_feature::NewFeatureTab;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    // ... existing tabs
    NewFeature = 12,  // Next available index
}

impl Tab {
    pub fn title(&self) -> &'static str {
        match self {
            Tab::NewFeature => "[13] New Feature",
            // ...
        }
    }
}
```

3. **Add to App struct** in `src/tui/mod.rs`:

```rust
pub struct App {
    // ... existing fields
    pub new_feature: tabs::NewFeatureTab,
}
```

4. **Add TaskConfig variant** in `src/tui/workers/runner.rs`:

```rust
pub enum TaskConfig {
    // ... existing variants
    NewFeature {
        target: String,
        // ... params
    },
}

pub enum TaskResult {
    // ... existing variants
    NewFeature(NewFeatureResults),
}
```

5. **Implement the async runner** and handle results in `handle_result()`.

### Reusable Components

| Component | Purpose | Key Methods |
|-----------|---------|-------------|
| `InputField` | Single text input | `insert()`, `backspace()`, `render()` |
| `InputGroup` | Collection of inputs | `focus_next()`, `focus_prev()`, `blur()` |
| `ProgressGauge` | Progress bar | `update()`, `percent()`, `render()` |
| `ScrollableText` | Scrollable content | `add_line()`, `page_up()`, `page_down()` |
| `ScrollableTable` | Table with selection | `add_row()`, `scroll_up()`, `scroll_down()` |
| `Selector` | Dropdown selector | `toggle()`, `next()`, `prev()` |
| `Checkbox` | Boolean toggle | `toggle()`, `render()` |
| `RadioGroup` | Single selection | `select()`, `selected_option()` |

### Color Conventions

```rust
Color::Yellow   // Focused/active elements
Color::Cyan     // Info/normal state
Color::Green    // Success/completed
Color::Red      // Error/critical
Color::Gray     // Inactive/disabled
Color::DarkGray // Placeholder/muted
```

### Key Bindings

| Key | Normal Mode | Insert Mode |
|-----|-------------|-------------|
| `i` | Enter Insert mode | - |
| `Esc` | - | Return to Normal mode |
| `h/j/k/l` | Navigate left/down/up/right | - |
| `Ctrl+u/d` | Page up/down | Page up/down |
| `1-9,0` | Switch tabs | Type numbers |
| `Tab` | Next tab | - |
| `?` | Toggle help | - |
| `q` | Quit | - |
| `r` | Reset current tab | - |
| `Enter` | Start/stop action | - |

### Async Task Execution

Tasks that require network I/O or long-running operations should use the `TaskRunner`:

```rust
// In App::handle_enter()
if self.new_feature.is_running() {
    self.stop();
} else {
    self.new_feature.start();
    self.spawn_task(self.build_new_feature_task());
}

// Build the task config
fn build_new_feature_task(&self) -> Option<workers::TaskConfig> {
    let target = self.new_feature.target();
    if target.is_empty() { return None; }
    
    Some(workers::TaskConfig::NewFeature {
        target: target.to_string(),
        // ... other params
    })
}
```

### Testing TUI Changes

1. Run the TUI: `cargo run -- tui`
2. Test all key bindings in both Normal and Insert modes
3. Verify tab switching works correctly
4. Ensure long outputs scroll properly with `Ctrl+u/d`
5. Check that async operations can be started and stopped

## Pull Request Process

### Before Submitting

1. **Update from upstream**
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Run all checks**
   ```bash
   cargo fmt --check
   cargo clippy --all-features -- -D warnings
   cargo test --all-features
   cargo audit
   ```

3. **Update documentation**
   - Update doc comments
   - Update README if needed
   - Update CHANGELOG.md

### PR Checklist

- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] Commit messages are clear

### Commit Messages

Follow conventional commits:

```
type(scope): brief description

Longer explanation if needed.

Fixes #issue-number
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting, semicolons, etc.
- `refactor`: Code restructuring
- `test`: Adding tests
- `chore`: Maintenance tasks

### Review Process

1. PRs require at least one approval
2. CI must pass
3. Address all review comments
4. Squash commits before merge (if requested)

## Release Process

Releases are handled by maintainers:

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create git tag
4. Build release binaries
5. Publish to crates.io
6. Create GitHub release

## Getting Help

- **GitHub Discussions**: For questions and discussions
- **GitHub Issues**: For bug reports and feature requests
- **Discord**: Join our community server

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT OR Apache-2.0).

---

Thank you for contributing to Slapper!
