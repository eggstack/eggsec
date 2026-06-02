# Auth Context Module

## Overview

YAML-based authentication context parsing for multi-user/multi-role testing. Defined in `crates/slapper/src/auth_context/mod.rs`.

## Key Types

### AuthContext

Top-level struct with `version: u32` and `contexts: HashMap<String, AuthContextEntry>`.

### AuthContextEntry

Individual auth context with:
- `description: Option<String>` - human-readable description
- `headers: HashMap<String, String>` - HTTP headers to inject
- `cookies: HashMap<String, String>` - cookies to inject

## Functions

| Function | Description |
|----------|-------------|
| `parse_auth_context(content)` | Parse YAML, interpolate `${VAR}` and `${VAR:-default}` env vars |
| `apply_auth_context(headers, entry)` | Apply context headers to HTTP request headers |
| `list_context_names(ctx)` | Get list of available context names |

## Environment Variable Interpolation

Supports `${VAR}` and `${VAR:-default}` patterns in header and cookie values. Resolved at parse time from process environment.
