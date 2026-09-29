# NSE Runtime Extraction Milestone 006B — Eggsec 0.2.0 Adoption and Safe Adapter Staging

Status: implemented

Eggsec planning baseline: `51f9747deef7bb5c2488e7dd392b0f2c96667653`

Standalone runtime baseline: `eggstack/eggsec-nse@9fe149fbb22480a63e254e91d60083b7a29a8ff4`

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-006--020-release-and-safe-eggsec-adoption`

Hard dependency:

- accepted closure of `plans/implementation/nse-runtime-extraction/006-standalone-0-2-0-release.md` with a published `eggsec-nse 0.2.0` registry artifact.

Applicable ADRs:

- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`
- `plans/adrs/ADR-0001-scoped-transport-eggfetch-backend.md`

Primary class: infrastructure

Affected repository:

- consumer/integration target: `eggstack/eggsec`.

## 1. Objective

Move Eggsec from the published `eggsec-nse 0.1.0` dependency to the qualified crates.io `0.2.0` release, replay the already-qualified engine-owned HTTP adapter onto current main, and make the consumer state truthful and safe **without activating automated NSE execution through the adapter yet**.

The adapter is merged as dormant integration code so it compiles and tests against the real 0.2.0 registry contract. Production automated NSE activation remains deferred until the protocol-library capability-gating/scope-threading milestone because M005E proved 72 specialized direct-I/O files still bypass runtime capability policy.

## 2. Why activation is intentionally deferred

M005C produced and tested `NseHttpTransportProvider`, but production dispatch was intentionally not rewired because NSE execution did not carry the approved scope into the runtime path.

M005E then established a stronger constraint:

- 72 specialized direct-I/O files perform network I/O with no capability consultation;
- this residual is pinned against expansion but remains reachable under deny-all runtime policy;
- the M005 closure classified the residual medium severity while Eggsec NSE remained manual-only.

Activating automated/strict NSE now would change that blast radius: outer HTTP authority would protect provider-backed HTTP operations, but specialized direct socket paths could still escape that authority. Therefore M006 adoption must not turn NSE into a newly automated surface.

## 3. Current consumer state

At the planning baseline:

- `crates/eggsec/Cargo.toml` uses `eggsec-nse = { version = "0.1.0", optional = true }`;
- Guard 144 enforces a crates.io registry source and the released version;
- Checks 145-146 enforce single direct consumer, `eggsec::nse` facade, TUI/Python forwarding, engine-owned adapters, and canonical runtime execution;
- `m005c-eggsec-http-adapter` contains one unique adapter commit `4ade61a85b9cb52df89e000e502f8a33badc2a8a` based on an older Eggsec main;
- that branch is now stale/diverged and must not be merged wholesale;
- `OperationMetadata` currently advertises `nse` as manual/TUI/MCP/REST/agent/gRPC exposable;
- the canonical NSE execution branch still calls `dispatch::api::run_nse`, which constructs `ManualPermissive` and a native-provider request.

## 4. Invariants

- Eggsec remains the only direct `eggsec-nse` consumer.
- TUI/Python remain behind `eggsec::nse`.
- Runtime source is crates.io only; no Git/path override lands.
- Adapter code stays engine-owned and standalone remains Eggsec-independent.
- No automated/strict production path may execute NSE with `ManualPermissive`.
- No automated/strict production path may activate `NseHttpTransportProvider` while the 72-file ungated residual remains unresolved.
- Manual CLI/TUI NSE behavior remains source-compatible.
- The staged adapter must never manufacture `NetworkAuthority`; constructor continues requiring an existing authority.
- Script-provided TLS intent cannot escalate adapter TLS posture.
- 0.2.0's corrected send/write accounting is accepted; do not restore old counters.
- The breaking `register_vulns_library` change is absorbed only through the released dependency update.

## 5. Scope

### In scope

- change Eggsec dependency to `eggsec-nse 0.2.0`;
- update `Cargo.lock` and checksum/source evidence;
- update Check 144 and dependency docs to require 0.2.0;
- replay the adapter change from commit `4ade61a...` onto current main rather than merging the stale branch history;
- add/adjust `crates/eggsec/src/nse_http_provider.rs` and the engine module export under the correct feature gate;
- run the adapter's mapping/authority/TLS/no-concrete-client tests against the real registry package;
- update architecture docs and `.opencode/skills/eggsec-nse/SKILL.md` to say 0.2.0 is the consumed runtime and the adapter is staged-but-dormant;
- add a fail-closed exposure gate so automated NSE is not newly available before the protocol-gating milestone;
- requalify manual NSE/TUI/Python behavior against 0.2.0.

### Automated-exposure safety gate

Until the follow-up protocol-gating/scope-threading milestone closes, the canonical operation catalog must not advertise NSE as an automated API/agent operation.

Preferred temporary posture:

- `manual_exposable = true`;
- `tui_exposable = true`;
- `mcp_exposable = false`;
- `rest_exposable = false`;
- `agent_exposable = false`;
- `grpc_exposable = false`.

If another repository-wide mechanism is the canonical way to quarantine an operation, use that instead, but closure must prove automated surfaces cannot dispatch NSE.

Do not weaken `TargetPolicyKind::ExplicitScopeRequired`; it will be needed when automated NSE is re-enabled.

### Explicitly out of scope

- production dispatch threading of `NetworkAuthority`;
- changing `execute_approved_execution` to execute scoped NSE;
- new runtime profiles;
- protocol-library capability gating;
- broad operation-metadata redesign;
- publishing another standalone release;
- deleting the native provider path used by manual surfaces.

