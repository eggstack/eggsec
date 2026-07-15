# Eggsec Python API Releases 1–4 Final Readiness Follow-Up

## Handoff objective

This pass closes the remaining release-readiness blockers after the operational correction work for Releases 1–4.

The repository now has the major implementation work in place:

- a shared process-global Tokio runtime fixes chained async session usage;
- local and feature-gated Python APIs have broad contract and lifecycle coverage;
- NSE, interception proxy, database, browser, mobile, daemon, repository, artifact, and reporting surfaces exist;
- process-level daemon tests and subsystem-specific test scripts are present;
- CI includes Python feature profiles and artifact publication.

The remaining risk is false confidence caused by non-blocking guards, environment-dependent skips, and feature profiles that can pass without exercising the subsystem they claim to validate.

This pass must make the release evidence authoritative: each dedicated profile either provisions and exercises its required subsystem or fails with an actionable error. Releases 1–4 should not be declared closed based on symbol availability, DTO construction, or skipped integration tests.

## Required end state

At completion:

1. architecture, capability, type, and release guards are mandatory CI gates;
2. every dedicated feature profile defines required prerequisites and fails when they are absent;
3. the daemon profile builds and launches a real daemon process and validates restart/replay behavior;
4. NSE network scripts run against deterministic loopback fixtures rather than skipping for connection refusal;
5. proxy tests exercise live HTTP, HTTPS, WebSocket, mutation, replay, and shutdown behavior;
6. database profiles run against real supported backend services with deterministic fixtures;
7. browser tests launch a real browser backend and verify cleanup;
8. mobile tests either run against a provisioned emulator or are isolated into a clearly non-blocking external validation workflow;
9. Python cancellation semantics are explicitly tested and documented;
10. release artifacts contain a per-profile pass/fail/skip report tied to the exact commit SHA;
11. maturity classifications reflect the evidence actually produced by CI;
12. no required profile can report success because all meaningful tests skipped.

## Scope boundaries

This is a closure and evidence pass, not a feature expansion release.

Do not:

- add new assessment domains;
- expand the stable operation registry;
- redesign the public session APIs unless a correctness issue requires it;
- add new storage backends;
- introduce a second CI framework;
- promote provisional APIs solely because test files exist.

Small implementation fixes discovered by the new strict tests are in scope.

---

# Workstream 1 — Make all release guards blocking

## Problem

The current workflow invokes architecture guards with both `|| true` and `continue-on-error: true`. This converts a release invariant into advisory output.

## Required work

1. Remove `|| true` from architecture, capability, type, stub-parity, and maturity checks.
2. Remove `continue-on-error` from release-critical validation steps.
3. Create one named required job, for example `python-release-guards`, that runs:
   - `scripts/check-architecture-guards.sh`;
   - `scripts/check-python-capability-matrix.py`;
   - `scripts/check_python_stub_parity.py`;
   - `scripts/check_python_types.sh`;
   - documentation/maturity consistency checks;
   - plan status and validation schema checks.
4. Ensure failures preserve logs and JSON reports as artifacts while still failing the job.
5. Add a negative fixture or unit test proving each guard detects an intentionally malformed temporary manifest or stub.
6. Document which CI jobs should be configured as protected-branch required checks.

## Acceptance criteria

- Deliberately breaking the capability manifest fails CI.
- Deliberately removing a public stub fails CI.
- Deliberately marking an experimental domain stable without evidence fails CI.
- No release-critical validation command contains `|| true` or `continue-on-error`.

---

# Workstream 2 — Define strict profile contracts

## Problem

Dedicated feature profiles can skip tests when their binary, service, browser, emulator, or system dependency is unavailable. This can produce false-green results.

## Required work

Introduce a profile contract file, preferably machine-readable, such as:

`crates/eggsec-python/validation_profiles.json`

Each profile must declare:

- Cargo features;
- system packages;
- binaries to build;
- services to start;
- environment variables;
- required tests;
- allowed skip reasons;
- forbidden skip reasons;
- timeout budget;
- expected artifacts;
- supported CI platforms;
- whether the profile is release-blocking or external/scheduled.

