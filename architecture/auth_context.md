# Auth Context Module

## Role & Responsibilities

`auth_context/mod.rs` provides YAML-based authentication context parsing for multi-user/multi-role security testing. It loads credential sets (HTTP headers and cookies) from YAML files, interpolates environment variables, and applies them to HTTP requests.

**Non-responsibilities:**
- Auth context does not perform authorization or scope checking — it only injects credentials.
- Auth context does not manage session lifecycle (token refresh, expiry) — it is a static credential injection mechanism.
- Auth context does not perform TLS or certificate handling.
- Auth context does not enforce which roles are valid — role names are free-form strings in the YAML file.

## Location & Feature Gating

| Item | Path | Feature Gate |
|------|------|:------------:|
| Auth context module | `crates/eggsec/src/auth_context/mod.rs` | None (always compiled) |
| Fuzzer consumer (apply) | `crates/eggsec/src/fuzzer/engine/utils.rs:96,141,237` | None |
| Fuzzer consumer (load) | `crates/eggsec/src/fuzzer/engine/core.rs:184` | None |

## Architecture

### Key Types

| Type | Line | Derives | Purpose |
|------|------|---------|---------|
| `AuthContext` | `:14` | `Debug, Clone, Serialize, Deserialize` | Top-level parsed YAML structure |
| `AuthContextEntry` | `:20` | `Debug, Clone, Serialize, Deserialize` | Individual role's credentials |

Both structs use `#[serde(deny_unknown_fields)]` (`:13,19`) — extra YAML keys cause a parse error.

#### `AuthContext` (`:14`)

| Field | Type | Purpose |
|-------|------|---------|
| `version` | `u32` | File format version (must be `1`) |
| `contexts` | `HashMap<String, AuthContextEntry>` | Map of role name → credentials |

#### `AuthContextEntry` (`:20`)

| Field | Type | `#[serde]` | Purpose |
|-------|------|-----------|---------|
| `description` | `Option<String>` | — | Human-readable description |
| `headers` | `HashMap<String, String>` | `default` | HTTP headers to inject |
| `cookies` | `HashMap<String, String>` | `default` | Cookies to inject |

### Constants

| Constant | Line | Value | Purpose |
|----------|------|-------|---------|
| `SUPPORTED_VERSION` | `:9` | `1` | Accepted file format version |
| `ENV_VAR_RE` | `:29` | LazyLock regex | Matches `${VAR}` and `${VAR:-default}` patterns |

### Functions

Phase B canonical paths are transport-neutral (no concrete client types).
The `reqwest` wrapper is a temporary compatibility shim for the
pre-migration backend.

| Function | Signature | Canonical? | Purpose |
|----------|-----------|------------|---------|
| `parse_auth_context()` | `(content: &str) -> Result<AuthContext>` | Yes | Parse YAML, validate version, interpolate env vars |
| `apply_auth_context()` | `(headers: &mut HashMap, entry: &AuthContextEntry)` | Yes (headers only) | Apply context headers to a header map |
| `apply_auth_context_to_transport()` | `(&mut HeaderMap, &AuthContextEntry) -> Result<(), String>` | Yes | Apply headers (overwrite) + cookies (true merge) to `eggsec_transport::HeaderMap` |
| `apply_auth_context_to_map()` | `(&mut HashMap, Option<&str>, &AuthContextEntry) -> Option<String>` | Yes | Pure-map headers + merged `Cookie` value |
| `apply_auth_context_to_request()` | `(RequestBuilder, &AuthContextEntry) -> RequestBuilder` | Compat only | Temporary concrete-backend wrapper; delegates to canonical merge |
| `load_auth_context_file()` | `(path: &Path) -> Result<AuthContext>` | Yes | Load + parse from file path |
| `get_context_entry()` | `(&AuthContext, role: &str) -> Result<&AuthContextEntry>` | Yes | Lookup by role; error with available roles on miss |
| `list_context_names()` | `(&AuthContext) -> Vec<String>` | Yes | List all role names |
| `interpolate_env_vars()` | `(input: &str) -> String` | Yes | Replace `${VAR}` / `${VAR:-default}` patterns |

### Environment Variable Interpolation (`:33`)

