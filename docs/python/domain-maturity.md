# Python domain maturity

`eggsec-python` exposes more modules than the current stable execution
contract covers. Importability and Cargo feature availability do not imply a
compatibility guarantee.

The stable release boundary is the ten-operation engine registry:

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

All other domains are classified as `provisional` or `experimental` until
they satisfy the graduation checklist:

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
