# Eggsec Long-Term Architecture and Product Specification

Status: canonical long-term implementation directive

Companion documents:

- `plans/001-terminology-and-domain-model.md`
- `plans/002-long-term-roadmap.md`

This document defines the intended end state for Eggsec. It establishes product scope, enforcement boundaries, crate ownership, transport contracts, security properties, interoperability requirements, and acceptance criteria. The roadmap decomposes this specification into ordered execution phases. The terminology document is normative whenever older Eggsec code or documentation uses overlapping terms such as scope, policy, operation, tool, transport, or session.

The keywords MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, and MAY are normative.

## 1. Product definition

Eggsec is a Rust-native, scope-enforced security assessment and defense-validation engine with multiple frontends (CLI, TUI, REST, MCP, gRPC, Agent), centralized policy enforcement, and domain execution crates. The engine crate `eggsec` is lib-only (no binary); the binary shell is `eggsec-cli`.

The same enforcement and dispatch architecture MUST support all surfaces without creating separate products:

```text
CLI / TUI (manual) ── permissive profile, operator overrides allowed
REST / MCP / agent / CI (strict) ── no overrides, fail closed
```

Manual surfaces evaluate through `EnforcementContext` with operator overrides permitted. Strict surfaces evaluate through the same context with no overrides and dispatch only on `Allow`.

## 2. Primary product goals

Eggsec MUST provide:

1. A scope-enforced assessment engine where every operation passes the mandatory pre-dispatch authorization gate.
2. Deterministic authorization semantics separable from configuration loading, DNS, transport, and frontends.
3. A dependency-light, implementation-neutral outbound HTTP contract with a pinned, qualified backend.
4. Narrow, reusable domain crates (database, web-proxy, mobile, NSE, load-test, distributed) that never authorize — callers enforce.
5. Stable report and evidence data contracts with renderer ownership separated from data ownership.
6. Programmable APIs (Python, REST, MCP, gRPC) with parity to the engine operation model.
7. Frontend-neutral runtime and daemon session persistence with typed wire DTOs as the single match surface.
8. Reproducible verification (`make check` contract) and supply-chain policy (ring-only TLS, MSRV 1.89).

## 3. Non-goals

Eggsec is not:

- an exploitation framework that bypasses its own scope enforcement;
- a general-purpose HTTP client library (the transport contract exists to serve assessment, not to compete with one);
- a CI definition system, Git hosting platform, or enterprise identity provider.

Eggsec MAY integrate with those systems where they support assessment workflows. It MUST NOT absorb their complete product scope.

## 4. Enforcement invariants

These invariants MUST hold across all releases and implementation strategies:

