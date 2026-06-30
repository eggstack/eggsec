# Phase 4 Handoff Plan: Metadata Unification for Operations, Tools, Docs, and Policy

## Objective

Unify Eggsec's operation, tool, command, frontend, report, and documentation metadata around a single canonical model. The immediate goal is to reduce drift between code, README/docs, policy explain output, TUI preflight, MCP/tool registration, and Cargo feature descriptions.

This phase should consume the domain contract introduced in Phase 3 and connect it to at least one practical output: a generated or validated capability matrix, policy explain output, or tool metadata view.

## Context

Eggsec currently has useful metadata primitives, but information is still duplicated across multiple places:

- `Cargo.toml` feature definitions and comments;
- README capability tables;
- operation metadata;
- command handlers and CLI args;
- tool registry registration;
- TUI tab/preflight code;
- MCP exposure gates;
- report/evidence documentation;
- architecture and safety docs.

This duplication produces drift. For example, workspace crate counts, feature histories, MCP exposure notes, and domain status can diverge between Cargo manifests, README, and code. A security tool with hazardous lab features should avoid ambiguous or stale documentation.

## Deliverables

1. Define a canonical metadata model or extend the Phase 3 `DomainDescriptor` model so that operation/risk/capability/feature/scope/exposure metadata is available from one source.

2. Add a capability matrix generator or validator.

3. Move detailed capability status out of README where appropriate and into a generated or validated `docs/CAPABILITY_MATRIX.md`.

4. Ensure policy explain or preflight can consume canonical metadata for at least the pilot domain.

5. Add tests that verify important metadata remains synchronized.

6. Add documentation describing the metadata ownership model and update workflow.

## Canonical metadata fields

The model should be able to represent at least:

- domain ID;
- operation ID;
- display name;
- description;
- category: standard assessment, defense lab, hazardous lab, frontend/API, output/report;
- Cargo feature gate;
- optional MCP/tool feature gate;
- operation mode;
- risk tier;
- required capabilities;
- intended uses;
- target policy kind;
- explicit scope requirement;
- private/local target requirement;
- dry-run support;
- TUI exposure;
- CLI exposure;
- MCP/tool exposure;
- REST/gRPC exposure if applicable;
- report support;
- evidence bundle support;
- baseline/regression support;
- strict-surface support;
- manual override confirmation classes if derivable;
- docs URL or docs path.

Do not require every domain to fill every field immediately. Prefer defaults and `Option` fields where appropriate.

## Capability matrix target

Create or update `docs/CAPABILITY_MATRIX.md`. It should be generated or validated from canonical metadata as much as feasible.

Suggested columns:

- Domain
- Operation
- Category
- Feature
- Risk
- Capabilities
- CLI
- TUI
- MCP/API
- Dry-run
- Evidence/report support
- Scope requirement
- Notes

For generated output, add a command or test utility. If full generation is too much for this pass, add a test that validates a checked-in matrix against metadata for the pilot domain and leaves clear TODOs for expanding coverage.

## README cleanup

The README should remain concise and user-facing. Move phase-history-heavy details and long caveats into dedicated docs.

Recommended README shape:

- brief project description;
- what Eggsec is and is not;
- safety model summary;
- quick start;
- concise workspace layout;
- concise capability overview linking to `docs/CAPABILITY_MATRIX.md`;
- links to detailed docs for safety, enforcement modes, domain docs, and architecture.

Do not remove important safety warnings. Move them to more maintainable locations if they are too detailed for README tables.

## Policy explain and preflight integration

At least one consumer should use canonical metadata. Preferred order:

1. `policy-explain` / `preflight` command includes metadata-derived feature/risk/capability text.

2. TUI preflight displays metadata-derived domain/risk/capability information.

3. MCP/tool registration uses metadata-derived operation mappings.

4. Docs generator uses metadata.

Do not attempt all consumers in one pass unless the change remains small.

## Implementation steps

1. Review Phase 3 domain contract implementation.

2. Inspect existing operation metadata helpers and decide whether to merge, wrap, or bridge them with the new domain metadata.

3. Define the canonical metadata source for operation-level fields.

4. Implement metadata extraction for the pilot domain and at least the already-central operation metadata list.

5. Add a capability matrix generator or validator.

6. Generate or update `docs/CAPABILITY_MATRIX.md`.

7. Simplify README capability details where safe, replacing long status/history text with links.

8. Update policy explain/preflight or another selected consumer to use canonical metadata.

9. Add tests for metadata consistency:

   - operation IDs are unique;
   - tool aliases resolve to known operation IDs;
   - capability matrix rows match known operation metadata for pilot domain;
   - hazardous MCP exposure is not accidentally default-enabled;
   - documented feature names exist in Cargo features or the metadata marks them as external.

10. Run validation.

## Metadata uniqueness rules

Add tests or validation for:

- unique domain IDs;
- unique operation IDs within a domain;
- globally stable operation IDs when used by tool dispatch;
- no duplicate tool IDs;
- all tool IDs map to known operation metadata;
- all required capabilities are valid `Capability` enum variants;
- all risks are valid `OperationRisk` variants;
- all docs paths referenced by metadata exist if they are local files.

## Safety requirements

- Metadata must not grant authorization.
- Feature presence must not imply runtime authorization.
- MCP/tool exposure must stay opt-in for hazardous domains.
- Generated docs must not overstate capabilities that are feature-gated or dry-run-only.
- Manual CLI/TUI semantics must remain distinct from automated strict semantics.

## Validation commands

Run at minimum:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --lib
```

If a doc generator command is added, run it and verify no unintended diff remains:

```bash
cargo run -p eggsec-cli -- <metadata-doc-command-if-added>
git diff -- docs/CAPABILITY_MATRIX.md README.md
```

Run feature checks for the pilot domain and tool/API surfaces touched:

```bash
cargo check -p eggsec --features db-pentest
cargo check -p eggsec --features web-proxy
cargo check -p eggsec --features rest-api,tool-api
```

## Non-goals

Do not fully rewrite command dispatch.

Do not perform broad domain extraction.

Do not require every existing feature to have perfect metadata in this pass; prioritize the canonical model and pilot coverage.

Do not remove safety text without preserving it in dedicated docs.

Do not make generated docs the only source of safety documentation.

## Acceptance criteria

- A canonical metadata path exists for domain/operation/risk/capability/feature/exposure information.
- At least one practical consumer uses the canonical metadata.
- `docs/CAPABILITY_MATRIX.md` exists or is validated/generated from metadata.
- README is less prone to status drift, with detailed capability status moved into dedicated docs where appropriate.
- Tests cover uniqueness and key safety exposure rules.
- Existing user-facing behavior remains compatible.

## Handoff notes for Phase 5

Phase 5 should use the canonical metadata model to slim the main crate. Extracted domains should provide descriptors and report adapters through the same metadata path rather than adding new one-off integration code.
