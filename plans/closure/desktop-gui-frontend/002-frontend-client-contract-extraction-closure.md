# Desktop GUI Frontend Milestone 002 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/desktop-gui-frontend/002-frontend-client-contract-extraction.md`

Source subsystem roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md`

Repository baseline reviewed: `203ab5dd958415340d39cd47fb1add432c687edd`

Implementation commits or pull requests:

- Uncommitted working-tree change on branch `feat/auto-20261022-76692679`.

## 1. Executive finding

The milestone's capability boundary is complete. The frontend runtime-client
contract is no longer trapped inside the terminal UI.

Before this milestone, a second frontend could not reuse the contract without
depending on `eggsec-tui` — and therefore on ratatui, crossterm, the theme
pack, and the clipboard stack. The contract was also *named* for the TUI
(`TuiRuntimeClient`), which would have baked that dependency into every future
frontend's vocabulary.

Both obstacles are gone. The contract now lives in its own frontend-neutral
leaf crate, `eggsec-frontend-client`, under the neutral name `RuntimeClient`,
and the TUI consumes it with no observable behavior change. The workspace is
now 24 crates.

What the milestone deliberately did **not** deliver: no GUI crate, no shell, no
form rendering. The plan's constraint that this must be "a move, not a
redesign" was honoured — the moved code is byte-comparable, and the only
non-mechanical changes are a trait rename, one newly-public error type, and
three log strings that described the old home.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| The contract lives in a frontend-neutral crate | `crates/eggsec-frontend-client/` | pass | New leaf crate. |
| The crate depends on no frontend framework and not on the engine | Guard 153 | pass | The manifest must not reference `eggsec =`, `eggsec-tui`, `ratatui`, `crossterm`; the source must not reference `ratatui`, `crossterm`, `eggsec_tui`, or `eggsec::`. |
| The trait is no longer TUI-named | `RuntimeClient` in `crates/eggsec-frontend-client/src/lib.rs` | pass | `RuntimeClientFuture` and `RuntimeEventReceiverHandle` were already neutral and kept their names. |
| `DaemonClientError` is publicly reachable | Re-exported from the crate root | pass | It was crate-private before the move; a frontend must be able to distinguish a closed connection from a protocol error. |
| The TUI's observable behavior is unchanged | Only types and import paths changed; the TUI's own `runtime_client` field name and all call sites were kept | pass | 1002 TUI tests pass. |
| The daemon client's shared-connection behavior survives the move | The `Arc<DaemonConnection>` shape, the `FxHashMap` pending-response registry, and the `RwLock<Vec<UnboundedSender<RuntimeEvent>>>` event fan-out were moved verbatim | pass | Cloning the connection instead of the client would silently break event delivery to every other subscriber. |
| The extracted crate carries at least one test | 6 tests in `crates/eggsec-frontend-client/src/lib.rs` | pass | Object safety, session create/list/snapshot, unknown-session errors, event delivery, the public error type, and reported capabilities. |
| The crate joins the mandatory lint gate | `Makefile`'s `clippy` target | pass | `cargo clippy -p eggsec-frontend-client -- -D warnings` is clean. |
| The TUI still passes | `cargo test -p eggsec-tui --lib` | pass | 1002 tests. |

## 3. Production implementation evidence

**New crate `crates/eggsec-frontend-client/`.** Public API:
`RuntimeClient`, `RuntimeClientFuture`, `RuntimeEventReceiverHandle`,
`EmbeddedRuntimeClient`, `DaemonRuntimeClient`, `DaemonClientError`.

Dependencies: `eggsec-runtime`, `eggsec-daemon-protocol`, `serde_json`,
`thiserror`, `tracing`, `rustc-hash`, and Tokio with exactly `rt`, `sync`,
`time`, `io-util`, `net` — no `rt-multi-thread`, no `macros`.

**The trait.** `RuntimeClient` keeps its 8 methods and its
`Send + Sync + 'static` supertraits, so `Arc<dyn RuntimeClient + Send + Sync>`
remains the holder type every frontend uses. `RuntimeClientFuture<T>` stays a
boxed `Send + 'static` future of `Result<T, String>` — deliberately coarse,
because that is the contract the existing frontends already bind to.

