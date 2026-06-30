# Phase 5 Handoff Plan: Main Crate Slimming Pass

## Objective

Reduce the long-term architectural pressure on the main `eggsec` crate by moving additional domain execution logic behind clearer domain-crate boundaries, using the domain contract and metadata model from Phases 3 and 4.

This phase should not be a broad rewrite. It should choose one high-value extraction or deep extraction cleanup and complete it with compatibility facades, feature-gate preservation, report conversion, tests, and documentation.

## Context

The main `eggsec` crate currently acts as composition root, policy owner, command dispatcher, pipeline coordinator, tool registry owner, and home for many domain modules. Some of this is appropriate. The risk is that every new capability continues to expand the main crate.

`eggsec-db-lab` is the best current model: it owns database lab execution, types, reports, baselines, compliance mapping, correlation, and drivers, while the main crate retains enforcement and orchestration. `eggsec-web-proxy` is also already extracted and should be normalized against the same pattern.

The target is not to make the main crate empty. The target is to make it a composition root rather than a feature warehouse.

## Deliverables

1. Choose one extraction target or extraction cleanup target.

2. Move domain execution/types/report logic into the appropriate domain crate or finish normalizing an existing domain crate.

3. Keep central policy/enforcement in the main crate.

4. Preserve public compatibility paths where needed with re-export facades or thin adapters.

5. Connect the extracted/normalized domain to the Phase 3 domain contract and Phase 4 metadata model.

6. Update command handlers so they perform argument normalization, descriptor/enforcement, and domain invocation rather than owning domain logic.

7. Update report/evidence conversion paths.

8. Add tests for domain crate behavior and main-crate integration.

9. Update architecture and capability docs.

## Candidate extraction targets

### Option A: Web proxy normalization

Recommended if `eggsec-web-proxy` still has logic split between the main crate and the domain crate.

Goals:

- move proxy execution types, flow buffer, metrics, HAR/evidence, replay/manipulation audit, transparent proxy metadata, and dynamic plugin markers into `eggsec-web-proxy` where appropriate;
- keep CLI policy gate and command integration in main crate;
- ensure `TrafficInterception` risk and capability metadata are declared through the domain contract;
- preserve optional MCP exposure through `web-proxy-mcp` only;
- ensure real interception requires runtime allow flags and policy approval;
- preserve dry-run safe behavior.

This is a good target because traffic interception is high-risk and benefits from a crisp boundary.

### Option B: Mobile lab extraction

Recommended if mobile code is still mostly inside the main crate and is growing.

Goals:

- create `crates/eggsec-mobile-lab`;
- move static APK/IPA analysis types and execution into the domain crate;
- move dynamic Android/ADB/logcat/Frida-adjacent execution behind feature gates;
- keep CLI/TUI integration and enforcement in the main crate;
- ensure dynamic/frida operations remain lab-only and runtime-gated;
- expose report conversion through a bridge.

This is a good target because static and dynamic mobile analysis will otherwise grow rapidly.

### Option C: Wireless lab extraction

Recommended if wireless passive and advanced active operations are currently entangled.

Goals:

- create `crates/eggsec-wireless-lab`;
- move passive scan/security-analysis logic into the domain crate;
- keep advanced active operations behind `wireless-advanced`;
- preserve lab-only semantics and policy gates;
- provide dry-run/test fixtures where real wireless hardware is unavailable.

This is a good target if cfg/platform issues are causing test friction.

### Option D: Purple-lab extraction for evasion/postex/C2

Recommended only if those domains share enough internal types to justify one grouped crate or a small crate family.

Goals:

- move evasion, postex, and C2 domain logic out of the main crate;
- preserve separate feature gates and risk classes;
- keep C2 MCP exposure opt-in only;
- maintain ATT&CK mapping/report conversion;
- ensure dry-run and lab-mode semantics remain safe.

This is high value but may be broader than desirable for a single pass.

## Recommended target for this phase

Prefer Option A, web proxy normalization, unless inspection shows it is already cleanly extracted. If web proxy is already clean, choose mobile extraction because it is likely to grow and has a natural static/dynamic split.

## Boundary rules

Domain crate may own:

- domain-specific args after normalization;
- domain execution engine;
- dry-run implementation;
- domain report structs;
- evidence bundle internals;
- domain-specific compliance/correlation/mapping;
- domain fixtures/tests;
- optional driver/protocol dependencies.

