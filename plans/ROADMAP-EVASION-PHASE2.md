# Phase 2: Post-Exploitation & LOTL Toolkit Roadmap

## Overview
This document provides a detailed, step-by-step plan for implementing Phase 2 of the Evasion/Red Team enhancements. Designed for smaller models or contributors to follow sequentially.

**Alignment with Conventions**:
- Feature-gated (`postex`)
- Scoped access
- Dry-run first
- Policy enforcement
- CLI/TUI raw control vs MCP agentic
- MITRE ATT&CK mapping
- Baselines and regression

## Prerequisites
1. Phase 1 evasion module complete.
2. Review `crates/eggsec/src/evasion/` and `plans/ROADMAP-EVASION.md`.
3. Ensure `postex` feature added to Cargo.toml.

## Step-by-Step Implementation Plan

### Step 1: Setup Module Structure (1-2 days)
- Create `crates/eggsec/src/postex/` directory.
- Add `mod.rs`, `lotl.rs`, `persistence.rs`, `lateral.rs`, `credential.rs`.
- Update `Cargo.toml` with `postex` feature.
- Add to lib.rs and main CLI.

### Step 2: LOTL Wrappers (2-3 days)
- Implement safe wrappers for PowerShell, WMI, certutil etc. with obfuscation.
- Add scope checks.
- Dry-run mode that logs intended commands.

### Step 3: Persistence Mechanisms
- Registry, scheduled tasks, DLL hijacking.
- Reversible in lab mode.
- Policy-gated.

### Step 4: Lateral Movement
- Proxy pivoting, SMB/RDP.
- Integrate with network modules.

### Step 5: Credential Handling
- Safe LSASS dump with evasion.
- Token impersonation.

### Step 6: Integration & Reporting
- Bridge to SARIF, baselines.
- Update TUI/CLI.
- Tests and docs.

## Detailed Tasks
[Full detailed steps...]

See full content in repo.