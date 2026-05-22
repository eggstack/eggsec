# Slapper Scanner Skill

Port scanning and endpoint discovery module workflows and patterns.

## Key Types and Patterns

### Port Scanning
- `scanner/ports/mod.rs` - Main port scanning logic
- `scanner/ports/spoofed.rs` - Raw socket scanning (feature-gated behind `stress-testing`)
- `scan_ports()` delegates to `spoofed::scan_ports_spoofed()` when spoof enabled
- Packet trace uses `OnceLock<Mutex<File>>` for thread-safe file writing

### CLI Integration
Port scanning is invoked via CLI commands:
- `slapper scan-ports <host>` - TCP port scanning
- `slapper scan-endpoints <url>` - HTTP endpoint discovery
- `slapper fingerprint <host>` - Service fingerprinting

Arguments are defined in `cli/scan.rs` (`PortScanArgs`, `EndpointScanArgs`, `FingerprintArgs`).

Handlers are in `commands/handlers/scan.rs`:
- `handle_scan_ports()` - Port scanning entry point
- `handle_scan_endpoints()` - Endpoint discovery entry point
- `handle_fingerprint()` - Service fingerprinting

### CLI Argument Patterns

**PortScanArgs key fields:**
- `host: String` - Target IP or hostname
- `ports: String` - Port range (e.g., "1-1024" or "22,80,443")
- `source_ip: Option<String>` - Source IP for spoofing
- `spoof_range: Option<String>` - Spoof IP range
- `source_port: Option<u16>` - Source port
- `timeout: u64` - Scan timeout in seconds

**EndpointScanArgs key fields:**
- `url: String` - Target base URL
- `wordlist: Option<String>` - Path to wordlist file
- `spoof_ip: Option<String>` - Spoof source IP
- `decoy: Option<String>` - Decoy IP for stealth

### Endpoint Discovery
`scanner/endpoints.rs` handles HTTP endpoint discovery.

### Templates
`scanner/templates/` - Nuclei-style template engine.

### Fingerprinting
`scanner/fingerprint.rs` and `scanner/udp_fingerprint.rs` for service detection.

## Testing

### Running Scanner Tests
```bash
cargo test --lib -p slapper scanner::
```

### Writing Tests
Follow existing test patterns in `scanner/` modules, testing port scanning, endpoint discovery, and fingerprinting logic.

## Common Tasks

### Adding a New Port Scan Type
1. Implement scan logic in `scanner/ports/`
2. Gate raw socket features behind `stress-testing` feature flag
3. Use `OnceLock<Mutex<File>>` for thread-safe packet tracing
4. Add tests for new scan type

### Adding Endpoint Discovery Rules
1. Update `scanner/endpoints.rs` with new discovery logic
2. Test endpoint extraction from HTTP responses

## Resources
- `crates/slapper/src/scanner/AGENTS.override.md` - Detailed scanner patterns
- `AGENTS.md` - General project guidelines
- `ARCHITECTURE.md` - Overall design
