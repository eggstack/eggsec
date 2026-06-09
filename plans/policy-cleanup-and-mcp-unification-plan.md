# Policy Cleanup and MCP Unification Plan

## Purpose

This plan is a narrow cleanup pass after the policy-integration hardening work. The repo now has a much better policy taxonomy, `OperationDescriptor`, `PolicyDecision`, a shared `evaluate_operation_policy` function, profile-aware `plan` output, and stronger MCP profile boundaries. The remaining issues are mostly integration seams and likely test failures.

The goal of this pass is to make the policy model reliable and consistent across CLI, plan, policy-explain, MCP, and reports without expanding the feature surface.

## Current State Summary

Implemented and in good shape:

- `OperationRisk::ExploitAdjacent` is now distinct from `AgentAutonomous`.
- `ExecutionPolicy` has `allow_exploit_adjacent`.
- `ProbeRisk::ExploitAdjacent` maps to `OperationRisk::ExploitAdjacent`.
- `OperationDescriptor` exists.
- `evaluate_operation_policy` exists.
- `PlanOutput`, `PlannedStage`, and `SkippedStage` exist.
- `plan` now builds per-stage policy decisions.
- `ScanProfile` has `from_str`, `operation_mode`, `intended_uses`, feature helpers, and risk-budget helpers.
- `McpProfilePolicy` now restricts coding-agent tools, arguments, targets, concurrency, timeout, stress, packet features, and broad recon.

Known remaining issues:

1. `extract_hostname` in MCP policy likely fails its own tests for userinfo URLs and bracketed IPv6.
2. `policy-explain` appears to evaluate against `ExecutionPolicy::default()` instead of the loaded `ctx.config.execution_policy`.
3. `evaluate_operation_policy` copies required features into `PolicyDecision`, but does not populate `missing_features` or deny when required compile-time features are absent.
4. `CommandContext::enforce_operation_policy` still performs legacy inline checks rather than wrapping `evaluate_operation_policy`.
5. MCP denials use `PolicyViolation`, not the shared `PolicyDecision` contract.
6. MCP policy is still parallel to the main policy evaluator rather than integrated with `OperationDescriptor`.
7. Report output may not yet include policy summaries from `PlanOutput` or scan execution.

## Non-Goals

Do not remove stress, packet, WAF-stress, proxy, distributed, or Synvoid lab functionality.

Do not loosen safety gates.

Do not redesign the entire MCP server.

Do not introduce a new policy system. Use the existing `OperationDescriptor`, `PolicyDecision`, `ExecutionPolicy`, and `McpProfilePolicy` types.

Do not add new scanners, fuzzers, payloads, or offensive behaviors.

## Phase 1: Fix MCP Target Hostname Parsing

The current MCP `extract_hostname` helper appears to split host strings on `:` too early. It likely fails cases such as:

```text
http://user:pass@host.com:8080/path
http://[::1]:8080
[::1]:8080
```

Implement robust parsing.

Recommended approach:

1. Try `url::Url::parse(target)` first when the input includes a scheme.
2. Use `Url::host_str()` for URL inputs.
3. Preserve bracketed IPv6 handling correctly.
4. For schemeless inputs, manually handle:
   - bracketed IPv6 with optional port: `[::1]:8080`
   - plain IPv6: `::1`
   - host:port: `localhost:8080`
   - userinfo-style strings if they can occur
5. Avoid treating the username segment as the host.

Acceptance criteria:

- Existing tests for userinfo URLs and bracketed IPv6 pass.
- Add tests for:
  - `http://user:pass@host.com:8080/path` -> `host.com`
  - `https://example.com` -> `example.com`
  - `http://127.0.0.1:3000` -> `127.0.0.1`
  - `http://[::1]:8080` -> `::1` or `[::1]`, but tests and implementation must agree.
  - `[::1]:8080` -> `::1` or `[::1]`, but tests and implementation must agree.
  - `::1` -> `::1`
  - `localhost:8080` -> `localhost`
- `validate_target` correctly allows localhost/private targets and denies public/metadata targets after parsing.

Implementation note: prefer normalizing IPv6 to the unbracketed host string internally, then adjust tests accordingly.

## Phase 2: Make `policy-explain` Use Loaded Configuration

`policy-explain` should reflect the actual loaded execution policy. It should not use `ExecutionPolicy::default()` unless no config is loaded.

Current issue:

- `cli::explain::evaluate_policy_decision` constructs/evaluates using `ExecutionPolicy::default()`.
- `commands::handlers::explain::handle_policy_explain` has access to `ctx.config.execution_policy`, but does not pass it into the helper.

Recommended fix:

- Change helper signature to accept `&ExecutionPolicy`.
- Pass `&ctx.config.execution_policy` from the handler.
- Keep a test helper for default policy if needed, but name it explicitly.

