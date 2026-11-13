# Desktop GUI Frontend Milestone 001 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/desktop-gui-frontend/001-operation-parameter-schema-publication.md`

Source subsystem roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md`

Repository baseline reviewed: `203ab5dd958415340d39cd47fb1add432c687edd`

Implementation commits or pull requests:

- Uncommitted working-tree change on branch `feat/auto-20261022-76692679`.

## 1. Executive finding

The milestone's capability boundary is complete. The operation-parameter gap
that made a GUI expensive — no operation published the shape of its own request
parameters — is closed by a **derived**, never-transcribed JSON Schema served
over one read-only daemon command.

Three things make this trustworthy rather than merely present:

1. The schema is **derived** from the canonical request DTOs, so it cannot
   describe a shape the runtime would reject.
2. The published identity and policy metadata are **read** from
   `OperationMetadata`, so the wire description cannot disagree with the
   engine's own operation catalog.
3. The artifact is **descriptive only**. It is unreachable from every
   authorization, dispatch, scope, and policy path, and that is enforced
   structurally by a guard rather than by convention.

What the milestone deliberately did **not** deliver: no GUI crate, no shell, no
form rendering. Those are M4–M6, and M4 is blocked on the ADR-0009 ratification.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| Schema derived from the canonical request DTOs, never transcribed | `crates/eggsec-runtime/src/request.rs` — `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]` on `TaskKind` and all 30 params structs | pass | The derive cannot live in another crate: `schemars` cannot implement `JsonSchema` for a type it does not own. |
| Published identity read from the canonical accessors, not copied | `crates/eggsec-runtime/src/schema.rs` — `task_kind_schemas()` calls `TaskKind::operation_id()` / `TaskKind::capability_name()` | pass | A sample value per variant exists only to reach those accessors; it is never submitted. |
| Catalog metadata read from `OperationMetadata`, not copied | `crates/eggsec-daemon-protocol/src/operations.rs` — `describe_operation()` | pass | Proven field-for-field by `published_operations_match_the_catalog_field_for_field`. |
| No hand-written parameter table | Guard 152 | pass | No frontend-neutral crate outside `crates/eggsec-runtime/src/request.rs` may define a `pub struct *Params`. |
| The schema never appears on an authorization path | Guard 151 | pass | No reference to `schemars`, `schema_for!`, `eggsec_runtime::schema`, `describe_operations`, or `DescribeOperations` under the engine's dispatch, runtime-bridge, config-policy, config-scope, or the policy kernel; and neither the engine nor the policy kernel gained a `schemars` dependency. |
| One read-only command serves the schema | `ClientCommand::DescribeOperations` → `ServerMessage::Operations` | pass | Read-only: creates no session, mutates no task state, takes part in no authorization decision. |
| The daemon's default dependency set stays `eggsec-runtime` + `eggsec-daemon-protocol` only | `crates/eggsec-daemon/Cargo.toml` gained **no** dependency | pass | The catalog is read in the protocol crate, which gained `eggsec-policy`. See §7 for the recorded deviation from the plan's wording. |
| Older daemons degrade gracefully | `ErrorCode::UnsupportedCommand` path; `DaemonCapabilities.operation_schema` is `#[serde(default)]` | pass | A client that sees `false`, or that gets `UnsupportedCommand`, must degrade to a static form set rather than fail. |
| Access control: a declared client is required | `crates/eggsec-daemon/src/host.rs` — the `DescribeOperations` arm is preceded by the same declaration check the other daemon-wide reads use | pass | The command carries no session id, so it is answerable before any session exists. |
| No credential, secret, or scope material in the response | `operations.rs::response_carries_no_credential_or_scope_material` | pass | Closed field set on the metadata half; no `cidr` / `scope_rules` / `api_key` / `secret` / `bearer` anywhere in the payload. |
| The published variant set cannot drift from `TaskKind` | `schema.rs::published_variants_cover_every_task_kind`, `per_variant_and_whole_enum_schemas_agree` | pass | The per-variant map and the whole-enum adjacently-tagged document must agree. |

