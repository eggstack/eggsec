# eggsec-python AGENTS Override

Python bindings via PyO3/maturin. Host-language binding over the Rust engine.

## Key Files

| File | Purpose |
|------|---------|
| `src/lib.rs` | PyModule definition, class/function registration |
| `src/engine.rs` | Sync `Engine` class — primary entry point |
| `src/async_engine.rs` | Async `AsyncEngine` class (tokio-backed) |
| `src/client.rs` | Sync `Client` class — wraps Engine internally |
| `src/async_client.rs` | Async `AsyncClient` class — wraps AsyncEngine |
| `src/scope.rs` | `Scope` class — target authorization |
| `src/status.rs` | `ExecutionStatus`, `ExecutionStats`, `Artifact`, `OperationResult` |
| `src/requests.rs` | `OperationRequest` + 10 typed request DTOs, `RequestBuilder` |
| `src/handles.rs` | `ExecutionHandle`, `ExecutionEvent`, `EventLog` |
| `src/cancellation.rs` | `CancellationToken` — atomic cancellation |
| `src/pipeline.rs` | `Pipeline`, `AsyncPipeline`, `PipelineStep`, `StepResult`, `PipelineResult` |
| `src/planning.rs` | `PlanStep`, `ScanPlan` — heuristic scan plan generation |
| `src/checkpoint.rs` | `Checkpoint`, `CheckpointStore` — pipeline resumption |
| `src/consolidated_recon.rs` | `ConsolidatedReconConfig`, `run_consolidated_recon()` |
| `src/graphql.rs` | `GraphQLTestConfig`, `GraphQLSchema`, `graphql_test()` |
| `src/oauth.rs` | `OAuthTestConfig`, `OAuthEndpoint`, `oauth_test()` |
| `src/auth_assess.rs` | `AuthTestConfig`, `AuthTestReport`, `auth_test()` |
| `src/browser_assess.rs` | `BrowserTestConfig`, `BrowserTestReport`, `browser_test()` (feature-gated) |
| `src/hunt.rs` | `HuntTestConfig`, `HuntReport`, `hunt_test()` (feature-gated) |
| `src/nse.rs` | `NseConfig`, `NseReport`, `nse_run()` + D1 types + Release 3: `NseLibraryDescriptor`, `NseLibraryRegistry`, `NseArgument`, `NseEvidenceItem`, `nse_list_libraries_detailed()`, `nse_get_library_descriptor()`, `nse_run_with_config()`, `nse_validate_script()` (feature: `nse`) |
| `src/packet_inspection.rs` | `CaptureConfig`, `PacketInfo` + D2/D3 types: `PacketFilter`, `FlowRecord`, `LiveCaptureResult`, `TracerouteConfig`, `TracerouteResult` (feature: `packet-inspection`) |
| `src/proxy.rs` | `ProxyManager`, `ProxyConfig` + D4 types + Release 3: `InterceptSessionState`, `InterceptStats`, `InterceptFilter`, `InterceptRule`, `CertificateAuthorityConfig`, `IssuedCertificate`, `HarEntry`, `HarDocument`, `run_intercept_session()` (feature: `web-proxy`) |
| `src/mobile.rs` | `MobileScanReport`, `analyze_apk/ipa` + D5 types: `MobileDevice`, `DynamicMobileConfig`, `DynamicMobileReport` (feature: `mobile`) |
| `src/daemon.rs` | `DaemonClient`, `daemon_connect` + D6 types: `DaemonCapabilities`, `TaskHandle`, `TaskStatus`, `DaemonEvent`, `SessionSummary`, `TransportMetadata` (feature: `daemon-client`) |
| `src/db_pentest.rs` | `DbPentestReport`, `db_probe` + D7 types + Release 3: `DbDriverRegistry`, `DbTarget`, `DatabaseSessionState`, `DatabaseConnectionMetadata`, `DatabaseSessionStats`, `DatabaseCredentialRequest`, `DatabaseCredentialResult`, `DatabaseQuery`, `DatabaseQueryResult`, `DatabaseColumn`, `DatabaseSchemaInfo`, `DatabaseTableInfo`, `DatabasePrivilegeInfo` (feature: `db-pentest`) |
| `src/error.rs` | Python exception hierarchy |
| `src/runtime_sync.rs` | Sync blocking wrapper |
| `src/runtime_async.rs` | Async runtime (`PyFuture`) |
| `python/eggsec/__init__.py` | Public API re-exports |
| `python/eggsec/__init__.pyi` | Type stubs |
| `pyproject.toml` | maturin build configuration |

## Engine/Operation Model

The canonical entry point is `Engine` (sync) or `AsyncEngine` (async). `Client`/`AsyncClient` wrap these internally for backward compatibility.

