# Compile Time Baseline

## Context

The first crate-splitting pass created `slapper-core` for shared types/constants.
The second pass extracted the TUI into `slapper-tui` and the binary into `slapper-cli`.
The third pass removed unused TUI dependencies from `slapper` and extracted `slapper-output`.

## Workspace layout (post third pass)

```text
crates/
  slapper-core/    # Dependency-light types and constants
  slapper/         # Assessment engine library (no binary)
  slapper-nse/     # Optional NSE compatibility
  slapper-tui/     # Terminal UI adapter (ratatui/crossterm)
  slapper-cli/     # CLI binary entry point (binary named "slapper")
  slapper-output/  # Report formatting and output adapters
```

## Third pass changes

- Removed `ratatui`, `crossterm`, `arboard` from `slapper` crate (unused dependencies)
- Added explicit `[[bin]] name = "slapper"` to `slapper-cli` (was relying on auto-discovery)
- Created `slapper-output` crate for report formatting (JSON, CSV, HTML, SARIF, JUnit, Markdown)
- Output modules with deep engine coupling (`pdf`, `template`, `run_manifest`, `attack_graph`, `report`, `report_summary`) remain in `slapper`

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
