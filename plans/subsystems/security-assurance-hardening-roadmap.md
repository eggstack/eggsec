# Security Assurance Hardening Roadmap

Status: conditionally closed

Long-term references:

- `plans/000-long-term-specification.md` (enforcement boundary, authorized-use contract)
- `plans/001-terminology-and-domain-model.md`
- `plans/002-long-term-roadmap.md`
- `AGENTS.md` "Enforcement (critical)" and "Dispatch / scope / runtime ownership"

Related ADRs:

- None required to start. An ADR IS required before the daemon HTTP adapter is
  wired to a listener (design decision recorded in Milestone 005's stop
  conditions, not decided here).

Origin: static security audit of baseline `77188755`, 2026-11-17. The audit
report is the evidence source for this roadmap; it is not itself a tracked
document, so each finding is restated with file-level evidence in the milestone
plans.

## 1. Purpose and ownership boundary

This workstream owns remediation of the audit findings that affect Eggsec's own
trustworthiness rather than the capability set:

- integrity of the manual-surface approval gate;
- memory safety of the Python buffer exports;
- admission control on automated network surfaces (`mcp-serve`, `codegg-mcp`,
  and the dormant daemon HTTP adapter);
- secret and credential egress paths (process environment, unverified TLS,
  remote worker command policy);
- hygiene of shipped artifacts and generated output (container build, lab
  compose defaults, third-party script loading, dependency-exception review).

It does **not** own:

- new scan capabilities or operation descriptors;
- transport, policy, or runtime architecture redesign;
- the `chrono` migration (tracked in `docs/DEPENDENCY_EXCEPTIONS.md` with a
  2026-12-12 review-by date);
- the NSE workstream (`plans/subsystems/nse-runtime-extraction-roadmap.md`).

Ownership note: milestones 001, 003, 004, 006 touch the `eggsec` engine and
`eggsec-tui`; 002 touches `eggsec-python` only; 005 touches `eggsec-daemon`
only. No milestone may alter `eggsec-policy` semantics.

## 2. Work classification

### Invariants

- **I1 — An approval token must be consumed, not merely accepted.** Today
  `spawn_task` receives `_approved: Option<ApprovedOperation>` and never reads
  it. The invariant "manual dispatch is gated by an `ApprovedOperation`" is
  therefore documentation, not structure.
- **I2 — A buffer export transfers an owner reference.** Setting
  `Py_buffer.obj` to null while `shape` points into the Rust object hands the
  consumer an unmanaged pointer.
- **I3 — An automated surface cannot become network-reachable without an
  explicit, reviewed decision.** The daemon has `allow_public_bind`; the MCP
  server does not; the daemon HTTP adapter has auth disabled by default.
- **I4 — Secrets do not enter the process environment and do not cross
  unverified TLS.** Both currently happen (storage password; proxy health
  credentials).
- **I5 — Remote command execution is an explicit trust grant, not a
  source-coded command sandbox.** A PSK-authenticated peer may run arbitrary
  programs as the listener account; transport authentication, host isolation,
  and resource bounds must be documented and preserved.

### Capabilities

- None new. This roadmap intentionally adds no user-facing capability; its
  success criterion is that existing capabilities keep their claims.

### Infrastructure

- A shared bind-address admission helper (loopback / explicit opt-in), used by
  both the MCP server and (later) the daemon HTTP adapter. Location must respect
  crate direction: the engine cannot depend on `eggsec-daemon`, so the helper
  lives engine-side and the daemon keeps its own copy until a neutral home
  exists.
- An explicit `AuthMode` for the daemon HTTP configuration (no `Default`).
- A `RunRequest` ↔ `ApprovedOperation` binding check alongside the existing
  `ToolRequest` binding check.

### Polish

- Dockerfile/compose/stray-root-file cleanup.
- Subresource integrity or vendoring for the report's chart script.
- Dependency-exception review record refresh.

## 3. Non-goals

