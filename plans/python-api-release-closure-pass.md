# Eggsec Python API Release-Closure Plan

> Status: Local closure pass implemented. The stable-core release contract is
> local `Engine`/`AsyncEngine` only; daemon parity is deferred. TestPyPI upload,
> multi-platform CI evidence, and manual publication remain external gates.

## Purpose

This plan covers the remaining work required to move `eggsec-python` from a scoped pre-1.0 release-candidate state to a defensible first public `0.x` release of the stable core.

The stable-core boundary is intentionally limited to the ten canonical operations already governed by `StableOperation`:

- `scan_ports`
- `scan_endpoints`
- `fingerprint_services`
- `recon_dns`
- `inspect_tls`
- `detect_technology`
- `detect_waf`
- `validate_waf`
- `fuzz_http`
- `load_test`

No additional capability domain should be graduated during this pass. The objective is to close validation, parity, packaging, and publication gates for the existing stable core.

## Current baseline

The current implementation already provides:

- canonical sync/async dispatch through `StableOperation`;
- typed payload preservation in `OperationResult`;
- structured `OperationError` values and exception reconstruction;
- mandatory policy and audit evaluation for stable-core execution;
- monotonic event sequencing;
- reliable delivery classes and backpressure accounting;
- machine-readable domain maturity classification;
- a documented provisional/experimental boundary for all other domains.

The remaining open gates are:

1. deterministic non-skipping fixtures for every stable operation;
2. local/daemon contract parity or an explicit daemon deferral;
3. checkpoint/resume equivalence for stable-core pipelines;
4. comprehensive secret-sentinel coverage;
5. architecture-guard cleanup;
6. clean wheel build/install and installed-wheel smoke tests;
7. visible CI evidence across supported platforms;
8. TestPyPI dry run and release metadata completion.

## Closure-pass decisions

- **Daemon boundary:** Option B. Daemon-client APIs remain provisional and are
  excluded from the first-release stable-core guarantee until transport,
  reconnect, checkpoint portability, and event-replay parity are tested.
- **Fixture policy:** Required stable-core coverage uses managed loopback TCP,
  HTTP, and TLS fixtures plus deterministic `localhost` resolution. The DNS
  operation is covered through the host resolver with the same explicit local
  fixture policy; a public DNS dependency is not part of the required suite.
- **Checkpoint contract:** Checkpoints use schema version 3, atomic sibling
  replacement, compatibility identity fields, typed result restoration, and
  recursive redaction of sensitive keys before persistence.
- **Publication boundary:** Local wheel/profile and installed-wheel checks are
  automated by `scripts/validate_python_release_candidate.sh`. TestPyPI and
  production publication remain manual CI/environment gates and are not
  represented as completed by this repository-only pass.

---

# Workstream 1 — Deterministic stable-core fixture harness

## Objective

Create a hermetic local test harness capable of exercising every stable operation without public internet access, environmental assumptions, or conditional skips.

## Required fixture services

### TCP fixture

Provide a managed local TCP fixture with:

- one known open port;
- one known closed port;
- deterministic banner content;
- configurable accept and response delay;
- explicit startup readiness signal;
- deterministic shutdown;
- IPv4 support as the required baseline;
- IPv6 support where available, with separate non-blocking coverage.

Use this fixture for:

- `scan_ports`;
- `fingerprint_services`;
- timeout behavior;
- cancellation behavior;
- closed-port handling.

### HTTP fixture

Provide a local HTTP service with fixed routes:

- `/` returning a stable response;
- `/admin` returning a distinct status/body;
- `/missing` returning 404;
- `/redirect-local` redirecting to another in-scope local route;
- `/redirect-external` attempting an out-of-scope redirect;
- `/slow` delaying response for cancellation and timeout tests;
- `/echo` reflecting request metadata in a deterministic schema;
- `/waf-clean` producing non-WAF behavior;
- `/waf-block` returning a deterministic blocking signature;
- `/fuzz/{value}` returning deterministic classifications by payload;
- `/load` returning a minimal response suitable for bounded load tests.

Use this fixture for:

- `scan_endpoints`;
- `detect_technology`;
- `detect_waf`;
- `validate_waf`;
- `fuzz_http`;
- `load_test`.

### TLS fixture

Provide a local TLS server with a generated test certificate and predictable properties:

- fixed subject and issuer fields;
- deterministic SAN entries;
- known validity window relative to fixture creation;
- optional self-signed mode;
- optional hostname mismatch mode;
- stable cipher and protocol configuration.

Use this fixture for `inspect_tls`.

### DNS fixture

Provide a local deterministic DNS responder or injectable resolver abstraction with fixed records:

- A;
- AAAA;
- CNAME;
- MX;
- TXT;
- NS;
- SOA;
- CAA;
- NXDOMAIN;
- timeout response.

Use this fixture for `recon_dns`.

## Harness architecture

Implement a reusable fixture manager that:

- allocates ephemeral ports;
- starts services before extension invocation;
- returns target metadata to tests;
- waits for explicit readiness;
- captures service-side request logs;
- exposes deterministic shutdown;
- fails the test if a service cannot start;
- never converts fixture failure into `pytest.skip`.

Prefer a Rust or Python fixture service based on whichever gives the least platform variance. Avoid external containers for the required baseline suite.

## Mandatory assertions by operation

Every stable operation must assert:

- successful policy decision;
- exact payload type;
- meaningful payload contents;
- nonzero or semantically valid execution statistics;
- target metadata;
- stable serialization output;
- sync/async equivalence;
- deterministic error behavior for an invalid or denied case;
- expected event sequence;
- no secret sentinel leakage.

## Acceptance criteria

- No stable-core behavior test uses conditional skip for a required path.
- All ten stable operations execute against controlled local fixtures.
- Fixture startup or execution failure fails the suite.
- Sync and async operations produce equivalent normalized results.
- The suite runs without public network access.

---

# Workstream 2 — Local and daemon contract parity

## Objective

Decide and enforce the daemon support boundary for the first release.

## Decision gate

Choose one of two explicit release positions:

### Option A — daemon included in the release contract

If daemon execution is included, every stable operation declared daemon-capable must satisfy parity tests.

Required parity dimensions:

- request normalization;
- stable operation ID;
- scope and policy decision;
- structured audit event;
- structured error kind and code;
- payload type and normalized payload contents;
- execution status;
- cancellation behavior;
- event ordering;
- artifact metadata;
- serialization schema version;
- timeout behavior;
- reconnect and result retrieval.

### Option B — daemon deferred

If daemon parity cannot be fully closed in this pass:

- mark daemon execution provisional in `domain_maturity()`;
- exclude daemon claims from the first-release stable-core contract;
- state that the stable guarantee applies to local `Engine` and `AsyncEngine` only;
- retain daemon APIs but classify them as provisional;
- remove daemon parity from publication blockers and replace it with a documented follow-up milestone.

Do not leave the boundary ambiguous.

## Daemon fixture requirements

For Option A, add a temporary daemon fixture that:

- uses an isolated temporary database;
- binds an ephemeral Unix socket or local TCP endpoint;
- starts with a deterministic capability profile;
- exposes startup readiness;
- supports clean shutdown;
- leaves no socket or database residue;
- captures server logs for assertion on failure.

## Contract snapshot

Create a normalized contract snapshot for each stable operation containing:

- request schema;
- policy outcome;
- payload type;
- error schema;
- event kinds;
- result schema version.

Run the same snapshot assertions for local and daemon execution.

## Acceptance criteria

- The release boundary explicitly includes or excludes daemon execution.
- If included, parity passes for every declared daemon operation.
- If deferred, documentation, runtime maturity metadata, and release checklist agree.
- No API is described as stable in one document and provisional in another.

---

# Workstream 3 — Pipeline checkpoint and resume equivalence

## Objective

Demonstrate that stable-core pipelines can be checkpointed and resumed without semantic drift.

## Required scenarios

Build deterministic pipeline scenarios covering:

1. two sequential stable operations;
2. one parallel-safe pair;
3. cancellation during a stage;
4. timeout during a stage;
5. failure with fail-fast policy;
6. failure with continue-on-error policy;
7. resume from a completed-stage checkpoint;
8. resume after cancellation;
9. incompatible checkpoint rejection;
10. corrupted checkpoint rejection.

## Equivalence model

Define normalized equivalence over:

- completed stage IDs;
- pending stage IDs;
- operation payloads;
- findings;
- artifacts;
- execution statistics where deterministic;
- policy metadata;
- event sequence categories;
- final terminal status;
- schema versions.

