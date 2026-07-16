# Python API Release 5 Phase C — Namespace and Maturity Governance

## Objective

Replace the current flat export surface with an intentional package structure that separates stable application APIs, reusable primitives, provisional managed subsystems, and hazardous experimental capabilities. Preserve source compatibility through aliases and deprecation policy while making maturity discoverable and mechanically enforced.

## Target package structure

The final structure should follow capability ownership rather than Rust module layout. A proposed shape is:

```text
eggsec/
  __init__.py          stable convenience imports
  engine.py            Engine, AsyncEngine, operation requests/results
  findings.py          findings, evidence, artifacts, workflow
  events.py            event protocol, streams, callbacks
  net/                 targets, transports, probes, HTTP, WebSocket
  tools/               tool descriptors, schemas, invocation adapters
  sessions/            browser, mobile, database, proxy, capture contracts
  storage/             finding/assessment repositories, artifacts
  reporting/           reporters, streaming, baselines, formats
  daemon/              provisional daemon client and parity contracts
  experimental/        wireless, evasion, postex, C2, hunting, AI, raw packet work
```

The exact files may differ, but public ownership and maturity boundaries must be explicit.

## Workstream C1 — Public inventory

Create a generated inventory of every currently exported symbol containing:

- current public name;
- defining Rust module;
- Python alias;
- proposed destination module;
- maturity;
- feature requirement;
- stable compatibility obligation;
- replacement name, if renamed;
- deprecation version and removal floor;
- intentional top-level re-export status.

Resolve duplicated names such as multiple retry policies, migration results, daemon events, task statuses, confidences, artifacts, and operation descriptors. Public names must represent distinct concepts or be unified.

## Workstream C2 — Naming normalization

Remove implementation suffixes from public Python names where compatibility permits:

- `Py` suffixes must not appear in canonical Python class names;
- Rust-internal distinctions should not leak unless semantically meaningful;
- `Graphql`/`GraphQL`, `Oauth`/`OAuth`, `Db`/`Database`, and similar naming must be consistent;
- synchronous and asynchronous pairs should use `Type` and `AsyncType` consistently;
- factory functions should use predictable verbs;
- request and result naming must align with operation IDs and schemas.

Maintain temporary aliases for existing public spellings. Add tests that canonical names and compatibility aliases resolve to the same type where appropriate.

## Workstream C3 — Stable top-level surface

Keep the top-level package intentionally small. It should expose:

- version and feature introspection;
- `Engine`, `AsyncEngine`, `Client`, and `AsyncClient` where retained;
- scope and policy primitives;
- stable operation request/result types;
- core finding, evidence, artifact, status, error, cancellation, and event types;
- stable convenience functions.

Do not top-level re-export every feature-gated session DTO or experimental technique type.

## Workstream C4 — Experimental isolation

Move or alias the following under `eggsec.experimental` unless a separate maturity review promotes them:

- wireless testing;
- evasion validation;
- post-exploitation simulation;
- C2 simulation;
- advanced hunting;
- AI post-processing and generated payload suggestions;
- raw packet injection/replay with active transmission;
- highly platform-sensitive or provider-dependent capabilities.

Requirements:

- importing `eggsec` must not initialize experimental dependencies;
- experimental modules must expose a clear warning in documentation, not on every import;
- top-level legacy aliases emit `DeprecationWarning` only when accessed or instantiated;
- experimental status must appear in descriptors and generated docs;
- no experimental API may be presented as part of stable wheel compatibility.

## Workstream C5 — Provisional subsystem namespaces

Place daemon, browser, mobile dynamic, interception proxy, database sessions, packet capture, and NSE runtime APIs in explicit subsystem modules. Their maturity may remain provisional even if some one-shot operations in the same domain are stable.

Document this distinction. For example, static APK analysis may be stable while emulator sessions remain provisional; `db_probe` may be stable while arbitrary stateful database sessions remain provisional.

## Workstream C6 — Lazy feature imports

Feature-gated modules should import lazily and fail with structured `FeatureUnavailableError` carrying:

- requested module or symbol;
- required Cargo feature;
- current wheel profile;
- supported installation/build route;
- maturity;
- platform prerequisites.

Avoid an `AttributeError` or generic `ImportError` when a known capability is unavailable. Ensure introspection can list unavailable capabilities without importing their native classes.

## Workstream C7 — Maturity source of truth

Move maturity classification into the authoritative registry/capability model. Generate:

- `domain_maturity()`;
- `api_surface()` maturity fields;
- package documentation maturity badges/tables;
- feature profile documentation;
- deprecation inventory;
- stable compatibility baseline selection.

CI must reject:

- documentation claiming higher maturity than the registry;
- stable symbols under `experimental` without an exception;
- experimental symbols in stable compatibility baselines;
- feature-gated exports missing capability records;
- maturity promotion without required evidence references.

## Workstream C8 — Deprecation policy

Define a pre-1.0 but disciplined policy:

- stable names are not removed without a documented migration path;
- canonical renames retain aliases for at least one minor release or an explicitly stated interval;
- deprecated access emits standard `DeprecationWarning` with replacement and removal floor;
- deprecations are listed in machine-readable metadata;
- aliases are tested in installed wheels;
- removal requires compatibility-baseline update and release notes.

## Workstream C9 — Import and dependency tests

Test:

- `import eggsec` on every wheel profile;
- import time and imported-module count;
- no browser, database, container, packet, mobile, or AI dependency initializes in the core profile;
- lazy feature modules provide structured errors;
- canonical and compatibility imports work;
- experimental modules do not pollute `dir(eggsec)`;
- type checkers resolve submodule imports correctly;
- documentation examples use canonical modules.

## Acceptance criteria

Phase C is complete when:

- every public symbol has one canonical namespace and maturity;
- the stable top-level surface is bounded and documented;
- experimental capabilities are isolated;
- provisional subsystem APIs are distinct from stable one-shot operations;
- known unavailable features produce structured guidance;
- compatibility aliases and deprecations are tested;
- maturity, exports, docs, and wheel profiles derive from the same source of truth;
- importing the core package does not initialize optional subsystem dependencies.