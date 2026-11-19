# Desktop GUI Frontend Milestone 001 — Operation Parameter Schema Publication

Status: ready for handoff

Repository baseline: `203ab5dd958415340d39cd47fb1add432c687edd` (`feat/auto-20261022-76692679`)

Source roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#milestone-1--operation-parameter-schema-publication`

Long-term requirements:

- `plans/000-long-term-specification.md#4-enforcement-invariants`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/000-long-term-specification.md#6-dispatch-runtime-and-wire-identity`
- `plans/000-long-term-specification.md#8-runtime-and-dependency-baselines`
- `plans/002-long-term-roadmap.md#phase-7--standing-maintenance-and-future-capability-open`

Applicable ADRs:

- `plans/adrs/ADR-0008-operation-parameter-schema-publication.md`

Primary class: infrastructure

## 1. Objective

Publish a mechanically derived, machine-readable description of every
operation's request shape, reachable over the daemon protocol, so that any
frontend can discover parameter structure instead of hand-coding it.

Concretely: derive a JSON Schema over `eggsec_runtime::TaskKind` and its 30
params structs, and serve it — together with the operation identity and
policy metadata already owned by `OperationMetadata` — through a new
additive, read-only daemon command.

## 2. Why this milestone is ready

- **No hard dependencies.** This milestone is self-contained and does not
  depend on the frontend-client extraction (M002) or the GUI surface variant
  (M003).
- **Stable interface dependency.** `ADR-0008` is accepted and fixes the
  mechanism: derivation from canonical request DTOs, exposure over the daemon
  protocol as an additive command, and descriptive (non-authoritative)
  status.
- **Existing contracts to extend, not invent.** The daemon protocol already
  carries `ClientCommand` / `ServerMessage` variants with a flat
  `#[serde(tag = "type")]` shape, an `ErrorCode` set that already includes
  `UnsupportedCommand`, and a `DaemonCapabilities` handshake. The operation
  catalog already exposes `OperationMetadata` through the
  `config/policy_catalog` facade.

This milestone is also independently valuable: it de-brittles the REST, MCP,
and Python surfaces' parameter duplication, even if the GUI is never built.

## 3. Current implementation evidence

### The parameter shapes exist but are not self-describing

`crates/eggsec-runtime/src/request.rs` defines `TaskKind` with 30 variants,
each bound to a typed params struct:

| Variant | Params struct | Variant | Params struct |
|---|---|---|---|
| `LoadTest` | `LoadTestParams` | `AuthTest` | `AuthTestParams` |
| `StressTest` | `StressTestParams` | `Nse` | `NseParams` |
| `PortScan` | `PortScanParams` | `Hunt` | `HuntParams` |
| `EndpointScan` | `EndpointScanParams` | `Browser` | `BrowserParams` |
| `Fingerprint` | `FingerprintParams` | `Compliance` | `ComplianceParams` |
| `Fuzz` | `FuzzParams` | `Storage` | `StorageParams` |
| `Waf` | `WafParams` | `Integrations` | `IntegrationsParams` |
| `WafStress` | `WafStressParams` | `Workflow` | `WorkflowParams` |
| `Pipeline` | `PipelineParams` | `Vuln` | `VulnParams` |
| `Resume` | `ResumeParams` | `Wireless` | `WirelessParams` |
| `Recon` | `ReconParams` | `WirelessActive` | `WirelessActiveParams` |
| `PacketCapture` | `PacketCaptureParams` | `DbPentest` | `DbPentestParams` |
| `PacketTraceroute` | `PacketTracerouteParams` | `Intercept` | `InterceptParams` |
| `PacketSend` | `PacketSendParams` | `C2` | `C2Params` |
| `GraphQl` | `GraphQlParams` | | |
| `OAuth` | `OAuthParams` | | |

All 30 derive `Serialize` and `Deserialize`. `RunRequest` carries the
`TaskKind` plus `surface` and scope. `TaskKind::operation_id()` is the
canonical wire-side identity match.

### No schema is published anywhere

- `OperationMetadata` (`crates/eggsec-policy/src/catalog.rs`) carries
  `id`, `display_name`, `mode`, `risk`, `intended_uses`, `required_features`,
  `required_policy_flags`, `required_capabilities`, `target_policy`, and the
  per-surface exposure flags. It carries **no parameters**.
