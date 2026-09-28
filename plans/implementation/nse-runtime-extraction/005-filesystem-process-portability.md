# NSE Runtime Extraction Milestone 005D — Filesystem, Process, and Cross-Platform Host Portability

Status: implemented

Closure: `plans/closure/nse-runtime-extraction/005d-closure.md` (closed).

Unblocked by accepted 005A closure (`plans/closure/nse-runtime-extraction/005a-closure.md`); may proceed in parallel with 005B.

Eggsec planning baseline: `b5d27348a829a4c1c2a49fbc267349c52e86a1f6`

Standalone runtime baseline: `eggstack/eggsec-nse@854f153f56d1abc929d9abd255f0606759342f81`

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-005--host-provider-inversion-and-portability-hardening`

Hard dependency:

- accepted closure of `plans/implementation/nse-runtime-extraction/005-provider-broker-foundation.md`.

Long-term requirements:

- `plans/000-long-term-specification.md#2-primary-product-goals`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/001-terminology-and-domain-model.md#3-execution-terms`
- `plans/002-long-term-roadmap.md#phase-7--standing-maintenance-and-future-capability-open`

Applicable ADRs:

- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`

Primary class: infrastructure

Affected repositories:

- implementation target: `eggstack/eggsec-nse`;
- consumer verification/planning: `eggstack/eggsec`.

## 1. Objective

Move the shared/core filesystem and process side effects behind narrow providers, remove process-global working-directory semantics from NSE execution, localize platform-specific host mechanics, and add Windows qualification without breaking current NSE APIs or default behavior.

This slice must improve portability and per-run isolation; it is not a license to rewrite every protocol-specific helper or the broad Rust convenience API.

## 2. Why this milestone is blocked

The filesystem/process provider contracts must reuse the service bundle/broker sequencing proven by 005A.

Current evidence justifies the work once that contract is stable:

- `io.rs`, `lfs.rs`, `os.rs`, and `nmap.rs` directly invoke filesystem/process/environment/CWD operations after capability checks;
- `lfs.chdir` and `os.chdir` call `std::env::set_current_dir`, changing process-global CWD across concurrent runs;
- runtime code contains Unix-specific `PermissionsExt` and symlink operations;
- `nmap.rs` shells out to `id`, `ip`, or `ipconfig`;
- standalone CI currently qualifies Linux/macOS but not Windows;
- current provider wrappers leak `std::fs::Metadata`, `DirEntry`, and `std::process::Output`, which are unsuitable durable provider types.

## 3. Current implementation evidence

Filesystem compatibility code currently mixes three responsibilities:

1. runtime capability/sandbox checks;
2. Lua/NSE compatibility formatting;
3. direct platform filesystem mechanics.

Process/platform code similarly mixes privilege/interface discovery with command execution.

This makes deterministic testing difficult and spreads cfg-specific host behavior through compatibility libraries.

## 4. Invariants that must not regress

- Filesystem/process policy remains in `NseCapabilityContext`, not providers.
- Sandbox canonical-path/allowed-root semantics remain fail-closed.
- Provider boundaries expose runtime-owned DTOs/opaque handles, not `std::fs`/`std::process` types.
- Existing Lua file/process result shapes remain compatible.
- AgentSafe/CiSafe filesystem-write/process-exec denials remain at least as strict.
- Per-run CWD must not mutate process-global CWD after migration.
- Native default behavior remains available.
- Windows support must not weaken Unix containment semantics.
- No breaking cleanup of `public_api`.
- No Eggsec dependency enters standalone.

## 5. Scope

### In scope

- Add filesystem and process providers to the 005A service bundle.
- Define runtime-owned file metadata, directory entry, process result, and opaque file-handle types as required.
- Add native platform implementations.
- Migrate common/shared filesystem paths in:
  - `io.rs`;
  - `lfs.rs`;
  - filesystem portions of `os.rs`;
  - default script/module path mechanics where appropriate.
- Migrate common process/platform paths in:
  - `io.popen`;
  - privilege/interface discovery in `nmap.rs`;
  - other directly equivalent shared process helpers.
- Replace process-global `current_dir/set_current_dir` behavior with per-run virtual/current working-directory state used for path resolution.
- Localize Unix/Windows symlink/permission/process differences in native-provider/platform modules.
- Add Windows CI build/check coverage and targeted runtime tests that are platform-safe.
- Add static guards for new direct filesystem/process bypasses in migrated modules.
- Inventory intentionally remaining direct host operations.

### Explicitly out of scope

- Network/DNS/HTTP provider work.
- Redesign of NSE sandbox policy.
- Full virtual filesystem implementation.
- Arbitrary shell emulation.
- Providerizing every protocol-specific file/process helper if not shared/core.
- Breaking public Rust convenience API signatures.
- BSD-specific qualification beyond keeping native-provider design extensible.
- Removing existing OS dependencies solely for aesthetics.

## 6. Required production changes

### Core/domain

Define narrow filesystem/process contracts. Runtime-owned metadata must capture only fields NSE compatibility actually consumes, rather than mirroring every `std::fs::Metadata` property.

Opaque file handles should support the required read/write/seek/flush/close behavior without exposing `std::fs::File`.

Process result DTOs should include exit status, stdout, stderr, and necessary error context without exposing `std::process::Output`.

### Storage and migrations

No persistent storage migration.

### Protocol and DTOs

Lua-visible `io`, `lfs`, `os`, and `nmap` compatibility shapes remain stable.

### Runtime and concurrency

Introduce per-run CWD state in the host-services/filesystem layer. Relative path resolution must use that state and must not call `set_current_dir` on the embedding process.

Concurrent runs with different virtual CWDs must not interfere.

File-handle state should remain per-run or explicitly keyed to the owning run; current process-global handle maps should be reduced where practical.

### Frontend or operator surface

No user-visible configuration requirement.

### Security and authorization

Broker sequencing must ensure:

```text
capability/sandbox path decision
-> cancellation/resource preflight
-> provider operation
-> accounting/event result
```

Path checks must apply to the actual resolved provider path. Do not reintroduce a TOCTOU-prone second path transformation after approval if the provider can operate on a runtime-owned approved path token/identity.

Process exec remains denied in AgentSafe/CiSafe before provider invocation.

### Documentation and static guards

Document platform semantics, virtual CWD behavior, and any unsupported Windows compatibility details explicitly.

## 7. Ordered work packages

### Work package A — Define filesystem/process DTOs and provider contracts

Acceptance: no `std::fs`/`std::process` implementation types leak through public provider interfaces.

### Work package B — Add native providers and per-run CWD

Implement platform modules/native defaults and virtual working-directory state.

Acceptance: two concurrent runs can resolve relative paths under different CWDs without process-global mutation.

### Work package C — Migrate `io.rs` and `lfs.rs`

Route open/read/write/metadata/dir/link/permission/chdir operations through broker/providers.

Acceptance: current sandbox and local filesystem tests pass; migrated modules do not call direct filesystem APIs outside explicitly justified compatibility glue.

### Work package D — Migrate shared `os.rs`/process paths

Move remove/rename/process-exec and environment-adjacent file mechanics to providers where covered; preserve safe stubs.

Acceptance: process denial happens before provider invocation and manual behavior remains compatible.

### Work package E — Migrate privilege/interface discovery

Replace scattered `id`/`ip`/`ipconfig` command execution with provider/platform methods or explicitly platform-scoped native implementation.

Acceptance: compatibility library no longer owns shell-command parsing where a native provider owns it.

### Work package F — Add Windows qualification and guards

Add Windows to CI at least for compile/check of relevant features, plus tests that do not require Unix-only services.

Acceptance: `cargo check --features nse` and intended provider tests pass on Windows; Unix-only APIs are cfg-localized.

### Work package G — Residual host-operation inventory

Classify remaining direct filesystem/process/CWD/platform operations and block new untracked bypasses.

## 8. Failure, cancellation, restart, and contention semantics

Provider failures map to existing Lua error/result behavior.

Cancellation before blocking filesystem/process operations prevents invocation. Process execution must remain bounded where current APIs support timeouts; do not introduce detached child processes.

Per-run virtual CWD is ephemeral and resets with the run.

Concurrent file handles/CWD state must not leak between runs.

## 9. Compatibility and migration

Existing callers keep native defaults.

Lua semantics should remain compatible except that CWD becomes correctly run-local rather than process-global. This is an intentional isolation fix and must be documented/tested.

Existing public Rust convenience functions remain native compatibility shims unless additive provider-aware variants are clearly justified.

## 10. Required tests

### Focused unit tests

- metadata/process DTO mapping;
- path resolution under virtual CWD;
- platform permission/link helpers;
- provider denial/not-called.

### Integration tests

- `io` read/write/append;
- `lfs` attributes/dir/link/touch/chdir;
- manual process path where safe;
- profile denials.

### Restart and recovery tests

Run-local CWD/handle state disappears between executions.

### Contention and cancellation tests

- concurrent different CWDs;
- concurrent independent file handles;
- cancellation before process/filesystem calls.

### Security and negative tests

- sandbox escape/symlink tests remain green;
- AgentSafe/CiSafe write/process denial before provider call;
- no direct migrated-module host bypass;
- no process-global `set_current_dir`.

### Migration and compatibility tests

- existing runtime corpus;
- package/MSRV;
- Windows build/check;
- Eggsec consumer smoke.

## 11. Required verification commands

Standalone Linux/macOS:

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

Windows CI minimum:

```text
cargo check --no-default-features
cargo check --features nse
cargo check --features nse,sandbox
```

Run targeted provider/CWD tests on Windows where behavior is supported.

Eggsec:

```bash
cargo check -p eggsec --features nse,cli
cargo test -p eggsec --features nse,cli --test nse_tests --test nse_integration_tests
make check
```

## 12. Documentation updates

Standalone:

- provider architecture;
- platform support matrix;
- virtual CWD semantics;
- compatibility limitations;
- remaining host-operation inventory.

Eggsec:

- planning/closure references unless consumer behavior changes.

## 13. Acceptance criteria

1. Filesystem/process provider contracts are narrow and runtime-neutral.
2. Native implementations preserve existing behavior where platform-supported.
3. Runtime provider APIs do not expose `std::fs`/`std::process` types.
4. `io.rs`/`lfs.rs` common paths use brokered providers.
5. Process-global CWD mutation is removed from NSE execution.
6. Concurrent runs with different CWDs are isolated.
7. AgentSafe/CiSafe denial occurs before filesystem-write/process provider calls.
8. Platform-specific Unix/Windows behavior is localized.
9. Windows `nse` compilation/check is part of CI and green.
10. Existing corpus/sandbox behavior remains compatible.
11. Remaining direct host operations are explicitly inventoried and guarded.
12. Closure recommends readiness for 005E qualification.

## 14. Stop conditions

Stop if:

- filesystem providerization would require weakening canonical-path/sandbox checks;
- Windows parity requires silently changing NSE semantics rather than documenting an unsupported operation;
- public API breakage becomes necessary;
- the provider contract becomes a full virtual OS/monolithic host trait;
- run-local CWD cannot be implemented without process-global mutation;
- 005A closure is absent.

## 15. Closure evidence required

- provider API/DTO inventory;
- migrated filesystem/process path list;
- virtual CWD concurrency tests;
- denial/provider-not-called tests;
- sandbox/symlink regression results;
- Windows CI result;
- direct host-operation residual inventory;
- corpus/MSRV/package results;
- Eggsec consumer verification;
- GO/NO-GO for 005E.

## 16. Handoff notes

Do not chase perfect cross-platform emulation. The goal is to put platform mechanics behind a stable native-provider boundary, make unsupported behavior explicit, and stop compatibility libraries from mutating global process state.
