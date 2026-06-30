# Cleanup Handoff Plan: Documentation and Metadata Truthfulness

## Objective

Perform a narrow cleanup pass to make Eggsec's new metadata layer and human-facing documentation agree with one another. The previous corrective pass fixed the important code-level issues: always-present domain descriptors, feature-aware descriptor availability, separate tool/MCP/API exposure fields, mobile-dynamic reclassification, and stronger metadata tests. The remaining issues are mostly documentation/table drift and one explicit metadata policy decision around high-risk programmatic exposure flags.

This pass should be small and precise. Do not add new capabilities, do not extract additional domains, and do not redesign command or tool registration.

## Current state

The repo is structurally healthier after the corrective pass:

- `all_domain_descriptors()` now returns all known descriptors independent of feature state.
- `available_domain_descriptors()` filters descriptors by compiled feature.
- `CapabilityMatrixRow` now distinguishes `tool_integration`, `mcp_exposed_by_default`, `required_mcp_feature`, `rest_exposable`, and `agent_exposable`.
- `mobile-dynamic` is now `Intrusive`, declares `MobileDynamicAnalysis`, is not MCP/REST/agent/gRPC exposed, and has `strict_surface_support = false`.
- README no longer hardcodes an incorrect workspace crate count.
- Architecture docs now explicitly call out the orchestrator raw-dispatch transitional exception.

Remaining cleanup items:

1. `docs/CAPABILITY_MATRIX.md` still has rows that disagree with current metadata, especially mobile static/dynamic strict/scope/capability fields.
2. The capability definitions table omits `MobileDynamicAnalysis`.
3. Some metadata test comments still describe the old feature-gated registry model.
4. Docs URL validation is best-effort and does not fail when repo-local docs paths are missing.
5. The meaning of high-risk `mcp_exposable`, `rest_exposable`, `agent_exposable`, and `grpc_exposable` flags needs an explicit decision and probably better wording/tests.

## Non-goals

- Do not change manual CLI/TUI permissive semantics.
- Do not expose new MCP/REST/agent tools.
- Do not add runtime mobile behavior.
- Do not remove strict runtime policy gates.
- Do not implement the full command registry refactor.
- Do not add a full markdown generator unless it remains small and mechanical.

## Work item 1: Fix capability matrix domain rows

### Problem

`docs/CAPABILITY_MATRIX.md` currently lists:

- `mobile-static` with `Scope = explicit scope`, but metadata says `TargetPolicyKind::OptionalTarget` and the domain descriptor has `requires_explicit_scope = false`.
- `mobile-dynamic` with `Strict = Y` and `Scope = explicit scope`, but the descriptor has `strict_surface_support = false` and `requires_explicit_scope = false`.
- `mobile-dynamic` does not show its new `MobileDynamicAnalysis` capability in the domain row.

### Required changes

Update the Domain Operations table in `docs/CAPABILITY_MATRIX.md` so each row matches `DomainDescriptor`, `OperationIntegration`, and `OperationMetadata`.

Recommended row semantics:

- `mobile-static`
  - Risk: `SafeActive`
  - Capability: `—` unless a future `MobileStaticAnalysis` capability is added
  - MCP/API: `N`
  - REST: `N` if column exists or notes mention not programmatically exposed
  - Agent: `N` if column exists or notes mention not programmatically exposed
  - Strict: `Y` only if the `Strict` column means domain `strict_surface_support`; otherwise rename the column to remove ambiguity
  - Scope: `optional target` or `local file target`

- `mobile-dynamic`
  - Risk: `Intrusive`
  - Capability: `MobileDynamicAnalysis`
  - MCP/API: `N`
  - REST: `N`
  - Agent: `N`
  - Strict: `N`
  - Scope: `optional target` or `device/lab context` depending on final column semantics

### Acceptance criteria

- Mobile rows in the matrix match code metadata.
- No matrix row says `mobile-dynamic` has strict-surface support.
- No matrix row says `mobile-dynamic` is baseline-safe.
- No matrix row says mobile static/dynamic require explicit network scope unless the code does.

## Work item 2: Add `MobileDynamicAnalysis` to capability definitions

### Problem

The `Capability` enum includes `MobileDynamicAnalysis`, but `docs/CAPABILITY_MATRIX.md` does not list it in Capability Definitions.

### Required changes

Add a capability row:

```text
MobileDynamicAnalysis | No | Android dynamic/runtime lab testing via ADB/logcat/proxy/Frida-style instrumentation
```

Adjust wording as needed to match the exact implementation. Keep it defensive/lab-oriented.

### Acceptance criteria

- Every capability variant used by `OperationMetadata` appears in the capability definitions table.
- `MobileDynamicAnalysis` is explicitly non-baseline.