- `ToolInfo` (`crates/eggsec/src/tool/registry.rs:212-219`) carries `id`,
  `name`, `category`, `description`, `capabilities`, `protocols`. No
  parameters.
- `ToolListItem` / `ToolDetailResponse`
  (`crates/eggsec/src/tool/protocol/rest.rs:189-205`) carry identity,
  category, description, capabilities, protocols. No parameters.
- `crates/eggsec-python/src/operation_metadata.rs` exposes a
  `request_schema` **string hint**, not a schema. It is partial and does not
  match the actual struct fields.

### Daemon protocol facts

`crates/eggsec-daemon-protocol/src/protocol.rs`:

- `ClientCommand` is a flat `#[serde(tag = "type")]` enum with 15 variants
  (`Health`, `Capabilities`, `DeclareClient`, `CreateSession`,
  `ListSessions`, `GetSnapshot`, `GetTaskResult`, `SubmitTask`,
  `CancelTask`, `CancelActive`, `Subscribe`, `CloseSession`,
  `ApprovePolicy`, `ListPersistedSessions`, `GetPersistedSnapshot`).
- `ErrorCode` includes `UnsupportedCommand`, `Unsupported`, and
  `InvalidRequest`.
- `DaemonCapabilities` carries `runtime: RuntimeCapabilities` and
  `transports: Vec<TransportCapability>`.

The daemon host dispatches commands in `crates/eggsec-daemon/src/host.rs`.
The `ApprovePolicy` arm is the model for a command that is recognized but
not yet wired (it returns `ErrorCode::Unsupported`).

### Dependency and workspace facts

- Workspace `rust-version = "1.89"`; `edition = "2021"`; `license = "MIT"`.
- `deny.toml` `[licenses] allow` is a closed list including `MIT`,
  `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `MPL-2.0`,
  `Unicode-3.0`, and others. `[graph] all-features = true`.
- `make check-deps` runs `cargo deny --workspace --all-features check` over
  advisories, bans, licenses, and sources, and **fails closed** when
  `cargo-deny` is absent.
- `eggsec-runtime`'s current dependencies are `serde`, `serde_json`,
  `thiserror`, `tokio` (rt/macros/sync/time), `tokio-util`, `tracing`,
  `uuid`. It carries **no** engine or domain dependency (guard 22), and
  guard 11 requires it to stay free of TUI dependencies.
- `schemars` is **not** currently in the workspace dependency set.

## 4. Invariants that must not regress

- `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate.
  **The schema MUST NOT be imported by, or consulted by, any authorization,
  dispatch, scope, or policy path.**
- `OperationMetadata` remains the single source of truth for operation
  policy. The schema is descriptive and MUST NOT become a second policy
  surface.
- Strict-surface scope remains `LoadedScope`, never raw `Scope`; effective
  authorization remains engine-scope ∩ converted-spec.
- Strict surfaces dispatch only via `EnforcedDispatcher::dispatch_execution()`
  with an `ApprovedExecution` bundle.
- No existing `ClientCommand` or `ServerMessage` variant may change shape.
  The protocol's flat `#[serde(tag = "type")]` encoding is a published
  contract; do not box or restructure existing variants.
- The daemon's default dependency set stays `eggsec-runtime` +
  `eggsec-daemon-protocol` only (guard 16); any new dependency must be
  feature-gated and optional.
- `eggsec-runtime` stays free of engine and domain dependencies (guard 22)
  and free of TUI dependencies (guard 11).
- TLS stays ring-only; `reqwest` uses `rustls-no-provider`, never `rustls`.
- Workspace Tokio baseline stays `default-features = false` with per-crate
  feature lists; DTO crates carry no Tokio. `schemars` MUST NOT pull Tokio.
- Never `let _ =` or `filter_map(|e| e.ok())`; log via `tracing`.
- `deny.toml` remains the canonical dependency-policy source; do not
  reintroduce `.cargo/audit.toml`.

## 5. Scope

### In scope

- Schema derivation over `TaskKind` and its params structs.
- The choice and implementation of the schema's home: a `schema` feature on
  `eggsec-runtime`, **or** a new leaf crate `eggsec-runtime-schema`.
- A new read-only `ClientCommand::DescribeOperations` and its response
  type, carrying the schema plus the per-operation identity and policy
  metadata read from `OperationMetadata`.
- The daemon host arm that serves it.
- Admission of the new dependency into `deny.toml` and
  `docs/DEPENDENCY_EXCEPTIONS.md`.
