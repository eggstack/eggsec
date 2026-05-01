# Scanner Module Override

Specialized guidance for the port scanning and endpoint discovery module.

## Port Scanning

- `scanner/ports/mod.rs` - Main port scanning logic
- `scanner/ports/spoofed.rs` - Raw socket scanning (feature-gated behind `stress-testing`)
- `scan_ports()` delegates to `spoofed::scan_ports_spoofed()` when spoof enabled
- Packet trace uses `OnceLock<Mutex<File>>` for thread-safe file writing

## Endpoint Discovery

`scanner/endpoints.rs` handles HTTP endpoint discovery

## Templates

`scanner/templates/` - Nuclei-style template engine

## Fingerprinting

`scanner/fingerprint.rs` and `scanner/udp_fingerprint.rs` for service detection