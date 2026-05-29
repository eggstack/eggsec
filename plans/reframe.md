# Slapper Reframing Implementation Plan

Audience: implementation by a smaller coding model such as MiMo 2.5.

Repository: `dbowm91/slapper`

Primary goal: reframe Slapper from a broad offensive/pentest toolkit into a Rust-native, scoped security assessment and defense-validation engine with strong support for Synvoid-style local regression testing, low-level protocol probing, WAF validation, controlled load-bearing tests, and curated Nmap NSE compatibility.

This plan is intentionally incremental. Do not attempt a large architectural rewrite in one pass. Prefer documentation and type/API seams first, then small code changes that make the new framing explicit.

---

## 0. Current State Summary

Slapper is currently a Rust workspace with at least these visible crates:

- `crates/slapper`
- `crates/slapper-nse`

The main crate already documents a broad async-first security testing toolkit with scanning, fuzzing, WAF, load testing, distributed scanning, TUI, output formats, agent/tool APIs, and optional NSE support.

The root README already contains some of the newer positioning:

> Slapper is not intended to be a Metasploit clone or a general arbitrary-code plugin host. Its core value is a maintainable Rust engine with policy-aware execution, structured outputs, and optional compatibility layers such as Nmap NSE support.

The architecture docs still contain older language such as:

> full assessment pipeline from reconnaissance through exploitation, with autonomous AI-driven agents...

This should be corrected because the new positioning is not “exploitation framework” and not “agentic exploitation.” It is defense-validation, scoped assessment, and controlled adversarial testing.

NSE is currently kept as a separate optional compatibility crate. This is good. Preserve it. The plan should not remove NSE.

---

## 1. Strategic Reframe

Adopt this project identity across README, architecture docs, CLI help text where practical, and agent/tool docs:

> Slapper is a Rust-native security assessment and defense-validation engine for scoped testing of live systems. It combines high-level application security checks, low-level protocol probing, controlled load-bearing tests, WAF evaluation, and optional Nmap NSE compatibility to help developers and security teams understand, reproduce, and harden real attack surfaces. Slapper is designed for authorized testing, local lab validation, and agent-readable regression workflows, not arbitrary exploitation or unscoped scanning.

Do not frame Slapper as:

- A Metasploit clone.
- A general arbitrary-code plugin host.
- A drop-in Nmap replacement.
- An autonomous exploitation platform.
- A tool for unscoped internet scanning.

Do frame Slapper as:

- A scoped assessment engine.
- A defense-validation harness.
- A Rust-native way to model and test attack surfaces.
- A local/regression testing companion for Synvoid and similar defensive systems.
- A structured, agent-readable security test runner.
- A compatibility host for curated NSE behavior.

---

## 2. Highest-Priority Documentation Changes

Make these first. They are low-risk and establish the intended architecture before code changes.

### 2.1 Update `README.md`

Search for language implying broad offensive exploitation, arbitrary automation, or generic pentest replacement. Replace with scoped assessment / defense-validation language.

Specific edits:

1. First paragraph should use the strategic reframe above.
2. “Why Slapper?” should emphasize:
   - scoped repeatable testing,
   - Rust-native primitives,
   - structured outputs,
   - WAF and defense validation,
   - local lab/regression workflows,
   - optional NSE compatibility.
3. Keep the feature table, but re-label dangerous capabilities carefully:
   - “Stress Testing” should be “Controlled stress / load-bearing validation.”
   - “WAF bypass” should be “WAF evaluation and evasion-resistance testing.”
   - “Automation” should be “Repeatable assessment and regression profiles.”
4. Add a short “Intended Use and Guardrails” section:
   - authorized testing only,
   - scope files are expected,
   - intrusive/stress profiles require explicit opt-in,
   - local lab mode is encouraged for defensive development.
