# Logging Module

## Overview

Logging configuration and initialization for Slapper. Defined in `crates/slapper/src/logging/`.

## Files

| File | Purpose |
|------|---------|
| `mod.rs` | Re-exports `init_logging`, `LogFormat`, `LogLevel` |
| `init.rs` | Logging initialization implementation |

## Key Types

- `LogFormat` - log output format (e.g., pretty, compact, JSON)
- `LogLevel` - log verbosity level

## Functions

- `init_logging()` - initialize the logging subsystem with configured format and level

## Usage

Called during application startup to configure tracing/subscriber output.
