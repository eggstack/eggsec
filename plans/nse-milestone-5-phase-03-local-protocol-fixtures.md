# NSE Milestone 5 Phase 03: Local Protocol Fixtures

## Purpose

Move protocol compatibility validation from mocked/script-only patterns toward deterministic local service fixtures.

Milestone 4 introduced upstream-style fixtures and mocked protocol behavior. Phase 03 adds real local-only protocol fixtures for HTTP, TCP, UDP, DNS-like behavior, TLS/certificate parsing, and controlled process/filesystem denial paths. The goal is higher confidence without public internet.

## Non-Goals

Do not require public internet.

Do not run intrusive or brute-force behavior against external services.

Do not implement full protocol parity.

Do not migrate deferred libraries in this phase unless a small wrapper is required for fixture execution.

## Workstream 1: Local Fixture Harness

### Required Capability

Add reusable test helpers for local services:

- dynamic localhost TCP listener;
- simple HTTP response server;
- UDP echo responder;
- DNS-like UDP responder or resolver-denial fixture;
- TLS fixture using committed test cert material or generated local certs if deterministic;
- shutdown coordination that does not leak threads.

### Acceptance Criteria

- Local services bind only `127.0.0.1` or `[::1]`.
- Ports are dynamically assigned.
- Tests do not sleep unboundedly.
- Services shut down deterministically.

## Workstream 2: HTTP Fixtures

Add runtime fixtures for:

- HTTP title extraction;
- HTTP header capture;
- HTTP GET with body;
- HTTP POST shape if supported;
- denied HTTP/network under AgentSafe/CiSafe when out of scope.

### Acceptance Criteria

- HTTP fixtures use actual local socket I/O when the library path supports it.
- If HTTP library remains partially mocked, docs and matrix mark it `runtime-observed-optional` or `partial`, not `runtime-strict`.

## Workstream 3: TCP/UDP Fixtures

Add fixtures for:

- TCP connect success to local listener;
- TCP connect denied by AgentSafe/CiSafe out-of-scope policy;
- UDP send/receive local fixture;
- network operation counter increments;
- capability events for denied operations.

### Acceptance Criteria

- Network counters and capability events are asserted from observed runtime report fields.
- No test contacts a public address.

## Workstream 4: DNS-Like Fixtures

Depending on current DNS abstraction, choose one:

1. local UDP DNS responder for simple A/PTR-style responses;
2. deterministic resolver stub injected into tests;
3. denial-path-only fixtures until DNS resolver injection exists.

### Acceptance Criteria

- DNS fixture status is explicit in the compatibility matrix.
- If DNS is denial-only, do not claim full DNS runtime support.

## Workstream 5: TLS/Certificate Fixtures

Add fixtures for:

- certificate parsing from local fixture bytes;
- local TLS handshake if the wrapper/library supports it;
- TLS unsupported/partial reporting if not.

### Acceptance Criteria

- TLS cert evidence is deterministic.
- TLS protocol gaps remain documented.

## Workstream 6: Corpus Manifest Integration

Extend manifest metadata for protocol fixtures:

```toml
[local_service]
type = "http"
bind = "127.0.0.1"
response_fixture = "fixtures/http/title_basic.txt"

[harness]
execute = true
requires_local_service = true
```

### Acceptance Criteria

- Runtime harness starts required local services based on manifest metadata or named test helpers.
- Fixture expectations assert observed report fields, not service metadata alone.

## Workstream 7: Documentation and Guards

Update:

- `docs/NSE_COMPATIBILITY.md` protocol entries;
- `architecture/nse_integration.md` Milestone 5 section;
- architecture guards to prevent public-network fixture flags.

Add guard:

- fail if runtime corpus manifest has `public_network_required = true`;
- fail if local service fixtures hardcode non-loopback public targets.

## Verification

Run:

```bash
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- protocol --test-threads=1
cargo test -p eggsec-nse --features nse --test runtime_smoke_tests
cargo test -p eggsec-nse --features nse
bash scripts/check-architecture-guards.sh
```

## Final Acceptance Criteria

Phase 03 is complete when:

- At least HTTP and TCP local runtime fixtures exist and are asserted strictly.
- UDP/DNS/TLS fixture status is implemented or explicitly deferred.
- No public internet is required.
- Compatibility matrix differentiates mocked, denial-only, and real local protocol validation.