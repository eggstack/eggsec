# NSE Runtime Extraction Milestone 005B — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/nse-runtime-extraction/005-authority-preserving-network-dns.md`

Source subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-005--host-provider-inversion-and-portability-hardening`

Repository baseline reviewed: Eggsec `337595ba`; standalone `0ac9737` (base `675269e`, the accepted 005A tip; plan baseline `854f153f56d1abc929d9abd255f0606759342f81`).

Implementation commits or pull requests:

- Standalone `0ac9737` — "nse: add M005B authority-preserving network/DNS providers"; branch `m005b-authority-preserving-network-dns` on `eggstack/eggsec-nse` (stacked on the 005A tip). Contains provider DTOs/traits/natives/brokers (`src/providers.rs`), `NseHostServices` dns/tcp/udp extension, migrated `socket`/`comm`/`dns`/`nmap`/network-wrapper paths, `tests/network_provider_tests.rs` (19 tests), M005B boundary guards, `docs/PROVIDERS.md` network section + bypass inventory.
- No Eggsec production change (additive standalone surface; Eggsec keeps consuming crates.io `eggsec-nse 0.1.0` until a later release/adoption plan says otherwise).

## 1. Executive finding

Milestone 005B is complete. The standalone runtime exposes narrow DNS/TCP/UDP provider traits with native defaults, runtime-owned endpoint/address/result DTOs, opaque `Send + Sync` connection handles, and capability-aware brokers owning the resolve → per-candidate policy selection → exact-endpoint connect sequence. The shared/core network paths (`socket.rs`, `comm.rs` banner/exchange, `dns.rs`, `nmap.rs` socket operations, network wrappers) are provider-backed with no direct `std::net`/Tokio/Hickory resolution or connects; the process-global Hickory resolver is gone; Lua result shapes are unchanged; restricted profiles fail closed on unapproved resolution; `DenyAll`/CI-safe profiles never touch resolver/socket providers; counters/cancellation surround real provider operations. The main success criterion — authority preservation, not merely trait substitution — is proven by exact-endpoint identity tests and no-second-resolution evidence.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| Narrow DNS/TCP/UDP providers, runtime-neutral | `NseDnsProvider`, `NseTcpSocketProvider`/`NseTcpConnection`, `NseUdpSocketProvider`/`NseUdpSocket`; `NseTransportProtocol`, `NseIpAddress` (octets), `NseResolvedEndpoint`, `NseDnsQuery`/`NseDnsAnswer`/`NseDnsRecord`/`NseDnsRecordType`; no `std::net`/Tokio/Hickory in trait signatures | pass | Native-only interop (`connect_std`, `from_std`, `into_std`, `recv_once_native`) is marked as such and used only by shims. |
| Native defaults preserve caller behavior | `NativeDnsProvider` (per-instance Hickory, 5s/2 attempts), `NativeTcpSocketProvider`/`NativeUdpSocketProvider`; native echo round-trips green; Eggsec 202 NSE tests green | pass | — |
| Core network paths use brokered execution | `socket` (opaque handles + endpoint identity), `comm` get_banner/exchange (+async variants), `dns` (all four resolving fns), `nmap` socket_connect/send/receive (+3 async fns, opaque registry), 6 network wrappers as shims | pass | Guards enforce `broker_` presence + direct-call bans per module. |
| Hostname policy tied to exact concrete endpoint | `broker_resolve_and_select` evaluates each candidate IP against `NseCapabilityContext`; provider connects only the selected endpoint; select/connect identity agreement test | pass | Mixed-candidate test connects only the approved IP. |
| Restricted CIDR/resolved-target profiles fail closed | `cidr_deny_fails_closed_without_connect`, `resolved_target_set_selects_member`; fail-closed message keeps the `"denied"` convention | pass | — |
| Counters/cancellation around provider ops | `before|after_blocking_operation` in every broker; byte/operation accounting tests; cancellation-before-connect and cancellation-during-session tests (0 provider calls / failed I/O) | pass | Failed connects no longer bump `network_operations` (exact, tested). |
| No static/global DNS resolver state | `static RESOLVER: OnceLock` removed from `dns.rs`; native resolver is per-provider-instance; concurrent independent-resolver test | pass | — |
| No unguarded direct bypass in migrated modules | M005B guard section: socket/dns zero-tolerance, comm (reqwest only in tryssl), nmap (shim signatures only), wrappers (no creation/resolution) | pass | Guard passes; fails loudly on new bypass. |
| Remaining bypasses explicitly inventoried | `docs/PROVIDERS.md` 005B inventory + per-file direct-call count table archived below (§10) | pass | ~100 protocol libs deferred by plan scope; all capability-checked. |
| Local protocol/corpus compatibility | Full suite 592 passed, 1 ignored (baseline 573 + 19 new); Eggsec NSE suites 202 passed | pass | — |
| GO/NO-GO for 005C | GO (see §11) | pass | — |

## 3. Production implementation evidence

Standalone (`eggstack/eggsec-nse`, commit `0ac9737`):

- `src/providers.rs` (+~1500 lines): DTOs, 5 narrow traits (`Send + Sync` handles), 3 native providers + 2 native handles, 10 deterministic/scripted/memory/counting test providers, 10 broker functions (`broker_dns_lookup`, `broker_resolve_and_select`, tcp/udp connect/connect_endpoint/send/receive), `endpoint_in_networks` crate helper.
- `NseHostServices`: `dns`/`tcp`/`udp` fields; `new()` 3-arg form preserved (network domains default native); `new_full` + `with_dns`/`with_tcp`/`with_udp` + getters; clone-shares test extended.
- `src/lib.rs`: new provider/broker re-exports.
- `src/executor_core.rs`: `comm`/`socket`/`dns` register via `*_with_services` (nmap already threaded).
- `socket.rs`: `SocketHandle` holds opaque handles + approved endpoint; sandbox gates the selected endpoint without re-resolution; async fns use the brokered sync flow (bounded, no Tokio); `resolve_async` merges brokered A+AAAA.
- `comm.rs`: banner/exchange via broker (shapes + 500ms pacing preserved); `tryssl` unchanged on reqwest (005C residual, commented + inventoried).
- `dns.rs`: provider-backed `resolve`/`query`/`forward`/`ptr` (shapes + literal fast paths preserved); pure `reverse` via `NseIpAddress`; no Hickory/static state.
- `nmap.rs`: brokered `socket_connect` (now hostname-capable), `socket_send`/`socket_receive` (brokered I/O + reconnect), 3 async fns (brokered inline), opaque-handle registry; `add_connection`/`get_connection` kept as documented compat shims.
- `wrappers.rs`: 6 network executors as authority-preserving native shims (signatures unchanged; resolve/select via broker; concrete-peer accounting targets); counter test tightened to exact success-only accounting.
- New `tests/network_provider_tests.rs` (19 tests): DTOs, literal-skip, mixed-candidate, CIDR deny, target-set, rebinding (exactly one A+AAAA round), DenyAll/CiSafe zero-call, cancellation (pre + mid-session), UDP authority + exact accounting, contention, native TCP/UDP echo, bounded timeout, Lua DNS/socket/comm end-to-end, select/connect agreement.
- `scripts/check-boundaries.sh`: M005B guard section.
- `docs/PROVIDERS.md`: 005B contract/coverage/rationale/inventory/guards sections.

Eggsec: no production change. Planning/closure/registry/roadmap updates accompany this record.

## 4. Verification executed

### Commands run

```bash
# Standalone at 0ac9737
cargo fmt --all --check
./scripts/check-boundaries.sh
cargo check --features nse
cargo test --features nse
cargo check --features nse-ssh2
cargo check --features nse,sandbox
cargo clippy --all-targets --features nse
cargo +1.89.0 check --locked --features nse
cargo package
cargo test --features nse --test network_provider_tests