## 6. Required production changes

### Dependency adoption

Update:

```toml
eggsec-nse = { version = "0.2.0", optional = true }
```

and lockfile to the crates.io artifact published by 006A.

No `[patch.crates-io]`, Git, path, tag, branch, or rev is permitted in committed state.

### Adapter replay

Use `4ade61a...` as evidence/source material only. Replay its two-file logical change onto current main:

- `crates/eggsec/src/nse_http_provider.rs`;
- corresponding `lib.rs` module/export wiring.

Resolve against current 0.2.0 types and current Eggsec transport APIs. Do not merge the old feature branch wholesale.

### Exposure quarantine

Update operation metadata/guards so automated registries and strict programmatic surfaces do not expose NSE until M007.

If any strict dispatch path can still synthesize a canonical NSE request despite exposure metadata, add a fail-closed execution guard and test it.

Manual/TUI paths remain available.

## 7. Ordered work packages

### A — Registry adoption

Update manifest + lockfile, dependency guards, and verify `cargo tree` resolves 0.2.0 from crates.io.

### B — Replay the adapter

Port the staged adapter onto current main and compile/test it against 0.2.0.

Acceptance: no temporary override remains and the adapter names no concrete HTTP client.

### C — Quarantine automated NSE exposure

Make metadata/registry exposure truthful while the ungated residual exists.

Acceptance: MCP/REST/agent/gRPC operation enumeration cannot offer NSE, and a direct strict-path regression test fails closed if one bypasses enumeration.

### D — Manual consumer requalification

Verify existing manual NSE canonical dispatch, TUI, and Python paths behave against 0.2.0.

### E — Documentation and guards

Update architecture/skill/dependency docs and architecture guards for:
- registry version 0.2.0;
- dormant engine-owned adapter present;
- automated NSE disabled pending M007;
- no production scope/adapter activation claim.

## 8. Failure/recovery semantics

- If the registry package cannot compile Eggsec without a Git/path patch, stop; 006A release is defective or consumer code needs a separately justified compatibility fix.
- If replaying the adapter requires authority construction from URL/target alone, stop.
- If automated NSE cannot be quarantined without broad unrelated API churn, stop and write a focused prerequisite plan.
- If manual NSE behavior regresses under 0.2.0, do not activate or publish further integration changes; classify before proceeding.
- Rollback is the prior Eggsec commit consuming 0.1.0; do not yank 0.2.0 merely for an Eggsec-only integration defect.

## 9. Required tests

### Dependency/guard

```bash
cargo tree -p eggsec --features nse,cli -i eggsec-nse
make check-deps
make test-architecture-guards
```

### Eggsec NSE

```bash
cargo check -p eggsec --features nse-ssh2,nse-sandbox,cli
cargo test -p eggsec --features nse,cli --lib
cargo test -p eggsec --features nse,cli --test nse_bridge_tests --test nse_integration_tests --test nse_real_scripts --test nse_tests
cargo test -p eggsec-tui --features nse
cargo test -p eggsec-python --features nse
```

### Adapter

- all staged adapter mapping tests;
- in-scope/out-of-scope/deny-all authority tests;
- TLS non-escalation;
- invalid URL/no transport contact;
- source scan prohibiting reqwest/eggfetch concrete clients inside the adapter module.

### Exposure quarantine

- operation metadata says automated exposure false;
- tool/registry enumeration omits NSE on automated surfaces;
- direct strict approved path cannot execute NSE before M007;
- manual/TUI NSE remains available.

## 10. Acceptance criteria

1. Eggsec manifest and lockfile resolve crates.io `eggsec-nse 0.2.0`.
2. No Git/path/patch runtime source remains.
3. Guard 144 requires the 0.2.0 registry package.
4. Staged adapter code is replayed onto current main and compiles against 0.2.0.
5. Adapter authority/TLS/no-fallback tests pass.
6. Adapter has no production caller yet.
7. Automated exposure of NSE is disabled/fail-closed until M007.
8. Manual/TUI NSE remains functional.
9. Python remains behind the Eggsec facade and passes.
10. No authorization logic moves into standalone.
11. Docs state adapter dormant and protocol-gating prerequisite explicitly.
12. Full relevant Eggsec checks pass.
13. Closure unblocks 006C qualification.

## 11. Stop conditions

Stop if:

- adoption requires a temporary source override in committed state;
- automated NSE becomes executable through `ManualPermissive`;
- the adapter can be reached without an existing authority;
- production activation begins before protocol-gating/scope-threading work;
- unrelated protocol-gating work enters this plan.

## 12. Closure evidence

Record:

- published 0.2.0 identity from 006A;
- Eggsec manifest/lockfile before/after;
- `cargo tree` source/version;
- replayed adapter commit/file mapping from `4ade61a...`;
- adapter test results;
- automated exposure metadata before/after;
- strict-path fail-closed test;
- manual NSE/TUI/Python results;
- architecture/dependency guard results;
- unresolved findings;
- explicit GO/NO-GO for 006C.

## 13. Handoff notes

Do not merge `m005c-eggsec-http-adapter` wholesale; it is a stale one-commit evidence branch. Replay the logical adapter change on current main.

This milestone deliberately adopts and compiles the adapter without activating it. The 72-file ungated specialized residual means automated NSE scope enforcement is not complete yet; preserve the M005 blast-radius assumption by keeping NSE manual-only until M007.
