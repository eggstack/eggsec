# Eggsec Python API Release 4 — Stateful and Remote Execution

## Handoff objective

Release 4 completes the Python API for long-lived assessment sessions, remote execution, persistence, comparison, and reporting.

The release focuses on four major areas:

- mobile dynamic-analysis sessions;
- browser assessment sessions;
- local/daemon execution parity;
- persistent findings, assessments, artifacts, baselines, and reports.

Release 4 should begin only after Release 3 has established consistent managed-session patterns for NSE, interception proxy, and database APIs. Those patterns should be reused rather than reinvented for mobile, browser, daemon, and storage domains.

## Release outcome

At completion, Python users can:

- discover and manage mobile devices or emulators;
- run bounded dynamic mobile assessments with logs, captures, screenshots, and artifacts;
- create browser sessions for security-focused navigation, DOM, route, console, network, cookie, and storage inspection;
- execute the same stable operations through local or daemon-backed engines;
- reconnect to daemon tasks, replay events, retrieve results, cancel work, and recover artifacts;
- persist assessments, findings, evidence, artifacts, baselines, checkpoints, and reports;
- compare assessments and generate stable output formats without application-specific storage infrastructure.

## Scope

### Mandatory areas

1. mobile device and dynamic-analysis session lifecycle;
2. browser assessment session lifecycle;
3. local/daemon request, result, event, cancellation, timeout, and artifact parity;
4. reconnect, replay, idempotency, and daemon restart semantics;
5. persistent finding and assessment repositories;
6. content-addressed and directory-backed artifact stores;
7. baseline comparison and resumption;
8. reporting parity and streaming output;
9. typing, packaging, fixtures, documentation, and release validation.

### Conditional areas

The following remain provisional unless all lifecycle and platform requirements are met:

- invasive mobile instrumentation;
- arbitrary Frida scripts supplied by Python callbacks;
- unrestricted browser JavaScript execution;
- daemon migration across incompatible protocol versions;
- remote privileged packet capture;
- automatic trust-store modification;
- cloud object-store adapters.

### Explicit non-goals

Release 4 does not complete:

- public 1.0 stabilization;
- agent framework adapters;
- final namespace cleanup;
- broad plugin SDK stabilization;
- all specialized lab domains;
- unrestricted remote execution outside Eggsec’s operation policy model.

## Architectural principles

- Mobile, browser, proxy, database, and NSE sessions should share consistent lifecycle semantics.
- The daemon is a transport for the same operation contract, not a separate API family.
- Persistent stores hold stable DTOs, schema versions, hashes, and references—not Rust implementation objects.
- Remote reconnect and replay semantics must be explicit and testable.
- Secrets, private keys, credentials, cookies, tokens, and sensitive body data remain redacted or separately protected.
- Large data should be streamed or artifact-backed.
- Storage and reporting should reuse native Eggsec libraries where possible rather than duplicate formatting logic in PyO3.

## Workstream 1 — Common managed-session contract

### Goal

Consolidate the lifecycle conventions established in Release 3 into reusable session infrastructure.

### Common contract

Every managed session should define:

- session ID;
- state enum;
- created/started/closed timestamps;
- sync and/or async context-manager protocols;
- explicit start and stop;
- graceful and forced close;
- cancellation token;
- timeout policy;
- event stream;
- statistics;
- artifact store;
- scope and policy identity;
- deterministic cleanup;
- idempotent close;
- structured use-after-close errors.

### Suggested reusable types

- `SessionState`
- `SessionIdentity`
- `SessionStats`
- `SessionCloseMode`
- `SessionEventStream`
- `SessionCapabilities`

### Acceptance criteria

- Mobile and browser sessions reuse shared lifecycle helpers.
- Session behavior does not diverge by domain without documented reason.

## Workstream 2 — Mobile device discovery and capabilities

### Proposed API

- `MobileDeviceRegistry`
- `MobileDevice`
- `MobileDeviceDescriptor`
- `MobileDeviceCapabilities`
- `MobilePlatform`
- `MobileTransport`

### Required capabilities

