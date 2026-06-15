# Phase 3: Full C2 & Agentic Red Teaming

**ROADMAP-EVASION-PHASE3.md**

## Executive Summary
This phase implements a lightweight Rust-native Command & Control (C2) framework and advanced agentic red teaming capabilities. It builds on Phase 1 (evasion) and Phase 2 (postex/LOTL) while strictly following Eggsec conventions:
- Feature-gated (`c2` Cargo feature, depending on `postex` and `evasion`).
- Scoped, policy-driven execution (Scope struct, human-in-loop for MCP).
- Dry-run / simulation mode default.
- CLI/TUI for manual control with confirmations.
- MCP/agentic workflows with strict validation.
- MITRE ATT&CK profiles, structured outputs, baselines/regression.
- Lab-only; high-risk ops require explicit `--allow-c2`.

**Goals**: Beaconing agents, campaign orchestration, OPSEC suite for realistic purple teaming and defense validation.

## Prerequisites
1. Phase 1 and Phase 2 completed and merged.
2. Study `crates/eggsec/src/postex/`, `evasion/`, existing agent/MCP patterns in AGENTS.md.
3. Add `c2` feature in Cargo.toml.

## Detailed Step-by-Step Plan (for Smaller Models)

### Step 1: C2 Module Skeleton (1-2 days)
- Create `crates/eggsec/src/c2/` directory.
- Files: `mod.rs`, `beacon.rs`, `tasking.rs`, `agent.rs`, `report.rs`.
- Update `lib.rs`, root Cargo.toml (`c2` feature).
- Add CLI subcommand `c2` with `--dry-run`, `--scope`, `--profile`, `--allow-c2`.
- Basic tests and policy integration (`OperationRisk::C2`).
- Success: `cargo check --features c2,postex,evasion` passes.

### Step 2: Beaconing & Communication (3-4 days)
- In `beacon.rs`: Implement jittered beaconing (HTTP/3, DNS, custom protocols mimicking legit traffic).
- Support payload delivery, exfil (scoped).
- Reuse Phase 1 traffic obfuscation and Phase 2 LOTL.
- Dry-run: Simulate beacon logs with timing.
- Tests: Unit tests for protocol mimicry and jitter.

### Step 3: Tasking & Agent Runtime (3-4 days)
- In `tasking.rs` and `agent.rs`:
  - Task queue (recon, postex, evasion commands).
  - Modular payload execution (BOF-like, in-memory).
  - Agent lifecycle: register, check-in, self-destruct.
- Integration with postex primitives.
- Safety: All tasks validated against Scope and Policy.

### Step 4: Campaign Orchestration & Profiles (2-3 days)
- Support MITRE ATT&CK profiles (e.g., APT29 simulation).
- Automated campaign runner with timelines.
- Human-in-loop gates for MCP; raw CLI confirmation for manual.
- Generate attack graphs and timelines.

### Step 5: OPSEC & Anti-Forensics Suite (2-3 days)
- Parent spoofing, timestomping, log tampering, process masquerading.
- Burn mechanisms, decoys.
- OPSEC scoring in reports.

### Step 6: Integration, Reporting & Polish (3-4 days)
- Bridge to core reporting (SARIF, baselines, HTML).
- TUI dashboard for campaign monitoring.
- MCP tool exposures.
- Update docs: architecture/c2.md, README, AGENTS.md, ROADMAP-EVASION.md.
- Extensive tests (+30), clippy, security review.

### Step 7: Validation & Handoff
- Full test suite.
- Purple-team examples.
- PR with checklist.
- Update CHANGELOG.

## Safety & Risks
- Mandatory scopes, dry-run default, audit logs.
- Lab environments only for real testing.
- Disclaimers in outputs.
- Edge cases: multi-OS, air-gapped, EDR simulation.

## Success Metrics
- Clean integration with prior phases.
- Realistic dry-run campaigns.
- Comprehensive documentation.

**Next**: Phase 4 polish.

Contributions: Follow code style, prioritize safety.