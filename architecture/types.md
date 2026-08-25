# Core Types

## Overview

Shared types for the Eggsec workspace, split across two layers. The dependency-light core crate (`eggsec-core`) defines `Severity` and `SensitiveString`. The main engine crate (`eggsec`) re-exports those and adds `OutputFormat`, `ScanProfile`, and `CommonHttpArgs` which depend on `clap` or engine internals.

Related: [constants.md](constants.md), [error.md](error.md), [overview.md](overview.md).

---

## Location & Feature Gating

| Type | Defined in | Re-export path |
|------|-----------|----------------|
| `Severity` | `crates/eggsec-core/src/types.rs:15` | `eggsec_core::types::Severity`, `eggsec::types::Severity` |
| `SensitiveString` | `crates/eggsec-core/src/types.rs:136` | `eggsec_core::types::SensitiveString`, `eggsec::types::SensitiveString` |
| `OutputFormat` | `crates/eggsec/src/types.rs:106` | `eggsec::types::OutputFormat` |
| `ScanProfile` | `crates/eggsec/src/types.rs:123` | `eggsec::types::ScanProfile` |
| `CommonHttpArgs` | `crates/eggsec/src/types.rs:148` | `eggsec::types::CommonHttpArgs` |
| `check_config_file_permissions` | `crates/eggsec/src/types.rs:41` | `eggsec::types::check_config_file_permissions` |

`ValueEnum` derive (clap CLI integration) is conditional on `feature = "cli"` for both `OutputFormat` and `ScanProfile`.

---

## Architecture

### Severity

Canonical severity rating for findings and vulnerabilities. Defined in the core crate with zero internal dependencies.

**Variants** (5): `Critical`, `High`, `Medium`, `Low`, `Info` (default) — `crates/eggsec-core/src/types.rs:15-22`

**Derives**: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`, `Default` — `:13`

**Serde**: `#[serde(rename_all = "lowercase")]` — serializes as lowercase strings.

**Key methods** (all `#[must_use]`):

| Method | Source | Description |
|--------|--------|-------------|
| `parse_or_default(s)` | `:29` | Parse from string, defaults to `Info`. Prefer `s.parse::<Severity>()` for new code. |
| `from_cvss(score)` | `:38` | CVSS → severity: `>=9.0`=Critical, `>=7.0`=High, `>=4.0`=Medium, `>=0.1`=Low, else Info. NaN/negative → Info. |
| `as_str()` | `:50` | Lowercase `&'static str`: `"critical"`, `"high"`, `"medium"`, `"low"`, `"info"` |
| `as_int()` | `:62` | Integer ranking: Critical=4, High=3, Medium=2, Low=1, Info=0 |
| `cvss_color()` | `:74` | Unicode emoji: 🔴, 🟠, 🟢, 🔵, ⚪ |

**Trait implementations**:

| Trait | Behavior |
|-------|----------|
| `Ord` / `PartialOrd` | `:118` — based on `as_int()` ranking. Critical > High > Medium > Low > Info. |
| `Display` | `:85` — uppercase: `"CRITICAL"`, `"HIGH"`, `"MEDIUM"`, `"LOW"`, `"INFO"` |
| `FromStr` | `:97` — case-insensitive; also accepts `"moderate"` as `Medium`. Unknown → `Info` (infallible `Err = Infallible`). |
| `Default` | `Info` |

**Re-exports**: `eggsec-core/src/lib.rs:13` re-exports `Severity` at crate root.

---

### SensitiveString

Zeroized credential wrapper for passwords, API keys, and tokens. Defined in the core crate.

**Structure**: `struct SensitiveString(String)` — `:136`

**Derives**: `Clone`, `Zeroize`, `ZeroizeOnDrop` — `:135`

**Key methods**:

| Method | Source | Description |
|--------|--------|-------------|
| `new(s)` | `:140` | Create from any `Into<String>` |
| `len()` | `:145` | Length of inner string |
| `as_bytes()` | `:150` | Raw byte slice access |
| `is_empty()` | `:155` | Emptiness check |
| `expose_secret()` | `:161` | Borrow inner `&str` |
| `into_secret()` | `:169` | Consume and return inner `String` (uses `std::mem::take` to leave owned empty string for safe drop) |
| `log_secret(logger, redact)` | `:176` | Call logger with value or `"[REDACTED]"` based on `redact` flag |
| `for_logging(redact)` | `:189` | Returns `impl Display` that shows value or `"[REDACTED]"` |

**Trait implementations**:

