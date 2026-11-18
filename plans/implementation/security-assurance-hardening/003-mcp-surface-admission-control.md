# Security Assurance Hardening Milestone 003 — MCP Surface Admission Control

Status: closed

Repository baseline: `77188755` (`feat/auto-20261117-7451c367`, 2026-11-17)

Source roadmap:

- `plans/subsystems/security-assurance-hardening-roadmap.md` §7 Milestone 3
- Finding A3 in §4 of the same roadmap

Long-term requirements:

- `plans/000-long-term-specification.md` (enforcement boundary; authorized use)
- `plans/001-terminology-and-domain-model.md` (`ExecutionSurface::McpServer`,
  `ExecutionProfile::McpStrict`)
- `AGENTS.md` — "Enforcement (critical)"

Applicable ADRs: none. The milestone applies an existing daemon precedent
(`allow_public_bind`) to a second surface; it does not choose a new architecture.

Primary class: invariant

## 1. Objective

Make MCP admission operator-controlled at both the transport and tool layers.
`mcp-serve` and `codegg-mcp` refuse a non-loopback bind unless the operator opts
in **and** supplies an API key. Both subcommands also require one or more
repeatable `--allow-tool TOOL_ID` selections; live visibility is the intersection
of those IDs, the active registry, MCP exposure metadata, and profile controls.
Every call remains behind strict `EnforcementContext`/`LoadedScope` authorization.
Loopback defaults and keyless stdio transport remain unchanged.

## 2. Why this milestone is ready

- No hard dependencies.
- The precedent and its message text already exist in the daemon:
  `validate_bind_addr` (`crates/eggsec-daemon/src/http.rs:568-578`) refuses
  non-loopback unless `allow_public_bind` is set.
- Both subcommands share one handler, so one change covers both
  (`crates/eggsec/src/commands/handlers/mod.rs:512-524`).
- The surface already fails closed on authorization (`McpStrict` requires an
  explicit scope manifest for target-bearing operations), so this milestone adds
  transport admission, not a new policy layer.

## 3. Baseline implementation evidence

The following evidence describes the audited baseline before remediation; the
closure record captures the resulting implementation and verification.

```rust
// crates/eggsec/src/commands/handlers/serve.rs:88-125
let enforcement = crate::config::EnforcementContext::for_surface(
    ExecutionSurface::McpServer,
    ctx.config.execution_policy.clone(),
    ctx.enforcement.loaded_scope.clone(),
);
if args.stdio { ... return Ok(()); }
let router = create_mcp_router(registry, args.api_key.clone(), profile, enforcement).await;
let addr: SocketAddr = format!("{}:{}", args.bind, args.port).parse()?;
let listener = TcpListener::bind(addr).await?;
```

- `McpServeArgs` (`crates/eggsec/src/cli/misc.rs:318-333`): `bind` defaults to
  `127.0.0.1`, `api_key: Option<String>`, no bind-policy flag.
- `CodeggMcpArgs` (`cli/misc.rs:335-355`): same shape; converted into
  `McpServeArgs` and dispatched to the same handler
  (`commands/handlers/mod.rs:514-524`).
- Router construction (`crates/eggsec/src/tool/protocol/mcp/routes.rs:130-161`)
  adds no auth layer; authentication lives in `McpServer` and is conditional on
  `api_key: Some(..)`.
- Consequence: `--bind 0.0.0.0` with no `--api-key` yields an unauthenticated
  remote JSON-RPC endpoint that can submit operation requests. Scope enforcement
  still applies (strict profile, explicit manifest required), so impact is
  bounded to authorized targets, but it is unauthorized task submission and
  session access on the operator's host.

There is no loopback check and no warning anywhere on this path.

## 4. Invariants that must not regress

- `EnforcementContext::evaluate()` remains the pre-dispatch gate; neither
  transport nor operator tool selection duplicates or bypasses policy.
- `McpStrict` continues to require explicit `LoadedScope` provenance for
  target-bearing operations.
- Both entry points require explicit operator tool selection; no source-coded
  MCP tool-ID allowlist or denylist is introduced.
- `--stdio` remains keyless but still requires an explicit tool selection.
- Default `127.0.0.1` binding behavior is unchanged.
- Both `mcp-serve` and `codegg-mcp` keep dispatching through the same handler and
  keep their distinct `command_id()`.
- No new listener, TLS termination, or auth provider.

## 5. Scope

### In scope

- Bind validation in `handle_mcp_serve`.
- Repeatable `--allow-tool TOOL_ID` in both MCP argument structs, validated
  against the active registry and profile MCP exposure metadata.
