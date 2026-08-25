# Phase B Plan: Scope Resolution and Address-Set Correctness

## Status

Status: Executed.

Executed. Changes landed in main.

## Objective

Separate target parsing, DNS resolution, address classification, and scope
authorization so Eggsec makes deterministic, auditable decisions for literal IPs,
hostnames, URLs, IPv4/IPv6, loopback/private ranges, and multi-address targets.

The phase must eliminate authorization behavior that depends on the first
resolver result and make explicitly scoped local targets behave consistently.

## Dependencies

Phase A must land first or in a tightly coordinated branch because this phase
extends the target identity bound into `ApprovedOperation`. It must not re-open
Phase A's operation/target construction decisions unless a concrete address-set
requirement makes a small adjustment necessary.

## Scope

Primary areas:

```text
crates/eggsec/src/config/scope.rs
crates/eggsec/src/config/policy_decision.rs
crates/eggsec/src/config/policy.rs
crates/eggsec/src/config/http.rs
crates/eggsec/src/utils/
crates/eggsec/src/scanner/
crates/eggsec/src/recon/
crates/eggsec/src/tool/protocol/
crates/eggsec/src/runtime_bridge/
crates/eggsec-python/src/
crates/eggsec/tests/
```

Inspect actual connection creation points before finalizing the implementation
set. Scope checks that occur only in entrypoints are insufficient if lower-level
clients resolve or redirect independently.

## Non-goals

This phase does not:

- implement a custom recursive DNS resolver;
- add DNSSEC validation;
- guarantee protection against every rebinding attack across all external
  libraries;
- change authorized target sets without an explicit compatibility note;
- add active network tests against public infrastructure;
- require privileged networking or packet capture;
- add another scope manifest format.

## Target model

### Parsed target

Parsing should produce a target identity independent of resolution:

```text
original input
normalized scheme/host/port/path policy where applicable
literal IP or CIDR when present
target kind
```

### Resolution result

DNS resolution should return a structured result:

```text
normalized hostname
all unique IPv4/IPv6 addresses returned
resolver error, if any
resolution timestamp or generation where needed
```

The resolver must not reject loopback/private addresses. It reports facts;
policy decides whether they are authorized.

### Scope decision

Scope evaluation should record:

```text
matched allow rules
matched exclusion rules
resolved addresses evaluated
address classes encountered
whether all/any/no addresses are authorized
reason for denial or confirmation
```

The default strict rule for network execution should be conservative: every
address that may be selected by the client must be within the approved scope.
Any alternative, such as pinning one approved address, must bind the connection
to that exact address and preserve the original hostname for TLS/SNI.

## Workstream 1 — Inventory resolution and connection behavior

Identify:

- every use of `ToSocketAddrs`, Hickory, reqwest URL resolution, raw socket
  connection, proxy connection, redirect following, and headless browser
  navigation;
- which clients resolve internally after policy approval;
- whether clients can be configured to pin or override DNS results;
- where redirect targets are re-authorized;
- where IPv6 and dual-stack results are handled;
- whether Python wrappers repeat scope checks or rely on the engine.

Record the chosen policy per client class in the implementation PR description
or updated architecture documentation. Do not create a generated inventory.

## Workstream 2 — Decouple resolution from policy

Refactor `TargetScope::resolve_host()` or its replacement so it:

- returns all unique addresses rather than the first;
- does not reject loopback, private, link-local, multicast, or unspecified
  addresses as a resolver concern;
- preserves resolution errors distinctly from invalid target syntax;
- supports deterministic ordering for audit/testing without implying preference;
- does not perform synchronous blocking DNS on async hot paths when an existing
  async resolver is available.

A small resolver trait is acceptable if it allows deterministic unit tests and
shared engine behavior. Avoid a generalized pluggable DNS subsystem.

## Workstream 3 — Define address authorization semantics

For each target class, implement and document:

### Literal IP

Evaluate the literal directly. Explicit allow and exclusion rules take
precedence. Loopback/private defaults must match documented manual and strict
profiles.

### Hostname without CIDR rules

Hostname pattern rules may authorize the identity, but strict network execution
must still classify resolved addresses. A public hostname resolving to a private,
loopback, link-local, multicast, or unspecified address must not silently bypass
private-resolution policy.

### Hostname with CIDR rules

Evaluate all resolved addresses against CIDR allow/exclusion rules. Do not fail
solely because an explicitly allowed hostname resolves to loopback; let the rule
and profile decide.

### Mixed public/private or allowed/denied results

