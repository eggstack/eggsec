# Stress Testing Module

## Overview

The stress testing module provides load generation and denial-of-service simulation capabilities for authorized security testing. It requires the `stress-testing` feature flag and raw socket privileges.

**Feature gate:** `stress-testing` (in `Cargo.toml`)

## Module Structure

| File | Purpose |
|------|---------|
| `mod.rs` | `StressType` enum, `StressConfig`, orchestration |
| `syn.rs` | SYN flood implementation (raw sockets) |
| `udp.rs` | UDP flood implementation |
| `http.rs` | HTTP flood implementation |
| `icmp.rs` | ICMP flood implementation |
| `metrics.rs` | `StressMetrics`, `StressStats` collection |
| `authorization.rs` | `StressAuthorization` - pre-flight authorization checks |
| `warning.rs` | `display_warning()`, `require_confirmation()` - safety prompts |

## Key Types

### StressType Enum

Five flood types: `Syn`, `Udp`, `Http`, `Tcp`, `Icmp`.

### StressConfig

Configuration for a stress test run: `target`, `port`, `stress_type`, `duration_secs`, `concurrency`, `rate_pps`, `spoof_source`, `spoof_range`, `random_source_port`, `payload_size`, `use_proxies`, `proxy_pool`.

### StressAuthorization

Pre-flight authorization requiring explicit user confirmation before executing stress tests. Enforces scope validation and displays legal warnings.

## Feature Gating

The `http`, `icmp`, `syn`, and `udp` submodules are only compiled with `#[cfg(feature = "stress-testing")]`. The `authorization`, `metrics`, and `warning` modules are always compiled.