- No change to `ExecutionProfile` semantics, denial classes, or
  `OperationMetadata` policy entries.
- No new automated-surface exposure of manual-only operations.
- No manager/worker protocol redesign; milestone 004 narrows what a remote peer
  may invoke, it does not change the wire format.
- No `chrono` replacement.
- No wiring, exposing, or documenting the daemon HTTP API as a supported
  feature.
- No wholesale rewrite of the Python bindings.

## 4. Current state

Audit evidence at baseline `77188755`:

| ID | Severity | Finding | Location |
|---|---|---|---|
| A1 | medium | TUI approval token accepted but unused; "structural gate" claim false | `crates/eggsec-tui/src/app/task_runtime.rs:229`; claims at `app/mod.rs:637`, `app/enforcement_facade.rs:169` |
| A2 | medium | Python `__getbuffer__` exports `obj = NULL` with `shape` pointing into the object (two sites) | `crates/eggsec-python/src/buffer_support.rs:84,91`; `artifact.rs:159,165` |
| A3 | medium | MCP HTTP server accepts any bind address; auth optional; no loopback guard | `crates/eggsec/src/commands/handlers/serve.rs:110-125`; `crates/eggsec/src/cli/misc.rs:318-333` |
| A4 | medium (latent) | Daemon HTTP adapter: auth off by default, header-presence-only identity, anonymous ownership bypass, no Host/Origin validation; module unwired | `crates/eggsec-daemon/src/http.rs:23-36,96-101,609-630`; `host.rs:885` |
| A5 | low | Proxy health probe disables certificate verification, so a network attacker can forge a `2xx` and mark a bad proxy healthy (corrected: the flag covers the target fetch, not the proxy hop, so this is signal integrity, not credential disclosure) | `crates/eggsec-web-proxy/src/utils.rs:27-45`; `health.rs:111-126` |
| A6 | low | Storage password written to the process environment (child-process and `/proc` exposure); the TUI constant's name diverges from the documented CLI variable; `set_var` under a multi-threaded runtime | `crates/eggsec-tui/src/app/mod.rs:625-631`; `app/task_management.rs:18`; `app/runner.rs:239` |
| A7 | low | Distributed worker validates `eggsec` arguments by denylist; binary resolved via `PATH` | `crates/eggsec/src/distributed/command.rs:18-32,190` |
| A8 | low | HTML report loads Chart.js from a CDN with no SRI or CSP | `crates/eggsec-output/src/html.rs:68` |
| A9 | low | Stale Dockerfile (rust 1.80 vs MSRV 1.89, removed `eggsec-nse` crate path); hardcoded lab credentials; stray root `config.toml` | `Dockerfile:5,18`; `docker-compose.yml`; `config.toml` |
| A10 | low | 16 advisory ignores, 7 open Dependabot alerts, all mapped to documented exceptions; `db-pentest` ships in `full` | `deny.toml`; `docs/DEPENDENCY_EXCEPTIONS.md`; `crates/eggsec/Cargo.toml:360` |

Verified-sound controls are recorded in the audit and MUST NOT be weakened by
this workstream: approval binding, exclusion-first scope evaluation,
all-addresses-must-match with mixed-answer denial, TOCTOU-closed transport
authority, constant-time PSK/API-key comparison, `0600` socket and session
files, path-traversal validation, CSV formula neutralization, SHA-pinned CI.

## 5. Target architecture

- Manual dispatch: `spawn_task` takes a non-optional `ApprovedOperation`, binds
  it to the `RunRequest` (operation id + normalized target) before submitting,
  and returns an error on mismatch. `execute_canonical` / `dispatch_inner` stop
  being reachable as general-purpose public entry points; the TUI keeps its
  single guarded path.
- Automated surfaces: one admission rule — loopback by default, non-loopback
  requires an explicit opt-in *and* a credential. The rule is applied at
  configuration validation time, before `bind`, so a misconfiguration cannot
  degrade into an open listener.
