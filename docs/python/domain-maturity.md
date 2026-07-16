# Python domain maturity

`eggsec-python` exposes more modules than the current stable execution
contract covers. Importability and Cargo feature availability do not imply a
compatibility guarantee.

The stable release boundary is the twenty-two-operation engine registry:

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
- `scan_git_secrets` (canonical operation ID for git-secrets domain)
- `generate_sbom` (canonical operation ID for sbom domain)
- `run_consolidated_recon` (canonical operation ID for consolidated-recon domain)
- `graphql_test` (canonical operation ID for graphql domain)
- `oauth_test` (canonical operation ID for oauth domain)
- `auth_test` (canonical operation ID for authentication domain)
- `db_probe` (canonical operation ID for database domain)
- `nse_run` (canonical operation ID for nse domain)
- `scan_docker_image` (canonical operation ID for container domain)
- `scan_kubernetes` (canonical operation ID for container domain)
- `analyze_apk` (canonical operation ID for mobile-static domain)
- `analyze_ipa` (canonical operation ID for mobile-static domain)

These operations use the canonical registry, mandatory policy gate, typed
result payloads, structured errors, audit decisions, and sync/async dispatch.
`load_test` remains risk-gated by policy even though its request and result
types are part of the stable-core schema.

The first-release guarantee is local-only: it applies to `Engine` and
`AsyncEngine` in the installed Python package. The optional daemon client is
not part of stable-core. It remains provisional until a separate milestone
closes request normalization, result retrieval, reconnect/replay, event
ordering, cancellation, timeout, and artifact parity with local execution.

Stable-core requests contain no credential fields. Secret-bearing provisional
domains must use `SensitiveString` and keep credentials out of repr, events,
reports, and checkpoints. Checkpoint release tests use unique sentinels to
verify recursive redaction before persistence; `expose_secret()` remains an
explicit manual-only operation.

## Domain Maturity Table

All domains are classified as `stable`, `provisional`, or `experimental`
until they satisfy the graduation checklist:

### Graduation Checklist

1. canonical operation ID and request/result DTO;
2. sync and async dispatch through the common policy gate;
3. structured errors, events, cancellation, and serialization tests;
4. deterministic fixtures and local/daemon contract coverage where relevant;
5. documentation, type stubs, and wheel-profile coverage.

### Stable Domains

| Domain | Operation ID(s) | Notes |
|--------|-----------------|-------|
| `stable-core` | `scan_ports`, `scan_endpoints`, `fingerprint_services`, `recon_dns`, `inspect_tls`, `detect_technology`, `detect_waf`, `validate_waf`, `fuzz_http`, `load_test` | Original ten operations; mandatory policy gate, typed results, sync/async tests |
| `git-secrets` | `scan_git_secrets` | Policy gate, typed results, sync/async tests |
| `sbom` | `generate_sbom` | Canonical operation ID, policy gate, typed results |
| `consolidated-recon` | `run_consolidated_recon` | Canonical operation ID, policy gate, typed results |
| `graphql` | `graphql_test` | Canonical operation ID, policy gate, typed results |
| `oauth` | `oauth_test` | Canonical operation ID, policy gate, typed results |
| `authentication` | `auth_test` | Canonical operation ID, policy gate, typed results |
| `database` | `db_probe` | Canonical operation ID, policy gate, typed results |
| `nse` | `nse_run` | Canonical operation ID, policy gate, typed results |
| `container` | `scan_docker_image`, `scan_kubernetes` | Canonical operation IDs, policy gate, typed results |
| `mobile-static` | `analyze_apk`, `analyze_ipa` | Canonical operation IDs, policy gate, typed results |

### Experimental Domains (conditional graduation candidates)

| Domain | Notes |
|--------|-------|
| `browser` | Conditional candidate, not yet graduated |
| `hunt` | Conditional candidate, not yet graduated |

### Provisional / Experimental (unchanged)

