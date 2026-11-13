# ADR-0008: Operation parameter schema publication

Status: accepted

Date: 2026-10-22

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#4-enforcement-invariants`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/000-long-term-specification.md#6-dispatch-runtime-and-wire-identity`
- `plans/001-terminology-and-domain-model.md`
- `plans/002-long-term-roadmap.md#phase-7--standing-maintenance-and-future-capability-open`

Affected subsystem roadmaps:

- `plans/subsystems/desktop-gui-frontend-roadmap.md`

Related ADRs:

- `plans/adrs/ADR-0007-desktop-gui-frontend-boundary.md`

Related implementation plans:

- `plans/implementation/desktop-gui-frontend/001-operation-parameter-schema-publication.md`

## Context

`OperationMetadata` is the single source of truth for operation policy. It
lives in `crates/eggsec-policy/src/catalog.rs` and covers 34 canonical
operations plus 42 aliases. Each entry carries identity and policy facts:
`id`, `display_name`, `mode`, `risk`, `intended_uses`, `required_features`,
`required_policy_flags`, `required_capabilities`, `target_policy`, and
per-surface exposure flags (`manual_exposable`, `tui_exposable`,
`mcp_exposable`, `rest_exposable`, `agent_exposable`, `grpc_exposable`).

**It carries no parameter information.** Neither does the tool registry's
`ToolInfo` (`crates/eggsec/src/tool/registry.rs:212-219`), nor the REST
surface's `ToolListItem` and `ToolDetailResponse`
(`crates/eggsec/src/tool/protocol/rest.rs:189-205`). The Python bindings'
`request_schema` is a human-readable hint string rather than a schema
(`crates/eggsec-python/src/operation_metadata.rs`).

The consequence is that **no frontend can discover what an operation's
parameters are**. Each frontend must know them out of band. The TUI does this
by hand-building a form per tab — 33 tabs, approximately 65,000 lines of
`crates/eggsec-tui/src/`, with the per-tab form definitions living in
`crates/eggsec-tui/src/tabs/`. That work is correct but non-shareable: the
REST, MCP, gRPC, and Python surfaces each solve the same problem separately,
and a new frontend starts from zero.

The canonical parameter shapes already exist and are already serializable.
`eggsec-runtime` defines a `TaskKind` enum with 30 variants, each paired with
a typed params struct — `PortScanParams`, `FuzzParams`, `WafParams`,
`DbPentestParams`, and 26 more — all deriving `Serialize` and `Deserialize`
(`crates/eggsec-runtime/src/request.rs`). The runtime crate is deliberately
free of engine and domain dependencies (guard 22), so it is a legitimate home
for schema derivation.

The gap is therefore not a design question about where parameters live. It is
that the shapes are **not self-describing on the wire**. A frontend that
receives a request DTO can deserialize it, but cannot discover which fields
exist, what they mean, which are required, or what their valid values are —
so it cannot build a form, validate input, or explain a parameter to an
operator.

## Decision drivers

- A frontend MUST be able to discover operation parameters at runtime, so
  that adding an operation does not require hand-editing every frontend.
- The schema MUST be derived mechanically from the canonical types. A
  hand-maintained parallel table would drift and would create a second source
  of truth — the failure mode the enforcement invariants exist to prevent.
- The schema MUST NOT become an authorization input. Parameter metadata has a
  different trust level from policy metadata, and conflating them would let a
  descriptive artifact influence a decision.
- The mechanism MUST be cheap enough that it stays accurate. Derivation at
  compile time from the same types that are serialized is the only approach
  that cannot drift silently.
- The mechanism MUST be reusable by every frontend, not a GUI-specific
  affordance.
- Adding the mechanism MUST NOT force a break in the existing wire contract or
  the existing public Rust API of the runtime crate.

## Considered options

### Option A — Hand-maintained parameter catalog

Add a `parameters` field to each `OperationMetadata` entry describing every
operation's fields by hand.

Benefits: no new dependency; directly readable in one file.

Costs: this is a second source of truth for operation shape, maintained by
hand, that will drift from the actual request structs — and drift is silent,
because nothing fails when a field is added to a struct and forgotten in the
catalog. It would also require touching the policy crate for what is
descriptive, non-policy data, weakening the "policy is pure data and pure
algorithms" boundary that `eggsec-policy` was extracted to establish.

### Option B — Derive JSON Schema from the canonical request DTOs (selected)

Add a `schemars` derivation to the 30 params structs in `eggsec-runtime`,
publish the resulting JSON Schema for the `TaskKind` enum, and expose it over
the daemon protocol as a new read-only command.

