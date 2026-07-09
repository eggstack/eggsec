# Phase F Plan: Major Tool Expansion Through the Python API

## Objective

Expand the Python library from the core scanner/reporting/WAF/recon API toward broad coverage of Eggsec's major tool surface. This phase should proceed in staged module tracks, preserving scope enforcement, feature availability checks, packaging discipline, and documentation quality.

The purpose is eventual parity with major Eggsec capabilities from Python, not a single risky mega-merge.

## Dependencies

This phase assumes Phase E is complete:

- PyPI-ready packaging exists.
- Core wheels install cleanly.
- Docs and examples are in place.
- Sync and async APIs are established.
- Scope and mode behavior are tested.
- Reporting/finding integration exists.
- Feature introspection exists.

## Expansion principles

Each new tool module must follow the same public API rules:

- explicit `Client`/`AsyncClient` methods where appropriate
- stable Python DTOs rather than raw Rust structs
- `to_dict()` and `to_json()` on result types
- scope enforcement before network or active operations
- automation mode must not honor manual override paths
- missing optional capabilities raise `FeatureUnavailableError`
- docs and examples land with the binding
- tests use local fixtures where possible

Do not enable all features in the default wheel just because bindings exist. Optional features should remain optional in packaging.

## Track 1: WAF validation and HTTP fuzzing

Expose controlled active HTTP validation workflows after passive WAF detection is stable.

Candidate APIs:

```python
client.validate_waf(
    target="https://staging.example.com",
    profile="owasp-light",
    rate_limit_per_sec=2,
    max_requests=100,
)
```

```python
client.fuzz_http(
    target="https://staging.example.com/search",
    parameter="q",
    payload_set="xss-basic",
    rate_limit_per_sec=5,
    max_requests=100,
)
```

Required safety constraints:

- target must be in scope
- max request count required
- rate limit required or default conservative value
- named profiles instead of arbitrary unrestricted payload blasting by default
- result includes audit metadata

DTOs:

```python
WafValidationResult
FuzzResult
PayloadObservation
HttpObservation
```

Acceptance criteria:

- local/staging fixture tests exist
- rate limits and request caps are tested
- docs clearly distinguish validation from bypass/stress activity

## Track 2: HTTP load testing

Expose legitimate HTTP load testing before raw stress tooling.

Candidate API:

```python
client.load_test_http(
    url="https://staging.example.com",
    duration_s=30,
    concurrency=50,
    rate_limit_per_sec=500,
)
```

Result DTO fields:

```python
duration_s
requests_total
requests_per_second
status_counts
error_counts
latency_p50_ms
latency_p90_ms
latency_p95_ms
latency_p99_ms
latency_max_ms
samples
```

Required constraints:

- explicit duration
- explicit or bounded concurrency
- scoped URL
- conservative defaults
- no raw packet/stress features in this module

Acceptance criteria:

- local HTTP fixture load test works
- latency percentile math is tested
- examples use localhost or clearly scoped staging URLs

## Track 3: WebSocket testing

Expose WebSocket testing after HTTP primitives are stable.

Candidate APIs:

```python
client.websocket_probe("wss://example.com/socket")
client.websocket_fuzz(...)
```

Start with probe/handshake validation and metadata collection. Defer aggressive fuzzing until request caps, message caps, and rate limits are in place.

DTOs:

```python
WebSocketProbeResult
WebSocketMessageObservation
WebSocketFuzzResult
```

Acceptance criteria:

- local WebSocket fixture exists
- scoped URL enforcement works
- async APIs are available where appropriate

## Track 4: Git secrets and local artifact scanning

Expose local file/repo scanners that are useful in Python CI and notebooks.

Candidate APIs:

```python
client.scan_git_secrets(path=".")
client.scan_files_for_secrets(paths=["src", "config"])
```

These APIs may not require network scope, but they still require clear filesystem path validation.

DTOs:

```python
SecretFinding
SecretScanResult
FileFinding
```

Constraints:

- never print secret values by default
- redact sensitive evidence in repr/output
- expose explicit `include_secret_values=False` behavior only if absolutely needed
- document safe handling of findings

Acceptance criteria:

- fixture repository tests exist
- redaction is tested
- output integrates with `Report`

## Track 5: SBOM and supply-chain outputs

Expose SBOM generation and vulnerability/supply-chain inspection where available.

Candidate APIs:

```python
client.generate_sbom(path=".", format="cyclonedx")
client.scan_dependencies(path=".")
```

DTOs:

```python
SbomResult
DependencyFinding
PackageComponent
```

Acceptance criteria:

- fixture package manifests are tested
- JSON output is stable
- report integration works

## Track 6: Database testing modules

Expose database pentest primitives only after the core enforcement model is mature.

Candidate APIs:

```python
client.db.probe_postgres(...)
client.db.probe_mysql(...)
client.db.probe_mssql(...)
client.db.probe_mongodb(...)
client.db.probe_redis(...)
```

Packaging concern:

Database drivers may add native or large dependencies. Keep these out of the default wheel unless already present in the base dependency set. Prefer optional extras or separate wheel build profiles.

Safety constraints:

- scoped host enforcement
- explicit credential handling
- no credential logging
- no destructive queries by default
- automation mode strictness

Acceptance criteria:

- local containerized database fixtures if available
- credential redaction tests
- feature availability checks

## Track 7: Proxy and web proxy modules

