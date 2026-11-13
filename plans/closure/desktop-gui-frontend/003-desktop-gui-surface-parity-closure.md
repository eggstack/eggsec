# Desktop GUI Frontend Milestone 003 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/desktop-gui-frontend/003-desktop-gui-surface-parity.md`

Source subsystem roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md`

Repository baseline reviewed: `203ab5dd958415340d39cd47fb1add432c687edd`

Implementation commits or pull requests:

- Uncommitted working-tree change on branch `feat/auto-20261022-76692679`.

## 1. Executive finding

The milestone's capability boundary is complete. The desktop GUI is a
first-class **manual** surface: a peer of the CLI and the TUI, not a new policy
class and not a second enforcement mode.

Three properties were made true and then *tested*, rather than assumed:

1. The GUI derives the **same** `ManualPermissive` profile the CLI and TUI
   manual surfaces derive — no more, no less.
2. The GUI honors operator overrides and never lands on the strict side of the
   manual/strict split.
3. GUI activity is **attributable**: the daemon's persisted audit trail now
   records the session's runtime surface, so a GUI session is auditable as
   `desktop-gui-manual` and a TUI session as `tui-manual`.

The third property required a deliberate **behavior change** to the daemon's
audit trail, because the plan's premise about it turned out to be false. See
§3 and §7.

No new policy surface was created: the GUI reuses the existing
`manual_exposable` exposure flag, there is no `gui_exposable` flag, and there is
no strict GUI variant.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| A GUI surface exists in the runtime enum | `crates/eggsec-runtime/src/request.rs` — `RuntimeSurface::DesktopGuiManual` (11 variants, was 10) | pass | `label()` is `desktop-gui-manual`; the serde encoding is `DesktopGuiManual`; `Unknown` remains the `#[default]` and is still rejected. |
| A GUI surface exists in the policy enum | `crates/eggsec-policy/src/policy.rs` — `ExecutionSurface::DesktopGuiManual` (10 variants, was 9) | pass | |
| The bridge converts it in both directions | `crates/eggsec/src/runtime_bridge/surface.rs` | pass | It remains the single exhaustive conversion table. |
| It is a manual surface, not a strict one | `policy.rs::desktop_gui_derives_the_manual_permissive_profile` | pass | Equal to `CliManual`/`TuiManual`; unequal to both strict profiles. |
| It honors operator overrides | `policy.rs::execution_surface_honors_manual_override`; `surface.rs::manual_surfaces_honor_override_only_for_permissive` | pass | |
| It is not agent-controlled | `policy.rs::execution_surface_is_agent_controlled` | pass | |
| No `gui_exposable` flag was added | `crates/eggsec-policy/src/catalog.rs` is unchanged — the exposure flag set is unchanged | pass | The GUI reuses `manual_exposable`, so it is not a second policy surface. |
| A strict surface with no explicit scope still fails closed | No strict path was modified; the strict surfaces are untouched and `CliManualStrict`/`TuiManualStrict` keep `ManualGuarded` | pass | Confirmed by the enforcement matrix, which runs the strict surfaces against every scope state. |
| The GUI label is distinct from the CLI and TUI labels | `policy.rs::desktop_gui_labels_are_distinct`; `surface.rs::desktop_gui_is_a_manual_surface` | pass | `Desktop GUI manual` / `desktop-gui-manual`. |
| Existing surface encodings did not change | `surface.rs::existing_surface_wire_encodings_are_unchanged` | pass | A snapshot over all 10 pre-existing variants. |
| The GUI behaves identically to its manual peers **in enforcement** | `crates/eggsec/tests/enforcement_matrix.rs` — the GUI was added to `ALL_SURFACES`, `PERMISSIVE_SURFACES`, and `OVERRIDE_HONORING_SURFACES`, so every risk-tier, scope-state, capability, and override-isolation assertion in the matrix now runs against it. 173 tests pass. | pass | This is the load-bearing cross-surface evidence. |
| The audit trail distinguishes GUI from TUI | `host.rs::gui_session_audit_trail_carries_the_gui_surface_label` | pass | Both `desktop-gui-manual` and `tui-manual` are persisted, and no session-scoped event carries the blanket `daemon` label. |
| Session-less audit events keep the daemon label | `host.rs::session_less_audit_events_keep_the_daemon_label` | pass | |
| A future edit cannot silently demote the GUI | Guard 154 | pass | Pins the variant's presence in both enums, both conversions, its manual classification, and its profile. |
| The Python bindings can name the new surface | `crates/eggsec-python/src/audit.rs` and `daemon.rs` | pass | See §3. |

