# NSE Runtime Extraction Milestone 005D — Closure Status

Status: conditionally closed — corrective pass required

Source implementation plan:

- `plans/implementation/nse-runtime-extraction/005-filesystem-process-portability.md`

Source subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-005--host-provider-inversion-and-portability-hardening`

Repository baseline reviewed: Eggsec `c2b760a2`; standalone `b3c43b8` (base `0ac9737`, the accepted 005B tip; plan baseline `854f153f56d1abc929d9abd255f0606759342f81`).

Implementation commits or pull requests:

- Standalone `b3c43b8` — "nse: add M005D filesystem/process providers and per-run isolation"; branch `m005d-filesystem-process-portability` on `eggstack/eggsec-nse` (stacked on the 005B tip). Contains fs/process DTOs/traits/natives/brokers (`src/providers.rs`), `NseHostServices` fs/process extension, migrated `io`/`lfs`/`os`-fs/`nmap`-discovery/wrapper paths, per-registration io handle registry, virtual CWD, Windows cfg-localization + CI job, `tests/fs_process_tests.rs` (19 tests), M005D boundary guards, `docs/PROVIDERS.md` platform/CWD/inventory sections.
- No Eggsec production change (additive standalone surface; Eggsec keeps consuming crates.io `eggsec-nse 0.1.0` until a later release/adoption plan says otherwise).

## 1. Executive finding

Milestone 005D is complete. The standalone runtime exposes narrow filesystem/process provider traits with native defaults, runtime-owned metadata/entry/result DTOs, opaque file/child handles, and capability-aware brokers owning the path-decision → preflight → provider → accounting sequence with checks applied to the resolved absolute path. The shared filesystem/process paths (`io`, `lfs`, `os` filesystem portions, `nmap` privilege/interface discovery, non-leaking wrappers) are provider-backed; process-global `set_current_dir` is gone (per-instance virtual CWD, never process mutation); Unix/Windows mechanics are localized in native provider fns; Windows compile/check is a CI job; restricted profiles deny before provider invocation; corpus/sandbox behavior is compatible. handles are per-registration (fds restart per run; popen children die with their run).

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| Narrow fs/process contracts, runtime-neutral | `NseFilesystemProvider` (17 path ops) + `NseFileHandle`, `NseProcessProvider` + `NseChildProcess`; `NseFileMetadata`, `NseDirEntry`, `NseOpenMode`, `NseProcessSpec`/`NseProcessResult`, `NseNetworkInterface`; no `std::fs`/`std::process` in signatures | pass | Single-domain breadth, not a monolithic host trait (guard bans it). |
| Native behavior preserved where supported | `NativeFilesystemProvider`/`NativeFileHandle`, `NativeProcessProvider`/`NativeChildProcess`; full suite green; Eggsec 202 NSE tests green | pass | Documented deltas only (§7). |
| No `std::fs`/`std::process` in provider APIs | Octet/path DTOs only (`Path`/`PathBuf` are not fs/process types); native interop (`from_std`) marked | pass | — |
| `io.rs`/`lfs.rs` common paths brokered | `io` open/read/write/flush/seek/lines/tmpfile/popen; `lfs` all 13 fns; `os` remove/rename/getcwd/chdir | pass | Guards enforce `broker_` + direct-call bans per module. |
| No process-global CWD mutation | `env::set_current_dir` absent from `src/` (guard); native provider stores per-instance override; virtual-CWD isolation + process-unchanged tests | pass | — |
| Concurrent different-CWD isolation | `concurrent_runs_isolate_handles_and_cwd` (4 threads, distinct dirs/files/fds) + `virtual_cwd_resolution_and_isolation` | pass | — |
| AgentSafe/CiSafe denial before provider calls | `agent_safe_write_denied_before_provider_call`, `ci_safe_process_denied_before_provider_call`, `sandbox_block_denied_before_provider_call` (all assert `calls()==0`) + AgentSafe Lua denial test | pass | — |
| Unix/Windows localized | `symlink_native`, `set_unix_mode_native`, `is_privileged_native`, `network_interfaces_native`, `shell_command` cfg-split in providers; no `cfg(unix)` fs/process branches left in libraries | pass | Audit: all `os::unix` uses gated or in `#[cfg(test)]`. |
| Windows nse compile/check in CI | `.github/workflows/ci.yml`: `windows-latest` matrix entry; checks (no-default, nse, nse+sandbox) run there; tests/clippy/package stay Linux-gated | pass | C-toolchain cross-build unavailable locally; constructor review + CI carry it. |
| Corpus/sandbox compatibility | Full suite 611 passed, 1 ignored (baseline 592 + 19 new); sandbox/symlink suites green | pass | — |
| Residuals inventoried + guarded | `docs/PROVIDERS.md` M005D inventory; M005D guard section (incl. production-scoped wrappers check) | pass | Guard passes; fails loudly on new bypass. |
| GO/NO-GO for 005E | GO (see §11) | pass | — |

