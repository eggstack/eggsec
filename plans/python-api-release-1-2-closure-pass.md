# Eggsec Python API Release 1/2 Closure Pass

## Handoff objective

This pass closes the remaining evidence and implementation gaps identified after the Release 1/2 corrective integration work.

The repository now has strong validation infrastructure and a useful graduation audit, but the current evidence remains uneven:

- 14 always-compiled operations are fully graduated;
- 8 feature-gated operations remain conditional because enabled-path execution is not comprehensively tested;
- two test failures remain in the reported validation run;
- 89 skipped tests are not yet categorized into expected profile skips versus missing validation;
- live packet capture and privileged active probes remain incompletely validated;
- HTTP/WebSocket transition behavior is not yet fully exercised against live fixtures;
- registry/dispatcher architecture has been documented but not materially refactored;
- the validation matrix is not visibly published as a durable CI artifact.

This closure pass must produce a defensible end-state for Releases 1 and 2 before Release 3 implementation begins.

## Primary outcomes

At completion:

- all 22 stable operations have an explicit graduation state backed by enabled-path evidence;
- the eight feature-gated operations either pass their feature-enabled profiles or are reclassified accurately;
- the release validation suite has zero unexplained failures;
- every skip is categorized and justified;
- the 14-profile validation matrix runs in CI and publishes machine-readable artifacts;
- async resource APIs are validated against real loopback I/O;
- DNS, redirect, proxy, reconnect, and WebSocket upgrade transitions are scope-checked end-to-end;
- live packet capture and privileged probe support have explicit supported-platform evidence or remain clearly experimental;
- registry/executor ownership is tightened enough to prevent further monolithic growth;
- Release 1 is conclusively closed and Release 2 has a precise provisional boundary.

## Non-goals

This pass does not add:

- new assessment domains;
- full NSE runtime programmability;
- interception proxy lifecycle completion;
- stateful database sessions;
- mobile dynamic sessions;
- browser session lifecycle;
- daemon parity;
- public 1.0 stabilization.

Those belong to Releases 3–5.

## Workstream 1 — Resolve the remaining test failures

### Goal

Eliminate all unexplained failures from the canonical validation run.

### Required work

- Reproduce the two reported failures under the exact profile and environment in which they occur.
- Determine whether each failure is:
  - a real implementation defect;
  - an incorrect test expectation;
  - a platform-specific unsupported condition;
  - a fixture-ordering problem;
  - a race;
  - a stale test.
- Fix real defects in production code.
- Correct stale or invalid tests without weakening intended behavior.
- Use `xfail(strict=True)` only for a documented, externally blocked issue with a linked plan entry.
- Do not leave failures described only as “pre-existing.”

### Acceptance criteria

- The canonical default validation profile has zero failures.
- Any expected failure is strict, documented, and tied to a known blocker.
- Release scripts treat unexpected failures as fatal.

## Workstream 2 — Categorize and reduce skipped tests

### Goal

Make every skipped test explainable by profile, feature, privilege, platform, or external dependency.

### Required work

Produce a skip report grouped by:

- missing Cargo feature;
- unsupported operating system;
- missing system package;
- missing privilege;
- missing external fixture;
- intentionally offline CI;
- test not applicable to current wheel profile.

Add stable marker names such as:

- `requires_feature_git_secrets`
- `requires_feature_sbom`
- `requires_feature_db_pentest`
- `requires_feature_nse`
- `requires_feature_container`
- `requires_feature_mobile`
- `requires_packet_capture_privilege`
- `requires_raw_socket_privilege`
- `requires_linux`
- `requires_macos`

Update `validate_python_release_1_2.sh` so each profile reports:

- collected;
- passed;
- failed;
- skipped by reason;
- xfailed;
- duration.

### Acceptance criteria

- No anonymous or generic skip remains in release-critical tests.
- Skip counts are stable and profile-specific.
- The CI artifact records skip reasons.

## Workstream 3 — Feature-enabled validation for all eight conditional operations

### Goal

