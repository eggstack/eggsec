# NSE Runtime Extraction Milestone 007B — Protocol Migration Corrective and Closure Pass

Status: ready for handoff

Original implementation plan:

- `plans/implementation/nse-runtime-extraction/007-broker-compatible-protocol-migration.md`

Original closure record:

- none yet; M007B has implementation on standalone `main` but has not reached an accepted closure record.

Eggsec planning baseline: `8e282e916fa44edc2bb800ba1dc480e8f0ec2815`

Standalone implementation baseline: `eggstack/eggsec-nse@699d374ad16806a9e0c6f265f297a644b8b88c29`

Parent M007A closure:

- `plans/closure/nse-runtime-extraction/007a-closure.md`

Applicable ADRs:

- `plans/adrs/ADR-0004-nse-automated-activation-boundary.md`
- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`

Primary class: corrective security + infrastructure

Implementation repository:

- `eggstack/eggsec-nse`

Planning/closure repository:

- `eggstack/eggsec`

## 1. Objective

Finish and truthfully close M007B after a large broker-migration implementation landed on standalone `main` without completing the plan's required residual-pin, effect-manifest, hosted-CI, and closure reconciliation.

This corrective pass must not treat the current red boundary guard as a bookkeeping-only problem.

It must:

1. audit every still-direct `BrokerCompatible*` entry;
2. finish migration where the current provider contract truly fits;
3. reclassify any incorrectly labeled entry with explicit source evidence rather than forcing it through the wrong abstraction;
4. promote only modules whose complete automated-relevant effect surface is provider-backed;
5. regenerate the direct-I/O residual pins from the new source truth;
6. update guards so provider/broker infrastructure is not mistaken for a specialized protocol residual;
7. obtain fully green standalone hosted CI;
8. produce the first accepted `007b-closure.md` with exact before/after counts;
9. unblock M007C only after those conditions are observed.

## 2. Why a corrective pass is required

M007B implementation commit `699d374ad16806a9e0c6f265f297a644b8b88c29` is already on `eggsec-nse/main`.

It added:

- `scripts/nse-migration-classes.txt`;
- `src/brokered_stream.rs`;
- broad service-aware rewiring across dozens of protocol libraries;
- `scripts/m007b_migrate.py`;
- brokered stream tests.

The committed migration classification covers all 97 M005E residual entries:

| Class | Count |
|---|---:|
| `BrokerCompatibleTcp` | 74 |
| `BrokerCompatibleUdpConnected` | 1 |
| `UnconnectedDatagram` | 17 |
| `NativeHandleEscape` | 3 |
| `RawPacketOrInterface` | 1 |
| `PublicCompatibilityApi` | 1 |
| **Total** | **97** |

The source scan after `699d374` shows a major reduction, but not complete M007B acceptance:

- old classified residual: 97 files;
- direct-socket scan after migration: 40 files including `src/brokered_stream.rs`;
- specialized residual after excluding broker/provider infrastructure: **39 files**.

Those 39 currently classify as:

| Class | Still-direct files |
|---|---:|
| `UnconnectedDatagram` | 17 |
| `BrokerCompatibleTcp` | 16 |
| `NativeHandleEscape` | 3 |
| `RawPacketOrInterface` | 1 |
| `BrokerCompatibleUdpConnected` | 1 |
| `PublicCompatibilityApi` | 1 |

Therefore **17 entries currently described as broker-compatible still have direct socket effects** and cannot be treated as complete without further audit.

The current effect manifest also remains at the M007A counts:

| Eligibility | Current count |
|---|---:|
| `ManualOnlyDirectIo` | 80 |
| `ManualOnlyAdvisory` | 26 |
| `Pure` | 35 |
| `ProviderBacked` | 18 |

That means the successfully migrated protocol cohort has not yet been promoted into the automated-safe eligibility set.

Finally, hosted CI run `36620331641` on exact standalone main `699d374` is red:

- MSRV: success;
- SSH runtime: success;
- Windows: success;
- Ubuntu: failure;
- macOS: failure.

Linux/macOS fail in `scripts/check-boundaries.sh` because the M005E direct-socket pins still describe the pre-M007B set:

```text
M005E violation: direct-socket file set changed
```

This guard failure is expected when migration legitimately removes direct effects, but the original M007B plan required pin + manifest reconciliation in the same tranche. That requirement was not completed.

## 3. Current status of the original M007B plan

The original plan file still says `Status: blocked`, even though:

- M007A is closed;
- roadmap/registry moved M007B to ready;
- implementation has already landed.

The original plan must be updated to a truthful historical state such as:

```text
Status: implemented; corrective closure required
```

with this corrective plan as the controlling handoff.

Do not mark the original plan `closed` until `007b-closure.md` exists and this corrective acceptance set is satisfied.

## 4. Corrective invariants

- Automated NSE remains quarantined in Eggsec.
- M007A effect gate remains fail-closed.
- Any library with an unresolved direct host-network effect remains automated-unavailable.
- A module is not promoted merely because its most common connect path is brokered.
- File-level promotion requires all automated-relevant network effects in that module to be provider-backed or explicitly unreachable from the automated registration surface.
- Do not weaken the direct-I/O source scan to make CI green.
- `src/brokered_stream.rs` is provider/broker infrastructure and must not be counted as a specialized protocol residual.
- Unconnected/broadcast/multicast UDP is not forced through the connected UDP provider abstraction.
- Native socket handoff to `ssh2::Session` remains manual-only unless a separate safe contract is designed.
- Raw packet/interface operations remain manual-only.
- `src/public_api/api.rs` remains manual/native in this milestone.
- No breaking public API is introduced.
- MSRV remains Rust 1.89.

## 5. Scope

### In scope

- Audit the 17 still-direct entries currently classified `BrokerCompatibleTcp` / `BrokerCompatibleUdpConnected`.
- Finish broker migration for every entry that actually fits the current provider contract.
- Reclassify entries that were over-broadly labeled, with exact rationale.
- Add a mixed/manual-only classification if necessary rather than lying with a broker-compatible label.
- Update `effect_manifest.rs` for fully migrated modules.
- Add the M007A-requested reverse manifest→registration consistency check or explicit compatibility-entry allowlist.
- Regenerate/repartition `scripts/nse-specialized-ungated.txt` and `scripts/nse-specialized-advisory.txt` from the final specialized residual.
- Update `scripts/check-boundaries.sh` so:
  - provider/broker infrastructure is excluded from specialized-protocol residual comparison;
  - residual pins match source;
  - no new direct-I/O specialized file can appear silently;
  - every residual has a migration class;
  - every promoted ProviderBacked residual candidate is absent from the direct-socket scan.
- Reconcile `docs/PROVIDERS.md`.
- Run full standalone qualification and hosted CI.
- Write `plans/closure/nse-runtime-extraction/007b-closure.md` in Eggsec.
- Reconcile roadmap/registry and unblock M007C only after closure.

### Explicitly out of scope

- New async provider hierarchy.
- Unconnected/broadcast/multicast UDP provider redesign.
- Raw packet provider design.
- Native socket escape through provider handles.
- `public_api` redesign.
- Publishing 0.2.1/0.3.0.
- Eggsec dependency adoption or approved-scope threading.
- Automated NSE re-exposure.

## 6. Mandatory audit of the 17 unresolved broker-compatible entries

At corrective-plan authoring time the source scan still reports direct sockets in:

### Currently classified `BrokerCompatibleTcp`

- `src/libraries/ftp.rs`
- `src/libraries/imap.rs`
- `src/libraries/mongodb.rs`
- `src/libraries/mssql.rs`
- `src/libraries/mysql.rs`
- `src/libraries/openssl.rs`
- `src/libraries/postgres.rs`
- `src/libraries/rdp.rs`
- `src/libraries/redis.rs`
- `src/libraries/sip.rs`
- `src/libraries/smb.rs`
- `src/libraries/smtp.rs`
- `src/libraries/sslcert.rs`
- `src/libraries/telnet.rs`
- `src/libraries/tls.rs`
- `src/libraries/vnc.rs`

### Currently classified `BrokerCompatibleUdpConnected`

- `src/libraries/radius.rs`

For each file, produce a small audit record:

```text
path
direct effect sites
entry/global that reaches each site
manual-only vs automated-reachable
provider-contract compatibility
final class
migration action
effect-manifest eligibility
```

Allowed outcomes:

### Outcome A — Complete migration

All automated-relevant direct effects fit current broker/provider semantics and are migrated.

Then:

- no specialized direct-socket match remains in the file;
- module may become `ProviderBacked` if no other unsafe effect exists.

### Outcome B — Correct reclassification

A direct effect does not fit the current provider contract or belongs to a mixed manual-only path that cannot be safely separated.

Then:

- file remains manual-only;
- class is changed to an accurate manual/deferred class;
- rationale is pinned in `nse-migration-classes.txt`;
- effect manifest remains manual-only;
- no fake migration is performed.

A file may not retain a `BrokerCompatible*` classification while still containing unexplained automated-relevant direct socket effects at closure.

## 7. Residual source-of-truth redesign

M005E pins were intentionally immutable until a qualified migration changed the source truth. M007B is that migration.

The corrected guard model should distinguish:

### Provider/broker infrastructure

Examples:

- `src/providers.rs`;
- `src/brokered_stream.rs`;
- future explicitly named provider infrastructure.

These may contain native/provider mechanics and are governed by provider-zone guards, not specialized-library residual pins.

### Specialized compatibility residual

Only direct host socket effects in:

- `src/libraries/**`;
- `src/public_api/api.rs` where explicitly included.

This is the set pinned by the M007B-updated residual inventories.

Do not simply exclude arbitrary files until the set matches. Every exclusion must correspond to an accepted provider/native zone.

## 8. Effect-manifest promotion rules

For each module migrated by M007B:

Promote to `ProviderBacked` only if:

1. every automated-relevant network effect is broker/provider-backed;
2. no raw/unconnected/native-handle effect remains reachable from that Lua module under automated profiles;
3. direct-global and dynamic-`require` M007A gates remain correct;
4. denial/cancellation means zero unapproved native contact;
5. byte/op accounting is broker-owned.

If a module contains both brokered and unsafe effects that cannot be cleanly separated, keep the whole module manual-only.

The closure must report manifest counts before and after this corrective pass.

## 9. Reverse manifest-registration consistency

M007A closure recorded a low-severity gap: registration→manifest coverage is checked, but manifest→registration has 13 extra compatibility entries.

Resolve it in this corrective pass by either:

- a bidirectional generated comparison plus a small explicit compatibility-entry allowlist; or
- an equivalent mechanically enforced one-to-one mapping model.

Do not delete legitimate compatibility aliases just to make counts equal.

Acceptance:

- every registered library has exactly one security classification;
- every manifest-only entry has an explicit reviewed rationale;
- stale/orphaned entries fail CI unless allowlisted intentionally.

## 10. Ordered work packages

### A — Reproduce and freeze current residual

Record:

- implementation SHA `699d374`;
- CI run `36620331641`;
- current specialized direct-I/O residual = 39;
- current unresolved broker-compatible residual = 17;
- current manifest counts = 80 DirectIo / 26 Advisory / 35 Pure / 18 ProviderBacked.

### B — Audit the 17 unresolved broker-compatible files

Produce final class + action for each.

Acceptance:

- none remain ambiguously `BrokerCompatible*` with unexplained direct effects.

### C — Finish broker-compatible migration

Implement Outcome A files using existing broker/provider helpers.

Acceptance:

- direct socket scan is clean for each promoted file;
- focused protocol tests pass;
- no native-handle escape added.

### D — Correct over-broad classifications

Implement Outcome B entries.

Acceptance:

- rationale is source-specific;
- manual-only eligibility remains fail-closed.

### E — Reconcile the effect manifest

Promote fully safe modules and add reverse consistency enforcement.

Acceptance:

- counts change only from evidenced promotions;
- all 159 registered/compat entries remain classified.

### F — Regenerate residual pins and guards

Update the 72/25 historical pins to the new qualified residual source truth.

Preserve historical M005E counts in docs/closure; current pins describe current source, not historical source.

Acceptance:

- specialized residual scan equals pins;
- provider infrastructure is not counted as a specialized residual;
- every current pin has a migration-class entry.

### G — Full qualification

Run local/focused/full suite plus hosted CI.

Required hosted jobs:

- Ubuntu: success;
- macOS: success;
- Windows: success;
- MSRV: success;
- SSH runtime: success.

### H — Closure and planning reconciliation

Create:

- `plans/closure/nse-runtime-extraction/007b-closure.md`.

Then update:

- original M007B plan status;
- roadmap;
- registry.

Only then move M007C to ready for handoff.

## 11. Required tests

### Focused migration tests

For every promoted cohort:

- loopback success;
- AgentSafe allowed success where applicable;
- out-of-scope denial before host contact;
- CiSafe denial;
- cancellation before connect/send/receive;
- read/write accounting.

### Manifest tests

- promoted module is automated-safe;
- remaining direct/advisory module is automated-denied;
- direct-global and require paths agree;
- reverse manifest-registration consistency.

### Source guards

- no promoted module contains specialized direct socket effects;
- no unclassified specialized direct socket file exists;
- no provider infrastructure accidentally enters residual pins.

### Regression

Run all existing runtime/corpus/local protocol/provider tests.

## 12. Required verification

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
cargo package --list
cargo package
```

Explicitly run:

- `effect_manifest_tests`;
- `brokered_stream_tests`;
- local protocol tests touching every promoted family.

## 13. Acceptance criteria

1. All 97 original M005E residual entries retain exactly one final migration classification.
2. All 17 currently still-direct `BrokerCompatible*` entries are either fully migrated or accurately reclassified.
3. No closure-time `BrokerCompatible*` entry retains unexplained automated-relevant direct socket effects.
4. Every promoted module is `ProviderBacked` in the effect manifest.
5. Every remaining direct/advisory module remains automated-denied.
6. Reverse manifest→registration consistency is enforced with explicit compat allowlisting where required.
7. Specialized direct-I/O residual pins match current source truth.
8. Provider/broker infrastructure is not misclassified as protocol residual.
9. Current residual is materially below the M005E 97-entry baseline; exact before/after count is recorded.
10. Boundary guards are green.
11. Full standalone suite/MSRV/package qualification is green.
12. Hosted Ubuntu/macOS/Windows/MSRV/SSH CI is fully green on exact closure SHA.
13. `007b-closure.md` records exact classification, residual, and manifest-count deltas.
14. M007C remains blocked until criteria 1-13 pass.
15. Automated Eggsec NSE remains quarantined.

## 14. Stop conditions

Stop and report if:

- a supposedly broker-compatible file requires async provider redesign;
- a direct effect requires unconnected/broadcast/multicast UDP not supported by the current provider;
- a native library requires concrete socket ownership;
- a module can only be promoted by ignoring a direct effect;
- making the guards green requires weakening the source scan;
- a breaking public API is required;
- any automated Eggsec exposure changes during this pass.

## 15. Closure evidence required

`007b-closure.md` must include:

- original plan + this corrective plan;
- implementation baseline `699d374`;
- failed hosted run `36620331641`;
- corrective implementation SHA(s);
- final standalone main SHA;
- 97-entry final migration-class counts;
- 17-file audit outcomes;
- before/after specialized direct-I/O residual counts;
- before/after effect-manifest eligibility counts;
- promoted module list;
- reclassified/deferred module list + rationale;
- residual pin diff;
- reverse manifest-registration guard evidence;
- focused/full/MSRV/package test results;
- hosted CI run ID/job outcomes;
- unresolved findings by severity;
- explicit GO/NO-GO for M007C.

## 16. Handoff notes

The current implementation already accomplished a large part of M007B. Preserve that work.

Do not "fix" CI by copying the current 39-file source scan into the old pin files before auditing the 17 entries still labeled broker-compatible. The pin reconciliation is the final step after migration/classification truth is established.

Do not promote a module based on line-count reduction alone. Automated eligibility is an authority property, not a code-cleanliness metric.
