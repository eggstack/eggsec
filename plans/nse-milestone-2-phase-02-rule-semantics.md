# NSE Milestone 2 Phase 02: Rule Semantics and Truthfulness

## Purpose

Make NSE rule evaluation explicit and truthful. Eggsec should report how `portrule`, `hostrule`, `prerule`, and `postrule` were handled, whether the behavior was exact or approximate, and what inputs were missing or synthesized.

This phase builds on the Phase 01 library registry and the Milestone 1 loader/profile closure. It should not rewrite loader policy.

## Background

Nmap NSE rules are evaluated in the context of Nmap's scan engine, host/port tables, service probes, timing model, and script categories. Eggsec can support practical compatibility for many scripts, but not every rule can be exact without full Nmap parity.

The production-grade answer is not to pretend exactness. The runtime should expose rule evaluation metadata and compatibility status.

## Non-Goals

Do not implement full Nmap scan-engine parity.

Do not support every NSE rule shape in this phase.

Do not silently skip failed rules.

Do not make unsupported rules appear successful.

Do not change script/module loading policy.

## Target State

By the end of this phase:

- Rule evaluation has structured metadata.
- Each script run records which rule functions existed.
- The runtime reports whether each rule was evaluated, skipped, unsupported, errored, exact, or approximate.
- Approximate inputs are explicitly marked.
- Rule semantics are consumable by Phase 03 structured reports.

## Proposed Data Model

Add a rule-semantics module, likely:

```text
crates/eggsec-nse/src/rule_semantics.rs
```

Suggested types:

```rust
pub struct NseRuleEvaluationReport {
    pub script_name: String,
    pub rule_kind: NseRuleKind,
    pub status: NseRuleStatus,
    pub fidelity: NseRuleFidelity,
    pub result: Option<bool>,
    pub error: Option<String>,
    pub inputs: NseRuleInputsSummary,
    pub warnings: Vec<String>,
}

pub enum NseRuleKind {
    PortRule,
    HostRule,
    PreRule,
    PostRule,
}

pub enum NseRuleStatus {
    PresentEvaluated,
    PresentSkipped,
    PresentUnsupported,
    PresentErrored,
    NotPresent,
}

pub enum NseRuleFidelity {
    Exact,
    Practical,
    Approximate,
    SyntheticInput,
    Unsupported,
    Unknown,
}

pub struct NseRuleInputsSummary {
    pub target_present: bool,
    pub port_present: bool,
    pub service_present: bool,
    pub protocol_present: bool,
    pub host_table_synthetic: bool,
    pub port_table_synthetic: bool,
}
```

Keep the model report-oriented. It should describe behavior, not dictate every future engine detail.

## Workstream 1: Inventory Current Rule Behavior

### Steps

1. Inspect existing APIs:
   - `run_script`
   - `check_portrule`
   - `check_hostrule`
   - `get_prerule_result`
   - `get_postrule_result`
   - any category filtering helpers.
2. Document current behavior for:
   - absent rule function;
   - rule returns boolean;
   - rule returns non-boolean;
   - rule throws Lua error;
   - missing host/port/service context;
   - fallback category behavior.
3. Add a brief implementation note in `architecture/nse_integration.md` or inline module docs.

### Acceptance Criteria

- Current behavior is written down before changing it.
- Approximate behavior is identified explicitly.

## Workstream 2: Add Rule Evaluation API

### Steps

1. Add `rule_semantics.rs` with report types.
2. Add methods to `NseExecutor` and/or `ExecutorCore` such as:

```rust
pub fn evaluate_portrule_report(&self, port: &PortContext) -> NseRuleEvaluationReport;
pub fn evaluate_hostrule_report(&self, host: &HostContext) -> NseRuleEvaluationReport;
pub fn evaluate_prerule_report(&self) -> NseRuleEvaluationReport;
pub fn evaluate_postrule_report(&self) -> NseRuleEvaluationReport;
```

If existing context types are not suitable, create a minimal `NseRuleContext` that can be populated by current callers.

3. Preserve existing boolean convenience methods for compatibility, but route them through the new report API where feasible.
4. Ensure errors are captured in the report and not collapsed into `false` unless the legacy boolean API requires it.

### Acceptance Criteria

- New report APIs compile and are tested.
- Existing public methods remain source-compatible where practical.
- Legacy methods do not hide the richer report path from structured output.

## Workstream 3: Classify Fidelity

### Required Classifications

Use conservative defaults:

- `Exact`: only when inputs and behavior match the implemented contract closely.
- `Practical`: expected useful behavior but not strict Nmap parity.
- `Approximate`: missing/simplified context or known semantic mismatch.
- `SyntheticInput`: rule evaluated against generated host/port tables.
- `Unsupported`: rule shape exists but cannot be safely or meaningfully evaluated.
- `Unknown`: temporary classification while inventory is incomplete.

### Steps

1. For current `portrule` and `hostrule` paths, determine what context is synthetic.
2. Mark boolean returns with fidelity based on context completeness.
3. Mark non-boolean returns as `PresentErrored` or `PresentUnsupported` depending on Lua/NSE expectations.
4. Add warnings for fallback category behavior.

### Acceptance Criteria

- Rule reports never imply exactness for synthetic or incomplete inputs.
- Approximation warnings are stable enough for JSON output and docs.

## Workstream 4: Tests

### Required Test Scripts

Create inline Lua test scripts for:

- no rule functions;
- `portrule` returning true;
- `portrule` returning false;
- `portrule` throwing error;
- `portrule` returning non-boolean;
- `hostrule` returning true;
- `prerule` returning string/table;
- `postrule` returning value;
- script with category fallback only.

### Assertions

For each case, assert:

- `rule_kind`;
- `status`;
- `fidelity`;
- result/error content;
- warnings when context is synthetic or incomplete.

### Acceptance Criteria

- Rule tests fail if errors are silently collapsed into `false` in the report path.
- Rule tests distinguish `NotPresent`, `PresentErrored`, and `PresentUnsupported`.

## Workstream 5: Docs

### Steps

1. Add `Rule Semantics` section to `architecture/nse_integration.md`.
2. Document supported rule kinds and fidelity classes.
3. State that Eggsec does not claim exact Nmap rule behavior unless reported as `Exact`.
4. Add examples of report statuses.

### Acceptance Criteria

- Docs are precise about approximation.
- Users and agents can interpret rule statuses without reading code.

## Verification

Run:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
bash scripts/check-architecture-guards.sh
```

## Final Acceptance Criteria

Phase 02 is complete when:

- Rule evaluation has structured metadata.
- Existing boolean APIs are backed by or aligned with report APIs.
- Approximate and unsupported semantics are visible.
- Tests cover absent, true, false, errored, unsupported, and synthetic-input cases.
- Docs no longer leave rule behavior implicit.