Time-based fields may use bounded comparisons rather than exact equality.

## Checkpoint compatibility checks

A resume attempt must reject checkpoints with mismatched:

- package version where incompatible;
- checkpoint schema version;
- operation schema version;
- target set;
- scope hash;
- execution profile;
- enabled feature set;
- pipeline definition;
- artifact store identity where required.

## Atomicity and persistence

Verify:

- checkpoint writes are atomic;
- partial writes do not produce valid checkpoints;
- temporary files are removed or ignored;
- secrets are not serialized;
- concurrent readers do not observe invalid state;
- completed checkpoints are readable after process restart.

## Acceptance criteria

- Resumed pipelines are semantically equivalent to uninterrupted execution.
- Invalid checkpoints fail with structured errors.
- Checkpoint files contain no secret sentinels.
- Sync and async pipeline resume behavior is equivalent.

---

# Workstream 4 — Comprehensive secret-sentinel validation

## Objective

Prove that secret-bearing values cannot leak through any stable-core or release-support path.

## Sentinel strategy

Use distinctive sentinels such as:

`EGGSEC_SECRET_SENTINEL_7F4B9D2A`

Inject sentinels into every supported secret-bearing location, including:

- proxy authentication;
- HTTP authorization headers;
- database credentials where importable;
- OAuth client secrets;
- integration tokens;
- daemon authentication data;
- AI provider keys;
- remote execution credentials;
- custom metadata fields that are marked sensitive.

## Surfaces to inspect

Assert sentinel absence from:

- `repr()`;
- `str()`;
- Python exceptions;
- `OperationError`;
- `OperationResult.to_dict()`;
- `OperationResult.to_json()`;
- audit events;
- event envelopes;
- progress events;
- checkpoint files;
- daemon request and response logs;
- daemon database rows;
- reporters;
- JSON, JSONL, Markdown, HTML, SARIF, CSV, and PDF where available;
- artifact metadata;
- tracing output;
- test failure diagnostics.

## Type audit

Inventory all secret-bearing fields and require one of:

- `SensitiveString`;
- `SecretReference`;
- a documented non-serializable credential-provider interface.

Plain `String` fields for credentials must be migrated or explicitly blocked from serialization.

## Negative controls

Include tests proving that intentional explicit secret access still works only through the documented method, such as `expose_secret()`, and that this access is not used internally by formatting or serialization.

## Acceptance criteria

- Sentinel scans cover stable-core, daemon, reports, events, checkpoints, and logs.
- No credential field remains an unreviewed plain string.
- Any accepted exception is documented with a security rationale and excluded from release where appropriate.

---

# Workstream 5 — Architecture guard closure

## Objective

Restore repository-wide architecture guards to a consistently passing state.

## Plan-file policy

The repository retains handoff plans intentionally. Update the architecture guard so that it either:

- explicitly permits `plans/*.md`; or
- distinguishes active implementation plans from prohibited generated artifacts;
- documents the accepted retention policy.

Do not delete useful handoff history solely to satisfy an outdated guard.

## NSE assertion guard

Investigate the existing NSE HTTP assertion failure and choose one of:

- correct the assertion if the implementation is wrong;
- update the guard if the intended architecture changed;
- add a narrowly scoped waiver with owner, rationale, and expiration condition.

A permanent undocumented waiver is not acceptable.

## Guard evidence

Add a single command that runs all architecture guards and prints:

- guard name;
- status;
- source file checked;
- waiver identifier if applicable.

## Acceptance criteria

- Repository-wide architecture guards pass on the release candidate.
- Plan retention is an explicit supported policy.
- No broad disable or unconditional success path is introduced.
- Guard results run in CI.

---

# Workstream 6 — Wheel build and clean-environment validation

## Objective

Prove that the distributable package works independently of the source tree and development environment.

## Build profiles

At minimum validate:

### Core wheel

Contains the scoped stable core and has no unsupported system dependency.

### Full-no-system wheel

Contains optional domains that do not require external system libraries.

Do not publish a full system-dependent wheel unless each dependency and platform contract is explicitly supported.

## Clean-environment procedure

For each wheel:

1. build with `maturin` in release mode;
2. create a new temporary virtual environment;
3. install only the produced wheel and test dependencies;
4. verify import without repository paths on `PYTHONPATH`;
5. run installed-wheel smoke tests;
6. run stable-core fixtures against the installed package;
7. verify `py.typed` and stubs are present;
8. verify package metadata and version constants;
9. uninstall and confirm no residual package files.

