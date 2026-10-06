---
name: eggsec-knowledge-corpus
description: "Knowledge-corpus leaf crates (eggsec-service-db, eggsec-secrets, eggsec-payloads, eggsec-udp-scan) - use when working with port-to-service tables, banner heuristics, secret patterns or the entropy gate, PayloadType corpora and the payload/prober seam, UDP ICMP-error correlation or the open/closed/filtered classification lattice, or when touching the engine facades scanner::service_data, recon::secrets, fuzzer::payloads."
---

# Eggsec Knowledge Corpus Skill

Four near-empty-dependency leaf crates hold the repo's *data*: lookup tables, regex
patterns, payload strings, and UDP classification. They are reached only through
engine `pub use` facades. Full deep dive: `architecture/knowledge_corpus.md`.

## Key Files and Types

### `eggsec-service-db` — service fingerprint knowledge (316 LOC, `src/lib.rs` only)
- `COMMON_PORTS: &[(u16, &str)]` — **47 entries, all 47 ports distinct**
  (`crates/eggsec-service-db/src/lib.rs:24`, entries `:25-72`)
- `PORT_SERVICE_MAP: LazyLock<FxHashMap<u16, &'static str>>` (`src/lib.rs:74-75`),
  `get_service_name` → `"unknown"` on miss (`:77`), `get_service_by_port` (`:81`)
- `guess_service_from_banner` (`:85`) — lowercases once, then ordered substring
  branches; banner-first, port table as fallback in `guess_service` (`:138`)
- `is_web_service` (`:147`), `is_database` (`:154`), `is_mail_service` (`:158`)
- Only dependency is `rustc-hash` (`Cargo.toml:24`)

### `eggsec-secrets` — credential detection (520 LOC, `src/lib.rs` only)
- `SecretFinding` (`:34`), `SecretType` **30 variants** (`:44-75`),
  `Confidence` High/Medium/Low (`:78`), `pub use eggsec_core::types::Severity` (`:84`)
- private `SecretPattern` (`:123`), `build_patterns()` (`:131`),
  `static PATTERNS: LazyLock<Vec<SecretPattern>>` (`:339`), `SecretScanner` (`:341`)
