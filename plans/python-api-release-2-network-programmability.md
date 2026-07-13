# Eggsec Python API Release 2 — Network Programmability

## Handoff objective

Release 2 exposes reusable network and protocol primitives so Python users can build custom scanners, probes, protocol assessments, and traffic-analysis workflows on Eggsec internals.

The release should make Eggsec useful as a Python security library rather than only as a catalog of predefined assessment operations. It must do so without exposing unstable Rust runtime details, bypassing policy, or becoming an undifferentiated replacement for general-purpose Python networking libraries.

The intended result is a focused, security-oriented substrate built from Eggsec-owned transports, parsers, timers, evidence models, artifacts, policy gates, events, cancellation, and structured errors.

## Preconditions

Release 2 should begin only after Release 1 establishes:

- an authoritative capability manifest;
- an enlarged canonical operation registry;
- stable registry/executor conventions;
- mature pipeline dependencies, retries, timeouts, bounded parallelism, events, and checkpoints;
- synchronized exports, type stubs, feature metadata, and maturity documentation.

Release 2 may add new engine operations, but its defining work is low-level programmability.

## Release outcome

At completion, Python users should be able to:

- resolve targets using Eggsec resolution policy;
- establish managed TCP, UDP, TLS, HTTP, and WebSocket sessions;
- run individual protocol probes;
- inspect timing and connection metadata;
- capture structured evidence and transcripts;
- stream packet and message data with bounded buffering;
- parse packet and protocol layers;
- run controlled active probes;
- compose custom checks in sync and async Python code;
- integrate custom checks into Eggsec events, findings, artifacts, and pipelines;
- preserve scope, authorization, cancellation, timeout, and cleanup semantics.

## Scope

### Mandatory areas

1. shared network/session foundations;
2. DNS, TCP, UDP, TLS, banner, and HTTP probes;
3. security-oriented HTTP client/session API;
4. complete WebSocket session API;
5. packet capture lifecycle and streaming;
6. packet decoding and flow aggregation;
7. controlled active probing;
8. evidence, transcript, artifact, and event integration;
9. sync/async ergonomics and typing;
10. deterministic fixtures and performance validation.

### Conditional areas

The following may be included where native support is sufficiently mature:

- packet replay;
- PCAPNG writing;
- raw frame construction;
- raw packet injection;
- protocol-specific fuzz helpers;
- WebSocket frame-level mutation.

Raw injection and mutation must remain experimental unless privilege, policy, cleanup, and cross-platform semantics are fully defined.

### Explicit non-goals

Release 2 does not aim to:

- replace `httpx`, `aiohttp`, `requests`, Scapy, or general socket libraries;
- expose Tokio runtime handles or Rust socket objects directly;
- provide arbitrary unrestricted packet injection by default;
- complete the interception proxy lifecycle;
- complete full NSE runtime programmability;
- complete daemon transport parity;
- stabilize mobile dynamic or browser session APIs;
- add unrelated offensive features.

## API design principles

### Managed resources

Connections, captures, streams, and sessions must have deterministic lifecycle methods and context-manager support.

### Async canonicality

Native async implementations are canonical. Sync APIs use a controlled façade without nested-runtime hazards.

### Stable DTO boundary

Python sees stable request, response, timing, evidence, message, packet, flow, transcript, and error DTOs rather than internal Rust transport types.

### Bounded data movement

Streaming APIs use bounded queues, explicit backpressure policy, lazy artifacts, iterators, async iterators, and buffer protocol support where appropriate.

### Security-oriented semantics

The API exposes exact metadata needed for assessment work: handshake details, raw and normalized headers, duplicate headers, timing, certificates, protocol negotiation, socket endpoints, redirects, transcripts, packet layers, and evidence references.

### Policy preservation

Active network work remains scope-checked and policy-governed. Low-level primitives are not an escape hatch around the operation model.

## Workstream 1 — Shared network configuration and metadata

### Goal

Create a common set of Python-native configuration and result primitives used across transports and probes.

