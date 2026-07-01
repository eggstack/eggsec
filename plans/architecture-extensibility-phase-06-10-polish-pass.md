# Polish Handoff Plan: Phase 6–10 Registry and Exposure Semantics

## Objective

Perform a narrow polish pass after the Phase 6–10 corrective tightening work. The previous corrective pass fixed the largest semantic issues: tool registration now distinguishes metadata-level MCP exposure from conservative default visibility, command registration now separates CLI/TUI/programmatic/interactivity fields, command dispatch modes are explicit, duplicate ID tests are order-independent, feature snapshots are checked against Cargo, and the Phase 9 plan was restored.

This polish pass should close the remaining small contradictions and naming ambiguities before Phase 11 CI architecture guards are added. The goal is to avoid wiring CI around inconsistent terminology or subtly wrong profile-listing semantics.

## Current state

Recent changes landed correctly in these areas:

- `ToolRegistration` now has `mcp_metadata_exposable` and `mcp_default_visible`.
- `default_mcp_visible_for_operation()` provides a conservative default visibility helper.
- `mcp_tool_registrations_default_visible()` returns the conservative subset.
- Tool registration tests now assert high-risk operations are not default MCP-visible.
- `CommandRegistration` now has explicit `cli_visible`, `tui_visible`, `programmatic_visible`, `interactive_only`, `registry_backed`, and `dispatch_mode` fields.
- `CommandDispatchMode` distinguishes `RegistryBacked`, `LegacyWrapped`, `CatalogOnly`, `ServerLifecycle`, and `HelperOnly`.
- Duplicate command ID tests now use a set.
- Feature matrix tests parse `Cargo.toml` to validate the static feature snapshot.
- The Phase 9 plan file has been restored.

Remaining issues:

1. `mcp_tool_registrations("ops-agent")` still returns all `mcp_metadata_exposable` tools. This may be intentional for an expanded ops profile, but it is not the conservative default listing. Docs/tests must make that explicit or the profile should use `mcp_default_visible`.
2. `docs/TOOL_REGISTRATION.md` has contradictory text: it says MCP listing does not check metadata exposure, while later saying MCP listing filters through `ToolRegistration`.
3. `interactive_only` still has ambiguous naming. It currently means helper/CLI-interactive only, not all human-interactive surfaces. The field name can be misread as including TUI.
4. `side_effecting_entries_have_descriptor_builder()` still requires descriptor builders for all side-effecting commands with operation IDs, including legacy-wrapped entries. That blurs the distinction between registry-backed and legacy catalog metadata.

## Non-goals

- Do not redesign the command registry again.
- Do not remove the tool registration model.
- Do not change strict enforcement paths.
- Do not add Phase 11 CI jobs yet.
- Do not expand tool or command exposure.
- Do not add new capabilities or domains.

## Work item 1: Decide and encode MCP profile visibility semantics

### Problem

The code now has two distinct concepts:

- `mcp_metadata_exposable`: metadata-level permission for MCP exposure under feature/profile/policy.
- `mcp_default_visible`: conservative default visibility.

However, `mcp_tool_registrations("ops-agent")` returns all metadata-exposable tools. MCP `handle_tools_list()` calls `mcp_tool_registrations(profile_name)`, so the OpsAgent profile is an expanded metadata-exposable listing rather than a conservative default listing.

This may be correct, but it must be explicit.

### Required decision

Choose one model.

#### Model A: OpsAgent is expanded metadata-exposable listing

Use this if OpsAgent is intentionally a full operator/automation profile that can list any MCP-exposable tool, while still requiring strict runtime policy and `ApprovedOperation` before execution.

Required changes:

- Rename comments/docs from "default" to "profile-expanded" where referring to OpsAgent.
- Add or update helper names if needed:
  - `mcp_tool_registrations("ops-agent")` = expanded profile list.
  - `mcp_tool_registrations_default_visible()` = conservative default list.
- Add tests that explicitly state OpsAgent may include metadata-exposable high-risk tools, but those tools must not be `mcp_default_visible`.
- Add tests that high-risk OpsAgent-listed tools still require strict enforcement approval at execution/preflight level, if there is already a convenient test harness.
- Update `docs/TOOL_REGISTRATION.md`, `docs/METADATA_OWNERSHIP.md`, and `docs/CAPABILITY_MATRIX.md` to state OpsAgent is not the conservative default list.

