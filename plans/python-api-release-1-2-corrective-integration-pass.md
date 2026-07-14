# Eggsec Python API Release 1/2 Corrective Integration Pass

## Handoff objective

This pass validates and tightens the six implementation commits that followed the Python API completion roadmap and Release 1/2 plans.

The repository now contains a substantial implementation of both releases:

- the stable operation registry expanded from 10 to 22 operations;
- twelve provisional domains were promoted into the canonical engine;
- pipelines gained dependencies, parallelism, retries, failure policy, timeouts, and richer checkpoints;
- new network, transport, probe, HTTP, WebSocket, packet, flow, event, and active-probe primitives were added;
- type stubs, capability metadata, documentation, and tests were expanded.

The implementation is broad enough that the highest-value next step is not more feature expansion. It is an evidence-driven corrective and integration pass that verifies the release claims, removes semantic inconsistencies, prevents overstatement of maturity, and leaves Release 1 conclusively closed while Release 2 remains a clearly bounded provisional API.

## Primary outcomes

At completion:

- every one of the 22 stable operations has independently verified contract coverage;
- maturity metadata reflects observed capability rather than implementation intent;
- sync and async APIs obey correct Python protocols;
- DNS, redirect, proxy, reconnect, and active-probe scope enforcement is demonstrably correct;
- packet capture and privileged probes distinguish parser, synthetic, loopback, and live-platform validation;
- operation dispatch is registry-driven enough to avoid permanent engine monolith growth;
- the capability manifest records actual graduation evidence;
- CI publishes a canonical feature/profile matrix;
- wheel, type, performance, memory, cancellation, and cleanup checks are reproducible;
- Release 1 can be treated as closed with high confidence;
- Release 2 remains provisional with explicit per-surface blockers and graduation criteria.

## Scope

### In scope

- independent validation of Release 1 and Release 2 claims;
- stable-operation maturity audit;
- engine/registry architecture tightening;
- direct-function/engine equivalence;
- async protocol correctness;
- network scope and policy correctness;
- capture and active-probe validation;
- capability manifest enrichment;
- CI/profile matrix cleanup;
- installed-wheel testing;
- binary-size and resource budgets;
- documentation correction;
- test-count and validation-report normalization.

### Out of scope

- Release 3 NSE runtime completion;
- interception proxy lifecycle expansion;
- stateful database session API expansion;
- daemon parity work;
- mobile dynamic or browser lifecycle expansion;
- additional operation promotions;
- public 1.0 compatibility guarantees;
- new offensive capabilities.

Do not begin Release 3 implementation until this pass is complete.

## Workstream 1 — Canonical validation matrix

### Goal

Replace commit-message validation claims with one reproducible, machine-readable release matrix.

### Required profiles

At minimum validate:

1. default;
2. `full-no-system`;
3. `websocket`;
4. `git-secrets`;
5. `sbom`;
6. `db-pentest`;
7. `nse`;
8. `container`;
9. `mobile`;
10. `packet-inspection` parser-only;
11. `packet-inspection` privileged/live where supported;
12. combined `websocket,packet-inspection`;
13. installed default wheel;
14. installed broad non-system wheel.

### Required checks per profile

Record explicitly:

- `cargo check`;
- `cargo test`;
- extension build;
- wheel build;
- installed-wheel import test;
- installed-wheel pytest;
- export/stub parity;
- capability manifest validation;
- architecture guards;
- mypy;
- pyright;
- documentation example execution where applicable;
- platform and Python version.

### Deliverables

Add a script such as:

```text
scripts/validate_python_release_1_2.sh
```

and emit a JSON report such as:

```text
target/python-validation/release-1-2-matrix.json
```

The script should support:

- local developer mode;
- CI mode;
- parser-only mode;
- privileged mode;
- explicit profile selection;
- fail-fast or full-report behavior.

### CI integration

Update the Python wheel workflow or add a dedicated workflow that publishes:

- profile name;
- target platform;
- Python version;
- feature set;
- test count;
- passed/failed/skipped counts;
- skipped-test reasons;
- wheel artifact identity;
- binary size;
- validation report artifact.

