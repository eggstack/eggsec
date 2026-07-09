# Eggsec Python Library Roadmap

## Purpose

This roadmap defines the path for exposing Eggsec as a Python-native library backed by the existing Rust engine. The goal is to let Python users compose Eggsec scans, probes, recon workflows, WAF checks, load tests, findings, reports, and eventually all major Eggsec tools without shelling out to the CLI.

This is not a plan to add Python script execution inside Eggsec. Python is the host language and Eggsec is the native library. Lua/NSE remains the compatibility scripting surface through `eggsec-nse`. The Python API should sit beside CLI, TUI, daemon, REST/gRPC/WebSocket, and agent surfaces as another adapter over the same core engine.

## Current architectural fit

The current workspace is already suitable for this line of work. Core primitives, runtime DTOs, tool abstractions, NSE support, daemon support, CLI, and TUI are split across distinct crates. The Python library should use that separation rather than erode it.

The binding should be implemented as a new workspace crate, tentatively `crates/eggsec-python`, using PyO3 and maturin. It should depend on Rust library crates such as `eggsec`, `eggsec-core`, `eggsec-tool-core`, and `eggsec-runtime`, but it must not depend on `eggsec-cli` or `eggsec-tui`. The CLI and TUI are user interfaces; Python should bind engine primitives directly.

The first Python API should be intentionally narrow. It should expose a binding-specific facade rather than every public Rust module. This gives Eggsec room to keep refactoring internals while preserving a stable Python API.

## Non-goals

Do not reintroduce arbitrary Python plugin execution inside Eggsec.

Do not wrap the CLI with subprocess calls.

Do not make the first wheel enable every Eggsec feature.

Do not expose raw packet, stress, NSE, browser automation, database, wireless, cloud, or native-dependency-heavy tools in the default MVP package.

Do not bypass existing scope, runtime-surface, or enforcement semantics for convenience.

## Target end state

The eventual user experience should support simple synchronous scripts:

```python
import eggsec

client = eggsec.Client(
    scope=eggsec.Scope.allow_hosts(["example.com"]),
    mode="manual",
)

result = client.scan_ports(
    "example.com",
    ports=[22, 80, 443, 8080],
    concurrency=256,
    timeout_ms=1500,
)

for port in result.open_ports:
    print(port.port, port.service, port.banner)
```

It should also support asyncio-native workflows:

```python
import asyncio
import eggsec

async def main():
    client = eggsec.AsyncClient(
        scope=eggsec.Scope.allow_hosts(["example.com"]),
        mode="manual",
    )
    result = await client.scan_ports(
        "example.com",
        ports=eggsec.PortRange.top_1000(),
        concurrency=512,
    )
    print(result.to_dict())

asyncio.run(main())
```

Automation usage should be explicit:

```python
client = eggsec.Client(
    scope=eggsec.Scope.allow_hosts(["staging.example.com"]),
    mode="automation",
)
```

Automation mode should preserve strict non-manual-override semantics. Manual mode should be operator-directed, but still explicit about scope and dangerous feature activation.

## API design principles

The public Python API should be stable, small, typed, and documented. It should expose Python-facing DTOs that convert from Rust internals. These DTOs should support attribute access, `repr`, `to_dict()`, `to_json()`, and where appropriate `from_dict()`.

Expose both top-level convenience functions and client-oriented APIs, but document the client as the preferred long-term interface. Top-level functions are useful for scripts; clients are better for repeated scans, shared scope, runtime reuse, and policy settings.

Expose synchronous and asynchronous APIs. The Rust engine should remain async-first. The sync Python API should run on a managed Tokio runtime and should release the GIL while performing network work. The async API should integrate with Python asyncio without blocking the event loop.

The error model should map canonical Rust errors into Python exception classes. Do not expose opaque `anyhow` strings as the normal public failure mode. A first exception hierarchy should include `EggsecError`, `ConfigError`, `ScopeError`, `EnforcementError`, `NetworkError`, `ScanError`, `TimeoutError`, `FeatureUnavailableError`, `SerializationError`, and `InternalError`.

Feature availability should be introspectable from Python:

```python
eggsec.features()
eggsec.has_feature("nse")
eggsec.has_feature("packet-inspection")
```

Missing optional functionality should raise `FeatureUnavailableError`, not crash at import time.

## Packaging principles

