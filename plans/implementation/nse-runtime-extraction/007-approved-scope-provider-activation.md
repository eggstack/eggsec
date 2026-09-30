# NSE Runtime Extraction Milestone 007D — Eggsec Approved-Scope/Profile Threading and Scoped Provider Activation

Status: blocked

Planning baseline: `386fe63a522aac66340386ac83e9f5ed54500d8b`

Eggsec runtime baseline: crates.io `eggsec-nse 0.2.0`

Hard dependency:

- accepted closure of `plans/implementation/nse-runtime-extraction/007-standalone-security-patch-release.md` and adoption target version resolved (expected `0.2.1`).

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-007--protocol-library-gating-and-controlled-automated-activation`

Applicable ADRs:

- `plans/adrs/ADR-0004-nse-automated-activation-boundary.md`
- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`
- `plans/adrs/ADR-0001-scoped-transport-eggfetch-backend.md`

Primary class: security + infrastructure

Implementation repository:

- `eggstack/eggsec`

## 1. Objective

Adopt the M007 standalone security release and activate scoped NSE host-service injection on the **scope-bearing strict execution path only**, while keeping automated operation metadata quarantined until M007E qualification.

This slice must make strict NSE technically safe before making it discoverable.

## 2. Security boundary

After this slice:

- `execute_approved()` (approval token only, no scope snapshot) must reject NSE unconditionally;
- `execute_approved_execution()` may execute NSE using the exact `ApprovedExecution` scope snapshot;
- manual CLI/TUI NSE remains on the existing manual/native path;
- automated metadata flags remain false until M007E.

No strict NSE execution may fall back to native unscoped services.

## 3. Retain approval-time target facts

Current `ApprovedExecution` stores:

- `ApprovedOperation`;
- cloned `Scope`.

Extend it with the normalized/resolved `TargetScope` facts used during approval.

Requirements:

- target facts are resolved once by the existing engine approval resolver;
- the same facts are passed to the policy decision and retained in the execution bundle;
- strict NSE profile construction must not re-resolve the primary target merely to determine runtime target policy;
- no caller can replace the facts independently of the approval token/scope bundle;
- target-less operations retain `None`.

This is binding evidence, not a new authorization decision.

## 4. Extract generic owned scope authority

`loadtest::backend::OwnedScopeAuthority` is already the correct pattern but is owned by the load-test module.

Move/extract it to a generic engine policy-bridge location, e.g. `policy_bridge::transport::OwnedScopeAuthority`, and preserve compatibility re-exports for load-test users.

The implementation must continue delegating every checkpoint to canonical `ScopeAuthority`.

No second scope language.

## 5. Scoped NSE network providers

Add engine-owned wrappers implementing the standalone provider traits.

### DNS

`ScopedNseDnsProvider`:

- delegates to the standalone native DNS provider;
- for address-resolution results used by network brokers, calls the same `NetworkAuthority::authorize_resolved` policy;
- mixed allowed/disallowed candidate sets fail closed;
- preserves non-address DNS result compatibility where it does not itself create network authority.

### TCP

`ScopedNseTcpProvider`:

- receives an already-selected `NseResolvedEndpoint`;
- calls `authorize_socket(original_host, address, port)`;
- only then delegates to `NativeTcpSocketProvider`.

### UDP

Same exact-endpoint rule via `NativeUdpSocketProvider`.

These providers must never construct a broader scope or parse policy independently.

## 6. HTTP provider activation

Use the landed `NseHttpTransportProvider` with:

- `eggsec_transport_eggfetch::EggfetchTransport`;
- `SystemTransportResolver`;
- the same `Arc<dyn NetworkAuthority>` / owned scope authority as the TCP/UDP wrappers;
- profile-derived insecure-TLS intent only.

Inject it using the new authority-bound HTTP service-bundle API from M007A.

No native HTTP fallback on scoped strict execution.

## 7. Runtime profile mapping

Create one engine-owned mapping from Eggsec execution profile to NSE profile:

- `McpStrict` -> `AgentSafe`;
- `AgentStrict` -> `AgentSafe`;
- `CiStrict` -> `CiSafe`;
- `ManualGuarded` -> `ManualStrict` when using scope-bearing manual execution;
- `ManualPermissive` does not enter this strict helper.

For AgentSafe network policy, use no broader than the exact approved primary-target IP set retained in `ApprovedExecution.target_facts`.

Do not translate full Eggsec hostname patterns/exclusions/ports into runtime CIDRs. Scoped providers enforce the canonical Eggsec policy; runtime profile is defense-in-depth.

If approval has no usable resolved primary-target IPs for a networked NSE run, fail closed.

## 8. Canonical strict NSE execution

Add an engine helper such as `run_nse_approved_execution` that owns:

- profile mapping;
- host-service composition;
- `NseRunRequest` construction;
- `with_host_services`;
- report/result conversion.

`execute_approved_execution()` must route NSE through this helper.

`execute_approved()` must retain an NSE-specific rejection even after M007E changes metadata exposure.

Do not duplicate standalone orchestration.

## 9. Dependency adoption