- Python: buffer exports set `(*view).obj = slf.into_ptr()`; `shape` remains a
  field so it stays valid for the export's lifetime, now guaranteed by the
  owner reference.
- Secrets: credentials travel through typed in-memory handles or environment
  variables set by the operator before launch, never written by the process.
- Egress: proxy health treats reachability as the signal and does not attach
  credentials to an unverified channel; human-operated remote commands remain
  operator-selected, without source-coded executable or argument-pattern
  filters, under an explicit PSK/TLS trust boundary and resource limits.

## 6. Dependency graph

```text
001 enforcement gate integrity ──┐
                                 ├─(soft: same TUI files)──> 004 secret/credential egress
002 python buffer lifetime       │
                                 │
003 MCP admission control        │
                                 │
005 daemon HTTP adapter safety   │
                                 │
006 supply-chain/output hygiene ─┘   (independent)
```

- 001 → 004: **soft**. Both edit `eggsec-tui` task-management files; serialize
  them to avoid conflicting edits.
- 002, 003, 005, 006: **independent**. No hard dependencies exist; all
  prerequisites are already in the tree.
- 005 → future wiring: **interface**. The `AuthMode` contract introduced in 005
  is the interface the eventual wiring milestone consumes; wiring itself is out
  of scope.

No milestone is blocked at proposal time.

## 7. Milestones

### Milestone 1 — Enforcement gate integrity

Class: invariant

Objective: make the manual-surface approval token load-bearing and narrow
unchecked dispatch entry points.

Dependencies: none.

Deliverable boundary: `eggsec-tui` `spawn_task` signature and binding check; a
`RunRequest` binding helper in the engine; visibility tightening for
`dispatch::execute_canonical` / `dispatch::dispatch_inner`; regression test and
one static guard.

User or operator value: the documented enforcement claim becomes true, so a
future TUI call site cannot silently bypass policy.

Exit conditions: no `Option<ApprovedOperation>` remains on the dispatch path;
mismatched operation/target is rejected with a typed error; guard fails if the
unused parameter returns.

Deferred work: no change to confirmation UX; no new manual override semantics.

### Milestone 2 — Python buffer protocol lifetime

Class: invariant

Objective: every `Py_buffer` export transfers a real owner reference.

Dependencies: none.

Deliverable boundary: the two `__getbuffer__` implementations, plus
owner-reference regression tests for `BinaryBuffer` and `Artifact`.

User or operator value: Python consumers can no longer observe a pointer whose
owner may already be freed; the binding stops violating PEP 3118.

Exit conditions: `memoryview(obj).obj is obj` holds for both types; a
GC-after-drop test still reads correct bytes; `make check-python` passes.

Deferred work: no redesign of the buffer API; no new `BinaryBuffer` methods.

### Milestone 3 — MCP surface admission control

Class: invariant

Objective: make `mcp-serve` / `codegg-mcp` non-loopback exposure an explicit,
authenticated decision.

Dependencies: none.

Deliverable boundary: bind validation and credential requirement in
`handle_mcp_serve`, affecting both subcommands through their shared handler;
CLI help text; unit tests for the admission matrix.

User or operator value: a mistyped `--bind` cannot expose unauthenticated scan
orchestration to the network.

Exit conditions: non-loopback without `--api-key` is refused; non-loopback
without explicit opt-in is refused; loopback defaults are unchanged; `--stdio`
is unaffected.

Deferred work: no OAuth/identity provider integration; no TLS listener for MCP.

### Milestone 4 — Secret and credential egress

Class: invariant

Objective: stop leaking credentials into the process environment and across
unverified TLS, while preserving human-driven remote command flexibility and
recording the authenticated-peer trust boundary.

Dependencies: soft on Milestone 1 (shared TUI files).

Deliverable boundary: storage-password handoff in `eggsec-tui`; proxy health
probe behaviour; distributed worker command policy; tests for each.