Replace feature-unavailable-only evidence with real enabled-path execution evidence.

### Required profiles

#### Git secrets

Build with `git-secrets` and validate:

- direct function;
- sync engine;
- async engine;
- request validation;
- payload type;
- serialization;
- audit;
- cancellation;
- timeout;
- secret sentinel exclusion;
- installed wheel.

Use a temporary Git repository containing deterministic benign sentinel patterns.

#### SBOM

Build with `sbom` and validate:

- direct/sync/async execution;
- known lockfile and manifest fixtures;
- multiple output formats where supported;
- artifact generation;
- payload schema;
- installed wheel.

#### Database probe

Build with `db-pentest` and provide deterministic service fixtures. Prefer containerized ephemeral services for supported backends.

Validate:

- successful connection;
- authentication failure;
- read-only default behavior;
- timeout;
- cancellation;
- result-size bounds;
- secret redaction;
- engine/direct equivalence;
- installed wheel.

#### NSE

Build with `nse` and validate:

- script discovery;
- deterministic local script execution;
- sandbox limits;
- cancellation;
- timeout;
- structured output;
- policy classification;
- installed wheel.

#### Container

Build with `container` and validate both Docker image and Kubernetes manifest operations using deterministic local artifacts.

Do not require privileged Docker-in-Docker for tests that can run against static OCI archives or manifests.

#### Mobile static

Build with `mobile` and validate APK and IPA analysis against small, redistributable synthetic fixtures.

Validate malformed archives, missing metadata, cancellation, serialization, and artifact references.

### Graduation decision

For each feature-gated operation:

- mark `PASS` only when enabled-path sync and async execution, serialization, policy, timeout, cancellation, and installed-wheel tests pass;
- otherwise classify as `provisional` or `stable-conditional` with explicit blockers.

### Acceptance criteria

- Graduation audit no longer relies on default-build feature-unavailable behavior as primary evidence.
- The capability manifest records the exact passing profile and commit.

## Workstream 4 — CI publication of the canonical validation matrix

### Goal

Turn local validation scripts into durable repository evidence.

### Required CI jobs

- default Linux;
- default macOS;
- full-no-system;
- websocket;
- git-secrets;
- sbom;
- db-pentest;
- nse;
- container;
- mobile;
- packet parser only;
- privileged capture, where runner support exists;
- privileged active probe, where runner support exists;
- installed-wheel smoke and typing.

### Required artifacts

Publish:

- `python-validation-report.json`;
- per-profile JUnit XML;
- skip report;
- binary-size report;
- import-time report;
- capability manifest snapshot;
- architecture guard report;
- type/stub parity report.

### Acceptance criteria

- The latest `main` commit has visible CI status checks.
- Required release profiles are branch-protection candidates.
- Validation artifacts can be downloaded and audited independently.

## Workstream 5 — Real async I/O lifecycle validation

### Goal

Validate that async APIs are not only awaitable but behave correctly during real blocked I/O and cleanup.

### Targets

- `AsyncTcpSession`;
- `AsyncUdpSocket`;
- `AsyncHttpClient`;
- `AsyncWebSocketSession`;
- `AsyncCaptureSession`, where supported;
- `AsyncPipeline`.

### Required tests

- cancellation during connect;
- cancellation during read;
- cancellation during write or body upload;
- stalled HTTP response body;
- WebSocket receive cancellation;
- UDP receive cancellation;
- timeout during TLS handshake;
- repeated open/close cycles;
- concurrent operations on one client where supported;
- close while operation is in flight;
- cleanup after callback exception;
- no task or socket leak after failure.

Use real loopback services rather than only mocked futures.

### Acceptance criteria

- Async context managers use `__aenter__`/`__aexit__` where appropriate.
- No resource remains open after cancellation or timeout.
- Sync façades do not create nested-runtime failures.

## Workstream 6 — Scope enforcement across live transitions

### Goal

Prove that authorization is re-evaluated at every network transition.

### Required fixtures

