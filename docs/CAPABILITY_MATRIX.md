# Eggsec Capability Matrix

> **Canonical source**: This matrix is derived from `DomainDescriptor` and `OperationMetadata` in the
> `eggsec` crate. Edit metadata in `crates/eggsec/src/domain/mod.rs` and `crates/eggsec/src/config/policy.rs`
> rather than editing this file directly. Tests validate consistency between this matrix and the metadata.
>
> See [METADATA_OWNERSHIP.md](METADATA_OWNERSHIP.md) for the update workflow and ownership model.

## Standalone Operations

These operations are registered in `ALL_OPERATION_METADATA` and are not part of a specific domain.
They are available across all surfaces where their exposure flags permit.

| Operation | Display Name | Risk | Capabilities | Feature | CLI | TUI | MCP/API | REST | Agent | Dry-Run | Baseline | Strict | Scope |
|-----------|-------------|------|-------------|---------|-----|-----|---------|------|-------|---------|----------|--------|-------|
| `recon` | Reconnaissance | SafeActive | PassiveFingerprint | — | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `scan-ports` | Port Scan | SafeActive | ActiveProbe | — | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `scan-endpoints` | Endpoint Discovery | SafeActive | Crawl | — | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `fingerprint` | Service Fingerprint | SafeActive | ActiveProbe | — | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `fuzz` | Fuzzing | Intrusive | HttpFuzzLowImpact | — | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `waf-detect` | WAF Detection | SafeActive | WafDetect | — | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `waf-bypass` | WAF Bypass Simulation | Intrusive | WafBypassSimulation | — | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `waf-stress` | WAF Stress Test | StressTest | WafStressTest | — | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `load-test` | Load Test | LoadTest | LoadTest | — | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `stress-test` | Stress Test | StressTest | WafStressTest | — | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `packet` | Raw Packet | RawPacket | RawPacketProbe | — | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `graphql` | GraphQL Fuzzing | Intrusive | HttpFuzzLowImpact | — | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `oauth` | OAuth Testing | CredentialTesting | CredentialTesting | — | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `auth-test` | Authentication Testing | CredentialTesting | CredentialTesting | — | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `nse` | NSE Scripts | SafeActive | NseSafe | `nse` | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `db-pentest` | Database Pentesting | DbPentest | DatabaseAssessment | `db-pentest` | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `c2` | C2 Simulation | C2Operation | C2Simulation | `c2` | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `proxy-intercept` | Traffic Interception | TrafficInterception | TrafficInterception | `web-proxy` | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `wireless` | Wireless Scanning | SafeActive | PassiveFingerprint | `wireless` | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `hunt` | Vulnerability Hunting | SafeActive | ActiveProbe | `advanced-hunting` | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `browser` | Headless Browser | SafeActive | ActiveProbe | `headless-browser` | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `compliance` | Compliance Scanning | SafeActive | ActiveProbe | `compliance` | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `storage` | Database Storage | SafeActive | DatabaseAssessment | `database` | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `integrations` | External Integrations | SafeActive | ActiveProbe | `external-integrations` | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `workflow` | Finding Workflow | SafeActive | ActiveProbe | `finding-workflow` | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `vuln` | Vulnerability Management | SafeActive | ActiveProbe | `vuln-management` | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `pipeline` | Security Pipeline | SafeActive | ActiveProbe | — | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `proxy` | Proxy Management | SafeActive | — | — | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `remote` | Remote Execution | RemoteExecution | RemoteExecution | — | Y | Y | Y | Y | Y | always | — | — | explicit scope |
| `search` | Web Search | Passive | — | — | Y | Y | Y | Y | Y | — | — | — | no target |

## Domain Operations

Operations grouped by capability domain. Each domain declares a `DomainDescriptor` with
integrated CLI, TUI, tool, and report adapters.

