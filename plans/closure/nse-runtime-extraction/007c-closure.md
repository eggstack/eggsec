# NSE Runtime Extraction Milestone 007C — Closure Status

Status: blocked

Source implementation plan:

- `plans/implementation/nse-runtime-extraction/007-standalone-security-patch-release.md` (M007C)

Source subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-007--protocol-library-gating-and-controlled-automated-activation`

Applicable ADR:

- `plans/adrs/ADR-0004-nse-automated-activation-boundary.md`

Planning baseline: Eggsec `9f143759`

Implementation repository: `eggstack/eggsec-nse`

Repository baseline reviewed: `d4a22f1dbe56f4ccfb17b2a8135aae8395f44f19` (working tree at review; `0.2.0` published)

Implementation commits or pull requests: **none.** M007C is a release-only milestone and produced no code change. The finding below is a property of the already-pushed M007 tree.

Replan: `plans/implementation/nse-runtime-extraction/007-breaking-0-3-0-release.md` (M007C-R)

## 1. Executive finding

**The §3 semver gate fails with a major break. 0.2.1 was not published and must not be.**

`cargo semver-checks check-release --baseline-version 0.2.0` reports one failed major lint, `function_missing`: **74 public functions present in published 0.2.0 are absent from the current tree.** None of the 74 has a source-compatible replacement under its original path, so criterion §3 "existing constructors remain source-compatible" fails, and §3's remedy applies verbatim — *stop; mark this plan blocked; create a 0.3.0 release plan with explicit migration notes.*

The break is **not** a bookkeeping oversight that a compat shim could paper over. Two of the removed functions are `helpers::tls_connect` and `helpers::tcp_connect_with_timeout`, which in 0.2.0 were `pub` and returned a raw `std::net::TcpStream` from an unmediated `TcpStream::connect_timeout`. Restoring them would re-expose host TCP on the public API of a crate whose entire purpose (ADR-0004) is that no network effect occurs without a capability decision. That is the exact defect class M007B existed to eliminate. Patch compatibility is therefore **unachievable by construction**, not merely unverified — which is why the answer is a version bump rather than a restoration pass.

This is the second time M007's verification instruments produced a confident wrong answer, and the pattern is worth naming: in both cases a check was treated as authoritative without being read against the source it claimed to describe (M007B's `BrokerCompatible*` scan; M007B closure §8's "No breaking public API | holds"). The semver gate itself worked correctly — it was the plan's *carry-forward note* that pre-judged the outcome ("expected to remain a `0.2.1` patch release; the gate, not this note, decides") and the closure record that asserted compatibility without running a compatibility tool.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| §3 existing constructors remain source-compatible | `cargo semver-checks check-release --baseline-version 0.2.0` → `function_missing` | **fail** | 74 public functions removed; §4 |
| §3 existing provider traits not changed with required methods | same run, 196 checks: 195 pass / 1 fail / 0 warn / 57 skip | pass | only the one lint failed |
| §3 existing public structs do not gain required fields | same run | pass | no `struct_field_missing`/`field_removed` hit |
| §3 existing enum exhaustive-match surface not changed | same run | pass | no `enum_variant_removed` hit |
| §3 new APIs/builders/enums are additive | 89 `register_*_library_with_services` added | pass | additive, but 70 of them *replace* a published path rather than augment it |
| §3 behavior tightening confined to documented profile contracts | `target`/`radius`/`dnsbl` tightening documented in `007b-closure.md` | pass | not the cause of the break |
| §2 release notes disclose gating + residual honestly | not written | **not run** | gate failed first; notes authored in M007C-R |
| §5 exact candidate qualification | not run | **not run** | §3 gate precedes §5 by plan order |
| §5 hosted CI green on the candidate | run `36640412317` on `d4a22f1` is green (5/5) | pass (inherited) | from M007B closure; not re-run, no tree change |
| §6 0.2.1 absent before publication | crates.io API: versions `[0.2.0, 0.1.0]`, none yanked | pass | 0.2.1 absent; nothing to yank |
| §6 publication method | none used | **not run** | no publish, no tag, no GitHub Release |
| §6 never `--allow-dirty` / `--no-verify` | no publish attempted | pass (vacuous) | |
| §7 scratch consumer resolves registry 0.2.1 | not run | **not run** | no artifact to resolve |
| §10 no Eggsec change occurs here | `git status` in `eggstack/eggsec` clean throughout | pass | planning-side edits only |
| §11 closure unblocks M007D | M007D still requires a published artifact | **fail** | M007D remains blocked |

Publication authentication *was* available (`~/.cargo/credentials.toml` holds a crates.io registry token), so this is not an environment block. The block is a policy-correct stop, which is the intended behavior of the gate.

## 3. Production implementation evidence

None. No production, test, guard, or documentation file in `eggstack/eggsec-nse` was modified. The only repository interaction was read-only auditing plus one temporary negative probe (§8) that was reverted; `git status` is clean and `./scripts/check-boundaries.sh` and `cargo fmt --all --check` pass on the restored tree.

The 74 removals were already present in the pushed M007 tree, introduced by `699d374` (`feat(nse): M007B broker-compatible protocol migration`, the inherited commit) and untouched by the M007B corrective commits `0a6a826`, `42e4861`, `d4a22f1`.

## 4. Exact public API delta

`function_missing`, 74 items, verified individually absent from the current tree (`STILL PRESENT: 0`).

### 4.1 Registration entry points — 70 removed, 0 retained

Baseline signature, e.g. `src/libraries/nbd.rs:11` in 0.2.0:

```rust
pub fn register_nbd_library(lua: &Lua) -> LuaResult<()>
```

Current signature:

```rust
pub fn register_nbd_library_with_services(
    lua: &Lua,
    capability_ctx: &NseCapabilityContext,
    services: &NseHostServices,
) -> LuaResult<()>
```

Affected modules, all 70:

```text
afp ajp amqp anyconnect bitcoin bittorrent cassandra citrixxml cvs dicom drda finger ftp
http2 iec61850mms imap informix ipp irc iscsi isns jdwp kafka ldap libssh2_utility membase
memcached mongodb mqtt msrpc msrpcperformance mssql mysql nbd ncp ndmp netbios nrpc omp2
oops openssl oracle pgsql pop3 postgres proxy rdp redis rmi rpcap rsync rtsp sftp sip smb
smb2 smtp socks ssh1 sslcert sslv2 tls tn3270 tns versant vnc websocket whois winrm xmpp
```

Counts, measured: 94 plain `register_*_library` and 89 `register_*_library_with_services` exist; 18 modules expose both; **71 expose only the `_with_services` form, of which 70 lost a published plain entry point** (`register_telnet_library` is the 71st and was never published as a plain function, so it is additive, not a break).

The 70 are the promoted M007B cohort. Removing the one-argument form is *substantively* correct — an entry point receiving only `&Lua` had no way to obtain an `NseCapabilityContext` or `NseHostServices`, so it could only ever have constructed native defaults, which is the fallback path ADR-0004 forbids for automated authority claims. `699d374` did not record this as a deliberate public API decision, which is how it survived the M007B audit.

### 4.2 `helpers` — 4 removed, 2 of them unrecoverable

| Function | 0.2.0 signature | Restorable? |
|---|---|---|
| `helpers::tls_connect` | `(host: &str, port: u16, accept_invalid_certs: bool, accept_invalid_hostnames: bool) -> Result<(TcpStream, TlsConnector), String>` | **No** — returns a raw `TcpStream` from `TcpStream::connect_timeout` |
| `helpers::tcp_connect_with_timeout` | `(host: &str, port: u16, timeout_secs: u64) -> std::io::Result<TcpStream>` | **No** — same |
| `helpers::make_addr` | `(host: &str, port: u16) -> String` | Yes — pure `format!("{}:{}", host, port)` |
| `helpers::parse_socket_addr` | `(addr: &str) -> Result<SocketAddr, String>` | Yes — pure parse |

The two unrecoverable functions are the load-bearing reason 0.2.1 is impossible. They were `pub` on 0.2.0, so any 0.2.0 consumer could obtain an unmediated TCP connection to an arbitrary host, bypassing the capability context entirely. That is the same defect shape as the M007B high finding in `target.resolve` (`007b-closure.md` §3.1), except reachable through a public API rather than a registered Lua library — which is why the M007B audit, scoped to registered modules and residual pins, did not see them.

## 5. Verification executed

### Commands run

```bash
# registry state
curl -s https://crates.io/api/v1/crates/eggsec-nse     # versions, yank status

