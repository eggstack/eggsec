# Knowledge Corpus & Leaf Domain Crates

The four small crates that hold this repo's *knowledge* — the tables, patterns, and
payload strings — rather than executors. Three (`eggsec-service-db`, `eggsec-secrets`,
`eggsec-payloads`) were extracted out of the engine in Phase G of the capability
segregation program; `eggsec-udp-scan` was accepted as a domain crate a phase earlier
(Phase F). All four are workspace members (`Cargo.toml:20-25`), all four are
`publish = false`, and all four authorize nothing.

*Verified against source on 2026-10-06. Every count below carries a `file:line` cite.
Claims I could not verify from source are labelled **unverified**.*

## Crate map

| Crate | Role | Workspace deps | LOC / files | Deep dive |
|---|---|---|---|---|
| `eggsec-service-db` | Port→service tables + banner heuristics | **none** (`rustc-hash` only) | 316 LOC, 1 file | [below](#eggsec-service-db) |
| [`eggsec-secrets`](#eggsec-secrets) | Credential patterns, typed findings, entropy gate | `eggsec-core` | 520 LOC, 1 file | [below](#eggsec-secrets) |
| [`eggsec-payloads`](#eggsec-payloads) | Attack payload corpora, `PayloadType`, caches | `eggsec-core` | 8,849 LOC, 42 files | [below](#eggsec-payloads) |
| [`eggsec-udp-scan`](#eggsec-udp-scan) | UDP range scan + ICMP-error correlation | **none** (`libc`/`thiserror`/`tracing`) | 1,970 LOC, 6 files | [below](#eggsec-udp-scan) |

`eggsec-udp-scan` is included here because it is the fourth crate of this shape — a leaf
that owns a hard external contract (raw sockets) — even though it arrived via a different
decision record than the other three.

## The shared shape

Three properties recur across all four crates, and they are the reason they are separate
crates rather than engine modules:

1. **No authority.** None of them resolves DNS, reads a scope, constructs an approval
   bundle, or dispatches. `eggsec-udp-scan` goes further: it never *acquires* privilege, it
   only reports that privilege is required (`crates/eggsec-udp-scan/src/lib.rs:30-33`).
2. **The engine owns all I/O.** Each crate either takes a string/bytes/address as input
   (`service-db`, `secrets`, `udp-scan`'s pure modules) or is reached through a facade so
   the engine stays the only caller that does anything with the result.
3. **The facade is permanent, not scaffolding.** Each of the three corpus crates has a
   `pub use … as …` re-export at its pre-extraction path, and a guard fails if that line
   disappears.

---

## `eggsec-service-db`

### Purpose

Canonical single owner of service-fingerprinting *knowledge*. The scanner supplies a port
number and/or a banner string; this crate says what they mean. It opens no socket,
resolves no name, spawns no process (`src/lib.rs:9-12`).

### Data structures

| Item | Location | Detail |
|---|---|---|
| `COMMON_PORTS: &[(u16, &str)]` | `crates/eggsec-service-db/src/lib.rs:24` | **47 entries**, `lib.rs:25-72`; all 47 port numbers and all 47 service names distinct (counted) |
| `PORT_SERVICE_MAP: LazyLock<FxHashMap<u16, &'static str>>` | `lib.rs:74-75` | Built by collecting `COMMON_PORTS` on first touch |
| `get_service_name(port) -> &'static str` | `lib.rs:77` | Returns `"unknown"` on miss |
| `get_service_by_port(port) -> Option<&'static str>` | `lib.rs:81` | The `Option`-returning form |
| `guess_service_from_banner(banner) -> Option<&'static str>` | `lib.rs:85-136` | Lowercases once, then **14 ordered substring branches** (`return Some(…)` at `lib.rs:89,92,95,98,101,104,107,110,113,120,123,126,129,132`) |
| `guess_service(port, banner) -> String` | `lib.rs:138-145` | Banner first, port table as fallback — never the reverse |
| `is_web_service(port)` | `lib.rs:147-152` | 9 hardcoded ports |
| `is_database(port)` | `lib.rs:154-156` | 7 hardcoded ports |
| `is_mail_service(port)` | `lib.rs:158-160` | 7 hardcoded ports |

The `LazyLock` is load-bearing, not decorative: a process that never fingerprints a
service never pays to build the hash map (`lib.rs:18-19`). Only `PORT_SERVICE_MAP` is
lazy — `COMMON_PORTS` is a `&'static` slice, so it costs nothing to reference.

`guess_service_from_banner` is order-sensitive by design. `"SSH"` is tested first and
requires *both* `"ssh"` and `"version"` in the banner (`lib.rs:88`); the HTTP branch at
`lib.rs:115-121` is the widest (http/nginx/apache/iis) and sits below the specific
database and mail checks, so a MySQL banner never falls into it.

### Dependency posture

`Cargo.toml:22-25` declares exactly one dependency: `rustc-hash.workspace = true`. This is
held to a **stricter** bar than the other two corpus crates — guard 148 fails if it so much
as declares `eggsec-core` (`scripts/check-architecture-guards.sh:4214-4220`), on the
grounds that it does not even use `Severity`.

### How the engine bridges to it

`crates/eggsec/src/scanner/mod.rs:100` — `pub use eggsec_service_db as service_data;`. The
stable path is therefore `eggsec::scanner::service_data::*`, which is what keeps
`scanner/` and its consumers unchanged.

Two consumer shapes:

- A flat re-export of eight items at the `scanner` root (`crates/eggsec/src/scanner/mod.rs:116-119`):
  `get_service_by_port`, `get_service_name`, `guess_service`, `guess_service_from_banner`,
  `is_database`, `is_mail_service`, `is_web_service`, `COMMON_PORTS`, `PORT_SERVICE_MAP`.
- A thin alias inside the port scanner (`crates/eggsec/src/scanner/ports/mod.rs:32`,
  wrapped by a local `fn get_service_name` at `:36-38`), plus `COMMON_PORTS` used by the
  scanner's own duplicate-port test (`ports/mod.rs:953, 1073, 1080`).

---

## `eggsec-secrets`

### Purpose

Canonical owner of credential-*detection* knowledge: regex patterns, typed findings,
confidence tiers, and one entropy gate. Detecting a secret and masking one are explicitly
separate jobs (`src/lib.rs:13-16`).

### Key types and APIs

| Item | Location | Detail |
|---|---|---|
| `SecretFinding` | `crates/eggsec-secrets/src/lib.rs:34-41` | 6 fields: `secret_type`, `value_preview`, `location`, `confidence`, `severity`, `description` |
| `SecretType` | `lib.rs:44-75` | **30 variants**, `lib.rs:45-74` |
| `Confidence` | `lib.rs:78-82` | 3 tiers: `High`, `Medium`, `Low` |
| `pub use eggsec_core::types::Severity` | `lib.rs:84` | The crate's only engine edge |
| `impl Display for SecretType` | `lib.rs:86-121` | One arm per variant |
| `struct SecretPattern` (private) | `lib.rs:123-129` | `Regex` + type + confidence + severity + description |
| `build_patterns() -> Vec<SecretPattern>` | `lib.rs:131-337` | **25 pattern constructions** at `lib.rs:136,144,152,160,168,176,184,192,200,208,216,224,232,240,248,256,264,272,280,288,296,304,312,320,328` |
| `static PATTERNS: LazyLock<Vec<SecretPattern>>` | `lib.rs:339` | Compiled once, on first touch |
| `SecretScanner` | `lib.rs:341` | Unit struct; `new()` (`:350`) forces the `LazyLock` so a bad regex panics at construction, not mid-scan |
| `SecretScanner::scan(&str) -> Vec<SecretFinding>` | `lib.rs:356-384` | Runs every pattern with `find_iter` |
| `SecretScanner::scan_file(&str)` | `lib.rs:386-389` | **The crate's only I/O**: `std::fs::read_to_string` at `lib.rs:387` |
| `secret_entropy(&str) -> f64` (private) | `lib.rs:392-414` | Shannon entropy over byte counts; extracts the 40-char candidate at `lib.rs:393-396` |
| `scan_content(&str) -> Vec<SecretFinding>` | `lib.rs:416-419` | Free-function convenience wrapper |

### Pattern coverage — the count that matters

**25 patterns covering 20 of the 30 `SecretType` variants.** The 10 variants with no
pattern are `AzureKey`, `GcpServiceAccount`, `BitbucketToken`, `JwtToken`, `NpmToken`,
`PyPiToken`, `HerokuKey`, `NetlifyToken`, `DockerhubToken`, `KubernetesSecret`
(computed by comparing the `lib.rs:45-74` variant list against the
`secret_type: SecretType::…` assignments in `build_patterns`).

Three types carry multiple patterns, which is why 25 patterns map to 20 types:

| Type | Patterns |
|---|---|
| `DatabaseConnectionString` | 4 (`lib.rs:248, 256, 264, 272` — ADO-style, MongoDB URI, Postgres URI, MySQL URI) |
| `GithubToken` | 2 (`lib.rs:160` PAT, `lib.rs:168` `ghp_`) |
| `BasicAuth` | 2 (`lib.rs:240` `basic …` header, `lib.rs:296` credentials-in-URL) |
| the other 17 | 1 each |

`value_preview` truncates at 20 characters plus an ellipsis (`lib.rs:366-370`) — a
detection-surface limit, not redaction.

### The entropy gate

`lib.rs:362`: the gate fires **only** for `SecretType::AwsSecretKey` candidates whose
`secret_entropy(value) < 3.5`, and skips them with a `tracing::debug!`
(`lib.rs:362-365`). It is not a global threshold, and ADR-0005 requires it byte-for-byte
(`plans/adrs/ADR-0005-knowledge-corpus-crate-ownership.md:236-240`) because changing it
changes what the scanner finds. Guard 149 freezes both the constant and the scope.

### Dependency posture

`Cargo.toml:27-31`: `eggsec-core`, `regex`, `serde`, `tracing`. `eggsec-core` is the only
workspace edge.

### How the engine bridges to it

`crates/eggsec/src/recon/mod.rs:101` — `pub use eggsec_secrets as secrets;`. Consumers
include `recon/runner.rs:458, 470` (`SecretScanner::new()`, `scan`), `recon/mod.rs:252`
(`Vec<secrets::SecretFinding>` on the recon result), `recon/git_secrets.rs:441` (a
hand-constructed `SecretType`), and three `recon/*_auth.rs` modules that import
`recon::secrets::Severity` as a re-export path (`ssh_auth.rs:10`, `smtp_auth.rs:7`,
`ftp_auth.rs:7`).

The facade's load-bearing consumer is the Python bindings: `crates/eggsec-python/src/git_secrets.rs:132-163`
holds an exhaustive **30-arm** `match` over `eggsec::recon::secrets::SecretType` (30 arms
counted), mirroring its own 30-variant `SecretType` at `git_secrets.rs:52`. A
re-export, never a re-type — that is what kept the diff empty at extraction time.
Subprocess orchestration (`recon/git_secrets.rs`) stays engine-side.

---

## `eggsec-payloads`

### Purpose

Payload **data**: it builds strings and returns them. It opens no socket, spawns no
process, and performs no probing (`src/lib.rs:8-11`).

### Key types and APIs

| Item | Location | Detail |
|---|---|---|
| `pub mod` declarations | `lib.rs:47-88` | **41 modules** counted (`macros.rs` is one of them) |
| `PayloadType` | `lib.rs:95-136` | **40 variants**, `lib.rs:96-135`; derives `EnumIter` (`lib.rs:94`) |
| `impl Display for PayloadType` | `lib.rs:138-183` | 40 arms |
| `PayloadType::is_advanced()` | `lib.rs:193-203` | **6** types: `GraphQL`, `OAuth`, `Jwt`, `Idor`, `Ssti`, `Grpc` |
| `PayloadType::all_variants()` | `lib.rs:205-209` | `LazyLock<Vec<_>>` over `strum`'s `iter()` |
| `Payload` | `lib.rs:213-219` | 5 fields: `payload_type`, `payload`, `description`, `severity`, `tags` |
| `pub use eggsec_core::types::Severity` | `lib.rs:221` | Only engine edge |
| `get_payloads(PayloadType) -> Vec<Payload>` | `lib.rs:229-272` | **40 match arms, `lib.rs:231-270`** — exhaustive, no `unreachable!`, no `Vec::new()` |
| `static PAYLOAD_CACHE: LazyLock<FxHashMap<PayloadType, Vec<Payload>>>` | `lib.rs:280-287` | Built by looping `all_variants()` |
| `static ALL_PAYLOADS_CACHE: LazyLock<Vec<Payload>>` | `lib.rs:289-290` | Flattened union |
| `get_payloads_cached(&'static)` | `lib.rs:300-304` | `.expect(...)` rather than a silent empty `Vec` |
| `get_all_payloads_cached(&'static)` | `lib.rs:307-309` | |
| `pub(crate) const DEFAULT_PROBE_PAYLOAD` — *udp-scan, not here* | — | — |

One naming asymmetry: `oast.rs` exports `get_oast_payloads()`, not `get_payloads()`
(`lib.rs:260` dispatches to it). The other 39 modules export `pub fn get_payloads() -> Vec<Payload>`.
Three modules also export extra generators: `compression.rs`
(`generate_deflate_payload`, `generate_gzip_payload`), `graphql.rs`
(`generate_alias_overload`, `generate_depth_limit_bypass`).

LOC distribution across the 42 files: 8,849 total; `lib.rs` 422; `macros.rs` 89; the 40
payload modules 8,338. Largest modules: `websocket.rs` 485, `jwt.rs` 400,
`expression.rs` 409, `headers.rs` 369, `cache.rs` 309, `deser.rs` 305.

### The corpus/prober seam

Six types are marked `is_advanced` (`lib.rs:193-203`). Their **probers** live in the
engine; their **payload strings** live here. Both halves are verifiable:

| Type | Corpus module (payload data) | Engine prober (execution) | Prober's client shape |
|---|---|---|---|
| `GraphQL` | `crates/eggsec-payloads/src/graphql.rs` (185 LOC) | `crates/eggsec/src/fuzzer/payloads/graphql.rs` (543) | takes `&reqwest::Client` as a parameter (`graphql.rs:124`); `GraphQLFuzzer` itself holds no client (`graphql.rs:39-45`) |
| `OAuth` | `oauth.rs` (293) | `fuzzer/payloads/oauth.rs` (643) | owns `pub client: Option<Client>` (`oauth.rs:46`), `with_client` at `:73` |
| `Jwt` | `jwt.rs` (400) | `fuzzer/payloads/jwt.rs` (800) | owns `pub client: Option<Client>` (`jwt.rs:54`), `with_client` at `:79` |
| `Idor` | `idor.rs` (222) | `fuzzer/payloads/idor.rs` (284) | owns `pub client: Option<Client>` (`idor.rs:57`), `with_client` at `:73` |
| `Ssti` | `ssti.rs` (281) | `fuzzer/payloads/ssti.rs` (425) | owns `pub client: Option<Client>` (`ssti.rs:23`), `with_client` at `:203` |
| `Grpc` | `grpc.rs` (198) | `fuzzer/payloads/grpc.rs` (400) | methods take `&reqwest::Client` (`grpc.rs:83, 127, 153`); `GrpcFuzzer` holds no client (`grpc.rs:61-65`) |

So: **four of the six probers own a `reqwest::Client` field; `GraphQLFuzzer` and
`GrpcFuzzer` take one as a method parameter instead.** The common claim — "these six
hold a `reqwest::Client`" — is directionally right and literally right for four. All six
are async and reference `reqwest` (per-file `reqwest` hit counts: graphql 1, grpc 4,
idor 1, jwt 1, oauth 1, ssti 1), which is the property that actually keeps them
engine-side.

The corpus side holds **zero** `reqwest`/`tokio`/`Client`/`.await` hits in non-doc code;
the only `reqwest` mentions in `crates/eggsec-payloads/` are comments explaining why the
probers are elsewhere (`graphql.rs:6`, `grpc.rs:6`, `idor.rs:6`, `jwt.rs:6`, `oauth.rs:6`,
`ssti.rs:9`, `lib.rs:29`).

`is_advanced` selects an **execution strategy**, not a data location. The engine branches
on it at `crates/eggsec/src/fuzzer/engine/core.rs:394` — advanced types go to
`run_advanced_fuzzer`, everything else to the generic payload-batch runner. The strategy
dispatch table lives at `crates/eggsec/src/fuzzer/engine/advanced.rs:32, 39, 48, 74, 91, 104`
(one arm per prober type), and the string→`PayloadType` parser at
`advanced.rs:125-154` has **30 alias arms covering 30 distinct variants** of the 40
(counted). The 10 not reachable by name — `Saml`, `HtmlInject`, `CssInject`, `Ssi`,
`DomClobber`, `Xslt`, `Viewstate`, `DepConfusion`, `XsLeak`, `Latex` — are reachable only
via `"all"`, which uses `all_variants()` at `advanced.rs:117`.

### The cache invariant

Both caches are `LazyLock` (`lib.rs:280, 289`) and the doc comment states why:
eager materialization of all 40 variants at startup "would regress every binary that links
this crate, including the TUI" (`lib.rs:276-279`; ADR-0005 restates it as a MUST at
`ADR-0005:248-250`).

### Dependency posture

`Cargo.toml:25-33`: `eggsec-core`, `base64`, `serde`, `strum` (derive, no default
features), `flate2`, `rustc-hash`, `tracing`. No feature surface of its own.

### How the engine bridges to it

`crates/eggsec/src/fuzzer/payloads/mod.rs:39` — `pub use eggsec_payloads::*;`, with the
six prober modules declared *before* it (`mod.rs:23-28`) so that explicit items beat the
glob: `payloads::graphql` is the prober, `get_payloads` is the corpus. Each prober
re-exports its own corpus builder (`graphql.rs:7`, `grpc.rs:4`, `idor.rs:4`, `jwt.rs:6`,
`oauth.rs:4`, `ssti.rs:5`).

Consumers include `crates/eggsec/src/fuzzer/api_schema/mod.rs:2`,
`fuzzer/engine/core.rs:9` (imports `get_all_payloads_cached`, `get_payloads`, `Payload`,
`PayloadType` from the facade; uses at `:319, 321, 465, 467`), and
`crates/eggsec-tui/src/tabs/fuzz.rs:9` (`use eggsec::fuzzer::PayloadType`; a 13-arm
string→variant selector at `:218-235`).

An engine-side integration test guards the seam itself:
`crates/eggsec/tests/fuzzer_payload_corpus_seam.rs` — three tests, the all-40 count
pinned at `:78`, `is_advanced` required to equal the prober set at `:122-140`. Its
docstring states why it must live engine-side: nothing inside the corpus crate can catch
a dropped facade (`seam.rs:13-16`).

---

## `eggsec-udp-scan`

### Purpose

UDP range scanning with honest classification. The load-bearing invariant is in the
crate's own module doc: **closed is provable, open is not** (`lib.rs:5-15`). UDP has no
handshake, so silence is consistent with four different situations and is reported as
`OpenFiltered` (`lib.rs:10`).

Unlike the other three, this crate *does* open sockets and do I/O. What it never does is
authorize, resolve DNS (the caller passes a pre-resolved `Ipv4Addr`, `lib.rs:29-30` and
`lib.rs:89-91`), or acquire privilege.

### Classification states

`UdpPortState` (`classify.rs:14-26`) has **4 variants**:

| Variant | Line | Proven by |
|---|---|---|
| `Closed` | `classify.rs:16` | ICMP port-unreachable (type 3 / code 3) |
| `Filtered` | `classify.rs:19` | Admin-prohibited, net/host unreachable, or TTL expiry |
| `OpenFiltered` | `classify.rs:22` | Silence — the honest maximum for an unprobed port |
| `Open` | `classify.rs:25` | Only a protocol-specific probe can earn this; the range scan never does |

`Evidence` (`classify.rs:33-47`) has 6 variants, and `Evidence::is_proof()`
(`classify.rs:55-63`) returns true for only 4 of them — `Silence` and
`NetOrHostUnreachable` are explicitly *unproven* so a permissive verdict cannot be
mistaken for a measured one.

`HostState` (`classify.rs:112-123`) has 3 variants: `Up`, `Unresponsive`,
`Indeterminate`. `HostState::derive(correlated, orphans)` (`classify.rs:139-147`) uses
only *correlated* errors to claim liveness, because an orphan may be another process's
traffic on the shared ICMP socket. `ports_are_meaningful()` (`classify.rs:129-131`) is
true only for `Up`.

`classify()` (`classify.rs:79-93`) returns `(Filtered, NetOrHostUnreachable)` for
net/host-unreachable rather than `OpenFiltered` — a gateway saying "host unreachable" is
a statement about the host, not the port (`classify.rs:76-78`).

### ICMP parsing

`parse_icmp_error(bytes) -> Result<IcmpError, ParseFailure>` (`icmp.rs:94-110`).
`split_icmp` (`icmp.rs:119-128`) strips a leading IPv4 header only when byte 0 says
version 4, IHL ≥ 20, and the protocol byte says ICMP — a conservative test, because a
bare ICMP message cannot satisfy all three. This exists because delivery shape differs by
platform: a raw IPv4 ICMP socket omits the IP header, a macOS/BSD datagram ICMP socket
includes it (`icmp.rs:7-15`). Both directions are asserted in
`crates/eggsec-udp-scan/src/tests.rs:82` (`parses_both_delivery_shapes_identically`).

`IcmpError` (`icmp.rs:50-61`) carries 5 fields and yields a correlation key of
`(probe_src, probe_src_port)` (`icmp.rs:65-67`) — the source endpoint of the *original*
datagram, which is how an out-of-band error is tied back. `ParseFailure` (`icmp.rs:72-87`)
has 5 variants. `icmp_type` (`icmp.rs:20-27`) and `unreachable_code` (`icmp.rs:30-40`)
expose 3 type constants and 4 code constants.

### Correlation

`CorrelationTable` (`correlate.rs:65-77`) is the only non-trivial part of the scanner, and
it is pure bookkeeping over an explicit `now` — there is no clock call in the module
(`correlate.rs:5-7`).

- `register(...)` (`correlate.rs:98-122`) keys on `(probe_src, probe_src_port)` and
  refuses a live-key collision with `ProbeCollision` (`correlate.rs:46-49`) rather than
  silently overwriting.
- `correlate(key)` (`correlate.rs:129-140`) **removes** the entry, which makes duplicate
  delivery idempotent: the second copy finds nothing and is counted as an orphan.
- `sweep(now)` (`correlate.rs:144-156`) drops expired entries and returns their ports.
- Four counters: `expired`, `orphans`, `collisions`, `matched` (`correlate.rs:67-76`),
  read via `expired_count`/`orphan_count`/`collision_count`/`matched_count`
  (`correlate.rs:164-181`).

Memory is O(in-flight), never O(ports): a 65,535-port sweep holds at most `concurrency`
entries (`correlate.rs:9-13`), asserted at `tests.rs:417`.

The stale-key hazard is handled by never re-inserting an orphan — a late error must not
misattribute to a source port the kernel later reuses (`correlate.rs:33-36`), tested at
`tests.rs:331`.

### Sockets and privilege

`IcmpReceiver::open()` (`socket.rs:75-104`) is Linux-only: a datagram ICMP socket there is
confined to echo replies by `net.ipv4.ping_group_range`, so unsolicited errors need
`SOCK_RAW` and therefore `CAP_NET_RAW` (`socket.rs:83-87`, `socket.rs:24-27`). It
returns `PrivilegesRequired` on `EACCES` (`socket.rs:90-91`).

`IcmpReceiver::open_unprivileged()` (`socket.rs:107-153`) is the mirror image and is
Unix-not-Linux only (`socket.rs:108`): `SOCK_DGRAM`/`IPPROTO_ICMPV4` succeeds
unprivileged on macOS/BSD. It clears `IPV6_V6ONLY` to get native-IPv4 form rather than
IPv6 encapsulation (`socket.rs:125-139`).

The practical consequence is stated plainly at `socket.rs:29-32`: the unprivileged tier is
available on macOS/BSD and **not** on Linux — the inverse of the usual assumption.
Callers branch on `is_unprivileged()` (`socket.rs:156-158`) rather than guessing.
`socket2` is deliberately unused: no `IP_RECVERR` accessor, and `Socket::as_raw()` is
`pub(crate)`, so even a `libc::setsockopt` shim is impossible (`socket.rs:3-15`).

`recv_timeout` (`socket.rs:180-221`) treats `Interrupted`, `WouldBlock`, `TimedOut`, and
`ConnectionRefused` as `Ok(None)` — EINTR is normal under signals, and a
connection-style refusal on a shared ICMP socket belongs to another process
(`socket.rs:198-201`, `socket.rs:213-216`).

`ProbeSocket` (`socket.rs:229-232`) is kept **per probe** on purpose: connecting pins the
local endpoint, which is the correlation key the kernel quotes back
(`socket.rs:226-227`). `bind` (`socket.rs:246-293`) binds an ephemeral port;
`connect_to` (`socket.rs:311-354`) reconnects and re-reads `getsockname`, because a
wildcard-bound socket reports `0.0.0.0` until the route picks an address
(`socket.rs:304-310`). The driver does exactly this at `lib.rs:263-278`, with the
connect-before-register ordering commented at `lib.rs:270-273`.

Both socket types close their fd exactly once in `Drop` (`socket.rs:59-66`,
`socket.rs:234-239`).

### Public surface

| Item | Location |
|---|---|
| `pub mod classify / correlate / icmp / socket` | `lib.rs:40-43` |
| Flat re-exports of 11 types | `lib.rs:48-51` |
| `DEFAULT_PROBE_TIMEOUT = 800ms` | `lib.rs:54` |
| `DEFAULT_SWEEPS = 1` | `lib.rs:56` |
| `DEFAULT_CONCURRENCY = 64` | `lib.rs:58` |
| `MAX_CONCURRENCY = 512` | `lib.rs:65` — deliberately below the workspace's 1000, which is an EMFILE cliff at a 1024-descriptor soft limit (`lib.rs:60-64`) |
| `MAX_PORT = 65535` | `lib.rs:67` |
| `UdpScanError` | `lib.rs:71-84` — 6 variants |
| `UdpScanRequest` | `lib.rs:88-103` — 6 fields; `validate()` at `:123`, `port_count()` at `:146` |
| `IcmpEvidence` | `lib.rs:157-170` — 6 counters |
| `UdpScanResults` | `lib.rs:174-185` — 7 fields; accessors `closed_ports`/`filtered_ports`/`open_filtered_ports` at `:189, 198, 207` |
| `scan_with_receiver(&request, &receiver)` | `lib.rs:230-341` — **blocking**, deadline-bounded |
| `pub(crate) const DEFAULT_PROBE_PAYLOAD: &[u8] = b""` | `lib.rs:350` — deliberately empty |

`scan_with_receiver` takes the receiver rather than opening one, so the caller chooses the
privilege tier explicitly and the difference stays visible (`lib.rs:218-221`). Every port
in the range gets a verdict even if its probe never ran (`lib.rs:310-323`).

### How the engine bridges to it

Not a re-export: a **lossy projection** in `crates/eggsec/src/dispatch/scanner.rs:157-252`
(feature `udp-scan`). No `eggsec_udp_scan` type crosses the boundary — each is mapped into
an engine-owned `PortStatus` (`dispatch/scanner.rs:217-222`), `UdpHostState`
(`:239-243`), and `UdpEvidenceSummary` (`:244-250`). The engine-side types are declared at
`crates/eggsec/src/scanner/ports/mod.rs:236` (`UdpHostState`, 3 variants) and `:263`
(`UdpEvidenceSummary`, 5 fields), both under `#[cfg(feature = "udp-scan")]`. The stated
reason (`ports/mod.rs:226-231`): `PortScanResults` crosses the daemon protocol, the Python
bindings, and the report model, and embedding a scanner-crate type would couple the wire
contract to that crate's internals *and* drag `serde` into a crate that deliberately has
none.

Two details worth knowing before trusting a UDP result:

- `open_ports` holds **every** port with a verdict, not just open ones
  (`dispatch/scanner.rs:209-211`), so `open_ports.len()` is not an open count. Use
  `PortScanResults::proved_open_ports()` / `udp_host_state` (see
  [scanner.md](scanner.md)).
- The receiver is opened as `open_unprivileged().or_else(open())`, and a failure is
  surfaced as an explanation rather than an empty result (`dispatch/scanner.rs:184-193`).
- The blocking call runs under `spawn_blocking` with a 120s outer bound
  (`dispatch/scanner.rs:198-207`), because a cancelled join would not stop the thread.

Engine wiring: `eggsec-udp-scan` is an optional dependency behind the engine's `udp-scan`
feature (`crates/eggsec/Cargo.toml:122`, feature at `:376`); consumers are
`dispatch/scanner.rs`, `scanner/ports/mod.rs`, `scanner/ports/spoofed.rs:634-636`, and
`config/feature_registry.rs:219`.

---

## Why these left the engine

The decision record is `plans/adrs/ADR-0005-knowledge-corpus-crate-ownership.md`
(2026-10-05) for the three corpus crates and `architecture/capability_segregation.md:217-289`
(Phase F) for `eggsec-udp-scan`.

### The corpus argument

ADR-0005 rejects the obvious "split big modules per feature" instinct with measurement
(`ADR-0005:30-39`): almost nothing crosses a 13-distinct-`crate::`-path threshold except
`pipeline` (22 paths) and `tool` (658 references). The large engine modules are *executor
bodies* — coupled because their job is to be coupled.

What the analysis did surface was a different category (`ADR-0005:41-43`): **data-shaped
modules whose content is a body of domain knowledge, not logic**. The measured candidate
set (`ADR-0005:45-49`):

| Module | Lines | `crate::` coupling | Tests |
|---|---|---|---|
| `fuzzer/payloads/` (pure-data subset, at measurement) | 7,084 / 36 files | ~0 | 233 |
| `recon/secrets.rs` | 492 | 1 (`Severity`) | 11 |
| `scanner/service_data.rs` | 302 | 0 | 21 |

The analysis also surfaced a defect rather than a candidate: `utils/redaction.rs` (366
lines, 26 tests) had **zero production consumers** (`ADR-0005:60-66`). Milestone 001
deleted it under guard 147, which is why the docstring in `eggsec-secrets` says masking is
the report model's job, not this crate's (`src/lib.rs:13-16`).

Each candidate was validated by extraction spike before the ADR was written — copied to a
scratch crate depending only on `eggsec-core` plus its own third-party deps, compiled with
its full suite passing unmodified (`ADR-0005:51-58`).

Three decisions shape everything above:

1. **Ownership** (`ADR-0005:154-158`): corpora are owned by leaf crates whose only
   permitted workspace dependency is `eggsec-core`. "A corpus that needs an engine type is
   not a corpus."
2. **Re-export, never re-implementation** (`ADR-0005:160-164`):
   `eggsec::fuzzer::PayloadType` and `eggsec::recon::secrets::SecretFinding` must stay
   valid paths so the Python bindings' exhaustive matches and the TUI compile without
   edits.
3. **Publication is a separate milestone** (`ADR-0005:166-169`). All four crates are
   `publish = false`. `plans/adrs/ADR-0006-knowledge-corpus-publication.md` (accepted
   2026-10-05) deferred it: release qualification is proven expensive (the NSE
   milestone is blocked on a 74-function major break found by a semver gate), no external
   consumer is identified, and `eggsec-core` would need publishing on a shared version
   line.

The `eggsec-udp-scan` argument is different in kind and is worth reading separately
(`architecture/capability_segregation.md:222-243`): three reasons, in weight order — the
primitives are *incompatible*, not merely different (TCP proves `closed` from a failed
`connect`; UDP's answer lives in a rate-limited out-of-band signal that `Option<PortResult>`
cannot express); the authorization facts differ (`Capability::ActiveProbe` vs
`Capability::RawPacketProbe` plus a platform privilege gate); and the result type differs
in kind (a boolean list vs a four-state lattice with per-state evidence and a host-level
verdict that can invalidate the per-port claims).

### Status

`plans/registry.md:35` records the `security-knowledge-corpus` subsystem as **closed**
with all five milestones closed 2026-10-05: 001 redaction deleted (guard 147), 002
`eggsec-service-db`, 003 `eggsec-secrets`, 004 `eggsec-payloads`, 005 publication
deferred.

One correction is load-bearing enough to repeat here. Milestone 004 originally held that
6 of the 40 payload modules "generate payloads by performing live `reqwest` probing" and
so could not move; it routed them to a documented `unreachable!()` and `get_payloads`
panicked for 6 of 40 variants. That premise was false — each of those files bundles a
*prober* (needs a client) with a `get_payloads()` that is pure string construction
(`architecture/capability_segregation.md:363-374`, `plans/registry.md:163`). The payload
halves moved; only the probers stayed. The seam is **execution, not payload data**.

## Guards

`scripts/check-architecture-guards.sh`; run via `bash scripts/check-architecture-guards.sh`,
which `make check` invokes (`Makefile:127`).

| Check | Line | Asserts |
|---|---|---|
| **114** | `:3108-3137` | `eggsec-service-db/src/lib.rs` exists; `crates/eggsec/src/scanner/service_data.rs` and `utils/service_detection.rs` have not reappeared; `scanner/mod.rs` still contains `pub use eggsec_service_db as service_data` (`:3129`). Fails if the facade is dropped, not just if the file returns. |
| **147** | `:4124-4146` | Removed `utils/redaction.rs` stays removed: no file, no `pub mod redaction`, no engine-local `redact_sensitive`/`redact_json`. |
| **148** | `:4171-4225` | **Leaf policing** for all three corpus crates. Per crate: rejects 10 forbidden workspace deps (`:4185`, incl. `eggsec-udp-scan`), rejects 13 forbidden I/O/frontend deps (`:4193`), and rejects the authorization vocabulary `Scope\|ApprovedOperation\|ApprovedExecution\|EnforcementContext\|Capability` **in code** (`:4208`). Separately, `eggsec-service-db` must *not* declare `eggsec-core` (`:4216-4220`). |
| **149** | `:4227-4269` | Secret-detection owner + facade + frozen entropy gate: `crates/eggsec-secrets/src/lib.rs` exists; `recon/secrets.rs` has not reappeared; `recon/mod.rs` still has `pub use eggsec_secrets as secrets` (`:4239`); **every** `secret_entropy(value) < N` site must match the literal `3.5` (`:4254`); the gate must still be scoped to `SecretType::AwsSecretKey` (`:4260`). |
| **150** | `:4271-4365` | **Corpus/prober seam.** For each of `graphql grpc idor jwt oauth ssti`: the corpus module exists and defines `pub fn get_payloads` (`:4285-4291`); the engine prober module exists (`:4294`); the engine re-exports the corpus builder (`:4300`). Crate-wide: no module reaches for `reqwest\|tokio\|Client\|.await\|block_on\|TcpStream` (`:4308`); each of the 6 dispatches to its corpus module (`:4322`); no variant panics or stubs (`:4327`); `PAYLOAD_CACHE` is a `LazyLock` (`:4334`); no `static EMPTY` fallback (`:4339`); the all-40 test exists (`:4344`); the engine keeps `pub use eggsec_payloads::*;` (`:4352`) and no longer defines its own caches (`:4356`). |

Two supporting facts about these guards: check 148's vocabulary match strips string
literals *before* searching (`code_search` at `:4157-4169`), because the OAuth corpus
legitimately ships the description `"Scope escalation to admin"` as attack data; and
guard 150 was itself rewritten — its earlier version asserted the *opposite* premise (the
corpus must NOT have those 6 modules and must panic for them), which `guard comment
:4276-4280` records as encoding the false premise.

`eggsec-udp-scan` is named in guard 148 only as a **forbidden dependency of the corpora**
(`:4185`) — it is not itself subject to a dedicated guard. *Unverified:* whether the
corpus/UDP honesty invariants (`is_advanced` ⇒ prober exists, `HostState` projection
completeness) are policed anywhere else; I found only the engine integration test
`crates/eggsec/tests/fuzzer_payload_corpus_seam.rs` and no guard covering
`dispatch/scanner.rs`'s projection.

Test wiring: `Makefile:122-125` runs `cargo test -p eggsec-service-db/eggsec-secrets/eggsec-payloads/eggsec-udp-scan --tests`, so all four corpus crates are covered by `make check`.

> **Corrected 2026-10-06 (fixed, not just noted).** An earlier revision of this file
> recorded two gaps here as open findings. Both are now closed:
> 1. There was **no** `cargo test -p eggsec-udp-scan` line in `make check`, so its 23 tests
>    (21 in `tests.rs`, 2 in the loopback module `lib.rs:408`) never ran. Added —
>    `Makefile:125`; verified 23 passed / 0 failed.
> 2. Its `test-util` feature was declared with **no** `cfg(feature = "test-util")` site
>    anywhere in the crate and no consumer enabling it (grep-verified: the only reference in
>    the workspace was its own declaration). It gated nothing, so it was **removed** rather
>    than wired. This is distinct from `eggsec-transport`'s `test-util`, which is real —
>    it gates `fake.rs` behind `cfg(any(test, feature = "test-util"))` and is enabled by
>    `eggsec` and `eggsec-agent`.

Verified corpus test counts (2026-10-06): payloads **252** (across 42 files; only 4 in
`lib.rs`), service-db **21**, secrets **11**, udp-scan **23**. The registry's former
"233/21/11" figure predates the 2026-10-06 move of the last 6 payload sets into the corpus.

---

## Corrections to pre-existing prose

Claims I checked and found **wrong or stale**, recorded so the next reader does not
inherit them:

1. **"34 pure-data modules in `eggsec-payloads`, the 6 live-probe modules stay
   engine-side."** Stale. `crates/eggsec/Cargo.toml:110-112` carried this comment.
   **Fixed 2026-10-06** — it now reads that the corpus owns all 40 `PayloadType` variants
   and the cross-variant caches, with only the 6 live probers engine-side. The corpus has
   **40** payload modules (`lib.rs:47-88`, 41 `pub mod` including `macros`) and owns all 40
   variants; only the probers stayed engine-side. Corrected per `plans/registry.md:167`.
2. **"`eggsec-service-db`: … Only dep is rustc-hash."** Correct — and stronger than it
   looks. Guard 148 asserts it must not even declare `eggsec-core` (`:4216-4220`), and
   the crate is 316 lines, not the 302 the pre-extraction measurement recorded
   (`ADR-0005:49`) — the crate docstring (`lib.rs:1-19`) accounts for the difference.
3. **"secrets: 25 secret patterns covering 20 of 30 `SecretType` variants."** Correct,
   verified: 25 patterns (`lib.rs:136-328`), 30 variants (`lib.rs:45-74`), 20 covered.
   Worth stating the 10 uncovered ones explicitly (above), since "20 of 30" reads like
   an intentional gap only if you know which 10.
4. **"payloads: … The 6 modules holding a `reqwest::Client` (GraphQL, OAuth, Jwt, Idor,
   Ssti, Grpc) are probers not generators."** Mostly right, imprecise on ownership: only
   **4** of the 6 probers *hold* a client (`idor.rs:57`, `jwt.rs:54`, `oauth.rs:46`,
   `ssti.rs:23`); `GraphQLFuzzer` and `GrpcFuzzer` take `&reqwest::Client` as a method
   parameter (`graphql.rs:124`, `grpc.rs:83, 127, 153`) and hold none. The corrected
   claim — all six are async network modules; only four own the field — is what this doc
   records.
5. **"udp-scan: honest open/closed/filtered classification, zero workspace deps."**
   Correct, with a refinement: the state lattice is **4** states, not 3
   (`classify.rs:14-26`), and `Open` is a defined variant the range scan never emits
   rather than an absent one. "Zero workspace deps" is exact — `libc`, `thiserror`, and
   `tracing` are all external (`Cargo.toml:19-26`).

## See Also

- [overview.md](overview.md) — workspace crate table and dependency map.
- [capability_segregation.md](capability_segregation.md) — Phase F (`eggsec-udp-scan`)
  and Phase G (the corpus crates) decision records.
- [scanner.md](scanner.md) — the engine's consumer of `eggsec-service-db` and the
  `PortStatus`/`PortScanResults` projection target for `eggsec-udp-scan`.
- [fuzzer.md](fuzzer.md) — the engine's consumer of `eggsec-payloads`, including the
  six probers and the `is_advanced` strategy branch.
- [recon.md](recon.md) — the engine's consumer of `eggsec-secrets`.
- [dispatch.md](dispatch.md) — where `scan_ports_udp` (`dispatch/scanner.rs:157`) is
  dispatched from and how the projection is bounded.
- [types.md](types.md) — `Severity` and the shared DTO vocabulary these crates lean on.
- [generated.md](generated.md) — conventions for documenting generated/derived content.
- [domain_contract.md](domain_contract.md) — the data-contract discipline these leaf
  crates exist to satisfy.
- [supply_chain.md](supply_chain.md) — dependency policy (`deny.toml`) that the corpus
  crates' narrow graphs are checked against.

Plan and decision sources (outside `architecture/`):
`plans/adrs/ADR-0005-knowledge-corpus-crate-ownership.md`,
`plans/adrs/ADR-0006-knowledge-corpus-publication.md`,
`plans/subsystems/security-knowledge-corpus-roadmap.md`,
`plans/registry.md:35`.