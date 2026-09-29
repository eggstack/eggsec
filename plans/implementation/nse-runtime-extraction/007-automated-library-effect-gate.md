# NSE Runtime Extraction Milestone 007A — Automated Library Effect Gate and HTTP Authority Assurance

Status: closed (`plans/closure/nse-runtime-extraction/007a-closure.md`; standalone `c9df4d1`; GO for M007B)

Eggsec planning baseline: `386fe63a522aac66340386ac83e9f5ed54500d8b`

Standalone baseline: `eggstack/eggsec-nse@ff0d2c09feba1bcd7ba5f7312579690d905be09c` (`0.2.0`)

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-007--protocol-library-gating-and-controlled-automated-activation`

Applicable ADR:

- `plans/adrs/ADR-0004-nse-automated-activation-boundary.md`
- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`

Primary class: security

Implementation repository:

- `eggstack/eggsec-nse`

## 1. Objective

Establish a complete, fail-closed automated-library eligibility boundary in the standalone runtime before any Eggsec automated NSE surface is re-enabled.

This slice must:

- classify every Lua library/global registered by the runtime by host-effect safety;
- prevent AgentSafe/CiSafe runs from receiving direct-I/O/advisory/unknown libraries through either `require()` or direct globals;
- preserve manual compatibility;
- add an explicit authority-assurance contract for automated HTTP so the native reqwest provider is not silently treated as scope-authoritative.

No Eggsec production activation occurs in this slice.

## 2. Evidence motivating the work

M005E source audit produced machine-pinned residuals:

- 72 direct-socket files with no capability consultation;
- 25 advisory-gated files with direct socket effects outside provider injection/accounting.

The executor registers substantially more libraries than the current 43-entry declarative resolver registry describes. Many residual modules are inserted into the Lua environment directly by `ExecutorCore::register_libraries()`.

Therefore a gate applied only to static `require` parsing or only to the existing 43 descriptors is incomplete.

Current HTTP also requires correction before automated use:

- `broker_http_request` checks the string `request.host`;
- CIDR/resolved-target policy only makes a concrete membership decision for IP literals;
- `NativeHttpProvider` lets reqwest perform hostname resolution and redirects internally.

For manual compatibility this remains acceptable; for automated authority claims it must fail closed unless the HTTP provider is explicitly authority-bound.

## 3. Design requirements

### Complete effect manifest

Create a complete security manifest for every module/global registered into the Lua VM.

Use a companion manifest or separate additive type; do not add a required field to the public `NseLibraryDescriptor` struct and accidentally create another breaking release.

Minimum eligibility classes:

- `Pure`;
- `ProviderBacked`;
- `ManualOnlyDirectIo`;
- `ManualOnlyAdvisory`.

Unknown/unclassified names resolve to manual-only.

The manifest must be mechanically reconciled against the actual registration set in `ExecutorCore::register_libraries()` and the existing M005E inventories.

### Automated registration gate

For AgentSafe/CiSafe:

- register/expose only `Pure` and eligible `ProviderBacked` modules;
- unsafe modules must not be reachable as globals;
- dynamic `require()` must return a policy denial for unsafe/unknown modules;
- required-module reports must record `BlockedByPolicy` or an equivalent explicit source.

Manual profiles preserve current registration behavior.

### HTTP authority assurance

Extend `NseHostServices` additively so an injected HTTP provider can be marked authority-bound.

Safe default:

- native/default HTTP provider is **not** authority-bound;
- AgentSafe HTTP is denied before provider invocation unless the service bundle explicitly carries authority-bound HTTP assurance;
- CiSafe remains network-denied;
- manual profiles keep existing native behavior.

Do not claim that a provider is authority-bound merely because a capability pre-check occurred.

The Eggsec adapter will become the first authority-bound provider in M007D.

## 4. Invariants