# the §3 gate
cargo semver-checks check-release --baseline-version 0.2.0 --features nse --color never
# → exit 100; "Checked 196 checks: 195 pass, 1 fail, 0 warn, 57 skip"
# → "--- failure function_missing: pub fn removed or renamed ---"
# → "Summary semver requires new major version: 1 major and 0 minor checks failed"

# per-item verification of the reported removals against source
# feature sets compared: published 0.2.0 vs current [features] → identical
# git archaeology: git log -S for each removed symbol
# boundary-guard negative probe (§8), reverted
./scripts/check-boundaries.sh                        # pass
cargo fmt --all --check                              # pass
```

### Results

- §3 gate: **fail (1 major)**. Full output preserved at `plans/closure/nse-runtime-extraction/007c-semver-report.txt` (attached to this record).
- Per-item verification: 74/74 absent, 0 present. Feature sets identical, so the failure is not a `--features` artifact.
- Removal provenance: `699d374` for `helpers`; the 70 registration renames likewise resolve to `699d374`.
- crates.io: `0.2.1` absent; `0.2.0` and `0.1.0` present, neither yanked. No registry mutation of any kind was performed.

### Not run, and why

`cargo metadata`, `cargo tree`, the §5 `check`/`test`/`clippy`/MSRV/`publish --dry-run`/`package` block, tag creation, `cargo publish`, GitHub Release, docs.rs confirmation, and the §7 scratch consumer were **all** not run. The plan sequences §3 before §5, and §3's remedy is to stop. Running the qualification block would have produced green output that could be mistaken for release readiness; the candidate that was never a candidate does not need one. The inherited green run `36640412317` (5/5 jobs, MSRV/Linux/macOS/Windows/SSH) on `d4a22f1` is cited as M007B evidence, not re-claimed as M007C qualification.

## 6. Invariant review

| Source-plan invariant | Evidence | Result |
|---|---|---|
| ADR-0004 §8: no network effect without a capability decision | the removals *enforce* it — `tcp_connect_with_timeout`/`tls_connect` were capability-bypassing public API; guard rejects their restoration (§8) | holds, and improved by this tree |
| Public API remains source-compatible with 0.2.0 | §4 | **violated** — 74 removals; this is the blocking finding |
| No `--allow-dirty` / `--no-verify` | no publish attempted | holds |
| Published artifact VCS identity matches candidate/tag | nothing published | n/a |
| No Eggsec change occurs in the release slice | `eggstack/eggsec` working tree clean; only `plans/` edited | holds |

Correction to a prior record: `007b-closure.md` §8 asserted `| No breaking public API | ... | holds |`, reasoning from the fact that `register_target_library`/`register_radius_library`/`register_dnsbl_library` were retained as wrappers. Those three wrappers do exist and were verified present. The inference was still wrong, because those three are the only modules where a plain entry point was deliberately re-added; the 70-module promoted cohort was never checked. That row is corrected in place and the correction is recorded in the M007B closure record's correction log, per the closure rule that a record is immutable *except for factual corrections*.

## 7. Migration and compatibility review

No schema, storage, protocol, or configuration migration is involved. The compatibility delta is a pure public-API break, itemised in §4 with a per-function replacement mapping suitable for release notes.

Nothing was published, so there is no artifact, tag, or archive to roll back, and no version was consumed. `0.2.0` remains the current published release and the comparison baseline. The M007B tree is reachable at `d4a22f1`, so the pre-release state is fully recoverable and no history was rewritten.

One consequence deserves explicit statement: **published 0.2.0 still exposes the capability-bypassing `helpers::tcp_connect_with_timeout` and `helpers::tls_connect`.** Publishing 0.3.0 is therefore not only a semver formality; it is the step that withdraws those two functions from the public surface. M007C-R should not be treated as optional polish.

## 8. Security review

The removal of the two direct-connect helpers is a security improvement and must not be reverted for compatibility. Verified rather than assumed, by negative probe:

1. Re-insert `tcp_connect_with_timeout` into `src/libraries/helpers.rs` **in production scope** (before `mod tests`).
2. `./scripts/check-boundaries.sh` → **exit 1**, `M007B violation: specialized direct-I/O residual changed`, with `src/libraries/helpers.rs` present in *actual (source)* and absent from *expected (pinned)*.
3. Reverted; `git status` clean; guard green.

A first attempt at this probe placed the function at end-of-file and the guard passed. The cause is real and worth recording: `nse_production_code()` truncates each file at its first `mod tests` line, so a direct-I/O primitive placed after a test module is invisible to the residual scan. Any future direct-I/O reintroduction in a file with an existing `#[cfg(test)]` tail must be probed pre-`mod tests` to be meaningful, and this truncation is a standing caveat on the guard's completeness.

