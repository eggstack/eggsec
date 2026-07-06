# NSE Milestone 4 Phase 02: Curated Upstream NSE Subset Validation

## Purpose

Add deterministic validation against a curated subset of upstream-style NSE scripts without requiring public internet, dynamic downloads, or full Nmap parity.

This phase improves confidence that Eggsec’s NSE runtime handles realistic script structure while preserving the project’s safety and truthfulness boundaries.

## Non-Goals

Do not vendor the entire Nmap scripts repository.

Do not run arbitrary upstream scripts in CI.

Do not contact public targets.

Do not claim drop-in Nmap replacement behavior.

Do not bypass Eggsec profile/capability enforcement for compatibility.

## Source Strategy

Use a curated subset of scripts that are either:

1. clean-room miniature scripts modeled after common upstream patterns; or
2. small upstream-compatible fixtures whose license and attribution are acceptable; or
3. metadata-only references to upstream behavior with local fixture equivalents.

If using upstream text, preserve license headers and document provenance. If uncertain, prefer clean-room fixtures that mimic structure and expected behavior without copying source.

## Workstream 1: Selection Criteria

Select scripts/features that exercise high-value compatibility:

- common `categories` arrays;
- `portrule` using `shortport` helpers;
- `hostrule` scripts;
- `action(host, port)` shape;
- `stdnse.format_output` style output;
- `http` helper usage;
- `sslcert` / TLS certificate summary path;
- `dns` helper path;
- protocol script that degrades gracefully;
- unsupported script with explicit capability denial.

### Acceptance Criteria

- Subset has 10–25 representative fixtures.
- Each fixture has a local deterministic expected result.
- Each fixture maps to a compatibility status: compatible, compatible-with-warnings, partial, unsupported, or failed.

## Workstream 2: Provenance Manifest

Add or extend manifest fields:

```toml
provenance = "clean-room" # or "upstream-derived"
upstream_reference = "scripts/http-title.nse pattern"
license_note = "No upstream source copied" # or SPDX/license note
local_fixture = true
public_network_required = false
```

### Acceptance Criteria

- Every upstream-subset fixture has provenance metadata.
- No ambiguous copied source exists without license/provenance notes.

## Workstream 3: Local Compatibility Harness

Extend the corpus harness to run upstream-subset fixtures with:

- configured local services;
- explicit execution profile;
- expected resolver diagnostics;
- expected capability events;
- expected rule reports;
- expected output/evidence summaries.

### Acceptance Criteria

- Upstream-subset tests can run offline.
- Failures clearly state fixture, expected compatibility, and observed mismatch.

## Workstream 4: Feature Gap Reporting

For each upstream-subset fixture, record gap classification:

- `supported`: no known gap;
- `approximate`: behavior differs but report indicates approximation;
- `capability_denied`: blocked by profile/capability policy;
- `missing_library`: registry/library gap;
- `context_gap`: host/port/service data insufficient;
- `unsupported_runtime`: unsupported Lua/NSE behavior.

### Acceptance Criteria

- Gaps are visible in `NseRunReport.compatibility` or corpus expectation output.
- Unsupported/approximate cases are intentional, not silent false positives.

## Workstream 5: Regression Guard

Add an architecture or corpus guard that prevents docs from overclaiming upstream support:

- no “full upstream NSE compatibility” claims;
- no “drop-in Nmap NSE replacement” claims;
- compatibility matrix must link to tested fixture IDs.

### Acceptance Criteria

- Compatibility claims remain tied to tested fixtures and registry metadata.

## Verification

Run:

```bash
cargo test -p eggsec-nse --features nse upstream_subset
cargo test -p eggsec-nse --features nse compatibility_corpus
bash scripts/check-architecture-guards.sh
```

## Final Acceptance Criteria

Phase 02 is complete when:

- Curated upstream-style subset exists with provenance metadata.
- Tests run offline and local-only.
- Each fixture records expected compatibility status and gaps.
- Docs accurately describe the subset as representative, not exhaustive.
