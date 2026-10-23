# Desktop GUI Frontend Milestone 008 — Daemon Feature Profile and Build Recipe

Status: ready for handoff

Repository baseline: branch `feat/auto-20261022-76692679` at commit `e1916069`,
with milestones 001–003 closed, milestones 004–005 implemented, and milestones
006–009 planned.

Source roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#milestone-8--daemon-feature-profile-and-build-recipe`

Long-term requirements:

- `plans/000-long-term-specification.md#4-enforcement-invariants`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/000-long-term-specification.md#6-dispatch-runtime-and-wire-identity`
- `plans/000-long-term-specification.md#10-frontend-rules`
- `plans/001-terminology-and-domain-model.md`

Applicable ADRs:

- `plans/adrs/ADR-0007-desktop-gui-frontend-boundary.md`
- `plans/adrs/ADR-0009-gui-shell-technology-selection.md`
- `plans/adrs/ADR-0010-gui-supported-platform-scope.md`

Primary class: capability

---

## 1. Objective

Make it possible to **build** a daemon that actually exposes the engine's
feature set, and to record — machine-checkably — how to build it.

One bounded outcome, delivered as three artifacts that are mutually
conformance-checked:

1. **A daemon feature profile** — an explicit, documented list of engine
   features that a GUI-serving daemon enables — published as a cargo feature
   named `service` on `crates/eggsec-daemon`, with a per-feature inclusion
   rationale and a per-exclusion rationale for every engine feature the
   profile leaves out.
2. **A system-prerequisite map** — the host requirements each profile entry
   implies, mapped through `docs/PLATFORM.md` and the engine's own
   `crates/eggsec/src/platform/`, so an operator knows what to install before
   building and what will still not run live afterwards.
3. **A reproducible build recipe** — the exact cargo invocation, toolchain and
   MSRV constraint, target triple, and feature set per profile, plus the
   script and Makefile target that make it repeatable.

The deliverable is a **build recipe and a profile**, never an in-process
embedding: the GUI must not link the daemon crate or the engine
(`ADR-0007`, architecture guard 155), so the daemon stays a separate process
and this milestone only decides what that process contains.

**Primary class: capability.** The milestone adds a capability that did not
previously exist (a daemon that can run the operations the GUI submits) and
does not change any wire contract.

---

## 2. Why this milestone is ready

### 2.1 Closed hard dependencies

| Dependency | State at the baseline | Citation |
|---|---|---|
| Engine feature registry exists and is authoritative | Closed | `crates/eggsec/src/config/feature_registry.rs:110` (`ALL_FEATURES`), `:179` (the `feature_registry!` invocation) |
| Registry ↔ Cargo.toml bidirectional coverage is already machine-checked | Closed | `crates/eggsec/src/config/feature_registry.rs:622-657`; `crates/eggsec/tests/feature_matrix.rs:93-137` |
| `full` aggregate membership is already machine-checked | Closed | `crates/eggsec/src/config/feature_registry.rs:481-510`; `crates/eggsec/tests/feature_matrix.rs:596-637` |
| Features excluded from `full` carry machine-checked reasons | Closed | `crates/eggsec/src/config/feature_registry.rs:517-602`; `crates/eggsec/tests/feature_matrix.rs:641-691` |
| Runtime feature truth is a single snapshot function | Closed | `crates/eggsec/src/policy_bridge/features.rs:11-17` |
| Engine policy catalog publishes per-operation `required_features` | Closed | `crates/eggsec-policy/src/catalog.rs:291` (`ALL_OPERATION_METADATA`), `:31` (`required_features` field) |
| Platform prerequisite matrix exists for exactly six domains | Closed | `crates/eggsec/src/platform/prereqs.rs:391-403` (`all_domain_ids`), `:407-425` (`report_for`), `:430-444` (`skip_reason_for`); `docs/PLATFORM.md:16-23` |

### 2.2 Stable interface dependencies

| Interface | Why it is stable enough to build on | Citation |
|---|---|---|
| `eggsec::config::ALL_FEATURES` / `FULL_MEMBERS` / `FULL_EXCLUDED_WITH_REASON` | Public, re-exported, fail-closed, and already pinned by tests | `crates/eggsec/src/config/mod.rs:55-60` |
| `eggsec::policy_bridge::features::current_enabled_features()` | Public, pure, registry-backed; already asserted equal to the registry by `snapshot_matches_registry_oracle` | `crates/eggsec/src/policy_bridge/features.rs:11-17`, `:32-43` |
| `eggsec_policy::EnabledFeatures` | Public set with `contains`, `len`, `iter` | `crates/eggsec-policy/src/features.rs:21-63` |
| `crates/eggsec-policy::ALL_OPERATION_METADATA` | The catalog that already drives `DescribeOperations` | `crates/eggsec-policy/src/catalog.rs:291` |
| `crates/eggsec/src/platform/` | The engine's own runtime truth about host prerequisites | `crates/eggsec/src/platform/prereqs.rs:139-153`, `:372-388` |

### 2.3 What this milestone does **not** wait for

| Deferred | Owner | Why M008 can proceed |
|---|---|---|
| Runtime publication of the enabled-feature set (the additive handshake field) | **M006** | The profile's *compiled* feature set is observable inside the daemon process without any protocol change. M008 pins the build-side truth; M006 pins the wire-side truth. |
| Daemon obtainability (`eggsec daemon start`, install path) | **M009** | M008 changes only the cargo feature surface of `crates/eggsec-daemon` and the build recipe. M009 changes the CLI's `daemon start` behaviour and the install story. Neither edits the other's files. |
| Platform qualification of the GUI shell | **M007** | M007 qualifies the *GUI* binary per platform. M008 qualifies the *daemon* build per profile. The two binaries are separate processes and separate cargo invocations. |

### 2.4 The dependency this milestone exists to satisfy

Milestone 006's stated user value is "the GUI reaches the engine's full feature
set". That value is **unreachable with a stock daemon build**, for the reason
measured in §3.1: a `--full-executor` daemon links the engine with **none** of
the engine's 28 `full` members. M008 is the build-side prerequisite for M006's
coverage value; the roadmap states it directly — *"it gates M6's coverage
value"* (`plans/subsystems/desktop-gui-frontend-roadmap.md`, Milestone 8,
`Dependencies`).

---

## 3. Current implementation evidence

Everything in this section is **measured** at commit `e1916069` by reading the
source. Anything not measured is marked *assumed*.

### 3.1 The daemon builds with no engine feature set

`crates/eggsec-daemon/Cargo.toml`:

| Line | Declaration | Meaning |
|---|---|---|
| `:14` | `default = []` | A default daemon build has **no** engine dependency at all. |
| `:15` | `http-api = ["dep:axum", "dep:async-stream", "dep:futures"]` | Optional HTTP/SSE transport. |
| `:18` | `full-executor = ["dep:eggsec", "eggsec/cli"]` | The engine-backed executor path. |
| `:23` | `eggsec = { path = "../eggsec", version = "0.1.0", optional = true }` | The engine is an **optional** dependency, so it is absent from a default build. |
| `:45` | `eggsec = { path = "../eggsec", version = "0.1.0" }` (dev-dependency) | The daemon's tests always link the engine, with the engine's default features. |

The engine's `cli` feature is:

```toml
cli = ["dep:clap", "dep:clap_complete"]
```

(`crates/eggsec/Cargo.toml:301`.)

**Measured consequence.** A daemon built with `--features full-executor` links
the engine with exactly one engine feature enabled: `cli`. The engine's
`default` is empty (`crates/eggsec/Cargo.toml:292`), so nothing else is pulled
in implicitly. The engine's `full` aggregate has **28** members
(`crates/eggsec/src/config/feature_registry.rs:481-510`), and **none** of them
is enabled by that build.

This is the defect the milestone exists to remove. It is also why milestone
006's plan records that a stock daemon advertises 8 capabilities it cannot
execute (`plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md:345-362`).

### 3.2 The engine's `cli` feature is a compound gate, not a "CLI surface"

This is the single most important measured fact for the profile design, and it
**contradicts** the naive reading of "a daemon should not enable `cli`".

`crates/eggsec/src/lib.rs`:

| Line | Declaration |
|---|---|
| `:88-89` | `#[cfg(feature = "cli")] pub mod cli;` |
| `:90-91` | `#[cfg(feature = "cli")] pub mod commands;` |
| `:104-105` | `#[cfg(feature = "cli")] pub mod dispatch;` |
| `:136-137` | `#[cfg(feature = "cli")] pub mod runtime_bridge;` |

`runtime_bridge` is where the engine's `RuntimeTaskExecutor` implementation
lives — `EggsecRuntimeExecutor`
(`crates/eggsec/src/runtime_bridge/executor.rs:36-38`, `impl RuntimeTaskExecutor
for EggsecRuntimeExecutor` at `:128`). The daemon hosts exactly that executor:

```rust
#[cfg(feature = "full-executor")]
{
    let executor = eggsec::runtime_bridge::EggsecRuntimeExecutor::new();
    …
}
```

(`crates/eggsec-daemon/src/main.rs:103-109`.)

**Measured consequence.** `cli` is **not** a "CLI serving surface" — it is the
gate that exposes the engine's *dispatch and runtime-bridge machinery* to any
non-CLI host. A daemon that does not enable `eggsec/cli` cannot compile the
executor it is built around. The daemon's own Cargo.toml comment already says
so: *"Full executor needs engine CLI types (`runtime_bridge` is cli-gated)"*
(`crates/eggsec-daemon/Cargo.toml:16-17`).

Therefore the profile **must** include `cli`, and the "no CLI-shaped features"
rule applies to the **serving surfaces** (`rest-api`, `grpc-api`, `ws-api`) and
to the **aggregate-only** features (`config-watch`, `logging-subscriber`) — not
to the `cli` gate itself. §7 Work package A states this as a decision with its
reasoning, and §10 pins it with a test.

### 3.3 The engine's feature registry — the source of truth

`crates/eggsec/src/config/feature_registry.rs` is the authoritative list, generated
by the `feature_registry!` macro (`:103-168`) from one declaration (`:179-453`).

| Artifact | Count | Location |
|---|---|---|
| `ALL_FEATURES` — every Cargo feature except `default`, each with a `cfg!()`-derived `enabled` flag, a category, and a hint | **50** | `:110-119` (macro-generated), populated at `:179` |
| `FULL_MEMBERS` — the exact direct membership of the `full` aggregate | **28** | `:481-510` |
| `FULL_EXCLUDED_WITH_REASON` — every other non-`default` feature, with a machine-checked rationale | **21** | `:517-602` |

`50 = 28 + 21 + 1` — the 28 `full` members, the 21 excluded-with-reasons, and
the `full` aggregate itself. The partition is enforced by
`full_exclusions_are_documented` (`crates/eggsec/tests/feature_matrix.rs:641-691`),
which asserts that every non-`default` Cargo feature is in exactly one of the
two sets.

The exclusion reasons are the most useful input for this design, because they
are already a **classification** of every non-`full` feature. The engine
declares nine rationale classes at `crates/eggsec/src/config/feature_registry.rs:462-474`:
`test-only`, `security-risk`, `exposure-marker`, `backend-driver`,
`platform-mode`, `serving-surface`, `output-mode`, `implicit-base`, and
`process-host`.

**Important scoping note.** These reasons are scoped to the `full` aggregate.
They are *evidence*, not law: a different aggregate may legitimately include a
feature that `full` excludes — but then it must carry **its own** rationale.
The profile in §7 Work package A does exactly that, and the conformance checker
distinguishes "excluded because `full` excludes it" from "excluded because the
daemon does".

The registry's exclusion reasons are already surfaced by public helpers —
`is_full_member` (`:605-607`) and `full_exclusion_reason` (`:611-616`) — both
re-exported at `crates/eggsec/src/config/mod.rs:55-60`. **The profile MUST
derive from these helpers rather than re-declaring a parallel list.**

### 3.4 The engine's runtime feature truth

`crates/eggsec/src/policy_bridge/features.rs:11-17`:

```rust
pub fn current_enabled_features() -> eggsec_policy::EnabledFeatures {
    let names = crate::config::ALL_FEATURES
        .iter()
        .filter(|entry| entry.enabled)
        .map(|entry| entry.name);
    eggsec_policy::EnabledFeatures::from_names(names)
}
```

**Measured properties.**

- It reads the **registry**, not `cfg!` calls of its own, so the registry is
  the single source of truth at runtime too.
- It is **fail-closed** by construction: `EnabledFeatures::contains` returns
  `false` for unknown names (`crates/eggsec-policy/src/features.rs:44-49`).
- `snapshot_matches_registry_oracle`
  (`crates/eggsec/src/policy_bridge/features.rs:32-43`) already asserts that the
  snapshot and the registry agree, entry by entry. **The profile's runtime
  conformance test can therefore compare `current_enabled_features()` against a
  documented list without any new plumbing.**
- `EnabledFeatures` exposes `contains`, `is_empty`, `len`, and `iter`
  (`crates/eggsec-policy/src/features.rs:44-63`), so an exact set comparison is
  possible.

### 3.5 What the GUI's task kinds require — derived, not asserted

The requirement set is **derived from the engine's own policy catalog**, not
hand-written. `crates/eggsec-policy/src/catalog.rs` declares
`ALL_OPERATION_METADATA` at `:291`, and each entry carries
`required_features: &'static [&'static str]` (`:31`). The distinct non-empty
values in the catalog are:

| `required_features` value | Catalog line | Reaching task kind |
|---|---|---|
| `stress-testing` | `:451` | `StressTest` |
| `packet-inspection` | `:468` | `PacketCapture`, `PacketTraceroute`, `PacketSend` |
| `nse` | `:536` | `Nse` |
| `db-pentest` | `:557` | `DbPentest` |
| `c2` | `:574` | `C2` |
| `web-proxy` | `:591` | `Intercept` |
| `wireless` | `:608` | `Wireless`, `WirelessActive` |
| `wireless-advanced` | `:625` | `WirelessActive` |
| `advanced-hunting` | `:642` | `Hunt` |
| `headless-browser` | `:659` | `Browser` |
| `compliance` | `:676` | `Compliance` |
| `database` | `:693` | `Storage` |
| `external-integrations` | `:718` | `Integrations` |
| `finding-workflow` | `:743` | `Workflow` |
| `vuln-management` | `:768` | `Vuln` |

