# eggsec-ui-model Architecture

Frontend-neutral view DTOs and renderer registry for session, task, result, artifact, permission, and event views. Provides a shared rendering contract between TUI, CLI, and any future frontend without importing the runtime or engine crates directly.

## Role & Responsibilities

`eggsec-ui-model` bridges runtime types (`SessionSnapshot`, `TaskOutcome`, `RuntimeEvent`, etc.) to serializable view models that frontends can render. It owns the `ResultRendererRegistry` mapping result kinds to rendering metadata, enabling consistent display across all UIs.

## Location & Feature Gating

| Crate | Path | Dependencies |
|-------|------|-------------|
| `eggsec-ui-model` | `crates/eggsec-ui-model/` | `eggsec-runtime` only + `serde` + `serde_json` |

Architecture guards: no TUI, CLI, transport, or engine dependencies.

## Architecture (11 source files)

| Module | File | Purpose |
|--------|------|---------|
| `lib` | `src/lib.rs` | Public API re-exports |
| `session_view` | `src/session_view.rs` | `SessionSummaryView`, `SessionView`, `SessionScopeView`, `SessionCapabilitiesSummary` |
| `task_view` | `src/task_view.rs` | `TaskView`, `TaskProgressView` |
| `result_view` | `src/result_view.rs` | `ResultEnvelopeView`, `OutcomeView` |
| `event_view` | `src/event_view.rs` | `EventView` |
| `artifact_view` | `src/artifact_view.rs` | `ArtifactView` |
| `dashboard_view` | `src/dashboard_view.rs` | `DashboardSummaryView` |
| `permission_view` | `src/permission_view.rs` | `PermissionView`, `ClientRoleView` |
| `policy_view` | `src/policy_view.rs` | `PolicyPromptView` |
| `renderer_registry` | `src/renderer_registry.rs` | `ResultRendererDescriptor`, `RENDERER_REGISTRY` (23 entries), `renderer_for_kind()` |
| `conversion` | `src/conversion.rs` | Discoverability aid (actual `From` impls live in individual modules) |

### View DTOs

| Type | Source Type | Key Fields | File:Line |
|------|------------|-----------|-----------|
| `SessionSummaryView` | `SessionSummary` | `session_id`, `surface`, `surface_label`, `scope_source`, `has_explicit_scope`, `active_count`, `completed_count`, `created_at_secs` | `session_view.rs:7-17` |
| `SessionView` | `SessionSnapshot` | `session_id`, `surface`, `surface_label`, `scope: Option<SessionScopeView>`, `created_at_secs`, `generation`, `active_tasks`, `completed_tasks`, `capabilities_summary` | `session_view.rs:35-48` |
| `SessionScopeView` | `SessionScope` | `is_explicit`, `source`, `path` | `session_view.rs:50-55` |
| `SessionCapabilitiesSummary` | `RuntimeCapabilities` | `task_kind_count`, `supports_cancellation`, `transports` | `session_view.rs:67-72` |
| `TaskView` | `TaskSnapshot` | `task_id`, `status`, `status_label`, `task_kind`, `task_kind_label`, `request_summary`, `progress`, `last_error`, `has_outcome`, `outcome_kind` | `task_view.rs:9-21` |
| `TaskProgressView` | `TaskProgress` | `completed`, `total`, `percentage`, `message` | `task_view.rs:41-47` |
| `OutcomeView` | `TaskOutcome` | `outcome_type`, `summary`, `envelope`, `text_content`, `artifact_ref` | `result_view.rs:38-45` |
| `ResultEnvelopeView` | `TaskResultEnvelope` | `kind`, `kind_label`, `summary`, `payload`, `artifacts`, `artifact_count`, `supports_rich_tui`, `supports_json_detail` | `result_view.rs:8-18` |
| `ArtifactView` | `ArtifactRef` | `id`, `kind`, `path`, `mime_type`, `summary` | `artifact_view.rs:6-13` |
| `EventView` | `RuntimeEvent` | `session_id`, `event_type`, `task_id`, `message`, `timestamp_hint` | `event_view.rs:7-14` |
| `DashboardSummaryView` | `Vec<SessionSummary>` | `total_sessions`, `active_sessions`, `total_active_tasks`, `total_completed_tasks`, `sessions` | `dashboard_view.rs:6-13` |
| `ClientRoleView` | (manual) | `role`, `role_label`, `can_submit`, `can_cancel`, `can_close`, `can_approve_policy` | `permission_view.rs:4-12` |
| `PermissionView` | (manual) | `client_kind`, `client_kind_label`, `session_role`, `is_session_owner`, `surface`, `surface_label` | `permission_view.rs:58-66` |
| `PolicyPromptView` | `PolicyPrompt` | `message`, `confirmation_class`, `requires_explicit_approval`, `can_auto_approve` | `policy_view.rs:6-12` |

### EventView — 12 RuntimeEvent variant handlers (`event_view.rs:17-141`)

| # | RuntimeEvent Variant | event_type String |
|---|---------------------|-------------------|
| 1 | `SessionCreated` | `session-created` |
| 2 | `Snapshot` | `snapshot` |
| 3 | `TaskQueued` | `task-queued` |
| 4 | `TaskStarted` | `task-started` |
| 5 | `TaskProgress` | `task-progress` |
| 6 | `TaskLog` | `task-log` |
| 7 | `PolicyDecisionRequired` | `policy-decision-required` |
| 8 | `TaskCompleted` | `task-completed` |
| 9 | `TaskFailed` | `task-failed` |
| 10 | `TaskCancelled` | `task-cancelled` |
| 11 | `Audit` | `audit` |
| 12 | `SessionClosed` | `session-closed` |