- hostname resolving to allowed IP;
- hostname resolving to denied IP;
- hostname changing addresses between attempts;
- mixed IPv4/IPv6 resolution;
- allowed URL redirecting to denied authority;
- redirect loop;
- allowed proxy carrying request to denied target;
- denied proxy with allowed target;
- WebSocket HTTP upgrade to changed authority;
- reconnect to a different resolved address;
- DNS CNAME chain crossing policy boundaries.

### Policy requirements

Define and test evaluation of:

- original hostname;
- canonical name;
- every resolved address;
- explicit port;
- redirect authority;
- proxy endpoint;
- final target;
- reconnect destination;
- upgrade destination.

### Acceptance criteria

- No redirect, proxy, reconnect, or DNS transition bypasses scope.
- Denial errors identify the rejected transition without leaking secrets.
- Audit events preserve enough metadata to explain the decision.

## Workstream 7 — HTTP correctness closure

### Goal

Validate the security-oriented HTTP client against protocol edge cases.

### Required fixtures

- duplicate response headers;
- duplicate request headers;
- chunked response bodies;
- gzip/brotli or supported compression;
- malformed content length;
- response body larger than configured limit;
- redirect loop;
- cross-host redirect;
- cookie set/update/delete;
- TLS verification success/failure;
- connection reuse;
- server close during body;
- slow headers;
- slow body;
- streaming upload;
- streaming download;
- proxy route metadata;
- transcript redaction.

### Acceptance criteria

- Limits are enforced before unbounded allocation.
- Raw and normalized headers preserve intended semantics.
- Redirect and cookie behavior is deterministic.
- Secret headers and cookies do not appear in default transcripts, events, errors, or checkpoints.

## Workstream 8 — WebSocket correctness closure

### Goal

Validate the full provisional WebSocket lifecycle.

### Required fixtures

- plain `ws`;
- TLS `wss`;
- subprotocol negotiation;
- custom origin;
- ping/pong;
- clean close;
- abnormal close;
- fragmented text;
- fragmented binary;
- oversized message;
- idle timeout;
- receive cancellation;
- handshake rejection;
- redirected or changed authority;
- transcript redaction.

### Acceptance criteria

- Sync and async sessions have equivalent semantics.
- Message and close metadata are typed and complete.
- Policy is enforced at handshake and reconnect transitions.
- `websocket_assess` remains provisional unless all operation graduation gates pass.

## Workstream 9 — Live packet capture validation

### Goal

Separate parser confidence from real capture confidence and validate supported live paths.

### Required work

- Add a controlled loopback capture harness.
- Generate deterministic TCP, UDP, ICMP, and DNS traffic.
- Validate interface selection and privilege detection.
- Validate filter compilation.
- Validate packet iteration and async iteration.
- Validate queue bounds and drop accounting.
- Validate cancellation and stop behavior.
- Validate PCAP persistence and reopen.
- Validate repeated session lifecycle.
- Validate unsupported-platform errors.

### CI strategy

- parser and synthetic stream tests run everywhere;
- live capture runs only on explicitly supported runners;
- unsupported runners report structured skip reasons;
- no stable claim depends solely on an unavailable privileged job.

### Acceptance criteria

- Live capture is either validated on a documented support matrix or remains experimental.
- Layer C/D `xfail` markers are removed or tied to explicit external blockers.

## Workstream 10 — Privileged active-probe validation

### Goal

Define and validate the support boundary for ICMP, TCP SYN, UDP reachability, and traceroute.

### Required work

- detect raw-socket privileges before execution;
- validate IPv4 and IPv6 where supported;
- enforce target count and rate limits;
- test cancellation during probe wait;
- correlate responses correctly;
- validate unsupported-platform errors;
- verify scope on resolved targets and packet destination fields;
- ensure source spoofing is unavailable from stable APIs;
- separate safe one-shot probes from arbitrary injection.

### Acceptance criteria

- Supported platforms and privilege requirements are explicit in capability metadata.
- Raw injection remains experimental.
- Stable or provisional probes do not expose unrestricted packet construction.