- Zero Eggsec dependency in standalone.
- ManualPermissive/CompatibilityLab behavior remains compatible.
- Unsafe modules are not merely blocked at `require()`; direct global access also fails.
- Unknown library eligibility is deny-by-default under automated profiles.
- No residual inventory entry may be promoted to automated-safe without provider-backed evidence.
- Existing M005E pins remain active and are augmented rather than deleted.
- Native HTTP remains available manually.
- AgentSafe + native HTTP cannot perform a hostname request.
- Provider denial means zero provider contact.
- MSRV remains 1.89.

## 5. Ordered work packages

### A — Build authoritative registered-library inventory

Parse/encode the actual `register_libraries()` set and map every registered name to a source module and effect class.

Acceptance:

- no registered library/global lacks a manifest entry;
- no manifest entry names a non-registered module without an explicit compatibility reason;
- current 72/25 inventories map to manual-only classes.

### B — Implement automated eligibility API

Add runtime-owned query helpers such as:

```rust
automated_library_eligibility(name) -> NseAutomatedLibraryEligibility
is_automated_library_safe(name) -> bool
```

Exact naming may vary.

Acceptance:

- API is additive;
- existing public descriptor struct layout is untouched.

### C — Gate library registration

Thread `profile_kind` into registration decisions.

AgentSafe/CiSafe must never install unsafe globals.

Acceptance:

- direct-global regression tests cannot access representative residual libraries;
- manual profile can.

### D — Gate dynamic require

Use the same manifest, not a second allowlist.

Acceptance:

- representative safe module succeeds;
- representative 72-file residual and 25-file advisory module return policy-blocked reports;
- unknown module is fail-closed.

### E — Add HTTP authority assurance

Add service-bundle state/builder without changing existing constructors.

Acceptance:

- AgentSafe + native/default HTTP -> denied, provider call count zero;
- AgentSafe + explicitly authority-bound mock provider -> permitted when runtime network policy permits;
- CiSafe remains denied regardless;
- manual native HTTP remains unchanged.

### F — Guards/docs

Add static/runtime guards proving:

- complete manifest coverage;
- unsafe automated registration cannot silently expand;
- M005E inventory classification agrees with effect manifest;
- native HTTP is never labeled authority-bound by default.

Update `docs/PROVIDERS.md`.

## 6. Required tests

- manifest completeness over every registration;
- unsafe direct-global absence under AgentSafe/CiSafe;
- unsafe dynamic require blocked;
- manual compatibility for same module;
- authority-bound vs native HTTP provider call-count tests;
- current provider composition tests;
- current profile/corpus tests;
- boundary negative probes.

## 7. Verification

```bash
cargo fmt --all --check
./scripts/check-boundaries.sh
cargo check --no-default-features
cargo check --features nse
cargo test --features nse
cargo check --features nse-ssh2
cargo check --features nse,sandbox
cargo clippy --all-targets --features nse
cargo +1.89.0 check --locked --features nse
cargo package
```

Explicitly run new eligibility/authority tests.

## 8. Acceptance criteria

1. Every registered Lua library/global has an effect classification.
2. Unknown classification is automated-denied.
3. AgentSafe/CiSafe never register direct/advisory residual globals.
4. Dynamic require uses the same classification.
5. Required-module reports expose policy denial.
6. Manual profiles retain current compatibility.
7. Native HTTP is not authority-bound.
8. AgentSafe native HTTP fails before provider contact.
9. Authority-bound HTTP can be used under AgentSafe subject to runtime policy.
10. M005E inventories remain guard-pinned.
11. No breaking public API is introduced.
12. Closure unblocks M007B.

## 9. Stop conditions

Stop if:

- complete registration coverage cannot be derived deterministically;
- automated safety requires relying only on static-require parsing;
- a breaking public API is required;
- native HTTP must remain implicitly trusted in AgentSafe;
- manual compatibility must be removed to enforce the automated gate.

## 10. Closure evidence

Record:

- manifest schema + complete inventory;
- mapping of 72/25 residual sets;
- direct-global and require negative tests;
- HTTP assurance tests;
- full standalone verification;
- residual counts unchanged in this slice;
- GO/NO-GO for M007B.