### Acceptance criteria

- The latest commit receives visible status checks.
- Validation no longer depends on prose in commit messages.
- Test counts are consistent and explain differences between profiles.
- Stable-operation tests cannot be silently skipped.

## Workstream 2 — Stable-operation graduation audit

### Goal

Verify whether all twelve promoted operations deserve stable classification.

### Operations under audit

- `scan_git_secrets`
- `generate_sbom`
- `run_consolidated_recon`
- `graphql_test`
- `oauth_test`
- `auth_test`
- `db_probe`
- `nse_run`
- `scan_docker_image`
- `scan_kubernetes`
- `analyze_apk`
- `analyze_ipa`

### Required evidence per operation

The audit must record:

- canonical operation ID;
- request DTO;
- payload DTO;
- direct-function delegation status;
- sync engine success fixture;
- async engine success fixture;
- feature-unavailable behavior;
- request validation;
- scope or path-policy behavior;
- policy denial/confirmation behavior;
- audit event;
- lifecycle event sequence;
- timeout behavior;
- cancellation behavior;
- partial-result behavior where applicable;
- structured error mapping;
- serialization round-trip;
- secret-sentinel exclusion;
- installed-wheel test;
- supported-platform test;
- deterministic cleanup;
- documentation and stub parity.

### Classification rule

An operation remains stable only if all mandatory evidence is present and reproducible.

If not, choose one of:

- fix the missing contract;
- reclassify the operation as provisional;
- narrow the stable guarantee to a specific execution mode or artifact type.

Do not preserve a stable label merely to keep the registry count at 22.

### Domain-specific scrutiny

#### Database

Verify:

- `SensitiveString` coverage for every credential-bearing field;
- no secret leakage in repr, events, errors, reports, checkpoints, or transcripts;
- read-only behavior by default;
- driver-specific feature errors;
- connection timeout and cancellation;
- deterministic fixture coverage for every claimed stable driver.

#### NSE

Verify:

- script classification and confirmation gates;
- sandbox policy propagation;
- target scope enforcement;
- cancellation/timeout behavior;
- deterministic script fixture;
- feature-gated wheel coverage.

#### Container

Separate:

- local image/artifact analysis;
- Docker daemon access;
- Kubernetes API access;
- cluster credential behavior.

Do not classify all modes as stable based on a local DTO test.

#### Mobile

Separate:

- APK/IPA static artifact parsing;
- external tool invocation;
- dynamic/device behavior.

The stable claim should cover only deterministic static analysis unless broader behavior is actually validated.

## Workstream 3 — Capability manifest enrichment

### Goal

Turn `_capabilities.json` into a real graduation and availability ledger rather than a coarse domain list.

### Required per-operation fields

Add or generate:

- `operation_id`;
- `domain`;
- `maturity`;
- `request_type`;
- `payload_type`;
- `schema_version`;
- `cargo_feature`;
- `default_wheel`;
- `sync_dispatch`;
- `async_dispatch`;
- `direct_function`;
- `direct_function_delegates`;
- `policy`;
- `scope`;
- `audit`;
- `events`;
- `timeout`;
- `cancellation`;
- `fixture`;
- `serialization`;
- `secret_sentinel`;
- `stub`;
- `installed_wheel`;
- `supported_platforms`;
- `known_blockers`;
- `last_validated_commit`.

### Generation and drift prevention

Prefer deriving fields from Rust metadata and validation output where possible.

The matrix checker must fail when:

- a stable operation has any mandatory false/unknown field;
- a manifest operation is absent from the registry;
- a registry operation is absent from the manifest;
- a stub or export is missing;
- maturity docs disagree with runtime metadata;
- a feature-gated operation claims default-wheel availability incorrectly.

### Documentation

Generate `docs/python/API_CAPABILITY_MATRIX.md` from the machine-readable source. Avoid manually maintaining two independent truth tables.

## Workstream 4 — Registry and dispatch architecture tightening

### Goal

Prevent `engine.rs` and `async_engine.rs` from becoming permanent operation-specific monoliths.

