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

## Operational Correction Pass Status (Releases 1-4)

| Workstream | Status | Evidence |
|------------|--------|----------|
| WS1: Shared async runtime | Closed | `OnceLock<Runtime>`, 94/94 lifecycle tests pass |
| WS2: NSE runtime types | In progress | Library registry, script validation, evidence types registered; test coverage expanding |
| WS3: Interception proxy types | In progress | Session lifecycle, filtering, CA management, HAR export types registered |
| WS4: Database assessment types | In progress | Driver registry, session types, credential providers, query execution types registered |
| WS5: Network programmability | In progress | Target resolution, managed sessions, probes, HTTP client, WebSocket types registered |
| WS6: Policy & execution context | In progress | EnforcementContext, OperationRegistry, PreflightResult, audit types registered |
| WS7: Finding workflow & storage | In progress | FindingRepository, AssessmentRepository, BaselineComparator, compliance types registered |
| WS8: Domain registry & events | In progress | Domain registry, versioned event protocol, callback/sink contracts registered |
| WS9: Streaming reporting | Closed | StreamingReporter, StreamingDiffReporter, ReportManifest with artifact integration tests |
| WS10: Capability metadata | Closed | `_capabilities.json` validated against 22-operation registry; domain-maturity.md operational evidence added |
| WS11: Type stubs & API surface | In progress | Type stubs generated; runtime `api_surface()` introspection available |
| WS12: Release fixtures & validation | In progress | Loopback fixtures, wheel smoke tests, architecture guards |

WS9 closure evidence: 40+ streaming operational tests covering config
construction, incremental finding writes, buffer flush, summary generation,
severity distribution, large-volume handling (1000+ findings), output formats
(JSON/JSONL/CSV/Markdown), cancellation with partial report consistency,
secret redaction configuration, diff reporter with baseline comparison
(new/unchanged/changed finding tracking), ReportManifest construction with
artifact references and content hash verification.

WS10 closure evidence: `_capabilities.json` version 2 schema validated against
the twenty-two-operation stable registry. All operation entries include
`last_validated_commit`, `installed_wheel`, `direct_function_delegates`, and
`test_fixture` fields. Domain maturity table cross-referenced with operation
metadata. Streaming reporting types (`StreamingReporter`, `ReportDiff`,
`ReportManifest`) added to Release 4 provisional table.

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