**15 distinct required features.** Milestone 006's §3.3 table
(`plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md:159-190`)
lists the same 30 task-kind variants and their `required_features` column; the
task-kind↔operation-id mapping in that plan is reused here unchanged.

Two additional engine gates are **not** in the catalog but are still required,
and both are measurable:

| Gate | Why it is required | Citation |
|---|---|---|
| `cli` | The engine's dispatch and runtime-bridge machinery is `cli`-gated; without it the daemon cannot host the executor at all. | `crates/eggsec/src/lib.rs:88-91`, `:104-105`, `:136-137` |
| `tool-api` | Gates `pub mod tool` — the tool abstraction layer the operation layer is built on. Implied transitively by `nse` (`nse = ["tool-api", …]`). | `crates/eggsec/src/lib.rs:167-168`; `crates/eggsec/Cargo.toml:329` |

And two more are implied transitively but must be declared **explicitly** so
the profile's *effective* set equals its *declared* set (which is what makes
the conformance test an exact-equality test rather than a closure computation):

| Gate | Implied by | Citation |
|---|---|---|
| `postex` | `c2 = ["postex", "evasion"]` | `crates/eggsec/Cargo.toml:423` |
| `evasion` | `c2 = ["postex", "evasion"]` | `crates/eggsec/Cargo.toml:423` |

**15 + 1 + 1 + 2 = 19.** One more feature completes the set — `websocket` — on
the measured ground that it changes the *behaviour* of an already-required
task kind rather than adding a new one:

| Gate | Why it is required | Citation |
|---|---|---|
| `websocket` | The `Fuzz` task kind's WebSocket path is compiled unconditionally but its **live** testing branch is `#[cfg(feature = "websocket")]`. Without it the fuzzer only emits static payloads; with it, it opens a real WebSocket, tests origins and injection, and reports live results. | `crates/eggsec/src/fuzzer/engine/advanced.rs:99-104` (dispatch); `crates/eggsec/src/fuzzer/advanced.rs:420-426` (without the feature) vs `:428-456` (with it) |

**Final required set: 20 engine features.**

### 3.6 The engine's system prerequisites — measured, with one documented inconsistency

`docs/PLATFORM.md:16-23` is the platform matrix, and it covers **exactly six**
domains — the same six the engine's own code knows
(`crates/eggsec/src/platform/prereqs.rs:391-403`):

```rust
["mobile-dynamic", "packet-inspection", "wireless", "wireless-advanced", "stress-testing", "nse"]
```

| Domain | Engine feature | Build-time system dependency | Runtime prerequisite | Citation |
|---|---|---|---|---|
| `mobile-dynamic` | `mobile-dynamic` → `mobile` | none | `adb` optional (pure-Rust probe otherwise); lab device for live | `docs/PLATFORM.md:18`; `crates/eggsec/Cargo.toml:430`, `:426` |
| `packet-inspection` | `packet-inspection` | **none** — `pnet` is pure Rust | `CAP_NET_RAW`/root and Linux for live capture; fixtures run anywhere | `crates/eggsec/Cargo.toml:326`; `docs/PLATFORM.md:19` |
| `wireless` | `wireless` | none — pure marker, `[]` | wireless-tools (`iwlist`) for live discovery | `crates/eggsec/Cargo.toml:409`; `docs/PLATFORM.md:20` |
| `wireless-advanced` | `wireless-advanced` → `wireless` | none — pure marker | `CAP_NET_ADMIN`/root + monitor mode + `--allow-active-wireless`; manual lab only | `crates/eggsec/Cargo.toml:412`; `docs/PLATFORM.md:21` |
| `stress-testing` | `stress-testing` | **none** — `pnet`/`socket2`/`nix`/`surge-ping` are pure Rust | `CAP_NET_RAW`/root and Linux for live; fixtures run anywhere | `crates/eggsec/Cargo.toml:323`; `docs/PLATFORM.md:22` |
| `nse` | `nse` | **C toolchain** (C compiler + `perl` + `make`) for **vendored** OpenSSL — **not** a system `libssl-dev` | none (local script execution) | `crates/eggsec/Cargo.toml:329`; see below |

**Two measured corrections to the repository's own documentation.**

1. **`packet-inspection` does not need `libpcap`.** The engine's feature is
   `packet-inspection = ["pnet", "pnet_packet", "libc"]`
   (`crates/eggsec/Cargo.toml:326`) — pure Rust, no C library. The repository
   already says so in the sweep script: *"`stress-testing` / `packet-inspection`
   use pnet (pure Rust, no libpcap link required for compilation). They are NOT
   gated on libpcap here"* (`scripts/check-features-individual.sh:140-143`).
   `docs/FEATURE_MATRIX.md:334` and `crates/eggsec/src/platform/prereqs.rs:549`
   still name `libpcap-dev`, and `scripts/check-features-individual.sh:87-88`
   still sets `PREREQ_PACKET="libpcap-dev"` / `PREREQ_STRESS="libpcap-dev"`.
   **The code and the sweep script agree with each other; the prose docs are
   stale.** This milestone MUST NOT propagate the stale claim into the operator
   recipe; it must also record the discrepancy (§12, Work package D).

2. **`nse` does not need system `libssl-dev`.** `eggsec-nse`'s optional
   dependency is declared `openssl = { version = "0.10", features = ["vendored"],
   optional = true }` (upstream `eggsec-nse 0.2.0` the vendored-OpenSSL registry entry `~/.cargo/registry/src/index.crates.io-*/eggsec-nse-0.2.0/Cargo.toml:221-224`), and
   `Cargo.lock` confirms `openssl-sys 0.9.117` → `openssl-src 300.6.1`
   (`Cargo.lock:6101-6107`, `:6110-6125`). The OpenSSL build is **vendored**:
   it needs a C compiler and `perl`, not a system OpenSSL.
   `docs/PLATFORM.md:23` (*"`libssl-dev` at build"*), `docs/FEATURE_MATRIX.md:333`,
   and `crates/eggsec/src/platform/prereqs.rs:722` still name `libssl-dev`, while
   the sweep script correctly does **not** gate `eggsec/nse`
   (`scripts/check-features-individual.sh:193`). **Same shape of discrepancy:
   prose stale, code correct.**

   The operator recipe MUST name the **C toolchain** (`cc`, `perl`, `make`) as
   the real requirement, and MAY list `libssl-dev` as harmless-but-unnecessary.
   Naming `libssl-dev` as *required* would be false **and** would fail on a
   host that has a C toolchain but no OpenSSL headers — exactly the
   false-negative the correction avoids.

3. **`nse-ssh2` does need system `libssh2`.** `nse-ssh2 = ["nse",
   "eggsec-nse/nse-ssh2", "dep:ssh2"]` (`crates/eggsec/Cargo.toml:332`) pulls
   `ssh2 0.9.6` → `libssh2-sys 0.3.3` (`Cargo.lock:5041-5048`), and
   `libssh2-sys` links the **system** library. The sweep gates it:
   `PREREQ_NSE_SSH2="libssh2-1-dev"` (`scripts/check-features-individual.sh:86`).
   **`nse-ssh2` is excluded from the profile** (§7 Work package A), so this
   prerequisite does not reach the daemon's build.

4. **`grpc-api` needs `protoc`.** The generated Rust proto code is checked in,
   but `tonic-reflection` needs a binary descriptor set, so `crates/eggsec/build.rs`
   runs `prost_build::compile_protos` at build time and panics without
   `protoc` (`crates/eggsec/build.rs:5-29`). The sweep gates it:
   `PREREQ_GRPC_API="protobuf-compiler (protoc)"`
   (`scripts/check-features-individual.sh:85`). **`grpc-api` is excluded from
   the profile**, so `protoc` is not a daemon-build prerequisite.

**Consequence for the profile.** Of the six platform-sensitive domains, only
`nse` has a build-time cost, and that cost is a **C toolchain**, not a system
library. Every other feature in the required set is pure Rust or a pure marker.
This single fact is decisive for the option analysis in §7 Work package A.

### 3.7 The daemon's runtime flag vs. its cargo feature — two different things

| Thing | Kind | Where |
|---|---|---|
| `--full-executor` | **runtime CLI flag** on the `eggsec-daemon` binary | `crates/eggsec-daemon/src/main.rs:40-43`; the `#[cfg]`-selected branches at `:102-116` |
| `full-executor` | **cargo feature** on the `eggsec-daemon` crate | `crates/eggsec-daemon/Cargo.toml:18` |

The two share a name but are unrelated mechanisms. The runtime flag means
"host the engine executor instead of the no-op executor". The GUI's own
guidance string names the **runtime** flag:

```rust
format!("eggsec-daemon --full-executor --socket-path {socket_path}")
```

(`crates/eggsec-gui/src/connection.rs:52`; pinned by the GUI's own tests at
`crates/eggsec-gui/src/connection.rs:206` and `:213`, and
`crates/eggsec-gui/src/config.rs:87`, `:140`.)

**Consequence.** Renaming or replacing the *cargo* feature MUST NOT change
the runtime flag and MUST NOT change the GUI's guidance string. The GUI crate
is out of this milestone's ownership entirely; the runtime flag stays
`--full-executor`.

### 3.8 The guards that pin the daemon's feature surface

| Guard | What it pins today | Citation |
|---|---|---|
| 104 | The engine's library default is empty and process hosts opt in explicitly. Its daemon line pins the **exact literal** `full-executor = ["dep:eggsec", "eggsec/cli"]`. | `scripts/check-architecture-guards.sh:2656-2685`, daemon line at `:2677-2680` |
| 16 | `eggsec-daemon`'s default dependencies are `eggsec-runtime` + `eggsec-daemon-protocol` only; the engine dep stays optional. | `scripts/check-architecture-guards.sh:273` (the `optional = true` check) |
| 15 | The daemon has no TUI dependencies. | `scripts/check-architecture-guards.sh:271` |
| 155 | The GUI links no engine and no daemon crate. | `scripts/check-architecture-guards.sh:4516-4558` |

Guard 104's daemon line is a **literal match**, so M008 **must** update it
when the daemon's feature declarations change. The guard's *intent* — "the
engine library default is empty; a process host opts in explicitly" — is
preserved and strengthened: the new literal pins the **whole profile**, not
just `cli`.

### 3.9 The feature-matrix test file — where the conformance test lives

`crates/eggsec/tests/feature_matrix.rs` already contains exactly the machinery
M008 needs, and already sets the precedent for reading a sibling crate's
manifest:

| Existing asset | Citation |
|---|---|
| `parse_features_raw(manifest_path)` — parse a Cargo.toml `[features]` table into `name → raw dependency strings` | `:428-463` |
| `extract_engine_features(deps)` — filter `eggsec/<feature>` references out of a raw dependency list | `:464-475` |
| `cli_forwarded_features_exist_in_registry` — reads `../eggsec-cli/Cargo.toml` and checks each forwarded engine feature is known | `:476-493` |
| `tui_forwarded_features_exist_in_registry` — the same for `../eggsec-tui/Cargo.toml` | `:495-512` |
| `full_membership_matches_cargo` — `FULL_MEMBERS` ⇔ Cargo `full = [...]`, exact set equality | `:591-637` |
| `full_exclusions_are_documented` — the 28/21 partition | `:639-691` |
| `individual_sweep_covers_every_feature` — pins the sweep script's profile list, including `"eggsec-daemon --features full-executor"` | `:824-892`, daemon line at `:868` |

The whole file is `#![cfg(feature = "cli")]`-gated (`:10`), which is compatible
with a profile that includes `cli`.

**Measured constraint on where the test can live.** `make check`
(`Makefile:154-182`) runs `cargo test -p eggsec --features rest-api,cli --tests`
(`Makefile:170`). The engine's test suite therefore compiles with
**`rest-api,cli`**, not with the daemon's profile. A test that asserted the
engine's *compiled* feature set equals the profile would **fail** under
`make check`. The conformance check MUST therefore be split:

| Half | Where | When it runs |
|---|---|---|
| **Static** — parse the daemon's manifest, compare to the documented list, to the engine's registry, and to the catalog-derived required set | `crates/eggsec/tests/feature_matrix.rs` | `make check` (always) |
| **Runtime** — compare the engine's *compiled* feature set to the documented list | `crates/eggsec-daemon/tests/feature_profile.rs`, `#![cfg(feature = "service")]` | a new `make check-daemon-profile` target + `make check-feature-profiles` (deep check) |

### 3.10 Dependency-closure consequences of the excluded features

Measured from `Cargo.lock` (reverse-dependency walk over the resolved graph):

| Excluded engine feature | Dependency it would add | Chain | Advisory exposure |
|---|---|---|---|
| `mobile` (`:426`) | `zip 2.4.2` | `zip` ← `eggsec` (via `mobile = […, "zip", …]`) | `zip 2.4.2` carries RUSTSEC-2026-0367 / -0368 / -0369 (inherited; see §3.11) |
| `db-pentest-mssql-tiberius` (`:379`) | `tiberius 0.12.3` | `tiberius` → `tokio-rustls 0.24.1` → `rustls 0.21.12` | the `rustls 0.21` family (inherited; see §3.11) |
| `db-pentest-mongodb` (`:381`) | `mongodb` | `mongodb` ← `eggsec` | — |
| `db-pentest-redis` (`:383`) | `redis` | `redis` ← `eggsec` | — |
| `nse-ssh2` (`:332`) | `ssh2` → `libssh2-sys` | system `libssh2` at build | — |
| `grpc-api` (`:320`) | `tonic`, `prost-build`, … | `protoc` at build | — |
| `pdf` (`:406`) | `printpdf` | `printpdf` ← `eggsec` | `lopdf` / `ttf-parser` (already excepted in `deny.toml:30`, `:32`) |
| `rest-api` (`:311`) | `axum`, `tower`, `tower-http`, `notify`, `notify-debouncer-mini`, `lettre` | — | — |