### Proposed types

- `Target`
- `ResolvedTarget`
- `ResolutionResult`
- `ConnectionConfig`
- `TimeoutConfig`
- `RetryPolicy`
- `ProxyRoute`
- `SocketEndpoint`
- `ConnectionTiming`
- `ConnectionMetadata`
- `NetworkEvidence`
- `TranscriptEntry`
- `NetworkTranscript`

### Requirements

#### Target normalization

Support hostnames, IP addresses, URLs where relevant, IPv4, IPv6, explicit ports, and normalized scope identity.

Do not silently resolve a hostname and then skip scope validation of the resolved addresses. The policy model must define whether authorization is evaluated against the original name, resolved addresses, or both.

#### Resolution

Expose:

- A/AAAA resolution;
- resolver source metadata;
- canonical names where available;
- resolution timing;
- timeout;
- cancellation;
- structured resolver errors;
- bounded result counts;
- optional caching consistent with Eggsec configuration.

#### Timeouts

Distinguish:

- connect timeout;
- read timeout;
- write timeout;
- handshake timeout;
- operation timeout;
- idle timeout.

Do not collapse every timeout into one ambiguous integer.

#### Retries

Define retryable error classes and ensure retries do not bypass operation-level deadlines or policy gates.

#### Metadata

Preserve:

- local endpoint;
- remote endpoint;
- resolved address;
- address family;
- transport protocol;
- negotiated protocol;
- connection reuse;
- timing fields;
- proxy route;
- TLS state;
- byte counts.

### Likely files

- new `crates/eggsec-python/src/network.rs`
- new `crates/eggsec-python/src/transport.rs`
- existing `config_model.rs`
- existing `scope.rs` and `scope_eval.rs`
- existing `status.rs`, `artifact.rs`, and `event_protocol.rs`
- Python façade modules and stubs

## Workstream 2 — Managed TCP sessions

### Goal

Expose a safe, reusable TCP connection primitive for custom probes.

### Proposed API

- `TcpConfig`
- `TcpSession`
- `TcpConnectResult`
- `TcpReadResult`
- `TcpWriteResult`

### Required behavior

- sync context manager;
- async context manager;
- explicit `close()` / `aclose()`;
- connect with structured metadata;
- bounded read;
- exact read;
- read-until with maximum length;
- write and write-all;
- half-close where portable;
- idle timeout;
- cancellation;
- transcript capture;
- byte counters;
- deterministic cleanup;
- secret-safe representations.

### Constraints

Do not expose a raw file descriptor as the primary API. A controlled escape hatch may be considered experimental only if ownership and cleanup semantics are unambiguous.

### Evidence integration

Reads and writes may optionally produce transcript entries and artifact references. Large payloads should not be copied repeatedly into Python objects.

## Workstream 3 — Managed UDP exchanges

### Goal

Expose UDP primitives suitable for service probing and protocol-specific request/response workflows.

### Proposed API

- `UdpConfig`
- `UdpSocket`
- `UdpDatagram`
- `UdpExchangeResult`

### Required behavior

- connected and unconnected modes;
- send/send-to;
- receive/receive-from;
- maximum datagram size;
- timeout;
- cancellation;
- source/destination metadata;
- truncated-datagram indication where supported;
- bounded multi-response collection;
- transcript capture;
- deterministic cleanup.

### Policy

Broadcast, multicast, and wide-range target behavior must be separately classified and not enabled implicitly.

## Workstream 4 — Protocol probe primitives

### Goal

Expose focused one-shot probes that reuse the managed transport layer and return structured evidence.

### Mandatory probes

- `resolve_target`
- `tcp_connect_probe`
- `banner_probe`
- `udp_probe`
- `dns_query`
- `tls_probe`
- `http_probe`

### Probe contract

Every probe should provide:

- typed request/config DTO;
- typed result DTO;
- structured error;
- timing metadata;
- connection metadata;
- evidence references;
- timeout;
- cancellation;
- sync and async functions;
- policy/scope enforcement;
- deterministic fixture coverage.