- Guards and tests proving the properties in §4 and §13.
- Documentation updates listed in §12.

### Explicitly out of scope

- Any change to existing protocol message shapes.
- Any use of the schema on the authorization path.
- REST, MCP, or Python adoption of the artifact. (They may later consume
  it; this milestone does not migrate them.)
- Migrating the TUI's hand-built forms to the schema.
- Any frontend work, GUI crate, or shell framework.
- The GUI surface variant (that is M003).
- The frontend-client extraction (that is M002).

## 6. Required production changes

### Core/domain

- Add `schemars` schema derivation to the 30 params structs and to
  `TaskKind` in `crates/eggsec-runtime/src/request.rs`. Use the crate's
  `JsonSchema` derive. Fields that are `Option<T>` should be represented as
  optional in the schema, which the derive handles natively.
- `TaskKind` is `#[serde(tag = "type")]`-shaped; confirm the derived schema
  preserves the externally-tagged variant representation so a client can
  select a variant and fill its fields. If `schemars` cannot represent the
  serde representation faithfully, derive per-params-struct schemas and
  publish a variant map built from `TaskKind::operation_id()` rather than
  emitting a schema that misrepresents the wire shape. **Correctness of the
  published representation outranks convenience of a single derive.**
- Prefer `schemars` `serde` compatibility settings so the emitted schema
  matches the serde representation used on the wire. Verify this rather
  than assuming it.

### Storage and migrations

None. No persisted representation changes. The schema is derived, never
stored.

### Protocol and DTOs

- Add a `ClientCommand::DescribeOperations` variant to
  `crates/eggsec-daemon-protocol/src/protocol.rs`. It is flat and
  tag-serialized like every other command. It MUST carry no required
  fields beyond the command's own identity, so it can be issued before any
  session exists if the daemon's access-control policy permits; otherwise
  it follows the same client-declaration requirement as other restricted
  commands.
- Add a response payload type carrying:
  - the schema document (as `serde_json::Value`, or as a typed wrapper over
    `schemars::schema::Schema`, serialized to JSON);
  - the per-operation identity and policy metadata, sourced from
    `OperationMetadata` — **read from the catalog, not copied**.
- Add the capability to `DaemonCapabilities` so a client can discover that
  schema publication is available without issuing the command and failing.
  This is also how a client degrades gracefully against an older daemon.

### Runtime and concurrency

- The daemon host arm serves the command. The response is computed from
  immutable registry data; it may be computed once and cached, or computed
  per request. If cached, the cache is immutable and holds no lock on the
  hot path. **Prefer computing per request** unless measurement shows the
  derivation cost is material — the derivation itself is a compile-time
  artifact if `schemars` is used with `schema_for!`, and a
  `serde_json::Value` serialization at serve time is cheap.
- If the schema is served through the daemon's optional `http-api` surface
  as well, it MUST reuse the same payload type. Do not create a second
  representation.

### Frontend or operator surface

None. This milestone adds no UI.

### Security and authorization

- The command is **read-only** and reveals operation request shapes. It MUST
  be subject to the same access controls as other daemon commands, and MUST
  NOT be exposed on a non-loopback interface without the daemon's normal
  authorization checks.
- The schema is descriptive. Add an explicit test proving the schema is not
  consulted on the authorization path (see §10).

### Documentation and static guards

- Add a guard proving the schema artifact is not imported by any
  authorization, dispatch, or policy module. The guard should fail if a
  path containing the schema type appears under the policy, dispatch, or
  config-policy directories.
- Add a guard proving no hand-written parameter table has been introduced
  into the frontend-neutral crates. A practical form: assert that no
  frontend-neutral crate contains a struct whose name matches a params
  struct but is not the canonical one.

## 7. Ordered work packages

### Work package A — Select the schema home and admit the dependency

Intent: decide where the derivation lives and get the dependency through
supply-chain policy before writing code against it.

Required changes:

- Measure the dependency cost of both placements:
  - **Feature on `eggsec-runtime`**: adds `schemars` to `eggsec-runtime`'s
    dependency graph whenever the feature is enabled. Check whether
    `eggsec-runtime`'s consumers that do not need the schema would still
    pay it under `--all-features` (which `cargo deny` uses).
  - **Leaf crate `eggsec-runtime-schema`**: depends on `eggsec-runtime`,
    owns the derivation, and is pulled only by the daemon's schema feature
    and by clients that want the schema.