| Trait | Source | Behavior |
|-------|--------|----------|
| `PartialEq` (SensitiveString) | `:244` | Constant-time via `subtle::ConstantTimeEq` on byte slices |
| `PartialEq<str>` | `:252` | Constant-time comparison with `&str` |
| `PartialEq<&str>` | `:258` | Constant-time comparison with `&&str` |
| `PartialEq<String>` | `:264` | Constant-time comparison with `String` |
| `Eq` | `:250` | Marker trait |
| `Debug` | `:210` | Always redacted: `"SensitiveString([REDACTED])"` |
| `Display` | `:216` | Always redacted: `"[REDACTED]"` |
| `Serialize` | `:222` | **Plaintext** — intentional for config file compatibility. Security warning in doc comment. |
| `Deserialize` | `:238` | Transparent string deserialization |
| `From<String>` | `:270` | Direct wrap |
| `From<&str>` | `:276` | Converts to `String` then wraps |

**Intentionally NOT implemented**: `Hash` — prevents correlation attacks via hash tables. If a credential must be a map key, derive a non-secret identifier instead. (`:131-134`)

**Dependencies** (core crate): `subtle::ConstantTimeEq`, `zeroize::{Zeroize, ZeroizeOnDrop}`, `serde::{Serialize, Deserialize}`.

---

### OutputFormat

Canonical output format for reports and CLI output. Engine crate only (depends on `clap` behind `cli` feature).

**Variants** (8): `Pretty` (default), `Json`, `Compact`, `Html`, `Csv`, `Sarif`, `Junit`, `Markdown` — `:106-116`

**Derives**: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`, `Default` — `:103`. Conditional: `ValueEnum` when `feature = "cli"` — `:104`.

**Serde**: `#[serde(rename_all = "lowercase")]` — `:105`

**Impls**:

| Impl | Source | Notes |
|------|--------|-------|
| `OutputFormat::parse_or_default(s)` | `:169` | Parse with fallback to `Pretty` |
| `Display` | `:174` | Lowercase: `"pretty"`, `"json"`, etc. |
| `FromStr` | `:189` | Case-insensitive; unknown → `Err(String)` |

---

### ScanProfile

Scan profile controlling pipeline stage selection and risk budget. Engine crate only.

**Variants** (18): `Quick`, `Endpoint`, `Web`, `Waf`, `Full`, `Api`, `Recon`, `Stealth`, `Deep`, `Vuln`, `Auth`, `DefenseLab`, `SynvoidLocal`, `WafRegression`, `ProtocolEdge`, `NseSafe`, `DbRegression`, `WebProxy` — `:123-142`

**Derives**: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Serialize`, `Deserialize` — `:121`. Conditional: `ValueEnum` when `feature = "cli"` — `:122`. No `Hash`, no `Default`.

**Impls**:

| Impl | Source | Notes |
|------|--------|-------|
| `Display` | `:207` | Lowercase with hyphens: `"defense-lab"`, `"synvoid-local"`, `"waf-regression"`, etc. |
| `FromStr` | `:232` | Case-insensitive. `DbRegression` accepts 3 aliases: `"db-regression"`, `"db_regression"`, `"dbregression"`. `WebProxy` accepts 3: `"web-proxy"`, `"webproxy"`, `"proxy"`. Unknown → `Err(String)`. |
| `requires_private_scope()` | `:263` | Returns `true` for 7 defense-lab profiles: `DefenseLab`, `SynvoidLocal`, `DbRegression`, `WafRegression`, `ProtocolEdge`, `NseSafe`, `WebProxy` |
| `requires_packet_inspection()` | `:277` | Only `ProtocolEdge` |
| `requires_nse()` | `:282` | Only `NseSafe` |
| `max_risk_budget()` | `:287` | `SafeActive`: Quick, ProtocolEdge, NseSafe. `Passive`: Stealth. `Intrusive`: DefenseLab, SynvoidLocal, WafRegression, DbRegression, WebProxy, Endpoint, Web, Waf, Recon, Vuln, Auth. `Stress`: Full, Api, Deep. |
| `operation_mode()` | `:311` | StandardAssessment: 11 profiles. DefenseLab: 7 profiles. |
| `intended_uses()` | `:335` | Maps each profile to 1-2 `IntendedUse` variants |
| `mode_description()` | `:376` | Human-readable: `"DefenseLab mode (max risk: intrusive)"` |

---

### CommonHttpArgs

Shared HTTP client configuration for scan operations.

**Derives**: `Debug`, `Clone`, `Default` — `:147`

**Fields** (13):

| Field | Type | Source |
|-------|------|--------|
| `insecure` | `bool` | `:149` |
| `proxy` | `Option<String>` | `:150` |
| `proxy_auth` | `Option<String>` | `:151` |
| `auth` | `Option<String>` | `:152` |
| `bearer` | `Option<String>` | `:153` |
| `cookie` | `Option<String>` | `:154` |
| `api_key` | `Option<String>` | `:155` |
| `user_agent` | `Option<String>` | `:156` |
| `stealth` | `bool` | `:157` |
| `rate_limit` | `Option<u32>` | `:158` |
| `jitter` | `Option<String>` | `:159` |
| `auth_context` | `Option<String>` | `:160` |
| `auth_role` | `Option<String>` | `:161` |

---

### check_config_file_permissions()

Utility function that warns if a config file has overly permissive permissions.

**Platform**: Unix-only (`#[cfg(unix)]`) — `:40`. No-op on non-Unix (`:97-98`).

