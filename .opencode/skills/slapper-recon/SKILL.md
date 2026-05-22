# Slapper Recon Skill

Reconnaissance module workflows and patterns for information gathering.

## Key Components

### Full Recon Pipeline (`run_full_recon` in `runner.rs`)

The full recon pipeline runs 14 tasks in parallel via `tokio::join!`:
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
| Dependency | `dependency_scan/npm/`, `dependency_scan/cargo/`, `dependency_scan/go/` | Package scanning |
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
- `LOCAL_IP_DATA` in geolocation.rs uses `FxHashMap`
- `WaybackClient.endpoints` uses `FxHashSet`

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