## Work item 3: Clarify standalone operations exposure semantics

### Problem

The Standalone Operations table currently shows many high-risk operations as `MCP/API = Y`, `REST = Y`, and `Agent = Y`. This appears to reflect current `OperationMetadata` flags, but the table does not explain the difference between:

- metadata-level programmatic exposure;
- default availability;
- feature-gated registration;
- strict runtime policy approval;
- agent-safe-by-default operation.

This can be misleading for high-risk operations like `waf-stress`, `stress-test`, `packet`, `db-pentest`, `c2`, `proxy-intercept`, and `remote`.

### Required decision

Choose one of two models.

#### Model A: Keep broad programmatic exposure flags, clarify semantics

Use this if the intent is that high-risk operations may be callable from MCP/REST/agent surfaces when explicitly compiled, registered, scoped, and policy-authorized.

Required doc changes:

- Rename columns from ambiguous `MCP/API`, `REST`, and `Agent` to more precise names:
  - `MCP Metadata`
  - `REST Metadata`
  - `Agent Metadata`
  - or `Programmatic Metadata`
- Add a note before the table:
  - `Y` means metadata permits registration/exposure under the relevant feature/profile; it does not mean default safe execution.
  - Strict surfaces require explicit scope manifest provenance and policy/capability allowance.
  - High-risk rows are not baseline-agent-safe even if metadata says programmatic exposure is possible.
- Optionally add a `Default Safe` or `Baseline Agent` column for baseline-safe operations only.

Required test changes:

- Add a test that high-risk agent-exposable operations declare non-baseline capabilities and are blocked by default policy in `AgentStrict`.
- Existing high-risk exposure tests should use language like `agent-exposable-with-policy` rather than implying default agent safety.

#### Model B: Narrow high-risk programmatic exposure flags

Use this if high-risk operations should not be programmatically exposable by default metadata at all.

Required code changes:

- Set `mcp_exposable`, `rest_exposable`, `agent_exposable`, and/or `grpc_exposable` to `false` for high-risk operations unless there is an explicit opt-in feature and tool registration path.
- Keep manual CLI/TUI exposure as appropriate.
- Add tests that high-risk operations are not programmatic by default.
- Update docs accordingly.

### Recommended approach

Start with Model A unless the actual MCP/REST/agent registry already has a clear narrower exposure policy. This is safer for a cleanup pass because it avoids changing runtime availability unexpectedly. However, explicitly document that `Y` means metadata-level exposability, not default execution or safety.

### Acceptance criteria

- The capability matrix no longer implies high-risk operations are safe/default agent actions.
- The terms `exposable`, `default exposed`, `registered`, and `runtime approved` are used consistently.
- Tests preserve whichever model is chosen.

## Work item 4: Fix stale metadata test comments

### Problem

`metadata_consistency.rs` still has comments saying the domain registry is empty with no features enabled. That is false after the always-present descriptor model.

### Required changes

Update comments around `capability_matrix_has_rows_when_domains_registered()` and related tests.

Suggested wording:

```rust
/// The capability matrix should produce rows for all known domain descriptors,
/// independent of compile-time feature state. `available_domain_descriptors()`
/// is the filtered view for currently compiled features.
```

If the test currently has feature-gated assertions that no longer add value, simplify them.

### Acceptance criteria

- Test comments describe the current always-present descriptor model.
- No comment implies disabled-feature domains disappear from `all_domain_descriptors()`.

## Work item 5: Make docs URL validation deterministic

### Problem

`domain_docs_urls_reference_existing_files()` is currently best-effort and prints a note if a repo-local `docs/` path is missing. That will not prevent doc drift.

### Required changes

Use a deterministic workspace-relative path for local docs URLs.

Recommended implementation:

- Use `env!("CARGO_MANIFEST_DIR")` from the `eggsec` crate test context.
- Since `CARGO_MANIFEST_DIR` points to `crates/eggsec`, derive workspace root with `Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")` and normalize or join directly.
- For any `docs_url` beginning with `docs/`, assert that `workspace_root.join(url).exists()`.

Example sketch:

```rust
let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
let workspace_root = manifest_dir.join("../..");
let path = workspace_root.join(url);
assert!(path.exists(), "domain '{}' docs_url '{}' does not exist", domain.id, url);
```

### Acceptance criteria

- Missing local docs URLs fail tests.
- Existing `docs/DATABASE_PENTEST.md` and `docs/MOBILE.md` references pass.
- Non-local URLs, if ever added, are ignored or validated separately.

## Work item 6: Decide whether to add lightweight capability matrix validation

### Problem

The capability matrix is now explicitly manually maintained, which is honest. However, the current drift shows manual maintenance is already error-prone.

### Required decision

Choose one:

#### Option A: Keep manual table, add targeted metadata tests only