**Behavior**: Reads mode bits, checks in order of severity:
1. World-writable (`0o002`) → `tracing::warn!` with chmod suggestion
2. Group-writable (`0o020`) → `tracing::warn!`
3. World-readable (`0o004`) → `tracing::warn!`
4. Group-readable (`0o040`) → `tracing::warn!`

Uses `std::os::unix::fs::PermissionsExt` to read mode. Recommended mode: `0o600`.

**Used by**: `crates/eggsec/src/config/loader.rs:9` — called during config file loading.

---

## Integration Points

### Severity — workspace usage (92+ direct imports)

| Consumer | Import path | Purpose |
|----------|-------------|---------|
| `eggsec-output` (7 files) | `eggsec_core::types::Severity` | Finding envelope, SARIF/JUnit/CSV conversion, dedup, trends |
| `eggsec-tool-core` | `eggsec_core::types::Severity` | `From<Severity> for ResponseSeverity` conversion (`finding.rs:167`) |
| `eggsec-db-lab` (8 files) | `eggsec_core::types::Severity` | Finding severity in DB assessment results |
| `eggsec-web-proxy` | `eggsec_core::types::SensitiveString` | Proxy auth credentials |
| `eggsec-mobile-lab` (6 files) | `eggsec_core::types::Severity` | APK/IPA finding severity |
| `eggsec-nse` (3 files) | `eggsec_core::types::Severity` | NSE bridge finding severity |
| `eggsec-python` | `eggsec_core::types::Severity` | Python-side `Severity` ↔ engine `Severity` mapping (`finding.rs:56`) |
| Engine `fuzzer/` (7 files) | `crate::types::Severity` | Finding severity across fuzz engines |
| Engine `scanner/` (5 files) | `crate::types::Severity` | Port/service finding severity |
| Engine `waf/` | `crate::types::Severity` | Re-exported at `waf/types.rs:4` |
| Engine `recon/` (7 files) | `crate::types::SensitiveString` | API keys for recon services |
| Engine `hunt/` (5 files) | `crate::types::Severity` | Authorization/business/race finding severity |
| Engine `compliance/` (4 files) | `crate::types::Severity` | Compliance finding severity |
| Engine `config/` | `crate::types::SensitiveString` | Config secrets (settings, HTTP, scan) |
| Engine `pipeline/` | `crate::types::{CommonHttpArgs, ScanProfile}` | Pipeline stage configuration |
| Engine `cli/mod.rs` | Re-exported at `:570` | `pub use crate::types::{CommonHttpArgs, OutputFormat, ScanProfile}` |
| `eggsec-tui` (10+ files) | `eggsec::types::*` | Scan profile selection, output format, proxy credentials |

### SensitiveString — workspace usage

Primary consumers: `config/settings.rs`, `config/http.rs`, `config/scan.rs`, `recon/runner.rs`, `recon/geolocation.rs`, `recon/threatintel.rs`, `recon/wayback.rs`, `tool/session.rs`, `notify/webhook.rs`, `commands/proxy.rs`, `commands/webhook.rs`, `proxy/mod.rs`, `integrations/{jira,github,gitlab}.rs`, `eggsec-web-proxy/src/{config,socks,http_connect}.rs`, `eggsec-python/src/{config_model,requests,ai_postprocess}.rs`.

### OutputFormat — workspace usage

Consumers: `eggsec-tui` (export, settings, scan tabs), `eggsec/src/pipeline/mod.rs:59`, `eggsec-cli` argument parsing.

### ScanProfile — workspace usage

Consumers: `eggsec/src/pipeline/{mod,stage,session,executor}.rs`, `eggsec/src/dispatch/recon.rs:2`, `eggsec-tui/src/tabs/scan.rs:10`, `eggsec-cli` argument parsing, `crates/eggsec/tests/pipeline_tests.rs`, `crates/eggsec/tests/pipeline_stage_tests.rs`.

---

## Testing

Tests live in both `crates/eggsec-core/src/types.rs:282-477` and `crates/eggsec/src/types.rs:387-649` (the engine crate duplicates core tests plus adds engine-specific tests).

