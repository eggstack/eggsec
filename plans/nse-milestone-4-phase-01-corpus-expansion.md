# NSE Milestone 4 Phase 01: Local Compatibility Corpus Expansion

## Purpose

Expand the NSE compatibility corpus from a small representative set into a broader local-only regression suite that exercises common script patterns, library usage, rule semantics, capability events, and report truthfulness.

This phase provides the test foundation for the rest of Milestone 4.

## Non-Goals

Do not fetch scripts dynamically during tests.

Do not require public internet.

Do not claim full Nmap parity.

Do not add unsafe exploit/intrusive scripts to default CI.

Do not reopen Milestone 1 through 3 contracts.

## Corpus Layout

Recommended layout:

```text
crates/eggsec-nse/tests/fixtures/nse_corpus/
  manifest.toml
  scripts/
    discovery/
    version/
    default/
    auth/
    protocol/
    partial/
    unsupported/
  modules/
  expected/
  README.md
```

Each fixture should include metadata:

```toml
[id]
name = "http-title-basic"
category = "discovery"
profile = "agent_safe"
expected_status = "compatible"
expected_fidelity = "full"
expected_libraries = ["stdnse", "http"]
expected_rules = ["portrule"]
expected_capability_events = []
notes = "Local-only HTTP fixture. No public network."
```

## Workstream 1: Corpus Taxonomy

Define corpus categories:

- `discovery`: safe host/service discovery patterns.
- `version`: service/version detection patterns.
- `default`: scripts representative of Nmap default category behavior.
- `protocol`: protocol libraries with local fixtures.
- `auth`: credential-shape tests that do not perform real brute force.
- `partial`: supported with approximations or warnings.
- `unsupported`: expected denials, capability blocks, missing modules, or unsupported rule returns.
- `regression`: loader/report/capability regressions from prior milestones.

### Acceptance Criteria

- `manifest.toml` or equivalent exists.
- Every fixture has category, profile, expected status/fidelity, expected libraries, expected rule behavior, and expected capability behavior.

## Workstream 2: Local Fixtures

Add local-only fixtures for common patterns:

- script with no `require()`;
- script requiring `stdnse`;
- script requiring `http` but using a local mock service;
- script requiring `dns` with local/mock deny path;
- script with `portrule` true/false/error/non-boolean;
- hostrule/prerule/postrule coverage;
- script with capability-denied filesystem read;
- script with process-denied path;
- script with compression bounded path;
- script with approximate compatibility warning.

### Acceptance Criteria

- Fixtures do not contact public internet.
- Fixtures are small and committed to the repo.
- Expected outputs are stable.

## Workstream 3: Corpus Harness

Build or extend a harness such as:

```text
crates/eggsec-nse/tests/compatibility_corpus_tests.rs
```

Required features:

- load manifest;
- run fixtures under specified profile;
- build `NseRunReport`;
- assert compatibility status/fidelity;
- assert required libraries;
- assert rule reports;
- assert capability events;
- assert errors/warnings for unsupported cases.

### Acceptance Criteria

- Adding a fixture requires no custom test code unless the fixture needs a special local service.
- Harness failure messages identify fixture ID and failed expectation.

## Workstream 4: Local Service Fixtures

For protocol tests, add local in-process fixtures where practical:

- minimal HTTP server for title/header/body tests;
- local TCP echo server;
- local UDP echo server;
- local TLS fixture using generated/self-signed test material if already supported;
- fake DNS response fixture if resolver abstraction supports it, otherwise use denial-path tests only.

### Acceptance Criteria

- Tests bind only localhost.
- Ports are assigned dynamically.
- No test depends on timing beyond bounded local waits.

## Workstream 5: Report Snapshot Discipline

Avoid brittle full JSON snapshots. Assert semantic fields instead:

- `compatibility.status`;
- `compatibility.fidelity`;
- library names and `loaded` state;
- rule kind/evaluated/matched/error/unsupported;
- capability event kind/allowed/reason;
- resolver diagnostics count/kinds;
- output presence and key substrings.

### Acceptance Criteria

- Tests are stable across harmless formatting changes.
- Report truthfulness regressions fail.

## Verification

Run:

```bash
cargo test -p eggsec-nse --features nse compatibility_corpus
cargo test -p eggsec-nse --features nse,sandbox compatibility_corpus
cargo test -p eggsec-nse --features nse report
bash scripts/check-architecture-guards.sh
```

## Final Acceptance Criteria

Phase 01 is complete when:

- Expanded corpus fixtures and manifest exist.
- Harness runs local-only tests with semantic assertions.
- Corpus covers success, partial, unsupported, denied, errored, and approximate cases.
- The corpus becomes the base validation surface for later Milestone 4 phases.
