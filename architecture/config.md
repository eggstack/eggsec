# Configuration System

The configuration system handles loading settings from files, environment variables, and defaults, while also enforcing scanning scopes to prevent accidental testing of out-of-scope targets.

## Core Components (`src/config/`)

### `SlapperConfig` (`mod.rs`, `settings.rs`)

The main configuration struct that holds all tool settings. It is typically loaded from `slapper.toml` or `slapper.yaml`.

- **HTTP Settings**: Timeouts, user agents, proxy settings, retry logic.
- **Scanner Settings**: Thread counts, timing templates, default ports.
- **Fuzzer Settings**: Payload limits, recursion depth, detection thresholds.
- **Output Settings**: Default report formats and paths.

### `Scope` (`scope.rs`)

The `Scope` struct is critical for security and compliance. It defines which targets are "in-scope" and which are explicitly excluded.

- **Included Targets**: IP ranges (CIDR), domains, or specific URLs.
- **Excluded Targets**: Blacklisted IPs or domains that should never be touched.
- **Enforcement**: Most scanning and fuzzing operations check the `Scope` before initiating a connection.

### `Loader` (`loader.rs`)

Handles the mechanics of finding and parsing configuration files.

- Supports TOML and YAML formats.
- Merges file-based config with command-line overrides.
- Provides default values for all settings.

## Configuration Files

Slapper typically looks for:
1. `--config` / `-c` command-line argument.
2. `slapper.toml` in the current directory.
3. `~/.config/slapper/slapper.toml` on Linux/macOS.

## Usage in Code

Configuration is usually accessed via the `CommandContext`:

```rust
let config = ctx.config();
let timeout = config.http.timeout;
```

Scope check example:

```rust
if !ctx.scope().is_allowed(target) {
    warn!("Target {} is out of scope!", target);
    return Ok(());
}
```
