# NSE Milestone 4 Phase 03: Host, Port, and Service Context Fidelity

## Purpose

Improve the fidelity of NSE rule evaluation and script action execution by passing richer host, port, service, protocol, and scan-context data into Lua scripts.

Earlier milestones focused on safety and report truthfulness. This phase improves compatibility quality by making the inputs to `hostrule`, `portrule`, and `action(host, port)` closer to what practical NSE scripts expect.

## Non-Goals

Do not implement full Nmap scan-engine state.

Do not fake exact service data when Eggsec does not know it.

Do not remove truthfulness markers for approximate context.

Do not require live network access in tests.

## Target State

Scripts should receive structured host/port context with fields such as:

- host IP / hostname / target label;
- port number;
- protocol (`tcp`, `udp`);
- service name where known;
- service product/version where known;
- state (`open`, `closed`, `filtered`, `unknown`);
- transport metadata;
- source of service data (`scan`, `fixture`, `synthetic`, `unknown`).

Rule reports should summarize which context fields were real, synthetic, missing, or approximate.

## Workstream 1: Define Context Types

Add or refine types such as:

```rust
pub struct NseHostContext {
    pub ip: String,
    pub hostname: Option<String>,
    pub target_label: String,
    pub source: NseContextSource,
}

pub struct NsePortContext {
    pub port: u16,
    pub protocol: String,
    pub state: String,
    pub service: Option<NseServiceContext>,
    pub source: NseContextSource,
}

pub struct NseServiceContext {
    pub name: Option<String>,
    pub product: Option<String>,
    pub version: Option<String>,
    pub tunnel: Option<String>,
    pub confidence: Option<f32>,
}
```

### Acceptance Criteria

- Context types are serializable or report-summarizable.
- Unknown/synthetic status is explicit.

## Workstream 2: Lua Table Construction

Centralize construction of Lua `host` and `port` tables.

Required behavior:

- avoid duplicating host/port table shape across rule/action code;
- include only known values or mark synthetic/unknown clearly;
- preserve compatibility with existing scripts expecting `host.ip`, `host.name`, `port.number`, `port.protocol`, `port.service`, `port.state`;
- expose optional metadata under an Eggsec-specific field if useful, such as `port.eggsec_context_source`.

### Acceptance Criteria

- `portrule`, `hostrule`, and action execution use the same table builder.
- Tests cover table shape for known and unknown service context.

## Workstream 3: Rule Report Inputs Summary

Extend `NseRuleEvaluationReport` or add an adjacent summary to capture:

- rule kind;
- host context source;
- port context source;
- service context availability;
- exactness/fidelity reason;
- missing fields that caused approximation.

### Acceptance Criteria

- Reports explain why a rule result is exact or approximate.
- No rule silently appears exact when host/port context is synthetic.

## Workstream 4: Integration With Eggsec Scan Results

Where available, map Eggsec scan result data into `NseHostContext` / `NsePortContext`:

- target IP/host;
- discovered port;
- protocol;
- open/closed state;
- service name;
- banner/version data;
- TLS/certificate data if available.

Keep this mapping defensive: missing fields should produce `Unknown`/`Synthetic` context, not fabricated certainty.

### Acceptance Criteria

- CLI/TUI/manual paths can pass scan-derived context where available.
- Absence of scan context still works with explicit approximation markers.

## Workstream 5: Corpus Tests

Add fixtures that validate:

- `shortport.port_or_service` style behavior;
- port-number-only rules;
- service-name rules;
- protocol-specific rules;
- hostrule using host IP/name;
- approximate behavior when service context is missing;
- exact behavior when service context is known.

### Acceptance Criteria

- Rule fidelity changes are tested through reports.
- Local-only tests cover both exact and approximate context.

## Workstream 6: Documentation

Document:

- supported host/port/service fields;
- how context source affects fidelity;
- what remains different from Nmap;
- how users can provide or inspect context.

## Verification

Run:

```bash
cargo test -p eggsec-nse --features nse rule
cargo test -p eggsec-nse --features nse compatibility_corpus
cargo test -p eggsec-nse --features nse report
bash scripts/check-architecture-guards.sh
```

## Final Acceptance Criteria

Phase 03 is complete when:

- Host/port/service context builders are centralized.
- Rule/action execution uses richer context where available.
- Rule reports explain context fidelity.
- Corpus tests cover exact and approximate context cases.
- Docs state gaps without claiming full Nmap scan-engine equivalence.