No secrets were created, read into output, or transmitted. The crates.io token was inspected for *presence and shape only* (registry-scoped, 35 characters); its value never left the machine and is not reproduced in any record.

## 9. Documentation and operations

- No `docs/` or `README` changes: the milestone stopped before authoring release notes.
- `CHANGELOG.md` remains at 0.2.0; no 0.2.1 section was created.
- No static guard added or changed. The guard coverage gap noted in §8 (post-`mod tests` truncation) is recorded as an input to M007C-R, not fixed here, because this milestone is release-only and must not enlarge scope.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| **high** | 74 public functions removed vs 0.2.0; 0.2.1 is unachievable | Release blocked; M007D cannot adopt a registry artifact | M007C-R: publish 0.3.0 with explicit migration notes |
| **high** | Published 0.2.0 still exposes `helpers::tls_connect` and `helpers::tcp_connect_with_timeout`, both returning raw `std::net::TcpStream` from an unmediated `connect_timeout` | Any 0.2.0 consumer can bypass the capability context for TCP | M007C-R: 0.3.0 withdraws them; treat as the security motivation for the release, not a semver formality |
| medium | `699d374` removed 70 public entry points without recording it as an API decision, so two M007B reviews missed it | The removal itself is correct; the unrecorded decision is the defect | M007C-R release notes state the intent per function; M007D re-gate covers the migration |
| medium | `007b-closure.md` §8 asserted "No breaking public API \| holds" without running a compatibility tool | A closure record carried an unverified compatibility claim as evidence | Corrected in place; logged in the M007B correction log |
| medium | M007C's carry-forward note pre-judged the outcome as "expected to remain a `0.2.1` patch release" | Biased the reviewer toward a patch verdict before the gate ran | Removed; M007C-R states no expected version in advance |
| low | `nse_production_code()` truncates at the first `mod tests`, so direct-I/O after a test module is invisible to the residual scan | A future reintroduction could evade the guard if placed after a test module | M007C-R: add a whole-file (untruncated) direct-I/O sweep for `src/libraries`, or assert `mod tests` is the last item in scanned files |
| low | `register_telnet_library` is `_with_services`-only though never published as plain | None — additive, not a break | No action; recorded so the 70/71 distinction is not miscounted later |

