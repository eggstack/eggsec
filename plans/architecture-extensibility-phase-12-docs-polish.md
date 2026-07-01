# Phase 12 Docs Polish Handoff Plan

## Objective

Perform a final narrow documentation polish pass after Phase 12 implementation. The extensibility guide set landed in the right shape, but the top-level index has broken links and the local-check checklist should be aligned with the Phase 11 architecture guard command set.

This pass should not change code behavior. It should only fix discoverability, local validation accuracy, and optional documentation guard coverage.

## Current state

Phase 12 implementation added:

- `docs/EXTENSIBILITY.md`
- `docs/extending/operations.md`
- `docs/extending/domains.md`
- `docs/extending/commands.md`
- `docs/extending/tool-exposure.md`
- `docs/extending/tui-actions.md`
- `docs/extending/report-evidence.md`
- `docs/extending/features.md`
- `docs/extending/testing.md`
- `docs/extending/templates.md`
- top-level references in `AGENTS.md`, `CONTRIBUTING.md`, `README.md`, and architecture docs
- retention checks in `scripts/check-architecture-guards.sh`

The detailed guides are substantive and correctly document the architecture model: metadata-first extension, domains do not authorize, strict surfaces require `ApprovedOperation`, listing is not authorization, MCP Model A, command dispatch modes, feature matrix updates, report/evidence envelope, and pre-handoff validation.

Remaining issues:

1. `docs/EXTENSIBILITY.md` links to non-existent guide filenames such as:
   - `docs/extending/adding-operation.md`
   - `docs/extending/adding-domain.md`
   - `docs/extending/adding-command.md`
   - `docs/extending/adding-tool.md`
   - `docs/extending/adding-tui-action.md`
   - `docs/extending/adding-report.md`
   - `docs/extending/adding-feature.md`

   Actual files are:
   - `docs/extending/operations.md`
   - `docs/extending/domains.md`
   - `docs/extending/commands.md`
   - `docs/extending/tool-exposure.md`
   - `docs/extending/tui-actions.md`
   - `docs/extending/report-evidence.md`
   - `docs/extending/features.md`
   - `docs/extending/testing.md`
   - `docs/extending/templates.md`

2. `docs/EXTENSIBILITY.md` required local checks are close but not fully aligned with Phase 11. It omits explicit checks that `make check-architecture-ci` covers:
   - `cargo test -p eggsec --test tool_registration --features rest-api`
   - `cargo test -p eggsec --test enforced_dispatch_regression`
   - `cargo test -p eggsec-output --test report_envelope`

3. Guard script checks existence of docs, but does not detect broken internal Markdown links.

## Non-goals

- Do not add new architecture concepts.
- Do not rename the guide files unless there is a strong reason.
- Do not change enforcement or exposure behavior.
- Do not loosen Phase 11 CI guards.
- Do not expand required PR CI beyond docs/link checks.

## Work item 1: Fix broken guide links in `docs/EXTENSIBILITY.md`

Update the Detailed Guides table to reference actual files.

Recommended replacement:

```markdown
| Topic | Guide |
|-------|-------|
| Adding an operation | `docs/extending/operations.md` |
| Adding a domain | `docs/extending/domains.md` |
| Adding a CLI command | `docs/extending/commands.md` |
| Adding a protocol-exposed tool | `docs/extending/tool-exposure.md` |
| Adding a TUI action | `docs/extending/tui-actions.md` |
| Adding report output | `docs/extending/report-evidence.md` |
| Adding a feature flag | `docs/extending/features.md` |
| Testing and pre-handoff checks | `docs/extending/testing.md` |
| Copyable templates | `docs/extending/templates.md` |
| Enforcement and dispatch | `docs/ENFORCEMENT_MODES.md` |
| Metadata ownership | `docs/METADATA_OWNERSHIP.md` |
| Capability matrix | `docs/CAPABILITY_MATRIX.md` |
| Tool registration | `docs/TOOL_REGISTRATION.md` |
| Command registry | `docs/COMMAND_REGISTRY.md` |
| Report/evidence model | `docs/REPORT_EVIDENCE_MODEL.md` |
```

Acceptance criteria:

- Every path in the table exists.
- The table includes `testing.md` and `templates.md`.
- No `adding-*.md` links remain unless corresponding files are intentionally created.

## Work item 2: Align top-level local checks with Phase 11

Update `docs/EXTENSIBILITY.md` Required Local Checks to either:

### Option A: Use one authoritative Make target

Recommended to prevent drift:

```bash
make check-architecture-ci
make check-feature-profiles   # if feature-gated code changed
```

Then link to `docs/extending/testing.md` for the expanded command list.

### Option B: List the full required Phase 11 command set