Main crate should own:

- CLI command enum and raw CLI parsing unless Phase 6 has already changed this;
- command handler glue;
- `OperationDescriptor` construction or metadata lookup;
- central policy/enforcement call;
- `ApprovedOperation` handling;
- frontend routing;
- pipeline composition;
- tool/MCP registration glue;
- cross-domain report aggregation.

Domain crate must not own:

- central authorization decisions;
- manual override semantics;
- MCP/agent strictness semantics;
- global scope provenance policy;
- frontend-specific UI state.

## Implementation steps

1. Inspect current target domain module and identify logic that belongs in a domain crate versus main crate glue.

2. Define the desired crate/module boundary in a short section of the PR or commit notes.

3. If creating a new crate:

   - add it to workspace members;
   - add package metadata;
   - keep dependencies minimal and feature-gated;
   - add `src/lib.rs` with clear docs stating that enforcement remains outside the domain crate.

4. Move pure domain types first.

5. Move dry-run logic and tests next.

6. Move real execution logic behind the same feature gates.

7. Add bridge functions for report conversion and evidence export.

8. Update the main crate to call the domain crate through a small adapter.

9. Preserve compatibility re-exports where existing paths are likely used by tests or downstream code.

10. Register the domain through the Phase 3 domain contract.

11. Add or update metadata from Phase 4.

12. Update docs:

   - `docs/ARCHITECTURE.md` workspace table;
   - `docs/CAPABILITY_MATRIX.md` if generated/validated;
   - domain-specific docs;
   - README workspace layout if it lists crates.

13. Run validations.

## Testing requirements

Add tests at both levels.

### Domain crate tests

- dry-run produces a complete report without network or privileged side effects;
- report serialization is stable enough for downstream adapters;
- evidence bundle manifest, if supported, contains required fields;
- parsing/normalization handles invalid input safely;
- feature-gated real paths compile where possible.

### Main crate integration tests

- command handler builds the correct operation descriptor;
- enforcement runs before domain execution;
- denied policy prevents domain execution;
- manual override behavior remains manual-only;
- strict surfaces deny without explicit scope where required;
- report conversion still works;
- metadata lists the domain and operations correctly.

## Validation commands

Run at minimum:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --lib
```

If working on web proxy:

```bash
cargo check -p eggsec-web-proxy --all-features
cargo test -p eggsec-web-proxy --all-features
cargo check -p eggsec --features web-proxy
cargo test -p eggsec --features web-proxy --lib
```

If working on mobile extraction:

```bash
cargo check -p eggsec-mobile-lab --all-features
cargo test -p eggsec-mobile-lab --all-features
cargo check -p eggsec --features mobile
cargo check -p eggsec --features mobile-dynamic
```

If working on wireless extraction:

```bash
cargo check -p eggsec-wireless-lab --all-features
cargo test -p eggsec-wireless-lab --all-features
cargo check -p eggsec --features wireless
cargo check -p eggsec --features wireless-advanced
```

Adapt commands to the actual crate selected.

## Safety requirements

- No extraction may weaken policy gates.
- No domain crate may self-authorize real side effects.
- Dry-run must remain side-effect free.
- Hazardous MCP exposure must remain explicit opt-in.
- Existing manual CLI/TUI discretion semantics must remain unchanged unless tests prove a bug.
- Compatibility facades should be retained where practical to reduce churn.

## Non-goals

Do not extract multiple large domains in one pass unless they are trivially coupled.

Do not redesign command registration; that belongs to Phase 6.

Do not redesign report envelopes globally; that belongs to Phase 9.

Do not introduce dynamic plugin loading.

Do not move central policy into a domain crate.

## Acceptance criteria

- One selected domain is extracted or normalized behind a clear crate/module boundary.
- The main crate contains only glue for that domain: CLI normalization, policy descriptor/enforcement, invocation, and report bridge.
- The selected domain participates in the domain contract and metadata model.
- Tests prove enforcement still precedes execution.
- Docs reflect the new boundary.
- Feature-gated builds still pass for the selected domain.

## Handoff notes for Phase 6

Phase 6 should use the newly cleaned domain boundary to refactor command registration. The selected domain should become the example for how a domain can declare command metadata without forcing broad edits across the main command dispatcher and docs.
