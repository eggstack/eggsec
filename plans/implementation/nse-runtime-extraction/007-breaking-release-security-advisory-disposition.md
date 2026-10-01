# NSE Runtime Extraction Milestone 007C-S — Security Advisory and Predecessor-Version Disposition

Status: blocked (draft preparation complete — see `plans/closure/nse-runtime-extraction/007c-s-closure.md`; advisory publication and 0.1.0/0.2.0 yank execution pending verified `0.3.0` from M007C-R)

Eggsec planning baseline: `39603233d8b549d4b525203ba0869b40e5696569`

Standalone release baseline: `eggstack/eggsec-nse@d4a22f1dbe56f4ccfb17b2a8135aae8395f44f19`

Primary release plan:

- `plans/implementation/nse-runtime-extraction/007-breaking-0-3-0-release.md` (M007C-R)

Security evidence:

- `plans/closure/nse-runtime-extraction/007c-closure.md`
- `plans/closure/nse-runtime-extraction/007c-semver-report.txt`
- `plans/closure/nse-runtime-extraction/007b-closure.md`

Applicable ADRs:

- `plans/adrs/ADR-0004-nse-automated-activation-boundary.md`
- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`

Primary class: security communication + release operations

Affected repository/package:

- repository: `eggstack/eggsec-nse`
- ecosystem: Rust / crates.io
- package: `eggsec-nse`

## 1. Objective

Prepare and, only after the fixed release is publicly verified, publish an accurate security advisory for the capability-bypassing public TCP helper API present in released `eggsec-nse` versions, then make and record an explicit yank/no-yank decision for every affected predecessor version.

This plan is a companion to M007C-R. It does not publish `0.3.0`; it ensures the security meaning of that release is communicated with the correct affected-version range and without overstating exploitability.

## 2. Correct affected-version range

The current M007C-R plan discusses bypass-bearing `0.2.0`, but source inspection of immutable release tags proves the same public helper API is present in both public releases:

### `v0.1.0`

`src/libraries/helpers.rs` exports:

- `pub fn tls_connect(...)`
- `pub fn tcp_connect_with_timeout(...)`

and both reach `std::net::TcpStream::connect_timeout` directly.

### `v0.2.0`

The same public functions and direct-connect implementation remain.

### current M007 tree

Both functions are absent, along with the associated pure helper functions `make_addr` and `parse_socket_addr`.

Therefore the advisory candidate range is:

```text
affected: >= 0.1.0, < 0.3.0
fixed:    0.3.0
```

if `0.3.0` publishes from the qualified M007C-R candidate without reintroducing the helpers.

Do not publish an advisory that lists only `0.2.0`.

## 3. Security statement and threat-model discipline

The advisory must describe exactly what the defect is:

- published Rust API exposed helpers that opened raw TCP sockets directly;
- those helpers did not receive `NseCapabilityContext`;
- they did not execute through `NseHostServices` or the provider broker;
- therefore a consumer invoking them could bypass the runtime's capability decision, cancellation, accounting, and provider-selection boundary for that connection.

The advisory must **not** claim, without additional evidence, that:

- arbitrary remote attackers can call these Rust functions;
- every Lua/NSE script can reach them directly;
- every `eggsec-nse 0.1.0/0.2.0` consumer is remotely exploitable;
- Eggsec's quarantined automated NSE path exposed these functions as a supported operation.

The impact statement should be framed around embedders that relied on the crate's capability/provider boundary while exposing or internally invoking these public helpers.

Severity/CVSS must be derived from that actual threat model. Do not infer a severity from the word "bypass" alone.

## 4. GitHub Security Advisory workflow

GitHub repository security advisories support the Rust/crates.io ecosystem and allow affected and fixed versions to be encoded for advisory-database/Dependabot processing.

Use the following sequence.

### Work package A — Prepare a draft advisory before release

After M007C-R has a frozen release candidate but before public disclosure:

- create a draft repository security advisory;
- package/ecosystem: Rust / `eggsec-nse`;
- affected range: `>= 0.1.0, < 0.3.0`;
- fixed version: leave unpublished until `0.3.0` is actually available, or record the candidate as planned text only if the UI requires a published fixed version;
- vulnerable functions: `helpers::tls_connect`, `helpers::tcp_connect_with_timeout`;
- include the capability-boundary impact statement from §3;
- include migration guidance: move connection work to broker/provider APIs; do not recreate raw socket helpers.

Do not publish the advisory while `0.3.0` is unavailable unless an active exploitation/urgent disclosure reason is separately documented.

GitHub recommends publishing security advisories with a fix version when possible so dependency tooling can point users to a safe upgrade.

### Work package B — Bind the advisory to the verified fixed release

After M007C-R proves:

- crates.io `0.3.0` resolves;
- tag/source/archive identity matches;
- scratch consumers build;

then update the advisory with:

```text
Affected: >= 0.1.0, < 0.3.0
Fixed:    0.3.0
```

and references to the `v0.3.0` release/migration notes.

### Work package C — Publish the advisory

Publish only after the fixed artifact is available.

Record:

- GHSA identifier;
- whether a CVE was requested/assigned;
- publication timestamp;
- exact affected/fixed range;
- severity vector/rationale if supplied;
- whether GitHub accepted/reviewed it into the global advisory database.

A CVE is optional; do not make M007D depend on CVE assignment.

## 5. Predecessor-version yank decision

Cargo yanks do not delete crate contents. They prevent new dependency resolution from selecting the yanked version while existing lockfiles can continue using it.

That makes yanking a meaningful but non-destructive control for vulnerable predecessor releases.

### Versions requiring disposition

- `0.1.0`
- `0.2.0`

No version may be silently left without an explicit decision.

### Preferred decision rule

After `0.3.0` is verified:

**Prefer yanking both `0.1.0` and `0.2.0`** if all of the following hold:

- the advisory concludes the raw-connect API violates a security guarantee users could reasonably rely on;
- `0.3.0` is available and builds for the supported feature combinations;
- no identified downstream requires a fresh `0.1/0.2` resolution as its only viable migration path.

At planning time, public GitHub code search produced no external Cargo.toml hits for `eggsec-nse 0.1` or `0.2`. Treat that as weak evidence only; re-check crates.io/GitHub dependents immediately before the yank decision.

If either old release is intentionally left unyanked:

- record the specific compatibility reason;
- keep the security advisory affected range unchanged;
- state prominently that the version remains affected;
- do not present "not yanked" as "safe".

### Yank order

If the decision is to yank:

1. verify `0.3.0` registry availability and scratch consumption;
2. publish/update the advisory with fixed version `0.3.0`;
3. yank `0.1.0` and/or `0.2.0`;
4. verify registry metadata reflects the yank;
5. verify an existing lockfile can still resolve/build the yanked version if that compatibility property is material to the decision;
6. verify a fresh unconstrained/compatible resolution does not select the yanked version.

Do not yank before the fixed artifact is usable unless a separate emergency-disclosure rationale is recorded.

## 6. Security communication surfaces

Reconcile the same security message across:

- GitHub Security Advisory;
- `v0.3.0` GitHub Release notes;
- `CHANGELOG.md`;
- `docs/PROVIDERS.md`;
- `docs/RELEASING.md` release-history/security note.

The message must distinguish:

1. the public Rust helper bypass fixed by removing the two direct-connect helpers;
2. the broader M007 automated-library effect gating;
3. the remaining 22 specialized direct-I/O/manual-only residuals;
4. the still-deferred M007D DNS scope-binding work.

Do not collapse these into a claim that `0.3.0` provides universal NSE protocol scope enforcement.

## 7. Relationship to M007C-R closure

M007C-R may perform artifact publication, but its final closure must not declare the security release fully communicated until this plan has an explicit disposition.

Acceptable sequencing:

```text
freeze 0.3.0 candidate
        |
        +--> prepare draft advisory
        |
