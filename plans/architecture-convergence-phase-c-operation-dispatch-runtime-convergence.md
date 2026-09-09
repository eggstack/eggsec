# Phase C Plan: Operation, Dispatch, and Runtime Request Convergence

## Status

Status: Executed (2026-09-09). All workstreams implemented; see Completion record below.

## Objective

Finish the operation/command dispatch migration and reduce parallel request-schema ownership across CLI, TUI, runtime/daemon, protocol adapters, and Python.

Current policy metadata is substantially consolidated in `OperationMetadata`, but command dispatch remains split between a four-command `RegistryBacked` pilot and a much larger `LegacyWrapped` population. `eggsec-runtime` separately defines `TaskKind` plus per-task parameter structs, while protocol/tool/Python surfaces perform additional request translation. This phase converges those representations onto one canonical execution request path.

## Preconditions

Phase A must establish unambiguous scope declaration versus authorization semantics. Phase B must establish stable feature names/profile semantics. Do not build canonical request contracts on ambiguous scope/feature ownership.

## Primary files

```text
crates/eggsec/src/config/policy.rs
crates/eggsec/src/commands/registry.rs
crates/eggsec/src/commands/handlers/
crates/eggsec/src/dispatch/
crates/eggsec/src/tool/registration.rs
crates/eggsec/src/tool/registry.rs
crates/eggsec/src/tool/dispatcher.rs
crates/eggsec/src/tool/implementations/
crates/eggsec/src/runtime_bridge/
crates/eggsec-runtime/src/request.rs
crates/eggsec-tui/src/
crates/eggsec-cli/src/
crates/eggsec-python/src/operation_registry.rs
crates/eggsec-python/src/engine*.rs
crates/eggsec/src/tool/protocol/
docs/COMMAND_REGISTRY.md
docs/TOOL_REGISTRATION.md
docs/EXTENSIBILITY.md
```

## Non-goals

This is not a CLI redesign. It does not rename user-facing commands without compatibility need. It does not force helper/lifecycle commands into security-operation metadata. It does not replace Clap. It does not weaken type-level approval or turn the operation catalog into runtime dynamic registration.

## Target architecture

The target flow is:

```text
CLI/TUI/Python/REST/MCP/gRPC/daemon
        |
        v
surface-specific parse DTO
        |
        v
canonical operation request + normalized target/options
        |
        v
OperationMetadata -> descriptor
        |
        v
EnforcementContext -> ApprovedOperation
        |
        v
canonical executor/dispatcher
        |
        v
typed operation result/event stream
```

The frontend-specific parse DTO is allowed to differ for ergonomics, but it must be a shallow adapter. Policy-relevant and execution-relevant normalization must happen once.

## Workstream 1 — Inventory operation request ownership

For every canonical operation, map:

- CLI args type;
- TUI action state;
- `OperationMetadata` ID/alias;
- `ToolRequest` shape;
- runtime `TaskKind` variant and params type;
- protocol request mapping;
- Python request DTO/schema;
- engine execution function;
- result/event type.

Mark duplicated defaults, validation, enum/string parsing, target normalization, concurrency/timeout/rate options, and feature-specific flags.

Prioritize operations that have produced recent drift bugs: port scanning, load testing, pipeline construction, recon, fuzzing, and feature-gated runtime tasks.

## Workstream 2 — Define canonical operation request contracts

Introduce or identify one typed request struct per operation family in a dependency location appropriate to consumers. Do not put engine-heavy execution objects in `eggsec-runtime` solely for reuse.

Possible ownership:

- protocol-neutral primitive request types in `eggsec-tool-core` or a narrowly scoped shared operation-contract module/crate;
- engine-specific validated requests in `eggsec`;
- runtime wire wrappers referencing those dependency-light contracts where possible.

Requirements:

- canonical target representation;
- normalized timeouts/concurrency/rate limits;
- typed enums instead of repeated free-form strings where stable;
- explicit defaults owned once;
- separation between parsed request and policy approval;
- serde support where needed by runtime/protocol/Python schemas;
- no embedded authorization decision.

Avoid creating a generic JSON map as the new canonical representation; that would move drift from compile time to runtime.

## Workstream 3 — Central request normalization/validation

Create one normalization step per operation/family. It should handle currently duplicated behavior such as:

- port-range parsing and count bounds;
- load-test requests versus connections semantics;
- scan type parsing;
- timeout/concurrency bounds;
- profile parsing through canonical `ScanProfile`/`Pipeline::from_profile()` paths;
- target normalization;
- feature-specific mode selection.

Manual and programmatic surfaces may apply different user-experience validation before this step, but all must reach the same normalized request before policy approval/execution.

## Workstream 4 — Complete command registry migration

For every operation-backed command currently `LegacyWrapped`:

1. resolve canonical operation ID through `OperationMetadata`;
2. convert CLI args into the canonical request;
3. build the descriptor from canonical metadata/request target;
4. evaluate/approve through existing policy;
5. execute through the canonical dispatcher/service;
6. render the typed result in the CLI adapter.

Commands with multiple underlying operations (for example mobile static/dynamic or generic packet commands) may remain a command-level multiplexer, but each execution branch must resolve a canonical operation before approval.

When the final operation-backed command migrates, remove `LegacyWrapped` and `registry_backed` as migration-state concepts. Retain `HelperOnly`, `ServerLifecycle`, or an equivalent distinction for genuinely non-operation commands if useful.

## Workstream 5 — Runtime `TaskKind` convergence

`TaskKind` is a valid wire/runtime concept, but it should not independently own operation semantics.

Choose one of:

- make each `TaskKind` variant contain/reuse canonical request types; or
- generate/implement direct typed conversions from `TaskKind` to canonical request types with exhaustive matching.