## Wheel-content audit

Verify:

- no source-tree absolute paths;
- no debug binaries;
- no credentials or fixture secrets;
- expected native library naming;
- type stubs included;
- licenses and notices included;
- package size within documented budget;
- import surface matches `api_surface()` snapshot.

## Repair platform metadata

Confirm `pyproject.toml` classifiers and README support statements match reality for:

- Python versions;
- Linux architectures;
- macOS architectures;
- Windows experimental status.

## Acceptance criteria

- Core wheel builds and installs in a clean environment.
- Installed-wheel stable-core tests pass without source-tree imports.
- Wheel metadata and stubs are correct.
- Full-no-system wheel follows the same procedure or is explicitly deferred.

---

# Workstream 7 — CI matrix and visible release evidence

## Objective

Move release evidence from commit messages and local validation into visible GitHub checks.

## Required CI jobs

### Formatting and static analysis

- `cargo fmt --all --check`;
- Rust clippy for relevant crates;
- Python formatting/linting if enforced by project policy;
- stub/runtime export parity;
- documentation link checks;
- architecture guards.

### Rust tests

- `cargo test -p eggsec-python`;
- relevant `eggsec` engine tests;
- feature-matrix tests;
- daemon tests when included;
- checkpoint/pipeline tests.

### Python source-tree tests

- both Python test directories;
- deterministic stable-core fixtures;
- sync/async parity;
- secret-sentinel suite;
- serialization and API snapshot tests.

### Wheel tests

- build core wheel;
- clean-environment installation;
- installed-wheel stable-core fixture suite;
- wheel content audit.

## Platform matrix

Required release evidence:

- Linux x86_64;
- macOS arm64.

Optional or experimental evidence:

- Linux aarch64;
- macOS x86_64;
- Windows x86_64.

Windows must not block the first release unless it is declared supported. If experimental, failures should be visible but non-blocking and documented.

## Branch protection

Mark required release jobs as branch-protection checks for release branches or tags.

## Artifact retention

Retain:

- built wheels;
- test reports;
- API snapshots;
- architecture-guard report;
- installed-wheel smoke output;
- SBOM or provenance artifacts where available.

## Acceptance criteria

- The release candidate commit has visible passing checks.
- Linux x86_64 and macOS arm64 are required and green.
- Required checks cannot be bypassed by the release workflow.
- Release artifacts are traceable to the exact commit.

---

# Workstream 8 — TestPyPI dry run

## Objective

Validate the complete publication and installation path before production PyPI release.

## Preparation

Before upload:

- choose the pre-release version;
- update changelog;
- update migration notes;
- verify project URLs;
- verify license metadata;
- verify README rendering;
- ensure no production name collision or stale artifact exists;
- confirm provenance/signing support.

## Dry-run process

1. build release wheels from a clean tagged or release-candidate commit;
2. run wheel validation locally and in CI;
3. upload to TestPyPI;
4. create a clean environment with TestPyPI as the package source;
5. install the exact uploaded version;
6. run installed-package stable-core fixture tests;
7. inspect metadata from the installed distribution;
8. confirm documentation commands match actual installation behavior;
9. record uploaded artifact hashes.

## Failure handling

Any TestPyPI failure requires:

- a new version identifier;
- a new release candidate commit;
- rerun of all required checks;
- no mutation or replacement of already uploaded artifacts.

## Acceptance criteria

- TestPyPI upload succeeds.
- Clean installation from TestPyPI succeeds.
- Stable-core fixture suite passes against the installed artifact.
- Artifact hashes and commit provenance are recorded.

---

# Workstream 9 — Release documentation and security metadata

## Objective

Complete all user-facing and security-facing release metadata.

## Required documents

Update or create:

- changelog;
- release notes;
- migration guide from development/source installation;
- stable-core capability list;
- provisional/experimental domain list;
- platform support matrix;
- known limitations;
- security policy;
- vulnerability disclosure route;
- support policy;
- deprecation policy;
- versioning policy;
- daemon support statement;
- wheel-profile documentation.

## Security language

Documentation must clearly state:

- target authorization requirements;
- scope enforcement semantics;
- risk gating for `load_test` and other active operations;
- local versus daemon support boundary;
- experimental-domain limitations;
- credential-redaction guarantees;
- responsible-use expectations.

## Example validation

Every documented code example should run as part of a documentation smoke suite or be generated from tested snippets.

## Acceptance criteria

- Release documentation matches runtime behavior.
- No broad “all Eggsec tools are stable” claim remains.
- Security and vulnerability routes are current and visible.
- Examples execute against the release artifact.

---

# Workstream 10 — Final release-candidate gate

## Objective

Produce a single auditable go/no-go decision for the first scoped public release.

## Required evidence bundle

The release-candidate commit must provide:

- passing required CI checks;
- architecture-guard report;
- deterministic stable-core fixture results;
- local/daemon boundary decision;
- checkpoint/resume equivalence report;
- secret-sentinel report;
- wheel build and content audit;
- installed-wheel smoke results;
- TestPyPI installation results;
- API surface snapshot;
- domain maturity snapshot;
- changelog and release notes;
- artifact hashes and provenance.

## Go criteria

A release may proceed only when:

- every required checklist item is complete;
- no required test is skipped;
- no stable-core operation lacks deterministic coverage;
- no release document contradicts runtime maturity metadata;
- all required CI checks are attached to the exact release commit;
- TestPyPI installation is verified;
- publication is manually approved.

## No-go criteria

Block release for:

- secret leakage;
- payload loss;
- policy bypass;
- daemon ambiguity;
- nondeterministic stable-core failures;
- missing wheel installation evidence;
- failing architecture guards;
- missing required CI checks;
- mutable or untraceable release artifacts.

---

# Recommended implementation sequence

## Phase 1 — Hermetic behavior proof

1. Build the TCP, HTTP, TLS, and DNS fixture harness.
2. Replace skippable stable-core tests with required deterministic tests.
3. Add sync/async normalization assertions.
4. Add event, error, and policy assertions to each fixture test.

## Phase 2 — Lifecycle and security closure

1. Close checkpoint/resume equivalence.
2. Complete secret-sentinel coverage.
3. Decide daemon inclusion or deferral.
4. Add parity tests or update maturity boundaries accordingly.

## Phase 3 — Repository and packaging closure

1. Repair architecture guards.
2. Build core and full-no-system wheels.
3. Validate clean installation.
4. Run installed-wheel fixture tests.
5. Audit wheel content and metadata.

## Phase 4 — CI and publication proof

1. Add required CI matrix jobs.
2. Attach branch-protection requirements.
3. Produce visible release evidence.
4. Complete release documentation.
5. Upload to TestPyPI.
6. Verify clean TestPyPI installation.

## Phase 5 — Release decision

1. Freeze the release-candidate commit.
2. Generate the evidence bundle.
3. Review every release checklist item.
4. Manually approve or reject publication.

---

# Testing commands expected at completion

The final command set should include stable wrappers for:

```bash
cargo fmt --all --check
cargo clippy --lib -p eggsec
cargo check -p eggsec-python
cargo check -p eggsec-python --features full-no-system
cargo test -p eggsec-python
pytest crates/eggsec-python/tests/ crates/eggsec-python/python/tests/
bash scripts/check-architecture-guards.sh
bash scripts/build_wheel_profiles.sh
bash scripts/validate_wheel.sh
```

Add a single release-validation entry point, for example:

```bash
bash scripts/validate_python_release_candidate.sh
```

That script should orchestrate all required local checks and fail on any skipped stable-core fixture.

---

# Completion definition

This release-closure plan is complete when:

- all ten stable operations have deterministic, non-skipping fixture coverage;
- sync and async stable-core results are equivalent;
- daemon support is either fully parity-tested or explicitly deferred;
- checkpoint/resume equivalence passes;
- secret sentinels are absent from all inspected surfaces;
- repository architecture guards pass;
- core wheels build and install cleanly;
- installed-wheel stable-core tests pass;
- Linux x86_64 and macOS arm64 CI checks are visible and green;
- TestPyPI upload and installation succeed;
- release documentation and security metadata are current;
- the release candidate has a complete evidence bundle;
- publication is manually approved.

This pass should end with a scoped, defensible `0.x` release of the Eggsec Python stable core. It should not be used to claim full Python parity or 1.0 stability for the broader Eggsec capability set.
