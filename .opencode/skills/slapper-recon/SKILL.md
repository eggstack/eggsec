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