### Required review

Map each stable operation across:

- metadata registration;
- request conversion;
- sync executor;
- async executor;
- payload conversion;
- event conversion;
- feature check.

Identify duplicated match arms and per-domain branching in:

- `engine.rs`;
- `async_engine.rs`;
- `operation_registry.rs`;
- `requests.rs`;
- `status.rs`.

### Target architecture

Each operation should be represented by a registry-owned executor descriptor or domain adapter containing:

- operation constant;
- metadata;
- feature predicate;
- request validator/converter;
- sync executor;
- async executor;
- payload converter;
- event/finding/artifact hooks.

A small top-level dispatch remains acceptable, but operation-specific behavior should not be duplicated across several large matches.

### Constraints

- preserve public API compatibility;
- preserve operation ordering;
- preserve unknown-operation suggestions;
- preserve policy and audit sequencing;
- preserve typed payloads;
- avoid trait-object complexity unless it materially reduces duplication;
- benchmark dispatch overhead before and after.

### Acceptance criteria

- Adding a new operation requires one registry/domain adapter path, not edits across many central match blocks.
- Sync and async metadata cannot drift.
- Architecture guards enforce descriptor/executor completeness.

## Workstream 5 — Direct-function and engine equivalence

### Goal

Ensure every compatibility function delegates to the common engine semantics.

### Required tests

For every stable operation compare direct and engine paths for:

- success payload type;
- result schema version;
- error code;
- feature-unavailable error;
- scope denial;
- policy confirmation;
- timeout;
- cancellation where direct APIs expose it;
- serialization;
- audit/event behavior where supported.

### Implementation rule

If a direct function cannot delegate because its signature or return shape differs, implement an explicit compatibility adapter over the engine result. Do not retain a second native execution path.

### Acceptance criteria

- No stable direct function invokes a domain executor independently of `Engine`/`AsyncEngine`.
- Equivalence tests cover all 22 operations.

## Workstream 6 — Python async protocol correctness

### Goal

Verify that every object named or documented as async behaves as a native Python asynchronous API.

### Objects under review

- `AsyncEngine`;
- `AsyncPipeline`;
- `AsyncTcpSession`;
- `AsyncUdpSocket`;
- `AsyncHttpClient`;
- `AsyncWebSocketSession`;
- `AsyncCaptureSession`;
- HTTP body streams;
- WebSocket message streams;
- packet streams;
- event streams.

### Required behavior

Where applicable:

- `async def` methods return awaitable results;
- `__aenter__` and `__aexit__` are implemented;
- `__aiter__` and `__anext__` are implemented;
- cancellation propagates through awaiting tasks;
- no hidden blocking work runs under the GIL;
- no nested Tokio runtime is created per call;
- sync façades remain separate and clearly named;
- double close is deterministic;
- use-after-close produces structured errors;
- exceptions during iteration trigger cleanup.

### Stub audit

The `.pyi` files must reflect actual awaitability. Avoid `Any` where a concrete awaitable or async iterator can be expressed.

### Tests

Add tests using real `asyncio`:

- `async with` session lifecycle;
- `async for` message/body/packet iteration;
- task cancellation;
- timeout via `asyncio.wait_for`;
- concurrent use;
- close during blocked read;
- callback exception cleanup;
- no event-loop blocking under synthetic load.

## Workstream 7 — Scope enforcement across network transitions

### Goal

Prove that low-level network primitives cannot bypass scope through resolution, redirect, proxy, reconnect, or protocol upgrade.

### Required policy model

Define and test authorization for:

1. original hostname or URL;
2. each resolved IP address;
3. DNS re-resolution on retry;
4. HTTP redirect destination;
5. WebSocket redirect or upgrade destination;
6. proxy endpoint;
7. tunneled destination;
8. reconnect destination;
9. IPv4/IPv6 address-family changes;
10. SNI/Host authority mismatch.

### Required cases

