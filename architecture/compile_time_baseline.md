# Compile Time Baseline

## Context

The first crate-splitting pass created `eggsec-core` for shared types/constants.
The second pass extracted the TUI into `eggsec-tui` and the binary into `eggsec-cli`.
The third pass removed unused TUI dependencies from `eggsec` and extracted `eggsec-output`.

## Workspace layout (post third pass)

```text
crates/
  eggsec-core/       # Dependency-light types and constants
  eggsec-tool-core/  # Core data types for tool abstraction layer
  eggsec/            # Assessment engine library (no binary)
  eggsec-nse/        # Optional NSE compatibility
  eggsec-tui/        # Terminal UI adapter (ratatui/crossterm)
  eggsec-cli/        # CLI binary entry point (binary named "eggsec")
  eggsec-output/     # Report formatting and output adapters
  eggsec-agent/      # Agent coordination primitives (extracted from tool/agents/)
  eggsec-db-lab/     # Database pentesting domain crate (Postgres/MySQL/MSSQL/MongoDB/Redis)
  eggsec-web-proxy/  # Web proxy and MITM interception domain crate
  eggsec-mobile-lab/ # Mobile app security analysis domain crate (APK/IPA static + Android dynamic)
  eggsec-runtime/    # Frontend-neutral runtime DTOs and protocol types for daemon architecture
```

## Third pass changes

- Removed `ratatui`, `crossterm`, `arboard` from `eggsec` crate (unused dependencies)
- Added explicit `[[bin]] name = "eggsec"` to `eggsec-cli` (was relying on auto-discovery)
- Created `eggsec-output` crate for report formatting (JSON, CSV, HTML, SARIF, JUnit, Markdown)
- Output modules with deep engine coupling (`pdf`, `report`, `report_summary`, `run_manifest`, `attack_graph`) remain in `eggsec`

## Third pass commands

```bash
rustc --version
cargo --version
cargo check -p eggsec-core
cargo check -p eggsec --no-default-features
cargo check -p eggsec-tui
cargo check -p eggsec-cli
cargo check -p eggsec-output
cargo check --workspace --all-targets --no-default-features
cargo check -p eggsec-cli --features nse
cargo check -p eggsec-cli --features rest-api
cargo check -p eggsec-cli --features stress-testing
```

## Results after third pass

- `eggsec-core`: pass
- `eggsec --no-default-features`: pass
- `eggsec-tui`: pass
- `eggsec-cli`: pass
- `eggsec-output`: pass
- `--workspace --all-targets --no-default-features`: pass
- `eggsec-cli --features nse`: pass
- `eggsec-cli --features rest-api`: pass (pre-existing: server startup warnings)
- `eggsec-cli --features stress-testing`: pass

## Post-third-pass notes

- `eggsec-core` has 4 dependencies (serde, serde_json, subtle, zeroize)
- `eggsec` no longer depends on `ratatui`, `crossterm`, or `arboard`
- `eggsec-output` depends on `eggsec-core` (not on `eggsec`)
- Output modules with engine coupling remain in `eggsec` to avoid cycles
- The `eggsec` crate re-exports `eggsec_output` as `output` for backward compatibility

## Notes

Pre-first-pass and pre-second-pass timing data are not available.

## Interpretation

The current crate split isolates terminal UI dependencies from the engine crate and moves portable output/tool DTO code into separate crates. The main `eggsec` crate remains the largest compile unit because it still owns scanning, web/security modules, API adapters, command dispatch, config, and feature-gated integrations.

## Final modularization stabilization pass

### Workspace state

```text
crates/
  eggsec-core/       # Dependency-light types and constants
  eggsec-tool-core/  # Core data types for tool abstraction layer
  eggsec/            # Assessment engine library (no binary)
  eggsec-nse/        # Optional NSE compatibility
  eggsec-tui/        # Terminal UI adapter (ratatui/crossterm)
  eggsec-cli/        # CLI binary entry point (binary named "eggsec")
  eggsec-output/     # Report formatting and output adapters
  eggsec-agent/      # Agent coordination primitives (extracted from tool/agents/)
  eggsec-db-lab/     # Database pentesting domain crate (Postgres/MySQL/MSSQL/MongoDB/Redis)
  eggsec-web-proxy/  # Web proxy and MITM interception domain crate
  eggsec-mobile-lab/ # Mobile app security analysis domain crate (APK/IPA static + Android dynamic)
  eggsec-runtime/    # Frontend-neutral runtime DTOs and protocol types for daemon architecture
```

### Commands run

```bash
cargo check -p eggsec-core
cargo check -p eggsec-tool-core
cargo check -p eggsec-output
cargo check -p eggsec-agent
cargo check -p eggsec --no-default-features
cargo check -p eggsec-tui
cargo check -p eggsec-cli
cargo check -p eggsec-cli --features nse
cargo check -p eggsec-cli --features rest-api
cargo check -p eggsec-cli --features stress-testing
cargo check -p eggsec-cli --features pdf
cargo test -p eggsec-core
cargo test -p eggsec-tool-core
cargo test -p eggsec-output
cargo test -p eggsec-agent
cargo test -p eggsec --lib
```

### Results

- `eggsec-core`: pass
- `eggsec-tool-core`: pass
- `eggsec-output`: pass
- `eggsec-agent`: pass (new crate)
- `eggsec --no-default-features`: pass
- `eggsec-tui`: pass
- `eggsec-cli`: pass
- `eggsec-cli --features nse`: pass
- `eggsec-cli --features rest-api`: pass
- `eggsec-cli --features stress-testing`: pass
- `eggsec-cli --features pdf`: pass

### Final interpretation

This completes the initial crate modularization phase. The `eggsec-agent` crate was extracted from `tool/agents/` with zero blockers — all constants already lived in `eggsec-core` and the module had no coupling to engine types. Further splits should be driven by measured compile-time hot paths or clearly isolated adapter boundaries.

- `eggsec-agent` owns the agent coordination implementation; `eggsec::tool::agents` is only a compatibility facade.
- `reqwest` remains in `eggsec-agent` because lifecycle callback health checks use it.