5. Add a short “Relationship to Nmap/NSE” section:
   - Slapper borrows proven scanning concepts from Nmap,
   - NSE is an optional compatibility layer and protocol-testing knowledge source,
   - Slapper does not aim for full Nmap parity,
   - selected NSE ideas may be promoted into Rust-native probes over time.
6. Add a short “Defense-Lab Mode” section:
   - Slapper can run local repeatable profiles against Synvoid-like systems,
   - produces structured observations and baseline diffs,
   - useful for WAF regression testing, protocol-edge testing, and load-bearing validation.

Acceptance criteria:

- README no longer presents Slapper as an exploitation pipeline.
- README explicitly says authorized, scoped testing.
- README explicitly differentiates Slapper from Nmap and Metasploit.
- README explicitly describes local defense-validation use.

### 2.2 Update `architecture/overview.md`

Current text says Slapper provides a “full assessment pipeline from reconnaissance through exploitation.” Replace that with defense-validation language.

Suggested opening:

> Slapper is a Rust-native security assessment and defense-validation engine. It is designed for scoped, repeatable testing of live systems, including service discovery, protocol probing, WAF evaluation, application security checks, load-bearing validation, and agent-readable reporting. Its low-level networking capabilities are intended for controlled defensive validation, especially local lab testing against systems such as Synvoid.

Add a new section near the top:

## Architectural Principles

Include these principles:

1. Scope enforcement is a core invariant, not a CLI convenience.
2. Slapper-native Rust probes are the curated core.
3. NSE is a compatibility and knowledge layer, not the architectural center.
4. Low-level packet/protocol testing belongs in controlled defense-lab workflows.
5. Outputs should be structured and suitable for humans, CI, and agents.
6. Intrusive or stress behavior must be explicit and budgeted.
7. Profiles should compile into clear probe plans.

Update module descriptions if needed:

- `scanner/`: include low-level TCP/IP behavior and service fingerprinting, but specify controlled use.
- `waf/`: change “bypass” language to “evaluation and evasion-resistance testing.”
- `stress/`: describe as controlled stress/load-bearing validation requiring explicit feature gates.
- `agent/`: avoid “autonomous exploitation” language; use “agent-readable orchestration” or “scheduled assessment/alerting.”
- `nse_tool/`: describe as optional NSE compatibility adapter.

Acceptance criteria:

- No “reconnaissance through exploitation” phrasing remains.
- Low-level networking is described as controlled defense validation.
- NSE is described as optional compatibility/knowledge layer.
- Scope and risk boundaries are explicitly called out.

### 2.3 Update `architecture/nse_integration.md`

The current doc says Slapper includes “full integration” and “vast majority of existing NSE scripts.” Tone this down unless the codebase has tests proving it.

Preferred language:

> Slapper includes optional Nmap Scripting Engine compatibility through the `slapper-nse` crate. The goal is broad practical compatibility for useful script categories, not perfect Nmap runtime parity.

Add sections:

## NSE Compatibility Policy

Define support tiers:

- Tier 1: Safe discovery/version/default-style scripts that operate within Slapper scope and budgets.
- Tier 2: Service-specific scripts that require additional protocol libraries or credentials.
- Tier 3: Intrusive, brute-force, exploit-adjacent, or DoS-like scripts requiring explicit opt-in.
- Unsupported: Scripts requiring unrestricted filesystem/process access, uncontrolled network reachability, or behavior incompatible with Slapper guardrails.

## NSE as a Knowledge Source

Explain that NSE is also useful because its libraries and scripts encode mature protocol-testing concepts. Slapper may promote selected behaviors into Rust-native probes where repeatability, performance, and safety matter.

## Sandbox Defaults

Document the intended default:

- Agent/tool API paths should prefer sandboxed NSE.
- Filesystem, process execution, and arbitrary network access should be denied unless explicitly allowed.
- Scripts should have timeouts and execution budgets.
- Script category and capability manifests should eventually determine whether a script can run.

Acceptance criteria:

- Document no longer implies exact Nmap parity.
- NSE support is explicitly tiered.
- Sandbox-first operation is clearly recommended.
- NSE is positioned as compatibility plus a protocol-testing knowledge source.

---

## 3. Add New Architecture Document: `architecture/defense_lab.md`

Create a new document describing defense-lab mode.

Suggested title:

# Defense-Lab and Regression Validation Architecture

Content outline:

1. Purpose
   - local controlled testing against Synvoid and similar defensive systems,
   - repeatable adversarial traffic generation,
   - WAF and protocol behavior regression validation,
   - not a public-target stress or exploitation mode.

2. Core workflow
   - build/run Synvoid locally or in a controlled lab,
   - run a Slapper defense-lab profile,
   - collect Slapper observations,
   - optionally collect Synvoid logs/metrics,
   - compare against baseline,
   - convert regressions into test cases.

3. Probe categories
   - TCP/IP stack behavior,
   - malformed packet/protocol behavior,
   - TLS/client fingerprint variants,
   - HTTP ambiguity and smuggling checks,
   - WAF payload classification,
   - bot-like request patterns,
   - rate-limit/tarpit behavior,
   - load-bearing validation.

4. Safety model
   - localhost/private-lab defaults,
   - explicit target scope,
   - explicit rate/concurrency budgets,
   - stress features behind build/runtime gates,
   - no unscoped internet scanning.

5. Output model
   - run manifest,
   - probe suite,
   - observations,
   - findings,
   - latency histograms,
   - response class distributions,
   - baseline diff,
   - reproducibility metadata.

6. Future integration
   - Synvoid metrics/log import,
   - agent loop integration,
   - golden baseline fixtures,
   - CI-compatible regression profiles.

Acceptance criteria:

- New doc exists.
- It clearly separates defense-lab mode from general assessment mode.
- It explains why low-level OS/TCP/IP fingerprinting belongs in Slapper.
- It describes safety and budget boundaries.

---

## 4. Introduce Probe Intent and Risk Vocabulary

This is the first small code-oriented reframing step. Do not refactor every module at once. Start with types and docs.

Find the most appropriate location for shared enums. Candidate files:

- `crates/slapper/src/types.rs`
- or a new `crates/slapper/src/probe.rs`
- or an existing scanner/pipeline type module if clearly better.

Preferred: create `crates/slapper/src/probe.rs` and export it from `lib.rs`.