- discover connected devices and emulators;
- identify platform, version, architecture, device state, transport, and authorization status;
- distinguish physical device versus emulator;
- expose supported operations;
- detect required external tooling;
- return structured unavailable/unauthorized errors;
- refresh device state;
- avoid implicit device selection when multiple devices exist.

### Acceptance criteria

- Device discovery is deterministic and side-effect free.
- Capability metadata matches actual available operations.

## Workstream 3 — Mobile session lifecycle

### Proposed API

- `MobileSessionConfig`
- `MobileSession`
- `AsyncMobileSession`
- `MobileSessionState`
- `MobileSessionStats`

### Required behavior

- bind to explicit device;
- start and stop session;
- install or select application;
- launch and stop application;
- optional package removal;
- log streaming;
- screenshot capture;
- network capture integration;
- filesystem artifact extraction;
- process metadata;
- bounded storage;
- cancellation;
- timeout;
- deterministic cleanup;
- restore or document device state changes.

### Safety requirements

- destructive device actions require explicit elevated policy;
- package uninstall and data clearing are never implicit;
- device identifiers and extracted secrets are redacted appropriately;
- temporary artifacts are cleaned reliably.

## Workstream 4 — Mobile static/dynamic convergence

### Goal

Unify static APK/IPA analysis with dynamic session context.

### Required behavior

- static analysis result can seed dynamic plan;
- package IDs, permissions, URLs, certificates, and indicators can be referenced by later steps;
- dynamic findings link back to static evidence;
- shared artifact and finding schemas;
- no duplicate package metadata models;
- `analyze_apk` and `analyze_ipa` remain canonical bounded operations.

## Workstream 5 — Mobile instrumentation boundary

### Proposed types

- `InstrumentationConfig`
- `InstrumentationScript`
- `InstrumentationEvent`
- `InstrumentationResult`

### Requirements

- approved built-in instrumentation modules;
- explicit script source identity and hash;
- timeout and output limits;
- cancellation;
- event streaming;
- structured diagnostics;
- no secret leakage;
- no arbitrary callback portability claim.

Arbitrary user-provided instrumentation remains experimental until sandbox, cleanup, and platform behavior are reliable.

## Workstream 6 — Mobile evidence and artifacts

Capture:

- screenshots;
- logs;
- network traces;
- application files;
- process metadata;
- runtime permissions;
- dynamic API observations;
- crash traces;
- instrumentation output.

Large data must be artifact-backed with hashes, content type, origin, device identity, redaction state, and timestamps.

## Workstream 7 — Browser runtime and capability inventory

### Goal

Define the intended browser security-assessment boundary without becoming a general browser automation framework.

### Required inventory

Document:

- browser engine support;
- executable discovery;
- profile lifecycle;
- navigation;
- network events;
- console events;
- DOM snapshots;
- route discovery;
- cookies and storage;
- screenshots;
- script execution;
- download handling;
- proxy integration;
- TLS metadata;
- current native/browser crate coupling.

Deliver `docs/python/BROWSER_SESSION_ARCHITECTURE.md`.

## Workstream 8 — Browser session lifecycle

### Proposed API

- `BrowserSessionConfig`
- `BrowserSession`
- `AsyncBrowserSession`
- `BrowserSessionState`
- `BrowserSessionStats`
- `BrowserCapabilities`

### Required behavior

- discover supported browser runtime;
- create isolated temporary profile;
- start/stop;
- sync and async context managers where supported;
- navigate;
- wait conditions;
- cancellation;
- timeout;
- deterministic profile cleanup;
- proxy configuration;
- artifact store;
- event stream;
- no persistent global browser state by default.

### Acceptance criteria

- Repeated session lifecycle does not leak browser processes or profiles.
- Failure during launch or navigation cleans up all resources.

## Workstream 9 — Browser security primitives

### Required capabilities

- navigate to scoped URL;
- inspect final URL and redirect chain;
- collect DOM snapshot;
- discover forms, links, scripts, frames, and routes;
- capture console events;
- capture network request/response metadata;
- inspect cookies;
- inspect local/session storage;
- inspect service workers where supported;
- collect TLS and security-header metadata;
- take screenshots;
- export artifacts;
- bounded controlled script evaluation.

### Script execution boundary

