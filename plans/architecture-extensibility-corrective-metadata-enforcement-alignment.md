# Corrective Handoff Plan: Metadata and Enforcement Alignment Pass

## Objective

Correct the metadata, documentation, and enforcement-exposure inconsistencies introduced during the architecture/extensibility implementation pass. The repo is now substantially closer to the target architecture, but the new metadata layer must be made truthful before future phases use it for command registration, MCP/tool exposure, generated docs, or domain extraction.

This pass should be narrow and corrective. Do not add broad new capabilities. Do not perform another large domain extraction. Focus on making the new domain metadata, capability matrix, README, and strict-surface exposure semantics accurate and mechanically validated.

## Current state summary

The recent implementation made several positive changes:

- added architecture and invariant documentation;
- added enforcement matrix tests;
- introduced `crates/eggsec/src/domain/mod.rs` with `DomainDescriptor` and integration metadata;
- added metadata consistency tests;
- extracted mobile analysis into `eggsec-mobile-lab`;
- reduced `crates/eggsec/src/mobile/mod.rs` to a thin adapter/re-export layer.

The remaining issue is truthfulness and precision. Some metadata and documentation currently overstate exposure, disagree with implementation, or classify high-risk/dynamic behavior too permissively.

## Corrective targets

### 1. Fix README crate-count drift

The README currently says the workspace has nine crates while the actual workspace contains eleven members, including `eggsec-web-proxy` and `eggsec-mobile-lab`.

Required work:

- Update README workspace count to match `Cargo.toml`.
- Prefer avoiding hardcoded counts if possible: use language such as "Eggsec is organized as a Cargo workspace with these crates".
- Ensure the README table includes all current workspace crates exactly once.
- Remove or shorten stale phase-history prose in the workspace table where it makes the README brittle.

Acceptance criteria:

- README no longer contains an incorrect workspace crate count.
- README workspace table matches root `Cargo.toml` members.
- The wording will not immediately drift if one crate is added.

### 2. Restore or explicitly replace the removed Phase 1 plan file

The compare from the roadmap baseline showed `plans/architecture-extensibility-phase-01-inventory-invariants.md` was removed while later phase plans remain. This breaks handoff continuity unless intentional.

Required work:

- Check whether the file was intentionally removed after execution.
- If there is no explicit project convention for deleting executed plans, restore the file.
- If the team wants executed phase plans removed, replace it with a short archival note or update the roadmap to explain the convention.

Recommended action:

- Restore the Phase 1 plan file exactly or substantially from git history.
- Add a small note at the top if the phase has been executed, e.g. "Status: executed; retained for audit/handoff history."

Acceptance criteria:

- `plans/architecture-extensibility-phase-01-inventory-invariants.md` exists again, or an explicit documented replacement explains why it does not.
- Plan directory remains coherent for future handoff readers.

### 3. Fix `DomainDescriptor` registry semantics

`all_domain_descriptors()` currently documents that disabled-feature domains are still included and gated by `required_feature`, but the implementation uses `#[cfg(feature = ...)]`, so disabled-feature descriptors are absent.

Required work:

Choose one of two models and make code, docs, tests, and capability matrix agree.

Preferred model: always-present descriptors.

- `all_domain_descriptors()` returns all known domain descriptors independent of compile-time feature state.
- `required_feature` and per-operation `required_features` describe build gates.
- Consumers that want currently compiled domains call a separate helper, e.g. `available_domain_descriptors()` or check feature availability.
- Documentation and capability matrix can show full known capability surface even in no-default builds.

Alternate model: feature-present descriptors only.

- Keep `#[cfg]` gates in `all_domain_descriptors()`.
- Correct the doc comment to say the slice reflects only compiled features.
- Add a separate static known-domain inventory only if docs generation needs full visibility.

Recommended implementation:

- Use the preferred always-present descriptor model for metadata-only structs.
- Keep execution code, crate dependencies, and domain invocation behind existing Cargo features.
- Ensure descriptors are static data only and do not require linking optional domain crates unless absolutely necessary.

Acceptance criteria:

- The `all_domain_descriptors()` comment matches implementation.
- Tests cover no-default behavior and feature-enabled behavior.
- Metadata/documentation generation does not silently omit disabled but known feature-gated domains unless that is explicitly intended.

### 4. Split MCP/API exposure semantics

The current `CapabilityMatrixRow` has a single `mcp_api: bool` derived from whether a `ToolIntegration` exists for an operation. That conflates several different states:

- a domain has a tool integration record;
- a tool is exposed by default;
- a tool is available only behind an explicit MCP feature;
- a tool is REST/gRPC/API-compatible;
- a tool is agent-safe by default.

This is currently misleading for `db-pentest`: its tool integration says `mcp_exposed_by_default = false` and `required_mcp_feature = Some("db-pentest-mcp")`, but the capability matrix/test treats it as MCP/API true.

Required work:

- Replace or augment `CapabilityMatrixRow::mcp_api: bool` with more precise fields. Suggested fields:
  - `tool_integration: bool`
  - `mcp_exposed_by_default: bool`
  - `required_mcp_feature: Option<&'static str>`
  - `rest_exposable: Option<bool>` if known from `OperationMetadata`
  - `agent_exposable: Option<bool>` if known from `OperationMetadata`
