# Security Knowledge Corpus Milestone 001 — Redaction dead-code disposition

Status: closed (`plans/closure/security-knowledge-corpus/001-closure.md`; Option 3 executed — `utils/redaction.rs` deleted, guard check 147 added)

Repository baseline: `fix/cli-usability-audit` at `979dca67` (pushed); this plan lands on a branch cut from it

Source roadmap:

- `plans/subsystems/security-knowledge-corpus-roadmap.md#milestone-1--redaction-dead-code-disposition`

Long-term requirements:

- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/002-long-term-roadmap.md#phase-1--crate-boundaries-and-reusable-library-ownership`

Applicable ADRs:

- `plans/adrs/ADR-0005-knowledge-corpus-crate-ownership.md`

Primary class: polish (corrective)

## 1. Objective

Decide and execute one disposition for `crates/eggsec/src/utils/redaction.rs`, which is
366 lines with 26 tests and **zero production consumers**.

This milestone is corrective: it resolves a defect surfaced by the ADR-0005 analysis,
not an extraction.

## 2. Why this milestone is ready

No hard dependencies. `redaction.rs` depends only on `regex` and
`serde_json` (behind `redact_json`), is self-contained inside `utils/`, and has no
consumer to break. It is dependency-ready now and unblocks M003, whose scope depends on
whether redaction joins `eggsec-secrets` or is deleted.

## 3. Current implementation evidence

`crates/eggsec/src/utils/redaction.rs` declares exactly two public functions:

- `redact_sensitive(&str) -> String` — evidence-grade masking of bearer tokens, basic
  auth, API keys, AWS keys, JWTs, cookies, and private keys.
- `redact_json(&serde_json::Value) -> serde_json::Value` — recursive tree walk
  applying the same rules.

The module is publicly reachable at `eggsec::utils::redaction` (`utils/mod.rs:40`
declares `pub mod redaction;` and nothing else re-exports it).

**Consumer measurement.** A workspace-wide search for `redact_sensitive` and
`redact_json` returns no call into this module. Every hit is a different thing:

| Hit | Nature |
|---|---|
| `eggsec-python/src/checkpoint_store.rs:660` | an unrelated local `fn redact_json` |
| `eggsec-python/src/event_protocol.rs:370` | an unrelated local `redact_json_value` |
| `eggsec-python/src/integrations.rs:173,199,…` | a `redact_sensitive` **field/parameter name** |
| `eggsec-report-model/tests/roundtrip.rs:119` | the string literal `"redact_sensitive"` |
| `tool/protocol/mcp/handlers/server.rs:1612` | the JSON key `"redact_sensitive_data"` |

`utils/mod.rs` re-exports nothing from the module, so no glob import can reach it.

**Overlapping but distinct surface.** `eggsec-transport` maintains its own,
deliberately narrower redaction:

- `redacted_headers_debug` (`crates/eggsec-transport/src/headers.rs`) — masks values
  in an `http` header map for debug formatting.
- `redact_url_for_debug` (`crates/eggsec-transport/src/request.rs:565`) — masks URL
  userinfo and sensitive query parameters.

These are **not** duplicates of `redaction.rs`: they operate on `http::HeaderMap` and
`url::Url`, are part of a crate that must stay exactly `bytes`/`http`/`url`/`thiserror`
per guard 108, and serve debug formatting rather than evidence storage. Any unification
must not pull `regex` or `serde_json` into `eggsec-transport`.

**Precedent.** `Phase D` removed `utils::cache::ApiCache` for exactly this condition
(zero production consumers) and guard check 126 now hard-fails if
`crates/eggsec/src/utils/cache.rs` reappears
(check 126 at line 3457 of `scripts/check-architecture-guards.sh`, as of baseline
`979dca67`).

## 4. Invariants that must not regress