#### Model B: OpsAgent should use conservative default visibility

Use this if OpsAgent should not list high-risk tools by default.

Required changes:

- Change `mcp_tool_registrations("ops-agent")` to filter on `mcp_default_visible` or a profile allowlist that starts with default-visible tools.
- Add a separate explicit expanded profile, e.g. `ops-expanded`, if needed.
- Update MCP docs and tests accordingly.

### Recommended approach

Use Model A if that matches the current product intent: OpsAgent is an expanded, strict-policy-gated operator profile, not a safe/default profile. This is less disruptive and aligns with the current implementation. The key is documentation and tests that prevent future confusion.

### Acceptance criteria

- It is unambiguous whether OpsAgent is conservative default or expanded metadata-exposable.
- Tests reflect the chosen model.
- High-risk metadata-exposable tools are never described as default visible unless `mcp_default_visible = true`.
- Default-visible helper remains conservative.

## Work item 2: Fix `docs/TOOL_REGISTRATION.md` contradictions

### Problem

`docs/TOOL_REGISTRATION.md` still contains old audit text saying MCP listing does not check `mcp_metadata_exposable`, and that MCP listing has a key gap. Later it says protocol listings now filter through `ToolRegistration`. These statements contradict each other after the corrective pass.

### Required changes

Update `docs/TOOL_REGISTRATION.md` sections 4, 6, 8, and 10.

Required current-state wording:

- MCP listing first applies `McpProfilePolicy.filter_tools()` and then filters through `ToolRegistration` profile visibility.
- OpsAgent behavior must match the decision from Work item 1.
- REST listing filters through `ToolRegistration::rest_exposable`, if that is now true in code.
- gRPC listing filters through `ToolRegistration::grpc_exposable`, if that is now true in code.
- Agent listing/dispatch wording should distinguish listing metadata from dispatch authorization.
- Raw dispatch exceptions should remain documented.

Remove or rewrite stale lines such as:

- "MCP listing does not check `mcp_metadata_exposable`."
- "REST/gRPC listing do not filter by exposure flags" if no longer true.
- "Key gap" language that has been closed.

### Acceptance criteria

- The doc does not contradict itself.
- Protocol listing tables reflect actual current code.
- Terms `metadata-exposable`, `default-visible`, `profile-expanded`, and `runtime-approved` are used consistently.

## Work item 3: Rename or clarify `interactive_only`

### Problem

`interactive_only` can be read as "human interactive only," which would include TUI. In the current registry and tests, it means something narrower: helper/config/report-style commands that require CLI/operator interaction and should not be TUI-visible or programmatic.

### Required decision

Choose one approach.

#### Option A: Rename field

Recommended names:

- `cli_interactive_only`
- `operator_cli_only`
- `helper_interactive_only`

Recommended replacement:

```rust
pub cli_interactive_only: bool,
```

Meaning:

- Command is intended for direct CLI/operator invocation only.
- It is not TUI-visible.
- It is not programmatic-visible.

#### Option B: Keep field, strengthen docs

If renaming causes too much churn, update comments and docs:

```rust
/// CLI-helper interactive only. Does not mean all human-interactive surfaces.
/// TUI manual actions use `tui_visible`, not this flag.
```

### Recommended approach

Use Option A. The registry is still young, so renaming now avoids locking in an ambiguous API before Phase 11 CI starts enforcing it.

### Required tests

- Replace `interactive_only_not_programmatic()` with `cli_interactive_only_not_programmatic()` if renamed.
- Replace `tui_visible_excludes_interactive_only()` with a name that matches the new field.
- Add a test that TUI-visible manual commands are allowed and not confused with CLI-helper-only commands.

Example:

```rust
#[test]
fn tui_visible_commands_can_be_manual_operator_actions() {
    let recon = lookup_command("recon").unwrap();
    assert!(recon.tui_visible);
    assert!(!recon.cli_interactive_only);
}
```

### Acceptance criteria

- Field name no longer implies that all manual/TUI commands should be excluded.
- Tests make the intended distinction explicit.
- `docs/COMMAND_REGISTRY.md` uses the same terminology.

## Work item 4: Tighten descriptor-builder tests to dispatch mode

### Problem

