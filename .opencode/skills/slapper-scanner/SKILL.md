# Slapper Scanner Skill

Port scanning and endpoint discovery module workflows and patterns.

## Key Types and Patterns

### Port Scanning
- `scanner/ports/mod.rs` - Main port scanning logic
- `scanner/ports/spoofed.rs` - Raw socket scanning (feature-gated behind `stress-testing`)
- `scan_ports()` delegates to `spoofed::scan_ports_spoofed()` when spoof enabled
- Packet trace uses `OnceLock<Mutex<File>>` for thread-safe file writing

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
