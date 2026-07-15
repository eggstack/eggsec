# Eggsec Python API Releases 1–4 Final Polish Pass

## Handoff objective

This pass closes the remaining release-readiness gaps after the operational correction and final-readiness work for the Python API through Release 4.

The repository is now implementation-complete at the API-surface level and operationally credible for the default local engine. The remaining weakness is evidence quality across feature profiles: required integration jobs can still succeed without proving the subsystem they represent, large skip counts obscure coverage, and release artifacts are not yet authoritative for an exact commit.

This pass must convert the current clean local validation state into a strict, reproducible, multi-profile release gate suitable for declaring Releases 1–4 closed.

## Required end state

At completion:

1. architecture, capability, type, stub, maturity, and release guards are blocking CI jobs;
2. each feature profile has a machine-readable contract defining required binaries, services, environment, tests, and skip budget;
3. required profiles fail when prerequisites are missing or when all meaningful tests skip;
4. NSE tests use deterministic loopback services rather than connection-refusal skips;
5. proxy tests exercise live HTTP, HTTPS, WebSocket, mutation, replay, CA, and shutdown paths;
6. database profiles start and test real supported backend services;
7. daemon tests build and launch the daemon, then verify restart, reconnect, replay, idempotency, cancellation races, and artifacts;
8. browser tests launch a real browser backend and prove cleanup;
9. mobile validation has an explicit emulator workflow and cannot be confused with default CI coverage;
10. parser-only packet tests are separated from privileged live-capture and active-probe validation;
11. skip and xfail usage is governed by profile-specific budgets and reasons;
12. release evidence is tied to the exact commit SHA and uploaded as a durable artifact;
13. maturity classifications are generated from the evidence matrix rather than hand-maintained claims;
14. a single release-readiness summary reports pass, fail, skip, xfail, duration, binary size, and artifact links for every profile.

## Scope boundaries

This is a polish and evidence pass. Do not add new Python API domains, new scanning capabilities, new protocol implementations, or new compatibility promises.

Production-code changes are allowed only when required to make an existing profile operational, deterministic, or testable.

## Workstream 1 — Make all release guards blocking

### Problem

Some validation scripts have previously been invoked with `|| true`, `continue-on-error`, or equivalent non-blocking behavior. This permits false-green runs.

### Required work

- Audit `.github/workflows/test.yml`, `.github/workflows/python-wheels.yml`, and any reusable workflows.
- Remove `|| true`, `continue-on-error: true`, and shell constructs that suppress non-zero exits from required checks.
- Run these as independent required jobs:
  - architecture guards;
  - capability matrix validation;
  - API surface drift validation;
  - stub parity;
  - mypy;
  - pyright, with only narrowly documented false-positive exclusions;
  - maturity consistency;
  - feature metadata consistency;
  - release profile manifest validation.
- Ensure each job uploads logs even on failure.
- Add a top-level `python-release-gate` aggregation job that fails unless every required dependency succeeded.

### Acceptance criteria

- Deliberately breaking one guard causes the workflow to fail.
- No required guard is marked `continue-on-error`.
- The release aggregation job cannot report success when a required job is skipped or cancelled.

## Workstream 2 — Define profile contracts

### Problem

Feature names currently imply coverage without a canonical definition of what each profile must provision and exercise.

### Required work

Create `crates/eggsec-python/validation/profiles.json` or equivalent with one object per profile:

- `default-wheel`;
- `websocket`;
- `packet-parser`;
- `packet-live`;
- `git-secrets`;
- `sbom`;
- `nse`;
- `db-postgres`;
- `db-mysql`;
- `db-redis`;
- `db-mongodb` where supported;
- `web-proxy`;
- `container`;
- `mobile-static`;
- `mobile-emulator`;
- `headless-browser`;
- `daemon-client`;
- `full-no-system`.

Each profile must declare:

- Cargo features;
- system packages;
- binaries and services;
- fixture startup command;
- readiness probe;
- test selector;
- required test count minimum;
- maximum allowed skips and xfails;
- whether the profile is blocking, scheduled, or external;
- supported operating systems;
- expected artifacts;
- timeout and resource budget.

Add a validator that rejects missing fields, duplicate profile names, unknown features, invalid selectors, and impossible skip budgets.

