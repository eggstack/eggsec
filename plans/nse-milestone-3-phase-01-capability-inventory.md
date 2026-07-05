# NSE Milestone 3 Phase 01: Capability Inventory and Risk Classification

## Purpose

Create a complete, actionable inventory of side-effecting NSE Rust helper operations before adding wrappers. This phase classifies helper operations by risk, blocking behavior, profile policy needs, accounting needs, and report needs.

The output is a source-of-truth inventory that drives wrapper migration in later phases.

## Background

Milestone 1 established loader/profile safety. Milestone 2 established truthful registry/report semantics. The remaining known gap is helper-side enforcement: Lua execution hooks can interrupt Lua bytecode, but once a Lua script enters Rust helper code, any blocking filesystem, network, DNS, process, crypto, compression, time, or randomness work must enforce limits and cancellation cooperatively inside the helper path.

## Non-Goals

Do not migrate helper implementations in this phase except for tiny comments or annotations.

Do not remove manual behavior.

Do not introduce broad policy changes.

Do not attempt to classify full Nmap parity.

## Required Output

Create a capability inventory document, likely:

```text
architecture/nse_capability_inventory.md
```

or a section in `architecture/nse_integration.md` if the project prefers a single document.

## Inventory Scope

Inspect all NSE helper/library implementation files under:

```text
crates/eggsec-nse/src/libraries/
crates/eggsec-nse/src/executor*.rs
crates/eggsec-nse/src/*sandbox*.rs
```

Search for direct side effects:

- `std::fs::*`
- `std::process::*`
- `std::net::*`
- `TcpStream`, `UdpSocket`, async network clients
- DNS resolver usage
- HTTP clients
- TLS/crypto calls
- compression/decompression calls
- sleeps/timers/time reads
- randomness sources
- environment access
- filesystem metadata/path probing
- external command execution
- direct blocking loops or large parsing/decompression loops

## Classification Model

For each helper or function, record:

```markdown
| File | Function | Capability | Side Effect | Blocking Risk | Profile Policy | Accounting | Cancellation | Report Event | Notes |
```

Capability classes:

- `filesystem_read`
- `filesystem_write`
- `process_exec`
- `network_tcp`
- `network_udp`
- `dns_resolution`
- `tls_crypto`
- `compression`
- `time_clock`
- `randomness`
- `environment`
- `pure_cpu`
- `unknown`

Blocking risk:

- `none`
- `low`
- `medium`
- `high`
- `unknown`

Profile policy:

- `manual_allowed`
- `manual_prompt_or_warn`
- `agent_deny`
- `agent_allow_if_scoped`
- `ci_deny`
- `ci_allow_local_only`
- `unknown`

Accounting needs:

- `filesystem_operations`
- `filesystem_bytes_read`
- `filesystem_bytes_written`
- `network_operations`
- `network_bytes_read`
- `network_bytes_written`
- `process_operations`
- `crypto_operations`
- `compression_bytes_in/out`
- `none`

## Workstream 1: Static Search Inventory

### Steps

1. Use `rg` to identify all direct side-effect calls.
2. Group by library module and helper function.
3. Mark helper functions already sandboxed or profile-aware.
4. Mark functions that rely only on Lua sandboxing and still need Rust-side checks.
5. Mark unknowns rather than guessing.

### Acceptance Criteria

- Inventory lists all obvious direct side-effect calls.
- Unknowns are explicit.
- No helper class is silently omitted.

## Workstream 2: Risk Prioritization

### Priority Order

Prioritize migration based on risk:

1. Process execution and shelling out.
2. Filesystem write/delete/rename.
3. Filesystem read outside explicit roots.
4. Network TCP/UDP operations.
5. DNS lookups and external resolver calls.
6. Compression/decompression on untrusted inputs.
7. Crypto/TLS operations that may block or allocate heavily.
8. Time/randomness/environment reads.
9. Pure CPU helpers.

### Acceptance Criteria

- Inventory includes a migration priority per helper family.
- Later phases can select files without redoing analysis.

## Workstream 3: Policy Mapping

### Steps

1. Map capability classes to profile behavior:
   - ManualPermissive: allow with accounting/reporting unless dangerous process/filesystem mutation needs warning.
   - ManualStrict: allow only within configured roots/scopes.
   - AgentSafe: deny by default unless scoped and bounded.
   - CiSafe: deny network/process and allow only local deterministic filesystem fixtures.
   - CompatibilityLab: allow controlled local compatibility fixtures.
2. Document expected fallback behavior for denied helpers.
3. Mark helpers that must emit report diagnostics when denied.

### Acceptance Criteria

- Every capability class has preliminary profile behavior.
- Policy mapping does not change current behavior yet; it informs later phases.

## Workstream 4: Test Planning

For each high-risk helper class, define at least one future test:

- manual allowed path;
- agent denied path;
- CI denied/local-only path;
- cancellation before call;
- limit exceeded path;
- report event emitted.

### Acceptance Criteria

- The inventory includes a test matrix or links to phase-specific tests.

## Verification

Run:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
bash scripts/check-architecture-guards.sh
```

## Final Acceptance Criteria

Phase 01 is complete when:

- A capability inventory exists in repo docs.
- Helper side effects are classified by capability, risk, policy, accounting, cancellation, and reporting needs.
- High-priority migration order is clear.
- Later phases can migrate wrappers without redoing broad inventory work.