Profiles should include at least:

- `default-wheel`;
- `nse`;
- `db-postgres`;
- `db-mysql` or the actual supported SQL backend set;
- `web-proxy`;
- `websocket`;
- `packet-inspection-parser`;
- `daemon-client`;
- `headless-browser`;
- `mobile-emulator`;
- `repository-durability`;
- `stress-leak`.

Add a validator that rejects:

- unknown profiles;
- missing prerequisite commands;
- a required profile with zero executed tests;
- a required profile where all integration tests skipped;
- skip reasons outside the allowlist;
- missing expected artifacts.

## Acceptance criteria

- Every profile produces a structured summary containing collected, passed, failed, skipped, and xfailed counts.
- Required profiles fail if their required subsystem is absent.
- External profiles are visibly classified and cannot be mistaken for release-blocking green checks.

---

# Workstream 3 — Daemon profile: build, run, restart, and replay

## Problem

Daemon tests currently skip if `eggsec-daemon` was not built or cannot create its socket. A daemon-client profile must not pass under those conditions.

## Required work

1. Build `eggsec-daemon` explicitly in the daemon profile before Python tests.
2. Fail immediately if the binary is absent or not executable.
3. Start the daemon with an isolated temporary data directory and Unix socket.
4. Validate:
   - health and capability negotiation;
   - session create/list/get/close;
   - task submission and result retrieval;
   - idempotency-key duplicate submission;
   - cancellation before start and during execution;
   - event subscription and replay cursor behavior;
   - artifact upload/download integrity;
   - client disconnect and reconnect;
   - daemon termination during an active task;
   - daemon restart with persistence enabled;
   - session/task recovery after restart;
   - stale cursor and protocol mismatch errors;
   - cleanup of socket and child processes.
5. Add process logs and daemon data metadata to CI artifacts on failure.
6. Ensure the profile has no `pytest.skip()` path for missing daemon prerequisites.

## Likely files

- `.github/workflows/test.yml`;
- `crates/eggsec-python/tests/test_daemon_integration.py`;
- `crates/eggsec-python/tests/test_daemon_repository_operational.py`;
- `scripts/test_python_daemon_parity.sh`;
- daemon test-fixture helpers.

## Acceptance criteria

- Killing and restarting the daemon is part of the required profile.
- Duplicate submission returns the same logical task/result according to the documented idempotency contract.
- Event replay does not lose or duplicate events outside the documented at-least-once semantics.
- Missing daemon binary fails the job instead of skipping tests.

---

# Workstream 4 — Deterministic NSE service fixtures

## Problem

NSE tests skip when scripts encounter connection refusal or unavailable services. This prevents authoritative validation of runtime reuse and script behavior.

## Required work

1. Build a deterministic loopback fixture suite for the NSE scripts used in CI:
   - TCP banner service;
   - HTTP service with known headers and routes;
   - TLS service with a generated test certificate;
   - optional UDP/DNS fixture if required by supported scripts.
2. Parameterize target host and ports instead of assuming port 80.
3. Replace `_run_script_safe()` network-error skips in the strict NSE profile with fixture-backed assertions.
4. Retain skip behavior only in general/default-wheel runs where the `nse` feature is absent.
5. Exercise:
   - repeated runtime reuse;
   - multiple scripts in one runtime;
   - argument changes between runs;
   - cancellation during blocked I/O;
   - execution limits and timeout;
   - malformed scripts and diagnostics;
   - library registry/version conflict behavior;
   - cleanup after failed scripts;
   - concurrent runtimes.
6. Record per-script execution results in a JSON artifact.

## Acceptance criteria

- The strict NSE profile has no connection-refused skips.
- At least one real script executes against each relevant fixture protocol.
- Runtime reuse and cancellation tests execute rather than skip.

---

# Workstream 5 — Live interception proxy profile

## Problem

Proxy DTO and constructor coverage is strong, but release confidence requires real traffic flowing through the proxy.

## Required work

Provision local fixture services and validate:

