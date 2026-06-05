# Logging Module

## Overview

Logging configuration and initialization for Slapper. Defined in `crates/slapper/src/logging/`.

## Files

| File | Purpose |
|------|---------|
| `mod.rs` | Re-exports `init_logging`, `LogFormat` |
| `init.rs` | Logging initialization implementation |

## Key Types

- `LogFormat` - log output format: `Pretty` (default), `Json`, `Compact`

## Functions

- `init_logging(format: LogFormat)` - initialize the tracing subscriber with the given format. Reads `RUST_LOG` env var for level filtering (defaults to `info`). Logs an error to stderr if subscriber initialization fails.

## Usage

Called once during application startup in `main.rs`. The format is driven by the `--json` CLI flag.

## Related

- `utils/logging.rs` provides `sanitize_for_logging()` and `sanitize_for_logging_with_max()` for stripping ANSI escapes and control characters from log output (used in the agent module).