- allowed hostname resolving only to allowed addresses;
- allowed hostname resolving partly out of scope;
- out-of-scope IP returned after initial resolution;
- redirect from allowed to denied host;
- redirect from allowed path to denied port;
- proxy allowed but destination denied;
- proxy denied but destination allowed;
- DNS answer changes between retries;
- IPv6 fallback outside scope;
- WebSocket upgrade to a different authority;
- HTTP connection reuse across authorities;
- hostname permitted but raw IP denied, and vice versa.

### Resolution timing

Correct documentation that implies scope can always be checked before DNS contact. Distinguish:

- authorization to perform DNS resolution;
- authorization to connect to resolved addresses.

### Acceptance criteria

- Every network contact is preceded by the correct stage-specific authorization check.
- Redirects and reconnects cannot inherit authorization from the original target blindly.
- Tests use managed local fixtures and deterministic fake resolvers where needed.

## Workstream 8 — HTTP and WebSocket correctness pass

### Goal

Validate the new HTTP and WebSocket layers as reusable security-oriented primitives.

### HTTP requirements

Verify:

- duplicate ordered headers;
- redirect history;
- cookie persistence and redaction;
- decompression controls;
- response-size limits;
- truncated-response indication;
- body streaming;
- cancellation during body read;
- connection reuse;
- per-host concurrency;
- TLS metadata;
- proxy routing;
- secret-safe repr/transcripts;
- no cross-origin credential leakage on redirects;
- no body buffering beyond configured limits.

### WebSocket requirements

Verify:

- `ws` and `wss`;
- custom headers and cookies;
- origin;
- subprotocol negotiation;
- text and binary messages;
- ping/pong;
- close handshake;
- maximum message size;
- cancellation during receive;
- async iteration;
- transcript redaction;
- deterministic cleanup;
- assessment operation policy and events.

### Stability

Keep Release 2 network and WebSocket surfaces provisional unless their full contract and wheel profiles are validated.

## Workstream 9 — Packet parser, capture, and flow validation

### Goal

Separate model-level completeness from actual capture correctness.

### Validation layers

#### Layer A — Pure parser

Run without privileges using fixed byte and PCAP fixtures:

- Ethernet;
- VLAN;
- IPv4;
- IPv6;
- TCP;
- UDP;
- ICMP;
- DNS;
- selected TLS record metadata;
- malformed/truncated packets;
- unsupported protocols;
- checksum edge cases where applicable.

Parsers must never panic.

#### Layer B — Synthetic stream and backpressure

Validate:

- bounded queue behavior;
- block/drop-oldest/drop-newest/artifact-only policies;
- per-policy drop accounting;
- slow consumer;
- producer error;
- consumer exception;
- cancellation;
- terminal statistics;
- repeated start/stop;
- double stop;
- artifact thresholds.

#### Layer C — Loopback capture

Where supported, capture deterministic loopback TCP/UDP/ICMP traffic and verify packet/flow correlation.

#### Layer D — Privileged live capture

Run in explicit CI or documented local validation environments. Record platform, privileges, interface, packet counts, and unsupported behavior.

### Flow aggregation

Verify:

- five-tuple identity;
- reverse-flow handling;
- eviction policy;
- memory bound;
- timestamp updates;
- TCP flag accumulation;
- malformed packet exclusion;
- serialization.

### Acceptance criteria

- Parser-only support is independently releasable from live capture.
- Live capture is not claimed stable on unvalidated platforms.
- Drop statistics match observed queue behavior.

## Workstream 10 — Active probe hardening

### Goal

Ensure ICMP, TCP SYN, UDP reachability, and traceroute primitives remain controlled, portable, and honest about platform support.

### Required behavior

- explicit privilege detection;
- structured unsupported-platform errors;
- IPv4 and IPv6 behavior documented separately;
- scope checks on every target/address;
- rate limits;
- retry limits;
- cancellation;
- timeout;
- response correlation;
- no spoofed source support in stable APIs;
- cleanup of raw sockets;
- bounded target counts;
- audit and event metadata.

### Tests

- loopback success where supported;
- closed-port behavior;
- timeout behavior;
- cancellation during wait;
- privilege denial;
- unsupported platform;
- IPv4/IPv6 differences;
- rate-limit enforcement;
- scope denial before send;
- malformed or unrelated response correlation.