User or operator value: credentials stop being readable by sibling processes,
a network attacker can no longer forge a healthy proxy result, and operators
retain broad remote execution while understanding that a PSK grants arbitrary
OS-command execution as the listener account.

Exit conditions: no process-written secret environment variable remains; the
health probe verifies the target certificate by default and does not attach
credentials to a verification-disabled client; the worker accepts arbitrary
operator-selected programs and arguments without source-coded filters, while
retaining argument/environment/timeout/output bounds and documenting the
PSK-authenticated peer trust grant.

Deferred work: no secret-manager integration; no change to the PSK wire format.

### Milestone 5 — Daemon HTTP adapter safety

Class: invariant (latent surface)

Objective: make the dormant HTTP adapter safe by construction without wiring
it.

Dependencies: none. (The wiring milestone that follows is out of scope and
requires an ADR.)

Deliverable boundary: `HttpConfig` without `Default`, an explicit
`AuthMode`, bind validation that couples non-loopback to a credential, Host and
Origin validation, and fail-closed ownership for anonymous callers.

User or operator value: when the adapter is eventually wired, its defaults
cannot silently reproduce a DNS-rebinding-reachable, unauthenticated API.

Exit conditions: no configuration path yields non-loopback plus no credential;
anonymous callers cannot read snapshots or mutate sessions; `Host`/`Origin`
mismatches are rejected; the module remains unwired.

Deferred work: token issuance/rotation, TLS termination, and the wiring ADR.

### Milestone 6 — Supply-chain and output hygiene

Class: polish

Objective: remove stale/dirty shipped artifacts and unreviewed third-party
execution from generated output.

Dependencies: none.

Deliverable boundary: Dockerfile repaired to the real MSRV and crate set;
compose credentials parameterized; stray root files dispositioned; report chart
loading pinned; dependency-exception review refreshed.

User or operator value: the documented container build works, lab defaults stop
looking production-safe, and an analyst opening a report does not execute
unpinned third-party code.

Exit conditions: image builds at the workspace MSRV; compose contains no
hardcoded password; root `package.json`/`package-lock.json`/`config.toml` are
either removed or moved to a documented fixture location; report HTML carries
an integrity or vendored script; exception review record updated with a new
review-by date.

Deferred work: no base-image digest pinning programme; no SBOM policy change.

## 8. Cross-cutting requirements

### Storage and migration

No persistent schema changes. The Python change alters no serialized format. The
daemon HTTP change alters only an unwired config type. Root-file dispositioning
must not move `examples/configs/*.toml`, which the Dockerfile copies.

### Protocol and compatibility

`mcp-serve` and `codegg-mcp` share `handle_mcp_serve`; both must require the
operator-selected MCP tools and preserve strict loaded-scope enforcement. The
distributed executor remains human-driven and broad by explicit decision; it
retains bounded resource use but does not impose a source-coded command list.

### Security and authorization

Every milestone preserves: `EnforcementContext::evaluate()` as the mandatory
gate; strict surfaces dispatch only via
`EnforcedDispatcher::dispatch_execution()`; `OperationMetadata` as the single
policy source; adapters never calling `Scope::is_target_allowed` or
`tool.execute()`.

### Concurrency, cancellation, and recovery

No new spawned tasks. `set_var` removal eliminates a data race rather than
adding synchronization. Buffer exports must remain valid across GC, which is the
point of the owner reference.

### Observability and audit

New refusals (bind admission, worker command policy, binding mismatch) emit
`tracing` warnings with the deciding rule. No secret values in any log line.

### Performance and resource use

No measurable change. The Python fix adds one reference count per export.

### Documentation and operations

`AGENTS.md`, `architecture/tui.md`, `architecture/python_api.md`,
`architecture/daemon.md`, `architecture/supply_chain.md` (SBOM input note), and
`docs/DEPENDENCY_EXCEPTIONS.md` are updated where a claim changes.

## 9. Verification strategy

- Mandatory for every milestone: `make check`.
- Milestone 2: `make check-python` (builds into `.venv-ci/`) plus
  `pytest crates/eggsec-python/tests/test_buffer.py`.