Use maturin and PyO3. Start with Python 3.9+ or 3.10+ depending on PyO3/runtime constraints. Prefer ABI-stable wheels if practical, but do not force `abi3` if it makes async/runtime integration brittle during the MVP.

Default wheels should be small and reliable. Initial supported targets should be macOS arm64, macOS x86_64, Linux x86_64, and Linux aarch64. Windows can follow after network behavior and optional dependency policy are stable.

The first public wheel should include core scanner and reporting functionality only. NSE, stress testing, raw packets, browser automation, SSH2, OpenSSL-vendored NSE, database drivers, cloud SDKs, and wireless tooling should be staged behind explicit later tracks.

The release process should include TestPyPI dry-runs, wheel install smoke tests, import tests, localhost scan tests, serialization tests, and documentation checks.

## Phase sequence

Phase A establishes the binding foundation: crate skeleton, maturin packaging, importable module, version/features API, exception hierarchy, and architecture documentation.

Phase B delivers the scanner MVP: scope model, sync port scanning, result DTOs, JSON/dict serialization, and quickstart examples.

Phase C adds the async API, client lifecycle, endpoint discovery, service fingerprinting, cancellation behavior, and runtime reuse.

Phase D adds findings, reporting, passive recon, and WAF detection. This phase makes Python useful for real defensive validation pipelines.

Phase E hardens packaging and documentation for PyPI: wheel CI, TestPyPI, type stubs, examples, docs, smoke tests, and release policy.

Phase F expands toward all major tools: fuzzing, WAF validation, load testing, WebSocket, Git secrets, SBOM, database, proxy, mobile, cloud, packet/stress, daemon client, and optional NSE.

## Recommended implementation order

Start with the low-level scanner functions because they are high-value, naturally Rust-accelerated, and already presented as engine primitives. Port scanning should come first, followed by endpoint discovery and service fingerprinting. Only after those APIs are stable should WAF/recon/reporting be added.

Do not start with NSE. NSE is important for Eggsec, but Python-hosted Rust-hosted Lua should not be the first proof of the Python binding model. NSE should be exposed later as an optional submodule with clear sandbox and feature controls.

Do not start with stress/raw-packet features. They create packaging, privilege, portability, and safety complexity. Load testing can come earlier, but raw stress features should wait for explicit feature gating and documentation.

## Documentation structure

Add a Python documentation tree:

```text
docs/python/
  index.md
  installation.md
  quickstart.md
  sync-api.md
  async-api.md
  scope-and-safety.md
  scanner.md
  endpoint-discovery.md
  fingerprinting.md
  waf.md
  recon.md
  reports.md
  nse.md
  packaging.md
  api-reference.md
```

Add examples:

```text
examples/python/
  basic_port_scan.py
  async_multi_target_scan.py
  endpoint_discovery.py
  service_fingerprint.py
  waf_detection.py
  recon_report.py
  load_test_staging.py
  scan_to_json.py
  scan_to_pandas.py
  nse_http_title.py
```

Examples should prefer local fixtures, localhost, or clearly scoped staging targets. Active testing examples must describe scope and authorization assumptions before code.

## Release criteria for first public PyPI version

A user can run `pip install eggsec` on supported platforms without having Rust installed.

`import eggsec` works and exposes `__version__`, `features()`, `has_feature()`, `Client`, `AsyncClient`, `Scope`, and the base exception hierarchy.

Basic port scanning works against localhost or an explicitly scoped target.

Endpoint discovery and service fingerprinting work against local fixtures.

Results can be converted to dictionaries and JSON.

Docs include installation, quickstart, sync API, async API, scope/safety semantics, and examples.

The PyPI README clearly states which Eggsec tools are included in the default wheel and which are planned or feature-gated.

## Long-term API stability policy

Before Python API `1.0`, minor releases may adjust APIs but must include migration notes.

After Python API `1.0`, breaking API changes require a major version bump.

Rename paths should emit `DeprecationWarning` for at least one minor release before removal.

Rust internals may continue to evolve behind the Python facade. The facade should be the stability boundary.

## Handoff plan files

Detailed implementation plans are split into:

- `plans/python-bindings-phase-a-foundation.md`
- `plans/python-bindings-phase-b-scanner-mvp.md`
- `plans/python-bindings-phase-c-async-endpoints-fingerprinting.md`
- `plans/python-bindings-phase-d-reporting-recon-waf.md`
- `plans/python-bindings-phase-e-packaging-pypi-docs.md`
- `plans/python-bindings-phase-f-major-tool-expansion.md`