| Test | File | What it verifies |
|------|------|-----------------|
| `severity_ordering` | core `:287`, engine `:392` | `Critical > High`, integer rankings |
| `severity_from_str` | core `:294`, engine `:399` | Case-insensitive parse, `"moderate"` → Medium, unknown → Info |
| `severity_from_cvss` | core `:302`, engine `:407` | Boundary values: 9.5→Critical, 7.0→High, 4.0→Medium, 0.5→Low, 0.0→Info |
| `severity_display_is_uppercase` | core `:311`, engine `:416` | Display format |
| `severity_as_str_is_lowercase` | core `:317`, engine `:422` | `as_str()` format |
| `sensitive_string_expose` | core `:323`, engine `:428` | `expose_secret()` returns inner value |
| `sensitive_string_into_secret` | core `:329`, engine `:434` | Consuming access |
| `sensitive_string_debug_redacted` | core `:335`, engine `:440` | Debug never leaks secret |
| `sensitive_string_display_redacted` | core `:343`, engine `:448` | Display never leaks secret |
| `sensitive_string_serialize_deserialize` | core `:349`, engine `:454` | Round-trip: plaintext serde |
| `sensitive_string_equality` | core `:358`, engine `:463` | Constant-time comparison |
| `sensitive_string_from_conversions` | core `:367`, engine `:472` | `From<&str>` and `From<String>` |
| `severity_ord_matches_semantic_order` | core `:374`, engine `:479` | Full ordering chain |
| `severity_sorts_correctly` | core `:382`, engine `:487` | Vec sort produces correct order |
| `severity_cvss_boundary_values` | core `:404`, engine `:509` | Edge cases at 8.99/9.0, 6.99/7.0, 3.99/4.0, 0.09/0.1 |
| `sensitive_string_empty` | core `:416`, engine `:521` | Empty string handling |
| `sensitive_string_into_secret_leaves_owned` | core `:425`, engine `:530` | Ownership transfer safety |
| `sensitive_string_for_logging_redacted` | core `:432`, engine `:537` | `for_logging()` toggle |
| `sensitive_string_len` | core `:439`, engine `:544` | Length correctness |
| `sensitive_string_eq_str` | core `:445`, engine `:616` | Cross-type equality |
| `sensitive_string_eq_string` | core `:452`, engine `:623` | Cross-type equality |
| `severity_default_is_info` | core `:459`, engine `:578` | Default trait |
| `severity_display_from_str_roundtrip` | core `:464`, engine `:583` | All 5 variants round-trip |
| `severity_from_cvss_nan` | engine `:641` | NaN → Info |
| `severity_from_cvss_negative` | engine `:646` | Negative → Info |
| `output_format_display` | engine `:550` | All 8 variants |
| `output_format_default_is_pretty` | engine `:558` | Default |
| `output_format_from_str` | engine `:563` | Case-insensitive, unknown → Err |
| `output_format_parse_or_default` | engine `:630` | Fallback behavior |
| `output_format_serde_roundtrip` | engine `:598` | All 8 variants serde |

---

## Invariants & Gotchas

1. **Severity is infallible to parse**: `FromStr::Err = Infallible` (`:98`). Unknown strings silently become `Info`. New code should prefer `s.parse::<Severity>()` over `parse_or_default()`.

2. **SensitiveString serializes in plaintext**: This is intentional for config compatibility (`:222-235`). Config files containing secrets MUST have restrictive filesystem permissions (checked by `check_config_file_permissions()`).

3. **SensitiveString::into_secret() uses std::mem::take**: The inner value is replaced with an empty string before returning, so `ZeroizeOnDrop` does not panic on the moved-out field (`:169-171`).

4. **ScanProfile::FromStr has multiple aliases**: `DbRegression` accepts `"db-regression"`, `"db_regression"`, `"dbregression"`. `WebProxy` accepts `"web-proxy"`, `"webproxy"`, `"proxy"` (`:253-254`).

5. **OutputFormat::FromStr is fallible**: Unlike `Severity`, unknown strings return `Err(String)`. Use `parse_or_default()` for graceful fallback.

6. **ScanProfile has no Default**: Unlike `Severity` and `OutputFormat`, `ScanProfile` does not derive `Default` — callers must explicitly choose a profile.

7. **CommonHttpArgs derives Default, not Serialize/Deserialize**: It's used for CLI arg passing and engine construction, not config persistence.

8. **Core types have zero internal dependencies**: `eggsec-core` depends only on `serde`, `subtle`, `zeroize` — no workspace crates. This is enforced by architecture guards.

---

*Last verified against source: 2026-08-25*
