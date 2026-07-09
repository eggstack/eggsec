# eggsec-ui-model Architecture

Frontend-neutral view DTOs and renderer registry for session, task, result, artifact, permission, and event views. Provides a shared rendering contract between TUI, CLI, and any future frontend.

## Purpose

`eggsec-ui-model` bridges runtime types (`SessionSnapshot`, `TaskOutcome`, `RuntimeEvent`, etc.) to serializable view models that frontends can render without importing the runtime or engine crates. It owns the `ResultRendererRegistry` which maps result kinds to rendering metadata, enabling consistent display across all UIs.

## Crate Dependencies

```
eggsec-runtime (sole dependency)
    ↑
    └── eggsec-ui-model
            ↑
            ├── eggsec-tui   (consumes view DTOs for rendering)
            └── eggsec-cli   (consumes view DTOs for headless output)
```

No TUI, CLI, transport, or engine dependencies. Only `eggsec-runtime` + `serde`.

## View DTOs

| Type | Source Type | Purpose |
|------|------------|---------|
| `SessionSummaryView` | `SessionSnapshot` | Session list entry (id, surface, task count, status) |
| `SessionView` | `SessionSnapshot` | Full session detail with scope, capabilities, task list |
| `SessionScopeView` | `SessionScope` | Scope display (targets, exclusions) |
| `SessionCapabilitiesSummary` | `RuntimeCapabilities` | Allowed task kinds summary |
| `TaskView` | `TaskSnapshot` | Task list/detail with status/kind labels |
| `TaskProgressView` | `TaskProgress` | Progress percentage + message |
| `OutcomeView` | `TaskOutcome` | Terminal result display (5 variants) |
| `ResultEnvelopeView` | `TaskResultEnvelope` | Wraps result with kind + artifacts |
| `ArtifactView` | — | Neutral artifact display (id, kind, path, mime, summary) |
| `EventView` | `RuntimeEvent` | Streaming event display (12 variant handlers) |
| `DashboardSummaryView` | — | Dashboard: session/task counts + session list |
| `ClientRoleView` | `ClientRole` | Role capabilities (owner/controller/observer/approver) |
| `PermissionView` | `ClientKind` | Client kind labels (CLI, TUI, Daemon, MCP, REST, Agent) |
| `PolicyPromptView` | `PolicyPrompt` | Confirmation prompt (message, class, auto-approve flag) |

All DTOs implement `Serialize` for JSON transport and `From<RuntimeType>` for conversion.

## ResultRendererRegistry

Static registry mapping `TaskResultEnvelope.kind` → rendering metadata:

```rust
pub struct ResultRendererDescriptor {
    pub kind_label: &'static str,
    pub summary_fields: &'static [&'static str],
    pub artifact_kinds: &'static [&'static str],
    pub supports_tui: bool,
    pub supports_json: bool,
}
```

### Registered Kinds (22)

| Kind | Label | Rich TUI | JSON |
|------|-------|:---:|:---:|
| `port-scan` | Port Scan | Yes | Yes |
| `endpoint-scan` | Endpoint Scan | Yes | Yes |
| `fingerprint` | Fingerprint | Yes | Yes |
| `load-test` | Load Test | Yes | Yes |
| `stress-test` | Stress Test | Yes | Yes |
| `fuzz` | Fuzz | Yes | Yes |
| `waf` | WAF Detection | Yes | Yes |
| `waf-stress` | WAF Stress | Yes | Yes |
| `pipeline` | Pipeline | Yes | Yes |
| `recon` | Recon | Yes | Yes |
| `packet-capture` | Packet Capture | Yes | Yes |
| `traceroute` | Traceroute | Yes | Yes |
| `graphql` | GraphQL Fuzz | Yes | Yes |
| `oauth` | OAuth Fuzz | Yes | Yes |
| `auth-test` | Auth Test | Yes | Yes |
| `nse` | NSE Script | Yes | Yes |
| `hunt` | Hunt | Yes | Yes |
| `browser` | Browser | Yes | Yes |
| `compliance` | Compliance | Yes | Yes |
| `db-pentest` | DB Pentest | Yes | Yes |
| `wireless` | Wireless | Yes | Yes |
| `intercept` | Web Proxy | Yes | Yes |
| `c2` | C2 | Yes | Yes |

Lookup: `renderer_for_kind(kind)` → `Option<&'static ResultRendererDescriptor>`.

## Conversions

All `From` implementations are one-directional: runtime types → view DTOs. View DTOs never depend on engine types. This ensures frontends can render without importing the engine.

## Key Invariants

1. **No TUI dependencies** — no `ratatui`/`crossterm` imports.
2. **No engine dependencies** — no `eggsec` crate import.
3. **No transport dependencies** — no `axum`/`tonic`.
4. **Sole dependency**: `eggsec-runtime` for source types.
5. **Serialization-first**: all DTOs implement `Serialize`.
6. **Static registry**: `RENDERER_REGISTRY` is a compile-time array, no heap allocation.

## Tests

- `renderer_registry_covers_known_kinds` — verifies all expected kinds are registered
- `renderer_registry_no_duplicates` — no duplicate kind entries
- `unknown_kind_returns_none` — unknown kinds gracefully return None
- `tests/view_roundtrip.rs` — serialization roundtrip for all view DTOs

## See Also

- [runtime.md](runtime.md) — Runtime that produces the source types
- [tui.md](tui.md) — TUI that consumes these view DTOs
- [daemon.md](daemon.md) — Daemon that persists snapshots convertible to views
- [overview.md](overview.md) — System-wide architecture
