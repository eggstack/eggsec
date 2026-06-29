# Phase 7 Handoff Plan: REST API Posture Completion

## Goal

Complete the REST API posture work after the corrective pass. REST is now wired through `EnforcementContext`, but the remaining goal is to make its strict programmatic behavior unambiguous, tested, documented, and aligned with the metadata-derived descriptor work from Phase 6.

The existing REST API should be strict by default. A separate local/manual REST mode may be added later, but it should not be implicit and should not weaken the current programmatic API.

## Current context

The corrective pass made major progress:

- `RestState` carries `EnforcementContext`.
- `handle_serve()` constructs enforcement through `ExecutionSurface::RestApi`.
- REST builds an `OperationDescriptor` and evaluates enforcement before dispatch.
- REST no longer dispatches solely after raw scope checks.

Remaining concerns:

- REST currently permits `Warn` to dispatch. Strict programmatic surfaces should dispatch only on `Allow`.
- REST still uses a local string-based descriptor helper unless Phase 6 has already replaced it with canonical metadata.
- Error responses are still coarse and do not consistently expose structured policy decision data.
- REST docs should explicitly state that existing REST is strict programmatic API, not manual operator mode.

## Files likely to change

- `crates/eggsec/src/tool/protocol/rest.rs`
- `crates/eggsec/src/commands/handlers/serve.rs`
- `crates/eggsec/src/tool/metadata.rs` if Phase 6 exists
- `docs/ENFORCEMENT_MODES.md`
- `architecture/overview.md`
- Any REST API docs/OpenAPI output
- REST tests in `rest.rs` or integration tests

## Step 1: Deny REST warnings by default

In REST execute path, change enforcement handling from:

```rust
EnforcementOutcome::Allow(_) | EnforcementOutcome::Warn(_) => {}
```

to:

```rust
EnforcementOutcome::Allow(_) => {}
EnforcementOutcome::Warn(decision) => deny_rest_policy(decision, "REST strict enforcement warning")?
EnforcementOutcome::RequireConfirmation(decision) => deny_rest_policy(decision, "manual confirmation unavailable over REST")?
EnforcementOutcome::Deny(decision) => deny_rest_policy(decision, "REST strict enforcement denied")?
```

REST should be noninteractive and programmatic, so warning-class ambiguity should not dispatch. If a future local/manual REST mode is added, it must be explicitly named and separately configured.

## Step 2: Add structured REST policy error response

Create a REST-local policy error response:

```rust
#[derive(Debug, Serialize)]
pub struct RestPolicyErrorResponse {
    pub error: String,
    pub code: &'static str,
    pub decision: PolicyDecision,
}
```

Add helper:

```rust
fn policy_denied_response(message: impl Into<String>, decision: PolicyDecision) -> axum::response::Response
```

If the existing handler signature makes custom response awkward, change `execute_tool` to return `Result<impl IntoResponse, EggsecError>` or introduce a REST-local error enum.

Target behavior:

- Policy denial returns HTTP 403.
- Body includes `code: "POLICY_DENIED"`.
- Body includes serialized `PolicyDecision`.
- Auth/rate/target validation can keep existing error behavior.

## Step 3: Use metadata-derived descriptors if Phase 6 has landed

If Phase 6 is already implemented, remove the local string-classified `operation_descriptor_for_rest_tool()` helper and use metadata lookup:

```rust
let metadata = metadata_for_tool_id(&tool_id)
    .ok_or_else(|| policy/internal error)?;
if !metadata.rest_exposable { deny }
let descriptor = metadata.descriptor_for_target(Some(payload.target.clone()));
```

If Phase 6 has not landed, keep the local helper but make it explicit as transitional:

```rust
// Transitional until Phase 6 metadata-derived descriptors replaces this helper.
```

Do not proceed with additional string maps if metadata is already available.

## Step 4: Enforce REST exposure flags

When metadata exists, REST must check `rest_exposable` before policy evaluation. Missing or false exposure should fail closed:

- Missing metadata: 500/internal configuration error in development, or 403/disabled in production.
- `rest_exposable == false`: 403 with `TOOL_NOT_REST_EXPOSABLE` or equivalent.

Add tests for both cases.

## Step 5: Clarify scope precedence

`handle_serve()` currently allows `ServeArgs.scope_file` to override top-level `ctx.enforcement.loaded_scope`. Keep this behavior if it matches CLI shape, but document it in code and docs:

- `eggsec --scope global.toml serve --scope-file rest.toml` uses REST-specific `scope_file`.
- Without REST-specific scope file, REST inherits the loaded top-level scope.

Add tests around helper extraction if possible:

```rust
fn resolve_rest_loaded_scope(ctx, args) -> Result<LoadedScope>
```

## Step 6: Add REST posture tests

Required tests:

- REST surface maps to strict profile.
- REST dispatch only proceeds on `Allow`.
- REST `Warn` becomes HTTP 403 / policy denied.
- REST `RequireConfirmation` becomes HTTP 403 / policy denied.
- REST `Deny` becomes HTTP 403 / policy denied.
- Missing explicit manifest denies target-bearing REST operations.
- Positive scope allow match can proceed to dispatcher path.
- Positive scope miss denies.
- Manual override flags/config do not affect REST.
- Metadata `rest_exposable == false` denies if Phase 6 metadata exists.

Unit tests are acceptable if full Axum tests are expensive, but include at least one route-level test if the repo already has Axum test utilities.

## Step 7: Update OpenAPI and docs

Update `openapi_spec()` if it is hand-written:

- Add 403 response for policy denial.
- Add schema fields for `POLICY_DENIED` if practical.
- State that execution is evaluated under strict REST enforcement.

Update docs:

- REST is programmatic strict mode.
- REST does not provide manual confirmation.
- REST requires explicit scope provenance for target-bearing operations.
- Manual CLI/TUI remain the operator-discretion surfaces.

## Acceptance criteria

- REST dispatches only on `EnforcementOutcome::Allow`.
- `Warn`, `RequireConfirmation`, and `Deny` all return structured policy-denied responses.
- REST uses metadata-derived descriptors if Phase 6 is complete.
- REST exposure flags are enforced when metadata exists.
- Scope precedence is documented and tested.
- REST docs/OpenAPI describe strict enforcement.

## Validation commands

Run:

```bash
cargo fmt --all
cargo test -p eggsec --features rest-api --lib tool::protocol::rest
cargo test -p eggsec --features rest-api --lib config::policy_decision
cargo check -p eggsec-cli --features rest-api
```

If route-level tests exist, run the relevant integration test target as well.

## Non-goals

- Do not add a local/manual REST mode in this phase.
- Do not weaken MCP or agent strict behavior.
- Do not change manual CLI/TUI behavior.
- Do not introduce type-level dispatch tokens yet.