**The base `db-pentest` does not drag the drivers in.** `db-pentest = ["sqlx",
"dep:eggsec-db-lab", "eggsec-db-lab/db-drivers"]`
(`crates/eggsec/Cargo.toml:376`), and `eggsec-db-lab`'s `db-drivers` feature is
`["dep:sqlx"]` only (postgres + mysql). The MSSQL/MongoDB/Redis drivers are
separate markers (`crates/eggsec-db-lab/Cargo.toml` `mssql`, `mongodb`,
`redis`). So excluding the three driver markers genuinely keeps `tiberius` —
and therefore the `rustls 0.21` chain — **out of the shipped daemon binary**.

**An honest caveat.** `deny.toml` sets `[graph] all-features = true`
(`deny.toml:8-12`, documented at `docs/DEPENDENCY_EXCEPTIONS.md`), so
`cargo deny` resolves **all** optional-feature closures regardless of the
profile. **Excluding a feature from the profile does not remove its advisory
entry from `cargo deny`'s view.** What it does remove is the crate from the
**shipped daemon binary's** dependency closure. §14 records `make check-deps`
as an inherited failure that the profile does not and cannot fix.

### 3.11 Known pre-existing failures (inherited, not attributable to M008)

| Failure | Exact condition | Inherited? |
|---|---|---|
| `make check-deps` — advisories | `zip 2.4.2` (RUSTSEC-2026-0367 / -0368 / -0369); the `rustls 0.21.12` family via `tokio-rustls 0.24.1` → `tiberius 0.12.3` → `eggsec`; seven entries reachable only through the GUI's Tauri closure — `glib 0.18.5` (RUSTSEC-2024-0429), `glib-macros 0.18.5` → `proc-macro-error 1.0.4` (RUSTSEC-2024-0370), and the five `unic-*` crates (RUSTSEC-2025-0081 / -0075 / -0080 / -0100 / -0098) | **Yes.** `deny.toml` is a closed allow-list; M008 adds no dependency and records no exception. |
| `cargo check --workspace --all-features` — `E0616: field 'url' of struct 'WebSocketFuzzer' is private` | `crates/eggsec/src/fuzzer/advanced.rs:446` constructs `crate::websocket::WebSocketTestConfig { url: … }` while the field is private at `crates/eggsec-payloads/src/websocket.rs:43` | **Yes.** A payload-crate privacy defect, unrelated to any feature profile. It is *reachable only when `websocket` is enabled* — and the profile **includes** `websocket`. §14 makes "this defect blocks the profile's build" a stop condition rather than an invitation to fix it here. |

### 3.12 Known gaps at the baseline (not M008's to close)

| Gap | Measured | Owner |
|---|---|---|
| The daemon's `runtime_bridge` scope resolution does not recognise the GUI surface as permissive | `crates/eggsec/src/runtime_bridge/executor.rs:70-73` matches only `RuntimeSurface::CliManual \| RuntimeSurface::TuiManual`; `RuntimeSurface::DesktopGuiManual` exists (`crates/eggsec-runtime/src/request.rs:31`) and is classified as manual elsewhere (`crates/eggsec/src/runtime_bridge/surface.rs:240-245`), but a GUI session without an explicit scope falls into the strict branch and fails closed | **Not M008.** The code is inside `RuntimeTaskExecutor`, which §5 places out of scope. Recorded here because it bounds what a *correctly profiled* daemon can actually execute for the GUI. |
| The daemon publishes no enabled-feature set | `DaemonCapabilities` (`crates/eggsec-daemon-protocol/src/protocol.rs`) has no feature field; the capability list is a compile-time constant (`crates/eggsec-runtime/src/capabilities.rs`) | **M006.** |
| `eggsec daemon start` advertises 20 task kinds and executes none | `crates/eggsec-cli/src/daemon_cli.rs:45-53`, `:584-607` | **M009.** |
| The GUI cannot link the daemon or the engine | `ADR-0007`; guard 155 | **By design.** The daemon stays a separate process; M008 delivers a build recipe, never an embedding. |
| The daemon binary is not installed by anything | `cargo install --path crates/eggsec-cli` installs only `eggsec` | **M009.** |

---

## 4. Invariants that must not regress

| # | Invariant | Citation |
|---|---|---|
| 1 | The engine's library default stays empty (`default = []`). M008 adds no engine feature. | `crates/eggsec/Cargo.toml:292` |
| 2 | The daemon's default build stays engine-free (`default = []`, the engine dep stays `optional = true`). | `crates/eggsec-daemon/Cargo.toml:14`, `:23`; guard 16 at `scripts/check-architecture-guards.sh:273` |
| 3 | The GUI links no engine and no daemon crate. M008's code surface is `crates/eggsec-daemon` and the engine's feature surface — **never** `crates/eggsec-gui`. | `ADR-0007`; guard 155 at `scripts/check-architecture-guards.sh:4516-4558` |
| 4 | The daemon's runtime `--full-executor` flag and its semantics are unchanged: it still means "host the engine executor instead of the no-op executor". The GUI's guidance string is not edited. | `crates/eggsec-daemon/src/main.rs:40-43`, `:102-116`; `crates/eggsec-gui/src/connection.rs:52` |
| 5 | The engine's `cli` gate keeps its current meaning — it continues to gate `cli`, `commands`, `dispatch`, and `runtime_bridge`. M008 does not split the gate. | `crates/eggsec/src/lib.rs:88-91`, `:104-105`, `:136-137` |
| 6 | The daemon protocol does not change: no `ClientCommand`/`ServerMessage` variant, no `DaemonCapabilities` field, protocol version stays at 2. | `crates/eggsec-daemon-protocol/src/protocol.rs` |
| 7 | `RuntimeTaskExecutor` does not change: no method is added, removed, or re-signed. | `crates/eggsec/src/runtime_bridge/executor.rs` |
| 8 | The engine's feature registry stays the single source of truth. The profile derives from `ALL_FEATURES` / `FULL_MEMBERS` / `FULL_EXCLUDED_WITH_REASON`; M008 adds **no second hand-maintained feature list**. | `crates/eggsec/src/config/feature_registry.rs:110`, `:481-510`, `:517-602` |
| 9 | Ring-only TLS, and no new dependency at all: `deny.toml` needs no new exception and `docs/DEPENDENCY_EXCEPTIONS.md` needs no new entry. | `deny.toml`; `docs/DEPENDENCY_EXCEPTIONS.md` |
| 10 | The workspace Tokio baseline is unchanged (`default-features = false` with per-crate explicit feature lists). | `Makefile:2691-2699`; guard 105 |
| 11 | `full` keeps its 28 members and its 21 documented exclusions; the existing tests stay green without modification. | `crates/eggsec/tests/feature_matrix.rs:596-691` |
| 12 | The MSRV floor stays 1.89; the profile's feature set must not pull a crate whose MSRV exceeds it. | `Cargo.toml:39`; `make check-msrv` (`Makefile:128-143`) |

---

## 5. Scope

### In scope

| Artifact | Content |
|---|---|
| **A new cargo feature `service` on `crates/eggsec-daemon`** | Enables an explicit list of engine features, together with the **profile contract** that defines the list: the included set, the excluded set, the inclusion rule, and the pure conformance checker. |
| **`full-executor` becomes a deprecated alias** for `service` | So every existing build invocation and every existing documentation reference keeps compiling and now produces the profile daemon. |
| **The profile contract, in the engine** (`crates/eggsec/src/config/daemon_profile.rs`) | The engine owns the feature registry and the policy catalog the profile derives from; the daemon crate consumes it. |
| **Per-feature rationales** | An inclusion rationale and an exclusion rationale for every one of the engine's 50 features. |
| **The system-prerequisite map** | Mapped through `docs/PLATFORM.md` and `crates/eggsec/src/platform/`, with the two documentation discrepancies in §3.6 explicitly corrected. |
| **A reproducible build recipe** per profile | The exact cargo invocation, the toolchain and MSRV constraint, the target triple, the feature set, and the script + Makefile target that make it repeatable and machine-verifiable. |
| **A machine-checked conformance test** | Pins the documented list, the cargo feature, and the compiled feature set to each other, so the docs cannot drift from the build. |
| **The guard update** | Required by the change to the daemon's feature declarations (guard 104's daemon literal), strengthened to pin the whole profile. |
| **The documentation updates** | Listed in §12. |

### Explicitly out of scope

| Out | Detail |
|---|---|
| **Any change to the daemon protocol** | No `ClientCommand` or `ServerMessage` variant; no `DaemonCapabilities` field; no protocol version bump. **M006 owns the additive enabled-features handshake field** — M008 defines the profile, M006 publishes it. M008 MUST NOT touch `crates/eggsec-daemon-protocol/`. |
| **Any change to `RuntimeTaskExecutor` or `EggsecRuntimeExecutor`** | `crates/eggsec/src/runtime_bridge/executor.rs` is off-limits for M008, including the scope-resolution gap in §3.12, which is inside `RuntimeTaskExecutor`. |
| **Any change to `eggsec daemon start`** | `crates/eggsec-cli/src/daemon_cli.rs` is off-limits for M008 — **M009 owns obtainability and start honesty.** |
| **Any change to `crates/eggsec-gui/`** | M008's code surface is `crates/eggsec-daemon/` plus the engine's feature surface. The GUI's runtime guidance string, its connection code, and its tests are untouched. |
| **Any change to the CLI's or the TUI's feature profiles** | `crates/eggsec-cli/Cargo.toml` and `crates/eggsec-tui/Cargo.toml` are off-limits; guard 104's lines for those two crates (`scripts/check-architecture-guards.sh:2669-2676`) stay untouched. |
| **Any change to the engine's `full` aggregate or its 28 members** | — |
| **Any new dependency** | No crate is added, so `deny.toml` and `docs/DEPENDENCY_EXCEPTIONS.md` need no new entry. A new dependency is a **stop condition** (§14), not an implementation choice. |
| **Any packaging, signing, notarization, installer, or update channel** | Those belong to the separately-managed distribution workstream and to M007's handoff (`plans/implementation/desktop-gui-frontend/007-packaging-and-platform-qualification.md:452`). |
| **Any fix for the inherited `make check-deps` advisories or the inherited `--all-features` compile error** | Both are recorded as inherited (§3.11). |
| **Any change to the daemon's persistence, client registry, RBAC, or transport** | — |
| **Windows support** | `ADR-0010` puts Windows out of scope for the GUI because the daemon's transport is a Unix domain socket. M008's recipe covers the supported platforms only. |

---

## 6. Required production changes

### Core/domain

**New module `crates/eggsec/src/config/daemon_profile.rs`** — the profile
contract, owned by the engine because the engine owns the registry and the
catalog:

| Item | Kind | Content |
|---|---|---|
| `DAEMON_SERVICE_PROFILE` | `pub static [&str]` | The 20 engine features the `service` profile enables (exact list in §7 Work package A). **This constant is the single source of truth for the profile list.** |
| `DAEMON_SERVICE_EXCLUDED` | `pub static [&str]` | The 30 engine features the profile excludes, each with a rationale. |
| `DAEMON_SERVICE_FORBIDDEN` | `pub static [&str]` | The subset of the excluded set whose exclusion is **absolute** — the serving surfaces, the security-risk features, the test-only feature, the aggregate itself, and the MCP exposure markers. A profile that enables any of these is **invalid**, not merely discouraged. |
| `required_profile_features()` | `pub fn() -> Vec<&'static str>` | The **derived** required set: the distinct non-empty `required_features` of `eggsec_policy::ALL_OPERATION_METADATA`, plus the `cli` host gate, plus the explicitly-declared transitive members. Derived, not hand-written. |
| `profile_violations(profile: &[&str]) -> Vec<ProfileViolation>` | `pub fn` | The **pure** checker. Given a candidate list, return every violation: an unknown feature name, a forbidden feature, a missing required feature, a duplicate entry, or an entry that is not in the registry. Pure, deterministic, and callable from a test with a deliberately-bad list. |

`ProfileViolation` is a small enum with a `Display` impl carrying the feature
name and the reason, so a failure message names the offending feature.

**Exports.** `crates/eggsec/src/config/mod.rs` re-exports the module's public
items alongside the existing `feature_registry` re-exports
(`crates/eggsec/src/config/mod.rs:55-60`).

**No new engine feature.** The profile contract is compiled into the engine
**unconditionally** — it is data, not a capability. It must not be gated
behind `cli` or any other feature, so that the conformance test can run in
the `no-default-features` build as well.

### Storage and migrations

**None.** The profile is a compile-time list; it has no storage footprint, no
schema, and no migration. The daemon's SQLite store
(`crates/eggsec-daemon/src/store/`) is untouched.

### Protocol and DTOs

**None — and this is a hard boundary.** M008 adds no protocol field.

The distinction that matters, and that an implementer must not blur:

| Artifact | Owner | Milestone |
|---|---|---|
| The **compiled** feature set of the daemon process | M008 | this plan |
| The **published** enabled-feature set on the wire (`DaemonCapabilities`) | **M006** | `plans/implementation/desktop-gui-frontend/006-full-operation-coverage-and-report-export.md` |

M008's static and runtime conformance tests read the compiled feature set
**inside the daemon process** (`eggsec::policy_bridge::features::current_enabled_features()`).
They do not read it over a socket, and they do not add a socket field. If an
implementer finds themselves wanting to add a `DaemonCapabilities` field to
make a conformance test pass, that is the signal that M006's territory has
been entered and the milestone boundary has been crossed.

### Runtime and concurrency

**One runtime-visible change, and it is a no-op for existing behaviour.**

The `#[cfg(feature = "full-executor")]` gates in the daemon switch to
`#[cfg(feature = "service")]`:

| Site | Current | After |
|---|---|---|
| `crates/eggsec-daemon/src/main.rs:104` | `#[cfg(feature = "full-executor")]` | `#[cfg(feature = "service")]` |
| `crates/eggsec-daemon/src/main.rs:110` | `#[cfg(not(feature = "full-executor"))]` | `#[cfg(not(feature = "service"))]` |
| `crates/eggsec-daemon/src/host.rs:2862` | `#[cfg(feature = "full-executor")]` | `#[cfg(feature = "service")]` |

Because `full-executor = ["service"]`, a build with `--features full-executor`
still compiles the executor path — the alias keeps every existing invocation
working and simply produces a **stronger** daemon. The `#[cfg(not(...))]`
branch's warning text is updated to name the new feature
(`crates/eggsec-daemon/src/main.rs:112`).

**No concurrency change.** The profile changes what the daemon *can* run, not
how it runs it. Session lifecycle, task dispatch, cancellation
(`eggsec-runtime::race_with_cancel`), and the spawn-timeout discipline are
untouched.

### Frontend or operator surface

- **The GUI is not touched** (`crates/eggsec-gui/` is out of scope). Its
  runtime guidance string and its tests are unchanged.
- **The operator surface is the build recipe**: a new
  `scripts/build-daemon-profile.sh` and a new `make check-daemon-profile`
  target, plus a `docs/DAEMON_FEATURE_PROFILE.md` document.
- The daemon's runtime `--help` output changes in exactly one respect: the
  `--full-executor` flag's help text (`crates/eggsec-daemon/src/main.rs:40`)
  must stop naming a cargo feature that is now an alias. The flag's *name* and
  *meaning* are unchanged.

### Security and authorization

- **`insecure-tls` is never in the profile.** It is `security-risk` in the
  engine's own classification (`crates/eggsec/src/config/feature_registry.rs:567-569`)
  and it MUST appear in `DAEMON_SERVICE_FORBIDDEN`, so a future edit that adds
  it fails the conformance check rather than shipping.
- **The MCP exposure markers are never in the profile.** `db-pentest-mcp`,
  `web-proxy-mcp`, `c2-mcp` are `exposure-marker`
  (`crates/eggsec/src/config/feature_registry.rs:526-529`, `:538-541`,
  `:594-597`). Enabling them would register MCP tools in a daemon whose
  protocol is the Unix socket — an exposure surface the operator did not ask
  for.
- **No serving surface is in the profile.** `rest-api`, `grpc-api`, `ws-api`
  are `serving-surface` (`:562-565`, `:598-601`). The daemon already *is* the
  serving surface.
- **The profile does not change any authorization path.** `EnforcementContext`
  remains the mandatory pre-dispatch gate; the profile only decides which
  operations exist to be gated. `OperationMetadata` remains the single source
  of operation policy.
- **No new dependency** ⇒ no new `deny.toml` exception and no new entry in
  `docs/DEPENDENCY_EXCEPTIONS.md`. A new dependency is a stop condition.

### Build system and reproducible recipe

M008 adds a **build-side** production surface, and this is the only section of
§6 that describes a new operator-facing artifact beyond documentation.

| Artifact | Kind | Production change |
|---|---|---|
| `scripts/build-daemon-profile.sh` | new script | The reproducible build entry point: refuses a dirty tree without `--allow-dirty`, pins `cargo build --release --locked -p eggsec-daemon --features <profile> --target <triple>`, and writes a metadata sidecar (commit SHA, dirty flag, `rustc -Vv`, the MSRV floor, the target triple, the **resolved engine feature set**, the `Cargo.lock` hash, size, SHA-256). No signing, no bundling, no packaging. |
| `make build-daemon-profile` | new Makefile target | The operator-facing wrapper for the script. |
| `make check-daemon-profile` | new Makefile target | Runs the profile's static and runtime conformance tests plus a `cargo check` of the profile build. Also added to `make check-feature-profiles` (`Makefile:206-228`, the deep-check representative-profile target). |
| `scripts/check-features-individual.sh:216` | modified | Keeps the `--features full-executor` sweep entry (still valid as the alias) and gains a `--features service` entry plus a note that the profile needs a C toolchain for vendored OpenSSL — the same prerequisite class the sweep already assumes at `:193`. |

The script **MUST NOT** introduce an npm step, a code-signing identity, an
installer, or an updater — those belong to the separately-managed distribution
workstream and to M007's handoff
(`plans/implementation/desktop-gui-frontend/007-packaging-and-platform-qualification.md:452`).

The recipe's exact per-profile field table — command, toolchain, MSRV, target,
features, lockfile, output, prerequisites — is specified in §7 Work package E.

### Dependencies and supply chain

| Property | Consequence for M008 |
|---|---|
| **No new dependency is added.** | `deny.toml` and `docs/DEPENDENCY_EXCEPTIONS.md` gain **no** new exception or entry. A new dependency is a **stop condition** (§14), not an implementation choice: it would need an owner + a review-by date recorded in both files. |
| **The profile's dependency closure is narrower than the engine's `full`.** | Excluding `mobile` keeps `zip 2.4.2` out of the shipped daemon; excluding `db-pentest-mssql-tiberius` keeps `tiberius 0.12.3` → `tokio-rustls 0.24.1` → `rustls 0.21.12` out (§3.10). The base `db-pentest` does **not** drag the drivers in (`eggsec-db-lab/db-drivers` is `["dep:sqlx"]` only). |
| **`cargo deny` still resolves every optional closure.** | `deny.toml:8-12` sets `[graph] all-features = true`, so the advisory entries stay visible to `make check-deps` regardless of the profile. The profile narrows the **shipped binary**, not `cargo deny`'s view (§3.10). Do not claim the profile "fixes" an advisory. |
| **Ring-only TLS is preserved.** | The profile adds no TLS stack and changes no existing one. The engine's `insecure-tls` is never in the profile. |
| **The workspace Tokio baseline is preserved.** | The daemon's own Tokio feature list is unchanged (`crates/eggsec-daemon/Cargo.toml:28`); the profile adds no Tokio capability. |

### Documentation and static guards

The full target-by-target list is §12. Two items are load-bearing enough to
restate here:

- **Guard 104's daemon literal** (`scripts/check-architecture-guards.sh:2677-2680`)
  pins the *exact* text `full-executor = ["dep:eggsec", "eggsec/cli"]`, so it
  **must** change when the daemon's declarations change. Its *intent* — "the
  engine library default is empty; a process host opts in explicitly" — is
  preserved and strengthened: the new literal pins the **whole profile**, not
  just `cli`. Its other three checks (the engine's empty default, and the CLI
  and TUI's explicit `features = ["cli"]`) are untouched.
- **One new guard is proposed: 163.** It pins the daemon's `service` feature to
  the engine's `DAEMON_SERVICE_PROFILE` and pins `full-executor` to the alias
  shape.

---

## 7. Ordered work packages

### Work package A — Decide the profile shape and pin the list

Intent:

Choose **one** daemon feature profile design among four candidate shapes,
state the reasoning, and produce the exact engine-feature list with a
per-feature inclusion rationale and a per-exclusion rationale for all 50
engine features.

Required changes:

**The four options, as measured.**

| Option | Shape | Measured assessment |
|---|---|---|
| **A — one `service` profile** | A cargo feature on `crates/eggsec-daemon` enabling an explicit, documented list of engine features chosen for what a GUI-serving daemon needs | **Selected.** §3.5 shows the required set is **derivable** from the engine's own policy catalog (`required_features`), so the list has a mechanical source rather than a taste-based one. §3.6 shows only one required feature (`nse`) has a build-time cost, and that cost is a C toolchain rather than a system library — so a second, "dependency-free" profile would buy nothing at the build layer. One profile, one list, one conformance checker. |
| **B — reuse the engine's `full`** | `full` already aggregates 28 features and is already machine-checked | **Rejected.** `full` is the wrong target for three measured reasons. (1) It is **serving-shaped**: it contains `rest-api` (`crates/eggsec/src/config/feature_registry.rs:502`), which the engine's own classification calls a `serving-surface` (`:470-471`) — a daemon that already serves the Unix socket would gain a second, unintended HTTP surface with its own `email-notifications` and `config-watch` closure. (2) It contains `logging-subscriber` (`:495`) and `config-watch` (`:486`), whose only consumers live inside the `rest-api`-gated `agent` module (`crates/eggsec/src/lib.rs:173-174`) — dead weight in a daemon. (3) It contains `mobile`/`mobile-dynamic` (`:497-498`), which no task kind requires and which pull `zip` into the shipped closure. Reusing `full` would also mean the daemon's feature set is **not** the GUI's requirement set, but a developer/lab superset. |
| **C — two profiles** (a system-dependency-free core and an extended one) | Two cargo features, e.g. `service` and `service-full` | **Rejected on measurement.** The split premise is "the extended profile needs system dependencies the core does not". §3.6 falsifies it: of the features the two profiles would differ over, only `nse` has a build-time dependency, and it is a **C toolchain**, not a system library. The real split in this codebase is **runtime privilege and OS**, not build dependency — `packet-inspection`, `stress-testing`, and `wireless-advanced` need Linux plus `CAP_NET_RAW`/`CAP_NET_ADMIN` for *live* use, and the engine **already reports that at runtime** through `crates/eggsec/src/platform/prereqs.rs:430-444` (`skip_reason_for`) and `:139-153` (`PlatformReport::collect`). Duplicating that split at the cargo-feature layer would create two lists to maintain, two recipes to document, and a false promise that the "core" profile runs on a host where the extended one does not — when the difference is actually a privilege bit. |
| **D — status quo** (ad-hoc `--features`) | Keep `full-executor = ["dep:eggsec", "eggsec/cli"]` and let operators pass engine features by hand | **Rejected.** The status quo is what makes M006's headline value unreachable today (§3.1), and it is not even *documentable* — an operator cannot know which ad-hoc combination is correct, because the requirement set is derivable from the catalog but derivable only by reading `crates/eggsec-policy/src/catalog.rs`. It also has no machine-checked list, so it cannot drift-check, and it produces a daemon whose feature set nobody wrote down. |

**Decision: Option A — one `service` profile.**

**The 20 included engine features** (exact list; `service` in
`crates/eggsec-daemon/Cargo.toml`):

| # | Engine feature | Inclusion rationale |
|---|---|---|
| 1 | `cli` | **Host gate, not a CLI surface.** Gates `commands`, `dispatch`, and `runtime_bridge` (`crates/eggsec/src/lib.rs:90-91`, `:104-105`, `:136-137`), which is where `EggsecRuntimeExecutor` lives. Without it the daemon cannot host the executor it is built around (§3.2). |
| 2 | `advanced-hunting` | Required by the catalog (`crates/eggsec-policy/src/catalog.rs:642`) for the `Hunt` task kind. |
| 3 | `c2` | Required by the catalog (`:574`) for the `C2` task kind. |
| 4 | `compliance` | Required by the catalog (`:676`) for the `Compliance` task kind. |
| 5 | `database` | Required by the catalog (`:693`) for the `Storage` task kind. |
| 6 | `db-pentest` | Required by the catalog (`:557`) for the `DbPentest` task kind. The **base** domain only; the drivers are separate markers (§3.10). |
| 7 | `evasion` | Transitively implied by `c2` (`crates/eggsec/Cargo.toml:423`), declared **explicitly** so the effective set equals the declared set. |
| 8 | `external-integrations` | Required by the catalog (`:718`) for the `Integrations` task kind. |
| 9 | `finding-workflow` | Required by the catalog (`:743`) for the `Workflow` task kind. |
| 10 | `headless-browser` | Required by the catalog (`:659`) for the `Browser` task kind. Runtime prerequisite: a Chrome/Chromium binary. |
| 11 | `nse` | Required by the catalog (`:536`) for the `Nse` task kind. Build prerequisite: a C toolchain for vendored OpenSSL (§3.6). |
| 12 | `packet-inspection` | Required by the catalog (`:468`) for `PacketCapture`, `PacketTraceroute`, and `PacketSend`. |
| 13 | `postex` | Transitively implied by `c2` (`crates/eggsec/Cargo.toml:423`), declared explicitly. |
| 14 | `stress-testing` | Required by the catalog (`:451`) for the `StressTest` task kind. |
| 15 | `tool-api` | Gates `pub mod tool` (`crates/eggsec/src/lib.rs:167-168`); transitively implied by `nse` (`crates/eggsec/Cargo.toml:329`), declared explicitly. |
| 16 | `vuln-management` | Required by the catalog (`:768`) for the `Vuln` task kind. |
| 17 | `web-proxy` | Required by the catalog (`:591`) for the `Intercept` task kind. |
| 18 | `websocket` | Changes the **behaviour** of the already-required `Fuzz` task kind: with it, the fuzzer opens a real WebSocket and runs live connection/origin/injection tests (`crates/eggsec/src/fuzzer/advanced.rs:428-456`); without it, it only emits static payloads (`:420-426`). |
| 19 | `wireless` | Required by the catalog (`:608`) for the `Wireless` and `WirelessActive` task kinds. |
| 20 | `wireless-advanced` | The **actual** compile gate for `WirelessActive` (`crates/eggsec/src/dispatch/canonical_execution.rs:1842-1855`); the catalog's separate `wireless-deauth` entry carries it (`:625`). Without it, `WirelessActive` degrades to passive `wireless` only. |

**The 30 excluded engine features**, each with the reason. "Engine rule" means
the engine's own `FULL_EXCLUDED_WITH_REASON` classification
(`crates/eggsec/src/config/feature_registry.rs:517-602`); "Daemon rule" means
a reason specific to a GUI-serving daemon.

| # | Excluded feature | Engine rule | Daemon-specific reason |
|---|---|---|---|
| 1 | `ai-integration` | — | **Measured dead path.** The only ways to install an AI component are `FuzzEngine::set_ai_generator` (`crates/eggsec/src/fuzzer/engine/core.rs:264`) and `set_ai_bypass` (`crates/eggsec/src/waf/mod.rs:138`) — **neither has a caller** in the engine. The only other consumer, `crate::ai::AiClient` (`crates/eggsec/src/commands/handlers/ai_analyze.rs:41`), lives in the `rest-api`-gated `agent` module (`crates/eggsec/src/lib.rs:173-174`). Enabling it would add `eventsource-stream` + `semver` for no reachable capability. |
| 2 | `api-schema` | `marker-only` (`:518-521`) | The daemon publishes the operation schema through `eggsec-daemon-protocol`'s `DescribeOperations`, not through the engine's `api_schema` module (`crates/eggsec/src/lib.rs:81-82`). |
| 3 | `cloud` | `marker-only` (`:530-533`) | No catalog entry, no task kind. |
| 4 | `config-watch` | `implicit-base` (`:486` in `full`; excluded reasons list `:517-602`) | Its only consumer is `crates/eggsec/src/agent/config_watcher.rs`, inside the `rest-api`-gated `agent` module (`crates/eggsec/src/lib.rs:173-174`). Excluding `rest-api` excludes it. |
| 5 | `container` | — | **Measured unreachable.** The `container` module has public APIs (`crates/eggsec/src/container/docker.rs:44`, `:86`; `crates/eggsec/src/container/cis.rs:43`) and `crates/eggsec/src/recon/containers.rs` is declared at `crates/eggsec/src/recon/mod.rs:81`, but **no handler and no catalog entry references it** — `crates/eggsec-policy/src/catalog.rs` has no `container` entry and `crates/eggsec/src/commands/handlers/` has no container handler. Enabling it would add `kube` + `k8s-openapi` for an unreachable capability. |
| 6 | `c2-mcp` | `exposure-marker` (`:526-529`) | MCP exposure is a different protocol surface; the daemon speaks the Unix socket. |
| 7 | `daemon-client` | `process-host` (`:534-537`) | The daemon **is** the host. It has no daemon to be a client of. |
| 8 | `db-pentest-mcp` | `exposure-marker` (`:538-541`) | Same as `c2-mcp`. |
| 9 | `db-pentest-mongodb` | `backend-driver` (`:542-545`) | The base `db-pentest` covers Postgres + MySQL through `eggsec-db-lab/db-drivers` (`crates/eggsec/Cargo.toml:376`; `crates/eggsec-db-lab/Cargo.toml` `db-drivers = ["dep:sqlx"]`). The MongoDB driver is an operator opt-in. |
| 10 | `db-pentest-mssql-tiberius` | `backend-driver` (`:546-549`) | Same, and this one additionally keeps `tiberius 0.12.3` → `tokio-rustls 0.24.1` → `rustls 0.21.12` out of the shipped daemon closure (§3.10). |
| 11 | `db-pentest-redis` | `backend-driver` (`:550-553`) | Same as `db-pentest-mongodb`. |
| 12 | `dynamic-plugins` | `output-mode` (`:554-557`) | A web-proxy extension mode with its own profile. |
| 13 | `full` | — (the aggregate itself) | A profile must not enable an aggregate; it would create a second, larger, undocumented feature set and a circular definition. |
| 14 | `git-secrets` | `marker-only` (`:558-561`) | No catalog entry, no task kind. |
| 15 | `grpc-api` | `serving-surface` (`:562-565`) | The daemon is the serving surface. It also requires `protoc` at build time (`crates/eggsec/build.rs:5-29`). |
| 16 | `insecure-tls` | `security-risk` (`:566-569`) | **Never.** Bypasses certificate verification. Absolute exclusion; it belongs in `DAEMON_SERVICE_FORBIDDEN`. |
| 17 | `logging-subscriber` | `full` member (`:495`) | The daemon installs its own `tracing-subscriber` (`crates/eggsec-daemon/Cargo.toml:31`). The engine's file-appender mode (`crates/eggsec/src/lib.rs:122-123`) is a CLI/TUI concern. |
| 18 | `mobile` | `full` member (`:497`) | No mobile task kind; the catalog has no mobile entry. Excluding it keeps `zip 2.4.2` out of the shipped closure (§3.10). |
| 19 | `mobile-dynamic` | `full` member (`:498`) | Same, and it is a `platform-mode` domain (`docs/PLATFORM.md:18`) whose live path needs `adb` and a lab device. |
| 20 | `nse-sandbox` | `platform-mode` (`:570-573`) | An NSE hardening mode with its own profile; it is not what the `Nse` task kind needs. |
| 21 | `nse-ssh2` | `platform-mode` (`:574-577`) | Requires the system `libssh2` library (`libssh2-sys` → system link, `Cargo.lock:5041-5048`; `scripts/check-features-individual.sh:86`). Excluding it keeps a system-library prerequisite out of the daemon build. |
| 22 | `pdf` | `output-mode` (`:578-581`) | The daemon produces no reports — `TaskResultEnvelope.payload` is always `json!({})` (`crates/eggsec/src/dispatch/canonical_execution.rs:1111-1116`). Report rendering is M006's separate read path. |
| 23 | `rest-api` | `serving-surface` (`:562-565`, and a `full` member at `:502`) | The daemon already serves the Unix socket; a second HTTP surface would also pull `email-notifications` and `config-watch` in through `rest-api` (`crates/eggsec/Cargo.toml:311`). |
| 24 | `sbom` | `full` member (`:503`) | SBOM is a CLI-only path (`crates/eggsec/src/commands/handlers/sbom.rs`); there is no SBOM task kind and no catalog entry. |
| 25 | `test-helpers` | `test-only` (`:582-585`) | Never in a shipped aggregate. Absolute exclusion. |
| 26 | `transparent-proxy` | `output-mode` (`:590-593`) | A web-proxy extension mode with its own profile. |
| 27 | `udp-scan` | `platform-mode` (`:522-525`) | UDP verdicts depend on an ICMP receiver whose availability is OS- and privilege-dependent; no catalog entry requires it. |
| 28 | `web-proxy-mcp` | `exposure-marker` (`:594-597`) | Same as `c2-mcp`. |
| 29 | `ws-api` | `serving-surface` (`:598-601`) | The daemon is the serving surface. |
| 30 | `email-notifications` | `full` member (`:494`) | Its only consumer is `crates/eggsec/src/agent/alerts/routing.rs` (`lettre`), inside the `rest-api`-gated `agent` module. |

**Count check: 20 included + 30 excluded = 50**, matching
`ALL_FEATURES` exactly. The conformance test asserts both halves, so a new
engine feature cannot be added to the registry without being classified as
included or excluded by the profile — the same discipline the engine already
applies to `full`.

**The inclusion rule, stated so it can be machine-checked.**

> An engine feature is **included** iff a code path reachable from one of the
> 30 `TaskKind` variants is `cfg`-gated on it, **or** it is the `cli` host
> gate, **or** it is a transitive dependency of an included feature and is
> declared explicitly so the effective set equals the declared set.

Every inclusion in the table above satisfies this rule, and the exclusions are
either the engine's own classified exclusions or a **measured** unreachability
(§3.5, the table's "measured dead path" / "measured unreachable" rows). The
rule is checkable: `required_profile_features()` derives the required half
from the catalog, and the conformance checker rejects anything that is neither
required nor an explicitly-declared transitive member.

**`DAEMON_SERVICE_FORBIDDEN`** — the absolute subset (10 entries), any of
which in a profile is a **hard** conformance failure:
`insecure-tls`, `test-helpers`, `full`, `rest-api`, `grpc-api`, `ws-api`,
`db-pentest-mcp`, `web-proxy-mcp`, `c2-mcp`, `daemon-client`.

Acceptance evidence:

- The option table above exists in the implementation PR description, with the
  measurements that decided it.
- `DAEMON_SERVICE_PROFILE` has exactly 20 entries; `DAEMON_SERVICE_EXCLUDED`
  has exactly 30; the two partition `ALL_FEATURES`.
- Every one of the 20 inclusions cites a `file:line` in the table, and every
  one of the 30 exclusions cites either the engine's own
  `FULL_EXCLUDED_WITH_REASON` reason or a measured unreachability.

---

### Work package B — Implement the profile contract in the engine

Intent:

Make the profile a **data contract the engine owns**, so the daemon's
manifest, the documentation, and the compiled binary can all be checked
against one list.

Required changes:

- Add `crates/eggsec/src/config/daemon_profile.rs` with the items listed in
  §6 "Core/domain".
- Implement `required_profile_features()` **by reading
  `eggsec_policy::ALL_OPERATION_METADATA`** and collecting the distinct
  non-empty `required_features`, then unioning the `cli` host gate and the
  explicitly-declared transitive members. It MUST NOT hard-code the 15
  catalog-derived names.
- Implement `profile_violations()` as a **pure** function over a candidate
  list. It returns a violation for: an unknown name (not in
  `eggsec::config::ALL_FEATURES`); a forbidden name; a missing required
  name; a duplicate; and a name that is neither required nor an explicitly
  declared transitive member.
- Export the module's public items from `crates/eggsec/src/config/mod.rs`.
- The module is compiled **unconditionally** (no `cfg` gate), so the
  conformance test also runs in the `--no-default-features` build.
- Add a doc comment to `crates/eggsec/src/config/feature_registry.rs:455-474`
  recording that `FULL_EXCLUDED_WITH_REASON` is scoped to `full` and that a
  second aggregate exists with its own contract, so a future reader does not
  assume the exclusion list is a global policy.

Acceptance evidence:

- `cargo check -p eggsec --no-default-features` is green with the new module.
- The module's unit tests cover: the happy path (the profile has no
  violations); an unknown name; a forbidden name; a missing required name; a
  duplicate; and a name that is neither required nor transitive.
- `required_profile_features()` returns exactly the 15 catalog-derived names
  plus the gate and the transitive members, and a test asserts that the
  catalog-derived half is **exactly** the distinct non-empty
  `required_features` of `ALL_OPERATION_METADATA` — so a new catalog entry
  with a `required_features` value fails the test until the profile is
  updated.

---

### Work package C — Publish the profile as a daemon cargo feature

Intent:

Make the profile **buildable**: one cargo feature on `crates/eggsec-daemon`
that produces the profile daemon.

Required changes:

In `crates/eggsec-daemon/Cargo.toml`:

```toml
[features]
default = []
http-api = ["dep:axum", "dep:async-stream", "dep:futures"]
# The GUI-serving daemon profile: the engine features a daemon that serves
# the desktop GUI enables. The list is owned by the engine
# (`eggsec::config::daemon_profile::DAEMON_SERVICE_PROFILE`) and pinned by
# `crates/eggsec/tests/feature_matrix.rs`.
service = [
  "dep:eggsec",
  "eggsec/advanced-hunting",
  "eggsec/c2",
  "eggsec/cli",
  "eggsec/compliance",
  "eggsec/database",
  "eggsec/db-pentest",
  "eggsec/evasion",
  "eggsec/external-integrations",
  "eggsec/finding-workflow",
  "eggsec/headless-browser",
  "eggsec/nse",
  "eggsec/packet-inspection",
  "eggsec/postex",
  "eggsec/stress-testing",
  "eggsec/tool-api",
  "eggsec/vuln-management",
  "eggsec/web-proxy",
  "eggsec/websocket",
  "eggsec/wireless",
  "eggsec/wireless-advanced",
]
# Deprecated alias for `service`. Kept so every existing invocation and
# documentation reference keeps working; it now produces the profile daemon.
full-executor = ["service"]
```

- Switch the three `#[cfg]` gates listed in §6 "Runtime and concurrency" from
  `full-executor` to `service`.
- Update the `#[cfg(not(...))]` warning text at
  `crates/eggsec-daemon/src/main.rs:112` to name the `service` feature.
- Update the `--full-executor` flag's help text at
  `crates/eggsec-daemon/src/main.rs:40-41` so it no longer implies the flag
  is tied to a cargo feature of the same name. The **flag's name and meaning
  are unchanged**.

Acceptance evidence:

- `cargo check -p eggsec-daemon --features service` is green.
- `cargo check -p eggsec-daemon --features full-executor` is green and
  compiles the **same** code path (the alias resolves to `service`).
- `cargo check -p eggsec-daemon` (no features) is green and links no engine.
- The daemon's `--help` output still shows `--full-executor`, and the GUI's
  existing tests (`crates/eggsec-gui/src/connection.rs:206`, `:213`;
  `crates/eggsec-gui/src/config.rs:140`) still pass **unmodified**.

---

### Work package D — Map the profile onto system prerequisites

Intent:

Give the operator an honest, measured answer to "what must I install before I
build this, and what will still not run live afterwards?".

Required changes:

- Extend `docs/DAEMON_FEATURE_PROFILE.md` with a prerequisite table built
  from `crates/eggsec/src/platform/` — one row per profile feature that has
  a prerequisite, each row citing the domain id the engine's own
  `report_for`/`skip_reason_for` uses
  (`crates/eggsec/src/platform/prereqs.rs:407-444`) and the corresponding
  `docs/PLATFORM.md` row.
- The table MUST distinguish three categories, and MUST NOT conflate them:
  1. **Build-time** requirements — currently only the C toolchain (`cc`,
     `perl`, `make`) for vendored OpenSSL under `nse`, and a Chrome/Chromium
     binary is *not* build-time (it is runtime-only).
  2. **Runtime-privilege** requirements — Linux plus `CAP_NET_RAW` for
     `packet-inspection` and `stress-testing`; Linux plus `CAP_NET_ADMIN`,
     monitor mode, and `--allow-active-wireless` for `wireless-advanced`.
  3. **Runtime-binary** requirements — `iwlist` (wireless-tools) for
     `wireless`; Chrome/Chromium for `headless-browser`; `adb` (optional) and
     a lab device for `mobile-dynamic` (excluded from the profile).
- **Correct the two stale prerequisite claims** (§3.6) in `docs/PLATFORM.md`,
  `docs/FEATURE_MATRIX.md`, and `crates/eggsec/src/platform/prereqs.rs`:
  - `packet-inspection` / `stress-testing` do **not** require `libpcap-dev`.
    The engine uses pure-Rust `pnet` (`crates/eggsec/Cargo.toml:323`, `:326`),
    and the sweep script already states this
    (`scripts/check-features-individual.sh:140-143`). The stale strings at
    `scripts/check-features-individual.sh:87-88` (`PREREQ_PACKET`,
    `PREREQ_STRESS`) are corrected to match — but **only** after confirming
    the sweep still compiles those features on a host without libpcap.
  - `nse` does **not** require `libssl-dev`; OpenSSL is **vendored**
    (`eggsec-nse 0.2.0` the vendored-OpenSSL registry entry `~/.cargo/registry/src/index.crates.io-*/eggsec-nse-0.2.0/Cargo.toml:221-224`; `Cargo.lock:6101-6125`). The
    real requirement is a C toolchain.
- Add a **prerequisite probe** to the build script that reports, for the
  host, which of the profile's runtime domains are live-capable — by calling
  the engine's own logic rather than reimplementing it. The probe is
  **advisory**: it reports, it does not block the build.

Acceptance evidence:

- The prerequisite table's rows are traceable to
  `crates/eggsec/src/platform/prereqs.rs:391-403` and `docs/PLATFORM.md:16-23`
  by citation.
- The probe's output on a macOS host names `packet-inspection`,
  `stress-testing`, and `wireless-advanced` as not live-capable, using the
  engine's own `skip_reason_for` wording.
- The corrected prerequisite strings no longer name `libpcap-dev` for
  `packet-inspection`/`stress-testing` or `libssl-dev` for `nse`, and the
  sweep still passes on a host without either library.

---

### Work package E — Produce the reproducible build recipe

Intent:

Make "build the GUI-serving daemon" a **repeatable, recorded** act rather
than a remembered cargo command.

Required changes:

- Add `scripts/build-daemon-profile.sh`, which:
  - refuses to run on a dirty tree unless `--allow-dirty` is passed, and
    records the dirty flag either way;
  - takes `--profile <name>` (currently only `service`) and `--target
    <triple>` and `--out <dir>`;
  - pins the invocation to `cargo build --release --locked -p eggsec-daemon
    --features <profile> --target <triple>`;
  - records a metadata sidecar next to the binary containing: the commit
    SHA, the dirty flag, the toolchain string (`rustc -Vv`), the **resolved
    toolchain version and the MSRV floor (`1.89`)**, the target triple, the
    **resolved feature set** (the engine features the build enabled, read
    from the same source of truth the conformance test uses), the
    `Cargo.lock` hash, the binary size, and its SHA-256;
  - prints the runtime invocation the operator should then use:
    `eggsec-daemon --full-executor --socket-path <path>` — **the runtime flag
    is unchanged** (§3.7);
  - performs **no** signing, **no** bundling, and **no** packaging.
- Add a `make check-daemon-profile` target that runs the profile's static and
  runtime conformance tests plus a `cargo check` of the profile build, and
  add the same target to `make check-feature-profiles` (the deep-check
  representative-profile target, `Makefile:206-228`).
- Add `make build-daemon-profile` as the operator-facing wrapper.
- The recipe MUST NOT introduce an npm step, a code-signing identity, an
  installer, or an updater — those are outside this milestone (§5).

**The exact recipe, per profile.**

| Field | Value |
|---|---|
| Package | `eggsec-daemon` |
| Command | `cargo build --release --locked -p eggsec-daemon --features service --target <triple>` |
| Toolchain | `rust-toolchain.toml` pins `channel = "stable"` with `rustfmt` + `clippy`; the **MSRV floor** is `rust-version = "1.89"` (`Cargo.toml:39`), verified by `make check-msrv` (`Makefile:128-143`). The recipe records `rustc -Vv` and refuses a toolchain older than 1.89. |
| Target | The host triple by default; `--target` overrides. `ADR-0010` scopes the GUI to **Linux and macOS**; **Windows is out of scope** because the daemon's transport is a Unix domain socket. |
| Features | `service` (the profile). `full-executor` is accepted as a deprecated alias. `http-api` is **not** part of the profile. |
| Lockfile | `--locked`: the build must not float dependency versions. |
| Output | A caller-supplied directory **outside the source tree** by default, plus the metadata sidecar. |
| Prereqs | A C toolchain (`cc`, `perl`, `make`) for vendored OpenSSL under `nse`; a Rust toolchain ≥ 1.89. |

Acceptance evidence:

- Running the script twice from a clean tree at the same commit produces the
  same binary SHA-256, and the sidecar's `Cargo.lock` hash matches the
  repository's.
- The sidecar's recorded feature set equals `DAEMON_SERVICE_PROFILE`.
- The script refuses a dirty tree without `--allow-dirty` and records the
  dirty flag when it is passed.
- `make check-daemon-profile` is green.

---

### Work package F — Machine-checked conformance, guards, and documentation

Intent:

Make the docs, the manifest, and the compiled binary **provably** the same
thing, so this milestone cannot rot.

Required changes:

- Add the **static** conformance tests to `crates/eggsec/tests/feature_matrix.rs`
  (see §10 for the exact assertions).
- Add the **runtime** conformance test to
  `crates/eggsec-daemon/tests/feature_profile.rs`, gated `#![cfg(feature =
  "service")]`.
- Add the `docs/DAEMON_FEATURE_PROFILE.md` machine-parseable table, and a
  test that parses it and asserts set equality with
  `DAEMON_SERVICE_PROFILE`.
- Update guard 104's daemon literal and add **guard 163** (§6
  "Documentation and static guards").
