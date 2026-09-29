# NSE Runtime Extraction Milestone 007B — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/nse-runtime-extraction/007-broker-compatible-protocol-migration.md` (M007B)
- `plans/implementation/nse-runtime-extraction/007-protocol-migration-corrective.md` (controlling handoff; the original plan was not closable as written)

Source subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-007--protocol-library-gating-and-controlled-automated-activation`

Applicable ADRs:

- `plans/adrs/ADR-0004-nse-automated-activation-boundary.md`
- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`

Planning baseline: Eggsec `8e282e916fa44edc2bb800ba1dc480e8f0ec2815`

Implementation repository: `eggstack/eggsec-nse`

Implementation commits:

- `699d374ad16806a9e0c6f265f297a644b8b88c29` — `feat(nse): M007B broker-compatible protocol migration` (inherited; landed without the required reconciliation)
- `0a6a826dc61081254994d868113e2164a5b8fa95` — `fix(nse): M007B corrective — correct the residual scan, close the manifest gap`
- `42e4861d576d0add51cfd6f5a8188f023c490479` — `fix(nse): M007B corrective — freeze the M005E baseline and enforce it`
- `d4a22f1dbe56f4ccfb17b2a8135aae8395f44f19` — `test(nse): pin the CiSafe zero-network property for target.resolve` (final standalone main)

## 1. Executive finding

M007B is closed after a corrective pass. **The controlling premise of the corrective plan was wrong in its most important claim, and establishing that was the first piece of work.**

The corrective plan asserted that "17 entries currently described as broker-compatible still have direct socket effects and cannot be treated as complete without further audit." The audit showed that 16 of the 17 had **no direct socket effect at all** and the 17th had a *different* defect than described. The 17-file count was an artifact of the M005E scan, which matched `TcpStream::connect` as a substring and therefore also matched `BrokeredTcpStream::connect` — the brokered abstraction M007B had introduced. Sixteen migrated libraries were indistinguishable from unmigrated ones by the tool that was supposed to verify the migration. The classification file, which the plan took as the source of truth for "what was migrated", had never been checked against source and contained at least one false claim.

Acting on the plan's instruction not to "fix CI by copying the current 39-file source scan into the old pin files before auditing the 17 entries" produced a corrected scan that found **four real defects the plan did not know about**, one of them a live security issue in an automated-safe library. Had the corrective pass instead regenerated the pins from the broken scan, all four would have been cemented as the accepted state.

| Outcome | Detail |
|---|---|
| Specialized direct-I/O residual | **97 → 22** (15 ungated + 7 advisory) |
| Effect-manifest manual-only entries | **106 → 41** (34 `ManualOnlyDirectIo` + 7 `ManualOnlyAdvisory`) |
| Effect-manifest `ProviderBacked` | **18 → 84** |
| Real security defects fixed | 1 high (`target`), 2 medium (`radius`, `dnsbl`), 1 medium (manifest/registration drift) |
| M005E baseline coverage | 97/97 classified, now guard-enforced |
| Hosted CI on closure SHA `d4a22f1` | fully green (run `36640412317`, 5/5 jobs) |
| Eggsec automated NSE | unchanged — still quarantined |

**GO for M007C.**

## 2. The audit that changed the plan's premise

### 2.1 The scan defect

`m005e_production_socket_files()` in `scripts/check-boundaries.sh` matched three patterns over production source. `TcpStream::connect` is a substring of `BrokeredTcpStream::connect`, so every library M007B had migrated matched exactly like an unmigrated one. The plan's "39 specialized residual files" and "17 unresolved `BrokerCompatible*` entries" were both consequences.

The corrected scan (`nse_specialized_residual`) anchors every pattern behind `(^|[^A-Za-z0-9_])`, scans production code only, drops comment-only lines, adds direct DNS resolution (`to_socket_addrs`, `ToSocketAddrs`, `lookup_host`, `hickory`, `tokio::net`), and restricts the zone to `src/libraries/**` + `src/public_api/api.rs` so provider/broker infrastructure is not a protocol residual.

### 2.2 The 17-file audit record

Plan §6 required a per-file record. The `BrokerCompatibleTcp` set reduced to:

| File | Direct effect sites | Entry reaching them | Manual-only vs automated-reachable | Provider-contract fit | Final class | Action | Manifest eligibility |
|---|---|---|---|---|---|---|---|
| `ftp.rs` | 0 (2 `BrokeredTcpStream::connect`) | `ftp.connect`/`list`/… | automated-reachable | fits | `BrokerCompatibleTcp` | already migrated; scan corrected | promoted → `ProviderBacked` |
| `imap.rs` | 0 (1) | `imap.connect` | automated-reachable | fits | `BrokerCompatibleTcp` | already migrated | promoted |
| `mongodb.rs` | 0 (14) | `mongodb.*` | automated-reachable | fits | `BrokerCompatibleTcp` | already migrated | promoted |
| `mssql.rs` | 0 (6) | `mssql.*` | automated-reachable | fits | `BrokerCompatibleTcp` | already migrated | promoted |
| `mysql.rs` | 0 (1) | `mysql.*` | automated-reachable | fits | `BrokerCompatibleTcp` | already migrated | promoted |
| `openssl.rs` | 0 (2 + 2 comment mentions) | `openssl.connect*` | automated-reachable | TLS-over-brokered stream, no handle escape | `BrokerCompatibleTcp` | already migrated | promoted |
| `postgres.rs` | 0 (1) | `postgres.*` | automated-reachable | fits | `BrokerCompatibleTcp` | already migrated | promoted |
| `rdp.rs` | 0 (1) | `rdp.connect` | automated-reachable | fits | `BrokerCompatibleTcp` | already migrated | promoted |
| `redis.rs` | 0 (3) | `redis.*` | automated-reachable | fits | `BrokerCompatibleTcp` | already migrated | promoted |
| `sip.rs` | 0 (1 comment mention) | `sip.options`/`invite` | automated-reachable | fits | `BrokerCompatibleTcp` | already migrated | promoted |
| `smb.rs` | 0 (1) | `smb.*` | automated-reachable | fits | `BrokerCompatibleTcp` | already migrated | promoted |
| `smtp.rs` | 0 (1) | `smtp.*` | automated-reachable | fits | `BrokerCompatibleTcp` | already migrated | promoted |
| `sslcert.rs` | 0 (1 + 1 comment mention) | `sslcert.connect` | automated-reachable | TLS-over-brokered stream | `BrokerCompatibleTcp` | already migrated | promoted |
| `telnet.rs` | 0 (2) | `telnet.connect` | automated-reachable | fits | `BrokerCompatibleTcp` | already migrated; module unregistered | promoted (classification retained) |
| `tls.rs` | 0 (1) | `tls.connect_tcp` | automated-reachable | TLS-over-brokered stream | `BrokerCompatibleTcp` | already migrated | promoted |
| `vnc.rs` | 0 (7) | `vnc.*` | automated-reachable | fits | `BrokerCompatibleTcp` | already migrated | promoted |
| `radius.rs` | **1 real** — `AsyncUdpSocket::bind` + `connect` at `src/libraries/radius.rs:93` | `radius.connect_async` | automated-reachable **but classified `ManualOnlyDirectIo`** | fits (connected UDP, no datagram transfer) | `BrokerCompatibleUdpConnected` | **migrated** → `broker_udp_connect`; promoted | promoted → `ProviderBacked` |

**Result: 0 files retain a `BrokerCompatible*` classification with an unexplained automated-relevant direct socket effect.** Plan §6's prohibition is satisfied. No `Outcome B` reclassification was needed for the 16 — the correct action was to fix the instrument, not to relabel honest work.

## 3. Findings the corrective pass discovered beyond its brief

The corrected scan was also run across *all* registered modules, not just the 17. Four real defects surfaced.

### 3.1 HIGH — `target.resolve` was an unbrokered DNS effect classified `Pure`

`src/libraries/target.rs:113` (line number as of the pre-fix tree) called `format!("{}:0", hostname).to_socket_addrs()`. The effect manifest classified `target` as `Pure`, which is automated-safe under ADR-0004 §1. `target` **is** registered — `register_target_library`, called unconditionally from `register_libraries()` — so under `AgentSafe` and `CiSafe` a script could call `target.resolve("attacker.example")` and cause a real DNS query to leave the process with no capability check, no cancellation, and no accounting.

The severity is worse than "a missing gate", because it also defeated a profile-level control. `ResolvedNseExecutionProfile::ci_safe()` sets `network_policy: NseNetworkPolicy::DenyAll`; a direct `to_socket_addrs()` call consults no capability context, so it could not be denied. The documented "CI continues to use `CiSafe`, which has no network access" boundary (ADR-0004 §8) therefore did not hold for this entry point. The egress was also unauthenticated by any scope decision, so it was usable as a DNS exfiltration and existence channel by any script running under an automated profile.

`target_resolve_respects_the_ci_safe_zero_network_budget` pins the corrected property: under `CiSafe` the lookup is refused with zero provider contact.

**What the fix does *not* close.** `NseCapabilityContext::after_blocking_operation` counts only `NetworkTcp`/`NetworkUdp` into `network_operations`; `DnsResolution` has no counter. So a *permitted* brokered DNS lookup is not charged against `max_network_operations` (which is `Some(0)` under `CiSafe` and the `automated_defaults()` value under `AgentSafe`). That is pre-existing M005B behavior shared with the already-`ProviderBacked` `dns` library, and `DenyAll` already refuses it under `CiSafe` — but under `AgentSafe` a permitted `target.resolve`/`dns.resolve` loop is bounded only by the wall-clock and instruction budgets. Recorded as a low finding below rather than claimed as closed.

The M005E inventory could not see any of this: its scan covered only connect/bind call sites, never resolution. `target` was in neither the residual pins nor the review that produced them.

Fix: `broker_dns_lookup` (M005B) behind a new `register_target_library_with_services`; `register_target_library(lua)` retained as a compatibility wrapper. Reclassified `Pure` → `ProviderBacked` (automated eligibility unchanged, so no availability change; the classification is now truthful).

### 3.2 MEDIUM — `radius.connect_async` was claimed migrated and was not

