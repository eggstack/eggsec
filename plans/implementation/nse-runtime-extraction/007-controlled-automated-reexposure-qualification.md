# NSE Runtime Extraction Milestone 007E — Controlled Automated Re-Exposure and Cross-Repository Qualification

Status: blocked

Planning baseline: `386fe63a522aac66340386ac83e9f5ed54500d8b`

Hard dependency:

- accepted closure of `plans/implementation/nse-runtime-extraction/007-approved-scope-provider-activation.md`.

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-007--protocol-library-gating-and-controlled-automated-activation`

Applicable ADR:

- `plans/adrs/ADR-0004-nse-automated-activation-boundary.md`

Primary class: security qualification

Affected repositories:

- `eggstack/eggsec`;
- `eggstack/eggsec-nse`.

## 1. Objective

Deliberately restore automated NSE exposure only after the runtime library gate and Eggsec scope-bearing provider path are proven, then qualify the complete M007 boundary across both repositories.

This slice changes discoverability/exposure, not provider architecture.

## 2. Exposure policy

After qualification, NSE may be exposed on automated programmatic surfaces that use the canonical strict approval/execution path:

- MCP;
- REST;
- security agent;
- gRPC.

These surfaces map to AgentSafe through M007D.

CI remains governed by `CiSafe`; networked NSE behavior stays denied there.

Manual/TUI remain unchanged.

The operation-level exposure does **not** mean every builtin script is automated-safe. Script/library eligibility remains enforced dynamically by the standalone effect gate.

## 3. Required metadata changes

Update NSE `OperationMetadata` only after all M007D tests are green:

- `manual_exposable = true`;
- `tui_exposable = true`;
- `mcp_exposable = true`;
- `rest_exposable = true`;
- `agent_exposable = true`;
- `grpc_exposable = true`.

Keep:

- `TargetPolicyKind::ExplicitScopeRequired`;
- `Capability::NseSafe`;
- existing risk/mode classification unless separate evidence requires change.

Do not remove the scope-less NSE rejection in `execute_approved()`.

## 4. Guard transition

M006 guard 145 currently requires zero production construction of `NseHttpTransportProvider`.

Replace the dormancy pin with an exact-call-site pin:

- construction allowed only in the approved strict NSE composition helper;
- test constructors remain allowed in the adapter module;
- no CLI/TUI/Python/manual path may construct the scoped automated provider bundle;
- no secondary production helper may bypass the canonical path.

Likewise update quarantine guards/tests rather than deleting them:

- metadata tests now assert the intended automated exposure;
- strict scope-less entry test still asserts rejection;
- scope-bearing strict entry positive/negative tests become the activation proof.

## 5. Positive automated qualification

Select at least one builtin/clean-room script whose transitive library set is fully `Pure`/`ProviderBacked`.

For each automated surface class or common canonical strict path:

- approval requires explicit scope;
- target facts are bound;
- scoped host services are injected;
- safe script executes successfully;
- capability/report evidence records the strict profile;
- no manual warning/profile appears.

At least one HTTP script must execute through `NseHttpTransportProvider`.

At least one TCP/UDP provider-backed script should be qualified if the M007B promoted cohort supplies a stable fixture.

## 6. Negative automated qualification

Required zero-host-contact cases:

1. script requiring a `ManualOnlyDirectIo` module;
2. script requiring a `ManualOnlyAdvisory` module;
3. unknown/dynamic unsafe require;
4. direct-global access attempt to an unsafe module;
5. in-scope host but disallowed port;
6. mixed allowed/disallowed DNS candidates;
7. out-of-scope HTTP redirect;
8. scope-less strict `execute_approved()` NSE request;
9. CiSafe networked script;
10. attempt to force ManualPermissive on an automated path.

Failures must occur before unapproved host contact.

## 7. Residual disposition

M007 does not require the direct-I/O residual to reach zero.

Closure must report:

- M005E baseline: 72 ungated + 25 advisory;
- M007B promoted counts;
- remaining counts by migration class;
- proof every remaining direct/advisory module is automated-unavailable;
- future migration recommendation.

The residual severity may be downgraded only if the automated-unreachable property is mechanically enforced and manual-only status is documented.

## 8. Documentation reconciliation

Update:

Standalone:
- `docs/PROVIDERS.md`;
- compatibility/security docs;
- release notes if needed.

Eggsec:
- `architecture/nse_integration.md`;
- `architecture/nse_capability_inventory.md`;
- `docs/NSE_COMPATIBILITY.md`;
- `.opencode/skills/eggsec-nse/SKILL.md`;
- architecture guard docs;
- roadmap/registry at closure.

Required wording:

- automated NSE is supported only through explicit scope + approved execution;
- runtime effect gate may reject scripts/libraries that remain manual-only;
- provider availability is not universal compatibility;
- manual surfaces preserve broader compatibility.

## 9. Full qualification

### Standalone

Re-verify the published M007 release identity and effect-manifest/residual guards.

Run full standalone CI matrix if any post-release runtime correction occurs; otherwise verify immutable release artifact + existing release CI.

### Eggsec

```bash
cargo check -p eggsec --features nse-ssh2,nse-sandbox,cli
cargo test -p eggsec --features nse,cli --lib
cargo test -p eggsec --features nse,cli --test nse_bridge_tests --test nse_integration_tests --test nse_real_scripts --test nse_tests
cargo test -p eggsec-tui --features nse
cargo test -p eggsec-python --features nse
cargo test -p eggsec-policy --lib
make check-deps
make test-architecture-guards
make check-features-individual
make check
make check-python
```

Hosted main CI and code-quality checks must be green on the closure SHA.

## 10. Acceptance criteria

1. M007A automated library gate is active and complete.
2. M007B residual migration evidence is reconciled.
3. M007 standalone release is immutable and registry-resolved by Eggsec.
4. Strict NSE uses approval-bound scope + target facts.
5. HTTP/TCP/UDP scoped providers have no native fallback.
6. Scope-less strict NSE remains rejected.
7. Automated metadata is re-enabled only on intended surfaces.
8. Safe automated script succeeds through scope-bearing strict execution.
9. HTTP positive path uses the scoped adapter.
10. Unsafe residual script is rejected before host contact.
11. Direct-global and dynamic-require bypasses fail.
12. Port/DNS/redirect negative cases fail closed.
13. CI networked NSE remains denied.
14. Manual/TUI/Python behavior remains compatible.
15. Guards pin the single production adapter/service composition point.
16. Full local + hosted qualification is green.
17. Documentation states selective automated compatibility accurately.
18. Parent M007 receives an evidence-backed closure disposition.

## 11. Stop conditions

Stop and reopen/correct if:

- an unsafe library is reachable under AgentSafe/CiSafe;
- any automated path reaches `ManualPermissive`;
- `execute_approved()` can execute NSE without the scope snapshot;
- HTTP/TCP/UDP can fall back to native unscoped execution;
- a negative scope test makes host contact;
- re-exposure requires weakening `ExplicitScopeRequired`;
- registry source diverges from the qualified standalone release.

## 12. Closure evidence

Record:

- M007A-D closures;
- standalone release/version/SHA;
- Eggsec adoption SHA;
- metadata before/after;
- exact strict service composition call site;
- safe positive script identities/results;
- all negative zero-contact results;
- residual before/after counts/classes;
- manual/TUI/Python results;
- dependency/architecture/full checks;
- hosted CI run;
- unresolved risk list;
- recommendation for any M008 residual-migration expansion.

## 13. Handoff notes

Do not treat operation metadata re-exposure as the security boundary. The boundary is the combination of:

1. explicit Eggsec approval + scope snapshot;
2. approval-time target facts;
3. strict runtime profile;
4. effect-gated Lua environment;
5. scoped provider bundle;
6. scope-less NSE rejection.

The metadata change is last because it only makes the already-qualified path discoverable.
