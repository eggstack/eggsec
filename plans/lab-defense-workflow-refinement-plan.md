# Lab Defense Workflow Refinement Plan

> **Status**: All 12 phases implemented (2026-06-09)

## Purpose

This plan refines Eggsec's risky or ambiguous capability surface without removing the low-level tools that exist for legitimate defensive validation. The goal is to make stress testing, raw-packet tooling, proxy behavior, distributed-network probes, WAF regression tests, and Synvoid-specific validation clearly framed as scoped lab-defense workflows rather than generic offensive primitives.

Eggsec should continue to support aggressive traffic generation and low-level network testing for systems the operator owns or is explicitly authorized to test. The implementation work in this plan should make that intent explicit in code, CLI UX, documentation, MCP exposure, policy enforcement, reports, and tests.

## Current Context

Eggsec already has a strong architecture for this direction. The workspace is split into separate crates for core types, tool abstractions, the main assessment engine, NSE compatibility, TUI, CLI, output, and agent orchestration. The CLI and docs expose stress testing, packet inspection, WAF stress testing, proxy management, distributed scanning, remote/exec infrastructure, MCP profiles, and agent workflows.

These capabilities are valuable for hardening Synvoid and for validating distributed/networked systems under abnormal protocol and traffic conditions. The issue is not that the capabilities exist. The issue is that the repo should make their safety boundary and intended defensive use cases unambiguous.

## Desired End State

Eggsec should present three clearly separated operating modes:

1. `standard-assessment`: ordinary scoped recon, scanning, fuzzing, API testing, WAF detection, and reporting.
2. `defense-lab`: local/private/scope-constrained WAF and distributed-system validation, including load and selected stress tests.
3. `hazardous-lab`: raw packet operations, flood-style stress tests, proxy rotation, low-level protocol edge cases, and other aggressive tests requiring explicit build features plus explicit runtime policy approval.

The implementation should avoid hiding these tools when they are useful, but it should make accidental or ambiguous invocation difficult. Every target-bearing operation should be traceable through a common policy decision record. Every high-risk denial should explain exactly what was blocked and how to configure an authorized lab run correctly.

## Non-Goals

Do not remove stress testing, packet inspection, WAF stress testing, Synvoid profiles, distributed-network probes, or NSE compatibility.

Do not reintroduce arbitrary Python, Ruby, or Metasploit-style plugin runtimes.

Do not weaken scope enforcement for developer convenience.

Do not make coding-agent MCP access equivalent to ops-agent access. The coding-agent profile should remain bounded to local/scope-approved verification and should not expose broad recon, stress testing, raw packets, proxy rotation, remote execution, or autonomous external scans.

Do not attempt to implement new exploit chains. This pass is about safety model, workflow clarity, regression utility, and operational reliability.

## Phase 1: Introduce a Unified Operation Taxonomy

Create or refine a central operation taxonomy used by CLI commands, pipelines, tools, MCP profiles, agent workflows, and reports.

Recommended taxonomy:

```rust
pub enum OperationMode {
    StandardAssessment,
    DefenseLab,
    HazardousLab,
}

pub enum OperationRisk {
    Passive,
    SafeActive,
    Intrusive,
    LoadTest,
    StressTest,
    RawPacket,
    CredentialTesting,
    RemoteExecution,
    AgentAutonomous,
}

pub enum IntendedUse {
    WebAssessment,
    ApiAssessment,
    WafRegression,
    SynvoidRegression,
    DistributedSystemStress,
    ProtocolEdgeValidation,
    CiRegression,
    CodingAgentVerification,
}
```

If similar enums already exist, do not duplicate them. Extend or consolidate the existing `ProbeRisk` / execution policy representation instead.

Acceptance criteria:

- Each target-bearing command and pipeline stage declares its `OperationRisk` and intended use.
- Each scan profile maps to an `OperationMode` and maximum risk budget.
- The mapping is covered by unit tests.
- Documentation uses the same terms as the code.