If the file should remain self-contained, include the complete command set:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --lib
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test command_registry
cargo test -p eggsec --test tool_registration --features rest-api
cargo test -p eggsec --test feature_matrix
cargo test -p eggsec --test enforcement_matrix
cargo test -p eggsec --test enforced_dispatch_regression
cargo test -p eggsec-output --test report_envelope
bash scripts/check-architecture-guards.sh
```

Then note:

```bash
# Equivalent one-command wrapper:
make check-architecture-ci
```

Recommended approach: Option A for top-level brevity, with a pointer to `docs/extending/testing.md` and `docs/CI_ARCHITECTURE_GUARDS.md` for full details.

Acceptance criteria:

- `docs/EXTENSIBILITY.md` no longer has a partial required-check list that can drift.
- It points to the authoritative Make target and detailed testing guide.

## Work item 3: Review all new docs for path consistency

Search for stale or non-existent guide names:

```bash
rg 'adding-(operation|domain|command|tool|tui|report|feature)|adding_tool|adding-operation|adding-domain|adding-command|adding-tool|adding-tui-action|adding-report|adding-feature' docs AGENTS.md CONTRIBUTING.md README.md
```

Search for actual guide filenames and make sure top-level references use them consistently:

```bash
rg 'docs/extending/(operations|domains|commands|tool-exposure|tui-actions|report-evidence|features|testing|templates)\.md' docs AGENTS.md CONTRIBUTING.md README.md
```

Acceptance criteria:

- No stale guide filenames remain in current docs.
- Top-level docs use the actual file names.

## Work item 4: Add lightweight Markdown link guard if practical

The current guard script checks existence of required docs but not whether Markdown links point to existing files. Add a lightweight guard only if it can remain simple and low-noise.

Preferred minimal implementation:

- Add a small shell section in `scripts/check-architecture-guards.sh` that checks only known Phase 12 extensibility guide links in `docs/EXTENSIBILITY.md`.
- Avoid implementing a full Markdown parser.

Example approach:

```bash
EXPECTED_EXT_LINKS=(
  "docs/extending/operations.md"
  "docs/extending/domains.md"
  "docs/extending/commands.md"
  "docs/extending/tool-exposure.md"
  "docs/extending/tui-actions.md"
  "docs/extending/report-evidence.md"
  "docs/extending/features.md"
  "docs/extending/testing.md"
  "docs/extending/templates.md"
)
for link in "${EXPECTED_EXT_LINKS[@]}"; do
  if ! rg -F "$link" docs/EXTENSIBILITY.md >/dev/null; then
    echo "FAIL: docs/EXTENSIBILITY.md missing link: $link"
    FAIL=$((FAIL + 1))
  fi
  if [[ ! -f "$link" ]]; then
    echo "FAIL: linked extensibility doc missing: $link"
    FAIL=$((FAIL + 1))
  fi
done
```

Optional broader check:

- Add a `scripts/check-doc-links.sh` later if a full repo-wide Markdown link checker is desired.
- Do not add a heavy dependency for this polish pass.

Acceptance criteria:

- The specific broken-link class found in `docs/EXTENSIBILITY.md` would fail CI if reintroduced.
- The guard is deterministic and does not parse remote URLs.

## Work item 5: Update docs references if guard/checklist changes

If the Make target becomes the preferred top-level workflow, update:

- `docs/EXTENSIBILITY.md`
- `docs/extending/testing.md` if it has a conflicting checklist
- `docs/CI_ARCHITECTURE_GUARDS.md` if it should explicitly state `EXTENSIBILITY.md` delegates to `make check-architecture-ci`
- `AGENTS.md` only if it repeats stale paths
- `CONTRIBUTING.md` only if it repeats stale paths

Acceptance criteria:

- No docs disagree about the required local handoff command.
- `make check-architecture-ci` remains the authoritative final local check.

## Work item 6: Validation

Run:

```bash
cargo fmt --all --check
bash scripts/check-architecture-guards.sh
make check-architecture-ci
```

If the guard script changes, also test that it fails on a temporary missing extensibility link/doc in a local throwaway edit, then revert before commit.

Optional fast docs search:

```bash
rg 'adding-(operation|domain|command|tool|tui|report|feature)' docs AGENTS.md CONTRIBUTING.md README.md
```

Acceptance criteria:

- Architecture guard script passes.
- `docs/EXTENSIBILITY.md` links all resolve to existing files.
- No stale `adding-*.md` guide references remain.

## Files likely to change

- `docs/EXTENSIBILITY.md`
- `scripts/check-architecture-guards.sh`
- possibly `docs/extending/testing.md`
- possibly `docs/CI_ARCHITECTURE_GUARDS.md`
- possibly `AGENTS.md` / `CONTRIBUTING.md` if stale path search finds issues

## Completion criteria

This docs polish is complete when:

- The top-level extensibility guide links to the actual Phase 12 guide files.
- The local-check section delegates to or exactly matches the Phase 11 architecture guard command set.
- The guard script catches missing required extensibility docs and, preferably, missing top-level extensibility links.
- Validation commands pass.
- No stale guide filenames remain in current docs.

## Handoff note

After this small polish pass, the architecture/extensibility roadmap can be considered closed. Future work should use the new guides and CI guards rather than adding more roadmap infrastructure unless a new major extension area is introduced.
