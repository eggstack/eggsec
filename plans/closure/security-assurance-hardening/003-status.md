# Security Assurance Hardening Milestone 003 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/security-assurance-hardening/003-mcp-surface-admission-control.md`

Source subsystem roadmap:

- `plans/subsystems/security-assurance-hardening-roadmap.md` §7 Milestone 3

Repository baseline reviewed: `77188755a41bee15aee9507055ac6b2761ff9271` (implementation remains uncommitted in the worktree)

Implementation commits or pull requests:

- None. No commit or PR was created, per instruction; evidence below describes the current working tree.

## 1. Executive finding

MCP exposure is now selected by the operator at both the listener and tool layers. Non-loopback serving requires explicit public-bind opt-in and a non-empty API key; runtime tool visibility is the intersection of operator-selected exact active registry IDs, MCP exposure metadata, and profile controls. Tool calls continue through strict policy and scope approval.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| Non-loopback listener is explicit and authenticated | `crates/eggsec/src/commands/handlers/serve.rs`; shared `mcp-serve`/`codegg-mcp` handler; feature-enabled engine tests | pass | Loopback defaults and keyless stdio behavior are preserved. |
| Tool list is operator-selected and registry-backed | CLI argument definitions, registry/MCP exposure metadata, discovery and call handlers | pass | Selection names exact active registry IDs; no synthetic unregistered helper is listed or callable. No source-coded tool allowlist/denylist was added. |
| Aliases resolve to canonical runtime metadata | `eggsec-policy/src/catalog.rs` and MCP handlers | pass | Aliases such as `scan` and `load` resolve at runtime; operator selection remains exact-ID only. |
| Target-bearing calls remain strict and in scope | MCP policy and dispatch path; `LoadedScope`, `ApprovedExecution`, `EnforcedDispatcher` | pass | No direct tool execution or operation-name-only authorization. |
| Both MCP commands share the admission path | `commands/handlers/mod.rs`; canonical dispatch regressions | pass | Distinct command IDs remain distinct. |

## 3. Production implementation evidence

The MCP router filters discovery, tool calls, OpenAPI, and planning to the operator-selected view intersected with currently active MCP-exposable registry entries and profile constraints. It does not expose the synthetic `eggsec_preflight` helper because that helper is not a selectable active registry capability; calls to it are rejected as unknown. Call-time policy remains fail-closed. Scope-sensitive dispatch obtains an `ApprovedExecution` from the same strict `EnforcementContext` and loaded scope used for authorization. Bind validation occurs before listener creation.

## 4. Verification executed

### Commands run

```bash
cargo test -q -p eggsec --features rest-api,cli --test canonical_dispatch_ownership
cargo test -q -p eggsec --features rest-api,cli
make check
make check-full
```

### Results

- Canonical dispatch target: 26 passed.
- The complete REST/CLI-enabled engine test command exited 0 after a stale `SecurityTool` doctest was corrected from `request.request_id` to `request.id`.
- `make check` and `make check-full` exited 0; architecture guards and representative feature profiles passed.

## 5. Invariant review

- `EnforcementContext::evaluate()` remains the mandatory pre-dispatch decision.
- Strict MCP uses `LoadedScope`; target-bearing execution carries the approved scope snapshot in `ApprovedExecution`.
- `OperationMetadata` remains the policy source of truth.
- Only active, MCP-exposable registry tools selected by the operator are visible; there is no compile-time fixed tool-ID allowlist.

## 6. Failure and recovery review

Invalid bind or missing credentials refuse startup before a listener is opened. Unknown, inactive, or non-exposable tool selections fail closed. Runtime denials do not fall back to a manual profile or scope-insensitive dispatch. No persistence or cancellation contract changed.

## 7. Migration and compatibility review

MCP operators must supply one or more explicit tool selections. Loopback defaults are unchanged; stdio remains keyless. Tool aliases remain available for canonical runtime resolution but do not replace exact active registry IDs in the operator selection.

## 8. Security review

Network exposure requires a deliberate opt-in plus API key. Authorization still evaluates risk and scope at call time; no API key or secret value is added to logs or tool metadata. MCP does not inherit the permissive human-operated remote command policy.

## 9. Documentation and operations

- Updated MCP and tool registration guidance: `docs/mcp-protocol.md`, `docs/TOOL_REGISTRATION.md`, `docs/COMMAND_REGISTRY.md`, `docs/extending/tool-exposure.md`, `docs/METADATA_OWNERSHIP.md`, and `docs/CAPABILITIES.md`.
- Regression evidence is in `crates/eggsec/tests/canonical_dispatch_ownership.rs` and the feature-enabled engine test suite.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| — | No unresolved milestone finding identified by executed checks. | — | — |

## 11. Roadmap disposition

Milestone closed; both MCP entry points use the shared admission and strict dispatch path.

## 12. Registry updates

- Record this milestone as closed with this closure reference in the security-assurance-hardening roadmap and `plans/registry.md`.
