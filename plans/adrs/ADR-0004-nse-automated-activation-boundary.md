# ADR-0004: NSE automated activation requires effect-gated libraries and scope-bearing execution

Status: accepted

Date: 2026-09-29

Decision owners: project maintainers

Related subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md`

Related ADRs:

- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`
- `plans/adrs/ADR-0001-scoped-transport-eggfetch-backend.md`

## Context

M006 closed with `eggsec-nse 0.2.0` adopted by Eggsec, the engine-owned `NseHttpTransportProvider` replayed onto current main, and automated NSE quarantined on MCP/REST/agent/gRPC surfaces.

The quarantine is intentional. M005E proved that the standalone runtime still contains two specialized direct-I/O classes:

- 72 files with direct socket effects and no `NseCapabilityContext` consultation;
- 25 files with at least one capability gate but remaining direct socket effects that bypass provider injection/cancellation/accounting.

The runtime already has safer provider-backed domains for DNS/TCP/UDP/HTTP and a per-run `NseHostServices` bundle. The M005E residual is therefore not a missing transport primitive alone; it is a library/effect-eligibility problem.

M006 also established the Eggsec-side scope boundary:

- `ApprovedExecution` binds an `ApprovedOperation` to the exact `Scope` snapshot from the approving `EnforcementContext`;
- `ScopeAuthority` implements transport authorization without introducing a second policy language;
- strict approved execution currently rejects NSE because NSE is marked manual-only;
- manual NSE still uses `ManualPermissive` and native default host services.

A further issue exists in the standalone HTTP path. `broker_http_request` performs a runtime capability check on the request host string and then calls the selected HTTP provider. CIDR/resolved-target checks only make a concrete membership decision when the target is an IP literal. A native HTTP provider may subsequently resolve a hostname and follow redirects independently. That is not sufficient for an automated authority claim even though Eggsec's staged HTTP adapter is scope-aware.

The design must allow useful automated NSE again without claiming that every compatibility library has been rewritten, without translating the full Eggsec scope language into a weaker runtime policy language, and without allowing scope-less strict execution to reach NSE.

## Decision drivers

- Fail closed for automated scripts that depend on unclassified/direct-I/O libraries.
- Preserve manual compatibility for the long tail of NSE protocol libraries.
- Reuse the runtime provider/broker surface rather than create a second network policy implementation.
- Preserve the exact Eggsec `ApprovedExecution` scope snapshot through execution.
- Avoid re-resolving the primary target between approval and runtime profile construction.
- Require an authority-bound HTTP provider for automated HTTP.
- Keep standalone runtime policy distinct from Eggsec authorization.
- Restore automated surfaces only after positive safe-script and negative unsafe-script evidence.
- Permit incremental residual reduction rather than requiring a 72-file rewrite before any safe automation is possible.

## Considered options

### Option A — Rewrite all residual protocol libraries before any automated activation

Benefits:

- simplest conceptual final state;
- maximum provider/accounting consistency.

Costs:

- very large milestone spanning dozens of unrelated protocols;
- several libraries require unconnected/broadcast UDP, async Tokio I/O, or native socket handoff to external libraries and do not fit the current provider handle contract;
- high compatibility-regression risk;
- delays safe use of already-provider-backed libraries.

Rejected as the M007 entry strategy. Residual migration remains incremental and guard-driven.

### Option B — Add capability checks at every residual direct connect and re-enable automation

Benefits:

- smaller diffs;
- immediate reduction of the ungated count.

Costs:

- direct hostname checks do not by themselves preserve resolved endpoint identity;
- direct I/O still bypasses provider cancellation/accounting;
- async/direct native socket paths remain outside the scoped provider boundary;
- a file-level check does not prove every operation is safe.

Rejected as sufficient for automated eligibility. Such libraries remain manual-only until their relevant effects are provider-backed or otherwise qualify under an explicit automated-safe contract.

### Option C — Effect-gate the automated library environment, migrate compatible cohorts incrementally, and activate only through scope-bearing approved execution

Selected.

## Decision

M007 SHALL use the following boundary.

### 1. Every runtime library exposed to automated profiles has an explicit effect/eligibility classification

The runtime must maintain a complete classification for every library/global registered into the Lua environment, not only the existing 43-entry compatibility registry.

At minimum the classification distinguishes:

- `Pure`: no host side effects;
- `ProviderBacked`: all automated-relevant side effects route through the capability-aware provider broker;
- `ManualOnlyDirectIo`: direct host I/O remains;
- `ManualOnlyAdvisory`: some capability consultation exists but direct effects remain outside provider cancellation/accounting/authority.

Unknown/unclassified libraries are manual-only by default.

The existing M005E inventories are inputs to this classification, not an alternative source of truth.

### 2. Automated profiles receive an effect-gated Lua environment

For `AgentSafe` and `CiSafe`:

- `Pure` and eligible `ProviderBacked` libraries may be registered/exposed;
- direct-I/O/advisory/unknown libraries are absent or fail closed before script code can invoke them;
- dynamic `require()` and direct global access must have the same result;
- manual profiles retain compatibility unless their existing capability policy denies an operation.