- Filter discovery, tool calls, OpenAPI, and plan generation to the selected
  registry view; enforce profile risk/argument/resource controls at discovery
  and call time.
- Preserve strict `LoadedScope`/`EnforcementContext` authorization.
- CLI help text, operator docs, and architecture/metadata ownership updates.
- Unit tests for bind admission, operator selection, registry filtering, profile
  controls, and strict scope behavior.

### Explicitly out of scope

- TLS for the MCP listener.
- Token issuance, rotation, or storage.
- Changing the MCP JSON-RPC wire protocol or the compiled tool catalog; the
  runtime view is narrowed by explicit operator selection.
- Changing `McpStrict` policy semantics.
- Fixed bind-address allowlists or interface enumeration.
- `serve` (REST) and `grpc` commands.

## 6. Required production changes

### Core/domain

Add a pure admission function near the handler (engine-side; the engine cannot
depend on `eggsec-daemon`):

```rust
fn validate_mcp_bind(
    addr: &SocketAddr,
    api_key: Option<&str>,
    allow_public: bool,
) -> Result<(), String>;
```

Rules:

1. `addr.ip().is_loopback()` → `Ok`. No key required.
2. Non-loopback without `allow_public` → `Err` with an actionable message that
   names `--allow-public-bind` and states the risk.
3. Non-loopback with `allow_public` and no key (or an empty/whitespace key) →
   `Err` stating that a public bind requires `--api-key`.
4. Non-loopback with `allow_public` and a key → `Ok`; emit
   `tracing::warn!(%addr, "MCP server bound to a non-loopback address")`.
5. `0.0.0.0` and `::` are **not** loopback and follow rules 2–4.

Call it after parsing `addr` and before `TcpListener::bind`. Do not call it on
the `--stdio` path.

### Storage and migrations

None.

### Protocol and DTOs

`McpServeArgs` and `CodeggMcpArgs` gain:

```rust
#[arg(long, default_value_t = false,
      help = "Allow binding to a non-loopback address (requires --api-key)")]
pub allow_public_bind: bool,
```

The `CodeggMcpArgs → McpServeArgs` conversion at
`crates/eggsec/src/commands/handlers/mod.rs:514-524` must forward the field.
The JSON-RPC wire format is unchanged. `tools/list`, OpenAPI, resources, and the
ops-agent plan endpoint reflect the explicit selected-tool intersection.

### Runtime and concurrency

No new tasks. The check runs before bind, so a refused configuration never opens
a socket. No timeout wrapper is required because no new await is introduced
beyond the existing bind/serve calls.

### Frontend or operator surface

CLI only. Error text is the operator-facing surface; it must state the required
flags and the reason, not just "denied".

### Security and authorization

This is the security change. It must not read `ctx.config` policy or scope; it
validates transport exposure only. Log target address, never the API key.

### Documentation and static guards

- `docs/COMMAND_REGISTRY.md:99-100` (both command rows) if a column exists for
  flags; otherwise add a note below the table.
- CLI help strings in `crates/eggsec/src/cli/misc.rs`.
- `architecture/cli_commands.md` if it documents the server-lifecycle group's
  bind behavior.
- Optional guard: fail if `TcpListener::bind` appears in a protocol handler
  without a preceding admission call. Only add it if it can be written narrowly
  (one file, one pattern); a broad guard is out of scope.

## 7. Ordered work packages

### Work package A — Admission function

Intent: make the rule testable without a listener.

Required changes: implement `validate_mcp_bind` with the message text; place it
next to the handler, not in `eggsec-daemon`.

Acceptance evidence: unit tests for the full matrix (§10) run without binding a
port.

### Work package B — Wire both commands

Intent: one behavior for both entry points.

Required changes:

- add the flag to both structs;
- forward it in the `CodeggMcpArgs` conversion;
- call the validator before bind;
- ensure the stdio early return precedes validation.

Acceptance evidence: `cargo check -p eggsec --features rest-api,cli`; a CLI-level
test if the existing harness supports spawning the binary, otherwise a handler
test with a captured error.

### Work package C — Operator messaging and docs

Intent: no silent failure.

Required changes: refine the two error strings; update the docs listed in §6.

Acceptance evidence: message text asserted in tests; docs diff reviewed.

## 8. Failure, cancellation, restart, and contention semantics

- Refused configuration: no socket is opened, no router is built, the process
  exits non-zero with the message on stderr.
