# NSE Milestone 3 Phase 05: Time, Randomness, Crypto, Compression, and Accounting

## Purpose

Bring lower-risk but still security-relevant helper classes under capability accounting and report semantics: time, randomness, environment, crypto/TLS utility operations, compression/decompression, and heavy pure-CPU helpers.

These helpers are usually less dangerous than filesystem/process/network operations, but they can affect determinism, consume CPU/memory, leak environment state, or weaken CI/agent reproducibility.

## Background

Milestone 3 Phases 03 and 04 focus on high-risk side effects. This phase finishes the helper-control model by covering lower-risk side effects and heavy compute paths.

## Non-Goals

Do not rewrite cryptographic implementations.

Do not remove random/time helpers from manual use.

Do not block benign pure functions.

Do not claim deterministic execution unless all sources of nondeterminism are controlled.

## Target State

By the end of this phase:

- Time and randomness helpers are profile-aware and reportable.
- Environment access is denied or redacted for automated profiles unless explicitly allowed.
- Crypto and compression helpers perform accounting and cancellation checks around heavy operations.
- Reports expose nondeterminism and heavy-helper warnings where relevant.
- CI-safe mode can run deterministic compatibility tests without hidden time/random/env variability.

## Workstream 1: Time and Clock Helpers

### Scope

Inspect helpers using:

- `SystemTime`
- `Instant` for user-visible output
- `chrono`
- `time`
- `os.date` equivalents
- Lua date/time wrappers

### Required Behavior

- Manual profiles allow real time by default.
- CiSafe should use deterministic/frozen time where feasible or mark output as nondeterministic.
- AgentSafe should allow only bounded time reads and report nondeterminism.
- Time reads should record capability events if they affect output.

### Acceptance Criteria

- Tests can run deterministic time behavior in CI-safe contexts where possible.
- Reports can indicate time nondeterminism.

## Workstream 2: Randomness Helpers

### Scope

Inspect helpers using:

- `rand`
- OS randomness
- random token/session/user-agent generation
- Lua random wrappers

### Required Behavior

- Manual profiles allow randomness.
- CiSafe should use seeded deterministic randomness or deny nondeterministic helpers.
- AgentSafe should report randomness use.
- Random byte/token counts should be accounted where practical.

### Acceptance Criteria

- CI-safe tests can avoid nondeterministic flakes.
- Reports indicate randomness use.

## Workstream 3: Environment Access

### Scope

Inspect helpers using:

- `std::env::var`
- process environment enumeration
- home-directory lookup used by compatibility code

### Required Behavior

- Manual profiles may allow environment access with reporting.
- AgentSafe and CiSafe should deny or strictly redact environment access unless explicitly required for safe local config.
- Existing path discovery that reads `HOME` should be classified carefully: if it is only used for manual Nmap path discovery, keep it manual-only or reportable.

### Acceptance Criteria

- Automated profiles do not leak arbitrary environment variables.
- Docs state which environment reads remain and why.

## Workstream 4: Crypto and TLS Utility Accounting

### Scope

Inspect helpers in:

- `openssl`
- `sslcert`
- `tls`
- hashing/encoding helpers
- certificate parsing

### Required Behavior

- Pure hashing/encoding can be allowed but should respect input size limits where available.
- Certificate parsing should check cancellation before and after large parse operations.
- TLS/network handshakes belong mostly to Phase 04, but local parsing/accounting can live here.
- Large crypto operations should affect compatibility/report warnings if bounded approximately.

### Acceptance Criteria

- Heavy crypto helpers have size/cancellation checks.
- Reports can include crypto/helper warnings when operations are denied or bounded.

## Workstream 5: Compression and Decompression Accounting

### Scope

Inspect helpers using:

- `flate2`
- `zlib`
- gzip/deflate helpers
- archive/decompression utility code

### Required Behavior

- Enforce input size and output expansion limits where possible.
- Check cancellation before and after compression/decompression.
- AgentSafe and CiSafe should deny or bound decompression of untrusted large inputs.
- Record bytes in/out.

### Acceptance Criteria

- Decompression bombs are bounded by explicit limits.
- Tests cover over-limit decompression rejection.

## Workstream 6: Heavy Pure-CPU Helpers

### Scope

Inspect parsing/matching helpers that can become expensive:

- regex and PCRE helpers;
- ASN.1 parsing;
- XML parsing;
- JSON parsing;
- large string processing;
- brute/credential helper loops where present.

### Required Behavior

- Add size/iteration limits where practical.
- Check cancellation before/after heavy Rust helper calls.
- Report approximate bounds or unsupported cases.

### Acceptance Criteria

- Obvious unbounded helper loops are classified and guarded.
- Tests cover at least one size-limit rejection.

## Workstream 7: Report Integration

### Required Report Additions

Ensure reports can reflect:

- time used;
- randomness used;
- environment access denied;
- crypto/compression helper bounded or denied;
- deterministic/approximate status.

Do this via capability events if Phase 02 added them, or via warnings/compatibility summaries if not.

## Workstream 8: Tests

Required tests:

- CI-safe deterministic/no-time behavior where implemented;
- randomness warning or seeded deterministic behavior;
- environment read denied under AgentSafe/CiSafe;
- oversized compression/decompression rejected;
- heavy parse/regex operation bounded;
- capability events/warnings included in report.

## Verification

Run:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse time
cargo test -p eggsec-nse --features nse random
cargo test -p eggsec-nse --features nse compression
cargo test -p eggsec-nse --features nse,sandbox
bash scripts/check-architecture-guards.sh
```

## Final Acceptance Criteria

Phase 05 is complete when:

- Time/random/env/crypto/compression/helper CPU classes are controlled or explicitly documented as deferred.
- Automated profiles avoid hidden nondeterminism and environment leakage.
- Heavy helpers have cancellation/size accounting where practical.
- Reports expose relevant helper-side warnings and denials.
- Tests protect deterministic CI behavior and over-limit rejection.