### Banner probe

Support:

- passive read after connect;
- optional initial payload;
- maximum banner length;
- read timeout;
- raw bytes and decoded best-effort text;
- encoding metadata;
- fingerprint evidence handoff.

### DNS query

Support:

- common record types;
- UDP with TCP fallback;
- resolver selection;
- response code;
- authoritative/truncated flags;
- raw message artifact where useful;
- parsed records;
- timing and retries.

### TLS probe

Support:

- SNI;
- ALPN;
- protocol version controls;
- certificate chain metadata;
- cipher suite;
- negotiated version;
- hostname verification result;
- certificate verification result;
- handshake timing;
- raw certificate artifacts;
- explicit insecure mode guarded by configuration and documentation.

### Engine integration

Low-level probes may be exposed as stable functions or lightweight operations depending on policy and orchestration needs. Remote active probes should still have canonical operation descriptors even when a session API also exists.

## Workstream 5 — Security-oriented HTTP session API

### Goal

Expose the HTTP transport behavior required by Eggsec assessments and custom Python checks.

### Proposed types

- `HttpClient`
- `AsyncHttpClient`
- `HttpRequest`
- `HttpResponse`
- `HttpHeaders`
- `HttpCookie`
- `RedirectHistory`
- `HttpTiming`
- `HttpTranscript`
- `HttpBodyStream`

### Required request capabilities

- method;
- URL;
- duplicate-preserving headers;
- query parameters;
- bytes, text, JSON, form, and streaming body;
- cookies;
- proxy route;
- TLS configuration;
- redirect policy;
- per-request timeout overrides;
- response size limit;
- decompression controls;
- HTTP version preference where supported;
- connection reuse controls;
- transcript/artifact capture;
- redaction policy.

### Required response capabilities

- status;
- reason where available;
- raw ordered headers;
- normalized header lookup;
- duplicate headers;
- cookies;
- final URL;
- redirect history;
- protocol version;
- TLS metadata;
- timing breakdown;
- body bytes;
- bounded text decoding;
- streaming body iterator;
- content length and transferred bytes;
- truncation indication;
- artifact reference for large bodies.

### Session behavior

- connection pooling;
- bounded per-host concurrency;
- global concurrency limit;
- deterministic close;
- sync and async context managers;
- cancellation of in-flight requests;
- no hidden global client state;
- secret-safe request representations;
- configurable cookie persistence.

### Redaction

Default transcript serialization should redact:

- `Authorization`;
- proxy authorization;
- cookies where configured;
- API keys and tokens in known headers;
- configured query parameters;
- configured JSON/form fields.

Raw access should require an explicit opt-in and must never leak into `repr`, events, checkpoints, or default reports.

### Relationship to existing operations

Existing endpoint, WAF, GraphQL, OAuth, auth, browser, and recon modules should migrate toward this common HTTP substrate where technically appropriate. Do not rewrite all domains in Release 2 solely for uniformity; prioritize new public primitives and prevent new divergence.

## Workstream 6 — WebSocket session API

### Goal

Close the current WebSocket feature/API gap and expose both low-level session control and a policy-governed assessment operation.

### Proposed types

- `WebSocketConfig`
- `WebSocketSession`
- `AsyncWebSocketSession`
- `WebSocketHandshake`
- `WebSocketMessage`
- `WebSocketFrame`
- `WebSocketCloseInfo`
- `WebSocketTranscript`
- `WebSocketAssessmentRequest`
- `WebSocketAssessmentResult`

### Required session behavior

- connect over `ws` and `wss`;
- custom headers;
- cookies;
- origin;
- subprotocol negotiation;
- proxy and TLS configuration;
- text send;
- binary send;
- receive;
- ping;
- pong;
- close;
- message iterator;
- async message iterator;
- maximum message size;
- idle timeout;
- cancellation;
- transcript capture;
- close code/reason;
- deterministic cleanup.

### Frame-level behavior

