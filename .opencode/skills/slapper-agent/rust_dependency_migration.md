---
name: rust_dependency_migration
description: Guide for migrating Rust web framework dependencies in Slapper
triggers:
  - axum migration
  - tonic migration
  - update axum
  - update tonic
  - dependency update
  - 0.7 to 0.8
  - 0.12 to 0.14
metadata:
  category: maintenance
  tools: [cargo, rustc]
  scope: dependencies
---

## Overview

This skill guides the migration of major Rust web framework dependencies in the Slapper security toolkit. It documents the breaking changes and migration paths for Axum and Tonic.

## Current Versions (as of 2026-04-24)

- axum: 0.8.x
- tonic: 0.14.x
- prost: 0.14.x
- tokio-tungstenite: 0.26.x

## Axum 0.7 → 0.8 Migration

### Breaking Changes

1. **Path Parameter Syntax** (CRITICAL)
   - Old: `/:param`, `/*many`
   - New: `/{param}`, `/{*many}`
   - The app will panic at startup if using old syntax

2. **async_trait Removal**
   - Axum 0.8 uses native async trait support (RPITIT)
   - `#[async_trait]` attribute no longer needed for extractors
   - Can still use `async-trait` crate for custom traits

3. **Option Extractor Behavior**
   - Previously: silently swallows all rejections
   - Now: rejects in many cases without OptionalFromRequestParts

4. **WebSocket Message Types**
   - `Message::Text` now uses `Utf8Bytes` instead of `String`
   - `Message::Binary` now uses `Bytes` instead of `Vec<u8>`

### Migration Commands

```bash
# Update Cargo.toml
cargo upgrade axum@0.8 tonic@0.14 prost@0.14 --dry-run

# Test rebuild
cargo check --lib -p slapper --features rest-api,grpc-api,ai-integration

# Run tests
cargo test --lib -p slapper --features rest-api,ai-integration
```

### Files to Update

Update these files when migrating route paths:
- `crates/slapper/src/tool/protocol/rest.rs` - `/api/v1/tools/:tool_id` → `/api/v1/tools/{tool_id}`
- `crates/slapper/src/tool/protocol/mcp/routes.rs` - `/mcp/stream/:request_id` → `/mcp/stream/{request_id}`

## Tonic 0.12 → 0.14 Migration

### Breaking Changes

1. **Prost Extraction**
   - Prost moved to separate crates: `tonic-prost` and `tonic-prost-build`
   - All previous types still available but under new locations

2. **Prost Version Bump**
   - Must use prost 0.14 with tonic 0.14
   - Incompatible with older prost versions

### Cargo.toml Updates

```toml
[dependencies.axum]
version = "0.8"

[dependencies.tonic]
version = "0.14"

[dependencies.prost]
version = "0.14"

[build-dependencies.prost-build]
version = "0.14"
```

## Testing Verification

Run these commands to verify the migration:

```bash
# Core library
cargo test --lib -p slapper

# With API features
cargo test --lib -p slapper --features rest-api,ai-integration

# Feature combinations
cargo check --lib -p slapper --features rest-api,grpc-api,ai-integration

# Clippy
cargo clippy --lib -p slapper --features rest-api,ai-integration
```

## Known Test Failures

Pre-existing failures in AI modules (not related to dependency updates):
- `ai::client::tests::test_extract_content_valid_response`
- `ai::planner::tests::*` (6 tests)

These failures existed before the dependency migration and are related to response parsing, not Axum/Tonic.

## Triggers

Keywords that activate this skill:
- "axum migration"
- "tonic migration"  
- "update axum"
- "update tonic"
- "dependency update"
- "0.7 to 0.8"
- "0.12 to 0.14"