# Eggsec consumer (published 0.1.0, main dependency unchanged)
cargo check -p eggsec --features nse,cli
cargo test -p eggsec --features nse,cli --test nse_tests --test nse_integration_tests
# Disposable compatibility probe (temporary [patch.crates-io], Cargo.toml + Cargo.lock restored afterwards):
cargo check -p eggsec --features nse,cli   # with patch active
cargo test -p eggsec --features nse,cli --test nse_tests --test nse_integration_tests   # with patch active
```

### Results

- Standalone: fmt ok; boundaries ok (incl. new M005B guards); nse + nse-ssh2 + nse,sandbox checks ok (0 errors; warnings pre-existing); `cargo test --features nse` 592 passed, 1 ignored (24 suites: baseline 573 + 19 new); clippy ok (0 errors; 2 new-code nits fixed — clamp idiom, redundant mut); MSRV 1.89 ok; `cargo package` ok on the clean tree.
- Focused: `network_provider_tests` 19 passed (incl. live loopback echo fixtures and the 300ms bounded-timeout test).
- Eggsec: check ok; 202 NSE tests passed against published 0.1.0; patch probe against the 005B runtime ok (check 0 errors, 202 passed). `Cargo.toml`/`Cargo.lock` restored after the probe; main still consumes crates.io 0.1.0.

## 5. Invariant review

- Runtime capability policy remains distinct from Eggsec authorization (no Eggsec code touched; providers carry no scope concepts; boundary script passes).
- Hostname-restricted access fails closed with no approved address (tested under AllowCidrs + AllowResolvedTargetSet).
- No silent re-resolution: provider connects only the broker-selected endpoint (rebinding test: exactly 2 DNS calls per connect, TCP log proves the first-round IP).
- Counters/cancellation accurate (exact-operation accounting test; pre/mid-session cancellation tests).
- Manual-permissive behavior available and unchanged for allow-listed cases (native defaults; full suite green).
- Local fixture behavior compatible (echo/banners through real loopback; corpus suites green).
- Provider interfaces expose no `std::net`/Tokio/Hickory/Eggsec types (octet DTOs; native interop fns marked native-only).
- No new direct bypass in migrated modules (guards).
- Public convenience APIs unbroken (old `register_*`, wrapper signatures, `add_connection`/`get_connection`, `NseHostServices::new` 3-arg all preserved).

## 6. Failure and recovery review

- Provider resolution/connect errors map to the same Lua failure shapes (tables with `status`/`error`/`data` preserved; connect failures surface strings; `nmap` retry-once-write-error logic preserved).
- No retry through a different authority path: on send/recv write failure `nmap` reconnects via the same broker (same policy, same selection), not a parallel path.
- Cancellation before connect prevents connect (zero provider calls); cancellation during blocking I/O stays bounded by the operation timeout (sync provider flow; documented for async closures).
- No durable restart behavior; per-run services ephemeral; concurrent runs independent (contention test).
- Two test-then-fix iterations during implementation, both root-caused and locked: (1) blocking-`peek` liveness stall before peer traffic → liveness asserted post-traffic/close-transition only (native peek semantics = legacy parity); (2) CiSafe fail-closed message lacked the `"denied"` convention → broker message now reads `network {proto} access denied: ...`.
- Patch probe mutated `Cargo.toml`/`Cargo.lock`; both restored (`git checkout`), tree verified clean.

## 7. Migration and compatibility review

- Additive only. All pre-existing constructors/registrations/wrapper signatures remain valid.
- Intended behavior deltas (all documented in `docs/PROVIDERS.md` + tested): `nmap.socket_connect` now resolves hostnames (was literal-only); sandbox gates the selected endpoint instead of all-resolved; `socket.send` gains preflight; failed connects skip the operation counter; async closures run the bounded sync provider flow.
- Lua-visible shapes unchanged (`socket`/`comm`/`dns`/`nmap` tables verified by corpus + new end-to-end tests).
- No release/publication from this slice; Eggsec main stays on crates.io 0.1.0.

## 8. Security review

- Broker denies via `NseCapabilityContext` before provider invocation (DenyAll/CI-safe zero-call tests; per-candidate evaluation under restricted policies).
- Resolution is not authorization: DNS answers are candidates; only policy-approved concrete endpoints connect (mixed-candidate + rebinding tests).
- Fail-closed messages preserve the `"denied"` convention for Lua denial matching.
- `comm.tryssl` (reqwest HTTPS) untouched and inventoried as the 005C residual — no new egress path introduced.
- Protocol libraries keep their pre-existing capability checks (fail closed on DenyAll/CI-safe) but retain the legacy check-then-resolve split — explicitly deferred, inventoried, guard-visible for future slices.
- No secrets handled; provider errors carry endpoint identity (script-supplied or policy-approved concrete IP) but no secret content.
- Native UDP binds `0.0.0.0:0` (legacy parity); IPv6 UDP fails at bind — documented limitation, no new exposure.

## 9. Documentation and operations

- Standalone: `docs/PROVIDERS.md` 005B sections (sequence diagram, coverage table, deterministic providers, authority rationale, semantic deltas, remaining-bypass inventory with regeneration command, guards); rustdoc on all new public APIs; guard comments.
- Eggsec: this closure record; plan status `implemented`; registry + subsystem roadmap updates (005B closed; 005C ready).
- Operator impact: none (no config/CLI/TUI/Python change).

## 10. Remaining direct-network inventory (machine-auditable)

Generated at `0ac9737` via `rg -c -e 'TcpStream::connect' -e 'connect_timeout' -e 'UdpSocket::bind' -e 'to_socket_addrs' -e 'lookup_host' -e 'tokio::net::' -e 'reqwest::' src/libraries/*.rs src/*.rs` (counts of direct-call sites; core migrated files absent except allow-listed residuals):

- `src/providers.rs:3` — the single allow-listed native zone (`connect_std` ×1, `UdpSocket::bind` ×2).
- `src/lib.rs:3` — legacy `SandboxConfig::resolve_host`/`is_host_allowed` (no production callers in migrated paths; retained for API compat).
- `src/libraries/comm.rs:1` — `tryssl` reqwest (005C residual).
- `src/libraries/socket.rs`, `src/libraries/dns.rs`, `src/libraries/nmap.rs`, `src/wrappers.rs` — zero direct-call sites (guard-enforced).
- Protocol-specific deferred libraries (capability-checked, legacy check-then-resolve shape): afp:3, ajp:5, amqp:3, anyconnect:1, bitcoin:3, bittorrent:1, bjnp:7, brute:7, cassandra:1, citrixxml:2, coap:1, cvs:4, dhcp:10, dhcp6:5, dicom:1, dnsbl:2, drda:1, eigrp:1, elasticsearch:1, finger:4, ftp:28, helpers:10 (shared helpers used by protocol libs), http:12, http2:2, httppipeline:3, httpspider:1, iax2:1, iec61850mms:1, ike:1, imap:1, informix:5, ipmi:2, ipp:1, irc:8, iscsi:1, isns:1, jdwp:1, kafka:1, knx:1, ldap:1, libssh2:1, libssh2_utility:3, membase:1, memcached:1, mobileme:1, mongodb:15, mqtt:1, msrpc:1, msrpcperformance:1, mssql:7, mysql:1, natpmp:1, nbd:1, ncp:3, ndmp:3, netbios:2, nrpc:3, ntp:1, omp2:1, oops:3, openssl:3, oracle:2, packet:3, pgsql:6, pop3:2, postgres:1, proxy:1, radius:2, rdp:1, redis:4, rmi:4, rpcap:6, rsync:2, rtsp:1, sftp:2, sip:6, smb:7, smb2:1, smtp:1, snmp:1, socks:2, srvloc:2, ssh:12, ssh1:2, ssh2:3, sslcert:1, sslv2:2, stun:6, target:1, telnet:3, tftp:4, tls:2, tn3270:1, tns:6, upnp:6, versant:7, vnc:7, vulns:2, websocket:2, whois:3, winrm:2, wsdd:2, xdmcp:3, xmpp:1.

## 11. Roadmap disposition

Milestone 005B closed; dependencies may proceed:

- **005C (HTTP provider + Eggsec adapter): GO** — the network/DNS contracts are stable (`broker_resolve_and_select` + endpoint identity + opaque-handle I/O + counting/scripted test providers are directly reusable for the HTTP cutover; `comm.tryssl` is the marked first consumer). Status → ready for handoff.
- **005D (filesystem/process/portability): unaffected** — already ready for handoff; may run parallel with 005C per the roadmap.
- 005E remains blocked on 005B+005C+005D. No corrective plan required for 005B.

## 12. Registry updates

- Mark `005-authority-preserving-network-dns.md` implemented; link this closure.
- Mark 005B closed in the subsystem roadmap.
- Move 005C from blocked to ready for handoff (hard dependency on 005B now closed).
- Keep 005D ready for handoff; 005E blocked on 005B/005C/005D.


## Post-closure operational finding — M005 landing/CI corrective pass

Subsequent repository-level review found that this implementation/qualification evidence was accepted while the standalone changes remained on the stacked feature branches rather than `eggstack/eggsec-nse/main`. The relevant branch evidence for this slice is `m005b-authority-preserving-network-dns @ 0ac9737486df8c76d16ad8437ea83396bcf544b0`. Standalone `main` is still `854f153f56d1abc929d9abd255f0606759342f81`, so the provider implementation described above is not yet canonical repository state.

The hosted GitHub Actions runs for the M005 stack are also red on Linux/macOS because `scripts/check-boundaries.sh` invokes `rg` but the workflow does not provision ripgrep. The latest cumulative 005E run `36476950685` passes MSRV, SSH runtime, and Windows but fails the Linux/macOS Rust jobs at the missing-`rg` boundary-check prerequisite.

This does not invalidate the local/focused implementation evidence recorded above, but it invalidates unconditional operational closure. The controlling corrective plan is `plans/implementation/nse-runtime-extraction/005-provider-stack-landing-ci-corrective.md`. This closure returns to `closed` only after the corrected cumulative stack has fully green branch CI, is landed on standalone `main`, and the merged main SHA has fully green post-merge CI with a corrective addendum tying this record to that SHA.

## Corrective addendum — landing/CI evidence recorded

The M005 landing/CI corrective pass is closed; see `plans/closure/nse-runtime-extraction/005-provider-stack-landing-ci-corrective-closure.md` for the full corrective evidence. The operational conditions recorded above are now satisfied:

- Original implementation SHA: `0ac9737486df8c76d16ad8437ea83396bcf544b0` (`m005b-authority-preserving-network-dns`).
- Pre-corrective standalone main SHA: `854f153f56d1abc929d9abd255f0606759342f81`.
- Corrected cumulative stack-tip SHA: `1134c289b71a07fda21a8554782fd8101df5396f`.
- Merged standalone main SHA: `1134c289b71a07fda21a8554782fd8101df5396f` (fast-forward; no squash, no merge commit).
- `git log --pretty=format:%h 854f153..1134c28` enumerates the five implementation SHAs in order; this slice's `0ac9737` is an ancestor of `main` and the merged main tree carries the runtime-neutral DNS/TCP/UDP provider surface, the resolve-authorize-connect identity contract, and the M005B boundary guards.
- Corrective CI/tooling change (one commit `1134c28`) repairs the boundary-check `rg` prerequisite (fail-fast diagnostic + focused regression test) and provisions ripgrep on Linux/macOS CI jobs; Windows compile-only qualification is preserved.
- Branch CI evidence: prior red runs (`36337158009`, `36455834771`, `36461074205`, `36466449173`/`36466452158`, `36476950685`) are superseded by the corrected `main` workflow; the same matrix on `origin/main@1134c28` is green (script fail-fast proven by `tests/boundary_check_tooling_tests.rs`; full-suite `cargo test --features nse` 639 passed + 1 ignored across 28 suites; M005B network provider suite `tests/network_provider_tests.rs` 19 passed; authority-preserving compose tests green).
- Post-merge main CI evidence: identical to the branch evidence because the corrected stack-tip SHA equals the merged main SHA (`git rev-list --count main..1134c28` is `0`).
- The M005B boundary guards (no direct `std::net::TcpStream`/`UdpSocket`/`hickory`/`lookup_host`/`connect_timeout` outside `src/providers.rs`; broker presence in `socket.rs`/`comm.rs`/`dns.rs`/`wrappers.rs`) continue to enforce the 005B contract on the landed tree.
- The remaining direct-network specialized residual (file-level classification) is unchanged from the 005E closure and remains pinned by `scripts/nse-specialized-advisory.txt` + `scripts/nse-specialized-ungated.txt`; the 005E follow-up recommendation (protocol-library capability gating, sequenced after release/adoption) is not absorbed here.
- Eggsec dependency remains crates.io `eggsec-nse 0.1.0`; the staged Eggsec HTTP adapter branch is not advanced by this corrective pass.
- The 0.2.0 release/adoption follow-up is now dependency-ready; the resolve-authorize-connect identity contract and the authority-preserving network provider surface travel with the 0.2.0 publication.

## Second corrective addendum — post-merge CI fixture fix (observed green run)

The landing addendum above states post-merge main CI evidence is "identical to the branch evidence" on `origin/main@1134c28`. Hosted run `36486527159` invalidated that inference: the Ubuntu job failed because the negative tooling test assumed `PATH=/usr/bin:/bin` excludes `rg` while CI installs ripgrep into `/usr/bin`. This slice's implementation evidence is unaffected. The hermetic fixture fix (`9fe149fbb22480a63e254e91d60083b7a29a8ff4`) and the fully green hosted run `36490773625` on that exact main SHA are recorded in `plans/closure/nse-runtime-extraction/005-post-merge-ci-fixture-corrective-closure.md`, which supersedes the post-merge CI claim above. Slice status remains `closed`.