### Classification

Keep raw packet construction/injection experimental. Active probes should remain provisional until platform and privilege matrices are complete.

## Workstream 11 — Cancellation, timeout, and resource cleanup

### Goal

Prove that interruption does not leak tasks, sockets, files, captures, processes, or temporary artifacts.

### Required scenarios

- cancellation before dispatch;
- cancellation during DNS resolution;
- cancellation during connect;
- cancellation during TLS handshake;
- cancellation during HTTP headers;
- cancellation during body stream;
- cancellation during WebSocket receive;
- cancellation during capture;
- cancellation during active probe;
- pipeline cancellation with parallel steps;
- timeout racing with explicit cancellation;
- callback failure;
- exception during context-manager exit.

### Leak checks

Where practical measure:

- open file descriptors;
- active Tokio tasks;
- temporary files/directories;
- capture handles;
- connection pool entries;
- queue depth;
- memory before/after repeated cancellation cycles.

### Error semantics

Distinguish clearly:

- explicit cancellation;
- operation deadline;
- per-stage timeout;
- idle timeout;
- consumer abandonment;
- transport failure.

## Workstream 12 — Type and export correctness

### Goal

Make the large public surface mechanically trustworthy.

### Required checks

- extension export versus `__init__.py`;
- extension export versus `.pyi`;
- façade alias versus `.pyi`;
- feature-gated symbol behavior;
- async method signatures;
- iterator/async iterator protocols;
- context-manager protocols;
- `Literal` operation IDs;
- typed payload access;
- no public `Any` where a stable DTO exists;
- `py.typed` presence;
- mypy example suite;
- pyright example suite.

### Script improvement

Enhance `scripts/check_python_types.sh` so it:

- builds or installs the target wheel;
- checks the actual extension;
- validates stubs under multiple feature profiles;
- reports missing, extra, or signature-mismatched symbols;
- fails CI on mismatch.

## Workstream 13 — Binary size and dependency profile

### Goal

Quantify the cost of the new default-wheel dependencies and feature profiles.

### Measure

For each wheel profile:

- compressed wheel size;
- installed extension size;
- transitive native dependency count;
- build duration;
- import time;
- stripped symbol state;
- platform-specific dynamic dependencies.

### Particular scrutiny

- `reqwest` in the default profile;
- WebSocket feature additions;
- packet-inspection system dependencies;
- duplicate HTTP/TLS stacks already present elsewhere in Eggsec;
- whether common native HTTP infrastructure can be reused instead of duplicated.

### Budget

Set initial non-binding budgets and record regressions. Do not optimize prematurely, but do not allow size growth to remain invisible.

## Workstream 14 — Performance and memory validation

### Goal

Replace microbench claims with representative budgets.

### Benchmarks

- engine dispatch for original 10 versus promoted 12;
- direct-function adapter overhead;
- pipeline scheduler overhead;
- parallel pipeline scaling;
- HTTP request overhead versus native Rust path;
- connection pool reuse;
- WebSocket message throughput;
- packet decode throughput;
- packet stream Python delivery throughput;
- flow aggregation throughput;
- event creation and delivery;
- transcript growth;
- cancellation latency;
- memory under slow consumers;
- repeated session open/close.

### Requirements

- use stable benchmark fixtures;
- avoid fragile sub-second absolute limits where CI variance is high;
- prefer regression ratios and generous hard ceilings;
- publish benchmark artifacts;
- distinguish Rust-core time from Python boundary overhead.

## Workstream 15 — Documentation and maturity correction

### Goal

Align all user-facing claims with validated behavior.

### Files to review

- `README.md`;
- `architecture/python_api.md`;
- `crates/eggsec-python/README.md`;
- `docs/python/domain-maturity.md`;
- `docs/python/API_CAPABILITY_MATRIX.md`;
- `docs/python/api-reference.md`;
- `docs/python/network-programmability.md`;
- `.opencode/skills/eggsec-python/SKILL.md`;
- examples;
- release checklist files.

### Required corrections