### Acceptance criteria

- CI is generated from or validated against the profile contract.
- A profile cannot silently change its feature set or test selector without a manifest diff.

## Workstream 3 — Enforce skip and xfail budgets

### Problem

The combined suite reports a large skip count, making it difficult to distinguish legitimate feature exclusion from missing validation.

### Required work

- Add a pytest plugin or post-processor that emits structured skip and xfail records.
- Record:
  - test node ID;
  - profile;
  - reason;
  - feature gate;
  - prerequisite;
  - source file and line;
  - whether the skip was expected by the profile contract.
- Fail a blocking profile when:
  - all selected tests skip;
  - fewer than the required minimum tests run;
  - an unexpected skip occurs;
  - the skip budget is exceeded;
  - a broad `except Exception: pytest.skip(...)` hides an implementation failure.
- Prohibit network-error skips in dedicated integration profiles.
- Require an issue or explicit deferred-work reference for long-lived xfails.

### Acceptance criteria

- The default profile may skip feature-gated tests within its declared budget.
- Dedicated subsystem profiles have near-zero unexpected skips.
- A missing daemon, browser, database, or fixture causes failure rather than skip.

## Workstream 4 — Deterministic NSE fixture environment

### Problem

NSE runtime tests still skip scripts when expected target services are unavailable.

### Required work

Create a deterministic loopback fixture bundle supporting the built-in scripts under test:

- TCP banner service;
- HTTP server with deterministic headers and response bodies;
- HTTPS server with a generated test certificate;
- DNS fixture where required;
- controlled open and closed ports;
- predictable service fingerprints.

Update NSE tests to use fixture-provided targets and ports. Remove `_run_script_safe()` skip behavior from the dedicated NSE profile.

Validate:

- runtime reuse across multiple scripts;
- repeated runs;
- per-script arguments;
- cancellation;
- execution limits;
- diagnostics;
- library resolution;
- evidence and output serialization;
- cleanup after script failure.

### Acceptance criteria

- Dedicated NSE CI executes every required network script without connection-refusal skips.
- Fixture startup failure fails the profile.
- Test outputs are deterministic across repeated runs.

## Workstream 5 — Live interception proxy validation

### Problem

Proxy DTO and constructor coverage is strong, but live interception evidence must be authoritative.

### Required work

Provision local fixtures for:

- HTTP origin;
- HTTPS origin;
- WebSocket origin;
- generated test CA and client trust store.

Exercise:

- proxy startup and readiness;
- HTTP request and response capture;
- HTTPS MITM and certificate issuance;
- request mutation;
- response mutation;
- filtering and bypass rules;
- HAR export;
- replay and response comparison;
- WebSocket upgrade and message capture;
- concurrent clients;
- cancellation;
- graceful and forced shutdown;
- port and file cleanup.

Add explicit negative cases for invalid CA paths, out-of-scope destinations, malformed mutation callbacks, and replay to denied targets.

### Acceptance criteria

- The web-proxy profile performs real traffic through the proxy.
- At least one HTTPS request succeeds through the generated CA path.
- No required proxy test is satisfied solely by DTO construction.

## Workstream 6 — Real database backend profiles

### Problem

Database surface and contract coverage are broad, but backend behavior must be proven against real services.

### Required work

Add service-backed CI profiles using pinned container images for supported backends. At minimum:

- PostgreSQL;
- MySQL or MariaDB;
- Redis if represented as a supported assessment target;
- MongoDB if currently supported by the Rust backend.

For each backend:

- start the service with deterministic credentials;
- wait for readiness;
- create fixture schemas, users, roles, tables, indexes, and vulnerable configurations;
- test connection metadata;
- test credential providers;
- run bounded queries;
- stream rows;
- inspect schemas, indexes, extensions, and privileges;
- test timeout and cancellation;
- test invalid credentials;
- verify secret redaction;
- verify cleanup and connection release.

Do not require unrestricted destructive testing. Use isolated containers and bounded fixture databases.

### Acceptance criteria

- Every backend profile performs a real authenticated session.
- Missing container services fail the profile.
- Database stable or provisional maturity is derived from backend-specific results.

## Workstream 7 — Daemon process parity and failure modes

### Problem

