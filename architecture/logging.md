# Logging Module

## Overview

Logging configuration and initialization for Eggsec. Defined in `crates/eggsec/src/logging/`.

## Files

| File | Purpose |
|------|---------|
| `mod.rs` | Re-exports `init_logging`, `LogFormat` |
| `init.rs` | Logging initialization implementation |

## Key Types

- `LogFormat` - log output format: `Pretty` (default), `Json`, `Compact`

## Functions

- `init_logging(format: LogFormat, log_dir: Option<PathBuf>) -> Option<WorkerGuard>` - initialize the tracing subscriber with the given format. When `log_dir` is `Some`, a JSON file layer (`agent.log`, daily rotation) is composed alongside the console layer. Returns a `WorkerGuard` that must be held for the lifetime of the process when a file layer is active. Reads `RUST_LOG` env var for level filtering (defaults to `info`). Logs an error to stderr if subscriber initialization fails.

## Usage

Called once during application startup in `main.rs`. The format is driven by the `--json` CLI flag. When the `agent` subcommand is used, the log directory is resolved from the agent's `memory_dir` and passed to enable file-based logging.

## Related

- `utils/logging.rs` provides `sanitize_for_logging()` for stripping ANSI escapes and control characters from log output (used across scanner, fuzzer, pipeline, recon, stress, and waf modules).
