# Compile Time Baseline

## Context

The first crate-splitting pass created `slapper-core` for shared types/constants.
The second pass extracted the TUI into `slapper-tui` and the binary into `slapper-cli`.
The third pass removed unused TUI dependencies from `slapper` and extracted `slapper-output`.

## Workspace layout (post third pass)

```text
crates/
  slapper-core/      # Dependency-light types and constants
  slapper-tool-core/ # Core data types for tool abstraction layer
  slapper/           # Assessment engine library (no binary)
  slapper-nse/       # Optional NSE compatibility
  slapper-tui/       # Terminal UI adapter (ratatui/crossterm)
  slapper-cli/       # CLI binary entry point (binary named "slapper")
  slapper-output/    # Report formatting and output adapters
```

## Third pass changes

- Removed `ratatui`, `crossterm`, `arboard` from `slapper` crate (unused dependencies)
- Added explicit `[[bin]] name = "slapper"` to `slapper-cli` (was relying on auto-discovery)
- Created `slapper-output` crate for report formatting (JSON, CSV, HTML, SARIF, JUnit, Markdown)
- Output modules with deep engine coupling (`pdf`, `report`, `report_summary`, `run_manifest`, `attack_graph`) remain in `slapper`

## Third pass commands

```bash
rustc --version
cargo --version
cargo check -p slapper-core
cargo check -p slapper --no-default-features
cargo check -p slapper-tui
cargo check -p slapper-cli
cargo check -p slapper-output
cargo check --workspace --all-targets --no-default-features
cargo check -p slapper-cli --features nse
cargo check -p slapper-cli --features rest-api
cargo check -p slapper-cli --features stress-testing
```

## Results after third pass

- `slapper-core`: pass
- `slapper --no-default-features`: pass
- `slapper-tui`: pass
- `slapper-cli`: pass
- `slapper-output`: pass
- `--workspace --all-targets --no-default-features`: pass
- `slapper-cli --features nse`: pass
- `slapper-cli --features rest-api`: pass (pre-existing: server startup warnings)
- `slapper-cli --features stress-testing`: pass

## Post-third-pass notes

- `slapper-core` has 4 dependencies (serde, serde_json, subtle, zeroize)
- `slapper` no longer depends on `ratatui`, `crossterm`, or `arboard`
- `slapper-output` depends on `slapper-core` (not on `slapper`)
- Output modules with engine coupling remain in `slapper` to avoid cycles
- The `slapper` crate re-exports `slapper_output` as `output` for backward compatibility

## Notes

Pre-first-pass and pre-second-pass timing data are not available.

## Interpretation

The current crate split isolates terminal UI dependencies from the engine crate and moves portable output/tool DTO code into separate crates. The main `slapper` crate remains the largest compile unit because it still owns scanning, web/security modules, API adapters, command dispatch, config, and feature-gated integrations.

## Final modularization stabilization pass

### Workspace state

```text
crates/
  slapper-core/      # Dependency-light types and constants
  slapper-tool-core/ # Core data types for tool abstraction layer
  slapper/           # Assessment engine library (no binary)
  slapper-nse/       # Optional NSE compatibility
  slapper-tui/       # Terminal UI adapter (ratatui/crossterm)
  slapper-cli/       # CLI binary entry point (binary named "slapper")
  slapper-output/    # Report formatting and output adapters
  slapper-agent/     # Agent coordination primitives (extracted from tool/agents/)
```

### Commands run

```bash
cargo check -p slapper-core
cargo check -p slapper-tool-core
cargo check -p slapper-output
cargo check -p slapper-agent
cargo check -p slapper --no-default-features
cargo check -p slapper-tui
cargo check -p slapper-cli
cargo check -p slapper-cli --features nse
cargo check -p slapper-cli --features rest-api
cargo check -p slapper-cli --features stress-testing
cargo check -p slapper-cli --features pdf
cargo test -p slapper-core
cargo test -p slapper-tool-core
cargo test -p slapper-output
cargo test -p slapper-agent
cargo test -p slapper --lib
```

### Results

- `slapper-core`: pass
- `slapper-tool-core`: pass
- `slapper-output`: pass
- `slapper-agent`: pass (new crate)
- `slapper --no-default-features`: pass
- `slapper-tui`: pass
- `slapper-cli`: pass
- `slapper-cli --features nse`: pass
- `slapper-cli --features rest-api`: pass
- `slapper-cli --features stress-testing`: pass
- `slapper-cli --features pdf`: pass

### Final interpretation

This completes the initial crate modularization phase. The `slapper-agent` crate was extracted from `tool/agents/` with zero blockers — all constants already lived in `slapper-core` and the module had no coupling to engine types. Further splits should be driven by measured compile-time hot paths or clearly isolated adapter boundaries.

- `slapper-agent` owns the agent coordination implementation; `slapper::tool::agents` is only a compatibility facade.
- `reqwest` remains in `slapper-agent` because lifecycle callback health checks use it.