- `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate. Redaction is
  not a policy decision and must not acquire one.
- No `let _ =` or `filter_map(|e| e.ok())`; any newly wired redaction failure path is
  traced, never silently swallowed.
- `eggsec-transport` stays exactly `bytes`/`http`/`url`/`thiserror` (guard 108).
- `eggsec-policy` and `eggsec-transport` remain mutually independent (guard 122).
- No secret material may be logged unredacted by anything introduced here.
- Redaction, where it remains, is deterministic and side-effect free.

## 5. Scope

### In scope

- Choose and execute one disposition for `utils/redaction.rs`.
- Add a guard check in the shape of check 126 preventing silent recurrence.
- Record the disposition in `architecture/capability_segregation.md` (Phase G section),
  which guard 107 already requires to exist and which is the repo's canonical record for
  crate-boundary and ownership decisions.
- If the disposition is "move into `eggsec-secrets`", hand the module to M003 rather
  than moving it here.

### Explicitly out of scope

- Modifying `eggsec-transport`'s `redacted_headers_debug` or `redact_url_for_debug`.
- Adding `regex`/`serde_json` to `eggsec-transport`.
- Any new corpus crate. M003 creates `eggsec-secrets`.
- Tuning which patterns are masked. Content changes are not permitted.
- The other corpora (`service_data`, payloads). M002 and M004 own them.

## 6. Required production changes

The disposition is a decision the implementing agent must make from §3 evidence and
then execute. The three acceptable outcomes are:

### Option 1 — Adopt (wire up a real consumer)

Identify a genuine call site that should be masking evidence and is not. Candidates to
evaluate, in preference order:

1. Evidence or finding persistence paths that currently store raw values —
   `tool/finding.rs`, `findings/`, `output/report_summary.rs`.
2. MCP `coding_agent_output` rendering, where a finding's evidence is echoed to a model.
3. `nse_bridge.rs` report conversion.

If a consumer is adopted: call `redact_sensitive` / `redact_json` at that boundary,
preserve the function's existing public signature and all 26 tests unchanged, and record
the adopted call site in the closure record. **Changing behavior at the call site is in
scope** and is the entire point of this option; a caller that wants a different mask must
not silently bypass it.

If more than one call site needs a *different* masking policy, stop — that is a design
change, not a wiring fix, and belongs in an ADR.

### Option 2 — Consolidate (move to the transport crate)

Only if the agent can demonstrate that a debug-redaction need in `eggsec-transport` is
best served by `redaction.rs`. This requires proving `eggsec-transport` can keep exactly
`bytes`/`http`/`url`/`thiserror`, which means `regex` cannot come with it. **This option
is expected to fail** and is listed so the agent records a reason if it is rejected,
rather than silently skipping the question.

### Option 3 — Delete (the default)

Remove `crates/eggsec/src/utils/redaction.rs`, its `pub mod redaction;` declaration, and
its 26 tests, mirroring the `utils::cache::ApiCache` precedent exactly. Record the
deleted test count in the closure record so the loss is auditable.

Do **not** leave the file in place with a "TODO: unused" note — that recreates the
condition this milestone exists to resolve.

## 7. Ordered work packages

### Work package A — Decide

Intent: pick the disposition from §6 and write down why, before editing code.

Required changes: none.

Acceptance evidence: the chosen option and its rationale are recorded in the closure
record, including the consumer call site (Option 1) or the transport-constraint
analysis (Option 2).

### Work package B — Execute

Intent: implement the chosen disposition.

Required changes: as §6 for the chosen option.

Acceptance evidence: `cargo check --workspace --no-default-features` is green, and for
Option 1 the adopted call site has a test proving raw secret material does not reach the
sink.

### Work package C — Guard

Intent: make the recurrence impossible to miss.

Required changes: a new architecture guard check that fails if
`crates/eggsec/src/utils/redaction.rs` exists **and** has no production consumer. For
Option 1 this check should instead pin the adopted call sites so they cannot be
silently removed. Follow the shape of check 126.

Acceptance evidence: the guard is present, runs under `bash scripts/check-architecture-guards.sh`, and fails when the guard condition is artificially reintroduced.

### Work package D — Record

Intent: capture the decision where the repo looks for it.

Required changes: add a **Phase G** section to `architecture/capability_segregation.md`
recording the ADR-0005 analysis and this milestone's disposition. Guard 107 already
requires that file to exist; Phase G is where ADR-0005's decisions belong.

Acceptance evidence: guard 107 still passes; `architecture/capability_segregation.md`
references ADR-0005 and the roadmap.

## 8. Failure, cancellation, restart, and contention semantics

Not applicable in the usual sense: the functions are synchronous, pure, and stateless.
If Option 1 wires redaction into a persistence path, that call site's failure semantics
must match its existing behavior — a redaction failure must not abort evidence storage,
and must not silently store raw material. Trace and continue is acceptable only where the
value being written is already non-sensitive.

No tokio tasks are introduced, so no timeout wrappers are required. No cancellation or
restart behavior changes.

## 9. Compatibility and migration

No storage, protocol, configuration, or wire-format change. For Option 3, the removal of
`eggsec::utils::redaction::*` from the public path is a breaking change to a path with
zero consumers; confirm against the workspace-wide search in §3 before executing, and
disclose it in the closure record. If any external consumer is found, Option 3 is
blocked and Option 1 is required.

## 10. Required tests

### Focused unit tests

- Option 1: a test at the adopted call site proving secret material is masked before
  reaching the sink, including a negative case with non-sensitive input unchanged.
- Option 1: confirm the existing 26 `redaction.rs` tests still pass unmodified.

### Integration tests

- The adopted call site's owning integration suite (for example `tests/recon_*` or the
  MCP handler suite) passes.

### Restart and recovery tests

None. No persisted state changes.

### Contention and cancellation tests

None. No shared mutable state is introduced.

### Security and negative tests

- If Option 1: assert that a known fake credential in a bearer header, an `api_key`
  assignment, and an AWS access-key shape are all masked at the adopted sink.

### Migration and compatibility tests

- For Option 3: the workspace-wide search in §3 is re-run after deletion and must
  return no references to the removed path.

## 11. Required verification commands

```bash
make check                  # mandatory Rust contract
bash scripts/check-architecture-guards.sh
make check-deps             # if Option 2 was attempted
```

Do not claim commands that were not actually run in the closure record.

## 12. Documentation updates

- `architecture/capability_segregation.md` — Phase G section (work package D).
- `architecture/utils.md` — reflect the disposition and the module's new location or
  absence.
- `architecture/overview.md` — only if a new crate appears (it does not in this
  milestone).
- `plans/subsystems/security-knowledge-corpus-roadmap.md` — M001 status once closed.
- `plans/registry.md` — M001 status.

## 13. Acceptance criteria

- Exactly one disposition from §6 is executed, or Option 2 is rejected with a recorded
  reason and one of Options 1/3 is executed.
- `make check` is green.
- A guard check exists that makes the chosen failure mode non-recurring.
- `architecture/capability_segregation.md` carries a Phase G record referencing ADR-0005.
- No new workspace dependency is introduced in any crate.
- For Option 3, the deleted test count is disclosed in the closure record.

## 14. Stop conditions

The agent must stop and report rather than improvise when:

- a genuine external consumer of `eggsec::utils::redaction::*` is discovered, which
  blocks Option 3;
- the adopted call site turns out to need a masking policy that differs from
  `redaction.rs`, which makes this a design change requiring an ADR;
- `eggsec-transport` cannot keep its four-dependency closure under Option 2;
- executing the disposition would require weakening a canonical invariant;
- scope would expand into M002, M003, or M004.

## 15. Closure evidence required

`plans/closure/security-knowledge-corpus/001-closure.md` containing:

- the chosen option and its rationale;
- for Option 1, the adopted call site, its test, and confirmation that the 26 existing
  tests pass unmodified;
- for Option 3, the deleted test count and the post-deletion consumer search output;
- the exact commands run with their outcomes, including `make check` and the guards;
- the new guard check's name and what it fails on;
- the `architecture/capability_segregation.md` Phase G diff;
- residual findings classified by severity;
- recommendation: closed, conditionally closed, corrective pass required, or blocked.

## 16. Handoff notes

The 26 tests in `redaction.rs` are substantial and passing; deleting them is a real loss
of tested behavior and must be disclosed, not treated as cleanup. The agent should read
`crates/eggsec/src/utils/mod.rs`'s module doc comment, which asserts what `utils`
still owns, and update that comment to match the disposition.

Preserve unrelated user changes. Two untracked plan files
(`plans/c2-real-simulation-differentiation-plan.md`, `plans/post-exploit-plan.md`) exist
in the working tree and belong to the user — do not modify or delete them.