`scripts/nse-migration-classes.txt` at `699d374` recorded `radius.rs BrokerCompatibleUdpConnected … (rewired to broker_udp_connect)`. The source still performed `AsyncUdpSocket::bind("0.0.0.0:0")` then `.connect(format!("{host}:{port}"))` on the ambient Tokio handle — a direct bind to a wildcard local address plus a direct connect to a caller-supplied host. The class was a false claim.

Fix: `broker_udp_connect` behind `register_radius_library_with_services`; `register_radius_library(lua)` retained. Promoted to `ProviderBacked`.

### 3.3 MEDIUM — `dnsbl` had two direct DNS effects and a process-global resolver

`src/libraries/dnsbl.rs` used `std::net::ToSocketAddrs::to_socket_addrs` in `check` and `check_multi`, and a `OnceLock<hickory_resolver::TokioResolver>` in `check_async`. `hickory` was never in the M005E pattern set, and `dnsbl` is never registered, so the module was invisible to both the residual pins and the manifest.

Fix: all lookups go through `broker_dns_lookup`; the process-global resolver is deleted; `check_async` aliases the brokered path (the same bridge-removal pattern used across the M007B cohort). Left unregistered — registering it would be a new Lua-surface capability and is out of scope; it is pinned in the registration compat allowlist.

### 3.4 MEDIUM — the manifest had drifted from the registration surface in both directions

- **66 of 147** registered entries recorded a `register_fn` that no longer existed. M007B's migration rewrote call sites to `register_X_library_with_services`, but the manifest kept the base-name prefix. M007A's guard passed only because it tested `rg -q -F "register_afp_library"` against the manifest, which the longer name contains.
- **12 manifest entries described modules that are never registered** — `finger`, `kafka`, `match_lib`, `mqtt`, `nse_string`, `nse_table`, `packet`, `sftp`, `stun`, `telnet`, `websocket`, `whois`. This is the M007A closure's "low-severity finding" (§10), confirmed at 12 rather than 13, plus four further unregistered modules with no manifest entry at all (`dnsbl`, `elasticsearch`, `nsedebug`, `strict`).

Effect: a library could be promoted to `ProviderBacked` on a classification that described a function nobody calls, and an orphaned entry would not have failed CI.

Fix: `register_fn` corrected to the function actually called; `scripts/nse-registration-compat-entries.txt` pins all 16 unregistered modules with reviewed rationales; enforcement added in both a Rust test (via `include_str!`, so it holds in every feature combination) and the shell guard.

### 3.5 The guard could not have caught §3.4 on its own

A hand check of "does every one of the 97 baseline entries still have a class" found that `src/libraries/finger.rs` had been dropped while this record was being written. The M005E 97-file baseline was implicit — it lived in the very pin files M007B legitimately overwrote — so it could only be verified by comparing against git history. It is now `scripts/nse-m005e-direct-io-baseline.txt`, frozen and guard-enforced in both directions.

## 4. Requirement-to-evidence matrix (corrective plan §13)

| # | Acceptance criterion | Evidence | Result |
|---|---|---|---|
| 1 | All 97 original M005E residual entries retain exactly one final classification | `scripts/nse-m005e-direct-io-baseline.txt` (97, frozen) ⊆ `scripts/nse-migration-classes.txt` (99 = 97 + `dnsbl` + `target`); enforced by `m005e_baseline_keeps_exactly_one_class` and the shell baseline check | pass |
| 2 | All 17 still-direct `BrokerCompatible*` entries fully migrated or accurately reclassified | §2.2 table; 16 already migrated, `radius` migrated in this pass | pass |
| 3 | No closure-time `BrokerCompatible*` entry retains unexplained direct effects | `M007B violation: <path> is classified BrokerCompatible* but still has a direct host network effect` guard; verified by negative probe | pass |
| 4 | Every promoted module is `ProviderBacked` | `migrated_cohort_is_promoted_to_provider_backed` (65 names, includes `radius`/`target`); `automated_profiles_expose_promoted_cohort` (26 globals × 2 profiles) | pass |
| 5 | Every remaining direct/advisory module remains automated-denied | `unresolved_residual_stays_manual_only` (22 names × 2 profiles); `automated_direct_globals_absent_for_unsafe_libraries` (6 × 2); guard pins `tftp`/`ssh`/`snmp`/`eigrp`/`packet` to `ManualOnly*` | pass |
| 6 | Reverse manifest→registration consistency enforced with explicit compat allowlisting | `registration_and_manifest_agree`; `scripts/nse-registration-compat-entries.txt`; shell guard fails both on a new orphan and on a listed module that has since been registered | pass |
| 7 | Specialized direct-I/O residual pins match current source truth | pins regenerated; `nse_specialized_residual` == pin union; verified by negative probe (injecting a direct connect into `afp.rs` fails) | pass |
| 8 | Provider/broker infrastructure not misclassified as protocol residual | zone excludes `src/providers.rs` / `src/brokered_stream.rs`; explicit guard rejects them in the pins; verified by negative probe | pass |
| 9 | Residual materially below the M005E 97 baseline; exact before/after recorded | 97 → 22; §5 | pass |
| 10 | Boundary guards green | `./scripts/check-boundaries.sh` → `standalone boundary and provenance checks passed` | pass |
| 11 | Full standalone suite / MSRV / package qualification green | §6 | pass |
| 12 | Hosted Ubuntu/macOS/Windows/MSRV/SSH CI fully green on the exact closure SHA | run `36640412317` on `d4a22f1` — §7 | pass |
| 13 | `007b-closure.md` records exact classification, residual, and manifest deltas | §5 | pass |
| 14 | M007C remains blocked until 1–13 pass | §1–§7 satisfied, then unblocked in `plans/registry.md` | pass |
| 15 | Automated Eggsec NSE remains quarantined | no Eggsec production change; `git log` in `eggsec` shows planning-only commits | pass |