## 3. Production implementation evidence

**Runtime surface.** `RuntimeSurface::DesktopGuiManual` was added beside the
other manual variants. `Unknown` remains the `#[default]` and is still rejected
by the runtime bridge.

**Policy surface.** `ExecutionSurface::DesktopGuiManual` was added with the
identical `ManualPermissive` profile as `CliManual`/`TuiManual`,
`honors_manual_override() == true`, `is_agent_controlled() == false`, and
`requires_explicit_manifest_for_networked() == false`.

**Runtime bridge.** `crates/eggsec/src/runtime_bridge/surface.rs` converts the
new variant in both directions. It remains the single exhaustive conversion
table; no parallel table was introduced.

**Daemon audit trail (behavior change).** Session-scoped audit events
(`create-session`, `submit-task`, `cancel-task`, `cancel-active`,
`close-session`, `command-denied`) now record the **session's** runtime surface
label in `PersistedAuditEvent.surface` instead of the blanket literal
`"daemon"`. Events with no session (`declare-client`, `daemon-recovery`,
`approve-policy`) keep `"daemon"` — they are genuinely daemon-level. A
test-only read-back helper (`SqliteStore::audit_surfaces`) makes the persisted
rows assertable.

**Python bindings.** The GUI surface was added to the two string parsers that
enumerate surfaces:

- `crates/eggsec-python/src/audit.rs` — `parse_execution_surface()` now
  accepts `"desktop-gui-manual"`. Without this, a Python caller could not
  construct an audit event for a GUI session.
- `crates/eggsec-python/src/daemon.rs` — `parse_surface()` now accepts
  `"desktop_gui_manual"` / `"DesktopGuiManual"`, and the exhaustive
  `server_message_to_response()` match gained an `Operations` arm (it would
  otherwise fail to compile).

The Python `ExecutionSurface` **class** was deliberately **not** given a
`desktop_gui_manual()` constructor: that class is part of a versioned Python
public API with a generated compatibility baseline
(`crates/eggsec-python/tests/compatibility_baseline.json`), and adding a public
symbol would require regenerating the baseline and touching the separate Python
release contract. The string parsers cover the functional need without
changing the Python public symbol set, so the baseline stays valid.

The CLI's own `parse_surface()` was also left unchanged: it deliberately
offers the automation surfaces and already excludes the human-interactive
`tui-manual`. Adding `desktop-gui-manual` there would have been inconsistent
with that existing decision.

**Enforcement matrix.** `crates/eggsec/tests/enforcement_matrix.rs` now
iterates all **10** `ExecutionSurface` variants. This closed a pre-existing gap
as well: `GrpcApi` was never in the matrix's surface list.

## 4. Verification executed

### Commands run

```bash
cargo test -p eggsec-policy --tests
cargo test -p eggsec --features rest-api,cli --lib runtime_bridge
cargo test -p eggsec --features rest-api,cli --test enforcement_matrix
cargo test -p eggsec --features rest-api,cli,grpc-api --test frontend_surface_matrix
cargo test -p eggsec-daemon --lib
cargo check -p eggsec-python --features daemon-client
cargo check -p eggsec-daemon --features http-api,full-executor
cargo check -p eggsec-cli --all-targets
bash scripts/check-architecture-guards.sh
cargo clippy -p eggsec-policy -p eggsec-runtime --features schema -- -D warnings
cargo fmt --all --check
```

### Results

| Check | Result |
|---|---|
| `eggsec-policy` | 87 tests pass, including 3 new tests |
| Engine runtime-bridge unit tests | 66 pass, including 3 new tests |
| `enforcement_matrix` | 171 pass with `--features rest-api,cli` (166 with default features); 173 `#[test]` cases in the file, now covering all 10 surfaces |
| `frontend_surface_matrix` | 16 pass, including 1 new test |
| `eggsec-daemon` | 92 lib tests pass, including 2 new audit tests |
| `eggsec-python` (with `daemon-client`) | compiles |
| `eggsec-daemon` (`http-api`, `full-executor`) | compiles |
| `eggsec-cli` (`--all-targets`) | compiles |
| Architecture guards | ALL PASSED, including new check 154 |
| Guard negative test | Deliberate violations of checks 151, 152 and 153 each failed as designed; all passed again after restore |
| Clippy | clean |
| `cargo fmt --all --check` | clean |

## 5. Invariant review

