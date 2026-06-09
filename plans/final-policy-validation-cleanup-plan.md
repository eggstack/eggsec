# Final Policy Validation Cleanup Plan

## Purpose

This is the final narrow cleanup pass for the Eggsec policy/defense-lab hardening work. The repo now has the right core architecture: operation taxonomy, policy decisions, profile-aware planning, MCP profile restrictions, feature-gate reporting, and lab-report policy summaries. The remaining work should be limited to correctness validation, small parser fixes, and ensuring the shared policy path is actually used by command handlers.

Do not expand the feature surface in this pass. Treat this as a stabilization pass.

## Current State

Good current state:

- `OperationRisk`, `OperationMode`, `IntendedUse`, and `OperationDescriptor` exist.
- `PolicyDecision` and `evaluate_operation_policy` are the shared policy primitives.
- `policy-explain` now evaluates using `ctx.config.execution_policy`.
- Required Cargo features are now checked and reflected in `missing_features`.
- `PlanOutput` includes policy decisions and skipped stages.
- `McpProfilePolicy` restricts coding-agent tools, targets, concurrency, timeout, stress, packet, broad recon, and hazardous arguments.
- MCP denials can now embed a shared `PolicyDecision` via `McpPolicyDenial`.
- Report scaffolding now has `PolicySummary` and `LabDefenseReportSection`.

Remaining concerns:

1. MCP hostname parsing still appears vulnerable to bare IPv6 edge cases such as `::1` and `2001:db8::1`.
2. `CommandContext::evaluate_and_enforce_operation` exists, but command handlers may not consistently call it yet.
3. MCP policy-denial helpers exist, but they may not yet be wired into the actual `tools/call` JSON-RPC error path.
4. Policy report scaffolding exists, but scan/report output may not yet consistently include policy summaries.
5. Scope TOML examples and parser behavior should be confirmed before calling the safety docs stable.
6. The full test matrix still needs to be run and any fallout fixed.

## Non-Goals

Do not remove stress, raw packet, proxy, WAF-stress, distributed, NSE, or Synvoid defense-lab features.

Do not add new scan techniques, payload families, or offensive workflows.

Do not redesign the policy model.

Do not introduce a new parser dependency unless necessary. The existing `url` crate is already in the workspace and should be preferred if URL parsing is needed.

Do not make coding-agent MCP broader. It should remain constrained.

## Phase 1: Fix Final Hostname Parsing Edge Cases

Audit `extract_hostname` in `crates/eggsec/src/tool/protocol/mcp/policy.rs`.

Known issue to fix:

- The implementation strips a numeric suffix after the last colon even for bare IPv6, so `::1` can be interpreted incorrectly as host `:` with port `1`.
- `2001:db8::1` may similarly be truncated.

Desired behavior:

| Input | Expected normalized host |
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

- If the input has a scheme, try `url::Url::parse` and use `host_str()`.
- For schemeless bracketed IPv6, return the content between `[` and `]`.
- For unbracketed strings:
  - Count colons.
  - If there are zero colons, return the host.
  - If there is exactly one colon and the suffix parses as a port, strip the port.
  - If there is more than one colon, treat it as bare IPv6 and do not strip anything.
- Keep cloud metadata endpoint detection working.

Acceptance criteria:

- Add or update unit tests for every row in the table above.
- `validate_target("http://[::1]:8080")` remains allowed for coding-agent.
- `validate_target("::1")` is allowed.
- `validate_target("2001:db8::1")` should follow the intended policy for non-local IPv6; decide explicitly and test it.
- Metadata endpoint denials still pass.

## Phase 2: Audit Command Handler Policy Adoption

Search all command handlers under `crates/eggsec/src/commands/handlers/` for target-bearing operations.

Classify each handler into one of three categories:

1. Uses `ctx.evaluate_and_enforce_operation` or `evaluate_operation_policy` directly.
2. Uses older scope-only helpers such as `ensure_scope` / `ensure_scope_url`.
3. Does not touch targets or does not need policy evaluation.

Target-bearing handlers to audit at minimum:

- `scan`
- `scan_ports`
- `scan_endpoints`
- `fingerprint`
- `fuzz`
- `waf`
- `waf_stress`
- `graphql`
- `oauth`
- `auth_test`
- `recon`
- `load`
- `stress`
- `packet`
- `icmp`
- `traceroute`
- `cluster`
- `remote`
- `exec`
- `nse`
- `browser`
- `wireless`
- `agent`
- `serve` / REST tool execution
- `mcp_serve`
- `grpc`

Acceptance criteria:

- Add a short internal audit note, either as a code comment near `CommandContext` or as `docs/internal/POLICY_HANDLER_AUDIT.md`.
- Migrate high-risk handlers first: `stress`, `waf_stress`, `packet`, `proxy`, `remote`, `exec`, `nse`, `agent`.
- At minimum, no high-risk handler should rely only on `ensure_scope` without an operation-risk check.
- Every migrated handler should build an `OperationDescriptor` with correct mode, risk, intended use, and target.
- If any handler cannot be migrated in this pass, add a TODO with the exact reason and risk.