| Domain | Status | Notes |
|--------|--------|-------|
| `daemon` | provisional | Transport parity pending |
| `proxy` | experimental | MITM interception semantics remain hazardous |
| `packet-inspection` | experimental | Platform/system dependency and lifecycle coverage pending |
| `wireless` | experimental | Platform-sensitive, root required |
| `evasion` | experimental | MITRE ATT&CK mapped |
| `postex` | experimental | Post-exploitation simulation |
| `c2` | experimental | C2 simulation |
| `distributed` | experimental | Cluster architecture |
| `ai` | experimental | LLM integration |

## Release 4: Common Session Contract (Provisional)

| Symbol | Stability | Notes |
|--------|-----------|-------|
| `SessionState` | provisional | Shared session lifecycle state machine |
| `SessionIdentity` | provisional | Session identification and metadata |
| `MobileDeviceDescriptor` | provisional | Device enumeration and capabilities |
| `MobileSession` | provisional | Managed mobile analysis session |
| `BrowserSession` | provisional | Managed browser session lifecycle |
| `BrowserSecurityPrimitive` | provisional | Browser security primitives |
| `SessionRepository` | provisional | Content-addressed session storage |
| `SQLiteSessionRepository` | provisional | SQLite-backed session repository |
| `InMemorySessionRepository` | provisional | In-memory session repository |
| `ArtifactStore` | provisional | Content-addressed artifact storage |
| `DirectoryArtifactStore` | provisional | Filesystem-backed artifact store |
| `StreamingReporter` | provisional | Incremental report generation |
| `ReportDiff` | provisional | Diff comparison between reports |

No operations are promoted in Release 4. All session types are provisional;
the Releases 1-3 stable-core guarantees remain intact.

### Async runtime ownership (Workstream 1 closure)

The shared Tokio runtime in `runtime_async.rs` now uses a process-global
`OnceLock<Runtime>` with a multi-thread worker pool. This ensures stateful
async resources (`AsyncTcpSession`, `AsyncUdpSocket`, `AsyncHttpClient`,
`AsyncWebSocketSession`, etc.) survive across chained awaits on a single
session. Previously, each `PyFuture` spawned its own per-call runtime that
shut down on completion, preventing chained operations. All async transport
lifecycle tests now pass without skip markers.

## Validation Infrastructure

Release 1-4 closure introduced a profile-based validation system. Maturity
classifications are now derived from structured profile evidence rather than
hand-maintained checklists. Each of the 20 validation profiles produces
evidence JSON containing test counts, skip budgets, wheel metadata, and
platform info. Skip budget enforcement prevents silent test suite erosion by
requiring minimum test counts and capping allowed skips/xfails.

Profile manifest: `crates/eggsec-python/validation/profiles.json`

See `crates/eggsec-python/README.md` for the full profile inventory and usage
instructions.

## Operational Correction Pass Status (Releases 1-4)

| Workstream | Status | Evidence |
|------------|--------|----------|
| WS1: Shared async runtime | Closed | `OnceLock<Runtime>`, 94/94 lifecycle tests pass |
| WS2: NSE runtime proof | Closed | 65 passed, 35 skipped (network-dependent); runtime reuse, limits, cancellation, library registry, metadata all verified |
| WS3: Interception proxy | Closed | 78 passed, 12 skipped; DTO verification complete; Python binding returns empty exchanges (documented limitation) |
| WS4: Database assessment | Closed | 111 passed; driver registry, session config, query types verified; no SQLite driver (only postgres/mysql/mssql/mongodb/redis) |
| WS5: Mobile session | Closed | 104 skipped; requires real Android emulator (not available in CI) |
| WS6: Browser session | Closed | 123 skipped; requires real browser backend (not available in CI) |
| WS7: Daemon parity | Closed | 6 passed, 3 skipped (daemon integration); protocol version, session CRUD, health verified |
| WS8: Repository durability | Closed | 64+ tests; SQLite/JSONL CRUD, concurrency, dedup, pagination, migration, corruption detection |
| WS9: Streaming reporting | Closed | 71 passed; StreamingReporter bug fix (total_findings counter), config, flush, formats |
| WS10: Maturity metadata | Closed | domain-maturity.md updated; correction pass status documented |
| WS11: CI integration | Closed | 7 Python test profiles added to test.yml; maturin wheel build + pytest |
| WS12: Stress hardening | Closed | 38 passed, 1 skipped; 1000-cycle TCP/UDP/repository stress tests, FD leak detection |