| Invariant | Evidence | Status |
|---|---|---|
| `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate | The GUI surface routes through the same gate as every other manual surface; the enforcement matrix runs the GUI through every assertion. | holds |
| Strict surfaces fail closed without an explicit scope | No strict path was modified. The enforcement matrix's scope-state matrix still runs `CliManualStrict`/`TuiManualStrict` against DefaultEmpty, explicit allow, allow miss, and exclusion. | holds |
| `LoadedScope` on strict surfaces, never raw `Scope` | Unchanged; no scope code was touched. | holds |
| Strict surfaces dispatch only via `EnforcedDispatcher::dispatch_execution()` | Unchanged. | holds |
| `OperationMetadata` remains the single source of truth for operation policy | The catalog was not modified and no exposure flag was added. | holds |
| The executor reads the surface from the **session** context, not the request payload | Unchanged. | holds |
| `runtime_bridge` owns both conversion directions and rejects `Unknown` | Preserved and extended to the new variant in both directions; `Unknown` is still rejected. | holds |
| The daemon's default dependency set | Unchanged — the daemon's manifest gained no dependency. | holds |
| Workspace Tokio baseline / ring-only TLS | Untouched by this milestone. | holds |

## 6. Failure and recovery review

- **Duplicate delivery:** the new surface is an enum variant; there is no
  delivery path to duplicate.
- **Cancellation races:** unchanged.
- **Daemon restart:** the audit surface label is derived from the session's
  surface, which is re-established at session creation, so a restart does not
  change the label.
- **Partial persistence failure:** if the audit write fails, the daemon's
  existing `record_audit_event_logged` path handles it as before; the label
  change does not alter that failure mode.
- **Stale generation or lease:** not applicable.
- **Contention and resource release:** the surface label is read from the
  session-access map under the same lock the daemon already takes.
- **Malformed or unauthorized input:** an unrecognized surface string is still
  rejected by the parsers; the new value was added to both.
- **Bounded event and artifact behavior:** unchanged.

## 7. Migration and compatibility review

- **Schema migration:** none.
- **Backward compatibility:** the new enum variants are additive. A peer that
  does not know `DesktopGuiManual` cannot decode it, which is why the surface
  is only used by a client that opts in — the same additive-protocol posture the
  daemon's `DaemonCapabilities` flag uses.
- **Protocol negotiation:** unchanged. `RuntimeSurface` has no version
  negotiation; the daemon maps an unknown surface to `ErrorCode::InvalidSurface`.
- **Configuration behavior:** a daemon's `default_surface` may now be set to
  `desktop-gui-manual`; no other configuration changed.
- **Rollback:** removing the enum variants is a compile-time change; the audit
  trail's label format reverts with it. No data migration is involved — the
  persisted audit rows carry a label string, and older rows keep their
  `"daemon"` value, which remains a valid, meaningful label.

**Recorded deviation — the audit trail change was larger than the plan
assumed.** The plan stated that "the surface label that reaches the audit trail
is derived from the surface identity established at session creation".
Inspecting the daemon showed the opposite: **every** daemon audit event
recorded the literal `"daemon"`, so nothing distinguished TUI activity from CLI
activity — the plan's premise was false. Making the plan's acceptance criterion
satisfiable therefore required changing the daemon to record the session's
surface label for session-scoped events.

This is a deliberate behavior change to the persisted audit trail and a strict
improvement: a reader of the audit log can now tell which frontend produced an
action. Session-less events were deliberately left as `"daemon"`, and the
`session_less_audit_events_keep_the_daemon_label` test pins that distinction so
a later change cannot blur it.

**Recorded deviation — the engine-side audit path needed no change.**
`EnforcementAuditEvent.surface` is already an `ExecutionSurface`, so the new
variant flows into the engine's audit trail automatically once the bridge maps
it. The daemon-side label was the only gap.

## 8. Security review

- **Authorization enforcement:** the new surface is permissive by design and no
  more permissive than its peers — it derives the identical `ManualPermissive`
  profile that the CLI and TUI manual surfaces derive, so an operator at a GUI
  gets exactly the capabilities an operator at a CLI gets.
- **Scope checks:** unchanged. The GUI surface does not alter scope resolution;
  a GUI session without an explicit scope behaves exactly as a manual CLI
  session without one.
- **Manual override rule:** explicitly tested rather than assumed. The GUI is
  on the manual side of `honors_manual_override()` and is *never* classified as
  strict (`desktop_gui_derives_the_manual_permissive_profile` asserts its
  profile differs from both strict profiles).
- **Agent-controlled paths:** the GUI is not agent-controlled
  (`execution_surface_is_agent_controlled`), so it never gains the automated
  surface's ability to run without a human.
- **Privilege boundaries:** unchanged. The GUI adds no capability that the
  engine would not grant to an equivalent manual CLI session.
- **Audit behavior:** the GUI's activity becomes *attributable* rather than
  anonymous, which is a security improvement, and it does so without weakening
  any authorization check.
- **Denial-of-service bounds:** unchanged.

## 9. Documentation and operations

- `architecture/overview.md` — the `ExecutionSurface` surface table gained the
  `DesktopGuiManual` row; the footer records the 9 → 10 change.
- `architecture/config.md` — the surface → profile mapping table gained the GUI
  row, with the note that the GUI is a frontend, not a new policy class.
- `architecture/dispatch.md` — the manual-surfaces heading now reads
  "CLI/TUI/Desktop GUI" with a peer paragraph.
- `architecture/runtime_bridge.md` — the conversion table gained the GUI row;
  the override rule names the GUI; the flow diagram's variant count is 11.
- `architecture/runtime.md` — `RuntimeSurface` 10 → 11 with the
  `desktop-gui-manual` row.
- `architecture/audit.md` — the GUI added to the per-surface audit table.
- `architecture/daemon.md` — a new audit-trail section documenting the surface
  labels versus the literal `"daemon"`.
- `docs/ENFORCEMENT_MODES.md` — `ExecutionSurface` 9 → 10, a GUI row in the
  behavior matrix, a "Desktop GUI posture" paragraph, and the matrix-coverage
  claim corrected to all 10 surfaces.
- `docs/ARCHITECTURE.md` — `ExecutionSurface` 9 → 10 and a GUI row.
- `docs/ARCHITECTURE_INVARIANTS.md` — the override-honoring surfaces now
  include `DesktopGuiManual`.
- `.opencode/skills/eggsec-config/SKILL.md` — the override-honoring surface list
  now names the GUI.
- `plans/000-long-term-specification.md` — §1's frontends list, the manual/strict
  diagram, and the Desktop GUI rules.
- Static guard: 154 (the GUI stays a manual surface).

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| medium | `ApprovePolicy` remains unwired: `crates/eggsec-daemon/src/host.rs` answers it with `ErrorCode::Unsupported`. A GUI session therefore cannot obtain an approval it would not obtain elsewhere. | The GUI cannot drive an interactive-approval flow, so that flow must be designed before the GUI needs it. | A separate corrective milestone to wire daemon-mediated interactive approval. Plan 003 mandates the fail-closed behaviour and this milestone preserves it. **Deliberately left as-is.** |
| medium | A pre-existing `--all-features` compile error in the engine: `cargo check -p eggsec --all-features` fails with `E0616: field 'url' of struct 'WebSocketFuzzer' is private` (`crates/eggsec/src/fuzzer/advanced.rs:446`), because the corpus type in `crates/eggsec-payloads/src/websocket.rs` declares `pub(crate) url`, which is not visible across the crate boundary. **Verified pre-existing** by building the baseline SHA `203ab5dd` in a detached worktree — it fails with the identical error. | Affects the `--all-features` deep checks (`make check-full`, `make check-features-individual`). It is not in `make check`. | Fixing it means changing a knowledge-corpus crate's field visibility, which is outside this milestone. **Deliberately left unfixed and reported.** |
| low | The Python `ExecutionSurface` class has no `desktop_gui_manual()` constructor, so a Python caller cannot construct that object directly (it can still pass the surface string to the audit path). | A Python consumer cannot build an `ExecutionSurface` for the GUI without the string API. | Deferred to the Python release track: adding a public Python symbol requires regenerating `compatibility_baseline.json` and touching the versioned Python public API contract. |

## 11. Roadmap disposition

**Milestone closed and the next dependency may proceed.**

- **M3 (this milestone): closed.**
- **M1** and **M2**: closed — see the sibling closure records.
- **M4** (GUI shell skeleton): still blocked on ratification of ADR-0009
  (Tauri v2 vs. egui). Nothing in this milestone depends on that decision.
- **M5** (schema-driven forms): its M1 dependency is now discharged; it also
  requires M4.

## 12. Registry updates

`plans/registry.md`: the `desktop-gui-frontend` subsystem status is updated to
*active* with M1–M3 closed; the plan bullet is marked **closed 2026-10-23** and
names this closure record; the *Closure work and current control points* row
for the Desktop GUI frontend names the closure records.

`plans/subsystems/desktop-gui-frontend-roadmap.md`: milestone 3's row is
updated to **closed 2026-10-23** with this record; M4's blockers are updated to
name only the ADR-0009 ratification, since M2 and M3 are discharged.