## 5. Exact deltas

### 5.1 Migration classification (99 classified paths)

| Class | M005E/M007B baseline (97) | Final (99) | Delta |
|---|---:|---:|---:|
| `BrokerCompatibleTcp` | 74 | 74 | 0 (all migrated; none retain a direct effect) |
| `BrokerCompatibleUdpConnected` | 1 | 1 | 0 (`radius` migrated in this pass) |
| `UnconnectedDatagram` | 17 | 17 | 0 (manual-only) |
| `NativeHandleEscape` | 3 | 3 | 0 (manual-only) |
| `RawPacketOrInterface` | 1 | 1 | 0 (manual-only) |
| `PublicCompatibilityApi` | 1 | 1 | 0 (manual/native) |
| `AsyncDirectIo` | 0 | 0 | 0 (audit found no structural async) |
| `ProviderBackedDns` | — | 2 | +2 (`dnsbl`, `target`; found by the corrected scan) |
| **Total** | **97** | **99** | **+2** |

### 5.2 Specialized direct-I/O residual

| Inventory | M005E | At `699d374` (broken scan) | Final |
|---|---:|---:|---:|
| Ungated | 72 | — | 15 |
| Advisory | 25 | — | 7 |
| Total pinned | 97 | 97 (stale, hence red CI) | **22** |
| Files the broken scan reported | — | 40 (incl. `src/brokered_stream.rs`) | — |
| Specialized residual after excluding provider infra | — | 39 | **22** |
| Of those, labeled `BrokerCompatible*` | — | 17 | **0** |

Final 22, all shapes the current provider contract cannot represent:

- **Unconnected / broadcast / multicast UDP (16)**: `bjnp`, `coap`, `dhcp`, `dhcp6`, `eigrp`, `iax2`, `ike`, `ipmi`, `knx`, `natpmp`, `ntp`, `snmp`, `srvloc`, `stun`, `tftp`, `wsdd`, `xdmcp`
- **Raw packet / interface (1)**: `packet`
- **Native socket handoff to `ssh2::Session` (3)**: `libssh2`, `ssh`, `ssh2`
- **Public sync compatibility surface (1)**: `public_api/api.rs`

Ungated (15): `bjnp`, `coap`, `eigrp`, `iax2`, `ike`, `ipmi`, `knx`, `natpmp`, `packet`, `srvloc`, `ssh2`, `stun`, `tftp`, `wsdd`, `public_api/api.rs`.
Advisory (7): `dhcp`, `dhcp6`, `libssh2`, `ntp`, `snmp`, `ssh`, `xdmcp`.

### 5.3 Effect-manifest eligibility

| Eligibility | M007A | Final | Delta |
|---|---:|---:|---:|
| `ManualOnlyDirectIo` | 80 | 34 | −46 |
| `ManualOnlyAdvisory` | 26 | 7 | −19 |
| **Manual-only total** | **106** | **41** | **−65** |
| `Pure` | 35 | 34 | −1 (`target` reclassified) |
| `ProviderBacked` | 18 | 84 | **+66** |
| **Automated-safe total** | **53** | **118** | **+65** |
| Manifest entries | 159 | 159 | 0 |

All 159 entries remain classified. The 65 promotions are exactly the migrated registered modules plus `target` and `radius`.

Of the 41 remaining manual-only entries, 21 are the residual library modules themselves (the 22nd residual file, `public_api/api.rs`, is the public sync compatibility surface and has no registered-Lua manifest entry). The other 20 are conservative classifications of modules with no direct network effect: stubs (`eap`, `gps`, `sasl`, `multicast`, `pppoe`, `rpc`, `ospf`, `giop`, `vuzedht`, `dnssd`), unregistered compatibility modules (`finger`, `kafka`, `mqtt`, `sftp`, `telnet`, `websocket`, `whois`), and direct-reqwest modules (`mobileme`, `httpspider`). Conservative over-classification is fail-closed, not a safety defect; see §12.

### 5.4 `register_fn` drift

| Measure | Before | After |
|---|---:|---:|
| Registered modules in `register_libraries()` | 147 | 147 |
| Manifest entries whose `register_fn` matches the called function | 81 | 147 |
| Manifest entries with no registration call site | 12 | 12 (now allow-listed) |
| Unregistered library modules without a reviewed rationale | 16 | 0 |

## 6. Verification executed

### 6.1 Commands run (final closure SHA)