Process-level daemon testing exists, but dedicated CI must build the daemon and make restart/replay behavior mandatory.

### Required work

- Build `eggsec-daemon` explicitly in the daemon profile.
- Treat an absent or non-executable binary as failure.
- Start the daemon with an isolated temporary data directory and socket.
- Verify:
  - health and protocol negotiation;
  - session creation, listing, snapshot, and closure;
  - operation submission;
  - idempotency-key duplicate handling;
  - event subscription;
  - replay cursor continuity;
  - disconnect and reconnect;
  - daemon restart with persisted state where supported;
  - cancellation before start, during execution, and after completion;
  - artifact metadata and transfer;
  - concurrent sessions;
  - stale socket recovery;
  - process cleanup.
- Add a hard readiness timeout and capture stdout/stderr on failure.

### Acceptance criteria

- The daemon profile cannot pass without launching a real daemon.
- Restart/reconnect tests run rather than skip.
- Local/daemon normalized result differences are explicitly allowlisted and versioned.

## Workstream 8 — Real browser backend profile

### Problem

Browser session contracts exist, but browser process execution and cleanup must be proven.

### Required work

- Select the canonical supported backend for CI, such as Chromium through CDP, Playwright, or the existing Eggsec browser implementation.
- Pin the browser version or installation method.
- Launch an isolated local browser process.
- Run against deterministic local pages that exercise:
  - navigation;
  - redirects;
  - DOM inspection;
  - cookies;
  - local and session storage;
  - console capture;
  - network events;
  - downloads;
  - screenshots or artifacts;
  - security observations;
  - cancellation during navigation;
  - browser crash or forced termination;
  - cleanup of profiles and child processes.

### Acceptance criteria

- The headless-browser profile launches a real browser.
- Missing browser binaries fail the profile.
- No browser child processes remain after the suite.

## Workstream 9 — Mobile validation policy and emulator workflow

### Problem

A full Android emulator profile may be too expensive for every pull request, but mobile validation must be explicit rather than implied.

### Required work

Split mobile coverage into:

1. `mobile-static` — blocking, fixture APK/IPA analysis without emulator requirements;
2. `mobile-emulator` — scheduled or manually triggered, provisions Android SDK, emulator, ADB, and test application.

The emulator profile must validate:

- device discovery;
- application install;
- launch and stop;
- instrumentation attachment;
- event and evidence collection;
- cancellation;
- emulator disconnect;
- cleanup and uninstall;
- artifact retrieval.

Mark dynamic mobile APIs provisional until the emulator profile is green for the current release commit.

### Acceptance criteria

- Documentation clearly separates static and emulator-backed validation.
- Default or static profiles do not claim dynamic-session proof.
- Emulator failures are visible in scheduled release evidence.

## Workstream 10 — Packet and privileged probe profile separation

### Problem

Parser tests and privileged live network tests have different portability and permission requirements.

### Required work

Separate:

- `packet-parser`: deterministic PCAP files, decoders, filters, flow aggregation, serialization;
- `packet-live`: interface enumeration, live capture, BPF application, cancellation, and cleanup;
- `active-probes`: ICMP, SYN, UDP reachability, and traceroute with required capabilities.

For privileged profiles:

- document required Linux capabilities;
- apply capabilities narrowly;
- fail on missing privileges rather than skip;
- restrict all targets to loopback or isolated network namespaces;
- validate rate limits and scope enforcement.

### Acceptance criteria

- Parser-only CI remains portable and blocking.
- Privileged validation is isolated and produces explicit platform evidence.

## Workstream 11 — Python cancellation semantics

### Problem

The shared Tokio runtime fixes chained awaits, but the custom `PyFuture` bridge does not provide native `asyncio` cancellation propagation.

### Required work

Choose and document one supported contract:

- integrate Python task cancellation with Rust task cancellation; or
- explicitly require Eggsec cancellation tokens and treat Python task cancellation as future detachment only.

Add tests for:

- cancelling an awaiting Python task;
- cancelling through the Eggsec token;
- dropping a future before completion;
- ensuring background tasks do not leak;
- closing a session while an operation is pending;
- cancellation latency.

Do not claim native asyncio cancellation unless the underlying Rust task is actually cancelled.

### Acceptance criteria