Requirements:

- adding a canonical operation intended for runtime must produce a compile/test failure until runtime mapping is supplied;
- runtime parameter defaults cannot silently differ from local execution;
- runtime surface identity remains session-bound and not caller-forgeable;
- runtime request serialization remains stable or versioned;
- unsupported runtime operations fail explicitly rather than falling through string aliases.

## Workstream 6 — Tool/protocol/Python adapters

Refactor tool implementations and protocol/Python invocation to consume canonical requests.

`ToolRequest.params` may remain JSON at a wire boundary, but parsing into a canonical typed request must happen once through operation-owned code. Do not let each protocol handler deserialize a different ad hoc struct.

Python `ToolDescriptor`/JSON Schema generation should derive parameter schemas from the canonical request types or a single operation schema definition rather than parallel hand-written schemas.

## Workstream 7 — Result/event normalization

Where operation paths currently construct multiple equivalent result representations, define an engine canonical result and adapter conversions to:

- `ToolResponse`;
- runtime `TaskOutcome`/events;
- TUI view DTOs;
- Python typed results;
- CLI/output report envelopes.

Do not require all rich domain results to move to `eggsec-tool-core`; adapters are appropriate when the engine result is domain-heavy.

## Workstream 8 — Compatibility and migration

Preserve:

- CLI command aliases;
- public Rust facades intentionally maintained for semver;
- Python stable operation names;
- daemon protocol compatibility where documented.

Use deprecated compatibility constructors/converters temporarily if required. Every temporary shim must have a removal criterion in this phase or Phase G.

## Workstream 9 — Tests

Add matrix tests that exercise the same canonical request through multiple surfaces rather than merely comparing metadata strings.

At minimum choose representative operations from:

- scanner;
- fuzzer;
- WAF;
- loadtest;
- pipeline;
- auth/GraphQL/OAuth;
- one feature-gated high-risk domain;
- one local-file domain.

For each, prove equivalent normalized requests/defaults across CLI/runtime/tool/Python adapters where those surfaces exist.

Add exhaustive tests ensuring every operation-backed command has a canonical execution mapping and no `LegacyWrapped` operation remains at phase completion.

## Acceptance criteria

- operation-backed commands no longer use permanent `LegacyWrapped` dispatch;
- operation ID, feature/risk/target metadata remains canonical in `OperationMetadata` or its direct successor;
- execution parameter defaults/validation have one canonical owner per operation family;
- runtime `TaskKind` maps exhaustively to canonical typed requests;
- tool/protocol JSON parameters are parsed through operation-owned typed request code;
- Python schemas derive from canonical request contracts where feasible;
- equivalent requests through local/runtime/programmatic surfaces produce the same normalized engine request;
- strict approval still precedes execution and cannot be bypassed by the new adapters;
- command aliases and stable Python names continue to work;
- `make check`, `make check-python`, and representative runtime/protocol feature profiles pass.

## Completion record

Executed 2026-09-09.

- Baseline SHA: `c65477db` (Phase B completion).
- Final SHA: `c75c42df` (dispatch convergence implementation;
  follow-up test fix `e47aaaa3`).
- Operation-backed commands migrated: all 31 canonical operations now
  `RegistryBacked`; 4-command pilot expanded to full coverage (scan→pipeline,
  resume→pipeline, icmp/traceroute→packet, evasion/postex new metadata,
  mobile-dynamic + wireless-deauth multiplexer branches added).
- Removed transitional types/flags: `CommandDispatchMode::LegacyWrapped` and
  `CommandRegistration::registry_backed` deleted; `registry_backed_command_ids()`
  now derives from dispatch mode; `operation_backed_command_ids()` added.
- New canonical ownership:
  - `eggsec-tool-core::operation_request` (defaults, normalization, typed
    requests, `validate_tool_params`);
  - `eggsec::operation_request` facade (CLI/runtime/ToolRequest adapters,
    exhaustive `operation_id_for_task_kind`/`target_for_task_kind`);
  - `TaskKind::operation_id`/`canonical_target` exhaustive (no wildcard);
  - `runtime_bridge::descriptor_for_run_request` delegates to TaskKind methods;
  - `dispatch_inner` normalizes via canonical (fixed endpoint 10→20,
    fingerprint 1-1024→9-port list, fuzz xss→all/smart→sequential/0→3,
    graphql/oauth/auth timeouts to CLI values, profile unknown→error);
  - `ToolDispatcher::dispatch` validates params via canonical (alias-resolved);
  - Python stable params validate via canonical (12 ops tested; nse/sbom/etc.
    remain binding-layer until Phase G).
- Retained compatibility shims (with Phase G removal criterion):
  - CLI command aliases (`scan`, `waf`, `load`, `stress`, etc. via
    `ALL_OPERATION_METADATA_ALIASES`);
  - Python stable snake_case names (`scan_ports`, etc. via `to_engine_id`);
  - `registry_backed_command_ids()` as derived helper (not state);
  - Tool alias resolution in dispatcher (`scan`→`scan-ports`, etc.).
- Verification: `make check` PASS, `make check-python` PASS,
  `cargo check -p eggsec --features rest-api,grpc-api,tool-api` PASS,
  `cargo check -p eggsec-cli` (default + `--no-default-features`) PASS,
  `cargo check -p eggsec-daemon` PASS, `cargo test -p eggsec-python --lib`
  (231 passed) PASS, arch guards ALL PASSED.
  New tests: `operation_convergence` (12 matrix tests),
  `operation_request` unit tests (canonical + exhaustive TaskKind),
  Python `stable_operation_params_validate_through_canonical_contracts`.