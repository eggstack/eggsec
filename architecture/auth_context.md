# Auth Context Module

## Overview

YAML-based authentication context parsing for multi-user/multi-role testing. Defined in `crates/eggsec/src/auth_context/mod.rs`.

## Key Types

### AuthContext

Top-level struct with `version: u32` and `contexts: HashMap<String, AuthContextEntry>`.

### AuthContextEntry

Individual auth context with:
- `description: Option<String>` - human-readable description
- `headers: HashMap<String, String>` - HTTP headers to inject
- `cookies: HashMap<String, String>` - cookies to inject

## Constants

- `SUPPORTED_VERSION: u32` - Currently `1`. Files with other versions are rejected at parse time.

## Functions

| Function | Description |
|----------|-------------|
| `parse_auth_context(content)` | Parse YAML, interpolate `${VAR}` and `${VAR:-default}` env vars |
| `load_auth_context_file(path)` | Load auth context from a file path, parse, and validate version |
| `get_context_entry(ctx, role)` | Get an auth context entry by role name; returns error with available roles if not found |
| `apply_auth_context(headers, entry)` | Apply context headers to HTTP request headers |
| `apply_auth_context_to_request(request, entry)` | Apply headers and cookies to a reqwest `RequestBuilder` |
| `list_context_names(ctx)` | Get list of available context names |

## Cookie Merge Semantics

`apply_auth_context_to_request()` **merges** cookies rather than replacing them. Auth context cookies are joined with any existing `Cookie` header value using `"; "` as separator. If a cookie name from the auth context already exists in the current `Cookie` header, the auth context value wins (takes precedence).

Auth context headers **override** existing headers with the same name (standard `HashMap::insert` semantics).

## Environment Variable Interpolation

Supports `${VAR}` and `${VAR:-default}` patterns in header and cookie values. Resolved at parse time from process environment.