Expose web proxy and proxy pool primitives after stable client/config patterns exist.

Candidate APIs:

```python
client.proxy.check_proxy(...)
client.proxy.test_chain(...)
client.web_proxy.start_local(...)
```

Be careful with long-running services from Python. Prefer context managers:

```python
with client.web_proxy.local_proxy(config) as proxy:
    ...
```

Acceptance criteria:

- lifecycle cleanup tests
- port binding conflict tests
- docs explain local service behavior

## Track 8: Mobile lab tooling

Expose mobile analysis primitives only where packaging remains tractable.

Candidate APIs:

```python
client.mobile.inspect_apk(path="app.apk")
client.mobile.inspect_ipa(path="app.ipa")
```

Start with static metadata extraction and findings. Defer dynamic/mobile-device workflows until the base module is stable.

Acceptance criteria:

- fixture APK/IPA or synthetic archive tests
- report integration
- no platform-specific failures in default wheel unless feature-gated

## Track 9: Cloud and container tooling

Expose cloud/container tooling as explicit optional modules.

Candidate APIs:

```python
client.container.scan_manifest(path="deployment.yaml")
client.cloud.scan_aws_config(...)
client.cloud.scan_kubernetes(...)
```

Packaging and credential handling are the main risks.

Constraints:

- no credential logging
- explicit profile/context selection
- dry-run/static modes first
- networked cloud calls optional and documented

Acceptance criteria:

- static fixture scans work without cloud credentials
- credential redaction tests
- feature availability docs

## Track 10: Packet inspection and raw network features

Expose packet inspection only after default scanner/load/WAF APIs are mature.

Candidate APIs:

```python
client.packet.inspect_pcap(path="capture.pcap")
client.packet.summarize_pcap(path="capture.pcap")
```

Start with offline PCAP inspection before live capture or packet crafting.

Live capture and packet crafting should require explicit optional features and clear privilege documentation.

Acceptance criteria:

- offline fixture PCAP tests
- default wheel does not require packet privileges
- live capture is feature-gated and documented separately

## Track 11: Stress testing

Stress tooling must remain explicit and feature-gated.

Candidate APIs should require:

- explicit scoped target
- explicit duration
- explicit rate/concurrency limits
- explicit acknowledgement for manual mode
- no automation manual overrides

Do not include stress testing in the default PyPI wheel.

Acceptance criteria:

- stress APIs unavailable by default and raise `FeatureUnavailableError`
- feature-enabled builds have tests against local fixtures only
- docs emphasize authorized lab/staging use

## Track 12: NSE Python submodule

Expose NSE through Python only after the Rust-native API is stable.

Candidate API:

```python
import eggsec.nse

engine = eggsec.nse.ScriptEngine(
    sandbox=True,
    scope=eggsec.Scope.allow_hosts(["example.com"]),
)

result = engine.run_script("http-title.nse", target="https://example.com")
```

Design constraints:

- Python must not bypass NSE sandboxing
- NSE is optional and not in the default wheel unless packaging policy changes
- sandbox mode should be the documented default
- `nse-ssh2`, OpenSSL, DES, and other optional dependencies must be explicit

Acceptance criteria:

- sandbox tests
- feature availability tests
- docs explain Python-hosted Rust-hosted Lua architecture
- no arbitrary Python execution inside Eggsec

## Track 13: Daemon client bindings

Expose Python as a daemon client only after direct library bindings are stable.

Candidate API:

```python
client = eggsec.DaemonClient.connect("ws://127.0.0.1:...")
await client.run_scan(...)
```

This is separate from the native library client. It should use runtime DTOs and daemon protocol types where appropriate.

Acceptance criteria:

- direct library client and daemon client are documented separately
- daemon client follows automation/API enforcement semantics
- connection lifecycle and errors are tested

## Documentation requirements for every track

Each new module must add or update:

```text
docs/python/<module>.md
examples/python/<module>_example.py
```

Each doc page must include:

- installation/feature requirement
- sync or async status
- scope requirements
- result object shape
- example
- known limitations

## Packaging requirements for optional modules

Every optional module must declare whether it is:

- included in default wheel
- included in optional wheel profile
- source-build only initially
- unsupported on some platforms

`eggsec.features()` must reflect runtime availability.

Optional module imports should fail cleanly:

```python
try:
    import eggsec.nse
except eggsec.FeatureUnavailableError:
    ...
```

or module methods should raise `FeatureUnavailableError` with a message naming the required feature.

## Testing strategy

Prefer local fixtures over external network dependencies.

Use synthetic fixture files for secrets, SBOM, mobile, container, and PCAP tests.

Use local TCP/HTTP/WebSocket servers for network tests.

Use containerized services only for optional database tests and keep them outside the default fast suite.

Separate test groups:

```text
python-core
python-network-local
python-optional-db
python-optional-nse
python-optional-packet
python-release-smoke
```

## Acceptance criteria for Phase F as a whole

A staged module expansion plan is implemented without making the default wheel fragile.

Each newly exposed major tool has stable DTOs, docs, examples, tests, and report integration where appropriate.

Dangerous or privilege-sensitive capabilities remain feature-gated and explicit.

NSE remains optional and sandbox-aware.

Python remains a host-language library surface, not an internal script runtime.

The public API remains organized by coherent modules rather than mirroring Rust internals indiscriminately.
