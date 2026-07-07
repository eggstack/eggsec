# NSE Expansion Phase 04: Upstream-Style Corpus Growth

## Purpose

Broaden representative NSE compatibility coverage with curated, local-only, upstream-style scripts.

This phase should improve confidence in common NSE script shapes without claiming full Nmap NSE parity or importing uncontrolled upstream behavior.

## Non-Goals

Do not download or execute arbitrary upstream scripts in tests.

Do not add public-internet-dependent fixtures.

Do not make unsupported behavior look compatible.

Do not weaken runtime assertion strictness.

Do not expand corpus size until harness performance remains acceptable.

## Workstream 1: Select Corpus Categories

Prioritize script shapes rather than library breadth.

Suggested categories:

- discovery scripts;
- version-detection shape;
- HTTP metadata scripts;
- DNS query shape;
- TLS certificate shape after Phase 03;
- credential-helper shape without brute forcing;
- reporting/vulns output shape;
- unsupported/partial regression fixtures.

### Acceptance Criteria

- Each category has a reason and expected compatibility level.
- Each category remains local-only.

## Workstream 2: Corpus Manifest Extensions

If needed, add metadata fields:

- `upstream_style = true`;
- `inspired_by = "script-family-name"` without copying upstream code if clean-room;
- `verification_mode`;
- `local_service`;
- `expected_profile_set`;
- `expected_evidence_kinds`;
- `expected_denial_kinds`.

### Acceptance Criteria

- Metadata captures provenance and verification status.
- Clean-room/local-only status is visible.

## Workstream 3: Add Fixtures Incrementally

Add fixtures in small batches.

For each fixture:

- script file;
- manifest entry;
- expected libraries;
- expected rules;
- expected status/fidelity;
- expected capability events/evidence if applicable;
- runtime test coverage via existing harness or local protocol tests.

### Acceptance Criteria

- No fixture is metadata-only unless explicitly labeled.
- Runtime tests assert observed fields.

## Workstream 4: Preserve Harness Performance

Before and after adding fixtures, record runtime corpus duration.

If duration grows materially:

- add fixture category filters;
- split slow local-protocol tests if needed;
- keep default verification reasonable.

### Acceptance Criteria

- Corpus remains practical to run during development.
- Slow fixtures are identified.

## Workstream 5: Update Compatibility Matrix

For each added fixture:

- add fixture ID;
- category;
- fidelity;
- profile coverage;
- expected libraries/rules/events/evidence;
- known gaps.

### Acceptance Criteria

- Matrix claims remain tied to fixture IDs.
- No broad “Nmap parity” claim is introduced.

## Workstream 6: Guards

Add or maintain guards for:

- no public network in fixture scripts/manifest;
- no unreviewed upstream import claims;
- local service fixtures declare metadata;
- runtime harness executes or intentionally skips local-service fixtures.

## Verification

Run:

```bash
cargo test -p eggsec-nse --features nse --test compatibility_corpus_tests
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=1
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=4
cargo test -p eggsec-nse --features nse --test local_protocol_tests
cargo test -p eggsec-nse --features nse
bash scripts/check-architecture-guards.sh
cargo fmt --all --check
```

## Final Acceptance Criteria

Phase 04 is complete when:

- new upstream-style fixtures are local-only and provenance-labeled;
- runtime tests verify observed behavior;
- compatibility matrix maps claims to fixture IDs;
- harness runtime remains manageable;
- docs avoid full parity claims.