- Update `generate_capability_matrix()` to populate these separately.
- Update `docs/CAPABILITY_MATRIX.md` headings and rows to distinguish "Tool", "MCP Default", "MCP Feature", "REST", and "Agent".
- Update metadata consistency tests to assert hazardous or high-risk domains are not MCP-exposed by default unless explicitly justified.
- Update db-pentest tests to assert tool integration exists but default MCP exposure is false and `db-pentest-mcp` is required.

Acceptance criteria:

- No boolean column implies default MCP exposure for opt-in MCP tools.
- db-pentest, web-proxy, and C2 MCP exposure semantics are represented as opt-in where applicable.
- Tests fail if a hazardous/defense-lab tool becomes default MCP-exposed by accident.

### 5. Reclassify mobile dynamic risk and capability metadata

`mobile-dynamic` currently appears too permissive in domain metadata: risk is `SafeActive`, capabilities are empty, explicit scope is false, and strict surface support is true. Dynamic Android runtime testing involving ADB, logcat, proxying, and Frida-adjacent instrumentation should not look baseline-safe to strict programmatic surfaces.

Required work:

- Review actual mobile dynamic behavior in `eggsec-mobile-lab`, especially `dynamic.rs`, `adb.rs`, `traffic.rs`, and `frida.rs`.
- Decide the correct risk class for each mobile operation:
  - static APK/IPA analysis can likely remain `SafeActive` if it only reads local files;
  - dynamic app launch/logcat/ADB work should likely be `Intrusive` or a dedicated existing non-baseline category;
  - Frida/runtime instrumentation should likely be `Intrusive`, `ExploitAdjacent`, or another explicit lab risk depending on actual behavior.
- Add or reuse appropriate capabilities. If no existing capability fits, add a new capability variant such as `MobileStaticAnalysis`, `MobileDynamicAnalysis`, or `RuntimeInstrumentation`. Prefer not to overload unrelated capabilities.
- Set `strict_surface_support` carefully. If dynamic mobile is standalone CLI/TUI-only and MCP-absent by policy, mark strict support false or require explicit non-baseline capability and explicit feature/policy allow.
- Ensure `OperationMetadata` and `DomainDescriptor` agree.
- Update tests to verify mobile-dynamic is not baseline-strict-safe.

Recommended metadata posture:

- `mobile-static`: local-file analysis, `SafeActive`, capability either empty or `MobileStaticAnalysis`, no network target scope requirement, strict support only if there is an actual programmatic safe path.
- `mobile-dynamic`: defense-lab/runtime operation, non-baseline capability, risk at least `Intrusive`, no default MCP exposure, explicit runtime flag/policy gate required for real device interaction.
- `frida`-class operations: if separately represented, explicit non-baseline capability and no default programmatic exposure.

Acceptance criteria:

- Mobile dynamic metadata no longer appears baseline-safe.
- Strict profiles require explicit policy/capability allowance for mobile dynamic if exposed at all.
- Docs and capability matrix reflect static vs dynamic differences clearly.
- Tests cover mobile static/dynamic metadata separation.

### 6. Align `docs/CAPABILITY_MATRIX.md` with actual generation/validation semantics

The capability matrix currently says it is derived from metadata, but the tests mostly validate metadata structures rather than parsing or regenerating the file. This can create a false sense of generated-doc correctness.

Required work:

Choose one of two models.

Preferred model: generated or snapshot-validated matrix.

- Add a small internal generator function that renders the capability matrix markdown from `OperationMetadata` and `DomainDescriptor`.
- Add a test that compares the checked-in file to the generated string, or add a command that regenerates it and document that command.
- Keep the table formatting stable.

Alternate model: manually maintained matrix with metadata consistency tests.

- Change the language in `docs/CAPABILITY_MATRIX.md` from "derived from" to "maintained from" or "validated against selected metadata invariants".
- Add clear update instructions.
- Add tests for the most important claims that can be checked without parsing markdown.

Recommended implementation:

- Use snapshot validation if feasible.
- If a full generator is too much, at least correct the wording immediately and add a TODO for generated docs.

Acceptance criteria:

- The document no longer overstates its generation/validation status.
- Either the file is generated/snapshot-validated, or its manual-maintenance status is explicit.
- Metadata consistency tests cover the claims they can reasonably enforce.

### 7. Soften or fix overbroad architecture claims around raw dispatch

`docs/ARCHITECTURE.md` says every side-effecting operation passes through `EnforcementContext::evaluate()` before execution, but the same document lists the orchestrator using raw `ToolDispatcher::dispatch()` with "caller must enforce".

Required work:

- Either move the orchestrator behind enforced dispatch, or make the architecture document precise about the transitional exception.
- If keeping raw dispatch for orchestrator, add a clear invariant: the orchestrator may only be constructed or invoked after caller-level enforcement, and strict programmatic surfaces must not reach it without `ApprovedOperation` or an equivalent prior approval chain.
- Add or keep regression tests showing strict programmatic surfaces use `EnforcedDispatcher::dispatch_checked()`.
- Consider adding a tracked TODO to remove/replace raw orchestrator dispatch in a future pass.

