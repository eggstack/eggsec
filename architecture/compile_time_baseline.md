# Compile Time Baseline

## Context

The first crate-splitting pass created `slapper-core` for shared types/constants.
The second pass extracted the TUI into `slapper-tui` and the binary into `slapper-cli`.

## Workspace layout (post second pass)

```text
crates/
  slapper-core/    # Dependency-light types and constants
  slapper/         # Assessment engine library (no binary)
  slapper-nse/     # Optional NSE compatibility
  slapper-tui/     # Terminal UI adapter (ratatui/crossterm)
  slapper-cli/     # CLI binary entry point
```

## Current baseline

Date: 2026-06-08
Rust version: (see `rustc --version`)
Cargo version: (see `cargo --version`)

### Commands

```bash
cargo check -p slapper-core          # Core types only
cargo check --lib -p slapper         # Engine library
cargo check -p slapper-tui           # TUI adapter
cargo check -p slapper-cli           # CLI binary
cargo check -p slapper --no-default-features  # Engine without features
cargo check --workspace              # Full workspace
```

## Post-second-pass notes

- `slapper-core` now has 4 dependencies (serde, serde_json, subtle, zeroize)
- `ratatui` and `crossterm` are dependencies of `slapper-tui`, not `slapper`
- The `slapper` library no longer has a `tui` module
- Feature-gated TUI tabs are conditionally compiled in `slapper-tui`