```bash
cargo fmt --all --check
./scripts/check-boundaries.sh
cargo check --no-default-features
cargo check --features nse
cargo test --features nse
cargo check --features nse-ssh2
cargo check --features nse,sandbox
cargo clippy --all-targets --features nse
cargo +1.89.0 check --locked --no-default-features
cargo +1.89.0 check --locked --features nse
cargo +1.89.0 test --locked --features nse --lib
cargo package --list
cargo package
```

### 6.2 Results

- `cargo fmt --all --check`: pass.
- `./scripts/check-boundaries.sh`: `standalone boundary and provenance checks passed`.
- `cargo check --no-default-features` / `--features nse` / `--features nse-ssh2` / `--features nse,sandbox`: pass (89–91 pre-existing dead-code/deprecation warnings, unchanged class; one fewer than the pre-corrective baseline after the new tests).
- `cargo test --features nse`: **696 passed, 0 failed** across 29 test binaries (lib 208 + 28 integration) plus doc-tests. Named: `effect_manifest_tests` 23, `m007b_migration_tests` 15, `local_protocol_tests` 61, `brokered_stream_tests` 7, plus provider composition/broker, network provider, HTTP provider, fs/process, runtime corpus, compatibility corpus, evidence, limits, profile, report, and resolver suites.
- `cargo clippy --all-targets --features nse`: no errors; 222 warnings vs 223 on the pre-corrective baseline (net −1). No new warning class.
- `cargo +1.89.0 check --locked --no-default-features` / `--features nse`: pass. `cargo +1.89.0 test --locked --features nse --lib`: 207 passed.
- `cargo package --list --allow-dirty`: 319 files. `cargo package`: `Packaged 319 files, 3.3MiB` + verify compile ok.
- Explicit targeted runs: `effect_manifest` (lib) 12/12, `effect_manifest_tests` 23/23, `m007b_migration_tests` 15/15, `brokered_stream_tests` 7/7, `local_protocol_tests` 61/61.

### 6.3 Focused migration evidence (plan §11)

`tests/m007b_migration_tests.rs`:

| Evidence | Test |
|---|---|
| loopback/injected-provider success | `target_resolve_uses_injected_dns_provider`, `radius_connect_async_uses_injected_udp_provider`, `promoted_tcp_cohort_in_scope_connect_is_byte_accounted`, `promoted_tcp_cohort_write_and_read_are_byte_accounted` |
| literal fast path makes no provider call | `target_resolve_literal_ip_fast_path_makes_no_provider_call` |
| `CiSafe` zero-network budget is not evaded | `target_resolve_respects_the_ci_safe_zero_network_budget` |
| out-of-scope denial before host contact | `promoted_tcp_cohort_out_of_scope_denied_before_host_contact` (0 TCP provider calls), `radius_connect_async_out_of_scope_denied_before_host_contact` (0 UDP provider calls), `target_resolve_denied_when_resolution_denied` (0 DNS provider calls) |
| CiSafe denial | `target_resolve_ci_safe_denied_before_host_contact`, `radius_connect_async_ci_safe_denied_before_host_contact`, `ci_safe_denies_promoted_tcp_cohort_before_host_contact`, `dnsbl_check_ci_safe_never_touches_resolver` |
| cancellation before connect/send/receive | `promoted_tcp_cohort_cancelled_before_connect_touches_no_provider`, `brokered_stream_cancelled_means_zero_provider_contact`, `brokered_stream_send_denied_after_connect_is_surfaced` |
| read/write accounting | `promoted_tcp_cohort_write_and_read_are_byte_accounted` (asserts both `network_bytes_written ≥ 5` and `network_bytes_read ≥ 5`), `brokered_stream_accounting_uses_correct_buckets` |
| bounded-timeout behavior delta | `brokered_stream_default_timeout_stays_bounded` |

### 6.4 Guard negative probes (all confirmed to fail closed)

| Probe | Expected failure | Observed |
|---|---|---|
| append a `TcpStream::connect_timeout` to `src/libraries/afp.rs` | residual pin mismatch | `M007B violation: specialized direct-I/O residual changed` |
| add `src/brokered_stream.rs` to the residual pins | provider infra rejected | `M007B violation: specialized direct-I/O residual changed` |
| add an unregistered `pub fn register_*` module | reverse consistency | `M007B violation: library module src/libraries/probelib.rs … is not listed in scripts/nse-registration-compat-entries.txt` |
| relabel `tftp.rs` as `BrokerCompatibleTcp` | class/residual disagreement | `M007B violation: src/libraries/tftp.rs is classified BrokerCompatibleTcp but still has a direct host network effect` |
| delete the `afp.rs` classification line | baseline coverage | `M007B violation: M005E baseline entries lost their final migration class: src/libraries/afp.rs` |
| promote `mobileme` (direct `reqwest`) to `ProviderBacked` | direct-HTTP cross-check | `M007B violation: direct-HTTP module src/libraries/mobileme.rs (manifest name 'mobileme') is not ManualOnly` |

### 6.5 Guard robustness fix

The first draft of the corrected scan used `rg -q` as the final pipeline stage. Under `set -o pipefail`, `rg -q` exits on first match and SIGPIPEs the upstream `rg -v` stage, which can turn a true match into a false "no match". The stage now uses `-c` (count, always drains input) and a `|| true` guard. This was found and fixed before the closure SHA; it is recorded because it is the kind of defect that makes a guard silently permissive.