**The implementations.** `EmbeddedRuntimeClient` wraps an in-process
`Arc<Runtime>`; `DaemonRuntimeClient` speaks the daemon's JSON-lines protocol
over a Unix socket and is `Clone`, with every clone sharing one
`Arc<DaemonConnection>`.

**TUI migration.** `crates/eggsec-tui/src/runtime_client/` was deleted. The TUI
now depends on `eggsec-frontend-client` and no longer on `eggsec-daemon-protocol`
directly — it depends on the client contract, not on the wire protocol.

## 4. Verification executed

### Commands run

```bash
cargo test -p eggsec-frontend-client
cargo test -p eggsec-tui --lib
cargo check -p eggsec-frontend-client
cargo check --workspace --all-targets
cargo check --workspace --no-default-features
bash scripts/check-architecture-guards.sh
cargo clippy -p eggsec-frontend-client -- -D warnings
cargo fmt --all --check
```

### Results

| Check | Result |
|---|---|
| `eggsec-frontend-client` | 6 tests pass |
| `eggsec-tui` | 1002 tests pass, 0 fail |
| `cargo check -p eggsec-frontend-client` | clean |
| `cargo check --workspace --all-targets` | clean, no errors |
| `cargo check --workspace --no-default-features` | clean |
| Architecture guards | ALL PASSED, including new check 153 |
| Guard negative test | Adding `ratatui` to the new crate's manifest made check 153 fail as designed; the manifest was restored and the check passed again |
| Clippy | clean |
| `cargo fmt --all --check` | clean |

## 5. Invariant review

| Invariant | Evidence | Status |
|---|---|---|
| The engine never depends on a frontend | Guard 153; the new crate does not depend on the engine, so the edge cannot exist in either direction. | holds |
| The daemon never depends on a frontend | Unchanged; the daemon's manifest gained no dependency. | holds |
| `eggsec-runtime` stays free of engine and domain crates | Unchanged; the new crate depends on the runtime, never the reverse. | holds |
| The canonical executor imports no frontend types | Unchanged. The executor does not import the new crate. | holds |
| `OperationMetadata` remains the single source of truth | Unchanged; the move touched no catalog or policy code. | holds |
| Workspace Tokio baseline | The new crate uses a narrower Tokio feature set than the TUI it came from. | holds |
| `make check` contract | Extended: the new crate joined the `make clippy` gate. | holds |

## 6. Failure and recovery review

- **Duplicate delivery:** not applicable — the crate is a library with no
  delivery path.
- **Cancellation races:** unchanged. The moved daemon client preserves its
  request-id → oneshot response registry and its cancellation plumbing
  verbatim.
- **Daemon restart:** unchanged. A daemon client reconnects by re-declaring
  and re-subscribing, exactly as before the move.
- **Partial persistence failure:** not applicable — the crate persists nothing.
- **Stale generation or lease:** not applicable.
- **Contention and resource release:** the pending-response registry is
  cleaned per request; the moved code is unchanged.
- **Malformed or unauthorized input:** unchanged. The client is a transport
  shim; the daemon still applies every access control.
- **Bounded event and artifact behavior:** unchanged. The event fan-out still
  goes through the shared `Arc<DaemonConnection>`.
- **Regression risk introduced by the move:** a clone of the *connection
  internals* instead of the client would break fan-out. The moved code keeps
  the `Arc` shape, and the crate's own tests exercise the embedded path.

## 7. Migration and compatibility review

- **Schema migration:** none.
- **Backward compatibility:** the Rust API of `eggsec-tui` is unchanged from a
  consumer's point of view — only the crate that supplies the trait moved. The
  TUI's internal field name and call sites were kept.
- **Protocol negotiation:** unchanged. The wire protocol is untouched by this
  milestone.
- **Configuration behavior:** unchanged.
- **Rollback:** reverting restores the previous module location; the new crate
  would be deleted. No data migration is involved.