## Phase 2: Add Policy Decision Records

Implement a structured policy decision object that can be emitted in JSON, logs, reports, and MCP responses.

Suggested fields:

```rust
pub struct PolicyDecision {
    pub decision_id: String,
    pub allowed: bool,
    pub operation: String,
    pub operation_mode: OperationMode,
    pub operation_risk: OperationRisk,
    pub intended_use: IntendedUse,
    pub target_original: Option<String>,
    pub target_normalized: Option<String>,
    pub resolved_addresses: Vec<String>,
    pub matched_scope_rules: Vec<String>,
    pub matched_exclusion_rules: Vec<String>,
    pub required_features: Vec<String>,
    pub missing_features: Vec<String>,
    pub required_policy_flags: Vec<String>,
    pub denied_reasons: Vec<String>,
    pub warnings: Vec<String>,
    pub budgets: Option<ExecutionBudgetSummary>,
}
```

This should be created by one policy evaluation path, not manually reconstructed in each command.

Acceptance criteria:

- `eggsec plan` emits policy decisions for every planned stage.
- Denied commands print a human-readable denial and include the structured decision when `--json` is used.
- MCP tool denials return the same decision data rather than free-text-only errors.
- Agent logs include policy decision IDs for all autonomous operations.

## Phase 3: Add `policy explain` and `scope explain`

Add CLI commands that let the user inspect safety decisions before running traffic-generating operations.

Suggested commands:

```bash
eggsec policy explain --target http://127.0.0.1:8080 --profile waf-regression --scope examples/scope-localhost.toml

eggsec scope explain --target 10.0.0.5 --scope examples/scope-synvoid-lab.toml
```

The output should explain target normalization, allowed/excluded rule matches, operation risk, required build features, required config flags, and budget constraints.

Acceptance criteria:

- Commands support human-readable and JSON output.
- Commands perform no network traffic except optional DNS resolution when explicitly requested.
- Docs include examples for allowed localhost, denied public host, allowed private CIDR, and excluded target.

## Phase 4: Clarify CLI Namespaces for Lab-Only Operations

Refine CLI help and command grouping so high-risk tools are framed as lab-defense tools.

Options:

- Keep existing command names but update help text and long descriptions.
- Add aliases under a clearer namespace, such as `eggsec lab stress`, `eggsec lab packet`, `eggsec lab waf-regression`, and `eggsec lab synvoid`.
- Avoid breaking existing commands unless the codebase is still early enough that breakage is acceptable.

Recommended approach for this pass: keep existing commands, add clearer help text, and optionally add non-breaking aliases.

Commands to audit:

- `waf-stress`
- `stress`
- `packet`
- `proxy`
- `icmp`
- `traceroute`
- `cluster`
- `remote`
- `exec`
- `nse`
- `agent`

Acceptance criteria:

- Help text explicitly says whether a command is standard, defense-lab, or hazardous-lab.
- Help text names the expected use case, such as WAF regression, Synvoid hardening, distributed-system stress validation, or protocol edge validation.
- High-risk commands name the required feature flag and policy flag.
- No help text implies unscoped internet scanning or offensive use.

## Phase 5: Create Defense-Lab Presets

Add first-class presets for the main legitimate lab workflows.

Recommended presets:

```text
synvoid-local
synvoid-waf-regression
synvoid-protocol-edge
distributed-system-smoke
distributed-system-stress
waf-regression-safe
waf-regression-intrusive
```

Each preset should define:

- allowed operation mode
- maximum risk tier
- default concurrency
- max duration
- max request count
- max packet count if applicable
- default payload families
- whether DNS resolution is allowed
- whether raw sockets are allowed
- whether external targets are allowed
- whether localhost/private CIDR is required
- output/report defaults

Acceptance criteria:

- Presets are represented in code, not only docs.
- Presets can be referenced from CLI profiles or config.
- Presets produce stable `plan` output.
- Presets are covered by snapshot/golden tests.

## Phase 6: Add Budget Enforcement Everywhere