- Bind failure (port in use): unchanged error mapping.
- Restart: no persisted state.
- Contention: none; the check is a pure function over parsed arguments.

## 9. Compatibility and migration

- Loopback users see no change.
- Stdio users see no change.
- An existing deployment that binds non-loopback must add `--allow-public-bind`
  and `--api-key`. This is an intentional breaking change for an unsafe
  configuration; call it out in the closure record and, if the project keeps a
  changelog for CLI changes, add one entry.
- `codegg-mcp` gains the same flag; its default behavior is unchanged.

## 10. Required tests

### Focused unit tests

Matrix over `validate_mcp_bind`:

| addr | allow_public | api_key | expected |
|---|---|---|---|
| `127.0.0.1:8081` | false | none | Ok |
| `[::1]:8081` | false | none | Ok |
| `0.0.0.0:8081` | false | some | Err |
| `0.0.0.0:8081` | true | none | Err |
| `0.0.0.0:8081` | true | `"   "` | Err |
| `0.0.0.0:8081` | true | `"k"` | Ok |
| `[::]:8081` | false | some | Err |
| `192.168.1.10:8081` | true | `"k"` | Ok |

### Integration tests

- Handler-level test that a refused bind returns before `TcpListener::bind` (use
  an address that would otherwise fail to bind, e.g. a port already held, and
  assert the admission error wins).
- If the CLI test harness spawns processes, assert non-zero exit and that stderr
  contains both required flag names.

### Restart and recovery tests

Not applicable.

### Contention and cancellation tests

Not applicable.

### Security and negative tests

- Empty-string key treated as absent.
- Whitespace-only key treated as absent.
- Key value never appears in the error message or logs.

### Migration and compatibility tests

- Existing loopback command lines from the docs still start.

## 11. Required verification commands

```bash
make check
cargo test -p eggsec --features rest-api,cli
```

Do not claim commands that were not actually run in the closure record.

## 12. Documentation updates

- `docs/COMMAND_REGISTRY.md` (both rows or the note beneath them).
- `crates/eggsec/src/cli/misc.rs` help text.
- `architecture/cli_commands.md` if it covers server bind policy.
- `docs/ARCHITECTURE.md` / `docs/CAPABILITIES.md` only if they tell operators a
  non-loopback MCP bind is acceptable without a key.

## 13. Acceptance criteria

1. `mcp-serve --bind 0.0.0.0` without `--allow-public-bind` exits non-zero
   before binding.
2. `--allow-public-bind` without a non-empty `--api-key` exits non-zero before
   binding.
3. `--allow-public-bind --api-key K` binds and logs a warning.
4. `--stdio` remains keyless but requires an explicit tool selection.
5. Both `mcp-serve` and `codegg-mcp` require one or more repeatable
   `--allow-tool TOOL_ID` selections; empty, unknown, or non-exposable IDs fail
   closed at startup.
6. No source-coded fixed MCP tool-ID allowlist or denylist remains.
7. Discovery/OpenAPI/planning use only selected, active, MCP-exposable tools;
   profile-denied risk tools are hidden and rejected at call time.
8. Strict `LoadedScope` and `EnforcementContext` gates still precede dispatch.
9. `make check` and the feature-scoped test command pass.

## 14. Stop conditions

Stop and report rather than improvise when:

- the fix would require adding an auth layer to the axum router rather than
  reusing `McpServer`'s existing key check;
- `codegg-mcp` has a consumer that depends on non-loopback without a key and the
  change would break a documented workflow (report it; do not special-case);
- the CLI test harness cannot express the process-level assertion, and a
  handler-level test would require a new production seam;
- a reviewer asks for TLS or token rotation (that is a different milestone and
  needs an ADR).

## 15. Closure evidence required

- Commit/PR.
- Unit-test output for the admission matrix.
- The exact operator-facing error strings.
- `make check` and `cargo test -p eggsec --features rest-api,cli` output.
- A note confirming loopback and stdio behavior are unchanged.
- Any deviation from this plan.

## 16. Handoff notes

- `handle_mcp_serve` is `#[cfg(feature = "rest-api")]`; run the feature-scoped
  test command, not the default-feature one.
- Do not convert `api_key` to `SensitiveString` in this milestone; it would
  widen the diff into CLI parsing and is not needed for the admission rule. It
  is a reasonable follow-up.
- The daemon's `validate_bind_addr` is the wording precedent, but it cannot be
  imported (crate direction). Copy the shape, not the symbol.
- Preserve unrelated user changes in the worktree.
