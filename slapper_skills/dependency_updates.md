---
name: dependency_updates
description: "Dependency update patterns for Slapper including version upgrades and breaking changes"
triggers:
  - dependency update
  - version upgrade
  - pyo3 upgrade
  - magnus upgrade
  - mlua upgrade
  - serde_yaml
  - cargo update
  - breaking changes
metadata:
  category: maintenance
  tools: [build]
  scope: internal
---

## Overview

Slapper follows a systematic approach to dependency updates, prioritizing security fixes and maintaining compatibility.

## Current Dependency Versions

| Dependency | Current Version | Location |
|------------|-----------------|----------|
| pyo3 | 0.28 | crates/slapper-plugin/Cargo.toml |
| magnus | 0.8.2 | crates/slapper-plugin/Cargo.toml, crates/slapper-ruby/Cargo.toml |
| mlua | 0.11.6 | crates/slapper-nse/Cargo.toml |
| serde_yaml_neo | 0.11 | crates/slapper/Cargo.toml |
| pem | 3 | crates/slapper/Cargo.toml (replaces rustls-pemfile) |

## Version Upgrade Patterns

### pyo3 0.25 → 0.28

**Breaking changes:**
- `Python::with_gil` → `Python::attach` (0.26+)
- GIL lifetime constraints tightened

**Code changes required:**
```rust
// Before
Python::with_gil(|py| { ... })

// After
Python::attach(|py| { ... })
```

**Type inference fixes:**
```rust
// When iterating over &[&str], push dereferenced value
suspicious_found.push(*pattern);  // not pattern
```

### magnus 0.8 → 0.8.2

**Breaking changes:**
- `eval::<()>()` removed - use `let _: Value = eval(...)`
- Hash access: `RHash::lookup::<_, Value>(key)` not `funcall("get", ...)`
- Array iteration: `RArray::each()` yields `Result<Value>`

### mlua 0.11 → 0.11.6

**Breaking changes:**
- Patch release - minimal breaking changes
- May require fixing moved value issues with closures

### serde_yaml → serde_yaml_neo

**Note:** serde_yaml is deprecated, use serde_yaml_neo as drop-in replacement:
```toml
serde_yaml_neo = "0.11"
```

```rust
// Import change only
use serde_yaml_neo::Value;  // instead of serde_yaml::Value
```

### rustls-pemfile → pem crate

**Issue:** rustls-pemfile is unmaintained (RUSTSEC-2025-0134)

**Migration:**
```toml
# Remove rustls-pemfile from dependencies
# Add pem crate (already available as transitive dependency via rcgen)
pem = "3"
```

```rust
// Certificate parsing
use pem::{parse_many, Pem};
let certs: Vec<Pem> = parse_many(cert_pem)
    .into_iter()
    .filter(|p| p.tag() == "CERTIFICATE")
    .collect();

// Private key parsing (different tags for different key types)
use rustls_pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer, PrivatePkcs1KeyDer, PrivateSec1KeyDer};
for pem in parse_many(key_pem) {
    match pem.tag() {
        "PRIVATE KEY" => PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pem.contents().to_vec())),
        "RSA PRIVATE KEY" => PrivateKeyDer::Pkcs1(PrivatePkcs1KeyDer::from(pem.contents().to_vec())),
        "EC PRIVATE KEY" | "ECDSA PRIVATE KEY" => PrivateKeyDer::Sec1(Sec1KeyDer::from(pem.contents().to_vec())),
        _ => continue,
    }
}
```

## Verification Commands

```bash
# Check specific crate
cargo check -p slapper-plugin --features python-plugins

# Check slapper-nse
cargo check -p slapper-nse --features nse

# Run tests
cargo test --lib -p slapper

# Clippy check
cargo clippy --lib -p slapper
```

## Triggers

Keywords: dependency update, version upgrade, pyo3 upgrade, magnus upgrade, mlua upgrade, serde_yaml, cargo update, breaking changes

## References

- `crates/slapper-plugin/Cargo.toml` - Python/Ruby plugin dependencies
- `crates/slapper-nse/Cargo.toml` - NSE dependencies
- `crates/slapper/Cargo.toml` - Main dependencies