Fail closed for strict surfaces unless the client is provably pinned to an
approved address. Manual surfaces may require the existing explicit private or
scope override classes; they must not silently proceed.

### Unresolved hostname

A hostname-only scope explanation may still report pattern matching, but a
network execution path requiring address authorization must not treat resolution
failure as approval.

### URL redirects and canonicalization

Re-run target authorization when redirects change host, effective port, or
scheme policy. Preserve the existing cross-host redirect confirmation/denial
classes.

## Workstream 4 — Bind resolution decisions to execution

Extend the Phase A approval binding with the smallest address information needed
to prevent time-of-check/time-of-use drift:

- approved hostname identity;
- approved address set or approved pinned address;
- resolution generation/timestamp when useful for diagnostics;
- redirect policy.

Preferred execution approaches, in order:

1. pin the network connection to an approved address while retaining hostname
   semantics for TLS/SNI;
2. configure the client resolver with the approved address set;
3. immediately re-resolve and compare against the approved set before connection
   when pinning is unavailable.

Do not claim complete rebinding resistance for adapters that cannot bind or
recheck. Mark those paths and keep them out of strict automated exposure until a
sound mechanism exists.

## Workstream 5 — Normalize loopback and local-target behavior

Add explicit tests and behavior for:

```text
localhost
localhost.
127.0.0.1
127.0.0.0/8
::1
IPv4-mapped loopback addresses
private RFC1918 ranges
IPv6 ULA
link-local IPv4/IPv6
```

An explicitly allowed `localhost` rule must not become unusable merely because a
scope file also contains a CIDR rule. Conversely, a hostname resolving to
loopback must not be treated as public because only the hostname pattern was
checked.

If current manual defaults intentionally allow literal loopback, preserve that
behavior consistently for `localhost` unless the documented policy is changed
through an explicit decision.

## Workstream 6 — Improve decision records

Populate `PolicyDecision.resolved_addresses` or a replacement field with the
actual evaluated set. Include stable reason codes rather than relying only on
substring classification of human-readable denial messages.

Where possible, replace `classify_denial_reasons()` string inspection for scope
classes with typed decision reasons produced during evaluation. Do not broaden
this phase into a full error-system rewrite.

## Workstream 7 — Test with deterministic fake resolution

Add a test resolver supporting scenarios such as:

- one public address;
- one allowed private address;
- public plus private;
- allowed plus explicitly excluded;
- IPv4 plus IPv6;
- loopback hostname;
- no addresses;
- resolver error;
- changed address set between approval and execution.

Tests must not rely on external DNS behavior or public hosts.

## Validation commands

```bash
cargo fmt --all -- --check
cargo test -p eggsec --lib config::scope
cargo test -p eggsec --test enforcement_matrix
cargo test -p eggsec --test enforced_dispatch_regression --features rest-api
cargo test -p eggsec --test tool_registration --features rest-api
cargo check -p eggsec --features rest-api,grpc-api
make check-python   # when Python target/scope behavior changes
```

Run focused integration tests for reqwest, proxy, or browser adapters only when
those paths are changed. Avoid introducing Internet-dependent CI tests.

## Compatibility and migration

- Preserve existing scope file syntax.
- Preserve current allow/exclude precedence unless a documented bug requires a
  correction.
- Add typed fields to serialized policy results compatibly where possible.
- When behavior changes for ambiguous multi-address hostnames, document it as a
  security correction.
- Keep old single-address helpers private/deprecated during migration and remove
  them before phase completion.

## Rollback considerations

If client-level DNS pinning proves incompatible with a particular adapter, retain
the new all-address policy evaluation and disable strict automated exposure for
that adapter rather than reverting to first-address authorization.

## Acceptance criteria

1. Resolver code returns all unique addresses.
2. Resolver code does not itself authorize or reject address classes.
3. Literal and hostname forms of loopback/private targets follow one documented
   policy.
4. An explicitly allowed `localhost` can be evaluated even when CIDR rules exist.
5. Strict surfaces reject unresolved network targets.
6. Strict surfaces reject mixed allowed/denied address sets unless execution is
   pinned to the approved subset.
7. Explicit exclusions win for any address the client may use.
8. Policy decisions record the evaluated address set and typed denial classes.
9. Redirect host changes trigger re-authorization.
10. Approval/execution detects address-set drift for supported strict adapters.
11. Tests use deterministic fake resolution, not public DNS.
12. Existing scope manifest syntax remains valid.
13. No capability is removed from manual use solely because one strict adapter
    cannot bind DNS safely.
14. No package or release is published.