Expose frame-level details only where the native implementation can do so without breaking protocol invariants. Arbitrary malformed-frame generation belongs in an experimental assessment/fuzzing API.

### Assessment operation

Add canonical `websocket_assess` operation with checks such as:

- handshake and TLS metadata;
- origin validation;
- authentication behavior;
- subprotocol handling;
- unauthenticated message access;
- message-size handling;
- close behavior;
- selected malformed-input checks under elevated policy gates.

### Validation

Fixtures must cover:

- plain and TLS WebSocket servers;
- text and binary messages;
- subprotocol negotiation;
- ping/pong;
- close handshake;
- timeout;
- cancellation;
- oversized messages;
- transcript redaction;
- async iteration cleanup.

## Workstream 7 — Packet capture lifecycle

### Goal

Turn packet-inspection bindings into a managed streaming capture API.

### Proposed types

- `CaptureConfig`
- `CaptureSession`
- `AsyncCaptureSession`
- `CapturedPacket`
- `PacketTimestamp`
- `CaptureStats`
- `CaptureDropStats`
- `PacketStream`
- `PacketArtifact`

### Required behavior

- list interfaces;
- validate interface and privileges;
- compile filters;
- start capture;
- stop capture;
- packet iterator;
- async packet iterator;
- packet count limit;
- byte limit;
- duration limit;
- idle timeout;
- cancellation;
- bounded queue;
- configurable backpressure policy;
- drop accounting;
- live statistics;
- PCAP persistence;
- deterministic cleanup.

### Backpressure

Define and test at least:

- block producer where supported;
- drop newest;
- drop oldest;
- artifact-only persistence with sampled Python delivery.

Reliable terminal events and final statistics must not be dropped.

### Privilege behavior

Privilege requirements must fail with a structured capability/permission error. Do not allow platform-specific panics or ambiguous empty captures.

## Workstream 8 — Packet decoding and flow aggregation

### Goal

Expose structured packet-layer inspection without requiring users to parse raw bytes manually.

### Proposed types

- `LinkLayer`
- `NetworkLayer`
- `TransportLayer`
- `ApplicationHint`
- `EthernetFrame`
- `Ipv4Packet`
- `Ipv6Packet`
- `TcpSegment`
- `UdpDatagramInfo`
- `IcmpPacket`
- `DnsPacket`
- `TlsRecordInfo`
- `FlowKey`
- `FlowRecord`
- `FlowAggregator`

### Requirements

- preserve raw bytes through buffer protocol or lazy artifact;
- expose parsed fields as stable DTOs;
- identify truncation and malformed layers;
- never panic on malformed input;
- support offline PCAP parsing;
- support incremental flow aggregation;
- expose packet-to-flow correlation;
- bound flow-table memory;
- define eviction behavior;
- provide serialization suitable for findings and artifacts.

### Parser errors

Malformed packets should return partial decode information and structured diagnostics where possible, not collapse the entire capture.

## Workstream 9 — Controlled active probing

### Goal

Expose safe, policy-governed active probes used by network discovery and diagnosis.

### Mandatory candidates

- ICMP echo probe;
- TCP SYN probe;
- TCP ACK probe where supported;
- UDP reachability probe;
- traceroute primitives;
- response correlation.

### Proposed types

- `IcmpProbeConfig`
- `IcmpProbeResult`
- `TcpProbeConfig`
- `TcpProbeResult`
- `UdpReachabilityConfig`
- `UdpReachabilityResult`
- `TracerouteConfig`
- `TracerouteHop`
- `TracerouteResult`

### Requirements

- explicit privilege detection;
- scope validation;
- rate limits;
- retry limits;
- bounded target counts;
- timeout;
- cancellation;
- response correlation;
- timing;
- packet evidence references;
- platform capability metadata;
- structured unsupported-platform behavior.

### Raw injection boundary

Arbitrary packet construction and injection must be placed under an experimental feature and namespace until:

- ownership and cleanup are clear;
- platform support is documented;
- privilege behavior is deterministic;
- policy metadata captures risk;
- scope enforcement covers spoofed fields;
- tests prove no stable API path bypasses safeguards.

## Workstream 10 — Evidence, transcript, and artifact integration

### Goal

Make low-level primitives produce evidence that can flow into the common finding and reporting model.

### Requirements

- every probe may return structured `NetworkEvidence`;
- sessions may optionally collect a bounded transcript;
- large bodies, captures, certificates, and binary messages use artifact references;
- artifacts include content type, size, hash, origin, and redaction metadata;
- transcript entries have monotonic sequence and timestamps;
- default serialization is secret-safe;
- users can convert evidence into `VersionedEvidence` and findings;
- pipeline steps can pass artifact references without loading full contents.

### Artifact thresholds

Define configurable thresholds for when data remains inline versus externalized. Ensure thresholds are deterministic and included in relevant compatibility metadata where checkpoint behavior depends on them.

## Workstream 11 — Events and streaming integration

### Goal

Apply the governed event protocol to sessions, captures, probes, and long-running network work.

### Events

Add or reuse events for:

- resolution started/completed;
- connection started/completed;
- handshake completed;
- request sent;
- response headers received;
- body progress;
- WebSocket message;
- capture started;
- packet sampled;
- capture statistics;
- flow observed;
- probe response;
- artifact created;
- cancellation;
- failure;
- completion.

### Requirements

- high-frequency events must be filterable;
- sampling must be explicit;
- queue depth must be bounded;
- event delivery stats must account for drops by kind;
- reliable terminal events remain exactly-once in the in-process model;
- callback failures are structured and do not leak resources;
- secret-bearing data is excluded by default.

## Workstream 12 — Python ergonomics and typing

### Context managers

All managed resources should support the appropriate protocol:

```python
with eggsec.TcpSession.connect(...) as session:
    ...

async with eggsec.AsyncHttpClient(...) as client:
    ...
```

### Iteration

Use iterators and async iterators for:

- streaming HTTP bodies;
- WebSocket messages;
- captured packets;
- flow records;
- transcript entries where large.

### Buffer protocol

Raw packet, frame, body, and message bytes should reuse `BinaryBuffer` or a compatible PEP 3118 implementation to minimize copies.

### Typing

Add:

- complete `.pyi` coverage;
- `Protocol` definitions for sinks and adapters;
- typed message variants;
- typed packet-layer variants;
- overloads for sync/async constructors;
- optional generic stream/result typing where practical;
- mypy and pyright example suites.

### Exceptions

Reuse the existing structured exception hierarchy and `OperationError` mapping. Add narrowly scoped network/protocol codes rather than proliferating unrelated Python exception classes.

## Workstream 13 — Fixtures and test infrastructure

### Required deterministic fixtures

Create managed local fixtures for:

- DNS UDP and TCP fallback;
- TCP banner service;
- UDP echo/service response;
- HTTP/1.1 keep-alive;
- duplicate headers;
- redirects;
- chunked/streaming bodies;
- response truncation;
- TLS certificate chain and ALPN;
- plain WebSocket;
- TLS WebSocket;
- packet fixture files;
- capture tests where CI privileges permit;
- active-probe loopback tests.

### Test layers

#### Pure parser tests

Use stored byte fixtures and PCAP files. These must run without privileges.

#### Loopback integration tests

Use managed local services with explicit loopback fixture authorization.

#### Privileged capture/probe tests

Run in separate opt-in CI jobs with clear skip semantics outside supported environments. Privileged tests must not be the sole validation of parsers or high-level contracts.

#### Installed-wheel tests

Build and install wheels, then test public imports, context managers, probes, WebSocket, parser fixtures, and feature metadata.

## Workstream 14 — Performance and resource budgets

### Metrics

Track:

- TCP connect overhead;
- HTTP request overhead versus native Rust baseline;
- connection pooling behavior;
- WebSocket message throughput;
- Python callback overhead;
- packet iteration throughput;
- packet decode throughput;
- buffer copy counts where measurable;
- event throughput;
- memory growth under long streams;
- queue saturation behavior;
- capture drop accounting;
- cancellation latency;
- cleanup latency.

### Resource guarantees

Long-running streams must not exhibit unbounded growth when:

- Python consumers are slow;
- callbacks raise exceptions;
- cancellation occurs during blocking I/O;
- remote peers stall;
- body or message limits are reached;
- capture queues overflow.

## Suggested implementation sequence

1. shared target, timeout, retry, metadata, transcript, and evidence DTOs;
2. target resolution primitive;
3. managed TCP session;
4. managed UDP session;
5. banner, DNS, and TLS probes;
6. HTTP request/response DTOs;
7. async HTTP client and pooling;
8. sync HTTP façade;
9. HTTP transcript, redaction, and artifact integration;
10. WebSocket handshake/session API;
11. `websocket_assess` operation;
12. offline packet parser and layer DTOs;
13. capture session lifecycle and streaming;
14. flow aggregation;
15. controlled active probes;
16. events, typing, docs, examples, and release closure.

The HTTP and WebSocket work should reuse the same target, TLS, proxy, timeout, transcript, and artifact foundations rather than defining independent copies.

## Likely repository areas

Implementation is expected to touch:

- `crates/eggsec-python/src/lib.rs`
- new network/transport/session modules under `crates/eggsec-python/src/`
- `crates/eggsec-python/src/config_model.rs`
- `crates/eggsec-python/src/scope.rs`
- `crates/eggsec-python/src/scope_eval.rs`
- `crates/eggsec-python/src/operation_registry.rs`
- `crates/eggsec-python/src/requests.rs`
- `crates/eggsec-python/src/status.rs`
- `crates/eggsec-python/src/artifact.rs`
- `crates/eggsec-python/src/buffer_support.rs`
- `crates/eggsec-python/src/event_protocol.rs`
- `crates/eggsec-python/src/event_stream.rs`
- `crates/eggsec-python/src/packet_inspection.rs`
- underlying reusable modules in `crates/eggsec/` and `crates/eggsec-core/`
- Python façade and `.pyi` files;
- `docs/python/`;
- wheel and architecture validation scripts.

If the needed native primitives are embedded in CLI-oriented code, refactor them into reusable Rust library modules before binding them. Do not reproduce transport logic solely inside the PyO3 crate.

## Validation plan

### Build matrix

At minimum:

```bash
cargo fmt --all -- --check
cargo check -p eggsec-python
cargo check -p eggsec-python --features websocket
cargo check -p eggsec-python --features packet-inspection
cargo check -p eggsec-python --features websocket,packet-inspection
cargo test -p eggsec-python
bash scripts/check-architecture-guards.sh
```

Include platform-specific compilation checks for capture and active-probe code.

### Contract tests

Every public session/probe must test:

- valid construction;
- invalid configuration;
- feature unavailable behavior;
- scope denial;
- policy denial where applicable;
- timeout;
- cancellation;
- deterministic close;
- double-close behavior;
- context-manager cleanup;
- async context-manager cleanup;
- structured error mapping;
- metadata population;
- transcript behavior;
- redaction;
- artifact externalization;
- type-stub parity;
- installed-wheel behavior.

### Streaming tests

Test:

- slow consumer;
- queue saturation;
- drop accounting;
- consumer exception;
- cancellation during iteration;
- producer failure;
- terminal event delivery;
- resource release;
- large payloads;
- repeated open/close cycles.

### Security tests

Test:

- DNS rebinding or resolution changes against scope policy;
- redirects to out-of-scope targets;
- proxy routes that could bypass scope;
- TLS insecure-mode labeling;
- secret headers and cookies absent from `repr` and default transcripts;
- oversized bodies/messages/datagrams;
- malformed packet parsing;
- raw injection unavailable without explicit feature and policy;
- no arbitrary file overwrite through artifact or PCAP paths.