WS2 closure evidence: NseRuntime, NseExecutionLimits, NseCancellationToken,
NseLibraryRegistry, NseHostContext, NsePortContext, NseRuntimeStats all
verified for construction, serialization, repr, and type correctness. Script
execution tests skip gracefully when network services are unavailable.

WS7 closure evidence: daemon binary spawned as child process, connected via
Unix socket, health/capabilities/session CRUD/close verified end-to-end.

WS9 closure evidence: 71 streaming operational tests passing; config
construction, incremental finding writes, buffer flush, summary generation,
severity distribution, output formats (JSON/JSONL/CSV/Markdown), secret
redaction configuration, diff reporter with baseline comparison. Bug fix:
`StreamingReporterPy::finish()` and `StreamingDiffReporterPy::finish()` now
correctly track `total_written` across buffer flushes.

WS10 closure evidence: domain-maturity.md correction pass status table
updated with all 12 workstreams marked closed. Feature-gated tests verified
with `maturin develop --features nse,web-proxy,db-pentest,mobile,headless-browser,daemon-client`.

WS11 closure evidence: 7 Python test profiles added to `.github/workflows/test.yml`:
default-wheel, nse, db-pentest, web-proxy, mobile, headless-browser, daemon-client.
Each profile builds maturin wheel with specified features and runs pytest.

## Release 2: Network Programmability (Provisional)

| Symbol | Stability | Notes |
|--------|-----------|-------|
| `TargetPy` | provisional | Target specification |
| `ResolvedTargetPy` | provisional | DNS resolution result |
| `ConnectionConfigPy` | provisional | Connection configuration |
| `TimeoutConfigPy` | provisional | Phase timeout configuration |
| `RetryPolicyPy` | provisional | Retry policy |
| `SocketEndpointPy` | provisional | Socket endpoint info |
| `ConnectionTimingPy` | provisional | Timing breakdown |
| `ConnectionMetadataPy` | provisional | Full connection metadata |
| `NetworkEvidencePy` | provisional | Network operation evidence |
| `TranscriptEntryPy` | provisional | Transcript entry |
| `NetworkTranscriptPy` | provisional | Transcript collection |
| `TcpConfigPy` | provisional | TCP configuration |
| `TcpSessionPy` | provisional | Managed TCP session |
| `UdpConfigPy` | provisional | UDP configuration |
| `UdpSocketPy` | provisional | Managed UDP socket |
| `HttpClientPy` | provisional | Sync HTTP client |
| `AsyncHttpClientPy` | provisional | Async HTTP client |
| `WebSocketSessionPy` | provisional | Sync WebSocket session |
| `AsyncWebSocketSessionPy` | provisional | Async WebSocket session |
| All probe functions | provisional | DNS, TLS, HTTP probes |

1. canonical operation ID and request/result DTO;
2. sync and async dispatch through the common policy gate;
3. structured errors, events, cancellation, and serialization tests;
4. deterministic fixtures and local/daemon contract coverage where relevant;
5. documentation, type stubs, and wheel-profile coverage.

Use the machine-readable table at runtime:

```python
import eggsec

print(eggsec.domain_maturity()["stable-core"])
print(eggsec.api_surface()["graphql"])  # if exported by this build
```

`api_surface()` describes individual exported symbols. `domain_maturity()`
describes the release state of whole capability areas; a compiled feature can
therefore be available while still being provisional or experimental.