### ResultRendererRegistry — 23 entries (`renderer_registry.rs:27-212`)

| # | Kind | Title | Rich TUI | JSON | Summary Fields |
|---|------|-------|:--------:|:----:|----------------|
| 1 | `port-scan` | Port Scan | ✓ | ✓ | `open_ports`, `total_scanned` |
| 2 | `endpoint-scan` | Endpoint Scan | ✓ | ✓ | `endpoints_found` |
| 3 | `fingerprint` | Fingerprint | ✓ | ✓ | `services` |
| 4 | `load-test` | Load Test | ✓ | ✓ | `requests_per_second`, `latency_p99` |
| 5 | `stress-test` | Stress Test | ✗ | ✓ | `packets_sent`, `errors` |
| 6 | `fuzz` | Fuzz | ✓ | ✓ | `findings`, `payloads_tested` |
| 7 | `waf` | WAF Detection | ✓ | ✓ | `waf_detected`, `waf_name` |
| 8 | `waf-stress` | WAF Stress | ✗ | ✓ | `requests_sent`, `blocked` |
| 9 | `pipeline` | Pipeline | ✓ | ✓ | `stages_completed`, `findings` |
| 10 | `recon` | Recon | ✓ | ✓ | `subdomains`, `hosts` |
| 11 | `packet-capture` | Packet Capture | ✗ | ✗ | `packets_captured` |
| 12 | `traceroute` | Traceroute | ✓ | ✓ | `hops` |
| 13 | `graphql` | GraphQL | ✓ | ✓ | `schema_found`, `endpoints` |
| 14 | `oauth` | OAuth | ✓ | ✓ | `flow_tested`, `vulnerabilities` |
| 15 | `auth-test` | Auth Test | ✓ | ✓ | `credentials_tested`, `weaknesses` |
| 16 | `nse` | NSE Script | ✓ | ✓ | `script`, `output_lines` |
| 17 | `hunt` | Vulnerability Hunt | ✓ | ✓ | `vulns_found`, `severity` |
| 18 | `browser` | Browser | ✓ | ✓ | `pages_loaded`, `findings` |
| 19 | `compliance` | Compliance | ✓ | ✓ | `checks_passed`, `checks_failed` |
| 20 | `db-pentest` | DB Pentest | ✓ | ✓ | `findings`, `db_type` |
| 21 | `wireless` | Wireless Recon | ✓ | ✓ | `networks_found`, `clients` |
| 22 | `intercept` | Intercept Proxy | ✗ | ✓ | `requests_captured` |
| 23 | `c2` | C2 Simulation | ✗ | ✓ | `beacons`, `commands` |

Lookup: `renderer_for_kind(kind)` → `Option<&'static ResultRendererDescriptor>`. Unknown kinds degrade to generic JSON rendering.

### Conversion Flow

All `From` implementations are one-directional: runtime types → view DTOs.

```
SessionSnapshot ──From──► SessionView
SessionSummary ──From──► SessionSummaryView
TaskSnapshot ──From──► TaskView
TaskProgress ──From──► TaskProgressView
TaskOutcome ──From──► OutcomeView
TaskResultEnvelope ──From──► ResultEnvelopeView
ArtifactRef ──From──► ArtifactView
RuntimeEvent ──From──► EventView (12 variant handlers)
PolicyPrompt ──From──► PolicyPromptView
```

View DTOs never depend on engine types. This ensures frontends can render without importing the engine.

## Public API

| Function/Type | Purpose |
|---------------|---------|
| `renderer_for_kind(kind: &str)` | Look up renderer descriptor by result kind |
| `RENDERER_REGISTRY` | Static array of 23 `ResultRendererDescriptor` entries |
| `ResultRendererDescriptor` | Metadata: `kind`, `title`, `summary_fields`, `artifact_kinds`, `supports_rich_tui`, `supports_json_detail` |
| All view DTOs | `Serialize` + `From<RuntimeType>` for conversion |

## Integration Points

- **TUI**: consumes `SessionView`, `TaskView`, `OutcomeView`, `EventView`, `DashboardSummaryView`, `PermissionView` for rendering
- **CLI headless mode**: consumes `SessionSummaryView`, `TaskView`, `OutcomeView` for JSON/text output
- **Daemon**: `SessionSnapshot` → `SessionView` conversion for HTTP API responses
- **Python bindings**: `SessionSnapshot` DTOs available for Python-side rendering

## Testing

- `renderer_registry_covers_known_kinds` — verifies all 23 expected kinds are registered
- `renderer_registry_no_duplicates` — no duplicate kind entries
- `unknown_kind_returns_none` — unknown kinds return None
- `crates/eggsec-ui-model/tests/view_roundtrip.rs` — serialization roundtrip for all view DTOs

## Key Invariants

1. **No TUI dependencies** — zero `ratatui`/`crossterm` imports
2. **No engine dependencies** — zero `eggsec` crate import
3. **No transport dependencies** — zero `axum`/`tonic`
4. **Sole dependency**: `eggsec-runtime` for source types
5. **Serialization-first**: all DTOs implement `Serialize`
6. **Static registry**: `RENDERER_REGISTRY` is a compile-time array, no heap allocation
7. **One-directional conversion**: runtime → view only; views never produce runtime types

## See Also

- [runtime.md](runtime.md) — Runtime that produces the source types
- [tui.md](tui.md) — TUI that consumes these view DTOs
- [daemon.md](daemon.md) — Daemon that persists snapshots convertible to views
- [overview.md](overview.md) — System-wide architecture
- [cli_commands.md](cli_commands.md) — CLI headless output using view DTOs

*Last verified against source: 2026-08-25*