Benefits: the schema cannot drift, because it is generated from the same
types that are actually serialized; it lives in the frontend-neutral runtime
crate that already owns these DTOs; it is directly consumable by any web
frontend (JSON Schema is a web-native format) and by any Rust frontend
(via `schemars`); and it requires no change to any existing message shape.

Costs: a new workspace dependency (`schemars`) with a supply-chain footprint
that must be admitted through `deny.toml` and documented in
`docs/DEPENDENCY_EXCEPTIONS.md`; a new public item in the runtime crate
(either a `schema` feature or a separate `eggsec-runtime-schema` crate) which
must not force `schemars` onto crates that do not need it; and JSON Schema is
descriptive, so a GUI still needs per-field presentation decisions (labels,
grouping, ordering) that a schema cannot supply.

### Option C — Publish the request DTOs as opaque JSON and let clients introspect

Publish the Rust type names and let each frontend hard-code its knowledge of
the fields.

Benefits: almost no implementation cost.

Costs: this is Option A with extra steps — the field knowledge still lives in
each frontend, still drifts, and still must be hand-written per frontend. It
does not solve the problem the GUI is blocked on.

### Option D — Have the GUI reuse the CLI's argument definitions

The GUI builds an argv vector per operation, exactly as the TUI's
`cli_argv()` does, and lets the Clap tree parse it.

Benefits: reuses an existing, tested definition of every parameter.

Costs: it couples the GUI's data model to the CLI's textual interface. The
GUI would have to reverse-engineer argument names, handle the CLI's
multi-word subcommands and aliases, and lose type information — an integer
timeout and a string timeout become indistinguishable. It also means the GUI
can only offer operations the CLI exposes, and it keeps a user-facing command
surface alive as a de-facto API. Rejected.

## Decision

Operation parameters are published as a **mechanically derived JSON Schema**
over the canonical request DTOs, exposed as a **descriptive, non-authoritative**
artifact.

1. **Derivation.** The 30 params structs in `crates/eggsec-runtime/src/request.rs`
   and the `TaskKind` enum that binds them gain `schemars` schema derivation.
   The schema is generated from the same types that are serialized at runtime.
   There is no hand-written parameter table anywhere in the repository.

2. **Location.** The schema capability lives in the frontend-neutral runtime
   layer. Two placements are permitted and the implementation plan selects
   between them on measured dependency payoff:
   - a `schema` feature on `eggsec-runtime`, or
   - a new leaf crate `eggsec-runtime-schema` that depends on
     `eggsec-runtime` and owns the derivation.

   The second is preferred if the feature would otherwise force `schemars` onto
   every consumer of `eggsec-runtime`, because the workspace's stated rule is
   that new crates are created only on measured dependency payoff. The
   implementation plan MUST record which placement was chosen and why.

3. **Exposure.** The schema is reachable over the daemon protocol through a
   new read-only client command, `DescribeOperations`, returning the schema
   for the `TaskKind` operation set together with the per-operation identity
   and policy metadata that `OperationMetadata` already owns. The command is
   additive: no existing `ClientCommand` or `ServerMessage` variant changes
   shape, so existing clients are unaffected and older daemons answer the new
   command with the protocol's existing `UnsupportedCommand` error.

4. **Descriptive, not authoritative.** The schema MUST NOT participate in
   authorization. `OperationMetadata` remains the single source of truth for
   operation policy, and the schema MUST NOT be consulted by
   `EnforcementContext`, by the dispatcher, or by any adapter that decides
   whether an operation may run. A schema is a description of a request
   shape; a request that satisfies the schema is not thereby authorized.

5. **Single source of truth for shape.** Field names, types, optionality, and
   defaults in the schema are those of the canonical request DTOs by
   construction. Any frontend that hand-maintains a parallel parameter
   definition is violating this decision.

6. **Frontend presentation is separate.** Labels, grouping, ordering, widget
   choice, and per-field help text are frontend concerns and live in the
   frontend. The schema supplies structure; the frontend supplies meaning.

7. **Reuse.** The mechanism is published for all surfaces. The REST, MCP, and
   Python surfaces MAY consume the same artifact in later milestones; doing so
   is encouraged but is not required by this decision.

## Consequences

### Positive

- The GUI can build forms from discovered structure instead of hand-writing 34
  of them, which is the difference between a two-week and a multi-quarter GUI
  effort.
- Adding a field to an operation's request struct automatically updates every
  frontend that renders from the schema. Drift becomes impossible for shape.
- The schema is web-native, so a web-based frontend consumes it directly with
  no adapter, while a native Rust frontend consumes it through `schemars`.
- The runtime crate gains a genuinely useful public capability that is useful
  independently of any GUI.
- REST, MCP, and Python surfaces can later reuse the same artifact, which
  addresses the same duplication in the programmatic surfaces.