Acceptance criteria:

- Architecture docs no longer make an unqualified claim contradicted by the raw orchestrator exception.
- The exception is explicitly documented and tested.
- Strict surfaces remain protected.

### 8. Tighten metadata consistency tests

The new tests are useful, but several should be made more semantically precise.

Required work:

- Add tests for registry semantics chosen in target 3.
- Add tests that opt-in MCP features do not imply default MCP exposure.
- Add tests that `CapabilityMatrixRow` exposure fields align with `ToolIntegration` and `OperationMetadata`.
- Add tests that no high-risk or non-baseline operation is marked agent-exposable without explicit non-baseline capability and strict policy requirement.
- Add tests that docs URLs referenced by domain descriptors point to existing local files when they begin with `docs/`.
- Add tests that README workspace crate entries match root `Cargo.toml` members if feasible; if parsing TOML in tests is too much, add a simple documentation validation helper or a follow-up TODO.

Acceptance criteria:

- Metadata consistency tests would have caught the current db-pentest MCP/API conflation.
- Tests would catch mobile-dynamic being baseline-safe if the chosen policy says it should not be.
- Docs URL checks verify file existence for local docs paths.

## Suggested implementation order

1. Fix README crate count and restore/replace the removed Phase 1 plan file. These are low-risk cleanup items.

2. Decide and implement domain registry semantics. This affects later tests and docs.

3. Split tool/MCP/API exposure metadata fields. Update generated rows, tests, and capability matrix.

4. Reclassify mobile dynamic metadata and align `OperationMetadata` with `DomainDescriptor`.

5. Correct capability matrix wording or implement generation/snapshot validation.

6. Fix architecture doc claims around orchestrator/raw dispatch.

7. Add final metadata consistency tests.

## Files likely to change

- `README.md`
- `plans/architecture-extensibility-phase-01-inventory-invariants.md`
- `crates/eggsec/src/domain/mod.rs`
- `crates/eggsec/src/config/policy.rs`
- `crates/eggsec/tests/metadata_consistency.rs`
- `crates/eggsec/tests/enforcement_matrix.rs` if additional strict/mobile checks are needed
- `crates/eggsec/tests/enforced_dispatch_regression.rs` if orchestrator/raw dispatch invariants are adjusted
- `docs/CAPABILITY_MATRIX.md`
- `docs/METADATA_OWNERSHIP.md`
- `docs/ARCHITECTURE.md`
- `docs/ARCHITECTURE_INVARIANTS.md`
- `docs/MOBILE.md` if mobile dynamic risk semantics are clarified

## Validation commands

Run at minimum:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --lib
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test enforcement_matrix
cargo test -p eggsec --test enforced_dispatch_regression
```

Run feature-specific checks:

```bash
cargo check -p eggsec --features db-pentest
cargo test -p eggsec --features db-pentest --test metadata_consistency

cargo check -p eggsec --features mobile
cargo test -p eggsec --features mobile --test metadata_consistency

cargo check -p eggsec --features mobile-dynamic
cargo test -p eggsec --features mobile-dynamic --test metadata_consistency

cargo check -p eggsec --features rest-api,tool-api
cargo check -p eggsec --features db-pentest-mcp,rest-api,tool-api
cargo check -p eggsec --features web-proxy-mcp,rest-api,tool-api
cargo check -p eggsec --features c2-mcp,rest-api,tool-api
```

If the final set is expensive or platform-sensitive, record skipped commands and why. Do not mark the pass complete without at least no-default, base library tests, metadata consistency tests, and relevant feature checks for touched metadata.

## Non-goals

- Do not add new runtime capabilities.
- Do not perform another large domain extraction.
- Do not redesign the entire command registry.
- Do not expose additional MCP/agent tools.
- Do not weaken manual CLI/TUI discretion semantics.
- Do not make feature gates the only authorization mechanism.
- Do not move central authorization into domain crates.

## Acceptance criteria for the corrective pass

The pass is complete when:

- README workspace crate listing matches the actual workspace.
- Phase plan continuity is restored or explicitly documented.
- Domain registry semantics are accurate and tested.
- Capability matrix exposure fields distinguish tool integration from default MCP/API/agent exposure.
- db-pentest/web-proxy/C2 opt-in MCP semantics are represented accurately.
- mobile-static and mobile-dynamic have distinct and defensible risk/capability metadata.
- `docs/CAPABILITY_MATRIX.md` accurately describes whether it is generated, snapshot-validated, or manually maintained.
- architecture docs do not overstate enforcement coverage where raw orchestrator dispatch remains as a transitional exception.
- metadata consistency tests catch the specific classes of drift identified in this plan.

## Handoff note

After this corrective pass, the repo should be safe to continue with later roadmap phases: command registry refactor, tool/MCP metadata-driven registration, and additional domain extraction. Do not build those later phases on top of the current metadata layer until this alignment pass is complete.