The regex `\$\{([^}:]+)(?::-([^}]*))?\}` (`:30`) matches:
- `${VAR}` — replaced with `std::env::var("VAR")`, or empty string if unset.
- `${VAR:-default}` — replaced with env var value, or `"default"` if unset/missing.

Interpolation is applied to **all header and cookie values** during `parse_auth_context()` (`:58-65`). It is resolved at parse time from the process environment.

### Cookie Merge Semantics (Phase B canonical: true merge)

Canonical (`apply_auth_context_to_transport` / `_to_map` via
`eggsec_transport::merge_cookie_header`):

1. **Headers**: overwrite same-named headers.
2. **Cookies**: true merge with any existing `Cookie` header — auth-context
   values win on name collision, unrelated existing cookies preserved,
   deterministic (sorted) order.

**History note**: the pre-Phase-B concrete wrapper *replaced* the `Cookie`
header (auth-context cookies only). The canonical behavior is a true merge;
the wrapper now delegates to the canonical merge helper. See
[transport.md](transport.md) for the contract.

## Behavior / Flow

### Parse Flow

```
YAML content
  → serde_yaml_neo::from_str()
  → version check (must == 1)
  → interpolate env vars in all values
  → return AuthContext
```

### Apply Flow (canonical transport-neutral)

```
apply_auth_context_to_transport(map, entry)
  → eggsec_transport::apply_auth_headers(map, headers, cookies)
  → headers overwrite; cookies merged (auth wins on collision)

apply_auth_context_to_map(headers, existing_cookie?, entry)
  → headers overwrite; return merged Cookie value
```

### Error Handling

- `parse_auth_context()`: Returns `anyhow::Result`. YAML parse errors and version mismatches produce descriptive errors.
- `load_auth_context_file()`: Wraps file I/O and parse errors with context (`:84-88`).
- `get_context_entry()`: Returns error with available role names if role not found (`:92-98`).

## Integration Points

| Consumer | How It Uses Auth Context |
|----------|--------------------------|
| Fuzzer engine | Loads auth context file, gets entry by role |
| Fuzzer HTTP utils | Applies entry via compat `apply_auth_context_to_request()` (pre-migration); new code uses `apply_auth_context_to_transport()` |
| Transport contract | Canonical `apply_auth_context_to_transport()` / `_to_map()` + `eggsec_transport::apply_auth_headers()` / `merge_cookie_header()` |
| (Future) CLI scanner | Could load auth context for authenticated scans |
| (Future) REST/MCP tools | Could apply auth context to tool requests |

## Testing

All tests are in `auth_context/mod.rs`. Test count: 12 tests total
(10 original + `transport_map_merges_cookies_and_overwrites_headers`,
`transport_headermap_applies_without_concrete_types`).

## Invariants & Gotchas

### Invariants

1. **Version 1 only** — Files with `version != 1` are rejected at parse time.
2. **Strict deserialization** — `#[serde(deny_unknown_fields)]` rejects unexpected YAML keys.
3. **Env var interpolation at parse time** — `${VAR}` patterns are resolved once during parsing, not at apply time.
4. **Headers override, not merge** — Auth context headers replace existing headers with the same name.
5. **Cookies truly merge (Phase B)** — Auth-context cookies win on collision, unrelated existing cookies preserved, sorted order.
6. **Fail-closed on unknown roles** — `get_context_entry()` returns an error listing available roles.
7. **Canonical is transport-neutral** — New code uses `HeaderMap`/pure-map helpers; the concrete wrapper is compat-only.

### Gotchas

- **`ENV_VAR_RE` uses `expect()` on compilation**: `Regex::new(...).expect("valid env var regex")`. This panics if the regex is invalid. The regex is a compile-time constant and has been validated by tests, but a future modification to the regex pattern could cause a panic at first use.
- **`interpolate_env_vars` is not URL-aware**: If an env var value contains special characters (spaces, semicolons), they are passed through verbatim. Callers are responsible for encoding.
- **`apply_auth_context` (HashMap headers-only) vs canonical**: The legacy HashMap variant only applies headers, not cookies. Use `apply_auth_context_to_transport()` / `_to_map()` for headers + cookies.

---

See also: [transport.md](transport.md), [network_dependency_baseline.md](network_dependency_baseline.md)

*Last verified against source: 2026-09-12*