1. plain HTTP forwarding;
2. HTTPS interception using an ephemeral test CA;
3. certificate issuance and hostname handling;
4. request-header mutation;
5. request-body mutation;
6. response-header mutation;
7. response-body mutation;
8. filter match and non-match behavior;
9. HAR export and round-trip parsing;
10. replay of a captured exchange;
11. response comparison rules;
12. WebSocket upgrade and bidirectional message capture;
13. cancellation during active traffic;
14. graceful stop with in-flight connections;
15. forced shutdown cleanup;
16. no CA private-key leakage in logs, events, or serialized reports;
17. scope enforcement for proxy destination and redirect targets.

The strict profile must fail if the fixture, CA generation, or proxy listener cannot start.

## Acceptance criteria

- At least one complete HTTP, HTTPS, and WebSocket transaction passes through the proxy.
- Mutated and unmodified exchanges are distinguishable and serialized correctly.
- Replay reproduces the expected fixture response.
- Shutdown leaves no listener or worker process behind.

---

# Workstream 6 — Real database backend profiles

## Problem

Database models and tests are extensive, but authoritative validation requires real backends.

## Required work

1. Select the backend set officially supported for this release. Do not imply parity for untested drivers.
2. Use GitHub Actions services or Docker Compose to provision deterministic databases.
3. Create least-privileged test users, schemas, tables, indexes, extensions, and intentionally misconfigured privileges.
4. Validate per backend:
   - connection and authentication;
   - static/environment/callback credential providers;
   - failed credentials and retry limits;
   - session reuse;
   - bounded query execution;
   - row streaming and backpressure;
   - query plans;
   - schema/table/index/extension discovery;
   - privilege inspection;
   - cancellation and timeout;
   - connection loss and recovery;
   - cleanup and secret redaction.
5. Split backend-specific tests so one unavailable backend does not hide failures in another.
6. Produce a backend capability report artifact.
7. Reclassify drivers without an enabled CI profile as provisional or unsupported.

## Acceptance criteria

- Required backend services are started by CI and health-checked before tests.
- Missing service fails the matching backend profile.
- Credentials never appear in JUnit output, logs, JSON artifacts, or repr strings.

---

# Workstream 7 — Real browser backend profile

## Problem

Browser session APIs require proof against an actual installed browser/backend.

## Required work

1. Choose and document the primary backend for CI, such as Chromium via CDP or the currently implemented engine.
2. Install or cache the browser in the profile.
3. Fail if the browser executable is unavailable.
4. Run against controlled HTTP and HTTPS fixture sites.
5. Validate:
   - launch and capability discovery;
   - navigation and redirects;
   - DOM querying/snapshot;
   - JavaScript execution;
   - cookie and local/session storage;
   - console capture;
   - network request/response capture;
   - security observations;
   - download handling and artifact registration;
   - screenshots and PDF where supported;
   - proxy integration;
   - cancellation during navigation;
   - browser crash/disconnect behavior;
   - repeated session open/close;
   - no orphan browser processes.
6. Upload browser logs and screenshots only on failure, with secret redaction.

## Acceptance criteria

- The headless-browser profile launches a real browser.
- Required tests cannot skip because no backend was found.
- Process-leak checks confirm zero child browsers after completion.

---

# Workstream 8 — Mobile emulator profile and classification

## Problem

Hosted CI support for Android emulators can be slow and unstable. The release boundary must distinguish mandatory API contract validation from external device validation.

## Required work

1. Decide whether the Android emulator profile is:
   - release-blocking on GitHub-hosted CI;
   - scheduled/nightly;
   - self-hosted and release-blocking;
   - explicitly external and provisional.
2. Provision a known emulator image and API level where feasible.
3. Build or include a minimal test APK with deterministic behaviors.
4. Validate:
   - device discovery;
   - install/uninstall;
   - launch/stop;
   - static-to-dynamic plan conversion;
   - instrumentation script loading;
   - event and evidence collection;
   - cancellation;
   - emulator disconnect;
   - cleanup of package, instrumentation, and ADB processes.
