# Eggress 1.0.8 post-adoption compatibility and guard corrective pass

Status: Ready for handoff

Date: 2026-09-22

Eggsec corrective baseline: baea14e232a0cad570b971fa2453ad6fd83358b6

Parent implementation:

- roadmap: plans/eggress-1.0.8-adoption-roadmap-2026-09-22.md
- Phase A: plans/eggress-1.0.8-phase-a-proxy-engine-adoption-2026-09-22.md
- Phase B: plans/eggress-1.0.8-phase-b-health-qualification-and-closure-2026-09-22.md
- implementation SHA: d9749ccda6023322fccf54934d615db45f8de46b
- documentation/completion SHA: baea14e232a0cad570b971fa2453ad6fd83358b6
- hosted CI on the corrective baseline: CI run 35780421027 successful for Rust,
  dependency policy, and Python; Code Quality push run also successful.

## Purpose

Correct four bounded compatibility and closure defects found in review of the
successful Eggress 1.0.8 migration without reopening the selected architecture:

1. Restore the pre-adoption proxy-endpoint address acceptance boundary instead
   of silently allowing Eggress to DNS-resolve proxy hostnames that previously
   failed ProxyEntry::socket_addr().
2. Make Check 106 prove the exact approved direct Eggress dependency set rather
   than merely excluding known forbidden families.
3. Preserve the observable input/result ordering of
   HealthChecker::check_concurrent() while retaining O(concurrency) scheduling.
4. Reconcile the public ProxiedConnection.local_addr compatibility gap
   truthfully. Eggress 1.0.8 does not expose the established socket local
   address, so Eggsec must not claim this field's historical semantics are
   currently preserved.

This is a corrective compatibility and guard pass. The core 1.0.8 decision
remains accepted: eggsec-web-proxy uses listener-free eggress-outbound for
production proxy dialing, Reqwest remains health-only, and the canonical
scope-aware HTTP transport stays Eggress-free.

## Review findings

### Finding 1 — proxy endpoint hostname acceptance widened

Before d9749cc, production SOCKS and HTTP CONNECT paths resolved the proxy
endpoint through ProxyEntry::socket_addr(), which parses address:port as a
SocketAddr. Therefore the configured proxy endpoint had to be a socket/IP
literal. A value such as proxy.example.test:1080 failed before any network
activity.

The new adapter currently copies ProxyEntry.address directly into Eggress
EndpointSpec and lets Eggress execute it. Eggress may resolve a domain-valued
hop endpoint internally. This silently broadens accepted configuration and adds
new DNS/network behavior at the proxy-hop boundary.

The Phase A completion statement that DNS semantics were preserved is therefore
too strong.

Required disposition: preserve the pre-adoption literal endpoint contract in
this corrective pass. Hostname-valued proxy endpoints may be considered later
only through a separately authorized and documented design.

### Finding 2 — Check 106 is not an exact allowlist

Check 106 currently proves that eggsec-web-proxy contains the two intended
pinned dependencies, other Eggsec crates contain no Eggress dependencies, and
several heavy or advanced Eggress families/features remain forbidden.

However, the entire web-proxy manifest is excluded from the generic other-edge
check. A future direct dependency on an unlisted eggress-* crate could therefore
pass unless its name happens to match the explicit forbidden regex.

Required disposition: Check 106 must prove that the complete direct Eggress
dependency set in eggsec-web-proxy is exactly:

- eggress-outbound
- eggress-uri

with the already-required version and feature constraints.

### Finding 3 — concurrent health result order changed

Before Phase B, check_concurrent() used join_all(handles), which returned
results in input order.

The current bounded implementation uses buffer_unordered(concurrency), which
correctly limits live work but returns results in completion order. Internal
background pool updates key on proxy_url, so current internal behavior is safe,
but ProxyHealth.results is public and ordering is observable.

Required disposition: retain bounded O(concurrency) execution while restoring
enabled-input ordering.

### Finding 4 — ProxiedConnection.local_addr semantics regressed

Before adoption, the SOCKS and HTTP paths obtained the established stream's
actual TcpStream::local_addr().