- `scan()` (`:356`), `scan_file()` (`:386`, the crate's only I/O),
  `scan_content()` (`:416`)

### `eggsec-payloads` — payload data (8,849 LOC, 42 files)
- 41 `pub mod` including `macros` (`crates/eggsec-payloads/src/lib.rs:47-88`)
- `PayloadType` **40 variants** (`:95-136`), derives `EnumIter`;
  `is_advanced()` (`:193`) = `GraphQL | OAuth | Jwt | Idor | Ssti | Grpc`;
  `all_variants()` (`:205`)
- `Payload` (`:213`), `get_payloads()` (`:229`, exhaustive over all 40),
  `PAYLOAD_CACHE` (`:280`) and `ALL_PAYLOADS_CACHE` (`:289`) — both `LazyLock`
- One naming asymmetry: `oast.rs` exports `get_oast_payloads()`, not `get_payloads()`

### `eggsec-udp-scan` — UDP + ICMP correlation (1,970 LOC, 6 files)
- `UdpPortState` (`src/classify.rs:14-26`): `Closed` / `Filtered` / `OpenFiltered` /
  `Open` — 4 states, and `Open` is a variant the range scan never emits
- `Evidence` (`classify.rs:33-47`); `is_proof()` (`classify.rs:55`) is true for only 4
  of 6 — `Silence` and `NetOrHostUnreachable` are explicitly unproven
- `HostState` `Up` / `Unresponsive` / `Indeterminate` (`classify.rs:112`),
  `ports_are_meaningful()` (`:129`), `derive()` (`:139`)
- `parse_icmp_error` (`src/icmp.rs:94`), `CorrelationTable` (`src/correlate.rs:65`)
  with `register` (`:98`), `correlate` (`:129`), `sweep` (`:144`)
- `scan_with_receiver()` (`src/lib.rs:230`) is blocking; accessors
  `closed_ports`/`filtered_ports`/`open_filtered_ports` (`:189, :198, :207`)
- Zero workspace deps — `libc`, `thiserror`, `tracing` only; no `test-util` feature

## The ownership seam

**The corpus owns DATA; the engine owns I/O.** Everything else follows from that.

May never move into a corpus crate:
- Network/transport/TLS (`reqwest`, `hyper`, `rustls`, `axum`, `tonic`), `tokio`,
  filesystem/DB (`rusqlite`, `sqlx`), frontends (`clap`, `ratatui`, `crossterm`,
  `indicatif`) — guard 148 dep blacklist, `scripts/check-architecture-guards.sh:4193`
- Any workspace dep other than `eggsec-core` (10 forbidden names at `:4185`);
  `eggsec-service-db` is held stricter and must not declare `eggsec-core` at all
  (`:4216-4220`)
- Authorization vocabulary in code: `Scope`, `ApprovedOperation`, `ApprovedExecution`,
  `EnforcementContext`, `Capability` (`:4208`). String literals are stripped before
  the match, so attack payloads may carry the words.

May never be pulled back out:
- `reqwest::Client` / `.await` anywhere in `eggsec-payloads` (guard 150, `:4308`) —
  the six `is_advanced` **probers** stay in `eggsec/src/fuzzer/payloads/`; the payload
  **strings** they send are corpus data. The seam is execution, not payload data.
- DNS, scope checks, dispatch, privilege acquisition — `eggsec-udp-scan` reports that
  privilege is required, it never takes it (`eggsec-udp-scan/src/lib.rs:29-33`).

Facades are permanent, not scaffolding — guard 114 (`:3129`),
guard 149 (`:4239`), guard 150 (`:4352`):
- `eggsec::scanner::service_data` ← `crates/eggsec/src/scanner/mod.rs:100`
- `eggsec::recon::secrets` ← `crates/eggsec/src/recon/mod.rs:101`
- `eggsec::fuzzer::payloads::*` ← `crates/eggsec/src/fuzzer/payloads/mod.rs:39`
- UDP is *not* a re-export: a lossy projection into engine-owned `PortStatus` /
  `UdpHostState` / `UdpEvidenceSummary` at `crates/eggsec/src/dispatch/scanner.rs:157-252`
  (feature `udp-scan`)

## Per-crate worked facts

| Fact | Value | Cite |
|---|---|---|
| `COMMON_PORTS` entries | 47, all ports distinct | `eggsec-service-db/src/lib.rs:24-72` |
| Secret patterns | 25 constructions | `eggsec-secrets/src/lib.rs:131-337` |
| `SecretType` covered by patterns | **20 of 30** | variants `:45-74` vs `build_patterns` |
| `PayloadType` variants / modules | 40 / 40 corpus modules | `eggsec-payloads/src/lib.rs:96-135`, `:47-88` |
| Corpus test counts (`#[test]`, whole crate) | service-db 21, secrets 11, payloads **252** across 42 files (only 4 in `lib.rs`), udp-scan 23 (21 in `tests.rs`, 2 in `lib.rs`) | each crate's `src/` |
| Corpus deps | `eggsec-core` only (service-db: none) | each `Cargo.toml:22-33` |

## Common mistakes / gotchas

- **Never treat UDP silence as open.** A range scan can prove `closed` and `filtered`;
  silence is `OpenFiltered` (`eggsec-udp-scan/src/lib.rs:5-15`). Use the positive-evidence
  accessors (`closed_ports()`, `open_filtered_ports()`) and check
  `HostState::ports_are_meaningful()` before presenting per-port verdicts.
- **The entropy gate is frozen at 3.5 and scoped to `SecretType::AwsSecretKey` only**
  (`eggsec-secrets/src/lib.rs:362`). Guard 149 greps for the literal `3.5` (`:4254`) and
  the `AwsSecretKey` scope (`:4260`). It is detection semantics, not a tuning knob.
- **`SecretType` ↔ pattern is not 1:1.** 25 patterns cover 20 of 30 variants. Uncovered:
  `AzureKey`, `GcpServiceAccount`, `BitbucketToken`, `JwtToken`, `NpmToken`, `PyPiToken`,
  `HerokuKey`, `NetlifyToken`, `DockerhubToken`, `KubernetesSecret`. Three types carry
  multiple patterns (`DatabaseConnectionString` ×4, `GithubToken` ×2, `BasicAuth` ×2).
- **`value_preview` truncates at 20 chars + ellipsis** (`eggsec-secrets/src/lib.rs:374`) —
  a detection-surface limit, not redaction. Masking is the report model's job
  (`eggsec-secrets/src/lib.rs:13-16`); deleted engine `utils/redaction.rs` stays deleted
  under guard 147.
- **Payload caches must stay `LazyLock`** and must never degrade to a shared empty `Vec`
  (guard 150 `:4334`, `:4339`) — eager materialization regresses every linking binary,
  including the TUI.
- **`get_payloads` must never panic or return empty for a real variant** (guard 150 `:4327`).
  Adding a `PayloadType` variant means a new corpus module, a `get_payloads` arm, and a
  cache that resolves it — see `crates/eggsec/tests/fuzzer_payload_corpus_seam.rs:78`.
- **`guess_service_from_banner` is order-sensitive.** The SSH branch requires *both*
  `"ssh"` and `"version"` (`eggsec-service-db/src/lib.rs:88`); the wide HTTP branch sits
  below the specific database/mail checks.
- **UDP `open_ports` in the engine projection holds every port with a verdict**, not just
  open ones (`crates/eggsec/src/dispatch/scanner.rs:209`). Use
  `PortScanResults::proved_open_ports()` (`scanner/ports/mod.rs:318`).
- **`is_advanced` selects an execution strategy, not a data location.** All 40 types resolve
  in the corpus; the flag only routes to a prober in the engine fuzzer.
- Three corpora declare `publish = false` (`Cargo.toml:15`); `eggsec-udp-scan` has no
  `publish` key (verified by grep) — don't assume publication status from siblings.

## Verification

```bash
cargo test -p eggsec-service-db --tests
cargo test -p eggsec-secrets --tests
cargo test -p eggsec-payloads --tests
cargo test -p eggsec-udp-scan --tests
bash scripts/check-architecture-guards.sh   # guards 114, 147, 148, 149, 150
```

All four corpus crates are wired into `make check` at `Makefile:122-125`.
Note: several guard section headers print a literal `--- Check N: ... (FAIL) ---`
string (`scripts/check-architecture-guards.sh:3110, :4126, :4173, :4229, :4273`) while
their sub-checks PASS and the script exits 0 — a legacy hardcoded echo, not a failure.

Deep dive: `architecture/knowledge_corpus.md` (crate map, per-crate tables, ADR-0005
rationale, corrections to stale prose). Seams: `architecture/scanner.md`,
`architecture/recon.md`, `architecture/fuzzer.md`, `architecture/capability_segregation.md`.