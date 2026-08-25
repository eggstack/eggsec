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

| Function | Line | Signature | Purpose |
|----------|------|-----------|---------|
| `parse_auth_context()` | `:47` | `(content: &str) -> Result<AuthContext>` | Parse YAML, validate version, interpolate env vars |
| `apply_auth_context()` | `:71` | `(headers: &mut HashMap, entry: &AuthContextEntry)` | Apply context headers to a header map |
| `apply_auth_context_to_request()` | `:107` | `(RequestBuilder, &AuthContextEntry) -> RequestBuilder` | Apply headers and cookies to reqwest request |
| `load_auth_context_file()` | `:83` | `(path: &Path) -> Result<AuthContext>` | Load + parse from file path |
| `get_context_entry()` | `:91` | `(&AuthContext, role: &str) -> Result<&AuthContextEntry>` | Lookup by role; error with available roles on miss |
| `list_context_names()` | `:78` | `(&AuthContext) -> Vec<String>` | List all role names |
| `interpolate_env_vars()` | `:33` | `(input: &str) -> String` | Replace `${VAR}` / `${VAR:-default}` patterns |

### Environment Variable Interpolation (`:33`)

The regex `\$\{([^}:]+)(?::-([^}]*))?\}` (`:30`) matches:
- `${VAR}` — replaced with `std::env::var("VAR")`, or empty string if unset.
- `${VAR:-default}` — replaced with env var value, or `"default"` if unset/missing.

Interpolation is applied to **all header and cookie values** during `parse_auth_context()` (`:58-65`). It is resolved at parse time from the process environment.

### Cookie Merge Semantics (`:107-119`)

`apply_auth_context_to_request()` applies credentials to a `reqwest::RequestBuilder`:

1. **Headers**: Each auth context header is set via `req.header(key, value)` (`:112-114`). This **overwrites** any existing header with the same name (standard `reqwest::RequestBuilder::header` semantics).

2. **Cookies**: If the auth context has any cookies, `merge_cookies()` (`:125`) produces a `"; "`-joined string from auth context cookies only, set as the `Cookie` header (`:116`). This **replaces** any pre-existing `Cookie` header entirely.

**Important implementation note**: The `merge_cookies()` function (`:125-132`) only produces cookies from the `AuthContextEntry` — it does not read or merge with the request's existing `Cookie` header. The `reqwest::RequestBuilder::header()` call overwrites any prior `Cookie` header. The doc comment at `:104-106` states "auth context cookies are merged with any existing Cookie header" but the current implementation replaces rather than merges. In practice this is typically the desired behavior (auth context credentials take precedence), but callers with pre-existing cookies on the request should be aware.

## Behavior / Flow

### Parse Flow

```
YAML content
  → serde_yaml_neo::from_str()           (:48)
  → version check (must == 1)             (:50-56)
  → interpolate env vars in all values    (:58-65)
  → return AuthContext
```

### Apply Flow (to reqwest RequestBuilder)

```
apply_auth_context_to_request(request, entry)
  → set each header from entry.headers    (:112-114)
  → if cookies non-empty:
      → merge_cookies(entry)              (:125)
      → set "Cookie" header               (:116)
  → return modified RequestBuilder
```

### Error Handling

- `parse_auth_context()`: Returns `anyhow::Result`. YAML parse errors and version mismatches produce descriptive errors.
- `load_auth_context_file()`: Wraps file I/O and parse errors with context (`:84-88`).
- `get_context_entry()`: Returns error with available role names if role not found (`:92-98`).

## Integration Points

| Consumer | File:Line | How It Uses Auth Context |
|----------|-----------|--------------------------|
| Fuzzer engine | `fuzzer/engine/core.rs:184-185` | Loads auth context file, gets entry by role |
| Fuzzer HTTP utils | `fuzzer/engine/utils.rs:96,141,237` | Applies entry to fuzz requests via `apply_auth_context_to_request()` |
| (Future) CLI scanner | CLI handler code | Could load auth context for authenticated scans |
| (Future) REST/MCP tools | Tool protocol code | Could apply auth context to tool requests |

## Testing

All tests are in `auth_context/mod.rs:134-275`. Test count: 10 tests total.

| Test | Line | What It Verifies |
|------|------|------------------|
| `parse_auth_context_works` | `:152` | Parses sample YAML, 2 contexts |
| `context_descriptions_are_parsed` | `:161` | Description field extraction |
| `env_var_interpolation_with_default` | `:170` | `${VAR:-fallback}` with missing var |
| `env_var_interpolation_with_real_var` | `:178` | `${VAR}` with set env var |
| `apply_auth_context_to_headers` | `:187` | Header insertion |
| `test_list_context_names` | `:206` | Context name listing |
| `parse_auth_context_with_cookies` | `:214` | Cookie parsing, static + env-var |
| `unsupported_version_is_rejected` | `:234` | Version 2 → error |
| `deny_unknown_fields_rejects_extra_keys` | `:249` | Top-level unknown key → error |
| `deny_unknown_fields_rejects_extra_entry_keys` | `:263` | Entry-level unknown key → error |

## Invariants & Gotchas

### Invariants

1. **Version 1 only** — Files with `version != 1` are rejected at parse time (`:50-56`).
2. **Strict deserialization** — `#[serde(deny_unknown_fields)]` rejects unexpected YAML keys.
3. **Env var interpolation at parse time** — `${VAR}` patterns are resolved once during parsing, not at apply time.
4. **Headers override, not merge** — Auth context headers replace existing headers with the same name.
5. **Fail-closed on unknown roles** — `get_context_entry()` returns an error listing available roles.
6. **Regex is static** — `ENV_VAR_RE` is `LazyLock<Regex>` (`:29`), compiled once, no per-call cost.

### Gotchas

- **`ENV_VAR_RE` uses `expect()` on compilation** (`:30`): `Regex::new(...).expect("valid env var regex")`. This panics if the regex is invalid. The regex is a compile-time constant and has been validated by tests, but a future modification to the regex pattern could cause a panic at first use.
- **Cookie merge is not a true merge**: `merge_cookies()` (`:125`) only produces auth context cookies. It does not incorporate existing cookies from the request. The `Cookie` header is replaced entirely.
- **`interpolate_env_vars` is not URL-aware**: If an env var value contains special characters (spaces, semicolons), they are passed through verbatim. Callers are responsible for encoding.
- **`apply_auth_context` (HashMap variant) vs `apply_auth_context_to_request`**: The HashMap variant (`:71`) only applies headers, not cookies. The reqwest variant (`:107`) applies both headers and cookies. Callers must use the correct function.
- **`HashMap` iteration order**: Cookie header order depends on `HashMap` iteration order (random). If cookie order matters (some servers are order-sensitive), this could be an issue.

---

*Last verified against source: 2026-08-25*
