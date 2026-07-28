# ADR-002: Feature Flag Design Rationale

## Status

Accepted

## Context

Eggsec has multiple optional features that increase binary size and compilation time. We needed a strategy to manage these features while maintaining usability and build flexibility.

## Decision

We use Cargo feature flags with the following design principles:

1. **Granular Features**: Each optional capability is a separate feature flag (e.g., `stress-testing`, `nse`)

2. **Composite Features**: The `full` feature enables most features except those with known issues or special requirements:
   ```toml
   full = ["stress-testing", "packet-inspection", "rest-api", "nse",
           "ai-integration", "websocket", "headless-browser", "database",
           "container", "sbom", "advanced-hunting", "compliance",
           "external-integrations", "finding-workflow", "vuln-management",
           "wireless", "wireless-advanced", "mobile", "mobile-dynamic",
           "db-pentest", "web-proxy", "evasion", "postex", "c2"]
   ```

3. **Explicit Exclusions**: Features intentionally excluded from `full`:
   - `grpc-api`: Requires additional system dependencies
   - `ws-api`: Standalone WebSocket API server
   - `pdf`: PDF report generation
   - `nse-sandbox`: Security feature that may break some NSE scripts
   - `nse-ssh2`: Requires `ssh2` system library
   - `db-pentest-mssql-tiberius`: Requires tiberius driver
   - `db-pentest-mongodb`: Requires MongoDB driver
   - `db-pentest-redis`: Requires Redis driver
   - `db-pentest-mcp`: MCP exposure marker for db-pentest
   - `c2-mcp`: MCP exposure marker for C2
   - `web-proxy-mcp`: MCP exposure marker for web proxy
   - `transparent-proxy`: Linux iptables/nftables specific
   - `dynamic-plugins`: Shared library loading
   - `api-schema`: OpenAPI schema fuzzing
   - `git-secrets`: Git secrets scanning
   - `cloud`: Cloud asset discovery
   - `insecure-tls`: TLS verification bypass

4. **Feature Gating**: Code uses `#[cfg(feature = "...")]` to conditionally compile:
   - Entire module declarations when a module is only available with a feature
   - Function implementations when they depend on optional dependencies
   - Test code when tests require specific features

5. **Documentation**: AGENTS.md documents all feature flags and their interactions

## Consequences

- Positive: Users can build minimal binaries with only needed features
- Positive: CI can test with different feature combinations
- Positive: Optional dependencies don't affect users who don't need them
- Negative: Feature interactions can be complex to debug
- Negative: Some features have hidden dependencies on others

## Feature Flag Reference

| Feature | Description | Default |
|---------|-------------|---------|
| `stress-testing` | ICMP probing, IP spoofing, raw sockets | off |
| `packet-inspection` | Packet capture features | off |
| `rest-api` | REST API server | off |
| `grpc-api` | gRPC API server (NOT in `full`) | off |
| `nse` | Nmap NSE script support | off |
| `nse-sandbox` | NSE sandbox mode (NOT in `full`) | off |
| `nse-ssh2` | NSE with SSH2/libssh2 (NOT in `full`) | off |
| `ai-integration` | AI/LLM features | off |
| `websocket` | WebSocket security testing | off |
| `headless-browser` | DOM XSS and SPA crawling | off |
| `database` | SQLx-based persistence | off |
| `container` | Kubernetes/Docker scanning | off |
| `sbom` | SBOM generation | off |
| `advanced-hunting` | Advanced threat hunting | off |
| `compliance` | Compliance scanning | off |
| `external-integrations` | Jira, GitHub, GitLab connectors | off |
| `finding-workflow` | Finding lifecycle management | off |
| `vuln-management` | Vulnerability triage and CVSS scoring | off |
| `wireless` | WiFi scanning (passive) | off |
| `wireless-advanced` | WiFi active attacks (deauth/disassoc) | off |
| `mobile` | APK/IPA static analysis | off |
| `mobile-dynamic` | Android dynamic testing | off |
| `db-pentest` | Database security assessment | off |
| `db-pentest-mssql-tiberius` | MSSQL driver (NOT in `full`) | off |
| `db-pentest-mongodb` | MongoDB driver (NOT in `full`) | off |
| `db-pentest-redis` | Redis driver (NOT in `full`) | off |
| `web-proxy` | MITM proxy interception | off |
| `evasion` | Evasion technique detection | off |
| `postex` | Post-exploitation simulation | off |
| `c2` | C2 framework simulation | off |
| `ws-api` | WebSocket API server (NOT in `full`) | off |
| `pdf` | PDF report generation (NOT in `full`) | off |
| `full` | Most non-default features combined | off |

## References

- `crates/eggsec/Cargo.toml` - Feature definitions
- `AGENTS.md` - Feature flag documentation