Stress and low-level tests should always have explicit budgets. Budgets should be enforced centrally where possible.

Budget types:

- max duration
- max requests
- max packets
- max bytes
- max concurrency
- max targets
- max resolved addresses per host
- max payloads
- cooldown between runs
- per-target rate limit

Acceptance criteria:

- No stress/raw-packet operation can run without a finite duration and at least one additional finite bound, such as packet count or byte count.
- Defaults are conservative.
- Exceeding a budget returns a structured stop reason, not a generic error.
- Reports include consumed budget and termination reason.
- Agent workflows respect per-target cooldowns and do not bypass CLI-equivalent budgets.

## Phase 7: Make WAF Regression a Flagship Workflow

Elevate WAF regression from a generic WAF/stress command into a coherent workflow.

Suggested workflow:

```bash
eggsec scan http://127.0.0.1:8080 \
  --profile waf-regression \
  --scope examples/scope-localhost.toml \
  --baseline baselines/synvoid-waf.json \
  --json -o reports/waf-regression.json
```

The report should answer:

- Which payload families were tested?
- Which requests were blocked, challenged, tarpitted, allowed, or errored?
- Which behaviors changed relative to baseline?
- Which evasions regressed?
- Which detections improved?
- Which cases were skipped due to policy or budget?

Acceptance criteria:

- WAF regression emits a specific report section, not only generic findings.
- Baseline comparison distinguishes new bypasses from expected allowed traffic.
- Results include confidence and evidence redaction.
- There are fixture tests using a local/mock HTTP target.

## Phase 8: Add Synvoid-Specific Lab Documentation and Examples

Create documentation that explains why the aggressive tools exist and how they should be used safely for Synvoid hardening.

Suggested file:

```text
docs/lab/SYNVOID_DEFENSE_LAB.md
```

Suggested sections:

- Purpose: validating Synvoid WAF and distributed-network resilience.
- Threat classes modeled: request floods, malformed headers, protocol edge cases, TCP/UDP/ICMP behavior, WAF evasion attempts, rate-limit/tarpit behavior.
- Required environment: local/private lab, explicit scope file, conservative budgets.
- Example scope files.
- Example profiles.
- Example WAF regression run.
- Example distributed-system stress run.
- Expected outputs and how to interpret them.
- Safety constraints and non-goals.

Acceptance criteria:

- Docs never present these tools as public-target attack utilities.
- Docs include copy-pasteable localhost/private-lab examples.
- Docs link back to `docs/SAFETY.md`, `docs/BASELINES_AND_DIFFS.md`, and findings schema docs.

## Phase 9: Refine MCP Exposure

Preserve the two-profile MCP concept:

- ops-agent profile: full authorized toolkit, still policy-gated.
- coding-agent profile: bounded local/scope verification only.

For coding-agent MCP:

- Allow local target validation.
- Allow finding re-checks.
- Allow localhost/private port checks with strict limits.
- Allow limited endpoint/fuzz/WAF checks only when scope permits.
- Deny broad recon, load testing, stress testing, raw packets, proxy rotation, remote execution, and external targets by default.

For ops-agent MCP:

- Require explicit scope and policy for high-risk operations.
- Return policy decision records in every denial.
- Include budget summaries in every accepted high-risk operation.

Acceptance criteria:

- Tool registry labels each tool by operation risk and MCP profile availability.
- Coding-agent profile cannot invoke stress/raw/remote/proxy tools by name or alias.
- Ops-agent profile still cannot bypass policy or scope checks.
- MCP docs include denial examples and safe local verification examples.

## Phase 10: Improve Reports for Lab Defense Runs

Add report sections for lab-defense workflows.

Suggested sections:

- Policy summary
- Scope summary
- Feature flags used
- Risk tiers executed
- Budget summary
- Target resolution summary
- Baseline diff summary
- WAF behavior matrix
- Protocol edge-case summary
- Stress-test metrics
- Skipped/denied operation list
- Reproduction commands for safe local reruns