- Update `scripts/check-features-individual.sh` and the pinned profile list
  in `crates/eggsec/tests/feature_matrix.rs:861-877`.
- Apply the documentation updates in §12.

Acceptance evidence:

- Every test in §10 exists and passes.
- Guard 104 and the new guard 163 are green.
- A deliberate edit that adds `rest-api` to the daemon's `service` feature
  makes the static conformance test fail with a message naming `rest-api`.
- A deliberate edit that removes `nse` from the daemon's `service` feature
  makes the static conformance test fail with a message naming `nse` as a
  missing required feature.
- A deliberate edit that adds a row to the documentation table without adding
  the feature to the cargo feature makes the doc-parity test fail.

---

## 8. Failure, cancellation, restart, and contention semantics

M008 changes **what the daemon can run**, not how it runs. The following
semantics are therefore unchanged, and the milestone must not perturb them.

| Concern | Behaviour | Why it is unaffected |
|---|---|---|
| **Partial feature availability** | A daemon built with the `service` profile runs every task kind whose catalog `required_features` are in the profile — that is, all 30 — subject to the runtime prerequisites in §3.6. A task whose domain is not live-capable on the host fails with the engine's own `FeatureMissing` denial, naming the missing prerequisite. | The profile changes the compiled set, not the denial path. `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate. |
| **Duplicate delivery** | Unchanged. A resubmitted `RunRequest` produces a new task in the daemon's task table. | The daemon's request handling is untouched. |
| **Process restart** | Unchanged. The daemon recovers persisted sessions (`crates/eggsec-daemon/src/main.rs` recovery path) and the profile is a compile-time property, so a restarted daemon has exactly the same feature set. | The profile is not persisted, not negotiated, and not reported. |
| **Cancellation races** | Unchanged. Cancellation goes through `eggsec-runtime::race_with_cancel`; the profile does not alter task lifecycle. | No runtime change. |
| **Stale generations** | Unchanged. | No runtime change. |
| **Concurrent callers** | Unchanged. `WorkerConfig::max_concurrency` remains the real capacity contract; the profile does not add a fan-out path. | No runtime change. |
| **Spawned tokio tasks** | Unchanged. Any task the daemon spawns keeps a 30–300s timeout wrapper. | No runtime change. |
| **Build failure on a host without a C toolchain** | The `nse` feature's vendored-OpenSSL build fails with `openssl-sys`'s own error. The recipe's prerequisite probe (§ Work package D) reports this **before** the build and names the fix. | The probe is advisory; the build still fails loudly if the probe was skipped. |
| **Build failure from the inherited `--all-features` defect** | `crates/eggsec/src/fuzzer/advanced.rs:446` fails to compile when `websocket` is enabled **and** `--all-features` is on, because of the private-field defect at `crates/eggsec-payloads/src/websocket.rs:43`. The profile enables `websocket`, so `cargo check --workspace --all-features` fails. **This is inherited (§3.11) and is a stop condition (§14), not a milestone deliverable.** | The defect is in the payload crate's field visibility, not in the profile. |

---

## 9. Compatibility and migration

### 9.1 Build compatibility

| Caller | Before | After | Compatible? |
|---|---|---|---|
| `cargo build -p eggsec-daemon` | no engine | no engine | **Yes**, unchanged. |
| `cargo build -p eggsec-daemon --features full-executor` | engine with `{cli}` | engine with the 20-feature profile | **Yes** — same command, strictly more capability. This is the intended improvement. |
| `cargo build -p eggsec-daemon --features service` | *did not exist* | engine with the 20-feature profile | **New.** |
| `cargo build -p eggsec-daemon --features http-api` | unchanged | unchanged | **Yes.** |
| `cargo check -p eggsec-daemon --features full-executor` (the sweep, `scripts/check-features-individual.sh:216`) | compiles `{cli}` | compiles the profile | **Yes**, but **slower**: the profile pulls `nse`, which builds vendored OpenSSL. The sweep already compiles `eggsec/nse` (`scripts/check-features-individual.sh:193`) with the same prerequisite, so **no new prerequisite class** is introduced. |
| `crates/eggsec/tests/feature_matrix.rs:868` (the pinned sweep profile list) | `"eggsec-daemon --features full-executor"` | unchanged, plus `"eggsec-daemon --features service"` | **Yes**, after the test is updated. |

### 9.2 Runtime, configuration, and documentation compatibility

**Runtime — none required.** The daemon's wire protocol, its runtime flags, its
socket-path convention, its persistence, and its session semantics are all
unchanged. A daemon built with the profile speaks exactly the protocol it spoke
before; it simply answers more operations with real results instead of denying
them.

**Configuration — none.** The profile is a compile-time property, selected at
build time. There is no runtime configuration knob, no environment variable, and
no config-file entry. This is deliberate: a runtime-selectable feature set
would be unpublishable, because a feature that is compiled out cannot be turned
on at runtime.

**Documentation.** `architecture/daemon.md:25`, `:30`, `:398`,
`architecture/overview.md:70`, `:219`, and `crates/eggsec-daemon/Cargo.toml:16-17`
all describe the daemon's feature surface. All are updated in the same change,
so no document describes a feature that no longer exists.

### 9.3 Removal criteria

`full-executor` as a cargo feature is **deprecated, not removed** in this
milestone. Removal criteria: after the profile has shipped and the recipe has
been published, a follow-up may remove the alias **iff** every in-repo
reference (`scripts/check-features-individual.sh:216`,
`crates/eggsec/tests/feature_matrix.rs:868`, `architecture/daemon.md`,
`architecture/overview.md`) has been updated and no external operator recipe
in the wild is known to use it. The **runtime flag** `--full-executor` is
never removed by this milestone — it is a different mechanism (§3.7) and the
GUI's guidance names it.

---

## 10. Required tests

### Focused unit tests

**In `crates/eggsec/src/config/daemon_profile.rs` (the module's own
`#[cfg(test)]` block):**

| Test | Asserts |
|---|---|
| `the_documented_profile_has_no_violations` | `profile_violations(DAEMON_SERVICE_PROFILE)` is empty. |
| `the_documented_profile_excludes_nothing_it_claims_to_exclude` | `DAEMON_SERVICE_PROFILE` and `DAEMON_SERVICE_EXCLUDED` are disjoint and their union equals every name in `eggsec::config::ALL_FEATURES`. |
| `the_required_set_is_derived_from_the_policy_catalog` | The catalog-derived half of `required_profile_features()` equals exactly the distinct non-empty `required_features` of `eggsec_policy::ALL_OPERATION_METADATA` — **not** a hand-written list. |
| `the_required_set_contains_the_cli_host_gate` | `required_profile_features()` contains `cli`, and `cli` is not a catalog `required_features` value. |
| `the_documented_profile_covers_the_required_set` | Every name in `required_profile_features()` is in `DAEMON_SERVICE_PROFILE`. |
| `the_transitive_members_are_declared_explicitly` | `postex`, `evasion`, and `tool-api` are in `DAEMON_SERVICE_PROFILE`, so the effective set equals the declared set. |
| `the_profile_enables_no_forbidden_feature` | `DAEMON_SERVICE_PROFILE` is disjoint from `DAEMON_SERVICE_FORBIDDEN`. |

**In `crates/eggsec/tests/feature_matrix.rs` (the static conformance
tests — these run under `make check`):**

