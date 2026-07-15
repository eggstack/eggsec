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