5. Fail when the profile is designated release-blocking and the emulator is unavailable.
6. If the profile remains external, keep mobile dynamic APIs provisional and publish the latest external validation date and environment.

## Acceptance criteria

- The maturity label matches the selected validation tier.
- No documentation implies release-blocking emulator validation unless it actually runs.

---

# Workstream 9 — Packet capture and privileged probe profile policy

## Problem

Raw capture and active probes vary by kernel capabilities and runner environment.

## Required work

1. Separate parser-only tests from privileged live-capture tests.
2. Make parser/PCAP replay tests release-blocking on ordinary CI.
3. Define a privileged Linux profile for:
   - loopback capture;
   - BPF filters;
   - ICMP echo;
   - TCP SYN probe;
   - UDP reachability;
   - traceroute where deterministic.
4. Detect required capabilities (`CAP_NET_RAW`, `CAP_NET_ADMIN`) before running.
5. Fail the privileged profile if capabilities were expected but unavailable.
6. Keep unsupported platforms explicitly experimental.
7. Record kernel, libpcap, and capability metadata in the artifact.

## Acceptance criteria

- Parser tests never require privileges.
- Privileged tests cannot silently skip in their dedicated environment.
- Platform support tables match observed CI coverage.

---

# Workstream 10 — Python cancellation contract

## Problem

The shared runtime fixes resource lifetime, but the custom `PyFuture` does not provide native `asyncio` cancellation propagation.

## Required work

1. Define the supported cancellation semantics precisely:
   - explicit `CancellationToken` only;
   - automatic propagation from Python task cancellation;
   - or both.
2. Add tests for:
   - `asyncio.wait_for()` timeout;
   - cancelling the Python task before completion;
   - dropping a `PyFuture`;
   - explicit cancellation token;
   - cancellation during read/write/connect;
   - cancellation of daemon-backed operations;
   - resource cleanup after cancellation.
3. If automatic propagation is not implemented, ensure the Python task cancellation does not leak Rust tasks indefinitely.
4. Consider migrating the bridge to `pyo3-async-runtimes` only if required to meet the chosen contract; do not perform a broad rewrite without measured benefit.
5. Document behavior in API reference and examples.

## Acceptance criteria

- Cancellation behavior is deterministic and tested.
- No cancellation path leaks sockets, tasks, threads, child processes, or temporary files.
- Documentation does not imply native propagation if only explicit tokens are supported.

---

# Workstream 11 — Skip and xfail governance

## Problem

Skipped tests are currently used for both legitimate feature absence and missing operational prerequisites.

## Required work

1. Introduce standardized skip categories:
   - `feature_not_compiled`;
   - `platform_unsupported`;
   - `external_profile_only`;
   - `optional_tool_absent`;
   - `known_issue`.
2. Forbid generic skip messages such as “binary not built,” “service unavailable,” or “connection refused” in required profiles.
3. Produce a JSON skip report grouped by category, test, and profile.
4. Add profile-specific maximum skip budgets.
5. Require issue identifiers and expiration dates for `known_issue` xfails.
6. Fail CI when:
   - a forbidden skip occurs;
   - a skip budget is exceeded;
   - all tests in a required group skip;
   - an xpass occurs for a strict xfail.

## Acceptance criteria

- Every skip in CI has a recognized category.
- Required profiles have zero prerequisite-related skips.

---

# Workstream 12 — Authoritative release evidence bundle

## Problem

Workflow definitions exist, but release decisions need durable evidence tied to the exact commit.

## Required work

Create a release evidence bundle containing:

- commit SHA and dirty-state indicator;
- compiler, Rust, Python, OS, kernel, and architecture versions;
- Cargo feature set;
- wheel filename and SHA-256;
- dependency inventory;
- per-profile test counts;
- failures, skips, and xfails;
- architecture/capability/type guard results;
- performance and leak reports;
- binary-size report;
- backend/browser/emulator/daemon versions;
- privileged capability metadata;
- maturity classification snapshot;
- links or artifact names for JUnit XML and logs.