- Behavior is deterministic and documented.
- No sockets, threads, sessions, or tasks leak after cancellation cases.

## Workstream 12 — Commit-bound evidence bundle

### Required work

Create a release evidence bundle under `target/python-validation/<commit-sha>/` containing:

- profile manifest snapshot;
- commit SHA and dirty-tree status;
- Rust toolchain;
- Python version;
- platform details;
- Cargo feature list;
- wheel filename and SHA-256;
- test counts;
- skips and xfails with reasons;
- JUnit XML;
- guard results;
- type-check results;
- binary-size report;
- performance and leak report;
- subsystem fixture versions;
- maturity decision report.

Add an aggregation script that fails when required files are missing or reference a different commit.

Upload the bundle as a single named artifact with retention appropriate for release candidates.

### Acceptance criteria

- Every release candidate has one authoritative evidence bundle.
- Evidence from an older commit cannot satisfy the gate for a newer commit.

## Workstream 13 — Evidence-driven maturity reconciliation

### Required work

- Generate or validate maturity classifications from profile outcomes.
- Preserve `stable` only when all required stable profiles pass.
- Keep daemon, browser, mobile dynamic, live capture, raw probes, proxy, NSE runtime, and stateful database sessions provisional unless their required evidence is green.
- Record explicit blockers in `_capabilities.json` or the canonical maturity manifest.
- Add a guard preventing docs from claiming a higher maturity than the evidence report.

### Acceptance criteria

- Maturity cannot be promoted through documentation-only edits.
- Failed or missing required evidence automatically blocks promotion.

## Workstream 14 — Final release-readiness report

Create `docs/python/RELEASES_1_4_FINAL_READINESS.md` containing:

- exact evaluated commit;
- profile table;
- test and skip counts;
- required versus optional profiles;
- known limitations;
- maturity decisions;
- deferred platform validations;
- evidence artifact name;
- explicit recommendation: ready, conditional, or not ready.

The report must be generated from machine-readable results where possible and reviewed for consistency with README, domain maturity, capability matrix, and release checklist.

## Recommended implementation sequence

1. Make guards blocking.
2. Add profile manifest and validator.
3. Add skip-budget tooling.
4. Harden daemon profile.
5. Replace NSE skips with loopback fixtures.
6. Add live proxy fixtures.
7. Add database service profiles.
8. Add browser backend profile.
9. Split mobile static and emulator workflows.
10. Separate packet parser and privileged profiles.
11. Finalize cancellation semantics.
12. Build evidence bundle and maturity reconciliation.
13. Generate final readiness report.

Do not mark the plan complete before the strict profiles have run successfully for the final commit.

## Validation commands

At minimum, provide reproducible commands equivalent to:

```bash
python scripts/validate_python_profiles.py --manifest crates/eggsec-python/validation/profiles.json
python scripts/check-python-architecture-guards.py
python scripts/check-python-capability-matrix.py
python scripts/check_python_stub_parity.py
bash scripts/check_python_types.sh
python scripts/run_python_profile.py default-wheel
python scripts/run_python_profile.py nse
python scripts/run_python_profile.py web-proxy
python scripts/run_python_profile.py db-postgres
python scripts/run_python_profile.py daemon-client
python scripts/run_python_profile.py headless-browser
python scripts/build_python_release_evidence.py --commit "$(git rev-parse HEAD)"
```

Profile-specific setup may be delegated to scripts or CI services, but the profile runner must be the canonical interface.

## Acceptance criteria

This pass is complete only when:

- all required guards are blocking and green;
- no blocking profile succeeds with all meaningful tests skipped;
- dedicated subsystem profiles provision their prerequisites;
- NSE, proxy, database, daemon, and browser profiles execute real integrations;
- mobile validation is accurately separated into static and emulator-backed evidence;
- privileged network validation is isolated and explicit;
- cancellation behavior is documented and leak-tested;
- the exact commit has a complete evidence bundle;
- maturity documents match the generated evidence;
- the final readiness report recommends closure without unresolved required blockers.

## Handoff notes

Keep this pass narrow. Prefer deleting permissive skip paths and strengthening fixtures over adding more test files that only construct DTOs. Every new test should answer a release question, and every required profile should fail loudly when it cannot exercise its subsystem.