Add enums similar to:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProbeIntent {
    Discovery,
    Fingerprint,
    ServiceValidation,
    WafEvaluation,
    EvasionResistance,
    LoadBearing,
    Stress,
    MalformedProtocol,
    Regression,
    Compatibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProbeRisk {
    Passive,
    SafeActive,
    Intrusive,
    Credentialed,
    Stress,
    ExploitAdjacent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeMetadata {
    pub id: String,
    pub name: String,
    pub intent: ProbeIntent,
    pub risk: ProbeRisk,
    pub requires_explicit_scope: bool,
    pub requires_budget: bool,
    pub compatibility_source: Option<String>,
}
```

Names can be adjusted to fit existing style.

Use this initially in a minimal, non-invasive way:

- Add unit tests for serialization names.
- Add module docs explaining that this is the shared vocabulary for scanner, NSE, WAF, loadtest, and future defense-lab profiles.
- Do not force every existing scanner/fuzzer call to use it in the first pass.

Acceptance criteria:

- New type module compiles.
- Types serialize/deserialize.
- `lib.rs` exports the module.
- Unit tests cover at least a few enum JSON names.
- No major behavior change.

---

## 5. Add Defense-Lab Profile Metadata Without Implementing Full Engine Rewrite

Find pipeline/profile definitions. Candidate areas:

- `crates/slapper/src/pipeline/`
- config profile loading under `crates/slapper/src/config/`
- CLI profile definitions under `crates/slapper/src/cli/` or `commands/`

Goal: add or document future profile categories, not necessarily fully implement a new runner.

Add profile names if the current design supports it cleanly:

- `defense-lab`
- `synvoid-local`
- `waf-regression`
- `protocol-edge`
- `nse-safe`

If adding real profiles is too invasive, add TODO placeholders and docs only.

Profile semantics:

- `defense-lab`: local/private-scope controlled probe suite.
- `synvoid-local`: localhost/container/private lab defaults for Synvoid validation.
- `waf-regression`: WAF payload and evasion-resistance regression profile.
- `protocol-edge`: malformed protocol, TCP/TLS/HTTP edge behavior.
- `nse-safe`: sandboxed safe/default/version/discovery NSE scripts only.

Acceptance criteria:

- Either real profile definitions exist, or a clear architecture/TODO doc identifies where they will be added.
- No dangerous stress behavior is enabled by default.
- Any profile touching stress or packet behavior clearly requires explicit build feature and runtime scope.

---

## 6. Start a Canonical Run Manifest / Baseline-Diff Direction

The repo already has output/findings/diff modules. Do not rewrite them. Add a minimal document or type sketch that future work can build from.

Candidate files:

- `docs/BASELINES_AND_DIFFS.md`
- `architecture/output.md`
- `crates/slapper/src/diff/`
- `crates/slapper/src/output/`

Add an architecture section describing a `DefenseLabRunManifest` or generic `RunManifest` with fields:

- `schema_version`
- `run_id`
- `started_at`
- `ended_at`
- `slapper_version`
- `target_scope`
- `profile`
- `probe_intents`
- `risk_budget`
- `feature_flags`
- `observations`
- `findings`
- `artifacts`
- `baseline_id`
- `diff_summary`

If implementing code, keep it small and serde-only. Do not integrate into every output path in this pass.

Acceptance criteria:

- There is a clear schema direction for regression-oriented runs.
- Baseline/diff docs mention defense-lab regression validation.
- Existing output behavior remains compatible.

---

## 7. Guardrail Requirements

Where feasible, add docs and TODOs near code paths for guardrails. Do not break current CLI behavior unless tests can be updated cleanly.

Guardrail model:

1. Scope is required for intrusive/stress/defense-lab profiles.
2. Rate/concurrency/duration budgets are required for load-bearing and stress tests.
3. NSE sandbox should be the preferred path for agent/tool execution.
4. Intrusive NSE categories require explicit opt-in.
5. Packet/raw-socket tests require explicit feature gates and runtime confirmation.
6. Agent-facing tools should expose intent/risk metadata before running probes.
7. Reports should include enough provenance to reproduce a result.

Files likely relevant:

- `crates/slapper/src/config/`
- `crates/slapper/src/commands/`
- `crates/slapper/src/tool/`
- `crates/slapper/src/nse_tool.rs`
- `crates/slapper-nse/src/context.rs`
- `crates/slapper-nse/src/executor.rs`
- `crates/slapper-nse/src/public_api.rs`

Acceptance criteria:

- At minimum, docs state the guardrail model.
- If code changes are made, they should be small and covered by tests.
- Do not silently enable dangerous behavior in default builds.

---

## 8. Nmap/NSE Relationship Cleanup

Search the repository for these terms:

- `metasploit`
- `exploit`
- `exploitation`
- `autonomous`
- `full integration`
- `vast majority`
- `bypass`
- `DoS`
- `stress`
- `plugin`
- `Nmap`
- `NSE`

Review matches and adjust wording where it overstates intent.

Preferred substitutions:

- “exploitation” → “assessment,” “validation,” “controlled adversarial testing,” or “exploit-adjacent checks” depending on context.
- “bypass” → “evasion-resistance testing” unless the code actually means a bypass payload.
- “autonomous agent” → “agent-readable orchestration” or “scheduled assessment agent” unless it truly performs autonomous loops.
- “full NSE integration” → “optional NSE compatibility.”
- “vast majority of NSE scripts” → “broad practical compatibility for useful script categories” unless backed by compatibility tests.

Acceptance criteria:

- Product language is consistent across README, architecture docs, and NSE docs.
- Dangerous capability names are not hidden, but are described in controlled/authorized terms.
- NSE is presented as compatibility plus knowledge source, not as a promise of perfect Nmap parity.

---

## 9. Tests and Validation Commands

After each small set of changes, run:

```bash
cargo fmt
cargo check --workspace
cargo test --workspace
```

If full workspace tests are too slow or feature-related failures exist, at minimum run:

```bash
cargo check -p slapper
cargo check -p slapper-nse --no-default-features
cargo test -p slapper probe --no-default-features
```

If new docs only:

```bash
grep -R "reconnaissance through exploitation" -n README.md architecture docs crates || true
grep -R "Metasploit clone" -n README.md architecture docs crates || true
```

Optional feature checks if available:

```bash
cargo check -p slapper --features nse
cargo check -p slapper --features nse,nse-sandbox
cargo check -p slapper --features stress-testing,packet-inspection
```

Do not claim success unless commands actually pass.

---

## 10. Suggested Implementation Order

### Pass 1: Documentation Reframe

1. Update `README.md`.
2. Update `architecture/overview.md`.
3. Update `architecture/nse_integration.md`.
4. Add `architecture/defense_lab.md`.
5. Search and clean obvious stale wording.

Commit message:

```text
docs: reframe slapper as defense-validation engine
```

### Pass 2: Probe Vocabulary

1. Add `crates/slapper/src/probe.rs`.
2. Export `pub mod probe;` from `crates/slapper/src/lib.rs`.
3. Add serialization tests.
4. Mention `ProbeIntent` / `ProbeRisk` in architecture docs.

Commit message:

```text
feat(core): add probe intent and risk metadata types
```

### Pass 3: Profile Direction

1. Inspect `pipeline/`, `config/`, and `cli/` profile definitions.
2. Add docs or lightweight profile placeholders for:
   - `defense-lab`
   - `synvoid-local`
   - `waf-regression`
   - `protocol-edge`
   - `nse-safe`
3. Do not enable stress behavior by default.

Commit message:

```text
docs(pipeline): define defense-lab profile direction
```

or, if code is added:

```text
feat(pipeline): add defense-lab profile metadata
```

### Pass 4: Baseline/Run Manifest Direction

1. Update baseline/diff docs.
2. Add a minimal serde struct only if there is an obvious location and low coupling.
3. Avoid rewriting output/report generation in this pass.

Commit message:

```text
docs(output): define defense-lab run manifest direction
```

---

## 11. Non-Goals for This Handoff

Do not attempt these in this pass:

- Do not rewrite the scanner engine.
- Do not rewrite NSE execution.
- Do not remove NSE.
- Do not add arbitrary plugin runtime support.
- Do not attempt full Nmap parity.
- Do not add new offensive exploit modules.
- Do not make stress/packet functionality default.
- Do not perform large crate-splitting yet.
- Do not change CLI behavior in ways that break existing tests unless necessary and documented.
- Do not create a complex agent loop in this pass.

---

## 12. Final Acceptance Criteria

The handoff is successful when:

1. Slapper’s docs consistently present it as a scoped security assessment and defense-validation engine.
2. The README and architecture docs explicitly support Synvoid/local defense-lab regression workflows.
3. Nmap/NSE are framed as optional compatibility and a mature protocol-testing knowledge source.
4. Low-level OS/TCP/IP/protocol fingerprinting is justified as controlled defensive validation, not general Nmap cloning.
5. Guardrails are documented: scope, risk tiers, budgets, sandboxed NSE, feature-gated stress/packet behavior.
6. A new `ProbeIntent` / `ProbeRisk` vocabulary exists or is clearly specified for immediate implementation.
7. No dangerous behavior is enabled by default.
8. The workspace still formats and checks/tests at least at the same level as before the changes.