Acceptance criteria:

- JSON output includes machine-readable equivalents for every human report section.
- SARIF/JUnit outputs remain conservative and do not include noisy stress metrics as vulnerabilities unless they represent actionable findings.
- Markdown/HTML reports clearly separate vulnerabilities, observations, regressions, and load/stress metrics.

## Phase 11: Test Matrix

Add or expand tests around safety and lab workflows.

Minimum test categories:

- Scope matching: localhost, private CIDR, public domain, excluded target.
- Policy decisions: standard profile, defense-lab profile, hazardous-lab profile.
- Feature-gated command availability.
- Budget enforcement for load/stress/raw packet operations.
- MCP coding-agent denials.
- MCP ops-agent policy checks.
- WAF regression baseline diff using fixtures.
- Report serialization schema stability.
- CLI help text snapshots for high-risk commands.

Acceptance criteria:

- `cargo test --workspace --all-features` passes.
- Feature-minimal build still passes.
- Tests do not send external network traffic.
- Raw packet/stress tests use mocks or dry-run planning unless explicitly marked ignored.

## Phase 12: Documentation and Positioning Cleanup

Update docs so the repo's positioning is consistent.

Files to audit:

- `README.md`
- `docs/SAFETY.md`
- `docs/CAPABILITIES.md`
- `docs/AGENT.md`
- `docs/BASELINES_AND_DIFFS.md`
- `docs/FINDINGS_SCHEMA.md`
- Any architecture docs under `architecture/`
- CLI command docs if present

Key wording changes:

- Prefer `defense validation`, `WAF regression`, `authorized lab stress testing`, `distributed-system resilience testing`, and `scope-enforced assessment`.
- Avoid wording that sounds like generic offensive automation.
- Explain that stress/raw packet features exist to harden owned systems such as Synvoid and other authorized distributed/networked software.
- Keep the distinction between `standard-assessment`, `defense-lab`, and `hazardous-lab` consistent.

Acceptance criteria:

- README includes a concise explanation of why low-level/stress features exist.
- Safety docs explain the lab modes and policy flags.
- Capabilities docs classify each risky command by mode/risk.
- Agent docs explain policy/budget behavior for autonomous runs.

## Suggested Implementation Order

1. Audit existing risk/profile/policy types and avoid duplicate abstractions.
2. Implement or consolidate operation taxonomy.
3. Add policy decision records.
4. Add `policy explain` and `scope explain`.
5. Wire policy decisions into `plan`, command denials, MCP denials, and agent logs.
6. Add defense-lab presets and budget enforcement refinements.
7. Improve WAF regression output and baseline semantics.
8. Refine MCP profile exposure.
9. Add Synvoid defense-lab docs and examples.
10. Add tests and CLI help snapshots.
11. Clean up README/capabilities/safety/agent docs.

## Validation Commands

Run at minimum:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace
cargo build -p eggsec-cli
cargo build -p eggsec-cli --features stress-testing
cargo build -p eggsec-cli --features packet-inspection
cargo build -p eggsec-cli --features "rest-api ai-integration"
cargo build -p eggsec-cli --features full
```

Also run manual dry-run checks once implemented:

```bash
eggsec policy explain --target http://127.0.0.1:8080 --profile waf-regression --scope examples/scope-localhost.toml

eggsec plan --target http://127.0.0.1:8080 --profile waf-regression --scope examples/scope-localhost.toml --json

eggsec codegg-mcp --help

eggsec stress --help

eggsec packet --help
```

## Handoff Notes

The central principle is not to make Eggsec less capable. The goal is to make the high-risk capability surface obviously tied to authorized defensive validation. Preserve Synvoid and distributed-system stress testing use cases, but route them through explicit modes, explicit scope, finite budgets, structured policy decisions, and clear reports.

A smaller implementation model should prioritize consistency over breadth. It is better to implement policy decision records and wire them through a few high-value commands correctly than to partially relabel every command without improving enforcement.