Move Eggsec from `eggsec-nse 0.2.0` to the M007C-R published version (`0.3.0`). **The `expected 0.2.1` figure in this step was superseded:** M007C's public-API gate failed with a major break (74 removed public functions, `plans/closure/nse-runtime-extraction/007c-closure.md`), so no 0.2.1 exists and the adoption target is the breaking release planned in `007-breaking-0-3-0-release.md`. M007D must plan for the full 0.2.0 -> 0.3.0 migration: thread `NseCapabilityContext` + `NseHostServices` into every `register_*_library_with_services` call, and drop any use of the withdrawn `helpers::tls_connect` / `helpers::tcp_connect_with_timeout`.

Update:

- manifest;
- lockfile;
- guard 144;
- architecture/skill docs.

No Git/path patch is permitted.

## 10. Ordered work packages

### A — Adopt registry release

Acceptance: `cargo tree` shows the exact crates.io M007 release only.

### B — Bind approval-time target facts

Refactor approval to resolve facts once and store them in `ApprovedExecution`.

Acceptance:

- same facts feed approval and execution;
- mutation/rebinding tests fail.

### C — Extract `OwnedScopeAuthority`

Acceptance:

- load-test behavior unchanged;
- NSE can use the generic owned authority without depending on load-test internals.

### D — Implement scoped DNS/TCP/UDP providers

Acceptance:

- mixed DNS answers deny;
- allowed address + allowed port succeeds;
- allowed address + disallowed port denies before native connect;
- zero native provider calls on denial.

### E — Compose scoped HTTP provider

Acceptance:

- HTTP bundle is explicitly authority-bound;
- out-of-scope host, port, redirect, proxy peer, and TLS mismatch fail closed;
- no native fallback.

### F — Build strict NSE profile/request helper

Acceptance:

- profile mapping tests;
- approved target facts become runtime resolved-target policy;
- CiStrict maps to CiSafe/zero network;
- ManualPermissive cannot be selected.

### G — Rewire scope-bearing strict entry

Acceptance:

- `execute_approved_execution` can run a safe provider-backed NSE fixture through scoped services;
- `execute_approved` rejects NSE;
- metadata automated flags are still false.

## 11. Invariants

- Automated metadata quarantine remains during this slice.
- Manual/TUI path remains unchanged.
- Strict NSE never receives `NseHostServices::native()`.
- Strict NSE never constructs authority from URL/target alone.
- Scope snapshot and target facts are approval-bound.
- Port restrictions are enforced by scoped TCP/UDP providers.
- HTTP resolution/redirect authority stays in `HttpTransport + NetworkAuthority`.
- TUI/Python remain facade consumers.
- No Eggsec dependency enters standalone.

## 12. Required tests

### Approval binding

- facts retained exactly;
- target mismatch fails;
- scope snapshot cannot be substituted;
- DNS re-resolution not used to build runtime primary-target policy.

### Scoped providers

- mixed DNS deny;
- CIDR/host exclusion deny;
- port deny;
- TCP/UDP allowed loopback;
- zero-contact negatives.

### HTTP

Existing adapter tests plus production-composition tests using Eggfetch transport/fake resolver as appropriate.

### Execution

- strict safe script succeeds through `execute_approved_execution`;
- strict unsafe/manual-only library is rejected by runtime effect gate;
- scope-less `execute_approved` rejects NSE;
- CiStrict network script fails closed;
- manual dispatch unchanged.

## 13. Verification

```bash
cargo tree -p eggsec --features nse,cli -i eggsec-nse
cargo check -p eggsec --features nse-ssh2,nse-sandbox,cli
cargo test -p eggsec --features nse,cli --lib
cargo test -p eggsec --features nse,cli --test nse_bridge_tests --test nse_integration_tests --test nse_real_scripts --test nse_tests
cargo test -p eggsec-tui --features nse
cargo test -p eggsec-python --features nse
make check-deps
make test-architecture-guards
make check-features-individual
make check
make check-python
```

## 14. Acceptance criteria

1. Eggsec consumes the M007 standalone registry release.
2. `ApprovedExecution` retains approval-time target facts.
3. Generic owned scope authority is engine-owned outside load-test specialization.
4. Scoped DNS/TCP/UDP providers enforce canonical scope/port checks.
5. HTTP provider is authority-bound and uses scoped Eggfetch transport.
6. Strict profile mapping is centralized and tested.
7. Strict NSE uses injected scoped services.
8. Scope-less strict NSE remains rejected.
9. Unsafe residual libraries remain blocked by runtime effect gate.
10. Automated metadata flags remain false in this slice.
11. Manual/TUI/Python behavior remains green.
12. No native fallback exists on strict NSE path.
13. Closure unblocks M007E.

## 15. Stop conditions

Stop if:

- approval-time facts cannot be retained without a second resolution;
- scope must be approximated by a weaker policy language;
- strict execution needs `ManualPermissive`;
- any scoped provider can fall back to native after policy denial;
- metadata exposure is re-enabled before M007E;
- adoption requires a source override.

## 16. Closure evidence

Record:

- adopted release identity;
- approval-target-fact diff/tests;
- owned authority extraction;
- scoped provider source/tests;
- strict profile mapping;
- strict safe/unsafe fixture outcomes;
- scope-less rejection test;
- manual/TUI/Python results;
- dependency/architecture/full checks;
- GO/NO-GO for M007E.