| Test | Asserts |
|---|---|
| `daemon_service_profile_matches_the_documented_list` | Parse `crates/eggsec-daemon/Cargo.toml` with the existing `parse_features_raw` helper (`:428-463`), extract the `service` feature's `eggsec/<name>` entries with the existing `extract_engine_features` helper (`:464-475`), and assert **exact set equality** with `eggsec::config::daemon_profile::DAEMON_SERVICE_PROFILE` — no more, no fewer. |
| `daemon_service_profile_features_are_all_known_engine_features` | Every `eggsec/<name>` the daemon forwards is present in `eggsec::config::ALL_FEATURES` (the same check `cli_forwarded_features_exist_in_registry` performs at `:478-493`, applied to the daemon). |
| `daemon_service_profile_covers_every_required_feature` | Every name in `required_profile_features()` is in the daemon's `service` feature. |
| `daemon_service_profile_does_not_enable_cli_shaped_serving_features` | The daemon's `service` feature contains **none** of `rest-api`, `grpc-api`, `ws-api`, `config-watch`, `logging-subscriber`, `email-notifications`, `daemon-client`, `full`. (These are the *serving-surface* and *process-host* shapes. `cli` itself **is** present, and the test's doc comment states why — §3.2 — so a future reader does not "fix" it by removing `cli`.) |
| `daemon_service_profile_does_not_enable_security_risk_or_test_features` | The daemon's `service` feature contains neither `insecure-tls` nor `test-helpers`. |
| `daemon_service_profile_does_not_enable_exposure_markers` | The daemon's `service` feature contains none of `db-pentest-mcp`, `web-proxy-mcp`, `c2-mcp`. |
| `daemon_service_profile_does_not_enable_the_aggregate` | The daemon's `service` feature does not contain `full`. |
| `daemon_full_executor_is_a_deprecated_alias_for_service` | The daemon's `full-executor` feature is exactly `["service"]`. |
| `daemon_profile_documentation_matches_the_manifest` | Parse the machine-parseable table in `docs/DAEMON_FEATURE_PROFILE.md` and assert its feature-name column is **exactly** `DAEMON_SERVICE_PROFILE`. |
| `daemon_profile_conformance_checker_rejects_a_forbidden_feature` | **Negative.** `profile_violations(&["rest-api"])` is non-empty and names `rest-api`. |
| `daemon_profile_conformance_checker_rejects_an_unknown_feature` | **Negative.** `profile_violations(&["not-a-real-feature"])` is non-empty. |
| `daemon_profile_conformance_checker_rejects_a_missing_required_feature` | **Negative.** `profile_violations(&DAEMON_SERVICE_PROFILE minus "nse")` is non-empty and names `nse`. |
| `daemon_profile_conformance_checker_rejects_a_duplicate` | **Negative.** `profile_violations(&[...DAEMON_SERVICE_PROFILE, "nse"])` is non-empty. |
| `daemon_profile_conformance_checker_accepts_the_documented_profile` | **Positive control** for the three negative tests above — the checker is not simply always non-empty. |

### Integration tests

**In `crates/eggsec-daemon/tests/feature_profile.rs`, gated
`#![cfg(feature = "service")]`:**

| Test | Asserts |
|---|---|
| `the_compiled_engine_feature_set_equals_the_documented_profile` | `eggsec::policy_bridge::features::current_enabled_features()` — the engine's runtime truth (`crates/eggsec/src/policy_bridge/features.rs:11-17`) — contains **exactly** the names in `DAEMON_SERVICE_PROFILE`: every documented name is present, and no name outside the documented set is present. This is the test that satisfies "the profile's compiled feature set MUST equal the documented list". |
| `the_compiled_feature_set_is_fail_closed_on_unknown_names` | `current_enabled_features().contains("not-a-real-feature")` is `false` (`crates/eggsec-policy/src/features.rs:44-49`). |
| `the_compiled_feature_set_covers_the_required_features` | Every name in `required_profile_features()` is present in the compiled set. |
| `the_compiled_feature_set_excludes_the_forbidden_features` | No name in `DAEMON_SERVICE_FORBIDDEN` is present in the compiled set. |
| `the_engine_snapshot_agrees_with_the_registry` | The existing `snapshot_matches_registry_oracle` invariant holds for the profile build — i.e. the registry's `enabled` flags agree with the compiled set. |
| `the_daemon_hosts_the_engine_executor_under_the_service_profile` | The daemon's `#[cfg(feature = "service")]` branch is live: the `main.rs` executor branch compiles and `DaemonHost::new(config, eggsec::runtime_bridge::EggsecRuntimeExecutor::new(), store)` is constructible — the same construction the existing `full_executor_tests` module uses (`crates/eggsec-daemon/src/host.rs:2862-2872`), moved under the `service` gate. |

**The per-prerequisite test, using `crates/eggsec/src/platform/`:**

| Test | Asserts |
|---|---|
| `every_profile_domain_has_a_platform_prerequisite_row` | For each domain id in `crates/eggsec/src/platform/prereqs.rs:391-403` (`mobile-dynamic`, `packet-inspection`, `wireless`, `wireless-advanced`, `stress-testing`, `nse`), `report_for(domain).features` is non-empty, and each listed feature is either in the profile or is a **documented** exclusion with a rationale. This pins the profile↔platform-matrix relationship in both directions. |
| `the_profile_platform_rows_are_documented` | `docs/DAEMON_FEATURE_PROFILE.md`'s prerequisite table contains one row per domain id in `all_domain_ids()`, with the domain id spelled exactly. |
| `the_live_capability_probe_reports_engine_skip_reasons` | On the executing host, `skip_reason_for(domain)` — for each domain in the profile — either is `None` (live-capable) or begins with `"SKIP {domain}:"`, matching the engine's own wording (`crates/eggsec/src/platform/prereqs.rs:430-444`). The test asserts the **shape**, not the host-specific outcome, so it is deterministic on any host. |
| `the_profile_does_not_claim_live_capability_for_unmet_prerequisites` | The profile document does not assert live capability for `packet-inspection`, `stress-testing`, or `wireless-advanced` without naming `CAP_NET_RAW` / `CAP_NET_ADMIN` and the Linux-only constraint. |

### Restart, recovery, contention, and cancellation tests

**Restart and recovery.** M008 adds no runtime state, so no new restart test is
required; the existing daemon restart-recovery tests (`crates/eggsec-daemon/src/host.rs`
recovery path) continue to pass unchanged. One assertion is added to the
profile suite for completeness:

| Test | Asserts |
|---|---|
| `the_profile_is_a_build_time_property_not_a_runtime_one` | The daemon's persisted store contains **no** record of the enabled-feature set — i.e. the feature set is not persisted, negotiated, or reported. This is the property that makes the profile a build recipe rather than a protocol change, and it guards M006's ownership of the wire-side publication. |

**Contention and cancellation.** None required. M008 changes no concurrency path
(§8). The existing daemon concurrency tests are unaffected and MUST stay green.

### Security and negative tests

| Test | Asserts |
|---|---|
| `a_profile_that_enables_a_forbidden_feature_fails_the_conformance_check` | **The central negative test.** A synthetic candidate list containing `insecure-tls` is rejected by `profile_violations`, and the violation names the feature and the reason. |
| `a_profile_that_enables_a_serving_surface_fails_the_conformance_check` | A synthetic list containing `rest-api` is rejected. |
| `a_profile_that_enables_an_exposure_marker_fails_the_conformance_check` | A synthetic list containing `db-pentest-mcp` is rejected. |
| `a_profile_that_enables_the_aggregate_fails_the_conformance_check` | A synthetic list containing `full` is rejected. |
| `a_profile_missing_a_required_feature_fails_the_conformance_check` | A synthetic list with `nse` removed is rejected and names `nse`. |
| `a_profile_with_an_unknown_feature_fails_the_conformance_check` | A synthetic list containing `totally-fake-feature-xyz` is rejected — the checker is fail-closed on unknown names, mirroring `EnabledFeatures::contains` (`crates/eggsec-policy/src/features.rs:44-49`). |
| `the_daemon_profile_does_not_enable_insecure_tls` | The daemon's compiled feature set never contains `insecure-tls`. |

### Migration and compatibility tests