## 3. Production implementation evidence

**Derivation (`crates/eggsec-runtime`).** The crate gained an optional
`schemars` dependency and a `schema` feature, off by default. A new
feature-gated module `crates/eggsec-runtime/src/schema.rs` publishes:

- `task_kind_schemas() -> Vec<TaskKindSchema>` — one entry per `TaskKind`
  variant (30), each carrying the wire `kind` discriminator, the canonical
  `operation_id`, the `capability_name`, and a JSON Schema document for that
  variant's params struct;
- `task_kind_schema() -> serde_json::Value` — the whole `TaskKind` schema in
  its adjacently-tagged wire shape (`{"kind": ..., "params": ...}`), so a
  client can post a valid payload without guessing the envelope.

The schema is returned as `serde_json::Value`, so `schemars` types never enter
the runtime crate's public API.

**Catalog publication (`crates/eggsec-daemon-protocol`).** A new module
`operations.rs` defines `OperationDescription`, `TaskKindDescription`, and
`DescribeOperationsResponse`, plus `describe_operations()`. Every identity and
policy field is read from `eggsec_policy::ALL_OPERATION_METADATA`.

**Wire surface.** `ClientCommand::DescribeOperations` (16 variants, was 15) and
`ServerMessage::Operations` (15 variants, was 14). `DaemonCapabilities` gained
`operation_schema: bool`, marked `#[serde(default)]`.

**Daemon arm (`crates/eggsec-daemon/src/host.rs`).** The command is answered
from immutable registry data — no session, no task state, no registry mutation —
and is concurrency-safe and reproducible.

## 4. Verification executed

### Commands run

```bash
cargo test -p eggsec-runtime --features schema
cargo test -p eggsec-daemon-protocol
cargo test -p eggsec-daemon --lib
make check-deps
bash scripts/check-architecture-guards.sh
cargo fmt --all --check
cargo clippy -p eggsec-runtime --features schema -p eggsec-daemon-protocol -- -D warnings
```

### Results

| Check | Result |
|---|---|
| `eggsec-runtime` (with `schema`) | 92 lib tests pass, including 6 new schema tests |
| `eggsec-daemon-protocol` | 79 tests pass, including 5 new `operations` tests and 2 new protocol tests |
| `eggsec-daemon` | 92 lib tests pass, including 6 new tests |
| `make check-deps` | `bans ok, licenses ok, sources ok`; `advisories FAILED` for pre-existing reasons — see §10 |
| Architecture guards | ALL PASSED, including new checks 151 and 152 |
| Guard negative test | Deliberate violations of checks 151 and 152 each failed as designed; both passed again after restore |
| `cargo fmt --all --check` | clean |
| Clippy on the changed crates | clean |

The **load-bearing negative test** the plan required was implemented
structurally rather than as a single runtime assertion: guard 151 makes the
schema unreachable from every authorization path, so the engine *cannot* consult
it, and therefore an enforcement decision cannot depend on its presence. The
guard was demonstrated to fail when the condition is violated and to pass when
restored.

## 5. Invariant review