Eggress 1.0.8 chain execution currently reports OutboundInfo.local_addr = None.
The migrated production paths therefore substitute 0.0.0.0:0 or [::]:0.

The field remains public and its type did not change, so this is not equivalent
behavior even though no current workspace consumer was found.

There is no clean Eggsec-only way to recover the actual local address from
Eggress 1.0.8 without reverting production dialing to duplicate legacy
handshakes, depending on private/downcast assumptions, duplicating connection
ownership around Eggress, or changing the public API. Those are rejected here.

Required disposition: keep the migration, explicitly classify this as
upstream-gated compatibility debt, ensure unknown metadata is never presented
as a measured address in docs/logs/evidence, and remove the previous all-
acceptance-criteria-met overstatement. Restore the real address only after a
published Eggress API supplies it.

## Global constraints

Preserve all of the following:

1. eggress-outbound remains exactly pinned to 1.0.8 with default features off.
2. eggress-uri remains exactly pinned to 1.0.8.
3. Only eggsec-web-proxy owns direct Eggress edges.
4. Production ProxyManager SOCKS, HTTP CONNECT, and chain execution remains
   Eggress-backed.
5. No direct fallback after a configured proxy failure.
6. Eggsec proxy pool, rotation, health, and selection remain authoritative.
7. Reqwest remains health-only.
8. eggsec-transport, eggsec-transport-eggfetch, core, and policy remain
   Eggress-free.
9. No advanced Eggress features or crates.
10. Current local-target versus SOCKS5/Tor remote-domain target semantics.
11. Credential redaction.
12. Public Rust, Python, CLI, TUI, and MCP schemas; this pass needs no schema
    change.
13. Rust 1.89 and ring-only TLS.
14. No Git, path, branch, or patch dependency workaround.
15. Interception/MITM remains out of scope.

## Workstream 0 — freeze the corrective baseline

Record the starting SHA, clean/dirty state, focused web-proxy tests,
proxy_adapter_smoke, architecture guards, and web-proxy dependency/feature
graphs.

Confirm the review findings directly against current source before editing:

- ProxyEntry::socket_addr() rejects hostname-valued endpoints.
- eggress_outbound::hop_from_entry() currently accepts the raw address.
- Check 106 does not enumerate the complete direct Eggress dependency set.
- check_concurrent() uses buffer_unordered.
- Eggress 1.0.8 OutboundInfo.local_addr is unavailable on chain execution and
  current production code emits an unspecified sentinel.

If any premise has changed since baea14e2, adapt the pass to current source and
record the difference.

## Workstream 1 — restore the proxy endpoint literal-address boundary

Make the Eggress adapter preserve the pre-adoption proxy endpoint acceptance
contract.

Preferred implementation:

1. Validate each ProxyEntry through ProxyEntry::socket_addr() at the adapter
   conversion boundary.
2. Build EndpointSpec.host from the validated SocketAddr IP literal, not the
   original arbitrary string.
3. Preserve the configured port.
4. Do not perform a second Eggsec DNS lookup to make hostname entries work.
5. Do not let Eggress resolve a proxy hostname in this path.

Expected behavior:

- IPv4/IPv6 literal proxy entry -> SocketAddr validation -> literal Eggress
  endpoint -> dial.
- hostname proxy entry -> explicit configuration/adapter error -> no proxy DNS
  and no proxy connection attempt.

Add regression tests for IPv4, IPv6, SOCKS5 hostname rejection, HTTP hostname
rejection, credential-safe errors, and preservation of SOCKS5/Tor
remote-domain target behavior after the proxy endpoint itself has been
validated as a literal.

Do not conflate proxy-endpoint DNS with target-domain-at-proxy semantics.

If a future release intentionally wants proxy hostnames, it needs a separate
authorization/resolution contract and migration plan.

## Workstream 2 — make Check 106 an exact direct-dependency allowlist

Strengthen the existing architecture guard.

The guard must parse or reliably inspect the dependencies section of
crates/eggsec-web-proxy/Cargo.toml and prove that the complete direct dependency
set whose package key/name begins with eggress is exactly:

- eggress-outbound
- eggress-uri

Retain the exact version/feature checks:

- eggress-outbound is exactly 1.0.8.
- default features are disabled.
- eggress-uri is exactly 1.0.8.
- no approved optional Eggress features are enabled.

Also retain the repository-wide constraints that every other workspace
manifest has zero direct Eggress dependency, external Eggress crate references
stay confined to the internal adapter/tests, transport/Eggfetch/policy/core stay
clean, and the decision record identifies the accepted release.

Prefer a small Python tomllib manifest check over a growing negative regex.
The repository already uses manifest-aware Python in adjacent guards.

Failure output should print any unexpected direct dependency names.

Update docs/CI_ARCHITECTURE_GUARDS.md to describe the exact allowlist.

## Workstream 3 — restore concurrent health result ordering without losing bounds

Replace completion-order collection with bounded execution that preserves the
order of enabled input proxies.

Preferred implementation is stream::iter(...).map(...).buffered(concurrency)
unless there is a concrete reason to retain unordered internal completion and
reorder by index afterward.

Required properties:

- at most max(1, concurrency) checks in flight;
- no spawn-per-proxy JoinHandle retention;
- disabled proxies omitted exactly as before;
- one result per enabled proxy;
- results[i] corresponds to the i-th enabled input proxy;
- healthy/unhealthy totals unchanged;
- timeout, error, redaction, and no-direct-fallback semantics unchanged.

Add a deterministic test where proxy A completes after proxy B but the returned
vector remains A then B.

Do not revert to unbounded spawning merely to regain ordering.

## Workstream 4 — reconcile local_addr truthfulness and closure records

### Eggsec source behavior

Keep the current Eggress-backed production path. Do not restore duplicate
legacy handshakes solely to recover local_addr.

Because the public field is a non-optional SocketAddr, there is no
backward-compatible in-place representation for unknown. Until upstream support
exists:

- centralize sentinel construction in one helper instead of three call sites;
- name and comment it explicitly as unknown/unspecified metadata;
- do not log it as a measured local socket address;
- do not use it for routing, authorization, evidence claims, or policy;
- add a regression check proving production authorization/routing do not branch
  on it;
- record the exact removal condition.

Removal condition: a published Eggress release exposes the actual local
SocketAddr for the established outbound chain connection.

If inspection reveals a safe public Eggress 1.0.8 API already provides this
value without private/downcast assumptions, use it and restore the value now.
Do not infer or synthesize it from peer_addr.

### Planning/completion record correction

Append a post-adoption corrective note to the roadmap and Phase A/B records.
Do not rewrite their historical completion text.

The note must state:

- the 1.0.8 architecture adoption remains accepted;
- proxy endpoint hostname acceptance was unintended semantic broadening and is
  corrected by this pass;
- local_addr compatibility remains upstream-gated unless safely restored;
- prior statements that DNS semantics were fully preserved and all acceptance
  criteria were met are superseded by this corrective finding;
- Check 106 and health-order behavior were hardened by this pass.

Update plans/README.md so the campaign entry links this corrective pass and
does not present the earlier completion as unconditional final closure.

## Workstream 5 — current-state documentation reconciliation

Update only documentation affected by these corrections:

- architecture/egress_reuse_decision.md
- architecture/proxy.md
- docs/CI_ARCHITECTURE_GUARDS.md
- .opencode/skills/eggsec-proxy/SKILL.md if needed
- roadmap and Phase A/B records via appended follow-up notes
- plans/README.md

Document explicitly that proxy endpoint configuration remains literal-address-
only in this release, SOCKS5/Tor target remote-domain behavior remains
supported as a separate concern, local_addr is currently unknown on
Eggress-backed production paths unless upstream exposes it, Check 106 permits
exactly two direct Eggress crates, and concurrent health results retain
enabled-input order.

## Required verification

Focused:

- cargo fmt --all --check
- cargo check -p eggsec-web-proxy --no-default-features
- cargo check -p eggsec-web-proxy --features web-proxy
- cargo test -p eggsec-web-proxy -- --test-threads=1
- cargo test -p eggsec --features web-proxy --test proxy_adapter_smoke -- --test-threads=1
- bash scripts/check-architecture-guards.sh
- make check-deps

Repository gates:

- make check-feature-profiles
- make check-features-individual
- make check
- make check-msrv

Because this touches production Rust plus guards/docs, normal hosted CI on the
final pushed SHA must be green and its run ID/conclusions recorded.

## Expected files touched

Likely:

- crates/eggsec-web-proxy/src/eggress_outbound.rs
- crates/eggsec-web-proxy/src/lib.rs
- crates/eggsec-web-proxy/src/health.rs
- crates/eggsec-web-proxy/tests/eggress_parity.rs
- crates/eggsec-web-proxy/tests/health_matrix.rs
- scripts/check-architecture-guards.sh
- docs/CI_ARCHITECTURE_GUARDS.md
- architecture/egress_reuse_decision.md
- architecture/proxy.md
- .opencode/skills/eggsec-proxy/SKILL.md
- plans/eggress-1.0.8-adoption-roadmap-2026-09-22.md
- plans/eggress-1.0.8-phase-a-proxy-engine-adoption-2026-09-22.md
- plans/eggress-1.0.8-phase-b-health-qualification-and-closure-2026-09-22.md
- plans/README.md
- this plan

No Cargo dependency version change is expected.

## Non-goals

- no Eggress version bump;
- no upstream Eggress source change in this repository;
- no proxy-hostname support;
- no new resolver abstraction;
- no eggsec-transport changes;
- no removal of Reqwest health probes;
- no mixed-chain public capability expansion;
- no Https proxy-type semantic redesign;
- no removal of public TcpStream compatibility shims;
- no interception/MITM refactor;
- no public ProxiedConnection schema change;
- no attempt to infer local socket metadata from the proxy peer;
- no advanced Eggress crate or feature.

## Stop conditions

Stop and record the exact blocker rather than weakening invariants if:

- preserving literal proxy endpoints would require bypassing the existing
  ProxyEntry::socket_addr() contract;
- Eggress internally re-resolves an IP literal into a different destination;
- exact dependency allowlisting cannot be made reliable with repository
  tooling;
- preserving health result order requires unbounded task creation;
- restoring a real local_addr requires an unsafe/downcast/private API,
  duplicate production handshake, or public breaking API change.

The local_addr stop condition is expected on Eggress 1.0.8 and is not a reason
to roll back the otherwise-correct adoption.

## Acceptance criteria

This corrective pass is complete only when:

1. Hostname-valued proxy endpoints no longer become newly accepted production
   dial targets through Eggress.
2. IPv4 and IPv6 literal proxy endpoints continue to work.
3. Target remote-domain SOCKS5/Tor behavior remains intact.
4. Tests prove hostname proxy rejection occurs before network behavior
   attributable to that endpoint.
5. Check 106 proves the exact direct Eggress dependency set consisting only of
   eggress-outbound and eggress-uri.
6. An unexpected third direct Eggress dependency would fail the guard.
7. Concurrent health checks remain O(concurrency).
8. Concurrent health results preserve enabled-input order.
9. Health totals, redaction, no-direct-fallback, and application-level
   semantics remain intact.
10. local_addr sentinel handling is centralized and explicitly documented as
    unknown metadata, or the real value is restored through a safe public API.
11. No policy, authorization, routing, evidence, or security decision treats
    the sentinel as a real address.
12. The upstream condition required to restore actual local_addr is recorded
    if still blocked.
13. Roadmap, Phase A, and Phase B records contain appended corrective notes
    rather than rewritten history.
14. Current architecture and guard docs match corrected behavior.
15. The Eggress architecture boundary remains unchanged.
16. Focused tests, dependency policy, feature profiles, make check, and MSRV
    are green.
17. Normal hosted CI on the final pushed SHA is green.
18. This plan contains the final SHA, exact verification evidence, and any
    remaining upstream-gated local_addr debt.

## Completion record

Not yet executed.