- **Legacy-path status:** `crates/eggsec-tui/src/runtime_client/` is removed.
  Nothing else referenced it.

**Recorded deviation.** The plan's requirement that the extracted crate carry
at least one test was met, but with six. The moved code had **no** tests at all
before the move — the plan's premise that the extraction would carry existing
coverage was wrong — so the tests are new and cover the contract's observable
behavior (object safety, session lifecycle, error surfacing, event delivery)
rather than a port of prior coverage.

## 8. Security review

- **Authorization enforcement:** the crate is **not** an enforcement surface. It
  carries no authorization decision: the daemon applies its own access
  controls, and the embedded runtime applies the engine's. The contract's error
  type is `String`, deliberately coarse, exactly as before the move — moving
  the contract did not widen it.
- **Scope checks:** unchanged; the crate never resolves or checks scope.
- **Secret handling:** unchanged; the crate handles no credentials.
- **Privilege boundaries:** the move introduced no new network reach. The
  daemon client still speaks only over a Unix socket, and the embedded client
  still wraps an in-process runtime.
- **Transport checkpoints:** unchanged.
- **Denial-of-service bounds:** unchanged. The client issues one request per
  call and applies a request timeout, as before.
- **Audit behavior:** unchanged by this milestone.

## 9. Documentation and operations

- `architecture/frontend_client.md` — **new** deep-dive doc: role, boundary
  rule, the three source files, the public API, both implementations, the
  `Arc<DaemonConnection>` sharing requirement, and why the crate is not an
  enforcement surface.
- `architecture/overview.md` — crate count 23 → 24, new crate-table row, module
  index row, key-types row, dependency map, dependency-guardrails list, and the
  footer verification note.
- `architecture/tui.md` — the contract's new location and the `RuntimeClient`
  name.
- `docs/ARCHITECTURE.md` — crate count and the new crate row.
- `docs/CI_ARCHITECTURE_GUARDS.md` — a "Checks 151–154" section and a
  frontend-client crate-boundary bullet.
- `CONTRIBUTING.md` — the TUI tree no longer lists `runtime_client/`, and the
  new leaf crate is explained.
- `.opencode/skills/eggsec-frontend-client/SKILL.md` — **new** skill for the
  crate.
- `.opencode/skills/eggsec-tui/SKILL.md` — the remote-attach section now
  points at the shared crate and the renamed trait.
- `plans/000-long-term-specification.md` — crate count and the
  `eggsec-frontend-client` ownership rule.
- Static guard: 153 (the crate stays framework-free and engine-free).

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| low | The moved contract had **no** prior test coverage, so the plan's premise that the extraction would carry existing tests was wrong. | The extraction could have shipped a silently broken client. | **Resolved in this milestone**: 6 new tests now cover object safety, the session lifecycle, error surfacing, event delivery, the public error type, and reported capabilities. |
| low | The workspace-wide `make check-deps` advisory failure is recorded in the M1 closure record and is unrelated to this milestone. | The mandatory dependency gate stays red for pre-existing reasons. | Tracked under the M1 closure record. |

## 11. Roadmap disposition

**Milestone closed and the next dependency may proceed.**

- **M2 (this milestone): closed.**
- **M1** (operation parameter schema publication): closed — see
  `001-operation-parameter-schema-publication-closure.md`.
- **M3** (desktop GUI surface parity): closed — see
  `003-desktop-gui-surface-parity-closure.md`.
- **M4** (GUI shell skeleton): still blocked on ratification of ADR-0009. This
  crate is the seam that milestone consumes, so nothing about it depends on the
  framework choice.

## 12. Registry updates

`plans/registry.md`: the `desktop-gui-frontend` subsystem status is updated to
*active* with M1–M3 closed; the plan bullet is marked **closed 2026-10-23** and
names this closure record; the *Closure work and current control points* row
for the Desktop GUI frontend names the closure records.

`plans/subsystems/desktop-gui-frontend-roadmap.md`: milestone 2's row is
updated to **closed 2026-10-23** with this record.