| Invariant | Evidence | Status |
|---|---|---|
| `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate | Guard 151: no dispatch-path file references the schema. No dispatch code was changed. | holds |
| Strict surfaces fail closed without an explicit scope | No strict path was touched; the engine's strict-surface code is unchanged. | holds |
| `OperationMetadata` is the single source of truth for operation policy | The published description is read from it field for field (`published_operations_match_the_catalog_field_for_field`); the catalog itself was not modified. | holds, strengthened |
| `TaskKind::operation_id()` / `canonical_target()` remain the single wire-side match | The schema module calls those accessors; it does not define a parallel table. | holds |
| The daemon's default dependency set | `crates/eggsec-daemon/Cargo.toml` gained no dependency. | holds |
| Workspace Tokio baseline (`default-features = false`, per-crate lists) | `schemars` pulls no Tokio. | holds |
| TLS is ring-only | `schemars` pulls no TLS stack. | holds |
| Never `let _ =` / `filter_map(|e| e.ok())` | No such pattern was introduced; the schema module has no fallible discard. | holds |
| `make check` contract | Extended, not weakened: the `Makefile` clippy target now also lints `eggsec-runtime --features schema` and `eggsec-daemon-protocol`. | holds |

## 6. Failure and recovery review

- **Duplicate delivery:** `describe_operations()` is a pure function of
  immutable registry data. `payload_is_reproducible` asserts two calls produce
  identical payloads.
- **Cancellation races:** not applicable — the command creates no task.
- **Daemon restart:** not applicable — the command reads no persisted state, so
  it answers identically before and after a restart.
- **Partial persistence failure:** not applicable — the command writes nothing.
- **Malformed or unauthorized input:** the command takes no fields beyond its
  own identity, so there is no input to malform. An undeclared client is
  rejected with `ErrorCode::ClientNotDeclared` before the arm runs.
- **Denial-of-service bounds:** the payload is bounded by the catalog (34
  operations) and the task-kind set (30). The command creates no task, so it
  consumes no runtime capacity and cannot be used to flood the engine.
- **Concurrent access:** `describe_operations_is_concurrency_safe` asserts two
  concurrent calls return identical payloads.

## 7. Migration and compatibility review

- **Schema migration:** none — the schema is a derived artifact, not stored
  state.
- **Backward compatibility:** `DaemonCapabilities.operation_schema` is
  `#[serde(default)]`, so a newer daemon's capabilities decode against an older
  client and vice versa. `daemon_capabilities_decode_without_the_operation_schema_flag`
  pins this.
- **Protocol negotiation:** the command is additive. A daemon too old to know it
  answers `ErrorCode::UnsupportedCommand`; the plan requires a client to treat
  that as "schema unavailable" and degrade to a static form set rather than
  fail.
- **Configuration behavior:** unchanged.
- **Rollback:** the feature and the command are additive; reverting removes
  them without a data migration.
- **Legacy-path status:** no legacy path was removed. The optional `schema`
  feature means `eggsec-runtime`'s default build gains no new dependency for
  consumers that do not enable it.

**Recorded deviation from the plan's wording.** The plan's §4 says "any new
dependency must be feature-gated and optional". That clause is written about
*the daemon*, and the daemon gains no new dependency. The catalog is read in
`eggsec-daemon-protocol`, whose dependency on `eggsec-policy` is **not**
feature-gated, for a concrete reason: gating it would make the `ServerMessage`
variant set depend on the build, and a client built without the feature could
not deserialize an `Operations` message at all. That is a wire-contract hazard
for a published IPC protocol, and the repo already has a guard asserting the
protocol crate carries no Tokio and no network dependency — `eggsec-policy` is a
pure semantic kernel and satisfies both.

**Recorded deviation from the plan's drift detector.** The plan's acceptance
criterion reads "the schema's variant keys equal the set of
`TaskKind::operation_id()` values". That is unachievable as written: 30 task
kinds map to 26 distinct operation ids, because the three packet kinds share
`packet`, `Pipeline` and `Resume` share `pipeline`, and `Wireless` and
`WirelessActive` share `wireless`. A map keyed by operation id would silently
drop four wire shapes. The published artifact is therefore keyed by
**task-kind variant** (30 entries), each carrying its `operation_id`, and
`shared_operation_ids_are_the_expected_ones` pins the exact multiplicities.
This is a strictly stronger drift detector than the plan's wording.

## 8. Security review

- **Authorization:** the schema is descriptive and grants nothing. The engine's
  own enforcement remains the only authority, and the daemon enforces it.
  Guard 151 proves the schema cannot be consulted on an enforcement path.
- **Scope checks:** the published payload contains no scope material and no
  scope rule (`response_carries_no_credential_or_scope_material`).
- **Secret handling:** no credential, secret, or bearer material is published.
  The parameter half is a JSON Schema — property names, types, and the serde
  defaults of the request structs — which contains no request *values*.