## Workstream 11 — Registry and dispatcher tightening

### Goal

Reduce centralized dispatch growth before Release 3 adds more operations.

### Required refactor

Introduce domain executor adapters or typed executor entries that own:

- request conversion;
- sync execution;
- async execution;
- payload conversion;
- feature predicate;
- operation metadata.

The central engine should perform:

- registry lookup;
- common validation;
- policy and audit;
- timeout and cancellation setup;
- event lifecycle;
- executor invocation;
- common result finalization.

It should not contain detailed domain logic for every operation.

### Guardrails

Add guards for:

- maximum central dispatch match-arm growth;
- descriptor/executor parity;
- operation IDs duplicated outside the registry;
- sync/async executor mismatch;
- direct functions bypassing registry-backed adapters.

### Acceptance criteria

- At least the 12 promoted operations are migrated to adapter-owned execution.
- New Release 3 operations can be added without expanding a monolithic engine match.

## Workstream 12 — Maturity and documentation closure

### Goal

Make public maturity language match actual evidence.

### Required changes

- Update the graduation audit after feature-enabled runs.
- Distinguish API stability from wheel availability.
- Distinguish interface stability from enabled-path validation.
- Mark live capture and raw probes accurately.
- Document supported platforms and required privileges.
- Record the exact validation commit rather than `HEAD` placeholders.
- Add a Release 1/2 closure report.

### Acceptance criteria

- No operation is described as fully stable while its enabled implementation lacks release-profile evidence.
- Release 2 remains explicitly provisional where live-path evidence is incomplete.

## Workstream 13 — Performance and resource report publication

### Goal

Convert performance tooling into reproducible release evidence.

### Required metrics

- wheel and extension size by profile;
- import time;
- engine dispatch overhead;
- async dispatch overhead;
- HTTP request overhead;
- WebSocket message throughput;
- packet decode throughput;
- flow aggregation throughput;
- memory under slow consumers;
- cancellation latency;
- repeated session lifecycle leak checks.

### Acceptance criteria

- Reports are published as CI artifacts.
- Regressions beyond defined thresholds fail or warn according to policy.
- Profile size changes are tracked over time.

## Validation commands

The closure pass should drive the repository through the canonical script:

```bash
scripts/validate_python_release_1_2.sh --all --ci
```

Supplement with profile-specific commands for privileged or externally hosted fixtures.

At minimum also run:

```bash
cargo fmt --all -- --check
cargo clippy -p eggsec-python --all-targets -- -D warnings
python scripts/check-python-capability-matrix.py
python scripts/check-python-architecture-guards.py
python scripts/check_python_stub_parity.py
bash scripts/check_python_types.sh
bash scripts/measure_python_binary_size.sh
```

## Recommended commit sequence

1. fix two remaining failures;
2. skip categorization and report generation;
3. Git secrets and SBOM enabled profiles;
4. mobile and container artifact profiles;
5. NSE profile;
6. database fixture profile;
7. CI matrix and artifact publication;
8. real async I/O tests and fixes;
9. live scope transition fixtures and fixes;
10. HTTP/WebSocket protocol fixtures and fixes;
11. live capture validation;
12. active probe validation;
13. registry adapter refactor;
14. maturity, documentation, and closure report.

## Closure criteria

Releases 1 and 2 are closed when:

- the canonical validation run has zero unexplained failures;
- all skips are categorized;
- all 22 operations have evidence-backed graduation states;
- feature-gated operations have enabled-path validation or accurate reclassification;
- required CI profiles publish status checks and artifacts;
- async session cleanup is validated with real I/O;
- live network transitions cannot bypass scope;
- HTTP and WebSocket edge cases are covered by managed fixtures;
- live capture and active probes have explicit evidence-based support boundaries;
- registry architecture can absorb Release 3 without renewed monolithic growth;
- capability metadata and documentation record the exact validated commit;
- Release 1 is declared complete;
- Release 2 is declared complete as a bounded provisional network-programmability release, not as a fully stable raw-network API.