## 7. Hosted CI evidence

| Run | SHA | Conclusion | Jobs |
|---|---|---|---|
| `36620331641` | `699d374` (inherited) | **failure** | msrv success; rust(ubuntu) **failure**; rust(macos) **failure**; ssh-runtime success; rust(windows) success |
| `36639253621` | `0a6a826` | **success** | msrv, rust(ubuntu), rust(macos), rust(windows), ssh-runtime — 5/5 |
| `36639656030` | `42e4861` | **success** | msrv, rust(ubuntu), rust(macos), rust(windows), ssh-runtime — 5/5 |
| `36640412317` | `d4a22f1` (**closure SHA**) | **success** | msrv, rust(ubuntu), rust(macos), rust(windows), ssh-runtime — 5/5 |

The `699d374` failure was `M005E violation: direct-socket file set changed` on the Ubuntu and macOS jobs, i.e. the stale-pin condition the corrective plan set out to resolve.

## 8. Invariant review (corrective plan §4)

| Invariant | Evidence | Result |
|---|---|---|
| Automated NSE remains quarantined in Eggsec | No Eggsec production change; this record and the registry/roadmap updates are planning-only | holds |
| M007A effect gate remains fail-closed | `scrub_ineligible_globals` untouched; unknown names still default to `ManualOnlyDirectIo`; `unknown_library_fails_closed` green | holds |
| Any library with an unresolved direct host-network effect remains automated-unavailable | `unresolved_residual_stays_manual_only`; guard pins the representative set; direct-reqwest modules forced `ManualOnly*` | holds |
| A module is not promoted merely because its most common connect path is brokered | promotion required zero residual sites + `_with_services` registration + broker usage; `brute`/`upnp` were promoted only after confirming no remaining mixed path | holds |
| File-level promotion requires all automated-relevant network effects provider-backed or unreachable | promotion predicate applied per registered module; the corrected scan is the "all effects" evidence | holds |
| The direct-I/O source scan was not weakened to make CI green | the scan was **strengthened** (anchoring, comment stripping, DNS coverage, zone restriction) and the pin count fell 97 → 22; §6.4 probe 1 shows it still catches a reintroduced direct connect | holds |
| `src/brokered_stream.rs` is provider/broker infrastructure, not a protocol residual | excluded from the zone and explicitly rejected in the pins | holds |
| Unconnected/broadcast/multicast UDP is not forced through the connected UDP provider | all 16 remain `UnconnectedDatagram`/manual-only | holds |
| Native socket handoff to `ssh2::Session` remains manual-only | `ssh`, `ssh2`, `libssh2` unchanged | holds |
| Raw packet/interface operations remain manual-only | `packet` unchanged | holds |
| `src/public_api/api.rs` remains manual/native | unchanged | holds |
| No breaking public API | `register_target_library(lua)`, `register_radius_library(lua)`, `register_dnsbl_library(lua)` retained as wrappers; only additive `*_with_services` variants; `cargo package` verifies; MSRV 1.89 check passes | holds |
| MSRV remains Rust 1.89 | `cargo +1.89.0 check --locked` × 2, `+1.89.0 test --lib` pass | holds |

## 9. Stop conditions (corrective plan §14)

None triggered.

- *A supposedly broker-compatible file required async provider redesign* — no; 16 were already migrated, `radius` fitted the existing connected-UDP contract.
- *A direct effect required unconnected/broadcast/multicast UDP not supported by the current provider* — yes, for 16 files, and the correct response was to leave them manual-only and label them accurately (which the M007B baseline already did). This is a recorded residual, not a stop, because the plan explicitly places UDP provider redesign out of scope.
- *A native library requires concrete socket ownership* — yes, for `ssh`/`ssh2`/`libssh2`; left manual-only per invariant.
- *A module could only be promoted by ignoring a direct effect* — no; no promotion ignored a site.
- *Making the guards green required weakening the source scan* — no; the scan was strengthened and the plan's prohibition was honoured.
- *A breaking public API was required* — no.
- *Any automated Eggsec exposure changed* — no.

## 10. Migration and compatibility review

No schema, protocol, or config migration. Public API strictly additive: three new `*_with_services` registration entry points, no signature removed or changed. Lua-visible behavior changes, all deliberate:

1. **65+2 libraries become available under `AgentSafe`/`CiSafe`.** This is the promotion the plan required. Availability widens; authority does not — see §11.
2. **`target.resolve` failure mode.** On a broker denial it returns the input hostname instead of the old best-effort answer; the pre-existing "return the hostname" fallback shape is preserved, so scripts that treated non-resolution as identity still behave the same.
3. **`dnsbl.check_async`** no longer resolves through a separate hickory resolver; it shares the brokered path, so its result is now subject to the same capability gate as `check`. The module is unregistered, so no live script observes this.
4. **`radius.access_request_async`** dropped its artificial 50 ms Tokio sleep for a cancellation-aware check. The entry is a stub either way.
5. **One documented pre-M007B behavior delta carried forward**: `BrokeredTcpStream` maps `set_read_timeout(None)` / `set_write_timeout(None)` (infinite blocking) to a bounded 120 s default, because the provider contract requires a concrete duration. Pinned by `brokered_stream_default_timeout_stays_bounded`.

