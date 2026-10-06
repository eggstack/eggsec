---
name: eggsec-recon
description: "Reconnaissance module for information gathering - use when working with DNS, WHOIS, subdomain enumeration, technology detection, SSL analysis, threat intel, cloud detection, or email discovery."
---

# Eggsec Recon Skill

Reconnaissance module workflows and patterns for information gathering.

## Key Components

### Full Recon Pipeline (`run_full_recon` in `runner.rs`)

The full recon pipeline runs 13 tasks in parallel via `tokio::join!`:
```
reverse_dns, geolocation, threat_intel, ssl, whois, subdomain_enum,
dns_records, tech_detection, js_analysis, wayback_check,
content_analysis, cors_check, email_security
```

Sequential dependencies:
- Takeover check runs after subdomain enumeration
- CVE mapping runs after tech detection
- Cloud asset enumeration (`run_cloud_detection`, `cloud` feature) runs sequentially after the join block

### Module Structure (src/recon/)

| Category | Files | Notes |
|----------|-------|-------|
| Network | `dns_records.rs`, `reverse_dns.rs`, `whois.rs`, `geolocation.rs` | DNS, WHOIS, GeoIP |
| Web | `techdetect.rs`, `content.rs`, `js.rs`, `cors.rs` | Tech detection, content discovery |
| Subdomains | `subdomain.rs`, `wayback.rs`, `takeover.rs` | Enumeration, history, takeover |
| Security | `cve.rs`, `ssl.rs`, `threatintel.rs` | CVE, SSL, threat intel |
| Cloud | `cloud/mod.rs`, `cloud/services.rs`, `cloud/iam.rs`, `cloud/metadata.rs` | AWS/GCP/Azure discovery |
| Email | `email.rs`, `email_security.rs` | Discovery + SPF/DKIM/DMARC |
| Orchestration | `runner.rs`, `spinner.rs` | `run_full_recon` pipeline, progress spinner |
| Dependency | (removed) | Package scanning |
| Other | `api_schema.rs`, `containers.rs`, `git_secrets.rs` | Feature-gated modules |

`recon/mod.rs` declares **22** public modules. Seven further files are
intentionally *detached* (declared nowhere, included via explicit paths):
`asn.rs`, `cve_lookup.rs`, `dns_enhanced.rs`, `ftp_auth.rs`, `smtp_auth.rs`,
`ssh_auth.rs`, `ssl_audit.rs`.

Secret detection (`secrets.rs`) moved to the leaf crate `eggsec-secrets` in Phase G and is
re-exported at `eggsec::recon::secrets`; `git_secrets.rs` stays here because it is
`std::process::Command` orchestration, not pattern matching.

### Key Types

- `FullReconResult` - Aggregated results with error tracking
- `ReconStep<T>` - Graceful degradation enum (Skipped/Completed/Failed)
- `TechStack` - Detected technologies grouped by category
- `CveMapper` - CVE mapping with built-in database + NVD API cache

### SSL/TLS

`recon/ssl.rs` uses `rustls_pki_types::CertificateDer` for cert extraction.

**Certificate Info Extraction**: The `extract_certificate_info()` function parses PEM data:
```rust
if let Ok(pem_data) = pem::parse(der_bytes) {
    let pem_str = String::from_utf8_lossy(pem_data.contents());
    // Parse fields from PEM contents
}
```

Note: TLS version and cipher suite detection is **not** implemented. The
`supported_versions` / `supported_cipher_suites` fields are declared but left
empty in the analysis path (`ssl.rs:63-64`), so the SSLv3 / TLSv1.0 / TLSv1.1
weakness checks that read them (`ssl.rs:196-212`) never fire.

### Performance

- Use `FxHashMap`/`FxHashSet` instead of `std::collections::HashMap`/`HashSet`
- `CveMapper.cache` uses `FxHashMap` (cve.rs)
- `CveEngine.cve_cache` uses `FxHashMap` (cve_lookup.rs)
- `LOCAL_IP_DATA` in geolocation.rs uses `FxHashMap`
- Wayback endpoint dedupe uses `FxHashSet` (local `endpoints` set in wayback.rs)
- `TakeoverDetector` CNAME/NS maps use `FxHashMap` (takeover.rs)
- Email discovery de-dupe sets use `FxHashSet` (email.rs)
- `JsAnalyzer` endpoint/URL sets use `FxHashSet` (js.rs)
- `SubdomainEnumerator` per-source subdomains use `FxHashSet` (subdomain.rs)
- `CorsAnalyzer` findings use `FxHashSet` (cors.rs)
- `CloudScanner.generate_cloud_names` uses `FxHashSet` (cloud/mod.rs)
- Container config scans use `FxHashMap` (containers.rs)
- `compare_dns_records` uses `FxHashSet` (dns_enhanced.rs)
- `FullReconResult` callback metadata uses `FxHashMap` (mod.rs)

### Notable Bug Fixes

### 2026-05-28
- **20 instances of `unwrap_or_default()`** - Replaced with explicit match with `tracing::debug` across 12 files (cve_lookup.rs, containers.rs, email.rs, js.rs, cors.rs, reverse_dns.rs, ssl_audit.rs, cloud/storage_test.rs, asn.rs, techdetect.rs, threatintel.rs)

### 2026-05-23
- **geolocation.rs:308** - CIDR mask calculation was incorrect. Fixed to proper CIDR mask calculation.
- **smtp_auth.rs:248,256,285** - Base64 API used incorrect trait method syntax.
- **subdomain.rs:111,151** - Silent error suppression with `unwrap_or_default()` changed to explicit match with tracing.
- **api_schema.rs:115** - Silent error suppression on response body read changed to explicit match.

## Testing

### Running Recon Tests
```bash
cargo test --lib -p eggsec recon::
```

### Test Module Synchronization

The test `recon_modules_match_filesystem` (mod.rs) validates that `pub mod` declarations match the filesystem. Detached modules are explicitly excluded:
```rust
let intentionally_detached: BTreeSet<String> = [
    "asn", "cve_lookup", "dns_enhanced",
    "ftp_auth", "smtp_auth", "ssh_auth", "ssl_audit",
].into_iter().map(str::to_string).collect();
```

## Resources
- `crates/eggsec/src/recon/AGENTS.override.md` - Detailed recon module patterns
- `AGENTS.md` - General project guidelines
- `architecture/recon.md` - Architecture documentation
