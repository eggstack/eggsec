# NSE Milestone 3 Phase 04: Network and DNS Capability Wrappers

## Purpose

Migrate NSE helper-side network and DNS operations behind profile-aware capability wrappers with accounting, cancellation checks, scope validation, and report events.

Network helpers are central to NSE compatibility but also high-risk for automated operation. Manual CLI/TUI should remain useful, while agent/CI surfaces must be scoped and bounded.

## Background

Milestone 1 separated manual and automated profiles. Milestone 2 made compatibility/report truthfulness explicit. Milestone 3 Phase 02 introduced the capability context. Phase 04 applies it to TCP, UDP, HTTP, TLS handshakes where relevant, and DNS lookup helper paths.

## Non-Goals

Do not remove manual network capability.

Do not implement a full async networking rewrite.

Do not implement complete Nmap service-probe parity.

Do not migrate filesystem/process helpers here; those belong to Phase 03.

## Target State

By the end of this phase:

- Network helper operations route through capability wrappers or are explicitly documented as deferred.
- AgentSafe network operations require scope validation.
- CiSafe denies external network by default.
- Network operation and byte counters update from helper-side operations.
- DNS lookups are profile-aware and reportable.
- Timeouts/cancellation checks are applied before and after blocking operations.
- Reports include network/DNS denials and warnings.

## Workstream 1: Network Wrapper API

### Proposed API

```rust
pub fn nse_tcp_connect(
    ctx: &NseCapabilityContext,
    host: &str,
    port: u16,
    timeout: Option<Duration>,
) -> LuaResult<NseTcpHandle>;

pub fn nse_udp_socket(
    ctx: &NseCapabilityContext,
    target: &str,
    port: u16,
) -> LuaResult<NseUdpHandle>;

pub fn nse_network_read(
    ctx: &NseCapabilityContext,
    operation: &'static str,
    byte_count: usize,
) -> LuaResult<()>;

pub fn nse_network_write(
    ctx: &NseCapabilityContext,
    operation: &'static str,
    byte_count: usize,
) -> LuaResult<()>;
```

Adapt to existing socket abstractions.

### Required Behavior

- Check cancellation before connect/send/receive and after returning.
- Enforce target/scope policy for automated profiles.
- Enforce network operation limits.
- Enforce byte counters.
- Use configured timeouts where available.
- Emit capability events for allow/deny/failure.

### Acceptance Criteria

- TCP/UDP wrapper APIs exist and are tested.
- Scope denial produces stable reportable errors.

## Workstream 2: DNS Wrapper API

### Proposed API

```rust
pub fn nse_dns_lookup(
    ctx: &NseCapabilityContext,
    name: &str,
    record_type: Option<&str>,
) -> LuaResult<NseDnsLookupResult>;
```

### Required Behavior

- AgentSafe allows only scoped lookups where the target/scope policy permits it.
- CiSafe denies external DNS unless using local fixtures.
- Manual profiles allow with accounting/reporting.
- DNS lookup increments network/DNS counters or capability events.
- DNS failures are represented as ordinary helper errors, not panics.

### Acceptance Criteria

- DNS helper calls cannot bypass policy.
- Tests cover manual allow, agent scoped deny/allow, and CI deny.

## Workstream 3: Migrate Common Network Libraries

### Likely Libraries

Inspect and migrate direct network operations in:

- `socket`
- `comm`
- `http`
- `dns`
- `sslcert`
- `tls`
- protocol libraries such as `ssh`, `ftp`, `smtp`, `smb`, `redis`, `mysql`, `postgres`, etc.

### Migration Strategy

Do not migrate every protocol library in one risky patch. Use priority order:

1. Core socket and comm abstractions.
2. HTTP and DNS.
3. TLS/certificate helpers that open sockets.
4. Highest-use protocol libraries.
5. Lower-use protocol libraries.

After core socket/comm wrappers exist, many protocol libraries may inherit enforcement automatically.

### Acceptance Criteria

- Common network paths go through wrappers.
- Protocol libraries that still bypass wrappers are listed as deferred with risk classification.

## Workstream 4: Scope Validation

### Steps

1. Reuse existing `NseNetworkPolicy` and profile scope inputs where possible.
2. Normalize host/IP targets before policy checks.
3. For hostnames, decide whether policy checks happen before DNS, after DNS, or both.
4. AgentSafe should not allow arbitrary DNS/connection targets outside scope.
5. ManualPermissive should allow with event logging.

### Acceptance Criteria

- AgentSafe rejects obvious out-of-scope network operations.
- ManualPermissive remains useful.
- Scope failures appear in structured reports.

## Workstream 5: Counters and Limits

### Steps

1. Increment `network_operations` on connect/send/receive/DNS operations.
2. Increment byte counters for read/write where available.
3. Respect `max_network_operations` before starting operations.
4. Add stable errors for network operation limit exceeded.
5. Ensure errors set `NseLimitViolation` where appropriate.

### Acceptance Criteria

- Network counters reflect helper-side operations.
- Limit violations are visible in execution stats and report summaries.

## Workstream 6: Tests

Required tests:

- manual TCP connect wrapper decision allowed or attempted with local fixture;
- AgentSafe out-of-scope target denied before network call;
- CiSafe external network denied;
- DNS lookup denied under CI/agent unscoped;
- network operation limit exceeded;
- cancellation before network operation denied;
- report includes network denial event or warning;
- core socket/comm/http helper tests use wrappers.

Prefer local-only fixtures. Do not require public internet in CI.

## Workstream 7: Guards and Docs

### Guards

Add/tighten guards for direct network calls outside wrappers:

- `TcpStream::connect`
- `UdpSocket::bind/connect/send_to/recv_from`
- direct HTTP client sends
- direct DNS resolver calls

Start with warnings for broad legacy hits; convert migrated high-priority modules to failures.

### Docs

Update capability inventory and NSE integration docs with migrated network/DNS coverage.

## Verification

Run:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse network
cargo test -p eggsec-nse --features nse dns
cargo test -p eggsec-nse --features nse,sandbox
bash scripts/check-architecture-guards.sh
```

## Final Acceptance Criteria

Phase 04 is complete when:

- Network/DNS wrapper APIs exist.
- Core socket/comm/HTTP/DNS paths are migrated or clearly deferred.
- Automated profiles enforce scope/denial.
- Network counters and reports reflect helper-side operations.
- No public internet is required for tests.
