# Recon Module Override

Specialized guidance for the reconnaissance module.

## Auth Testing

`recon/auth/` - Multi-protocol authentication testing:
- `ssh_auth` - SSH authentication testing
- `ftp_auth` - FTP authentication testing
- `smtp_auth` - SMTP authentication testing

## Dependency Scanning

`recon/dependency_scan/` - Split by ecosystem:
- `npm` - npm package scanning
- `cargo` - Rust cargo scanning
- `go` - Go module scanning

## SSL/TLS

`recon/ssl.rs` uses `rustls_pki_types::CertificateDer` for cert extraction