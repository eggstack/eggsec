# Python API Release 5 Phase F — Compatibility, Performance, and Release Closure

## Objective

Close Release 5 with enforceable API compatibility, performance/resource budgets, exact-commit evidence, and a domain-by-domain graduation decision. This phase converts the improved architecture into a supportable public contract and prevents future changes from silently breaking Python consumers.

## Workstream F1 — Compatibility baseline

Create a committed machine-readable baseline containing:

- canonical public modules and symbols;
- signatures and keyword-only behavior;
- class constructors, methods, properties, and protocol support;
- enum names and values;
- exception hierarchy and stable error codes;
- stable operation IDs and aliases;
- request and result schema hashes;
- tool descriptor schema hashes;
- event, finding, artifact, checkpoint, ABI, and protocol versions;
- wheel-profile feature inventories;
- maturity and deprecation state.

Generate the baseline from an installed release-candidate wheel. Do not hand-maintain it independently from the canonical registry and stubs.

## Workstream F2 — Semantic compatibility checker

Implement a checker that classifies changes as:

- compatible addition;
- compatible widening;
- deprecation;
- provisional/experimental change;
- potentially breaking;
- definitely breaking.

It must detect:

- removed or moved stable symbols without aliases;
- changed required parameters or defaults;
- positional/keyword incompatibility;
- enum value removal or renaming;
- result field removal or type narrowing;
- schema incompatibility;
- exception/error-code changes;
- operation ID or alias changes;
- maturity promotion/demotion;
- feature-profile export drift;
- protocol/ABI changes without version increments.

Breaking changes require an explicit allowlist entry with rationale, migration documentation, versioning decision, and removal timeline.

## Workstream F3 — Stable versus provisional policy

Document compatibility guarantees separately:

- stable APIs: semantic compatibility required;
- provisional APIs: best-effort compatibility, migration notes required for material changes;
- experimental APIs: no compatibility promise, but changes remain documented;
- deprecated APIs: retained until declared removal floor;
- internal APIs: excluded from public inventories.

The checker must apply different severity rules by maturity and reject accidental stable expansion caused solely by top-level re-export.

## Workstream F4 — Performance baselines

Establish reproducible baselines for:

- package import time;
- imported module count;
- extension and wheel size;
- engine and async-engine construction;
- operation descriptor lookup;
- schema generation and validation;
- denied/no-op dispatch overhead;
- callback and event delivery overhead;
- finding/event serialization;
- binary buffer access and copying;
- repository inserts, queries, pagination, and baseline diff;
- reporter throughput;
- session open/close cycles;
- daemon request normalization and transport overhead.

Separate operation I/O performance from Python binding overhead. Use percentile distributions where timing noise matters.

## Workstream F5 — Resource budgets

Add blocking or scheduled tests for:

- file-descriptor growth;
- thread growth;
- Tokio task leaks;
- Python reference cycles;
- callback queue growth;
- resident memory growth under repeated DTO creation and serialization;
- sockets and subprocesses after session close;
- temporary directories and certificate stores;
- database connection-pool cleanup;
- browser/mobile child-process cleanup;
- repository and reporter behavior at large finding counts.

Define absolute or relative budgets per platform/profile. Record noise-tolerant thresholds and require rationale for budget increases.

## Workstream F6 — Multi-profile release matrix

Consume the canonical validation-profile manifest created by the Releases 1–4 polish pass. Required profiles should include:

- core/default wheel;
- full-no-system wheel;
- git secrets;
- SBOM;
- WebSocket;
- packet parser;
- privileged packet/live probes where supported;
- NSE with deterministic fixtures;
- PostgreSQL, MySQL/MariaDB, Redis, and MongoDB where supported;
- live web proxy;
- container scanning;
- mobile static;
- scheduled mobile emulator;
- real headless browser;
- daemon client/process parity;
- tool/schema integration;
- typing and consumer-package tests.

A dedicated profile must fail when its prerequisites are missing or all meaningful tests skip. Scheduled/external profiles must be visibly distinguished from blocking pull-request profiles.

## Workstream F7 — Security and redaction closure

Run adversarial tests across every serialization and integration boundary:

- credentials and sensitive strings;
- HTTP authorization headers and cookies;
- proxy captures and HAR;
- database credential callbacks and query errors;
- NSE script arguments and output;
- daemon requests, events, replay, and persisted snapshots;
- checkpoints and repositories;
- tool descriptors and generated schemas;
- callbacks, logs, repr, and exception chains;
- report and artifact manifests.

Use unique sentinels and inspect all generated files and event streams. Stable release is blocked by any secret leak.

## Workstream F8 — Exact-commit evidence bundle

Produce `target/python-validation/<commit-sha>/` containing:

- source commit and dirty-tree state;
- Rust and Python toolchains;
- platform/architecture;
- package and protocol versions;
- wheel filenames, tags, hashes, and sizes;
- profile manifest snapshot;
- compiled Cargo features;
- export and capability inventories;
- compatibility report;
- schema hash report;
- mypy and pyright results;
- test counts, skips, xfails, and reasons;
- performance and resource reports;
- security/redaction report;
- generated documentation/example results;
- subsystem fixture and service versions;
- maturity/graduation decisions.

The aggregation step must reject missing files, stale commit identities, mismatched wheels, or incomplete required profiles.

## Workstream F9 — Domain graduation review

Review each domain separately.

Candidates for stable or continued stable status must have:

- canonical registry identity;
- complete request/result/tool schemas;
- policy, scope, audit, timeout, cancellation, event, and serialization coverage;
- deterministic fixtures;
- installed-wheel tests;
- compatibility baseline entry;
- documentation and examples;
- supported profile evidence.

Expected outcomes:

- retain stable status for proven core and promoted Release 1 domains;
- consider WebSocket assessment and offline packet analysis only if evidence is complete;
- keep daemon, browser, mobile dynamic, live capture, interception proxy, and stateful database sessions provisional unless their process/service profiles are fully green;
- keep wireless, raw injection, evasion, postex, C2, hunting, and AI experimental unless separately justified.

No domain is promoted because its classes import successfully.

## Workstream F10 — Release checklist and publication gate

Create a blocking `python-release-5-gate` that requires:

- architecture and generated-file guards;
- compatibility approval;
- stable schema compatibility;
- all required wheel/profile tests;
- typing and consumer tests;
- docs/examples;
- performance/resource budgets;
- redaction/security suite;
- exact-commit evidence aggregation;
- clean package metadata;
- version/tag agreement;
- TestPyPI installation rehearsal.

The gate must fail on skipped required jobs, stale artifacts, or manual `continue-on-error` suppression.

## Workstream F11 — Release notes and migration guide

Publish:

- new tool/schema API;
- registry convergence and behavioral invariants;
- canonical namespace structure;
- compatibility aliases and deprecations;
- wheel profile and installation changes;
- asyncio cancellation contract;
- typing improvements;
- maturity changes;
- known unsupported platforms or subsystem prerequisites;
- performance and size changes;
- migration examples from flat imports and `api_surface()`-based integrations.

## Acceptance criteria

Phase F and Release 5 are complete when:

- a generated semantic compatibility baseline exists and is blocking;
- stable schemas and error contracts cannot drift silently;
- binding overhead, import time, wheel size, memory, threads, tasks, sockets, and file descriptors remain within explicit budgets;
- required subsystem profiles execute meaningful tests without hidden skip-based success;
- no secret sentinel appears in any public or persisted boundary;
- exact-commit release evidence is complete and internally consistent;
- every domain has an evidence-backed maturity decision;
- TestPyPI artifacts install and pass the release smoke suite;
- the release gate is green without suppressed failures.