`side_effecting_entries_have_descriptor_builder()` checks all side-effecting entries with operation IDs, including legacy-wrapped entries. This currently works because `build_descriptor()` uses metadata, but it blurs the intended distinction:

- Registry-backed commands must have descriptor support.
- Legacy-wrapped commands may have descriptor metadata for docs/preflight, but are not necessarily registry-dispatched.
- Catalog/helper/server commands may not have descriptors.

### Required changes

Replace or split the test into dispatch-mode-specific tests.

Recommended tests:

1. `registry_backed_side_effecting_commands_build_descriptors`
   - Applies only to `CommandDispatchMode::RegistryBacked`.
   - Requires operation ID and descriptor builder.

2. `legacy_wrapped_operation_metadata_is_optional_but_valid_when_present`
   - For `LegacyWrapped` entries with `operation_id`, metadata must resolve.
   - Descriptor generation may be allowed but should be described as metadata descriptor, not dispatch proof.

3. `helper_and_server_commands_do_not_require_descriptors`
   - `HelperOnly` and `ServerLifecycle` entries can have no operation ID.

### Acceptance criteria

- Tests do not imply all side-effecting legacy commands are fully registry-backed.
- Dispatch-mode semantics remain explicit and enforceable.

## Work item 5: Align MCP server comments with chosen visibility model

### Problem

`handle_tools_list()` comments currently mention `mcp_metadata_exposable`, but the semantics need to reflect the chosen profile model.

### Required changes

If using Model A:

- Comment should say:
  - MCP profile policy filters the runtime registry first.
  - `ToolRegistration` then restricts by profile visibility.
  - OpsAgent uses expanded metadata-exposable visibility.
  - Conservative defaults are available through `mcp_tool_registrations_default_visible()` where needed.

If using Model B:

- Comment should say OpsAgent uses default-visible or profile allowlist semantics.

### Acceptance criteria

- Comments are short and point to `docs/TOOL_REGISTRATION.md`.
- Comments do not call metadata-exposable tools "default visible."

## Work item 6: Verify docs after polish

Update these docs as needed:

- `docs/TOOL_REGISTRATION.md`
- `docs/COMMAND_REGISTRY.md`
- `docs/METADATA_OWNERSHIP.md`
- `docs/CAPABILITY_MATRIX.md`
- `docs/FEATURE_MATRIX.md` only if MCP profile or `full` feature wording is touched.

Search for stale terms:

```bash
rg "mcp_exposed_by_default|mcp_metadata_exposable|mcp_default_visible|manual_only|interactive_only|default MCP|OpsAgent" docs crates/eggsec/src crates/eggsec/tests
```

Acceptance criteria:

- No stale `manual_only` wording remains unless referring to old history.
- No doc says MCP listing lacks registration filtering if it does filter now.
- No doc equates OpsAgent profile listing with conservative default listing unless Model B is chosen.

## Work item 7: Validation commands

Run:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --test command_registry
cargo test -p eggsec --test tool_registration
cargo test -p eggsec --test feature_matrix
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --lib
```

If any MCP profile test or protocol listing code changes, also run:

```bash
cargo check -p eggsec --features tool-api,rest-api
cargo check -p eggsec --features db-pentest-mcp,tool-api,rest-api
cargo check -p eggsec --features web-proxy-mcp,tool-api,rest-api
cargo check -p eggsec --features c2-mcp,tool-api,rest-api
```

If TUI naming or docs are touched only indirectly, no TUI compile should be required. If command registry exports are consumed by TUI code, run:

```bash
cargo test -p eggsec-tui --lib
```

## Completion criteria

This polish pass is complete when:

- MCP OpsAgent/default visibility semantics are deliberately chosen and encoded in docs/tests.
- `docs/TOOL_REGISTRATION.md` reflects current code and has no stale key-gap wording.
- `interactive_only` is renamed or documented so it cannot be confused with general TUI/manual interactivity.
- Descriptor-builder tests are scoped to dispatch mode.
- MCP server comments use the chosen terminology.
- Search for stale terms finds no misleading references.
- Validation commands pass or any platform-sensitive skips are documented.

## Handoff note

After this polish pass, the repo should be ready for Phase 11 CI architecture guards. Do not add CI guard jobs until terminology and profile visibility semantics are final; otherwise CI may preserve the wrong invariants.
