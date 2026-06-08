# Core Types

## Overview

Shared types for the Slapper workspace. `Severity` and `SensitiveString` are defined in `crates/slapper-core/src/types.rs` and re-exported by `crates/slapper/src/types.rs`. `OutputFormat` and `check_config_file_permissions` remain in the main crate.

## Key Types

### Severity

Canonical severity rating for findings and vulnerabilities. Used by the fuzzer, WAF detector, recon, output, and tool modules.

**Variants:** `Critical`, `High`, `Medium`, `Low`, `Info` (default)

**Key methods:**
- `parse_or_default(s)` - parse from string, defaults to `Info`
- `from_cvss(score)` - derive from CVSS score (>=9.0=Critical, >=7.0=High, >=4.0=Medium, >=0.1=Low)
- `as_str()` - lowercase string representation
- `as_int()` - integer ranking (Critical=4, High=3, Medium=2, Low=1, Info=0)
- `cvss_color()` - color emoji for terminal display

**Trait implementations:**
- `Ord` / `PartialOrd` - based on `as_int()` ranking
- `Display` - uppercase representation (`"CRITICAL"`, `"HIGH"`, etc.)
- `FromStr` - case-insensitive parsing; also accepts `"moderate"` as `Medium`
- `Serialize` / `Deserialize` - lowercase serde rename
- `Default` - `Info`

### SensitiveString

Zeroized credential wrapper for passwords, API keys, and tokens.

- `Zeroize` + `ZeroizeOnDrop` - memory zeroized on drop
- Constant-time comparison (`ct_eq`) to prevent timing attacks
- `Hash` intentionally **not** implemented (prevents correlation attacks)
- `expose_secret()` / `into_secret()` - access inner value
- `log_secret()` / `for_logging()` - safe logging with redaction
- Serializes in plaintext (intentional for config compatibility)

**Additional methods:**
- `new(s)` - create from any `Into<String>`
- `len()` / `is_empty()` - length queries
- `as_bytes()` - raw byte access

**Trait implementations:**
- `Clone`, `Zeroize`, `ZeroizeOnDrop`
- `PartialEq` / `Eq` - constant-time comparison
- `Debug` / `Display` - always redacted (`"[REDACTED]"`)
- `Serialize` / `Deserialize` - plaintext (intentional)
- `From<String>` / `From<&str>` - convenience conversions

### OutputFormat

Canonical output format for reports. Variants: `Pretty` (default), `Json`, `Compact`, `Html`, `Csv`, `Sarif`, `Junit`, `Markdown`.

Derives `ValueEnum` for clap CLI integration. Implements `Display` (lowercase).

### check_config_file_permissions()

Utility function that warns if a config file has overly permissive permissions (world-readable or group-readable).

**Platform note:** Unix-only (`#[cfg(unix)]`). Uses `std::os::unix::fs::PermissionsExt` to read mode bits. No-op on non-Unix platforms.
