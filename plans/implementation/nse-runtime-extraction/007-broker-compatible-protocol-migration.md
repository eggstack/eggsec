# NSE Runtime Extraction Milestone 007B — Broker-Compatible Protocol Migration and Residual Hardening

Status: blocked

Planning baseline: `386fe63a522aac66340386ac83e9f5ed54500d8b`

Standalone baseline: `eggstack/eggsec-nse@ff0d2c09feba1bcd7ba5f7312579690d905be09c`

Hard dependency:

- accepted closure of `plans/implementation/nse-runtime-extraction/007-automated-library-effect-gate.md`.

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-007--protocol-library-gating-and-controlled-automated-activation`

Applicable ADRs:

- `plans/adrs/ADR-0004-nse-automated-activation-boundary.md`
- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`

Primary class: security + infrastructure

Implementation repository:

- `eggstack/eggsec-nse`

## 1. Objective

Shrink the M005E specialized direct-I/O residual by migrating the mechanically compatible protocol cohort onto the existing capability-aware DNS/TCP/UDP provider broker, while leaving genuinely incompatible native-handle/broadcast/raw/async cases explicitly manual-only.

The goal is not to rewrite all 72 residual files in one milestone. The goal is to turn residual migration into a repeatable, auditable promotion process and migrate every file that fits the current provider contract without adding a second transport architecture.

## 2. Research finding

The residual is heterogeneous.

Representative direct-I/O patterns include:

- simple blocking TCP connect/read/write (`bitcoin`, `cvs`, `dicom`, `proxy`, and many others);
- async Tokio TCP paths (`afp`, `ajp`, `sip`, `websocket`, `winrm`);
- unconnected/broadcast UDP `send_to/recv_from` (`bjnp`, `packet`, `wsdd`, `tftp`, `stun`, `natpmp`);
- native-socket handoff or protocol-library integration (`libssh2_utility`, SSH-family paths);
- mixed files where some entries already consult capability policy.

The current provider contract is strongest for:

- DNS resolution + concrete endpoint selection;
- connected TCP handles;
- connected UDP handles;
- brokered send/receive/accounting.

Therefore the first migration cohort should be selected by effect shape, not by protocol popularity.

## 3. Cohort classification

Before editing production files, produce a machine-readable migration classification for every M005E residual entry.

Minimum classes:

- `BrokerCompatibleTcp`: blocking connect-style TCP; no native handle escape;
- `BrokerCompatibleUdpConnected`: connected UDP semantics fit current provider handle;
- `AsyncDirectIo`: Tokio/native async operations require a separate bounded bridge;
- `UnconnectedDatagram`: send_to/recv_from/broadcast/multicast semantics not represented by current connected UDP provider;
- `NativeHandleEscape`: external library requires concrete `std::net::TcpStream` or equivalent;
- `RawPacketOrInterface`: operation is outside the current TCP/UDP provider contract;
- `PublicCompatibilityApi`: `src/public_api/api.rs`, kept manual/native unless separately redesigned.

Unknown classification is a stop condition.

## 4. Compatibility stream adapter

For the blocking TCP cohort, introduce an internal compatibility adapter over:

- `NseCapabilityContext`;
- `NseHostServices`;
- `Box<dyn NseTcpConnection>`;
- existing `broker_tcp_connect/send/receive`.

It may implement `std::io::Read`/`Write` internally if that materially reduces protocol churn, but it must not expose the native socket.

Required semantics:

- connect uses broker resolve-select-connect;
- writes use send accounting/write limits;
- reads use receive accounting/read limits;
- timeouts delegate to provider handles;
- cancellation/capability denial is preserved;
- errors map predictably to existing Lua/protocol failures.

A similar internal connected-UDP helper may be added only for files whose semantics fit `NseUdpSocket`.

Do not introduce an async provider hierarchy merely to improve the migration count.

## 5. Migration rule

A file may be promoted from manual-only only when **all automated-relevant network effects in that file** are provider-backed.

Merely adding one `check_network_*` call is insufficient.

For each promoted file:

- registration function receives capability context/services as needed;
- all direct connect/send/receive sites used by automated-visible entries are brokered;
- deny/cancel tests prove zero host/provider contact;
- byte/op accounting is asserted where practical;
- effect manifest moves the module to `ProviderBacked`;
- M005E residual pin is updated in the same commit.

Mixed files remain manual-only until every automated-relevant direct effect is dispositioned.