Suggested shape:

```rust
pub fn evaluate_policy_decision(
    target: Option<&str>,
    profile_name: Option<&str>,
    scope: Option<&Scope>,
    policy: &ExecutionPolicy,
) -> PolicyDecision
```

Acceptance criteria:

- A config enabling `allow_intrusive_fuzzing = true` causes `policy-explain --profile waf-regression` to reflect that policy.
- A default config still denies intrusive operations.
- Tests cover both default-denied and config-allowed behavior.
- Human and JSON output remain stable.

## Phase 3: Add Compile-Time Feature Availability Reporting

`PolicyDecision` has `required_features` and `missing_features`, but the shared evaluator currently does not appear to populate `missing_features`.

Add a small feature-availability helper.

Suggested approach:

```rust
pub fn is_feature_enabled(feature: &str) -> bool {
    match feature {
        "packet-inspection" => cfg!(feature = "packet-inspection"),
        "stress-testing" => cfg!(feature = "stress-testing"),
        "nse" => cfg!(feature = "nse"),
        "nse-sandbox" => cfg!(feature = "nse-sandbox"),
        "headless-browser" => cfg!(feature = "headless-browser"),
        "rest-api" => cfg!(feature = "rest-api"),
        "grpc-api" => cfg!(feature = "grpc-api"),
        "ws-api" => cfg!(feature = "ws-api"),
        _ => true, // or false; choose deliberately and document it
    }
}
```

Then update `evaluate_operation_policy`:

- copy `required_features`
- for each unavailable required feature, push to `missing_features`
- set `allowed = false` when a required feature is missing
- add a denial reason such as `required feature 'packet-inspection' is not enabled`

Acceptance criteria:

- `plan --profile protocol-edge` reports `packet-inspection` as missing when not built with that feature.
- `plan --profile nse-safe` reports `nse` as missing when not built with that feature.
- Tests cover both unavailable-feature behavior and feature-enabled behavior using `cfg` gates where necessary.
- Docs state that `required_features` are compile-time Cargo features, not config flags.

## Phase 4: Convert Legacy `CommandContext::enforce_operation_policy`

`CommandContext::enforce_operation_policy` still performs inline checks. Convert it into a wrapper around the shared policy evaluator or remove it after migration.

Recommended wrapper:

```rust
pub fn enforce_operation_policy(
    &self,
    descriptor: OperationDescriptor,
) -> Result<PolicyDecision>
```

Behavior:

- Call `evaluate_operation_policy(&descriptor, &self.config.execution_policy, Some(&self.scope))`.
- If denied and `self.json`, print/return structured JSON error where the command architecture permits.
- If denied and human mode, return an error containing `decision.to_human_readable()`.
- If allowed, return the `PolicyDecision` so callers can attach it to reports/logs.

Migration strategy:

- Add a new method, e.g. `evaluate_and_enforce_operation`.
- Migrate callers incrementally.
- Mark old risk-only method deprecated or remove after all callers are updated.

Acceptance criteria:

- No command path performs ad hoc risk/scope checks if it can use the shared evaluator.
- Existing call sites compile.
- Denial errors include policy decision details.
- Tests cover at least one command denial path through `CommandContext`.

## Phase 5: Unify MCP Denials With `PolicyDecision`

MCP currently has `McpProfilePolicy` and `PolicyViolation`, which is useful, but denials are separate from the main `PolicyDecision` contract.

Do not delete `PolicyViolation`; instead, create a stable wrapper that embeds or references a policy decision.