```python
from eggsec import Engine, Scope, PortScanRequest, PortRange

scope = Scope.allow_hosts(["example.com"])
engine = Engine(scope)

# Typed request
req = PortScanRequest("example.com", port=PortRange.top_1000(), timing="normal")
result = engine.run_port_scan(req)

# Dispatch by name
result = engine.run("port_scan", target="example.com")

# Plan and execute
plan = engine.plan("example.com")
for step in plan.steps:
    result = engine.run(step.operation, target=step.target)

# Pipeline
from eggsec import Pipeline
pipe = Pipeline(engine)
pipe.add_step("recon_dns", "example.com")
pipe.add_step("fingerprint", "example.com")
pipe_result = pipe.run()

# Async
from eggsec import AsyncEngine
async with AsyncEngine(scope) as aengine:
    result = await aengine.run_port_scan(req)
```

## Build & Test

```bash
cd crates/eggsec-python
maturin develop                           # dev build into active venv
maturin develop --features <feature>      # with specific features
maturin build --release                   # release wheel
cargo test -p eggsec-python               # Rust-side tests
pytest crates/eggsec-python/tests/        # Python-side tests
```

## Conventions

- Register new classes in `src/lib.rs` via `m.add_class::<T>()`.
- Register new functions via `m.add_function(wrap_pyfunction!(...)?)`.
- Re-export all public API in `python/eggsec/__init__.py`.
- Add type stubs in `python/eggsec/*.pyi` for every public class/function.
- Add tests in `tests/` for Python-side validation.
- GIL is released during network I/O; use `py.allow_threads()` for blocking calls.
- Feature-gated engine modules require explicit `--features` at build time.
- All result types follow `#[pyclass(frozen)]` with `from_engine()`, `to_dict()`, `to_json()`, `__repr__`, `__str__`.
- Engine methods return `OperationResult` (common protocol). `Client`/`AsyncClient` unwrap to domain-specific types for backward compat.
- `Client` wraps `Engine` internally; `AsyncClient` wraps `AsyncEngine`. All scope enforcement delegated to engine helpers.
- New code should use `Engine`/`AsyncEngine` + typed request DTOs. `Client`/`AsyncClient` retained for backward compatibility.
- Milestone C modules (consolidated_recon, graphql, oauth, auth_assess) are always-available. `browser_assess` and `hunt` are feature-gated.
- Milestone D adds: `nse` script metadata/sandbox, `packet_inspection` filter/flow/traceroute, `proxy` intercept types, `mobile` device/dynamic, `daemon` capabilities/tasks, `db_pentest` drivers/credentials. All feature-gated.
- Phase E adds: corrected package metadata, `wheel-profiles.json` manifest, enhanced `build_info()` with `wheel_profile()`, workflow-oriented docs landing page, 6 deterministic executable examples.
- Assessment module pattern: `*_test(config)` sync + `async_*_test(config)` async. Config types have `Default` impls. Result types are engine-produced only (no Python constructors).
- Session-oriented types (PcapWriter, DaemonClient, ProxyManager) implement `__enter__`/`__exit__` context managers with idempotent `close()` and `is_closed` property.

## Phase C: Namespace conventions

- New public symbols must be placed in the appropriate submodule (`net`, `sessions`, `storage`, `reporting`, `daemon`, `experimental`) based on capability ownership
- The top-level `__init__.py` retains backward-compatible re-exports; new code should import from submodules
- Py-suffixed names are deprecated; use canonical names (e.g., `Target` not `TargetPy`)
- Feature-gated imports use `try/except (AttributeError, ImportError): pass` pattern
- Experimental types go under `eggsec.experimental`, never at top level
- `FeatureUnavailableError` provides structured guidance when features are missing

## Validation Infrastructure

### Profile Manifest

`crates/eggsec-python/validation/profiles.json` defines named test profiles (e.g., `smoke`, `full`, `release-candidate`). Each profile specifies which Python test paths to run, feature gates, and skip-budget thresholds. The manifest is the single source of truth for profile-based CI and local validation.

### Profile Runner

```bash
python scripts/run_python_profile.py --profile <name>
```

Runs the test suite for a named profile. Invokes `pytest` with the correct paths, features, and environment variables. Used by CI and local validation workflows.

### Evidence Bundle Generation

```bash
python scripts/build_python_release_evidence.py --commit <sha>
```

Builds a release evidence bundle for a given commit. Collects test results, coverage data, feature-matrix snapshots, and stub-parity diffs into a structured artifact for audit/review.

### Skip Budget Enforcement

```bash
python scripts/python_skip_budget.py --profile <name>
```

Enforces per-profile skip budgets. Validates that the number of `xfail`/`skip` markers in the test suite does not exceed the thresholds defined in `profiles.json`. Fails the build if the budget is exceeded, preventing uncontrolled skip drift.

### Profile Validation

```bash
python scripts/validate_python_profiles.py
```

Validates the `profiles.json` manifest: checks required fields, profile name consistency, and cross-references against actual test paths. Run before committing changes to the manifest.