## 11. Roadmap disposition

**Milestone blocked.** M007C's exit gate — "Eggsec has a published immutable registry artifact containing the M007 runtime hardening" — is not met, so:

- M007C is marked `blocked`, with the §3 gate result as the blocking evidence.
- `007-breaking-0-3-0-release.md` (M007C-R) is created and becomes the current handoff, per §3's prescribed remedy.
- **M007D remains blocked.** Its hard dependency is an accepted release closure; M007C did not close. M007D's own plan already carries the M007B `broker_dns_lookup` per-target finding, which is unaffected.
- M007E remains blocked on M007D. Automated NSE remains quarantined.
- No milestone is closed by this record, and no roadmap exit gate is claimed as met.

## 12. Registry updates

- `plans/registry.md`: M007C → blocked on the §3 major break; add M007C-R as the current NSE handoff; keep M007D blocked on M007C-R closure.
- `plans/subsystems/nse-runtime-extraction-roadmap.md`: M007C status → blocked with the gate evidence and the replan pointer; M007D/M007E statuses unchanged (still blocked); the milestone-table row updated to name `007c-closure.md`.
- `plans/implementation/nse-runtime-extraction/007-standalone-security-patch-release.md`: `Status: ready for handoff` → `Status: blocked`, with a deviation record naming the §3 failure.
