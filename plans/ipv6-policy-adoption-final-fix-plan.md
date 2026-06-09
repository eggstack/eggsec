# IPv6 Policy Adoption Final Fix Plan

## Purpose

This is a small corrective handoff plan for the remaining issues after the final policy validation pass. Do not broaden scope. The policy architecture is now mostly in place; this pass should fix the remaining correctness gap and verify that the new shared policy path is actually used.

## Current Assessment

The repo is close but not finished.

Good state:

- `PolicyDecision`, `OperationDescriptor`, and `evaluate_operation_policy` exist and are the correct shared primitives.
- `policy-explain` now uses the loaded execution policy.
- Required Cargo features now populate `missing_features` and deny appropriately.
- MCP has `McpPolicyDenial`, tool-risk classification, coding-agent restrictions, and structured policy-denial helpers.
- Report-policy scaffolding exists through `PolicySummary` and lab-defense report sections.

Remaining blockers:

1. MCP hostname parsing still mishandles bare IPv6 such as `::1` and `2001:db8::1`.
2. Tests do not cover bare IPv6 or global IPv6 parsing.
3. `CommandContext::evaluate_and_enforce_operation` exists, but command-handler adoption still needs an explicit audit.
4. Full validation still needs to be run and any fallout fixed.

## Non-Goals

Do not add new scanner behavior.

Do not change the MCP profile model except where needed for tests or policy adoption.

Do not weaken coding-agent restrictions.

Do not remove lab/hazardous capabilities.

Do not perform broad refactors.

## Phase 1: Fix Bare IPv6 Hostname Parsing

File:

- `crates/eggsec/src/tool/protocol/mcp/policy.rs`

Problem:

The current `extract_hostname` logic strips the suffix after the final colon when that suffix parses as a port. That is valid for `host:8080`, but invalid for bare IPv6 addresses. Examples:

- `::1` can be truncated incorrectly.
- `2001:db8::1` can be truncated incorrectly.

Required behavior:

| Input | Expected host |
|---|---|
| `http://user:pass@host.com:8080/path` | `host.com` |
| `https://example.com` | `example.com` |
| `http://127.0.0.1:3000` | `127.0.0.1` |
| `localhost:8080` | `localhost` |
| `http://[::1]:8080` | `::1` |
| `[::1]:8080` | `::1` |
| `::1` | `::1` |
| `2001:db8::1` | `2001:db8::1` |
| `[2001:db8::1]:443` | `2001:db8::1` |

Implementation guidance:

Use a simple deterministic parser:

1. Trim input.
2. If input has `http://` or `https://`, prefer `url::Url::parse` and `host_str()`.
3. If URL parsing fails, fall back to current manual logic.
4. Strip userinfo before host processing.
5. Strip path/query/fragment before host processing.
6. If host starts with `[`, return the content until `]`.
7. Count colons in the remaining host string:
   - `0` colons: plain host.
   - `1` colon: treat as `host:port` only if suffix parses as `u16`; otherwise return the full string.
   - `>1` colons: treat as bare IPv6 and return the full string.

Avoid returning borrowed data from temporary `Url` values unless the function is changed to return `String` or `Cow<'_, str>`. The simplest safe change is to make `extract_hostname` return `String` and update callers/tests accordingly.

Acceptance criteria:

- Tests cover every row in the table.
- `is_loopback_or_private("::1")` returns true.
- `validate_target("::1")` is allowed for coding-agent.
- `validate_target("http://[::1]:8080")` remains allowed for coding-agent.
- `validate_target("2001:db8::1")` is explicitly tested. Decide expected behavior based on current private/global IPv6 policy; likely denied unless it is loopback/link-local/ULA.

## Phase 2: Add Focused MCP Policy Tests

Add tests for MCP classification and structured denials if not already present.

Required tests:

- `classify_tool_risk("stress") == OperationRisk::StressTest`
- `classify_tool_risk("waf-stress") == OperationRisk::StressTest`
- `classify_tool_risk("packet") == OperationRisk::RawPacket`
- `classify_tool_risk("proxy") == OperationRisk::ExploitAdjacent`
- `classify_tool_risk("remote") == OperationRisk::RemoteExecution`
- coding-agent denies `stress`, `waf-stress`, `packet`, `proxy`, `remote`, and `exec`.
- `policy_decision_for_mcp_call` returns `allowed = false` and includes denied reasons for at least one denied tool.
- `PolicyViolation::to_mcp_error_with_decision` includes serialized decision data in `McpError.data`.

Acceptance criteria:

- The MCP denial helpers are covered by unit tests.
- Tests do not require network traffic.
- Tests use local/private targets only when a target is needed.

## Phase 3: Audit Command Handler Adoption

Run a local grep/audit over command handlers:

```bash
rg "evaluate_and_enforce_operation|evaluate_operation_policy|ensure_scope|ensure_scope_url|check_scope" crates/eggsec/src/commands/handlers crates/eggsec/src/commands
```

Create or update an audit note:

- `docs/internal/POLICY_HANDLER_AUDIT.md`

The audit should classify handlers into:

1. Uses shared policy evaluator.
2. Uses older scope-only helpers and needs migration.
3. Does not take a target and does not require policy evaluation.
4. Deferred with explicit reason.

High-risk handlers that must not be left scope-only:

- `stress`
- `waf_stress`
- `packet`
- `proxy`
- `remote`
- `exec`
- `nse`
- `agent`
- REST/MCP/GRPC tool execution paths

Acceptance criteria:

- Audit doc exists and is accurate.
- Any high-risk target-bearing handler either uses the shared policy path or has a precise TODO explaining why not.
- No handler silently bypasses risk checks while only performing scope checks.

## Phase 4: Migrate One or More High-Risk Handlers if Needed

If the audit finds high-risk handlers still using only `ensure_scope`/`ensure_scope_url`, migrate them.

Recommended minimal migration pattern:

```rust
let descriptor = OperationDescriptor {
    operation: "stress".to_string(),
    mode: OperationMode::HazardousLab,
    risk: OperationRisk::StressTest,
    intended_uses: vec![IntendedUse::DistributedSystemStress],
    target: Some(target.clone()),
    required_features: vec!["stress-testing".to_string()],
    required_policy_flags: vec![],
    requires_private_or_local_target: true,
    requires_explicit_scope: true,
};
let policy_decision = ctx.evaluate_and_enforce_operation(descriptor)?;
```

Attach or log the `policy_decision` where practical.

Acceptance criteria:

- At least the highest-risk handlers have shared policy enforcement.
- Denials include structured policy details in JSON mode.
- Existing behavior remains unchanged for allowed operations.

## Phase 5: Run Validation

Required commands:

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

Manual checks:

```bash
eggsec policy-explain \
  --target http://127.0.0.1:8080 \
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

- Formatting passes.
- Tests pass.
- All-feature tests pass or environmental failures are documented precisely.
- Clippy passes.
- No test makes external network calls.

## Stopping Condition

Stop after:

1. Bare IPv6 parsing is correct and tested.
2. MCP denial helper tests pass.
3. Policy handler audit exists.
4. Any high-risk scope-only handler is migrated or explicitly documented.
5. Validation passes or failures are documented with exact cause.

This should be the final corrective pass for the current policy-hardening thread.