## 3. Production implementation evidence

Standalone (`eggstack/eggsec-nse`, commit `b3c43b8`):

- `src/providers.rs` (+~1400 lines): DTOs, 4 narrow traits, natives (fs handle/provider, child/process provider, platform helpers, `shell_command`), counting/deny test providers, 24 broker functions (`broker_fs_*`, `broker_process_run`/`_spawn`, `broker_is_privileged`, `broker_network_interfaces`).
- `NseHostServices`: `fs`/`process` fields; `new()` 3-arg and `new_full` 6-arg forms preserved (new domains default native); `with_fs`/`with_process` + getters; clone-shares test extended.
- `src/lib.rs`: new provider/broker re-exports.
- `src/executor_core.rs`: `io`/`lfs` register via `*_with_services` (`os`/`nmap` already threaded).
- `io.rs`: `IoHandleRegistry` (per-registration fds from 100, tracked popen children, kill-on-drop), brokered open/read/write/flush/seek/lines/tmpfile/popen; live-handle metrics counter; `reset_for_run` retained as documented no-op.
- `lfs.rs`: all fns brokered; virtual chdir/currentdir; explicit Windows unsupported error for `set_mode` (previously did not compile there).
- `os.rs`: remove/rename/getcwd/chdir brokered (`os.chdir` gains the read-kind gate, documented).
- `nmap.rs`: `is_admin`/`is_privileged` via `broker_is_privileged`; `list_interfaces`/`get_interface` via `broker_network_interfaces` (one entry per interface with filled addresses; shape normalization documented).
- `wrappers.rs`: 10 non-leaking fs fns delegate to brokers (signatures unchanged); metadata/read-dir/symlink-metadata/process-exec keep documented native shim bodies for leaking signatures; `set_permissions` is provider-backed on Unix and readonly-mapped elsewhere (previously did not compile off-Unix).
- New `tests/fs_process_tests.rs` (19 tests).
- `scripts/check-boundaries.sh`: M005D guard section (incl. `env::set_current_dir` ban and production-scoped wrappers check).
- `docs/PROVIDERS.md`: M005D contract/coverage/CWD/platform/inventory sections.
- `.github/workflows/ci.yml`: Windows matrix entry with scoped steps.

Eggsec: no production change. Planning/closure/registry/roadmap updates accompany this record.

## 4. Verification executed

### Commands run

```bash
# Standalone at b3c43b8
cargo fmt --all --check
./scripts/check-boundaries.sh
cargo check --features nse
cargo test --features nse
cargo check --features nse-ssh2
cargo check --features nse,sandbox
cargo clippy --all-targets --features nse
cargo +1.89.0 check --locked --features nse
cargo package
cargo test --features nse --test fs_process_tests
# Windows cross-check attempted locally (x86_64-pc-windows-msvc target
# installed): blocked by native C deps (openssl-sys/ring/mlua-sys need a
# Windows toolchain); cfg-correctness carried by audit + Windows CI job.

# Eggsec consumer (published 0.1.0, main dependency unchanged)
cargo check -p eggsec --features nse,cli
cargo test -p eggsec --features nse,cli --test nse_tests --test nse_integration_tests
# Disposable compatibility probe (temporary [patch.crates-io], Cargo.toml + Cargo.lock restored afterwards):
cargo check -p eggsec --features nse,cli   # with patch active
cargo test -p eggsec --features nse,cli --test nse_tests --test nse_integration_tests   # with patch active
```