## Phase 3: Wire MCP Policy Denials Into Actual JSON-RPC Errors

`McpPolicyDenial`, `policy_decision_for_mcp_call`, and `to_mcp_error_with_decision` exist. Verify they are actually used in the request path for `tools/call`.

Tasks:

- Inspect `crates/eggsec/src/tool/protocol/mcp/handlers.rs` and any route/stdio dispatch code.
- Locate where `McpProfilePolicy::validate_tool_call` and `validate_target` are called.
- Ensure denials return an MCP error whose `data` contains the serialized `PolicyDecision` or `McpPolicyDenial`.
- Ensure canonical denied tools and aliases behave the same.

Acceptance criteria:

- Coding-agent call to `stress`, `waf-stress`, `packet`, `proxy`, `remote`, or `exec` returns a policy error with structured policy data.
- Coding-agent call to an allowed localhost/local-private verification tool succeeds when otherwise valid.
- Coding-agent call to public target returns structured target-denial policy data.
- Ops-agent call still respects execution policy and scope when applicable.
- Unit or integration tests cover at least one denied tool, one denied target, and one allowed local call.

## Phase 4: Confirm Policy Summary Is Included in Reports Where Available

Report scaffolding exists through `PolicySummary` and `LabDefenseReportSection`. Confirm whether real scan/report conversion paths include it.

Tasks:

- Inspect report conversion in `crates/eggsec-output/src/convert.rs` and engine report builders.
- Determine whether `ScanReportData` has a `policy_summary` field or equivalent.
- If absent, add optional policy summary support without breaking existing consumers.
- Ensure Markdown/HTML renderers include a concise policy section when present.
- Keep SARIF/JUnit conservative; policy denials are not vulnerabilities.

Acceptance criteria:

- JSON report output can include a policy summary when policy decisions are available.
- Markdown/HTML output has a short policy section when present.
- Existing reports without policy summary still deserialize.
- Tests or fixtures cover policy summary serialization.

## Phase 5: Confirm Scope TOML Syntax and Examples

Validate that scope examples match the actual parser.

Tasks:

- Inspect `ScopeRule`, `Scope`, and `load_scope` parsing.
- Verify whether CIDR rules are expressed using `pattern = "10.0.0.0/8"`, `cidr = "10.0.0.0/8"`, or both.
- Standardize all docs and examples accordingly.
- Add a parser test for the documented examples.

Files to audit:

- `README.md`
- `docs/SAFETY.md`
- `docs/lab/SYNVOID_DEFENSE_LAB.md`
- `docs/CAPABILITIES.md`
- `examples/*.toml`
- test fixtures

Acceptance criteria:

- Every documented scope file parses.
- `scope-explain` works for localhost, private CIDR, public target, and excluded target examples.
- Invalid scope syntax emits a clear error.

## Phase 6: Run Full Validation Matrix

Run the complete validation matrix and fix failures.

Required commands:

```bash
cargo fmt --all
cargo test --workspace
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Feature-build checks:

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

If feature-specific builds fail because optional system dependencies are missing, document the failure precisely and separate environmental failures from code failures.

Acceptance criteria:

- Formatting passes.
- Workspace tests pass.
- All-feature tests pass, or any failure is documented with a clear environmental reason.
- Clippy passes or only documented existing warnings remain.
- No tests send external network traffic.

## Phase 7: Final Documentation Touch-Up

Update docs only where needed after the code cleanup.

Required doc checks:

- `README.md` accurately describes defense-lab and hazardous-lab modes.
- `docs/SAFETY.md` names current risk enums and policy flags.
- `docs/lab/SYNVOID_DEFENSE_LAB.md` uses valid scope syntax and working commands.
- MCP docs, if present, describe coding-agent restrictions and structured denial behavior.
- Any mention of policy summaries or reports matches implemented output.

Acceptance criteria:

- No stale references to removed names or old risk labels.
- Copy-paste examples are valid.
- Docs continue to frame stress/raw/lab capabilities as authorized defensive validation, especially Synvoid/WAF/distributed-system hardening.

## Suggested Implementation Order

1. Fix `extract_hostname` IPv6 edge cases and tests.
2. Audit command handlers for shared policy adoption.
3. Wire MCP denial helpers into actual JSON-RPC response path if not already wired.
4. Confirm policy summaries are included in real report output where available.
5. Standardize scope syntax examples.
6. Run full validation matrix and fix failures.
7. Perform final documentation touch-up.

## Handoff Notes

This should be the stopping-point pass for the current policy hardening thread. Avoid new architecture unless a test failure proves it is necessary. The intended final state is boring: tests pass, parser behavior is correct, policy decisions are consistently emitted, and risky lab tooling remains available only through explicit scope/policy/budget boundaries.