Arbitrary JavaScript evaluation must be explicit, policy-governed, time-bounded, output-bounded, and provisional. Stable APIs should prefer named inspection primitives.

## Workstream 10 — Browser event and network correlation

### Proposed types

- `BrowserNavigationEvent`
- `BrowserConsoleEvent`
- `BrowserNetworkEvent`
- `BrowserDomEvent`
- `BrowserDownloadEvent`
- `BrowserSecurityObservation`

### Requirements

- correlate events with navigation and frame IDs;
- preserve monotonic sequence;
- bound event queues;
- externalize large bodies;
- redact cookies, authorization, and sensitive storage values;
- correlate browser traffic with proxy captures where both are used.

## Workstream 11 — Browser assessment operation convergence

Refactor existing browser assessment functions to use the managed browser session implementation.

Potential canonical operations:

- `browser_assess`;
- `browser_route_discovery`;
- `browser_storage_audit`.

Promote only operations that satisfy the full registry graduation checklist. Keep general session APIs provisional until platform support is validated.

## Workstream 12 — Daemon protocol inventory and versioning

### Goal

Define a transport-independent operation protocol shared by local and daemon engines.

### Required protocol components

- protocol version;
- API schema version;
- capability negotiation;
- operation registry identity;
- feature profile;
- request envelope;
- result envelope;
- event envelope;
- artifact descriptor;
- cancellation request;
- result retrieval request;
- replay cursor;
- heartbeat;
- error envelope;
- checkpoint identity.

### Deliverable

Add `docs/python/DAEMON_PARITY_PROTOCOL.md`.

## Workstream 13 — Unified engine construction

### Proposed API

```python
Engine.local(...)
AsyncEngine.local(...)
Engine.daemon(...)
AsyncEngine.daemon(...)
```

or equivalent constructors sharing one interface.

### Requirements

- same operation request DTOs;
- same result payload DTOs;
- same structured errors;
- same operation descriptors;
- same policy metadata;
- same event schemas;
- explicit transport metadata;
- no daemon-only semantic result shape.

## Workstream 14 — Daemon submission and idempotency

### Required behavior

- client-generated idempotency key;
- deterministic request hash;
- at-most-once logical submission guarantee where supported;
- safe retry after connection loss;
- duplicate-submission response;
- task/session identity;
- submission audit metadata;
- structured rejection for incompatible capability profiles.

### Acceptance criteria

- A network retry cannot silently launch duplicate work.
- Duplicate behavior is documented and tested.

## Workstream 15 — Reconnect and result retrieval

### Required behavior

- reconnect by task/session ID;
- retrieve current state;
- retrieve terminal result;
- retrieve partial result where supported;
- retrieve artifacts lazily;
- distinguish unknown, expired, pruned, and unauthorized tasks;
- survive client restart;
- bounded retry/backoff;
- explicit server restart semantics.

## Workstream 16 — Event replay and ordering

### Required behavior

- monotonic sequence numbers;
- replay from sequence/cursor;
- duplicate detection;
- gap detection;
- explicit ordering guarantees;
- reliable terminal event;
- event retention metadata;
- backpressure on live subscriptions;
- transition from replay to live stream without race.

### Acceptance criteria

- Reconnect does not lose or duplicate semantically significant events without detection.

## Workstream 17 — Remote cancellation and timeout semantics

### Requirements

- cancellation request acknowledgment;
- cancellation race behavior;
- terminal outcome precedence;
- client timeout versus server timeout distinction;
- disconnect does not implicitly cancel unless configured;
- server-side cleanup;
- cancellation reason preservation;
- audit trail.

## Workstream 18 — Daemon artifact parity

### Proposed behavior

- artifact descriptor parity with local engine;
- lazy download;
- ranged or streaming retrieval where useful;
- integrity hash verification;
- content type and size;
- expiration/retention metadata;
- authorization checks;
- redaction metadata;
- local cache controls;
- cancellation during download.

## Workstream 19 — Local/daemon contract tests

Run the same operation contract suite against local and daemon engines for every declared daemon-stable operation.

Validate:

- request normalization;
- policy denial;
- feature unavailable;
- success payload;
- error payload;
- timeout;
- cancellation;
- event ordering;
- artifact metadata;
- serialization;
- checkpoint identity;
- direct/engine equivalence where applicable.

Do not claim daemon parity for operations lacking this matrix.

## Workstream 20 — Finding repository abstraction

### Proposed API

- `FindingRepository`
- `AsyncFindingRepository`
- `FindingQuery`
- `FindingPage`
- `FindingTransaction`

### Required capabilities

- insert/update;
- get by ID;
- query by assessment, severity, state, type, target, tags, time;
- pagination;
- deduplication key;
- transactions;
- optimistic concurrency or explicit conflict behavior;
- schema version;
- migration;
- retention and pruning;
- secret-safe storage.

## Workstream 21 — Assessment repository

### Proposed API

- `AssessmentRepository`
- `AssessmentRecord`
- `AssessmentQuery`
- `AssessmentState`
- `AssessmentResumeInfo`

### Required behavior

- create assessment;
- update lifecycle state;
- attach findings/evidence/artifacts;
- preserve operation and pipeline identity;
- store checkpoint references;
- resume metadata;
- query and pagination;
- delete/prune policy;
- audit metadata.

## Workstream 22 — Repository implementations

### Required implementations

- in-memory;
- SQLite;
- JSONL where append-oriented export is useful.

### Optional implementations

- directory-indexed repository;
- pluggable protocol for application-defined stores.

### Requirements

- identical semantic contract;
- migration tests;
- concurrent access behavior;
- corruption detection;
- atomic writes;
- deterministic ordering;
- installed-wheel tests.

## Workstream 23 — Artifact stores

### Proposed API

- `ArtifactStore`
- `DirectoryArtifactStore`
- `ContentAddressedArtifactStore`
- `ArtifactQuery`
- `ArtifactIntegrityResult`

### Required behavior

- put/get/delete;
- streaming write/read;
- content hash;
- deduplication;
- metadata;
- atomic finalization;
- integrity verification;
- retention/pruning;
- path traversal protection;
- size limits;
- redaction classification;
- optional encryption hook without inventing custom cryptography.

## Workstream 24 — Baselines and comparison

### Proposed API

- `BaselineRepository`
- `AssessmentBaseline`
- `AssessmentComparator`
- `AssessmentDiff`
- `FindingCorrelation`

### Required behavior

- save baseline;
- select baseline;
- compare findings;
- correlate by stable identifiers and evidence;
- classify new, resolved, changed, unchanged;
- severity and confidence changes;
- artifact references;
- schema compatibility;
- deterministic output.

## Workstream 25 — Checkpoint and resume persistence

### Requirements

- persist pipeline and operation identities;
- persist completed step results;
- persist artifact references;
- persist session reconstruction metadata;
- never persist open handles or secret values;
- migration and compatibility checks;
- resume after process restart;
- daemon/local compatibility where declared;
- atomic checkpoint writes.

## Workstream 26 — Reporting parity

### Required formats

- JSON;
- JSONL;
- CSV;
- Markdown;
- HTML;
- SARIF;
- native report envelope;
- streaming output where appropriate.

### Requirements

- one reusable native formatting source where possible;
- schema versions;
- deterministic ordering;
- redaction;
- artifact references;
- baseline/diff reports;
- partial assessment reports;
- daemon and local parity;
- no large in-memory aggregation requirement for streaming formats.

## Workstream 27 — Reporting and repository integration

Allow reports to read directly from repository queries without requiring users to materialize all findings in Python memory.

Support:

- filters;
- pagination;
- streaming rows;
- assessment metadata;
- baseline comparison;
- artifact links;
- report manifests;
- output integrity hashes.

## Workstream 28 — Common events and observability

Extend governed events for:

- mobile device/session lifecycle;
- browser lifecycle/navigation/network/console;
- daemon connection/reconnect/replay;
- repository writes/migrations/pruning;
- artifact transfers;
- report generation.

Preserve secret exclusion, bounded queues, reliable terminal events, and correlation IDs.

## Workstream 29 — Typing and ergonomics

Provide:

- complete `.pyi` files;
- sync/async repository protocols;
- context managers;
- async context managers;
- iterators and async iterators;
- typed query/filter objects;
- `pathlib.Path` and datetime support;
- generic page/result types where useful;
- mypy and pyright examples;
- deliberate namespace exports.

## Workstream 30 — Packaging profiles

Validate feature profiles for:

- mobile;
- mobile-dynamic;
- headless-browser;
- daemon-client;
- storage/reporting default profile;
- compatible combined profiles.

For each:

- wheel build;
- installed-wheel tests;
- dependency diagnostics;
- binary-size report;
- platform matrix;
- feature metadata;
- external runtime discovery behavior.

## Workstream 31 — Documentation and examples

### Required guides

- mobile device and session lifecycle;
- static/dynamic mobile convergence;
- browser session architecture;
- browser security primitives;
- daemon protocol and parity guarantees;
- reconnect, replay, cancellation, and artifacts;
- repositories and migrations;
- artifact stores;
- baselines and comparisons;
- checkpoints and resume;
- reporting formats and streaming.

### Required examples

- dynamic mobile session with logs and screenshots;
- static-to-dynamic mobile workflow;
- browser route/storage audit;
- browser network and console capture;
- local versus daemon execution using same request;
- reconnect and replay;
- remote cancellation;
- SQLite repository;
- content-addressed artifact store;
- baseline comparison;
- SARIF and HTML report generation.

## Validation plan

### Mobile

Validate device discovery, unauthorized devices, launch/stop, logs, screenshots, capture, extraction, cancellation, cleanup, malformed applications, and supported platforms.

### Browser

Validate runtime discovery, profile isolation, launch failure cleanup, navigation, redirects, DOM, routes, console, network, cookies, storage, screenshots, cancellation, and process cleanup.

### Daemon

Validate capability negotiation, idempotent submission, reconnect, result retrieval, replay, duplicate/gap handling, cancellation races, restart semantics, artifact integrity, and local parity.

### Storage/reporting

Validate migrations, concurrency, atomicity, corruption behavior, pagination, deduplication, retention, artifact integrity, baseline comparison, deterministic reports, redaction, and large streaming datasets.

## Performance and resource budgets

Track:

- mobile session startup and artifact throughput;
- browser startup, navigation overhead, and process memory;
- daemon submission and event latency;
- reconnect/replay throughput;
- artifact transfer throughput;
- SQLite query and write throughput;
- artifact-store deduplication efficiency;
- report generation memory and throughput;
- repeated session/process leak checks.

## Recommended implementation sequence

1. common managed-session contract;
2. mobile discovery and session lifecycle;
3. mobile static/dynamic convergence and evidence;
4. browser architecture and lifecycle;
5. browser security primitives and operation convergence;
6. daemon protocol/versioning;
7. unified engine constructors;
8. idempotency, reconnect, replay, cancellation, artifacts;
9. local/daemon contract matrix;
10. repository interfaces and SQLite implementation;
11. artifact stores;
12. baselines, checkpoints, and resume;
13. reporting parity and streaming;
14. typing, packaging, documentation, and closure.

## Release acceptance criteria

Release 4 is complete only when:

- mobile devices and sessions have explicit capabilities, bounded lifecycle, artifacts, events, and deterministic cleanup;
- browser sessions expose security-focused navigation, DOM, route, console, network, cookie, storage, screenshot, and artifact primitives;
- existing mobile/browser bounded operations delegate through common session implementations where appropriate;
- local and daemon engines accept the same stable request DTOs and return the same payload/error schemas;
- daemon submission, idempotency, reconnect, replay, result retrieval, cancellation, timeout, restart, and artifact semantics are explicit and tested;
- every daemon-stable operation passes the same contract suite locally and remotely;
- in-memory, SQLite, and selected export repositories satisfy one semantic contract;
- directory and content-addressed artifact stores support streaming, integrity, retention, and safe paths;
- baselines, comparisons, checkpoints, and resume work across process restarts;
- reporting formats are deterministic, redacted, artifact-aware, and streamable where appropriate;
- feature-specific installed wheels pass supported-platform validation;
- public maturity documentation distinguishes stable operations from provisional session and daemon surfaces;
- Releases 1–3 guarantees remain intact.