- Select one. The workspace rule is "new crates only on measured dependency
  payoff"; record the measurement and the decision in the closure record.
- Add `schemars` to the workspace `[workspace.dependencies]` with a pinned
  or bounded version.
- Admit it to `deny.toml`: confirm the license is inside the `[licenses]
  allow` list (the `schemars` crate is MIT OR Apache-2.0) and that its
  transitive closure introduces no banned crate and no new advisory.
- Record the new dependency in `docs/DEPENDENCY_EXCEPTIONS.md` with an
  owner and a review-by date, per the repository's dependency-exception
  convention.
- Confirm `schemars` does not pull Tokio, and that the workspace Tokio
  baseline is unaffected.

Acceptance evidence:

- `make check-deps` is green with the new dependency present.
- `docs/DEPENDENCY_EXCEPTIONS.md` contains the entry.
- The placement decision and its measurement are recorded in the closure
  record.

### Work package B — Derive the schema

Intent: produce a schema that faithfully represents the wire shape of
`TaskKind` and its params structs.

Required changes:

- Add the derivation to the 30 params structs and to `TaskKind`.
- Verify the derived schema's variant representation matches the serde
  representation actually used on the wire. `TaskKind` is tagged; the
  published schema must let a client construct the correct variant payload.
- Ensure every params struct's field appears in the schema with the correct
  optionality, and that enums nested inside params (for example a scan type
  or an auth type) are represented.
- Publish an accessor that returns the schema for the operation set.

Acceptance evidence:

- A test asserts the derived schema contains exactly one entry per
  `TaskKind` variant, keyed consistently with `TaskKind::operation_id()`.
- A test asserts a representative params struct's fields all appear with
  the expected optionality.
- A test asserts the schema's variant keys equal the set of
  `TaskKind::operation_id()` values — this is the drift detector.

### Work package C — Expose it over the daemon protocol

Intent: make the schema reachable by a client without changing any existing
message shape.

Required changes:

- Add `ClientCommand::DescribeOperations`.
- Add the response payload type.
- Add the capability flag to `DaemonCapabilities`.
- Add the daemon host arm. Serve the payload from `OperationMetadata` for
  the identity and policy fields, and from the derived schema for the
  parameter structure.
- Ensure the arm follows the daemon's existing access-control pattern
  (client declaration requirements, if any, are inherited from the
  surrounding dispatch code rather than re-implemented).
- Handle the "older daemon" case on the client side: a client that receives
  `UnsupportedCommand` MUST degrade to a static form set rather than fail.

Acceptance evidence:

- A daemon test issues `DescribeOperations` and asserts a well-formed
  response with the expected schema and metadata.
- A test asserts the operation identity and policy fields in the response
  match `OperationMetadata` exactly — proving the two are read from one
  source, not duplicated.
- The existing daemon protocol conformance tests pass unchanged.

### Work package D — Guards, docs, and verification

Intent: freeze the properties so a future change cannot silently turn the
schema into a policy surface or reintroduce a parallel parameter table.

Required changes:

- Add the two guards from §6.
- Update `architecture/runtime.md`, `architecture/daemon.md`, and
  `architecture/overview.md` to describe the schema artifact, the new
  command, and the new dependency.
- Update `docs/DEPENDENCY_EXCEPTIONS.md` and `deny.toml` (done in WP A, but
  confirm consistency).
- Update `docs/VERIFICATION.md` if the verification contract changes (it
  should not — the new command is covered by the existing daemon test
  suite).
- Update the matching skill under `.opencode/skills/` if any counted set
  changed. **Note:** the operation count does not change; a new protocol
  command is added, so check whether any skill counts daemon commands and
  update it if so.

Acceptance evidence:

- The guards fail when their condition is deliberately violated and pass
  otherwise.
- `make check` is green.
- The architecture documentation reflects the delivered behavior.

## 8. Failure, cancellation, restart, and contention semantics

- **Partial failure.** If the schema cannot be produced, the command MUST
  return the protocol's existing `Internal` error with a traced cause. It
  MUST NOT return a partial schema that a client might mistake for a
  complete one. If the schema is cached, a failed computation MUST NOT
  poison the cache.
- **Duplicate delivery.** The command is idempotent and read-only. Issuing
  it repeatedly MUST return equivalent payloads.
- **Process restart.** The schema is derived, not persisted, so a restart
  has no recovery obligation. A client that reconnects re-issues the
  command.