- Milestone 3: `cargo test -p eggsec --features rest-api,cli`.
- Milestone 5: `cargo test -p eggsec-daemon --features http-api` plus
  `make check-no-default`.
- Milestone 6: `make check-deps`, Docker build, and `make check`.
- `make check-features-individual` only if a feature matrix changes.

Deep checks (`make check-full`, `make test-tui-pty`) are not per-PR
requirements but ARE expected for Milestone 1 (TUI dispatch) and Milestone 4
(distributed worker).

## 10. Risks and decision points

| Risk / decision | Handling |
|---|---|
| Tightening `spawn_task` may require a new engine-public binding API | Bounded: the plan permits a `dispatch::binding` helper; it must not change policy semantics. Stop if it requires touching `eggsec-policy`. |
| The Python owner-reference fix may surface a pyo3 version constraint | Stop and report; do not vendor or patch pyo3. |
| Requiring an API key may break an existing non-loopback deployment | Admission is opt-in, not removal: `--allow-public-bind` remains available; the change adds a required credential, not a hard ban. |
| Removing the storage-password env bridge may strand daemon-attached setups | TUI-local dispatch works today with the TUI's own `EGSEC_`-style name; the plan must preserve that path (honour both names during transition or update the field default), and must prove the local path end to end. |
| Dropping credentials from the proxy health probe changes "authenticated" health semantics | Decision recorded in the plan: reachability (including 407) is health; authentication is not the probe's job. |
| Daemon HTTP auth model is unresolved | Out of scope. Milestone 5 introduces the `AuthMode` shape only; selecting an issuance/rotation model requires an ADR before wiring. |
| Deleting root `package.json` may break SBOM fixture expectations | The file is an SBOM *input*, not a build file. Disposition is recorded explicitly in Milestone 6 rather than assumed. |

## 11. Completion definition

The roadmap closes when:

1. all six milestones are closed with closure records;
2. no `Option<ApprovedOperation>` exists on a dispatch path;
3. both `__getbuffer__` implementations transfer an owner reference and the
   regression tests prove it;
4. `mcp-serve`/`codegg-mcp` cannot bind non-loopback without an explicit opt-in
   and a credential;
5. no process-written secret environment variable and no credential-bearing
   request over unverified TLS remain;
6. the daemon HTTP adapter cannot be configured into an open listener and stays
   unwired;
7. the container build matches the workspace MSRV and lab defaults no longer
   carry hardcoded production-shaped passwords;
8. `make check` passes on the closing baseline.

## 12. Milestone status

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| 1 enforcement gate integrity | closed | `plans/implementation/security-assurance-hardening/001-enforcement-gate-integrity.md` | `plans/closure/security-assurance-hardening/001-status.md` | — |
| 2 python buffer lifetime | closed | `plans/implementation/security-assurance-hardening/002-python-buffer-protocol-lifetime.md` | `plans/closure/security-assurance-hardening/002-status.md` | — |
| 3 MCP admission control | closed | `plans/implementation/security-assurance-hardening/003-mcp-surface-admission-control.md` | `plans/closure/security-assurance-hardening/003-status.md` | — |
| 4 secret/credential egress | closed | `plans/implementation/security-assurance-hardening/004-secret-and-credential-egress.md` | `plans/closure/security-assurance-hardening/004-status.md` | — (soft ordering after M1 satisfied) |
| 5 daemon HTTP adapter safety | closed | `plans/implementation/security-assurance-hardening/005-daemon-http-adapter-safety.md` | `plans/closure/security-assurance-hardening/005-status.md` | — |
| 6 supply-chain/output hygiene | conditionally closed | `plans/implementation/security-assurance-hardening/006-supply-chain-and-output-hygiene.md` | `plans/closure/security-assurance-hardening/006-status.md` | Docker image build/runtime and Compose set/unset validation unavailable; see M6 closure §10 |