| Test | Asserts |
|---|---|
| `the_full_executor_alias_builds_the_same_profile` | `cargo check -p eggsec-daemon --features full-executor` and `--features service` resolve to the same engine feature set. Asserted **statically** (the alias's literal is `["service"]`) and **at runtime** by the alias test in `crates/eggsec/tests/feature_matrix.rs`. |
| `the_default_daemon_build_still_links_no_engine` | `crates/eggsec-daemon/Cargo.toml`'s `default` is `[]` and the engine dep is still `optional = true` — the same assertion guard 16 makes (`scripts/check-architecture-guards.sh:273`). |
| `the_daemon_runtime_flag_is_unchanged` | The daemon's `--help` output still contains `--full-executor`, and the GUI's existing assertions (`crates/eggsec-gui/src/connection.rs:206`, `:213`; `crates/eggsec-gui/src/config.rs:140`) still pass **without modification**. |
| `the_engine_full_aggregate_is_unchanged` | `FULL_MEMBERS.len() == 28` and `full_exclusions_are_documented` still pass — the existing assertions at `crates/eggsec/tests/feature_matrix.rs:596-691`, unmodified. |
| `the_protocol_version_is_unchanged` | `crates/eggsec-daemon-protocol`'s protocol version is still 2 and no `DaemonCapabilities` field was added — asserted by the existing protocol tests, unmodified. |

---

## 11. Required verification commands

```bash
make check                  # mandatory Rust contract (Makefile:154-182)
make check-daemon-profile   # new: the profile's static + runtime conformance tests
make check-feature-profiles # deep check: gains the daemon profile (Makefile:206-228)
make check-features-individual  # deep check: the sweep now compiles the profile
make check-deps             # dependency policy only (expected: inherited failure, §3.11)
make check-msrv             # toolchain floor is 1.89 (Makefile:128-143)
```

Targeted commands the implementer should also run and record:

```bash
cargo check -p eggsec-daemon
cargo check -p eggsec-daemon --features service
cargo check -p eggsec-daemon --features full-executor
cargo test -p eggsec --features cli --test feature_matrix
cargo test -p eggsec-daemon --features service --test feature_profile
bash scripts/check-architecture-guards.sh
bash scripts/build-daemon-profile.sh --profile service --out /tmp/eggsec-daemon-dist
```

**Do not claim commands that were not actually run** in the closure record.

**Known-failing commands at the baseline, recorded as inherited (§3.11):**

| Command | Expected result |
|---|---|
| `make check-deps` | **Fails** on the advisories listed in §3.11. |
| `cargo check --workspace --all-features` | **Fails** with `E0616: field 'url' of struct 'WebSocketFuzzer' is private` (`crates/eggsec/src/fuzzer/advanced.rs:446` vs `crates/eggsec-payloads/src/websocket.rs:43`). |

Both MUST be reported as inherited in the closure record, not silently
skipped and not "fixed" under this milestone.

---

## 12. Documentation updates

| Target | Change | Owner |
|---|---|---|
| `docs/DAEMON_FEATURE_PROFILE.md` (new) | The profile: the included set with rationales, the excluded set with rationales, the prerequisite map, the build recipe, and a **machine-parseable** table (the conformance test parses it). | M008 |
| `crates/eggsec/src/config/feature_registry.rs:455-474` | One paragraph: `FULL_EXCLUDED_WITH_REASON` is scoped to `full`; a second aggregate (`service`) exists with its own contract; the registry remains the single source of truth. | M008 |
| `crates/eggsec/src/config/daemon_profile.rs` (new) | Module doc: the inclusion rule, the relation to `full`, the relation to the catalog, and the "the profile is a build-side contract; M006 owns the wire-side publication" boundary. | M008 |
| `crates/eggsec-daemon/Cargo.toml:16-18` | The feature block's comments: what `service` is, why `cli` is mandatory, and that `full-executor` is a deprecated alias. | M008 |
| `crates/eggsec-daemon/src/main.rs:40-43` | The `--full-executor` flag's help text: it selects the engine executor; it no longer implies a same-named cargo feature. The flag's name and meaning are unchanged. | M008 |
| `architecture/daemon.md:18`, `:25`, `:30`, `:334`, `:398` | The daemon's feature column: `service` (the profile) and `full-executor` (deprecated alias); the `EggsecRuntimeExecutor` note now names the `service` gate. | M008 |
| `architecture/overview.md:70`, `:219` | The daemon's one-line feature summary. | M008 |
| `docs/PLATFORM.md:19`, `:23` | **The two stale prerequisite claims**: `packet-inspection`/`stress-testing` do not need `libpcap-dev`; `nse` does not need `libssl-dev` (OpenSSL is vendored; a C toolchain is required). | M008 |
| `docs/FEATURE_MATRIX.md:333`, `:334` | The same two corrections. | M008 |
| `crates/eggsec/src/platform/prereqs.rs:549`, `:722` | The same two stale prerequisite strings, so the runtime skip reason does not name a prerequisite that does not exist. | M008 |
| `scripts/check-features-individual.sh:87-88`, `:140-143`, `:216` | Correct `PREREQ_PACKET` / `PREREQ_STRESS`; keep the pure-Rust note; add the `--features service` profile entry and the C-toolchain note. | M008 |
| `docs/DEPENDENCY_EXCEPTIONS.md` | **No new entry** — M008 adds no dependency. If an implementer adds one, this file and `deny.toml` MUST gain an owner + review-by date, and §14 applies. | — |
| `plans/registry.md` | **Not edited by this milestone's implementation** — the registry already carries the M008 row as *planned*; the closure record updates it. | closure |
| `plans/subsystems/desktop-gui-frontend-roadmap.md` | **Not edited by this milestone's implementation.** | closure |

**Skills.** The `eggsec-daemon` skill (`.opencode/skills/`) and the
`eggsec-gui` skill MUST be checked for claims that the daemon's `full-executor`
enables the engine's feature set. Any such claim is stale after this
milestone and MUST be corrected alongside the architecture docs
(`AGENTS.md`, "Skills drift").

---

## 13. Acceptance criteria

| # | Criterion | Evidence |
|---|---|---|
| A1 | `crates/eggsec-daemon` declares a cargo feature `service` that enables an **explicit** list of engine features, and `full-executor` is a deprecated alias for it. | `crates/eggsec-daemon/Cargo.toml`; the static conformance tests |
| A2 | The profile's enabled set is exactly 20 engine features, and the profile's excluded set is exactly the other 30 — the two partition the engine's 50 registry entries. | `daemon_service_profile_matches_the_documented_list`; `the_documented_profile_excludes_nothing_it_claims_to_exclude` |
| A3 | Every included feature has a **per-feature rationale** naming the task kind or the measured fact that requires it; every excluded feature has a **per-exclusion rationale** naming the engine's own classified reason or a measured unreachability. | `docs/DAEMON_FEATURE_PROFILE.md`; the tables in Work package A |
| A4 | The required set is **derived** from `eggsec_policy::ALL_OPERATION_METADATA`, not hand-written. | `the_required_set_is_derived_from_the_policy_catalog` |
| A5 | The profile enables **none** of the engine's serving surfaces (`rest-api`, `grpc-api`, `ws-api`), process-host concerns (`daemon-client`, `config-watch`, `logging-subscriber`, `email-notifications`), MCP exposure markers, the security-risk feature `insecure-tls`, the test-only feature `test-helpers`, the backend drivers, or the `full` aggregate. | `daemon_service_profile_does_not_enable_cli_shaped_serving_features`; `…security_risk_or_test_features`; `…exposure_markers`; `…the_aggregate`; `the_profile_enables_no_forbidden_feature` |
| A6 | The profile **includes** `cli`, and the reason — the `runtime_bridge` / `commands` / `dispatch` gate — is stated in the profile document and in the test's doc comment. | `crates/eggsec/src/lib.rs:88-91`, `:104-105`, `:136-137`; the test doc comment |
| A7 | The **compiled** engine feature set of a `service` build equals the **documented** list exactly, and the documentation table equals the same list. | `the_compiled_engine_feature_set_equals_the_documented_profile`; `daemon_profile_documentation_matches_the_manifest` |
| A8 | A candidate profile that enables a forbidden feature, an unknown feature, or omits a required feature **fails** the conformance check with a message naming the offending feature. | The four negative tests in §10 "Security and negative tests" |
| A9 | The profile's system prerequisites are mapped through `docs/PLATFORM.md` and `crates/eggsec/src/platform/`, and each mapping is cited. The two stale prerequisite claims (`libpcap-dev`, `libssl-dev`) are corrected in the source of truth. | `every_profile_domain_has_a_platform_prerequisite_row`; `docs/PLATFORM.md`; `crates/eggsec/src/platform/prereqs.rs` |
| A10 | A reproducible build recipe exists per profile: the exact cargo command, the MSRV constraint (1.89), the target triple, the feature set, the toolchain record, the `Cargo.lock` hash, and the output checksums — with no signing, bundling, or packaging. | `scripts/build-daemon-profile.sh`; the sidecar; `make build-daemon-profile` |
| A11 | The daemon's runtime `--full-executor` flag, its meaning, and the GUI's guidance string are unchanged. | `the_daemon_runtime_flag_is_unchanged`; `crates/eggsec-gui/src/connection.rs:52` |
| A12 | The daemon protocol, `DaemonCapabilities`, and `RuntimeTaskExecutor` are unchanged, and the protocol version stays at 2. | `the_protocol_version_is_unchanged`; the diff shows no protocol file changed |
| A13 | The engine's `full` aggregate, its 28 members, and its 21 documented exclusions are unchanged. | `the_engine_full_aggregate_is_unchanged` |
| A14 | The daemon's default build still links no engine, and the engine's library default is still empty. | `the_default_daemon_build_still_links_no_engine`; guard 104; guard 16 |
| A15 | No new dependency is added; `deny.toml` and `docs/DEPENDENCY_EXCEPTIONS.md` gain no new exception. | `make check-deps`'s license/source checks |
| A16 | The inherited `make check-deps` advisory failure and the inherited `--all-features` compile error are recorded as inherited in the closure record. | The closure record |
| A17 | `make check` is green, except for the inherited failures in §3.11, which are recorded rather than fixed. | The closure record's command log |

---

## 14. Stop conditions

The agent must **stop and report** rather than improvise when:

| # | Condition | What to report |
|---|---|---|
| 1 | **The profile would require a system dependency that cannot be satisfied on a supported platform.** The supported platforms are Linux and macOS (`ADR-0010`); the profile's only build-time dependency class is a C toolchain for vendored OpenSSL under `nse`. | The exact failing command and the missing prerequisite. Do **not** silently drop `nse` from the profile (that breaks A2/A4) and do **not** switch to a system OpenSSL (that needs a new dependency and a `deny.toml` exception — a separate decision). |
| 2 | **The profile's list drifts from the engine's registry.** A new engine feature is added to `ALL_FEATURES` and is not classified as included or excluded by the profile, or `required_profile_features()` starts returning a name the profile does not contain. | The drift. The profile's list is derived from the registry, so a drift means the derivation rule in Work package A is incomplete or wrong — a **plan change**, not an implementation choice. |
| 3 | **A non-additive protocol change would be needed to publish the profile.** The profile is a **build-side** contract; publishing it must not require changing `DaemonCapabilities`, adding a `ClientCommand`, or bumping the protocol version. | The required wire change, and an ADR escalation. The additive handshake field is M006's; M008 must not create a second one. |
| 4 | **The inherited failures become load-bearing.** If `make check-deps` or `cargo check --workspace --all-features` must be made green **for this milestone's profile to build or to be verified**. | The blocker and the exact command that fails. The `--all-features` `E0616` defect (`crates/eggsec/src/fuzzer/advanced.rs:446` vs `crates/eggsec-payloads/src/websocket.rs:43`) is reachable from the profile because the profile enables `websocket`; fixing the payload crate's field visibility is a separate, separately-owned change. |
| 5 | **The profile would need a new dependency.** `deny.toml` is a closed allow-list. | The dependency, its purpose, and the fact that it needs an owner + review-by date in `deny.toml` **and** `docs/DEPENDENCY_EXCEPTIONS.md` — a policy decision, not an implementation choice. |
| 6 | **The executor's scope resolution turns out to block the GUI.** §3.12 records that `crates/eggsec/src/runtime_bridge/executor.rs:70-73` does not recognise `RuntimeSurface::DesktopGuiManual` as permissive. | That a correctly-profiled daemon still cannot execute GUI-sourced tasks. The fix is inside `RuntimeTaskExecutor`, which §5 places out of scope; it belongs to M006 or to a separate corrective plan. |
| 7 | **The `full-executor` alias would change the daemon's runtime semantics.** The runtime flag `--full-executor` means "host the engine executor". | That aliasing the cargo feature would change what the runtime flag does. The runtime flag is owned by M009's obtainability work and by the GUI's existing guidance. |
| 8 | **A guard would need to be weakened rather than updated.** M008 updates guard 104's daemon literal and adds guard 163. | The guard, and an escalation. Satisfying the profile must never require **removing** or **loosening** an existing guard. |
| 9 | **The scope would expand into another subsystem roadmap.** The daemon binary's install path is M009's; the GUI binary's packaging and qualification is M007's; the wire-side publication is M006's. | The expansion, and the owning roadmap. |
| 10 | **A platform prerequisite claim cannot be verified.** The two corrections in §3.6 rest on the engine's dependency specifications and the repository's own sweep script. | That the claim is **unverified**, before publishing a prerequisite table — a wrong prerequisite costs an operator an afternoon. |

---

## 15. Closure evidence required

The closure record MUST contain:

| # | Evidence | Detail |
|---|---|---|
| 1 | **Commit SHA** | Of the change, and the branch. |
| 2 | **Command log**, verbatim | `make check`, `make check-daemon-profile`, `make check-feature-profiles`, `make check-features-individual`, `make check-deps`, `make check-msrv`, `cargo check -p eggsec-daemon` (no features), `cargo check -p eggsec-daemon --features service`, `cargo check -p eggsec-daemon --features full-executor`, `cargo test -p eggsec --features cli --test feature_matrix`, `cargo test -p eggsec-daemon --features service --test feature_profile`, and `bash scripts/check-architecture-guards.sh`. |
| 3 | **Inherited-failure record** | The exact output of `make check-deps` and `cargo check --workspace --all-features`, with an explicit statement that both were failing at the baseline commit `e1916069` and that neither was changed by this milestone. A diff of the failing output against the baseline output is the strongest form of this evidence. |
| 4 | **The profile table** | As it appears in `docs/DAEMON_FEATURE_PROFILE.md`: the 20 inclusions and the 30 exclusions with their rationales. |
| 5 | **The conformance evidence** | The passing output of the static and runtime conformance tests, plus **one deliberately-broken build** demonstrating that the check actually fails — the profile with `rest-api` added, and the profile with `nse` removed — with the failure messages captured. A conformance test that has never been seen to fail is not evidence. |
| 6 | **The build recipe's output** | The sidecar produced by `scripts/build-daemon-profile.sh`, plus two runs from a clean tree with matching SHA-256 values, plus the dirty-tree refusal message. |
| 7 | **The prerequisite probe's output** | On the executing host. |
| 8 | **The dependency statement** | Confirmation that no dependency was added, and that `deny.toml` and `docs/DEPENDENCY_EXCEPTIONS.md` are unchanged. |
| 9 | **The boundary statement** | Confirmation that `crates/eggsec-daemon-protocol/`, `crates/eggsec-gui/`, `crates/eggsec-cli/`, `crates/eggsec-tui/`, and `crates/eggsec/src/runtime_bridge/` were **not** modified, with a `git diff --stat` proving it. |
| 10 | **The skill-correction statement** | Which skills, if any, carried a stale claim about the daemon's feature set, and what they now say. |
| 11 | **The known-limitations record** | The two stale prerequisite corrections, the executor's scope-resolution gap (§3.12), and anything the profile does not cover. |

---

## 16. Handoff notes

### 16.1 Hazards

| Hazard | Detail |
|---|---|
| **The `cli` gate is not optional.** | The single most likely mistake is to read "a daemon should not enable `cli`" and remove it — which makes the daemon unable to compile its own executor (§3.2). `cli` is in the profile, and the negative test is scoped to the *serving surfaces*, not to `cli`. |
| **The profile enables `websocket`, which the inherited `--all-features` defect touches.** | `cargo check --workspace --all-features` fails at `crates/eggsec/src/fuzzer/advanced.rs:446` because of a private field at `crates/eggsec-payloads/src/websocket.rs:43`. The profile's own build (`--features service`) is **not** `--all-features` and is unaffected; the inherited failure is recorded, not fixed (§14, stop condition 4). |
| **Two prerequisite claims in the repository are stale** (§3.6). | Publishing either would send an operator after a library they do not need and would still not have the one they do need (a C toolchain). The corrections are in scope and MUST be made in the same change. |
| **The `full-executor` alias changes what an existing build command produces.** | `cargo build -p eggsec-daemon --features full-executor` now yields a daemon with 20 engine features instead of 1. That is the point of the milestone, but it is a **behaviour change to an existing command** and must be called out in the closure record and in the release notes. |
| **The sweep gets slower.** | `scripts/check-features-individual.sh:216` now compiles the full profile, including `nse`'s vendored-OpenSSL build. The prerequisite class is unchanged (the sweep already builds `eggsec/nse`), but the wall-clock cost rises. |
| **`cargo deny` still sees every optional dependency.** | `[graph] all-features = true` (`deny.toml:8-12`) means excluding a feature from the profile does **not** remove its advisory entry from `make check-deps`. What it removes is the crate from the **shipped binary**. Say this precisely in any operator-facing text; do not claim the profile "fixes" an advisory. |
| **The GUI's guidance string must not be edited.** | `crates/eggsec-gui/` is out of scope, and its tests pin the exact command string (`crates/eggsec-gui/src/connection.rs:206`, `:213`; `crates/eggsec-gui/src/config.rs:140`). The runtime flag keeps its name. |

### 16.2 Resource constraints

- The `service` profile's build compiles the `nse` closure, which builds
  **vendored OpenSSL from source**. Budget several minutes and a C toolchain
  (`cc`, `perl`, `make`) on a cold build.
- `make check-features-individual` compiles the profile in addition to the
  per-feature sweep; it is a deep check and is not per-PR.
- No npm step, no bundler, no installer, no signing identity, no updater —
  this milestone produces a binary and a recipe only.

### 16.3 Serial-test requirements

- The conformance tests read `crates/eggsec-daemon/Cargo.toml` and
  `docs/DAEMON_FEATURE_PROFILE.md` from the **workspace root**, resolved
  relative to `CARGO_MANIFEST_DIR` — the same technique as the existing
  `cli_forwarded_features_exist_in_registry`
  (`crates/eggsec/tests/feature_matrix.rs:476-493`). They are read-only and
  safe to run in parallel with other tests.
- The runtime conformance test requires `--features service`; it MUST be
  gated so it does not silently pass (or silently fail) in a default build.
- No test in this milestone requires root, a network, a raw socket, or a
  hardware device. The prerequisite probe is **advisory** and never blocks.

### 16.4 Environment requirements

| Requirement | Value | Source |
|---|---|---|
| Rust toolchain | ≥ 1.89 | `Cargo.toml:39`; `make check-msrv` (`Makefile:128-143`) |
| Toolchain pinning | `stable` + `rustfmt` + `clippy` | `rust-toolchain.toml` |
| C toolchain | `cc`, `perl`, `make` — for vendored OpenSSL under `nse` | §3.6 |
| Target | Linux or macOS (Windows out of scope) | `ADR-0010` |
| System libraries | **none** beyond the C toolchain | §3.6 |
| Network | not required to build | — |

### 16.5 Preserved user changes

- This plan is the **only** file created by the planning work for M008.
  `plans/registry.md`, `plans/subsystems/desktop-gui-frontend-roadmap.md`, and
  every source file are untouched by the planning step; the registry already
  carries the M008 row as *planned*.
- The implementation agent MUST NOT rewrite `plans/registry.md` or the
  roadmap as part of the implementation; the closure record owns those
  updates.

### 16.6 Boundaries restated for the implementer

> **M006 owns the runtime publication** (the additive enabled-features
> handshake field and the executor hook). **M008 defines the profile and the
> recipe** and MUST NOT change the daemon protocol, `DaemonCapabilities`, or
> `RuntimeTaskExecutor`. If a conformance test seems to need a wire field, the
> answer is no: read the compiled feature set *inside the daemon process*.
>
> **M009 owns obtainability and start honesty.** `eggsec daemon start` hosts a
> no-op executor that advertises 20 task kinds and executes none
> (`crates/eggsec-cli/src/daemon_cli.rs:45-53`, `:584-607`). **M008 MUST NOT
> fix the CLI's `daemon start`.**
>
> **This milestone's code surface is outside the GUI crate** — it touches
> `crates/eggsec-daemon/` and the engine's feature surface. It is tracked in
> the desktop-GUI roadmap because M006 depends on it and there is no daemon
> subsystem roadmap to carry it.
>
> **M007 owns platform qualification; M008 owns the build side.**
