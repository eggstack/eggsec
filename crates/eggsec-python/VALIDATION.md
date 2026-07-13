# eggsec-python Validation Report

## Final integration checkpoint — 2026-07-12

The closure pass completed the local scoped pre-1.0 stable-core gates:

- Installed-wheel Python suite: **1353 passed, 58 skipped, 23 deselected**.
- Focused regression and release fixture/checkpoint suite: **33 passed**.
- Stable-core registry and sync/async dispatch use one canonical operation
  identifier source.
- `OperationResult.error` is a versioned `OperationError` DTO with typed
  exception mapping and a compatibility `error_message` view.
- Stable-core dispatch records structured policy decisions in the audit log.
- Event envelopes carry monotonic sequence numbers; reliable lifecycle events
  are protected from progress-event drops and delivery statistics are exposed.
- `domain_maturity()` and the architecture/release documentation now mark the
  ten-operation stable-core boundary separately from provisional and
  experimental domains.

Remaining external gates are intentionally recorded in
[`RELEASE_CHECKLIST.md`](RELEASE_CHECKLIST.md): current multi-platform CI
evidence and TestPyPI/PyPI publication. Daemon parity is explicitly deferred
from the first-release contract. The Python crate’s Rust test target remains
link-limited in this environment because the `cdylib` test binary is not linked
with the Python runtime; `cargo check` and installed-extension pytest coverage
pass.

## Validation Summary

**Date:** 2026-07-12
**Platform:** linux (x86_64)
**Rust toolchain:** nightly (via rtk)
**Python:** 3.12.3
**maturin:** 1.14.1

| Category | Status |
|----------|--------|
| Rust validation matrix | 14 PASS, 1 environment-limited |
| Python build + smoke | PASS |
| Network failure triage | RESOLVED (6 tests properly skipped) |
| Async API tests | PASS (fixture equivalence plus existing async coverage) |
| Python suite | 1353 passed, 58 skipped, 23 deselected |
| Export checker | PASS (263 default exports resolve) |
| GitHub Actions workflow | EXISTS, VALID |
| Release checklist | UPDATED |

---

## 1. Rust Validation Matrix

| Command | Result | Notes |
|---------|--------|-------|
| `cargo check -p eggsec-python` | PASS | 11 warnings (pre-existing PyO3 cfg) |
| `cargo test -p eggsec-python` | ENVIRONMENT-LIMITED | The `cdylib` test binary is not linked with the Python runtime in this container; Rust compilation/checks pass and pytest covers the installed extension |
| `cargo check -p eggsec-python --features full-no-system` | PASS | |
| `cargo check -p eggsec-python --features websocket` | PASS | |
| `cargo check -p eggsec-python --features git-secrets` | PASS | |
| `cargo check -p eggsec-python --features sbom` | PASS | |
| `cargo check -p eggsec-python --features container` | PASS | |
| `cargo check -p eggsec-python --features db-pentest` | PASS | |
| `cargo check -p eggsec-python --features web-proxy` | PASS | |
| `cargo check -p eggsec-python --features mobile` | PASS | |
| `cargo check -p eggsec-python --features packet-inspection` | PASS | |
| `cargo check -p eggsec-python --features stress-testing` | PASS | |
| `cargo check -p eggsec-python --features nse` | PASS | |
| `cargo check -p eggsec-python --features daemon-client` | PASS | |
| `cargo check -p eggsec-python --all-features` | PASS | 131 warnings (all pre-existing dead_code) |

All warnings are pre-existing PyO3 `cfg` or downstream dead_code. None block the default wheel.

## 2. Python Build + Smoke

| Step | Result |
|------|--------|
| `maturin develop` | ENVIRONMENT-LIMITED | Shared development venv is read-only; release-wheel installation used instead |
| `import eggsec` | PASS (version 0.1.0, 24 features) |
| Release wheel build | PASS (core and full-no-system profiles; manylinux 2.38/2.39) |
| Clean venv wheel install | PASS (both profiles) |
| `__all__` check | PASS (263 default names, all resolve) |
| Scanner smoke | PASS (generate_fuzz_payloads, Scope, Client, scope enforcement) |
| Report smoke | PASS (Report, Finding, FindingSet, Evidence, to_dict/to_json/write_json/to_rows/write_markdown) |

## 3. Network Failure Triage

**Root cause:** 6 `@pytest.mark.network` tests depend on example.com DNS/TLS, which is unavailable in the test environment.

**Fix:** Added `[tool.pytest.ini_options]` to `pyproject.toml` with `addopts = "-m 'not network'"`.

| Test | Classification | Action |
|------|---------------|--------|
| `test_recon_dns` | Environment-dependent DNS | Skipped by marker |
| `test_recon_dns_records` | Environment-dependent DNS | Skipped by marker |
| `test_client_recon_dns` | Environment-dependent DNS | Skipped by marker |
| `test_inspect_tls` | Environment-dependent TLS | Skipped by marker |
| `test_inspect_tls_certificate_details` | Environment-dependent TLS | Skipped by marker |
| `test_inspect_tls_versions` | Environment-dependent TLS | Skipped by marker |

**Result:** Default pytest suite has 0 failures. Network tests available via `pytest -m network`.

## 4. Async API Tests

Existing async tests plus the closure fixture suite cover:
- `test_async_scan_ports_returns_future` — verifies `async_scan_ports` returns a `PyFuture`
- `test_async_scan_ports_denied_scope` — verifies `EnforcementError` for out-of-scope target
- `test_async_validate_waf_denied_scope` — verifies scope enforcement on async WAF validation
- `test_async_fuzz_http_denied_scope` — verifies scope enforcement on async fuzzing
- `test_async_load_test_denied_scope` — verifies scope enforcement on async load testing

The closure suite additionally compares normalized sync/async results for the
stable-core network operations from an installed release wheel. True
`await`-based tests still require `pytest-asyncio` and a running event loop;
the extension’s polling awaitable is covered directly by those equivalence
tests.

## 5. Export Checker

Created `scripts/check_eggsec_python_exports.py` which verifies:
- `__all__` exists and is non-empty
- Every `__all__` name resolves at runtime
- Feature-gated symbols are absent by default
- Required default symbols are present
- Active top-level APIs enforce scope

## 6. GitHub Actions Workflow

`.github/workflows/python-wheels.yml` exists and is valid:
- Builds wheels for x86_64/aarch64 on macOS and Linux
- Test job installs wheel in clean venv, runs smoke tests and pytest
- Publish job gated by `workflow_dispatch` (manual only)
- TestPyPI dry run before PyPI publish

## 7. Remaining Pre-PyPI Gates

- [ ] TestPyPI dry run (requires manual trigger)
- [ ] Install from TestPyPI succeeds (requires manual trigger)
- [ ] Final PyPI publish (requires ALL gates to pass)

**Status:** Local validation gates pass. Multi-platform CI evidence and
TestPyPI/PyPI still require the manual workflow/environment gates.