Rollback: standalone commits `d4a22f1`, `42e4861`, `0a6a826` revert cleanly in reverse order; no cross-repo coupling and no Eggsec production dependency.

## 11. Security review

**The high finding is closed.** `target.resolve` no longer performs an unbrokered DNS query; `CiSafe` can no longer be made to emit DNS through a `Pure`-classified registered library.

**Promotion does not weaken authority.** The eligibility change is a classification, not a policy change. The runtime network policy is untouched: `ResolvedNseExecutionProfile::agent_safe(target, cidrs)` builds `AllowCidrs(scope)` or `AllowResolvedTargetSet([approved target])`, and `ci_safe()` builds `DenyAll`. `broker_tcp_connect_endpoint` / `broker_udp_connect_endpoint` build the capability request from the **concrete resolved endpoint** (`endpoint.address.to_string()`), and `NseCapabilityContext::evaluate` performs a real membership decision against that literal. So a promoted library under `AgentSafe` can only reach the approved target address set, and under `CiSafe` cannot reach anything. §6.3 proves the denial happens before provider contact. Eggsec's `ScopeAuthority` remains the authoritative broader language; the narrower runtime policy is the intended conservative mapping per ADR-0004 §6.

**One honest limitation, recorded rather than papered over**: `broker_dns_lookup` gates on `DnsResolution` with the hostname as the target, and the `DnsResolution` arm only denies on `DenyAll` — it does not perform per-target membership. That is pre-existing M005B behavior for the already-`ProviderBacked` `dns` library, and `target.resolve` inherits it. It is a metadata/timing and egress-surface concern, not a scope-bypass of the connection path (no connection is made), and M007D's approved-scope threading is where the runtime DNS policy is bound to Eggsec scope. Listed in §12 as medium, not closed by this pass.

No secrets, path, or privilege changes. DoS bounds unchanged. Audit trail: denials and warnings flow through the existing `tracing` and `report.capability_events` paths; no `let _ =` or `filter_map(ok)` silencing was introduced (the one pre-existing `pairs::<i32, String>().flatten()` in `dnsbl` was replaced with an explicit match that logs at `debug`).

## 12. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| medium | `broker_dns_lookup` gates `DnsResolution` on `DenyAll` only; it does not evaluate per-target membership for the resolved name | A promoted `ProviderBacked` library can resolve a name outside the approved target set under `AgentSafe` (no connection is possible; egress/timing surface only) | Bind the runtime DNS policy to approved scope in M007D (`007-approved-scope-provider-activation.md`); do not treat `dns`/`target` DNS resolution as scope-authoritative before then |
| low | 20 of the 41 remaining manual-only entries have no direct network effect (stubs, unregistered compatibility modules, direct-reqwest modules) | Conservative over-classification: fail-closed, not a safety defect. Costs automated availability for libraries that are in fact inert | Optional M007C polish: re-derive eligibility for the stub cohort from the same corrected scan, with the direct-HTTP guard still binding `mobileme`/`httpspider`/`elasticsearch` |
| low | `DnsResolution` is not charged to `network_operations`, so a permitted brokered DNS lookup is bounded only by wall-clock/instruction budgets (pre-existing M005B behavior shared with the `dns` library; `DenyAll` already refuses it under `CiSafe`) | A DoS-budget gap for DNS-heavy scripts under `AgentSafe`, not a scope or authority bypass | Add a DNS resolution counter when the runtime DNS policy is bound to approved scope in M007D |
| low | Non-network direct host effects remain in the promoted cohort: `rand::random()` (`anyconnect`, `isns`, `openssl`, `sip`, `smb`, `versant`), `thread::sleep` (`brute`, `irc`, `netbios`, `oracle`), `SystemTime`/`Instant` reads (`msrpcperformance`, `mysql`, `oops`, `versant`) | Not network/authority effects and out of this plan's residual definition (the M005A clock/random policy is closed scope). The `thread::sleep` sites are cancellation-unaware and are a liveness concern under automated profiles | Route a randomness/cancellation migration to its own milestone; do not reopen M005A/M007B |
| low | `upnp.discover` now performs a brokered TCP connect to the SSDP multicast group `239.255.255.250:1900` (inherited from `699d374`) | Safe (the capability gate refuses it out of scope; in-scope it will simply fail to connect). Functional regression: UDP multicast discovery no longer works | Restore real SSDP semantics as part of a future unconnected/multicast provider work; explicitly out of scope here |
| low | `upnp`/`brute` classification rationales in `699d374` said "mixed … stays manual-only-gated with file" | The rationale was stale: both are fully brokered and promoted. Corrected in `scripts/nse-migration-classes.txt` and the manifest | none |
| low | 89–91 pre-existing lib warnings (dead code, deprecated `as_utf8`, one unreachable pattern in `capabilities.rs`) | Noise only; unchanged class, net −1 warning | out of scope |
| low | `scripts/m007b_migrate.py` is a spent one-shot tool kept as a record | Rerunning it is a no-op that exits non-zero; docstring now says so | none |