A require-only gate is insufficient because libraries are pre-registered as Lua globals.

### 3. Automated HTTP requires an authority-bound provider

The native standalone HTTP provider is not sufficient evidence of authority binding for automated hostname/redirect execution.

The per-run service bundle must carry an explicit runtime-owned assurance that the injected HTTP provider is authority-bound for host resolution/socket/redirect decisions. Safe defaults do not assert this.

Automated HTTP fails closed unless:

- the provider is explicitly authority-bound; or
- the runtime later implements equivalent resolve-authorize-bind semantics internally.

Eggsec's `NseHttpTransportProvider`, backed by `HttpTransport + NetworkAuthority`, is the initial authority-bound consumer.

### 4. Eggsec automated NSE requires `ApprovedExecution`, never a scope-less approval token

After re-enablement:

- `execute_approved()` remains fail-closed for NSE;
- `execute_approved_execution()` is the strict NSE entry because it carries the exact scope snapshot;
- manual CLI/TUI paths may continue using the manual path.

The existing general strict-execution quarantine may be relaxed for other newly exposed operations, but NSE must explicitly require the scope-bearing bundle.

### 5. Approval-time target facts are retained for execution

`ApprovedExecution` must carry the normalized/resolved `TargetScope` facts used for approval, when a target exists.

Strict NSE profile construction uses those same facts rather than resolving the primary target again merely to decide runtime policy.

This is binding metadata, not a new authorization decision.

### 6. Eggsec policy profile maps to runtime profile conservatively

Initial mapping:

- `McpStrict` -> `AgentSafe`;
- `AgentStrict` -> `AgentSafe`;
- `CiStrict` -> `CiSafe`;
- `ManualGuarded` -> `ManualStrict` when the scope-bearing path is used manually;
- `ManualPermissive` remains on the existing manual path and is never selected for automated execution.

For strict networked execution, the runtime network policy should initially be no broader than the exact approved primary-target address set. Eggsec `ScopeAuthority` remains the authoritative broader scope language.

Do not flatten hostname patterns, exclusions, and port rules into a lossy runtime CIDR approximation.

### 7. Scoped host-service injection is engine-owned

Eggsec may compose:

- the authority-bound HTTP provider;
- scoped DNS/TCP/UDP provider wrappers where required;
- native deterministic/local providers for non-network domains.

All Eggsec-specific authority adapters remain in Eggsec. `eggsec-nse` remains free of Eggsec dependencies.

### 8. Automated exposure is restored only after selective qualification

MCP/REST/agent/gRPC NSE metadata may be re-enabled only after:

- the automated library environment is fail-closed;
- strict NSE requires `ApprovedExecution`;
- scoped services/profile mapping are active;
- at least one provider-backed positive script path succeeds;
- at least one residual/direct-I/O script is rejected before host contact;
- guard coverage prevents unsafe library eligibility from silently expanding.

CI continues to use `CiSafe`, which has no network access.

## Consequences

### Positive

- Automated NSE can return without waiting for a full 72-file rewrite.
- Unsafe compatibility libraries remain available manually but are structurally absent from automated environments.
- Dynamic `require()` and direct-global bypasses are both covered.
- Eggsec scope identity flows from approval to execution without target re-resolution.
- The existing provider work becomes useful without being mistaken for universal protocol coverage.
- Residual migration becomes measurable: promotion from manual-only to provider-backed eligibility is an explicit event.

### Negative

- Some automated scripts that worked accidentally under permissive/manual execution will now fail because an unsafe dependency is intentionally unavailable.
- The library effect manifest becomes a maintained security artifact.
- Initial strict network scope is intentionally narrower than the full Eggsec scope for raw TCP/UDP unless scoped provider adapters broaden it safely.
- HTTP provider assurance adds another integration contract that must be tested.

### Deferred

- Full migration of every residual direct-I/O library.
- Raw packet/dnet/pcap parity where current provider interfaces do not cover the operation.
- Broadening strict runtime policy to multiple arbitrary scope targets.
- Removal of the manual compatibility APIs in `public_api`.

## Security implications

Unknown library effect status is deny-by-default for automated profiles.

An automated-safe classification is stronger than "contains a capability check": it requires provider-backed or otherwise authority-preserving execution for the effects the library exposes.

No HTTP provider may be labeled authority-bound merely because it performs a pre-request capability check; hostname resolution, concrete socket selection, redirects, proxy endpoints, and TLS identity all belong to the authority claim.

The scope-less strict entry must reject NSE even after operation metadata is re-exposed.

## Verification

M007 closure must include:

- complete registered-library effect manifest coverage;
- negative tests for unknown/direct/advisory libraries under AgentSafe/CiSafe;
- direct-global and dynamic-require bypass tests;
- HTTP native-provider denial vs authority-bound-provider success under automated profile;
- approval-time target-fact binding tests;
- strict profile mapping tests;
- exact scope snapshot/authority propagation tests;
- safe automated script success and unsafe residual script zero-host-contact denial;
- architecture guards preventing production adapter fallback/native construction;
- current M005E residual counts plus promoted-file deltas.

## Supersession

None.