## 6. Scope

### In scope

- complete effect-shape classification of 72 ungated + 25 advisory files;
- internal brokered TCP I/O compatibility adapter;
- connected-UDP adapter if the classification identifies a meaningful cohort;
- migrate all files classified as broker-compatible under the existing contract;
- per-entry audit of advisory files touched by the migration;
- update effect manifest and residual pins;
- deterministic tests for promoted libraries;
- documentation of remaining incompatible classes.

### Explicitly out of scope

- async provider-trait redesign;
- unconnected/broadcast/multicast UDP provider redesign;
- raw packet/dnet/pcap provider work;
- concrete native socket escape from provider handles;
- breaking public API changes;
- Eggsec scope/profile threading;
- automated surface activation;
- `public_api/api.rs` providerization.

## 7. Ordered work packages

### A — Freeze the residual classification

Commit a generated/auditable classification file for all 97 M005E residual entries.

Acceptance: every residual appears exactly once.

### B — Add brokered stream compatibility helper

Acceptance:

- local TCP fixture can connect/read/write through the helper;
- denial/cancellation means zero native provider contact;
- accounting uses correct read/write buckets.

### C — Migrate blocking TCP cohort

Migrate all `BrokerCompatibleTcp` files.

Acceptance:

- no direct `TcpStream::connect*` remains in promoted files;
- effect manifest marks them provider-backed;
- compatibility tests pass.

### D — Migrate connected UDP cohort

Only if semantics fit current `NseUdpSocket`.

Acceptance:

- promoted files contain no direct bind/send/receive path;
- unconnected/broadcast cases remain explicitly manual-only.

### E — Harden touched advisory files

Perform per-entry rather than file-level review.

Acceptance:

- a file is promoted only when every automated-relevant effect is brokered;
- otherwise it remains manual-only with exact rationale.

### F — Residual reconciliation

Regenerate:

- specialized ungated pin;
- specialized advisory pin;
- effect manifest;
- migration-class inventory.

Acceptance:

- total direct-I/O residual count measurably decreases;
- no new direct-I/O file appears;
- every remaining item has a non-providerizable/deferred class.

## 8. Invariants

- M007A automated gate remains in force throughout migration.
- Direct residual files remain unavailable under AgentSafe/CiSafe until promoted.
- No file is promoted from a file-level superficial check.
- Native provider zone remains centralized.
- Manual behavior remains compatible.
- No blocking direct socket call is introduced into async runtime code.
- No detached background task is introduced.
- MSRV 1.89 preserved.

## 9. Required tests

For each migrated cohort:

- representative success against loopback fixture;
- AgentSafe in-scope success;
- AgentSafe out-of-scope/denied zero contact;
- CiSafe zero contact;
- cancellation before connect/send/receive;
- read/write accounting;
- concurrent runs with distinct providers where applicable.

Run the full runtime/corpus/local protocol suite.

## 10. Verification

```bash
cargo fmt --all --check
./scripts/check-boundaries.sh
cargo check --features nse
cargo test --features nse
cargo check --features nse-ssh2
cargo check --features nse,sandbox
cargo clippy --all-targets --features nse
cargo +1.89.0 check --locked --features nse
cargo package
```

## 11. Acceptance criteria

1. All 72+25 residual entries have an effect-shape classification.
2. Broker-compatible blocking TCP cohort is fully migrated.
3. Connected UDP cohort is migrated where current provider semantics are sufficient.
4. No promoted file retains automated-relevant direct network effects.
5. Promoted modules move to ProviderBacked eligibility.
6. Remaining modules stay automated-denied.
7. Residual pin count decreases from the M005E baseline.
8. No async/raw/native-handle workaround weakens ADR-0004.
9. Full standalone verification passes.
10. Closure records exact before/after residual counts and unhandled classes.
11. Closure unblocks M007C.

## 12. Stop conditions

Stop if:

- a migration requires exposing native sockets through the provider API;
- async migration would require blocking an executor thread without a bounded design;
- unconnected/broadcast semantics are forced into the connected UDP abstraction;
- protocol behavior changes materially;
- a breaking API is required.

## 13. Closure evidence

Record:

- classification artifact;
- broker-compatible cohort list;
- migrated file list;
- before/after residual counts;
- effect-manifest promotions;
- direct-I/O source scans;
- denial/cancel/accounting results;
- full suite/MSRV/package results;
- deferred class list;
- GO/NO-GO for M007C.
