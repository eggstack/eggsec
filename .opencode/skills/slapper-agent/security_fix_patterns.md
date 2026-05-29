---
name: security_fix_patterns
description: Patterns for implementing security fixes in Slapper
triggers:
  - security fix
  - vulnerability fix
  - constant-time comparison
  - side-channel attack
metadata:
  category: code_quality
  tools: [all]
  scope: implementation
---

## Overview

This skill documents common security fix patterns implemented across the Slapper codebase. Future agents should follow these patterns when implementing similar fixes.

## Security Patterns Implemented

### 1. Constant-Time Comparison

**Issue**: Using `.unwrap_u8() == 1` on `ConstantTimeEq::Choice` enables side-channel attacks through branch prediction.

**Vulnerable code**:
```rust
Some(v) if key.as_bytes().ct_eq(v.as_bytes()).unwrap_u8() == 1 => Ok(()),
```

**Secure code**:
```rust
Some(v) if bool::from(key.as_bytes().ct_eq(v.as_bytes())) => Ok(()),
```

**Files**: `tool/protocol/rest.rs`, `tool/protocol/ai_routes.rs`, `tool/protocol/agent_routes.rs`, `tool/protocol/openai/handlers.rs`, `tool/protocol/mcp/auth.rs`, `tool/protocol/grpc.rs`

### 2. TOCTOU Race Condition Prevention

**Issue**: Separate existence check then read operation allows race between check and use.

**Vulnerable code**:
```rust
if !path.exists() {
    return Ok(SlapperConfig::default());
}
let content = fs::read_to_string(&path)
```

**Secure code**:
```rust
let canonical_path = path.canonicalize().map_err(|e| {
    anyhow::anyhow!("Failed to canonicalize config path '{}': {}", path.display(), e)
})?;
let content = fs::read_to_string(&canonical_path)
```

**Files**: `config/loader.rs`

### 3. Path Traversal Prevention

**Issue**: Reading files from directories without canonicalization allows path traversal.

**Pattern**: Use `canonicalize()` and validate paths start with base directory.

```rust

### 4. Silent Data Loss Prevention

**Issue**: Using `unwrap_or_default()` on serialization failures loses data silently.

**Vulnerable code**:
```rust
pub fn to_json_line(&self) -> String {
    serde_json::to_string(self).unwrap_or_default() + "\n"
}
```

**Secure code**:
```rust
pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
    serde_json::to_string(self).map(|s| s + "\n")
}
```

**Files**: `tool/response.rs`

### 5. TLS Verification Bypass with Warning

**Issue**: Disabling TLS verification without warning logging.

**Pattern**: Use centralized `create_insecure_http_client()` from `utils/http.rs` which logs warnings, or add explicit `tracing::warn!()` before bypass.

```rust
tracing::warn!(
    "TLS certificate verification disabled. This is insecure and should only \
     be used in isolated testing environments."
);
client = client.danger_accept_invalid_certs(true);
```

**Files**: `scanner/cms/mod.rs`, `scanner/cms/joomla.rs`, `scanner/cms/drupal.rs`, `scanner/templates/executor.rs`, `waf/detector/compare.rs`, `stress/http.rs`, `proxy/health.rs`, `recon/ssl_audit.rs`

### 6. Credential Exposure in Logging

**Issue**: `to_url()` exposes credentials in URLs for logging/display.

**Pattern**: Add `to_log_key()` method that redacts passwords.

```rust
pub fn to_log_key(&self) -> String {
    match (&self.username, &self.password) {
        (Some(user), Some(_)) => format!("{}://{}:***@{}:{}", scheme, user, self.address, self.port),
        _ => self.to_url(),
    }
}
```

**Files**: `proxy/config.rs`, `proxy/pool.rs`, `proxy/health.rs`, `proxy/rotator.rs`, `commands/handlers/stress.rs`

### 7. Formula Injection Prevention

**Issue**: CSV output not protected against fullwidth Unicode bypass (U+FF1D =, U+FF0B +, etc.).

**Pattern**: Normalize to NFKC form before checking formula characters.

```rust
use unicode_normalization::UnicodeNormalization;
let normalized: String = s.nfkc().collect();
if normalized.starts_with('=') || normalized.starts_with('+') { ... }
```

### 8. IMAP Injection Prevention

**Issue**: User input concatenated directly into IMAP commands without escaping.

**Pattern**: Use per-RFC 3501 escaping function.

```rust
fn escape_imap_quoted(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    for ch in s.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\r' | '\n' => {},  // Strip these
            c => result.push(c),
        }
    }
    result
}
```

**Files**: `slapper-nse/src/libraries/imap.rs`

### 9. HMAC Signing for Webhooks

**Issue**: Webhooks send raw secret in header instead of HMAC signature.

**Pattern**: Use HMAC-SHA256 signing like agent alerts.

```rust
type HmacSha256 = Hmac<Sha256>;
let mut mac = HmacSha256::new_from_slice(secret.expose_secret().as_bytes())
    .map_err(|e| format!("HMAC error: {}", e))?;
let canonical_json = serde_json::to_string(payload).map_err(...)?;
mac.update(canonical_json.as_bytes());
let result = mac.finalize();
let signature = format!("sha256={}", hex::encode(result.into_bytes()));
request = request.header("X-Signature-256", signature);
```

**Files**: `notify/webhook.rs`, `agent/alerts/routing.rs`

### 10. Error Sanitization Expansion

**Issue**: Stack trace patterns miss Rust panics, Python tracebacks, Go panics, Windows paths.

**Pattern**: Add language-specific patterns:

```rust
static RUST_PANIC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"thread\s+'[^']+'\s+panicked\s+at").unwrap()
});

static PYTHON_TRACEBACK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"Traceback \(most recent call last\):").unwrap()
});

static GO_PANIC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(panic:\s+runtime\s+error:|goroutine\s+\d+\s+\[)").unwrap()
});

static WINDOWS_PATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z]:\\[\w\\]+").unwrap()
});
```

**Files**: `utils/error.rs`
use unicode_normalization::UnicodeNormalization;
let normalized: String = s.nfkc().collect();
// Then check normalized string for formula chars (=, +, -, @)
```

**Files**: `output/escape.rs`

### 8. Atomic Operations for Counters

**Issue**: TOCTOU race between reading and incrementing counters with multiple mutex acquisitions.

**Vulnerable pattern**:
```rust
let count = *results_count.lock().await;  // Lock #1
if count >= limit {
    false
} else {
    *results_count.lock().await += 1;      // Lock #2
    true
}
```

**Secure pattern**: Use `AtomicU64::fetch_add()` for atomic check-and-increment.

```rust
Some(limit) => {
    let old = results_count.fetch_add(1, Ordering::Relaxed);
    old < limit
}
```

**Files**: `scanner/ports/mod.rs`, `scanner/fingerprint.rs`, `scanner/endpoints.rs`

## Verification Commands

After implementing security fixes:
```bash
cargo test --lib -p slapper
cargo clippy --lib -p slapper
cargo check --lib -p slapper --features rest-api,ai-integration
```

## Related Skills

- `code_quality_patterns` - Performance patterns (FxHashMap, parking_lot, etc.)