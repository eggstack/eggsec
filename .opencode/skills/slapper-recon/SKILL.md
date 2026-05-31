# Slapper Recon Skill

Reconnaissance module workflows and patterns for information gathering.

## Key Components

### Full Recon Pipeline (`run_full_recon` in `runner.rs`)

The full recon pipeline runs 13 tasks in parallel via `tokio::join!`:
```
reverse_dns, geolocation, threat_intel, ssl, whois, subdomain_enum,
dns_records, tech_detection, js_analysis, wayback_check, cloud_detection,
content_analysis, cors_check, email_discovery
```

Sequential dependencies:
- Takeover check runs after subdomain enumeration
- CVE mapping runs after tech detection

### Module Structure (src/recon/)

| Category | Files | Notes |
|----------|-------|-------|
| Network | `dns_records.rs`, `reverse_dns.rs`, `whois.rs`, `geolocation.rs` | DNS, WHOIS, GeoIP |
| Web | `techdetect.rs`, `content.rs`, `js.rs`, `cors.rs` | Tech detection, content discovery |
| Subdomains | `subdomain.rs`, `wayback.rs`, `takeover.rs` | Enumeration, history, takeover |
| Security | `cve.rs`, `secrets.rs`, `ssl.rs`, `threatintel.rs` | CVE, secrets, SSL, threat intel |
| Cloud | `cloud/mod.rs`, `cloud/services.rs`, `cloud/iam.rs`, `cloud/metadata.rs` | AWS/GCP/Azure discovery |
| Email | `email.rs`, `email_security.rs` | Discovery + SPF/DKIM/DMARC |
| Dependency | (removed) | Package scanning |
| Other | `api_schema.rs`, `containers.rs`, `git_secrets.rs` | Feature-gated modules |

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

Note: TLS version and cipher suite detection is not yet implemented - `supported_versions` and `supported_cipher_suites` fields are populated by external tooling.

### Performance

- Use `FxHashMap`/`FxHashSet` instead of `std::collections::HashMap`/`HashSet`
- `CveMapper.cache` uses `FxHashMap` (cve.rs)
- `CveEngine.cve_cache` uses `FxHashMap` (cve_lookup.rs)
- `LOCAL_IP_DATA` in geolocation.rs uses `FxHashMap`
- `WaybackClient.endpoints` uses `FxHashSet`
- `TakeoverDetector.cname_map`/`ns_map` uses `FxHashMap` (takeover.rs:455-456)
- `EmailDiscoveryClient` methods use `FxHashSet` (email.rs:132,155,174)
- `JsAnalyzer` methods use `FxHashSet` (js.rs:229,287)
- `SubdomainEnumerator` methods use `FxHashSet` (subdomain.rs:74,112,158)
- `CorsAnalyzer.findings` uses `FxHashSet` (cors.rs:43)
- `CloudScanner.generate_cloud_names` uses `FxHashSet` (cloud/mod.rs:342)
- `ContainerScanner.check_container_config` uses `FxHashMap` (containers.rs:243)
- `compare_dns_records` uses `FxHashSet` (dns_enhanced.rs:247,252)
- `FullReconResult` callback metadata uses `FxHashMap` (mod.rs:221,253)

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
cargo test --lib -p slapper recon::
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
- `crates/slapper/src/recon/AGENTS.override.md` - Detailed recon module patterns
- `AGENTS.md` - General project guidelines
- `architecture/recon.md` - Architecture documentation