No critical or high findings remain. All four real defects found by the corrective pass are fixed and covered by tests.

## 13. Requirement-to-test map (corrective plan §11)

| Required test class | Where |
|---|---|
| Focused migration tests for every promoted cohort | `tests/m007b_migration_tests.rs`; `tests/brokered_stream_tests.rs`; `tests/local_protocol_tests.rs` (61, including 10 automated-denial tests) |
| Promoted module is automated-safe | `migrated_cohort_is_promoted_to_provider_backed`, `automated_profiles_expose_promoted_cohort` |
| Remaining direct/advisory module is automated-denied | `unresolved_residual_stays_manual_only`, `automated_direct_globals_absent_for_unsafe_libraries` |
| Direct-global and `require()` paths agree | `automated_dynamic_require_blocked_for_representative_unsafe`, `automated_dynamic_require_succeeds_for_safe_module` |
| Reverse manifest-registration consistency | `registration_and_manifest_agree` + `scripts/nse-registration-compat-entries.txt` guard |
| No promoted module contains specialized direct socket effects | `nse_specialized_residual` == pin union; negative probe §6.4/1 |
| No unclassified specialized direct socket file exists | residual-requires-class guard; negative probe §6.4/4 |
| No provider infrastructure accidentally enters residual pins | explicit guard; negative probe §6.4/2 |
| M005E baseline retention | `m005e_baseline_keeps_exactly_one_class` + baseline guard; negative probe §6.4/5 |
| Direct-HTTP module cannot be promoted | direct-HTTP cross-check; negative probe §6.4/6 |
| Regression | full 696-test suite, incl. provider composition/broker, network provider, HTTP provider, fs/process, runtime corpus, compatibility corpus |

## 14. Documentation and operations

- `docs/PROVIDERS.md`: M005B text corrected to the 22-file residual; new `## M007B` section (adapter contract and the 120 s timeout delta, the four corrective findings with resolutions, the residual breakdown by reason, the promotion rule, the seven new guard groups, the test map); M007A counts and the registration↔manifest section updated; `dnsbl` removed from the runtime-plumbing row.
- `scripts/nse-m005e-direct-io-baseline.txt` (new, 97 paths, labelled history).
- `scripts/nse-migration-classes.txt`: rewritten with one final class per baseline entry plus rationale, and the class vocabulary extended with `ProviderBackedDns`.
- `scripts/nse-registration-compat-entries.txt` (new, 16 entries with reviewed rationales).
- `scripts/nse-specialized-{ungated,advisory}.txt`: regenerated (15 + 7).
- `scripts/check-boundaries.sh`: M005E socket section replaced by the corrected M007B scan; M007A representative-residual block updated; seven new M007B guard groups; `pipefail`/SIGPIPE robustness fix.
- `scripts/m007b_migrate.py`: docstring marked as a spent one-shot tool.
- `.gitignore`: `__pycache__/` and `*.pyc` added (a stray `.pyc` was being packaged).
- Operator diagnostics: `eligibility_counts()` / `classified_libraries()` for manifest inventory; `report.capability_events` now carries the denial evidence that automated-profile tests assert against; `blocked-by-policy` warnings unchanged.

## 15. Roadmap disposition

**GO for M007C** (`007-standalone-security-patch-release.md`).

M007B is closed on its own merits: the residual is reconciled to source, the pins describe current reality, the M005E baseline is preserved and enforced, the manifest is consistent with the registration surface in both directions, and hosted CI is green on the exact closure SHA. M007C's stated precondition — "perform a public-API compatibility gate only after `007b-closure.md` exists and hosted standalone CI is fully green" — is now satisfied.

This closure also **closes the M007A low-severity finding** recorded in `007a-closure.md` §10 row 1 (manifest→registration was unenforced) and replaces the 007A §10 row 1 wording "13 extra entries" with the verified 12 + 4.

M007D (`007-approved-scope-provider-activation.md`) and M007E (`007-controlled-automated-reexposure-qualification.md`) remain dependency-gated behind M007C and M007D respectively. The medium finding in §12 (runtime DNS policy is not bound to approved scope) is explicitly in scope for M007D and must not be treated as resolved here.

Automated Eggsec NSE remains quarantined. This milestone changed no Eggsec production surface and enabled no automated operation.

## 16. Registry and roadmap updates

- `plans/registry.md`: M007B → closed with both plans cited; `007-protocol-migration-corrective.md` → closed; `007-standalone-security-patch-release.md` → ready for handoff (unblocked, precondition satisfied); handoff-boundary note moved from M007B to M007C; the NSE subsystem row advanced to "M007B closed".
- `plans/subsystems/nse-runtime-extraction-roadmap.md`: milestone 007 status advanced; M007C is the current handoff; M007D/E remain gated.
- `plans/implementation/nse-runtime-extraction/007-broker-compatible-protocol-migration.md`: `Status: implemented; corrective closure required` → `Status: closed (corrective)`, with a pointer to this record.
- `plans/implementation/nse-runtime-extraction/007-protocol-migration-corrective.md`: `Status: ready for handoff` → `Status: closed`, with a pointer to this record and a note that its §6 premise was falsified by the audit (recorded rather than silently dropped, per `003-planning-process.md` §7).