- **Cancellation races.** None. The command does not create or touch task
  state.
- **Stale generations.** The schema describes the operation set of the
  daemon that serves it. A client MUST treat a schema whose operation set
  does not match the daemon's advertised capabilities as mismatched. The
  capability flag lets a client detect this before relying on the schema.
- **Concurrent callers.** Multiple concurrent `DescribeOperations` requests
  MUST be safe. If a cache is used, it MUST be immutable-after-init or
  protected by a lock that is not held across the serialization of the
  response.

## 9. Compatibility and migration

- **Wire protocol.** Additive only. No existing `ClientCommand` or
  `ServerMessage` shape changes. Older daemons answer the new command with
  `UnsupportedCommand`; clients MUST treat that as "schema unavailable",
  not as a failure.
- **Rust API.** `eggsec-runtime`'s existing public API is unchanged. If the
  schema is exposed behind a feature or in a separate crate, the default
  build of `eggsec-runtime` gains no new required items and no new
  dependencies for consumers that do not enable it.
- **Enforcement.** No change to any operation's policy, risk tier, exposure
  flags, dispatch path, or approval semantics. The schema is not consulted
  on any of them.
- **Existing frontends.** No change required. The TUI, CLI, REST, MCP, gRPC,
  and Python surfaces continue to function identically.
- **Dependency policy.** `schemars` is admitted to `deny.toml` and
  documented in `docs/DEPENDENCY_EXCEPTIONS.md` with an owner and a
  review-by date. `deny.toml` remains the only dependency-policy source.

## 10. Required tests

### Focused unit tests

- Schema derivation covers every `TaskKind` variant; the variant key set
  equals the `TaskKind::operation_id()` set.
- A representative params struct's fields all appear, with correct
  optionality.
- Nested enums inside a params struct are represented.
- The derived schema's variant representation matches the serde
  representation on the wire (round-trip: build a `TaskKind` value from a
  schema-conformant payload and deserialize it back).

### Integration tests

- The daemon serves `DescribeOperations` over a real client connection and
  the response is well-formed.
- The response's identity and policy fields match `OperationMetadata`
  exactly, compared field by field against the catalog.
- `DaemonCapabilities` advertises the schema capability.
- A client against a daemon that does not support the command receives
  `UnsupportedCommand` and degrades gracefully (this may be simulated by a
  test double).

### Restart and recovery tests

- None required. The artifact is derived, not persisted. A test that the
  schema is reproducible across daemon restarts is sufficient if the daemon
  is restarted in an existing test fixture.

### Contention and cancellation tests

- Concurrent `DescribeOperations` requests from multiple clients return
  equivalent payloads.
- No task state is created or mutated by the command.

### Security and negative tests

- **The schema is not consulted on the authorization path.** Assert that
  the enforcement evaluation for a representative operation produces an
  identical decision with and without the schema artifact present. This is
  the load-bearing negative test for `ADR-0008`.
- The command is rejected for an undeclared client if the daemon's
  surrounding dispatch requires client declaration — proven by following the
  existing pattern rather than by a bespoke rule.
- The response contains no credential, secret, or scope material beyond
  operation identity, policy metadata, and parameter structure.

### Migration and compatibility tests

- The existing daemon protocol conformance suite passes unchanged.
- The existing frontend surface matrix test passes unchanged.

## 11. Required verification commands

```bash
make check                  # mandatory Rust contract
make check-deps             # dependency policy, incl. the new schemars entry
make check-msrv             # the new dependency must not raise the floor
bash scripts/check-architecture-guards.sh
```

If the schema's placement changes the runtime crate's feature surface, also
run:

```bash
cargo check --workspace --no-default-features
make check-features-individual
```

Do not claim commands that were not actually run in the closure record.

## 12. Documentation updates

- `architecture/runtime.md` — the schema artifact, its derivation, and where
  it lives.
- `architecture/daemon.md` — the new `DescribeOperations` command, its
  response, the capability flag, and the graceful-degradation contract.
- `architecture/overview.md` — the new command and, if a new crate was
  created, the new crate entry and dependency map edge.
- `deny.toml` — the `schemars` admission.
- `docs/DEPENDENCY_EXCEPTIONS.md` — owner and review-by date for the new
  dependency.
- `docs/VERIFICATION.md` — only if the verification contract changes.
- `plans/registry.md` — this milestone's status.
- The matching skill under `.opencode/skills/` — only if a counted set
  changed (check whether any skill counts daemon protocol commands).