Suggested shape:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPolicyDenial {
    pub violation: String,
    pub code: i32,
    pub policy_decision: PolicyDecision,
}
```

For tool/profile-level denials where there is no target, build an `OperationDescriptor` with:

- `operation = tool_id`
- `mode = OperationMode::StandardAssessment` or profile-derived mode
- `risk = inferred tool risk`
- `intended_uses = vec![CodingAgentVerification]` for coding-agent profile
- `target = extracted target if present`

Add a helper:

```rust
fn policy_decision_for_mcp_call(
    profile_policy: &McpProfilePolicy,
    tool_id: &str,
    arguments: &serde_json::Value,
    execution_policy: &ExecutionPolicy,
    scope: Option<&Scope>,
) -> PolicyDecision
```

Acceptance criteria:

- Coding-agent denied tool calls can return/serialize a `PolicyDecision` or `McpPolicyDenial` containing it.
- Denied public targets include the target and denial reason.
- Denied stress/raw/proxy/remote-style tools include profile/tool denial context.
- Tests cover:
  - coding-agent stress/raw/proxy denial
  - coding-agent public target denial
  - coding-agent localhost allowed
  - ops-agent still policy-gated by execution policy/scope where applicable

## Phase 6: Improve MCP Tool Risk Metadata

`McpProfilePolicy::validate_tool_call` currently builds a synthetic `ToolInfo` with category `Scanning` for selector checks. That can hide category-based denials for tools that are not in the registry or when call-time metadata is absent.

Recommended fix:

- Prefer looking up real `ToolInfo` from the registry before validation.
- If synthetic fallback is necessary, infer category/risk from the tool ID.
- Add a small static classifier for known risky tool IDs/aliases.

Suggested classifier:

```rust
fn classify_mcp_tool(tool_id: &str) -> ToolRiskMetadata {
    match tool_id {
        "stress" | "waf-stress" => StressTest,
        "packet" | "raw-packet" => RawPacket,
        "proxy" => ExploitAdjacent,
        "remote" | "exec" => RemoteExecution,
        "load" => LoadTest,
        "fuzz" => Intrusive,
        _ => SafeActive,
    }
}
```

Acceptance criteria:

- Coding-agent denial does not depend only on exact allow-list strings.
- Tool aliases cannot bypass denial.
- Category and risk metadata are consistent with CLI policy.
- Tests cover aliases if aliases exist.

## Phase 7: Report Policy Summaries

If report output does not already include policy summaries, add them for JSON and human-oriented formats.

Minimum JSON shape:

```json
{
  "policy_summary": {
    "operation_mode": "defense-lab",
    "max_risk": "intrusive",
    "decisions": [],
    "denied_count": 0,
    "warning_count": 0
  }
}
```

Rules:

- Policy denials are not vulnerabilities.
- Stress/load metrics are not vulnerabilities unless converted into an actionable finding elsewhere.
- Markdown/HTML should separate vulnerabilities, observations, policy denials, and lab metrics.
- SARIF/JUnit should stay conservative.

Acceptance criteria:

- JSON reports include a policy summary when policy decisions are available.
- Markdown/HTML include a concise policy section.
- Golden fixtures are updated or added.
- Existing output consumers are not broken without a version bump or documented schema change.

## Phase 8: Validate Scope Syntax Consistency

Confirm scope TOML syntax across docs and examples.

Tasks:

- Inspect `ScopeRule` and `load_scope` parser behavior.
- Decide whether CIDRs should be expressed with `cidr = "10.0.0.0/8"`, `pattern = "10.0.0.0/8"`, or both.
- Standardize all docs and examples.
- Add config validation tests for CIDR examples.

Files to audit:

- `README.md`
- `docs/SAFETY.md`
- `docs/lab/SYNVOID_DEFENSE_LAB.md`
- `docs/CAPABILITIES.md`
- `examples/*.toml`
- tests/fixtures

Acceptance criteria:

- Every documented scope file parses.
- `scope-explain` behaves correctly for domain, localhost, private CIDR, public IP, and excluded targets.
- Invalid CIDR/scope syntax produces a clear validation error.

## Phase 9: Run and Fix Test Suite

Run the full suite after implementation.

Minimum commands:

```bash
cargo fmt --all
cargo test --workspace
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Feature builds:

```bash
cargo build -p eggsec-cli
cargo build -p eggsec-cli --features stress-testing
cargo build -p eggsec-cli --features packet-inspection
cargo build -p eggsec-cli --features "rest-api ai-integration"
cargo build -p eggsec-cli --features full
```

Manual dry-run checks:

```bash
eggsec policy-explain \
  --target http://127.0.0.1:8080 \
  --profile waf-regression \
  --scope examples/scope-localhost.toml \
  --json

eggsec policy-explain \
  --target https://example.com \
  --profile waf-regression \
  --scope examples/scope-localhost.toml \
  --json

eggsec plan \
  --target http://127.0.0.1:8080 \
  --profile protocol-edge \
  --scope examples/scope-localhost.toml \
  --format json
```

Acceptance criteria:

- Full test suite passes.
- No external network traffic in tests.
- Denied paths produce structured policy information.
- No aliases bypass MCP or CLI policy checks.

## Suggested Implementation Order

1. Fix MCP hostname parsing first; this is likely test-breaking.
2. Make `policy-explain` use `ctx.config.execution_policy`.
3. Add feature availability checks and `missing_features` population.
4. Convert or wrap `CommandContext::enforce_operation_policy` around the shared evaluator.
5. Add MCP denial wrapper containing `PolicyDecision`.
6. Improve MCP tool risk/alias classification.
7. Add report policy summaries.
8. Standardize scope syntax and docs.
9. Run the full validation matrix.

## Handoff Notes

This is a cleanup and unification pass. The previous work added the right architecture. This pass should make the implementation trustworthy by removing parallel behavior, fixing likely failing tests, and making policy output stable for downstream consumers such as codegg.

The most important invariant remains: every target-bearing operation should either produce or consume a shared policy decision, and policy denials should be inspectable in a stable structured form.