1. `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate for **all** surfaces. It MUST NOT be bypassed.
2. `OperationMetadata` is the single source of truth for operation policy. No inline policy checks.
3. Strict surfaces dispatch only via `EnforcedDispatcher::dispatch_execution()` with an `ApprovedExecution` bundle (token + scope snapshot from the same context). Bundles are obtained via `EnforcementContext::approve_execution()` / `approve_manual_execution()`; they MUST NOT be constructed directly. Operation matching uses `matches_descriptor()` plus scope/policy/surface checks, never operation names alone.
4. Strict-surface scope MUST be `LoadedScope`, never raw `Scope`. Effective authorization is engine-scope intersected with converted spec (`eggsec::config::scope_from_spec` is fail-closed). `eggsec-tool-core::ScopeSpec` is a transport DTO with no auth methods.
5. Raw `dispatch_checked()` with `ApprovedOperation` alone remains available only for scope-insensitive tools.
6. Authorization lives in `EnforcementContext`. Adapters use narrow service traits (`tool::service::EngineServices`, `agent::services::AgentExecutionService`, `mcp::bridge::McpEngineBridge`) and MUST NOT call `Scope::is_target_allowed` or `tool.execute()` directly.

## 5. Crate ownership

The workspace has 20 crates (see `architecture/overview.md` for the verified table). Durable ownership rules:

- `eggsec-core`: shared primitives only. Zero internal deps.
- `eggsec-tool-core`: protocol-neutral tool DTOs. No auth methods.
- `eggsec-report-model`: stable report/evidence data contracts (data only). Depends only on `eggsec-core`. `eggsec-output` renders over it, never the reverse.
- `eggsec-output`: rendering and analysis only. No engine/runtime deps.
- `eggsec-policy`: deterministic authorization semantics only — data + pure algorithms. No Tokio, network, filesystem, or frontends. The engine bridges DNS, features, and transport via `policy_bridge/` (resolver reports facts, policy decides; pure evaluation takes explicit `EnabledFeatures` + `TargetScope`, no `cfg!`/DNS inside).
- `eggsec-transport`: scope-aware outbound HTTP contract — neutral DTOs, mandatory `NetworkAuthority`, recording fake. Dependency-light (`bytes`/`http`/`url`/`thiserror` only).
- `eggsec-transport-eggfetch`: `HttpTransport` over published `eggfetch-core`. The engine owns the adapter; policy never depends on transport.
- `eggsec-db-lab`, `eggsec-web-proxy`, `eggsec-mobile-lab`, `eggsec-nse`: domain crates. They MUST NOT authorize — the caller enforces.
- `eggsec-runtime`, `eggsec-daemon`, `eggsec-daemon-protocol`, `eggsec-ui-model`: task lifecycle, session host, IPC types, view DTOs. `eggsec-daemon` default deps are `eggsec-runtime` + `eggsec-daemon-protocol` only.
- `eggsec-agent`: coordination (registry, scheduler, lifecycle, cron). `eggsec-python`: PyO3/maturin bindings.

## 6. Dispatch, runtime, and wire identity

- Canonical executor: `dispatch::canonical_execution::execute_approved_execution` (plus `execute_approved` for scope-insensitive ops). CLI routes once via `commands::route::route_for_commands`. Executor code MUST NOT import Clap/Ratatui/daemon-protocol/Python types; cancellation goes through `eggsec-runtime::race_with_cancel`.
- `TaskKind::operation_id()` / `canonical_target()` is the single wire-side match; engine helpers delegate to it (no parallel tables). `RuntimeSurface` is a wire DTO — `runtime_bridge` owns both conversion directions (`Unknown` rejected).
- CLI handlers live in-engine at `crates/eggsec/src/commands/handlers/`; `crates/eggsec/src/cli/` holds command types only; `crates/eggsec-cli/src/` is main/daemon-client/logging.

## 7. Transport and cryptography policy

- Outbound HTTP uses logical-URL + singular authorized resolved routing, manual per-hop redirect authorization, aggregate body-through-EOF deadlines, explicit pinned proxy routing, fail-closed unsupported proxy shapes, H1/H2 only (no H3), no automatic retries, no environment-derived proxy routing, decompression-off response semantics.
- TLS is ring-only everywhere: `rustls`/`tokio-rustls` with `default-features = false` + `["ring", ...]`; `reqwest` with `rustls-no-provider`, never `rustls`.
- Proxy dial execution behind `eggress-outbound` (listener-free edge only); `eggress-embed`, runtime/server, routing, advanced-protocol, and pproxy compatibility adoption are out of scope (see `plans/adrs/ADR-0002-eggress-selective-reuse-boundary.md`).
- Reqwest remains the application-level proxy-health owner where qualified; it MUST NOT become a second production transport.

## 8. Runtime and dependency baselines

- MSRV 1.89 (workspace `rust-version`). Verified with `make check-msrv`.
- Workspace Tokio baseline is `default-features = false` with per-crate `features = [...]` (`test-util` nowhere; DTO crates carry no Tokio).
- System-dependency features (`wireless`, `packet-inspection`, `nse`, `nse-ssh2`, `grpc-api`) require their native prerequisites; `full` aggregates are curated, not exhaustive — the oracle is `make check-features-individual`.
- High-cardinality fan-out stays bounded by configured concurrency (`WorkerConfig::max_concurrency` is a real capacity contract; no spawn-per-payload retention).
- All spawned tokio tasks need 30–300s timeout wrappers. Never `let _ =` or `filter_map(|e| e.ok())` — log via `tracing`.

## 9. Verification contract

- `make check` is mandatory: fmt, no-default checks, check-deps, clippy, tests, guards.
- `make check-deps` (cargo deny over `--workspace --all-features`) fails closed when `cargo-deny` is absent. `deny.toml` is canonical.
- `make test` is `cargo test --lib -p eggsec` only; full suite is `make test-ci` (`-p eggsec --features rest-api,cli`).
- `make check-python` builds into `.venv-ci/`; pytest excludes `network`-marked tests by default.
- `make check-full` / `make check-features-individual` / `make clippy-domain` / `make test-tui-pty` are deep checks, not per-PR. Contract: `docs/VERIFICATION.md`.
- Guards need `ripgrep` (`rg`): `bash scripts/check-architecture-guards.sh`.

## 10. Frontend rules

- TUI: use `.get(i)` not `chunks[i]`; input/nav handlers check `!self.is_running()`; `reset()` clears all state; no `println!/eprintln!/print!/eprint!/dbg!` outside `#[cfg(test)]` in production `eggsec-tui`; recoverable errors go through notification/per-tab-error/popup state; single terminal writer; `copy-cli` emits only flags the real Clap tree accepts; fuzz HTTP-session flag is `--http-session`; `TerminalSession` owns terminal setup/teardown with idempotent cleanup + silent `Drop` fallback; daemon sync paths reuse the ambient runtime via `runner::block_on_ambient` (never nested `Runtime::new`); no `Stdio::inherit()` in `eggsec-tui`.

## 11. Acceptance criteria

The end state is reached when every product goal above holds simultaneously with `make check`, dependency policy, feature profiles, MSRV, and all architecture guards green, Python stable-core parity asserted by test, and each subsystem roadmap closed with accepted closure evidence.