qualify/publish 0.3.0
        |
        +--> verify registry/tag/archive/docs
        |
        +--> publish advisory with fixed=0.3.0
        |
        +--> yank/no-yank decisions for 0.1.0 + 0.2.0
        |
        v
007C-R / 007C-S closure
        |
        v
M007D adoption
```

M007D should not be blocked on GitHub Advisory Database review/CVE issuance after the advisory itself is published and the fixed artifact is verified.

## 8. Required verification

Before advisory publication:

- compare `v0.1.0` and `v0.2.0` helper source;
- verify both direct-connect functions are absent from the 0.3.0 candidate;
- verify boundary guards fail if either helper is restored;
- verify the M007C-R semver report still contains the helper removals.

After publication:

- verify advisory affected range includes both released vulnerable versions;
- verify fixed version is exactly 0.3.0;
- verify advisory references the correct release;
- verify crates.io yank metadata for each predecessor matches the recorded decision.

## 9. Acceptance criteria

1. The affected range is correctly recorded as `>= 0.1.0, < 0.3.0` unless new release-history evidence changes it.
2. The advisory names the two direct-connect public helpers accurately.
3. Impact language distinguishes library-boundary bypass from remote exploitability.
4. Fixed version is not published in the advisory until 0.3.0 is actually available.
5. A repository security advisory is published after the fixed artifact is verified, unless closure records a concrete reason not to publish one.
6. CVE request/assignment is explicitly decided but not required for M007D.
7. `0.1.0` has an explicit yank/no-yank decision and rationale.
8. `0.2.0` has an explicit yank/no-yank decision and rationale.
9. If yanked, registry state is verified and fresh-resolution behavior is checked.
10. Release notes/changelog/advisory do not overclaim universal provider coverage.
11. Closure records the GHSA identifier or the explicit no-advisory rationale.
12. M007D is unblocked only after the fixed `0.3.0` artifact exists and this security-disposition record is complete.

## 10. Stop conditions

Stop and reconcile if:

- `0.3.0` is not the first release without the raw-connect helper API;
- any helper is restored in the candidate;
- advisory affected versions omit a known vulnerable release;
- the impact statement requires assumptions not demonstrated by source/tests;
- a yank would strand the principal consumer before M007D can adopt `0.3.0`;
- security communication claims M007D scope binding is already complete.

## 11. Closure evidence required

Create:

- `plans/closure/nse-runtime-extraction/007c-s-closure.md`

Record:

- source evidence from `v0.1.0`, `v0.2.0`, and the fixed candidate;
- advisory draft/publication identity;
- affected/fixed version range;
- severity/CVSS rationale if used;
- CVE decision/status;
- yank/no-yank decision for 0.1.0;
- yank/no-yank decision for 0.2.0;
- crates.io state after disposition;
- release/advisory/changelog consistency check;
- remaining security limitations;
- GO/NO-GO for M007D from the security-communication perspective.

## 12. Handoff notes

The release plan already owns code/version/package/tag publication. Do not duplicate that work here.

This companion plan exists because the security range is broader than the release plan's current shorthand: the raw-connect helper API is present in both immutable public releases, not only 0.2.0.

GitHub advisory publication should follow the fixed artifact, not precede it, unless a separately documented emergency disclosure decision overrides normal sequencing.