Add a script such as:

`scripts/build_python_release_evidence.sh`

The script must fail if required component reports are absent or refer to a different commit.

## Acceptance criteria

- One downloadable artifact represents the complete Releases 1–4 validation state.
- Every report inside the bundle identifies the current commit SHA.
- The evidence builder rejects stale reports.

---

# Workstream 13 — Maturity and support-table reconciliation

## Required work

After strict profiles run:

1. Update `_capabilities.json` with actual validation profile and commit.
2. Update `domain-maturity.md` and stability documentation.
3. For each domain, record:
   - stable/provisional/experimental;
   - default-wheel or feature-gated;
   - supported platforms;
   - required external systems;
   - latest successful operational profile;
   - known limitations;
   - cancellation contract;
   - privilege requirements.
4. Do not promote:
   - mobile dynamic sessions without emulator evidence;
   - browser sessions without live backend evidence;
   - daemon parity without restart/replay evidence;
   - database drivers without backend profiles;
   - live capture on unsupported runners.
5. Add guards preventing maturity promotion without a named passing validation profile.

## Acceptance criteria

- Maturity claims are generated or verified against release evidence.
- No domain is stable solely because its symbols and stubs exist.

---

# Workstream 14 — Final validation and handoff report

Run the complete strict matrix and publish a final handoff report.

The report must include:

- exact commit;
- all required profiles and results;
- all non-required/external profiles and their status;
- zero unexplained failures;
- zero forbidden skips;
- architecture/type/capability guards green;
- resource-leak summary;
- daemon restart/replay result;
- database backend coverage;
- proxy HTTP/HTTPS/WebSocket coverage;
- browser backend coverage;
- mobile emulator coverage or explicit provisional deferral;
- remaining known limitations;
- recommendation: ready for Release 5, conditionally ready, or not ready.

Suggested file:

`docs/python/RELEASES_1_4_FINAL_VALIDATION.md`

---

# Recommended implementation sequence

1. Make guards blocking.
2. Add the machine-readable profile contract and skip governance.
3. Fix daemon profile prerequisite handling.
4. Add deterministic NSE fixtures.
5. Close live proxy validation.
6. Add real database service profiles.
7. Close browser backend validation.
8. Decide and implement the mobile emulator validation tier.
9. Separate parser and privileged network profiles.
10. Lock down cancellation semantics.
11. Build the evidence bundle.
12. Reconcile maturity metadata.
13. Run the full matrix and write the final validation report.

Do not postpone strictness until after fixture work: required profiles should fail early while incomplete so false-green results are eliminated immediately.

# Commit strategy

Prefer focused commits:

1. `ci(python): make release guards blocking`
2. `test(python): add strict validation profile contracts`
3. `test(python): require real daemon lifecycle in daemon profile`
4. `test(python): add deterministic NSE fixtures`
5. `test(python): validate live proxy traffic and replay`
6. `test(python): add real database backend profiles`
7. `test(python): require real browser backend lifecycle`
8. `test(python): define mobile emulator validation tier`
9. `test(python): split parser and privileged network profiles`
10. `fix(python): close cancellation lifecycle gaps`
11. `ci(python): publish commit-bound release evidence bundle`
12. `docs(python): reconcile maturity after strict validation`

# Final acceptance criteria

This follow-up is complete only when all of the following are true:

- release guards are blocking;
- required profiles cannot pass with absent prerequisites;
- daemon binary build and process-level restart/replay tests pass;
- strict NSE tests use deterministic services without connection-error skips;
- proxy profile exercises HTTP, HTTPS, WebSocket, mutation, replay, and shutdown;
- required database backends are tested against live services;
- browser profile launches a real backend and leaves no processes;
- mobile validation tier is explicit and reflected in maturity;
- parser and privileged packet tests are correctly separated;
- cancellation behavior is explicitly specified and leak-free;
- all skips are categorized and within profile budgets;
- one commit-bound release evidence bundle is published;
- maturity claims match the evidence;
- the final validation report recommends proceeding to Release 5 without unresolved release-blocking issues.
