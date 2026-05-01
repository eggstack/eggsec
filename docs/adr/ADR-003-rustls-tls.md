# ADR-003: rustls Over native-tls (Except NSE)

## Status

Accepted

## Context

We needed a TLS library for HTTPS connections that:
- Works on all platforms without system dependencies
- Provides consistent behavior across platforms
- Avoids OpenSSL dependency conflicts
- Supports modern TLS features

## Decision

We use `rustls` (with `tokio-rustls`) for the main `slapper` crate, but retain `native-tls` (OpenSSL) for `slapper-nse`.

### Main Slapper Crate

Uses `rustls` 0.23 + `tokio-rustls` 0.26:

1. **No System Dependencies**: `rustls` is pure Rust, avoiding OpenSSL installation issues on Windows and macOS.

2. **Consistent Behavior**: Same TLS implementation on all platforms.

3. **Security**: `rustls` has a strong security track record and is maintained by the AWS s2n team.

4. **Insecure Mode for Internal Use**: We use `NoVerifier` for internal TLS connections that bypass verification (with runtime warnings).

### slapper-nse Exception

The NSE module uses `native-tls` because:

1. **Nmap Compatibility**: Nmap scripts expect OpenSSL-based TLS behavior.

2. **System Integration**: Nmap uses OpenSSL for certificate validation.

3. **Script Expectations**: Many NSE scripts assume OpenSSL is available.

## Consequences

- Positive: No OpenSSL dependency hell
- Positive: Consistent TLS behavior across platforms
- Positive: NSE scripts work with standard OpenSSL
- Negative: rustls has different API than OpenSSL
- Negative: Must maintain separate TLS backends for main code and NSE

## References

- `crates/slapper/src/distributed/io.rs` - TLS implementation
- `crates/slapper/src/recon/ssl.rs` - SSL certificate extraction
- `slapper-nse/src/lib.rs` - NSE module TLS