## Documentation and examples

### Required guides

- network primitive overview;
- target resolution and scope semantics;
- timeout and retry model;
- TCP and UDP sessions;
- custom banner probe;
- TLS inspection;
- HTTP client and redaction;
- WebSocket sessions and assessment;
- packet parsing;
- capture lifecycle and privileges;
- flow aggregation;
- active probing and policy;
- evidence and artifact conversion;
- async streaming and cancellation.

### Required executable examples

- resolve and connect to a scoped target;
- custom TCP banner check;
- DNS query with TCP fallback;
- TLS handshake inspection;
- async HTTP assessment with connection reuse;
- streaming HTTP body to an artifact;
- WebSocket text/binary exchange;
- WebSocket assessment operation;
- offline PCAP parsing;
- live capture where supported;
- flow aggregation;
- controlled traceroute or ICMP probe;
- custom finding built from `NetworkEvidence`.

## Risks and mitigations

### Risk: API becomes a general networking framework

Mitigation: expose only primitives required by Eggsec assessment workflows and preserve security-specific metadata and policy integration.

### Risk: sync wrappers create nested-runtime failures

Mitigation: use the established runtime bridge and test sync calls inside and outside existing event loops where supported.

### Risk: Python streaming causes unbounded memory growth

Mitigation: bounded queues, explicit drop/block policy, lazy artifacts, and stress tests with slow consumers.

### Risk: scope validation is bypassed after DNS resolution or redirect

Mitigation: define and test authorization across original target, resolved addresses, redirect destinations, proxy routes, and reconnects.

### Risk: secrets leak through transcripts

Mitigation: default redaction, secret-sentinel tests, explicit raw-access escape hatches, and exclusion from events/checkpoints/reports.

### Risk: platform-specific packet APIs destabilize wheels

Mitigation: feature-gate capture/injection, separate parser-only support from privileged live capture, and publish accurate feature metadata.

### Risk: data copying destroys packet and body performance

Mitigation: buffer protocol, lazy artifacts, streaming, and copy/throughput benchmarks.

### Risk: raw packet APIs become an enforcement escape hatch

Mitigation: experimental namespace, explicit feature, privilege checks, target/scope enforcement, rate limits, and no default-wheel guarantee until hardened.

## Release acceptance criteria

Release 2 is complete only when:

- common target, timeout, retry, connection metadata, transcript, and evidence primitives are stable and documented;
- managed TCP and UDP sessions have sync/async lifecycle support and deterministic cleanup;
- DNS, banner, TLS, HTTP, and general transport probes are typed, policy-governed, cancellable, and tested;
- a security-oriented HTTP client supports pooling, raw/normalized headers, redirects, TLS metadata, streaming, limits, transcripts, artifacts, and redaction;
- the WebSocket Cargo feature maps to a complete typed Python session API;
- `websocket_assess` is a canonical operation with policy, events, cancellation, and fixtures;
- offline packet parsing exposes stable layer DTOs and never panics on malformed input;
- live capture has explicit lifecycle, bounded streaming, statistics, drop accounting, cancellation, and privilege errors;
- flow aggregation is bounded and tested;
- controlled active probes have explicit risk, privilege, rate, scope, timeout, and cancellation behavior;
- raw injection remains unavailable from stable APIs unless all graduation gates are met;
- large network data uses buffers, streams, lazy artifacts, or artifact references rather than uncontrolled Python object expansion;
- high-frequency events are filterable and backpressure-aware;
- mypy and pyright pass representative examples;
- installed-wheel tests validate the declared feature profiles;
- documentation accurately distinguishes stable primitives, provisional capabilities, and experimental raw access;
- Release 1 operation, pipeline, policy, event, checkpoint, and secret-handling guarantees remain intact.

## Handoff note

The implementation should prefer extracting reusable native Rust transport and parser components over building Python-specific duplicates. The public Python API should feel Pythonic, but the source of truth for network behavior, policy, parsing, and cleanup must remain the Rust library layer.