### Negative

- A new workspace dependency enters the supply-chain policy surface and must
  be admitted explicitly. `schemars` must be added to `deny.toml` (and to
  `docs/DEPENDENCY_EXCEPTIONS.md` with an owner and a review-by date) or the
  `make check-deps` gate fails closed.
- Deriving schemas over 30 structs adds compile time to any crate that pulls
  the derivation in. This is why feature-gating or crate-splitting is
  required rather than deriving unconditionally in the runtime crate.
- JSON Schema cannot express Eggsec-specific semantics such as "this field is
  mutually exclusive with that one" or "this value must be a member of the
  engine's known scope set". Frontends must implement those rules themselves
  or the engine must reject them at submission time. The engine's existing
  validation remains authoritative.
- A schema published over the wire is a public artifact describing the
  request surface of every operation. It is a reconnaissance aid for the
  operation surface and MUST be treated as such: it should be exposed only
  through the same local transport and access controls as the rest of the
  daemon protocol, and it must not become the only discoverable route to
  operation details.

### Neutral or deferred

- The schema format version is tied to the daemon protocol version. A schema
  change that alters an existing operation's shape is a wire-compatible break
  only if it is additive to that operation's request struct; a breaking shape
  change must follow the daemon protocol's compatibility rules.
- Per-field help text, labels, and grouping are deferred to the frontend.
- The REST and MCP surfaces' adoption of the artifact is deferred to later
  milestones of their own roadmaps.

## Compatibility and migration

- **Wire protocol.** Additive only. `DescribeOperations` is a new command.
  Existing commands and message shapes are unchanged. A client that receives
  `UnsupportedCommand` from an older daemon degrades to a static, hand-built
  form set — it does not fail to launch.
- **Rust API.** `eggsec-runtime`'s existing public API is unchanged. If the
  schema is exposed behind a feature or a separate crate, the default build
  of `eggsec-runtime` gains no new dependency and no new required items.
- **Enforcement.** No change to any operation's policy, risk tier, exposure
  flags, dispatch path, or approval semantics.
- **Existing frontends.** No change required. The TUI continues to use its
  hand-built forms; migrating it to the schema is a separate, optional
  milestone and is not a prerequisite for the GUI.
- **Dependency policy.** `schemars` MUST be admitted to `deny.toml` and
  recorded in `docs/DEPENDENCY_EXCEPTIONS.md` with an owner and a review-by
  date before the implementation is considered complete. The canonical
  `deny.toml` remains the only dependency-policy configuration; no
  `.cargo/audit.toml` may be introduced.

## Security and reliability implications

- **No authorization effect.** The schema is descriptive. It MUST NOT be read
  by `EnforcementContext`, `EnforcedDispatcher`, or any scope or policy
  decision path. A request that conforms to the schema is not authorized; the
  engine's evaluation is unchanged and remains the only authorization gate.
- **No policy effect.** `OperationMetadata` remains the single source of truth
  for operation policy. Publishing a schema alongside metadata does not create
  a second policy surface, and consumers MUST NOT treat schema-derived facts
  as policy facts.
- **Information exposure.** The schema enumerates operation parameter shapes
  across the whole operation surface. This is a reconnaissance aid for anyone
  who can reach the daemon protocol. It MUST be served only over the daemon's
  local transport under the same access controls as other commands, and it
  MUST NOT be exposed on a non-loopback interface without the daemon's normal
  authorization checks.
- **Validation authority.** Schema-conformant requests are still subject to
  the engine's own validation, scope enforcement, and per-operation checks at
  execution time. The schema is advisory to clients and never a substitute for
  server-side validation.
- **Reliability.** Schema publication is a read-only operation with no state
  and no effect on task execution. A failure to derive or serve a schema
  MUST NOT block operation execution for existing clients.

## Verification

Implementation is conformant when:

- a test proves the published schema round-trips: every `TaskKind` variant is
  present, every params struct's fields appear, and the schema's variant set
  matches the enum's variant set;
- a test fails if a params struct gains a field and the published schema does
  not change — proving the derivation is live rather than snapshot;
- a test proves the `DescribeOperations` command returns identity and policy
  metadata consistent with `OperationMetadata`, and that the two are read from
  the same registry rather than duplicated;
- a guard proves no hand-written parameter table has been introduced into the
  frontend-neutral crates;
- `make check` and `make check-deps` are green, with `schemars` admitted to
  `deny.toml` and documented in `docs/DEPENDENCY_EXCEPTIONS.md`;
- an explicit test proves the schema is not consulted on the authorization
  path — that is, that enforcement decisions are unchanged when a schema is
  present.

## Supersession

None.
