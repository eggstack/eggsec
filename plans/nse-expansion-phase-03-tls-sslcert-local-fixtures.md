# NSE Expansion Phase 03: TLS and sslcert Local Fixtures

## Purpose

Add deterministic local TLS/sslcert fixture coverage without claiming full TLS or Nmap `ssl` parity.

This is the next safest protocol expansion because it can be tested against local certificates and local listeners without external dependencies or public network access.

## Non-Goals

Do not implement full TLS scanner parity.

Do not add public-internet certificate tests.

Do not broaden into SSH/SMB/database protocols.

Do not relax automated profile network policy.

## Workstream 1: Audit Current TLS/SSL Surface

Inspect:

- `ssl`/`sslcert` library registration;
- TLS wrapper paths;
- certificate parsing helpers;
- current registry status;
- current tests and fixtures;
- capability events emitted by TLS operations.

### Acceptance Criteria

- Current supported vs stubbed TLS behavior is documented before new fixtures are added.

## Workstream 2: Local Certificate Fixtures

Add deterministic certificate fixtures:

- committed test PEM certificate and key, if acceptable;
- or generated deterministic test material if already supported;
- fixture metadata documenting that certs are test-only.

Avoid runtime certificate generation if it introduces nondeterminism or extra dependencies.

### Acceptance Criteria

- Test cert material is local, deterministic, and clearly marked test-only.
- No private production key material is included.

## Workstream 3: Local TLS Listener

Add a local TLS test listener if current dependencies support it.

Requirements:

- binds only `127.0.0.1`;
- dynamic port;
- deterministic shutdown;
- short timeouts;
- no public network;
- works under normal test parallelism or documents serialization.

If a local TLS listener is too much for this phase, implement certificate parsing fixtures first and explicitly defer handshake fixtures.

### Acceptance Criteria

- Local listener is reliable or handshake tests are explicitly deferred.

## Workstream 4: Fixture Scripts

Add NSE fixture scripts for:

- certificate metadata extraction;
- TLS handshake success under ManualPermissive if supported;
- automated profile denial under AgentSafe/CiSafe;
- unsupported/partial path if handshake is not implemented.

### Acceptance Criteria

- Scripts are local-only and small.
- Expected compatibility status is accurate.

## Workstream 5: Runtime Tests

Add tests that assert:

- ManualPermissive certificate/TLS fixture succeeds or reports partial with precise reason;
- AgentSafe/CiSafe denied network behavior emits capability events;
- evidence/report fields contain certificate-related observations only when observed;
- no public network is contacted.

### Acceptance Criteria

- Tests prove actual observed runtime behavior.
- Partial behavior is truthfully labeled.

## Workstream 6: Compatibility Docs

Update `docs/NSE_COMPATIBILITY.md`:

- mark `ssl`/`sslcert` status by enforcement and fidelity;
- distinguish certificate parsing from handshake behavior;
- list fixture IDs;
- state remaining gaps.

## Workstream 7: Guards

Add guards to prevent:

- public TLS targets in fixtures;
- hardcoded public hosts;
- committed non-test private key language;
- TLS tests without local fixture metadata.

## Verification

Run:

```bash
cargo test -p eggsec-nse --features nse --test local_protocol_tests
cargo test -p eggsec-nse --features nse --test runtime_corpus_tests -- --test-threads=1
cargo test -p eggsec-nse --features nse
bash scripts/check-architecture-guards.sh
cargo fmt --all --check
cargo clippy --lib -p eggsec-nse --features nse
```

## Final Acceptance Criteria

Phase 03 is complete when:

- TLS/sslcert current behavior is documented;
- local cert fixtures exist or are explicitly deferred with rationale;
- runtime tests cover certificate/TLS behavior without public network;
- automated profile denial remains enforced;
- compatibility matrix reflects observed behavior only.