- distinguish stable operation API from provisional low-level primitives;
- distinguish default-wheel availability from API stability;
- distinguish parser-only packet support from live capture;
- document privilege and platform requirements;
- document actual async protocols;
- document DNS and redirect scope semantics accurately;
- document unsupported-platform errors;
- avoid claiming all Release 2 acceptance criteria are closed until this pass verifies them;
- include canonical validation commands and matrix artifact locations.

## Workstream 16 — Architecture and release guards

### Goal

Prevent regression after the corrective pass.

### Add guards for

- stable operation missing validation metadata;
- registry/executor mismatch;
- direct stable function not delegating;
- sync/async operation list mismatch;
- feature metadata mismatch;
- stable operation with skipped fixture;
- provisional network API accidentally marked stable;
- async-named type lacking async protocols;
- raw injection exposed in default wheel;
- secret-bearing fields using plain strings;
- docs claiming unsupported stability;
- missing installed-wheel profile.

## Recommended implementation sequence

1. canonical validation matrix and CI status publication;
2. capability manifest enrichment;
3. stable-operation graduation audit;
4. direct-function/engine equivalence suite;
5. async protocol audit and fixes;
6. DNS/redirect/proxy/reconnect scope tests and fixes;
7. HTTP/WebSocket correctness pass;
8. packet parser fixture suite;
9. capture/backpressure lifecycle tests;
10. active-probe privilege/platform hardening;
11. cancellation and leak testing;
12. registry/dispatch refactor;
13. type/export validation;
14. binary-size and performance reporting;
15. documentation correction;
16. final release closure report.

The validation matrix should be implemented first so every subsequent fix produces visible, reproducible evidence.

## Suggested commit structure

Use focused commits such as:

1. `test(python): add release 1/2 validation matrix`
2. `chore(python): enrich capability graduation metadata`
3. `test(python): audit 22 stable operation contracts`
4. `fix(python): unify direct and engine operation paths`
5. `fix(python): correct async session and stream protocols`
6. `fix(python): enforce scope across resolution and redirects`
7. `test(python): harden HTTP and WebSocket contracts`
8. `test(python): add packet parser and capture fixture layers`
9. `fix(python): harden privileged active probes`
10. `test(python): verify cancellation and resource cleanup`
11. `refactor(python): consolidate registry-owned executors`
12. `chore(python): close typing and export parity`
13. `perf(python): publish size and runtime budgets`
14. `docs(python): correct release maturity claims`
15. `chore(python): close release 1/2 corrective pass`

Avoid combining broad refactors with maturity reclassification in one opaque commit.

## Final acceptance criteria

This corrective pass is complete only when:

- a visible CI matrix validates every supported profile;
- one canonical report explains test counts, skips, features, platforms, and wheel identities;
- all stable-operation fixtures are non-skipping;
- each of the 22 operations has complete graduation evidence;
- operations lacking evidence are fixed or reclassified;
- direct functions and engine dispatch are semantically equivalent;
- async-named resources support native async context and iteration protocols where applicable;
- DNS, redirects, proxies, reconnects, and upgrades enforce scope at every network transition;
- HTTP and WebSocket transcripts remain secret-safe;
- parser tests cover malformed packet inputs without panics;
- capture queues, backpressure, drops, cancellation, and cleanup are validated;
- active probes have explicit privilege, platform, rate, timeout, cancellation, and scope behavior;
- repeated cancellation and failure do not leak observable resources;
- capability metadata, runtime introspection, exports, stubs, docs, and wheel profiles agree;
- registry/executor architecture no longer requires broad central edits for every operation;
- binary-size and performance reports are published;
- Release 1 is explicitly marked closed;
- Release 2 remains provisional with clear per-surface blockers and future graduation requirements;
- no Release 3 work is required to satisfy this pass.

## Handoff note

Treat existing commit messages as implementation claims to verify, not as acceptance evidence. Preserve the broad work already completed, but prefer reclassification over weakening tests or inventing unsupported guarantees. The correct outcome is a smaller set of defensible stable claims and a precise provisional network surface, not the largest possible list of completed features.