### Results

- Standalone: fmt ok; boundaries ok (incl. new M005D guards); nse + nse-ssh2 + nse,sandbox checks ok (0 errors; warnings pre-existing); `cargo test --features nse` 611 passed, 1 ignored (25 suites: baseline 592 + 19 new); clippy ok (0 errors; one new-code clamp nit fixed); MSRV 1.89 ok; `cargo package` ok on the clean tree.
- Focused: `fs_process_tests` 19 passed (incl. loopback-free tempdir fixtures, bounded-timeout process test, 4-thread isolation test).
- Eggsec: check ok; 202 NSE tests passed against published 0.1.0; patch probe against the 005D runtime ok (check 0 errors, 202 passed). `Cargo.toml`/`Cargo.lock` restored after the probe; main still consumes crates.io 0.1.0.

## 5. Invariant review

- Filesystem/process policy stays in `NseCapabilityContext` (brokers decide; providers never authorize).
- Sandbox canonical-path/allowed-root semantics fail closed (broker checks the resolved path; sandbox block precedes provider; tests prove zero provider calls).
- Contracts expose runtime-owned DTOs/opaque handles, never `std::fs`/`std::process` types.
- Lua shapes compatible (io/lfs/os/nmap tables verified by corpus + new end-to-end tests; two intentional normalizations documented: interface entries, tmpfile names).
- AgentSafe/CiSafe write/exec denials at least as strict (zero-call proofs; `os.chdir` newly read-gated like `lfs.chdir`).
- No process-global CWD mutation (guard + process-unchanged assertions incl. concurrency).
- Native defaults available (all old constructors/registrations preserved as shims).
- Unix containment not weakened on Windows (Windows gets explicit unsupported errors or portable fallbacks, never silent permission grants).
- No `public_api` breakage; no Eggsec dependency in standalone (boundary script).
- `NseRunReport` serialization unchanged.

## 6. Failure and recovery review

- Provider failures map to legacy Lua shapes (`false`/`-1`/`{error}`/error strings; open failures keep the sandbox wording for blocked paths).
- Cancellation precedes every brokered call (zero-call proof); blocking I/O stays bounded by timeouts; process `run` kills on timeout; spawned children die with their registry.
- Two fix iterations during implementation, both root-caused and locked: (1) `expect_err` on opaque-handle results needs explicit match (no `Debug` on trait objects); (2) wrappers inline-test fixtures must stay direct-`std` (denial tests set up files without capability) so the M005D wrappers guard scans production code only.
- One tooling note: file-write/edit payloads above ~16KB truncate in transit; the M005D provider section and both library rewrites were applied in verified sub-8KB chunks (compile-checked per chunk). No partial content remains (full suite + guards green on the committed tree).
- Patch probe mutated `Cargo.toml`/`Cargo.lock`; both restored, tree verified clean.

## 7. Migration and compatibility review

- Additive only. All pre-existing constructors/registrations/wrapper signatures valid.
- Intended deltas (documented + tested): per-run handle tables (fds restart at 100); capability-gated parent creation; handle I/O preflight + accounting; tmpfile randomness suffix; tracked popen children killed at run end; `os.chdir` read gate; interface-entry normalization; success-only fs accounting; `set_mode` explicit-unsupported off-Unix; `set_permissions` readonly-mapped off-Unix.
- No release/publication from this slice; Eggsec main stays on crates.io 0.1.0.

## 8. Security review

- Broker denies before provider invocation (AgentSafe write, CiSafe exec, sandbox escape, cancellation — all zero-call proven).
- Checks apply to the resolved absolute path; sandbox canonicalizes and the provider operates on the approved path (no post-approval transformation).
- `run` is timeout-bounded with kill; `spawn` children are registry-owned and killed at run end (no detached processes).
- No shell implied by the contract: `run`/`spawn` take program+args; only `io.popen` selects `sh -c`/`cmd /C` via the localized `shell_command` helper (legacy semantics preserved).
- Windows fallbacks never silently grant: unsupported ops error; privilege probe is false off-Unix.
- No secrets handled; provider errors carry paths/specs but no secret content.