This is acceptable for a short cleanup pass. Add tests for the exact drift cases:

- `mobile-dynamic` strict support false in generated row.
- `mobile-dynamic` scope requirement is not explicit scope.
- `mobile-dynamic` capability includes `MobileDynamicAnalysis`.
- `mobile-static` scope requirement is not explicit scope.

#### Option B: Add snapshot validation for domain rows only

Create a small deterministic renderer for the Domain Operations table from `generate_capability_matrix()` and test it against a checked-in snippet or generated string.

This avoids validating the full long standalone operations table now, but catches the domain-row drift that just happened.

### Recommended approach

Use Option A unless the renderer is straightforward. The next larger roadmap phase can introduce generated docs more systematically.

### Acceptance criteria

- At minimum, tests catch the current mobile row drift if it reappears.
- If snapshot validation is added, it is small and stable.

## Work item 7: Clean up README workspace table verbosity

### Problem

README no longer has the wrong crate count, but the `eggsec-tui` table entry contains long historical phase prose. That makes the workspace table noisy and likely to drift.

### Required changes

Shorten the `eggsec-tui` row to a durable description, e.g.:

```text
Terminal UI adapter (`ratatui`/`crossterm`) with packaged themes, tab workflows, task runtime, and interactive enforcement preflight.
```

Move detailed TUI phase history to architecture docs or leave it in existing plans, not the README table.

### Acceptance criteria

- README workspace table is concise and stable.
- No important safety behavior is removed; detailed TUI architecture remains linked from docs if needed.

## Work item 8: Update metadata ownership docs if needed

### Problem

`docs/METADATA_OWNERSHIP.md` may still describe the old or ambiguous matrix semantics.

### Required changes

Inspect and update it so it clearly states:

- `OperationMetadata` owns canonical operation risk/capability/exposure flags.
- `DomainDescriptor` owns domain grouping and integration metadata.
- `docs/CAPABILITY_MATRIX.md` is currently manually maintained from those sources unless a generator/snapshot test is added.
- Programmatic exposure flags do not equal default runtime approval.
- Strict runtime approval still requires `EnforcementContext` and, for strict programmatic surfaces, `ApprovedOperation`.

### Acceptance criteria

- Metadata ownership docs match the chosen exposure model.
- The update workflow tells maintainers exactly which files/tests to update.

## Suggested implementation order

1. Update stale test comments.
2. Fix capability matrix domain rows and add `MobileDynamicAnalysis` to capability definitions.
3. Clarify standalone operation exposure semantics in `docs/CAPABILITY_MATRIX.md`.
4. Make docs URL validation deterministic.
5. Add targeted tests for mobile row drift and high-risk exposure semantics.
6. Shorten the README TUI workspace row.
7. Update `docs/METADATA_OWNERSHIP.md` if its language conflicts with the chosen model.

## Files likely to change

- `docs/CAPABILITY_MATRIX.md`
- `docs/METADATA_OWNERSHIP.md`
- `README.md`
- `crates/eggsec/tests/metadata_consistency.rs`
- optionally `crates/eggsec/src/config/policy.rs` if choosing Model B for exposure flags
- optionally `crates/eggsec/src/domain/mod.rs` if adding renderer helpers or test-only formatting helpers

## Validation commands

Run at minimum:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test mobile_adapter --features mobile
cargo test -p eggsec --test mobile_adapter --features mobile-dynamic
```

Also run the core tests if metadata or enforcement code changes:

```bash
cargo test -p eggsec --lib
cargo test -p eggsec --test enforcement_matrix
cargo test -p eggsec --test enforced_dispatch_regression
```

If exposure flags are changed for high-risk operations, also run the relevant MCP/API feature checks:

```bash
cargo check -p eggsec --features rest-api,tool-api
cargo check -p eggsec --features db-pentest-mcp,rest-api,tool-api
cargo check -p eggsec --features web-proxy-mcp,rest-api,tool-api
cargo check -p eggsec --features c2-mcp,rest-api,tool-api
```

## Completion criteria

This cleanup is complete when:

- `docs/CAPABILITY_MATRIX.md` accurately reflects mobile static/dynamic metadata.
- `MobileDynamicAnalysis` appears in capability definitions.
- High-risk programmatic exposure semantics are explicitly documented and tested according to the chosen model.
- Stale comments about feature-gated domain descriptors are fixed.
- Local docs URL references fail tests if missing.
- README workspace table is concise and stable.
- Metadata ownership docs match the current source-of-truth model.

## Handoff note

After this cleanup, the metadata layer should be stable enough to support the next architectural phase: command registry refactor or metadata-driven tool/MCP registration. Do not begin those larger phases while the capability matrix or exposure semantics remain ambiguous.