## 13. Acceptance criteria

1. The schema is derived mechanically from the canonical request DTOs, and
   no hand-written parameter table exists anywhere in the repository.
2. The schema's variant set equals the `TaskKind::operation_id()` set, and
   the published representation matches the serde representation on the wire.
3. A client can obtain the schema over the daemon protocol with one
   read-only command, and can discover its availability from
   `DaemonCapabilities`.
4. The operation identity and policy metadata in the response are read from
   `OperationMetadata` and match it field for field.
5. No existing protocol message shape changes, and the existing protocol
   conformance suite passes unchanged.
6. The schema is not imported by, or consulted by, any authorization,
   dispatch, scope, or policy path — proven by a guard and by an explicit
   negative test showing identical enforcement decisions with and without it
   present.
7. `make check`, `make check-deps`, and `make check-msrv` are green, and the
   new dependency is admitted to `deny.toml` and recorded in
   `docs/DEPENDENCY_EXCEPTIONS.md` with an owner and a review-by date.

## 14. Stop conditions

The agent must stop and report rather than improvise when:

- the derived schema cannot faithfully represent the serde wire shape of
  `TaskKind` without misrepresenting it — report the specific mismatch
  rather than publishing an approximate schema;
- admitting the dependency to `deny.toml` requires a license exception
  outside the existing allow-list — report it rather than widening the
  allow-list;
- `schemars` pulls a banned crate, an unmaintained crate, or a second TLS
  stack — report it;
- the daemon's access-control pattern for the new command cannot be
  determined from the surrounding dispatch code — report rather than
  inventing a rule;
- exposing the schema would require changing an existing message shape —
  report and stop, since the milestone's compatibility contract forbids it;
- enforcement behavior is observed to differ with and without the schema
  present — this is a contract violation and MUST be reported immediately.

## 15. Closure evidence required

- The implementation commits or pull requests.
- A requirement-to-evidence matrix mapping each §13 acceptance criterion to
  the specific test or guard that proves it.
- The recorded placement decision (feature on `eggsec-runtime` vs. leaf
  crate) and the measurement that justified it.
- The `make check`, `make check-deps`, `make check-msrv`, and guard run
  outcomes, including any command not run and why.
- The `deny.toml` diff and the `docs/DEPENDENCY_EXCEPTIONS.md` entry with
  owner and review-by date.
- Evidence that the schema is not on the authorization path, including the
  negative test's output.
- Evidence that no existing protocol message shape changed, including the
  unchanged conformance suite run.
- Known limitations — in particular, what the schema cannot express (for
  example cross-field mutual exclusivity) and that engine-side validation
  remains authoritative.
- Unresolved findings classified by severity.
- Recommendation: closed, conditionally closed, corrective pass required, or
  blocked.

## 16. Handoff notes

- **Baseline.** `203ab5dd958415340d39cd47fb1add432c687edd` on
  `feat/auto-20261022-76692679`. The workspace is a virtual manifest; the
  binary shell is `eggsec-cli`.
- **Do not touch the enforcement path.** If you find yourself editing
  `crates/eggsec-policy/src/policy.rs`, `crates/eggsec/src/dispatch/`,
  `crates/eggsec/src/config/policy_*.rs`, or `crates/eggsec/src/runtime_bridge/`
  to make the schema work, you are out of scope. The schema is descriptive.
- **Do not widen `deny.toml`'s license allow-list.** Admission of an
  in-list license is expected; a new allow-list entry is a stop condition.
- **Do not change existing protocol variants.** The flat
  `#[serde(tag = "type")]` encoding is a published contract, and the protocol
  file documents that boxing a variant would change the serialized shape
  every client already agrees on.
- **Toolchain.** `schemars` must not introduce Tokio. The workspace Tokio
  baseline is `default-features = false` with per-crate feature lists, and
  DTO crates carry no Tokio.
- **Verification.** `make check` is the minimum. `make check-deps` is
  mandatory here because a dependency is being added, and it fails closed
  when `cargo-deny` is absent — install it rather than skipping the target.
- **Preserve unrelated user changes.** This worktree may carry changes
  unrelated to this milestone.
- **The schema's home is a real decision, not a formality.** Record the
  measurement. If the feature-on-`eggsec-runtime` route forces `schemars`
  onto every consumer under `--all-features`, prefer the leaf crate.