## 9. Documentation and operations

- Standalone: `docs/PROVIDERS.md` M005D sections (sequence, coverage table, CWD semantics, platform matrix, deltas, inventory with regeneration command, guards); rustdoc on all new public APIs; guard comments; CI job comments.
- Eggsec: this closure record; plan status `implemented`; registry + subsystem roadmap updates (005D closed).
- Operator impact: none (no config/CLI/TUI/Python change).

## 10. Remaining direct host-operation inventory (machine-auditable)

- Loader/lookup reads: `executor_core.rs` script search reads, `datafiles.rs` authorized reads (policy enforced pre-read on the same path; broker events would double-count into reports).
- Wrapper shims with leaking signatures: `nse_fs_metadata`, `nse_fs_read_dir`, `nse_fs_symlink_metadata` (`std::fs` types), `nse_process_exec` (`std::process::Output`) — capability-gated native bodies, documented for 005E disposition.
- `os.hostname` (`hostname` crate), `os.tmpdir`/`io.tmpfile` env `temp_dir` fallbacks, `std::process::id` labels, `NSE_ENV` thread-local setenv state: preserved legacy host interactions, not bypasses.
- `SandboxConfig::resolve_host`/`is_host_allowed`: no production callers in migrated paths.
- `nmap` registry metadata + `add/get_connection` shims: per 005B inventory.
- `comm.tryssl` (reqwest): per 005B inventory, first consumer of 005C.
- Protocol-specific file/process helpers (not shared/core): deferred per plan scope; covered by 005E source audit.
- Regeneration: `rg -n -e 'std::fs::' -e 'std::process::' -e 'std::env::' -e 'std::os::' src/libraries/io.rs src/libraries/lfs.rs src/libraries/os.rs src/libraries/nmap.rs src/wrappers.rs` (only allow-listed residuals may remain).

## 11. Roadmap disposition

Milestone 005D closed; dependencies are otherwise unchanged:

- **005C (HTTP provider + Eggsec adapter): unaffected** — already ready for handoff; may proceed in parallel per the roadmap.
- **005E qualification: still blocked on 005C** (005B and 005D closures now accepted). No corrective plan required for 005D.
- **GO for 005E readiness tracking**: all 005D closure evidence required by the plan (§15) is present; 005E can consume the fs/process contracts, guards, and inventories as soon as 005C closes.

## 12. Registry updates

- Mark `005-filesystem-process-portability.md` implemented; link this closure.
- Mark 005D closed in the subsystem roadmap.
- Keep 005C ready for handoff; 005E blocked on 005C (005B/005D closed).


## Post-closure operational finding — M005 landing/CI corrective pass

Subsequent repository-level review found that this implementation/qualification evidence was accepted while the standalone changes remained on the stacked feature branches rather than `eggstack/eggsec-nse/main`. The relevant branch evidence for this slice is `m005d-filesystem-process-portability @ b3c43b8d2128f47d83ea66ceb94fcbfe5dc9c69c`. Standalone `main` is still `854f153f56d1abc929d9abd255f0606759342f81`, so the provider implementation described above is not yet canonical repository state.

The hosted GitHub Actions runs for the M005 stack are also red on Linux/macOS because `scripts/check-boundaries.sh` invokes `rg` but the workflow does not provision ripgrep. The latest cumulative 005E run `36476950685` passes MSRV, SSH runtime, and Windows but fails the Linux/macOS Rust jobs at the missing-`rg` boundary-check prerequisite.

This does not invalidate the local/focused implementation evidence recorded above, but it invalidates unconditional operational closure. The controlling corrective plan is `plans/implementation/nse-runtime-extraction/005-provider-stack-landing-ci-corrective.md`. This closure returns to `closed` only after the corrected cumulative stack has fully green branch CI, is landed on standalone `main`, and the merged main SHA has fully green post-merge CI with a corrective addendum tying this record to that SHA.