- **Privilege boundaries:** the command requires a declared client identity, so
  it is not reachable by an anonymous peer on the socket.
- **Denial-of-service bounds:** the payload is bounded by the catalog size and
  the command creates no task.
- **Audit behavior:** the command is read-only and creates no state. It is not
  separately audited; the daemon's existing command-level audit trail covers
  command dispatch.
- **Redaction:** not applicable — nothing in the payload is secret.

## 9. Documentation and operations

- `architecture/overview.md` — crate table, module index, dependency map,
  dependency-guardrails list, and the `eggsec-runtime` / `eggsec-daemon-protocol`
  rows updated for the optional `schema` feature, the `operations` module, the
  `eggsec-policy` dependency, and the new command/message counts.
- `architecture/runtime.md` — the `schema` module and the optional `schemars`
  dependency.
- `architecture/daemon.md` — the `DescribeOperations` command, the `Operations`
  response, the `DaemonCapabilities.operation_schema` flag with its
  `#[serde(default)]` compatibility rule, and the audit-trail surface labels.
- `docs/DAEMON_PARITY.md` — the `DaemonCapabilities` shape and an
  operation/schema-discovery parity row.
- `docs/CI_ARCHITECTURE_GUARDS.md` — a "Checks 151–154" section.
- `.opencode/skills/eggsec-daemon/SKILL.md` — the new command and the
  capability flag.
- `plans/000-long-term-specification.md` — crate count and the
  `eggsec-frontend-client` ownership rule.
- `plans/registry.md` and `plans/subsystems/desktop-gui-frontend-roadmap.md` —
  milestone status updated.
- Static guards: 151 (schema stays off the authorization path) and 152 (no
  hand-written parameter table).

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| medium | `make check-deps` reports `advisories FAILED` against `rustls 0.21.12` (via `tiberius` → `sqlx-mysql`) and related crates — RUSTSEC-2026-0346/0347/0348/0349, 0367/0368/0369, 0381/0382/0383, RUSTSEC-2024-0449. None is in this milestone's dependency delta, and none of the four crates this milestone added (`schemars`, `schemars_derive`, `dyn-clone`, `serde_derive_internals`) appears in any advisory. The `deny.toml` advisory ignore list was last reviewed 2026-09-13. | The mandatory dependency gate stays red for reasons unrelated to this milestone. | Maintainer-led dependency-exception review. `deny.toml` is the canonical place; each exception needs an owner and a review-by date in `docs/DEPENDENCY_EXCEPTIONS.md`. **Deliberately left unfixed here.** |
| low | `clipboard-win 5.4.1` (→ `arboard` → `eggsec-tui`) was yanked upstream on 2026-10-23 and was already in the baseline lock file. | `cargo deny` failed on the yanked crate before this milestone could run the dependency gate at all. | **Repaired**: one-line lock bump to the non-yanked `5.4.3`. Changes no first-party dependency edge and no first-party code. |
| low | The plan's drift-detector wording is unachievable as written (30 task kinds → 26 operation ids). | None — the implemented detector is stronger. | None; recorded in §7. |

## 11. Roadmap disposition

**Milestone closed and the next dependency may proceed.**

- **M1 (this milestone): closed.**
- **M2** (frontend-client contract extraction): closed — see
  `002-frontend-client-contract-extraction-closure.md`.
- **M3** (desktop GUI surface parity): closed — see
  `003-desktop-gui-surface-parity-closure.md`.
- **M4** (GUI shell skeleton): still blocked on ratification of ADR-0009
  (Tauri v2 vs. egui). Nothing in this milestone depends on that decision.

## 12. Registry updates

`plans/registry.md`: the `desktop-gui-frontend` subsystem status is updated to
*active* with M1–M3 closed; the plan bullet is marked **closed 2026-10-23** and
names this closure record; the *Closure work and current control points* row
for the Desktop GUI frontend names the closure records.

`plans/subsystems/desktop-gui-frontend-roadmap.md`: milestone 1's row is
updated to **closed 2026-10-23** with this record; the roadmap's front-matter
status becomes *active*.