| Domain | Category | Operation | Risk | Feature | CLI | TUI | MCP/API | Dry-Run | Evidence | Baseline | Strict | Scope | Docs |
|--------|----------|-----------|------|---------|-----|-----|---------|---------|----------|----------|--------|-------|------|
| db-pentest | defense-lab | db-pentest | DbPentest | `db-pentest` | Y | Y | opt-in | always | always | always | Y | explicit scope | DATABASE_PENTEST.md |
| mobile-static | defense-lab | mobile-static | SafeActive | `mobile` | Y | Y | N | always | N | N | Y | explicit scope | MOBILE.md |
| mobile-dynamic | defense-lab | mobile-dynamic | SafeActive | `mobile-dynamic` | Y | Y | N | always | always | always | Y | explicit scope | MOBILE.md |

## Risk Tiers

| Tier | Description | Examples |
|------|-------------|----------|
| Passive | Read-only, no network impact | Recon, Search |
| SafeActive | Low-impact active probes | Port scan, Fingerprint, WAF detect |
| Intrusive | May trigger security alerts | Fuzzing, WAF bypass |
| LoadTest | Sustained load generation | Load testing |
| StressTest | High-volume stress testing | WAF stress, Stress test |
| RawPacket | Raw packet crafting/injection | Packet inspection |
| CredentialTesting | Credential validation | Auth testing, OAuth |
| DbPentest | Direct database security checks | Database pentesting |
| TrafficInterception | MITM proxy operations | Web proxy intercept |
| EvasionTesting | Defense evasion detection | Evasion testing |
| PostExploitation | Post-exploitation simulation | Postex |
| ExploitAdjacent | Near-exploit operations | — |
| C2Operation | Command and control simulation | C2 framework |
| RemoteExecution | Remote command execution | SSH/exec |
| AgentAutonomous | Fully autonomous agent ops | — |

## Capability Definitions

| Capability | Baseline Allowed | Description |
|-----------|-----------------|-------------|
| PassiveFingerprint | Yes | Passive host/service identification |
| ActiveProbe | Yes | Active port and service probing |
| Crawl | Yes | Web crawling and endpoint discovery |
| WafDetect | Yes | WAF detection and identification |
| HttpFuzzLowImpact | No | HTTP parameter fuzzing (low impact) |
| IntrusiveFuzz | No | Aggressive fuzzing payloads |
| WafBypassSimulation | No | WAF bypass technique testing |
| WafStressTest | No | WAF load/stress testing |
| LoadTest | No | HTTP load generation |
| RawPacketProbe | No | Raw packet crafting and injection |
| CredentialTesting | No | Credential validation testing |
| RemoteExecution | No | Remote command execution |
| NseSafe | No | Safe NSE script execution |
| NseIntrusive | No | Intrusive NSE scripts |
| TrafficInterception | No | HTTP/HTTPS traffic interception |
| EvasionTesting | No | Defense evasion technique detection |
| DatabaseAssessment | No | Direct database security checks |
| C2Simulation | No | C2 framework simulation |

## Feature Gating Summary

| Feature | Operations | Status |
|---------|-----------|--------|
| `nse` | nse | Experimental |
| `db-pentest` | db-pentest | Stable |
| `c2` | c2 | Stable |
| `web-proxy` | proxy-intercept | Stable |
| `wireless` | wireless | Stable |
| `mobile` | mobile-static | Stable |
| `mobile-dynamic` | mobile-dynamic | Stable |
| `advanced-hunting` | hunt | Stable |
| `headless-browser` | browser | Stable |
| `compliance` | compliance | Stable |
| `database` | storage | Stable |
| `external-integrations` | integrations | Stable |
| `finding-workflow` | workflow | Stable |
| `vuln-management` | vuln | Stable |

## Updating This Document

This document is validated by `tests/metadata_consistency.rs` against the canonical metadata.
If you add or modify an operation:

1. Update `ALL_OPERATION_METADATA` in `crates/eggsec/src/config/policy.rs`
2. Update `DomainDescriptor` entries in `crates/eggsec/src/domain/mod.rs` (if domain-scoped)
3. Run `cargo test -p eggsec --lib` to verify consistency
4. Update this file to match the metadata
