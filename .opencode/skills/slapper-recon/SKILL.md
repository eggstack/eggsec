# Slapper Recon Skill

Reconnaissance module workflows and patterns for information gathering.

## Key Types and Patterns

### Auth Testing
`recon/auth/` - Multi-protocol authentication testing:
- `ssh_auth` - SSH authentication testing
- `ftp_auth` - FTP authentication testing
- `smtp_auth` - SMTP authentication testing

### Dependency Scanning
`recon/dependency_scan/` - Split by ecosystem:
- `npm` - npm package scanning
- `cargo` - Rust cargo scanning
- `go` - Go module scanning

### SSL/TLS
`recon/ssl.rs` uses `rustls_pki_types::CertificateDer` for cert extraction.

**Certificate Info Extraction**: The `extract_certificate_info()` function parses PEM data to extract:
- `subject` - Certificate subject
- `issuer` - Certificate issuer
- `valid_from` / `valid_until` - Validity dates (RFC3339 format)
- `serial_number` - Serial number
- `is_expired` - Boolean, computed from validity dates
- `days_until_expiry` - Computed from `valid_until`
- `subject_alternative_names` - SAN entries

```rust
if let Ok(pem_data) = pem::parse(der_bytes) {
    let pem_str = String::from_utf8_lossy(pem_data.contents());
    // Parse fields from PEM contents
}
```

## Testing

### Running Recon Tests
```bash
cargo test --lib -p slapper recon::
```

### Writing Tests
Follow existing test patterns in `recon/` modules, testing auth, dependency scanning, and SSL/TLS logic.

## Common Tasks

### Adding a New Auth Protocol Test
1. Create module in `recon/auth/`
2. Implement authentication testing logic
3. Add tests for new protocol

### Adding Dependency Scanning for New Ecosystem
1. Create module in `recon/dependency_scan/`
2. Implement package scanning logic
3. Add tests for new ecosystem

## Resources
- `crates/slapper/src/recon/AGENTS.override.md` - Detailed recon patterns
- `AGENTS.md` - General project guidelines
- `ARCHITECTURE.md` - Overall design
