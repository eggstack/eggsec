# Python API Release 5 Phase E — Packaging, Documentation, and Executable Examples

## Objective

Turn the stabilized API into predictable installable artifacts with correct metadata, explicit feature profiles, supported platform guarantees, and executable documentation. Built wheels—not source-tree imports—must become the primary release validation unit.

## Workstream E1 — Package metadata correction

Audit and correct:

- repository, homepage, documentation, and issue URLs;
- package author and contributor metadata;
- license metadata and included license files;
- classifiers and development status;
- supported Python versions;
- supported operating systems and architectures;
- Rust minimum supported version;
- project description and keywords;
- package version synchronization with the Rust workspace;
- embedded feature, schema, ABI, and protocol versions.

Add a CI check that rejects stale organization/repository URLs and version disagreement among Cargo, Python metadata, generated constants, and release tags.

## Workstream E2 — Wheel profile definition

Define explicit wheel profiles in a machine-readable manifest. At minimum:

- `core`: always-compiled stable engine and dependency-light primitives;
- `full-no-system`: broadly useful features without platform system dependencies;
- source-build/system profiles for NSE, packet inspection, databases, proxy, browser, mobile, stress, and other platform-sensitive domains;
- any separately published profile only if the operational and support cost is justified.

Each profile must declare:

- Cargo features;
- expected Python exports;
- unavailable capability descriptors;
- system libraries and runtime binaries;
- supported platforms and architectures;
- required integration tests;
- maximum wheel and extension size;
- maturity claims;
- expected shared-library dependencies.

Do not use Python extras to imply that a prebuilt native wheel gains Cargo features after installation. Documentation must clearly distinguish Python dependencies from compile-time native features.

## Workstream E3 — Build matrix

Build and test supported CPython versions and platforms. Initial target:

- CPython 3.9–3.13 or a deliberately reduced set justified by PyO3 and CI support;
- Linux x86_64 and aarch64 where runners are reliable;
- macOS x86_64 and arm64;
- Windows only after explicit support audit; do not imply Windows support through generic classifiers if it is not validated.

For each wheel:

1. build in a clean environment;
2. inspect tags and native dependencies;
3. install into a clean virtual environment;
4. run import, introspection, stub, and stable-operation smoke tests;
5. verify feature exports against the profile manifest;
6. verify package data, `py.typed`, stubs, docs metadata, and version constants;
7. record wheel hash, size, import time, and extension dependencies.

## Workstream E4 — Source distribution

Decide whether to publish an sdist. If supported:

- include all required workspace crates and generated sources;
- document Rust and system dependency requirements;
- test sdist builds in isolated environments;
- fail cleanly with actionable diagnostics when prerequisites are missing;
- ensure generated capability and schema files are present and current.

If a reliable workspace sdist is not feasible, omit it rather than publishing a broken source artifact.

## Workstream E5 — Documentation information architecture

Create a Python documentation landing structure organized by workflow:

1. installation and wheel profiles;
2. capability and maturity discovery;
3. stable operations with `Engine` and `AsyncEngine`;
4. scope, policy, confirmation, and audit;
5. events, callbacks, cancellation, and timeouts;
6. pipelines and checkpoints;
7. low-level networking and protocol probes;
8. tools and JSON schemas;
9. repositories, artifacts, and reporting;
10. daemon execution;
11. provisional managed sessions;
12. experimental domains;
13. API reference and compatibility policy.

Avoid documentation that merely mirrors the flat extension symbol list.

## Workstream E6 — Executable examples

Create small deterministic examples for:

- sync and async port scanning against loopback fixtures;
- DNS/TLS/HTTP probes;
- a custom protocol workflow using TCP/UDP primitives;
- WebSocket session use;
- engine capability discovery;
- policy preflight and denial handling;
- event streaming and progress sinks;
- cancellation and timeout;
- pipelines, retries, and checkpoints;
- finding repositories and baseline comparison;
- content-addressed artifacts;
- streaming reports;
- tool descriptor/schema discovery and invocation;
- daemon execution and reconnect;
- feature-unavailable handling;
- experimental namespace import and maturity inspection.

Examples must not depend on public internet services. Use deterministic local fixtures and bounded resource use.

## Workstream E7 — Documentation testing

Run examples and code blocks in CI against installed wheels. Requirements:

- mark profile-specific examples with required features;
- fail on stale symbols or signatures;
- prohibit examples from silently skipping;
- assert cleanup of spawned servers, sockets, files, and subprocesses;
- validate output semantically rather than relying on unstable exact text;
- test both sync and async paths where documented.

## Workstream E8 — Generated API reference

Generate reference pages from the canonical registry, stubs, and docstrings. Include:

- canonical namespace;
- signature;
- maturity;
- required feature and wheel profile;
- risk and policy behavior;
- sync/async counterpart;
- request and result schema links;
- lifecycle and cancellation notes;
- compatibility aliases and deprecation status;
- validation profile and evidence status.

Do not expose internal `Py` names or Rust implementation modules as canonical documentation.

## Workstream E9 — User-facing diagnostics

Improve installation and feature errors so users can determine:

- which wheel/profile is installed;
- which features were compiled;
- why a symbol is unavailable;
- required system packages or runtime binaries;
- supported platform combinations;
- whether a capability is stable, provisional, or experimental;
- how to build from source when appropriate.

Add `eggsec.build_info()` and feature-report examples to issue templates and troubleshooting docs.

## Workstream E10 — Release automation

Add release automation that:

- builds from a clean tagged commit;
- validates generated files before build;
- signs or attests artifacts where project policy supports it;
- computes SHA-256 hashes;
- uploads wheel evidence;
- tests the exact uploaded artifacts before publication;
- supports TestPyPI rehearsal;
- refuses publication from a dirty tree or mismatched version/tag;
- generates release notes from compatibility and maturity changes.

## Acceptance criteria

Phase E is complete when:

- package metadata contains no stale repository identity;
- wheel profiles are explicit and machine-validated;
- every supported wheel is installed and tested in isolation;
- feature availability matches the profile manifest;
- documentation is workflow-oriented and generated where appropriate;
- all stable examples execute against deterministic local fixtures;
- API references show maturity, feature, policy, and lifecycle information;
- publication automation validates the exact artifacts destined for PyPI.