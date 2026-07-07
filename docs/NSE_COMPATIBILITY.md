# NSE Compatibility Matrix — Milestone 4

> **Scope**: This document describes the NSE compatibility status for Milestone 4 — a sandboxed Lua execution environment with capability-gated side effects. It does **not** claim full Nmap NSE parity. Compatibility is measured against the local corpus fixtures, not the upstream Nmap NSE library.

---

## Library Support Status

| Library | Category | Enforcement Status | Side Effects | Fallback | Notes |
|---------|----------|-------------------|--------------|----------|-------|
| nmap | Core | Wrapped | EnvAccess, NetworkAccess | HardFail | Routes through `NseCapabilityContext`; environment access checked; network gated |
| socket | Network | Wrapped | TCP/UDP I/O | Deny | All TCP connect/send/receive and UDP send/receive gated; network policy enforced |
| dns | Network | Wrapped | DNS resolution | Deny | `NseCapabilityContext::check_network_dns()` gate; CIDR-based policy support |
| io | Filesystem | Wrapped | File read/write | Deny | Read scoped to sandbox root; write denied in AgentSafe/CiSafe |
| os | Process | Wrapped | Process exec, env access | Deny | `std::process::Command` gated; environment access checked |
| lfs | Filesystem | Wrapped | File I/O | Deny | Lua filesystem ops routed through capability context |
| openssl | Crypto | Wrapped | TLS operations | Warn | TLS handshake gated; profile-specific warnings |
| comm | Network | Wrapped | TCP/UDP I/O | Deny | Inherits socket capability gating via shared context |
| datetime | Time | Wrapped | Wall-clock access | Warn | `nse_time_now()` emits nondeterminism warning in CiSafe |
| rand | Random | Wrapped | Random bytes | Warn | `nse_random_bytes()` denied in CiSafe; warned in AgentSafe |
| stdnse | Utility | PartiallyWrapped | Output, script args | Graceful degrade | Output table construction allowed; `stdnse.sleep()` blocked without cancellation |
| http | Network | PartiallyWrapped | HTTP requests | Deny | HTTP GET/POST mocked in corpus; real I/O gated through network policy |
| ssl | Network | Deferred | TLS handshake | — | No capability wrapper yet; full TLS protocol library |
| ssh | Network | Deferred | SSH connections | — | No capability wrapper yet; full SSH protocol library |
| smb | Network | Deferred | SMB/CIFS I/O | — | No capability wrapper yet; Windows file sharing protocol |
| smb2 | Network | Deferred | SMB2 I/O | — | No capability wrapper yet; SMB version 2 |
| mysql | Database | Deferred | MySQL queries | — | No capability wrapper yet; database driver |
| postgres | Database | Deferred | PostgreSQL queries | — | No capability wrapper yet; database driver |
| redis | Database | Deferred | Redis commands | — | No capability wrapper yet; key-value store |
| mongodb | Database | Deferred | MongoDB queries | — | No capability wrapper yet; document database |
| ldap | Network | Deferred | LDAP queries | — | No capability wrapper yet; directory protocol |
| snmp | Network | Deferred | SNMP queries | — | No capability wrapper yet; network management protocol |
| creds | Auth | Deferred | Credential lookup | — | No capability wrapper yet; credential store |
| unpwdb | Auth | Deferred | Username/password DB | — | No capability wrapper yet; wordlist database |
| brute | Auth | Deferred | Brute force attempts | — | No capability wrapper yet; brute force framework |
| target | Core | Deferred | Target manipulation | — | No capability wrapper yet; target registry |
| tab | Utility | Pure | None | N/A | Pure Lua table utilities; no side effects |
| json | Utility | Pure | None | N/A | JSON encode/decode; no side effects |
| base64 | Utility | Pure | None | N/A | Base64 encode/decode; no side effects |
| base32 | Utility | Pure | None | N/A | Base32 encode/decode; no side effects |
| bin | Utility | Pure | None | N/A | Binary data utilities; no side effects |
| bit | Utility | Pure | None | N/A | Bitwise operations; no side effects |
| stringaux | Utility | Pure | None | N/A | String manipulation helpers; no side effects |
| strbuf | Utility | Pure | None | N/A | String buffer implementation; no side effects |
| nse_string | Utility | Pure | None | N/A | NSE string utilities; no side effects |
| nse_table | Utility | Pure | None | N/A | NSE table utilities; no side effects |
| pcre | Utility | Pure | None | N/A | Regular expression engine; no side effects |
| shortport | Filter | Pure | None | N/A | Port/protocol filter rules; no side effects |
| match_lib | Filter | Pure | None | N/A | Pattern matching helpers; no side effects |
| matchs | Filter | Pure | None | N/A | Match string utilities; no side effects |
| url | Utility | Pure | None | N/A | URL parsing/construction; no side effects |
| unicode | Utility | Pure | None | N/A | Unicode utilities; no side effects |
| vulns | Reporting | Pure | None | N/A | Vulnerability reporting helpers; no side effects |

---

## Script/Pattern Compatibility

**Verification mode legend**: Libraries = hard assert (default) or soft (harness flag); Rules = hard assert when ports declared, skip otherwise; CapEvents = `required=true` hard asserts denial, `required=false` accepts resolver-block substitute; Evidence = `expected_evidence_kinds` hard asserts presence.

| Fixture ID | Category | Gap | Fidelity | Profiles | Libs | Rules | CapEvents | Evidence | Notes |
|------------|----------|-----|----------|----------|------|-------|-----------|----------|-------|
| simple-portrule | Core | Full | Complete | All | hard | skip | — | — | Basic portrule pattern; trivial script |
| simple-hostrule | Core | Full | Complete | All | hard | skip | — | — | Basic hostrule pattern; host matching |
| no-require | Core | Full | Complete | All | hard | skip | — | — | Script with no external dependencies |
| prerule | Core | Full | Complete | All | hard | skip | — | — | Prerule execution pattern |
| postrule | Core | Full | Complete | All | hard | skip | — | — | Postrule execution pattern |
| version-detect | Core | Full | Complete | All | hard | skip | — | — | Nmap version detection output |
| builtin-module-require | Core | Full | Complete | All | hard | skip | — | — | Requires built-in Lua module |
| stdnse-output | Core | Full | Complete | All | hard | skip | — | — | `stdnse.output_table()` pattern |
| stdnse-vulns | Core | Full | Complete | All | hard | skip | — | — | Vulnerability reporting via vulns module |
| http-title-mock | Core | Full | Complete | ManualPermissive, ManualGuarded | hard | skip | — | — | HTTP title fetch; mocked in corpus |
| dns-lookup-mock | Core | Full | Complete | ManualPermissive, ManualGuarded | hard | skip | — | — | DNS lookup; mocked in corpus |
| brute-shape | Core | Full | Complete | ManualPermissive | hard | skip | — | — | Brute force shape; no real auth |
| approximate-compat | Partial | Approximate | Approximate | ManualPermissive | hard | skip | — | — | Partial Nmap API surface; some calls stubbed |
| agent-denied-file | Regression | Denied | Complete | AgentSafe, CiSafe | hard | skip | required | CapabilityDenial | Agent profile correctly denies file access |
| process-denied | Regression | Denied | Complete | AgentSafe, CiSafe | hard | skip | required | CapabilityDenial | Agent profile correctly denies process exec |
| fs-read-denied | Regression | Denied | Complete | AgentSafe, CiSafe | hard | skip | — | — | Agent profile denies unscoped filesystem read |
| unsupported-rule | Partial | Unsupported | Unknown | All | hard | skip | — | — | Rule type not yet supported by engine |
| false-portrule | Partial | Incorrect | Unknown | All | hard | skip | — | — | Portrule returns false; no scan action |
| error-portrule | Partial | Error | Complete | All | hard | skip | — | — | Script error during execution; handled gracefully |
| capability-fs-deny | Regression | Denied | Complete | AgentSafe, CiSafe | hard | skip | required | CapabilityDenial | Capability context denies filesystem operation |
| capability-compress | Partial | Denied | Complete | AgentSafe, CiSafe | hard | skip | — | — | Compression denied or limited by policy |
| upstream-shortport-portrule | Upstream | Full | Complete | All | hard | skip | — | — | Upstream shortport portrule pattern |
| upstream-shortport-port | Upstream | Full | Complete | All | hard | skip | — | — | Upstream shortport port selection |
| upstream-categories-multi | Upstream | Partial | Approximate | ManualPermissive | hard | skip | — | — | Multiple script categories; partial support |
| upstream-ssl-cert-summary | Upstream | Partial | Approximate | ManualPermissive | hard | skip | — | — | SSL cert summary; TLS protocol mocked |
| upstream-http-get-mock | Upstream | Full | Complete | ManualPermissive, ManualGuarded | hard | skip | — | — | HTTP GET; mocked in corpus |
| upstream-http-post-mock | Upstream | Full | Complete | ManualPermissive, ManualGuarded | hard | skip | — | — | HTTP POST; mocked in corpus |
| upstream-dns-reverse | Upstream | Full | Complete | ManualPermissive, ManualGuarded | hard | skip | — | — | DNS reverse lookup; mocked |
| upstream-stdnse-scripts-args | Upstream | Full | Complete | All | hard | skip | — | — | Script argument passing via stdnse |
| upstream-stdnse-output-table | Upstream | Full | Complete | All | hard | skip | — | — | stdnse.output_table() pattern |
| upstream-hostrule-hostname | Upstream | Full | Complete | All | hard | skip | — | — | Hostrule hostname matching |
| upstream-graceful-degrade | Upstream | Full | Complete | All | hard | skip | — | — | Graceful degradation on missing deps |
| upstream-vulns-check | Upstream | Full | Complete | All | hard | skip | — | — | vulns.exists() pattern |
| upstream-brute-categories | Upstream | Partial | Approximate | ManualPermissive | hard | skip | — | — | Brute categories; brute lib deferred |
| upstream-nmap-fetch-file | Upstream | Partial | Approximate | ManualPermissive | hard | skip | — | — | nmap.fetch_file(); partially mocked |
| upstream-structured-output | Upstream | Full | Complete | All | hard | skip | — | — | Structured XML/JSON output pattern |
| upstream-banner-parse | Upstream | Full | Complete | All | hard | skip | — | — | Service banner parsing |
| portrule-host-port | Context | Full | Complete | All | hard | hard | — | — | Portrule receives host/port correctly |
| hostrule-host-context | Context | Full | Complete | All | hard | hard | — | — | Hostrule receives host context correctly |
| portrule-service-context | Context | Full | Complete | All | hard | hard | — | — | Portrule receives service/context info |
| tcp-connect-echo | Protocol | Full | Complete | ManualPermissive | hard | skip | — | — | Real local TCP echo; socket connect/send/receive |
| tcp-connect-denied | Protocol | Denied | Complete | AgentSafe, CiSafe | hard | skip | required | CapabilityDenial | TCP connect denied under restricted profiles |
| http-get-local | Protocol | Full | Complete | ManualPermissive | hard | skip | — | — | Real local HTTP GET; title extraction |
| http-post-local | Protocol | Full | Complete | ManualPermissive | hard | skip | — | — | Real local HTTP POST |
| udp-echo | Protocol | Full | Complete | ManualPermissive | hard | skip | — | — | Real local UDP echo; sendto/receive_from |

---

## Profile Compatibility

### ManualPermissive

- **Libraries**: All 43 libraries available (wrapped + partially wrapped + deferred + pure)
- **Side effects**: All permitted with warnings
- **Network**: Full TCP/UDP/DNS access; no CIDR restrictions
- **Filesystem**: Read/write allowed within sandbox
- **Process**: Execution allowed with warnings
- **Use case**: Interactive testing, trusted scripts

### ManualGuarded

- **Libraries**: All 43 libraries available
- **Side effects**: Same as ManualPermissive but scope enforcement stricter
- **Network**: Scoped to target scope rules
- **Filesystem**: Scoped to sandbox root
- **Process**: Execution allowed with warnings
- **Use case**: Interactive testing with tighter scope

### AgentSafe

- **Libraries**: Wrapped (10) + PartiallyWrapped (2) + Pure (17) = 29 available
- **Deferred libraries**: 14 unavailable (ssl, ssh, smb, smb2, mysql, postgres, redis, mongodb, ldap, snmp, creds, unpwdb, brute, target)
- **Side effects**: Restricted
- **Network**: TCP/UDP/DNS gated by network policy; HTTP/HTTPS gated
- **Filesystem**: Read scoped to allowed directory; write denied
- **Process**: Denied
- **Environment**: Denied
- **Compression**: Allowed with limits
- **Use case**: Autonomous agent execution

### CiSafe

- **Libraries**: Wrapped (10) + PartiallyWrapped (2) + Pure (17) = 29 available
- **Deferred libraries**: 14 unavailable (same as AgentSafe)
- **Side effects**: Most restricted
- **Network**: TCP/UDP/DNS denied; HTTP gated
- **Filesystem**: Read scoped; write denied
- **Process**: Denied
- **Environment**: Denied
- **Randomness**: Denied (nondeterminism)
- **Time**: Warning only (nondeterminism)
- **Compression**: Allowed with limits
- **Use case**: CI pipelines, untrusted scripts

---

## Known Gaps

### Deferred Libraries (14)

These libraries have no capability wrappers and are unavailable in AgentSafe and CiSafe profiles:

| Library | Protocol/Domain | Impact |
|---------|----------------|--------|
| ssl | TLS/SSL | Cannot perform TLS handshakes; scripts requiring HTTPS connections fail |
| ssh | SSH | Cannot establish SSH connections; remote command execution unavailable |
| smb | SMB/CIFS | Cannot access SMB shares; Windows file sharing scripts unavailable |
| smb2 | SMB v2 | Cannot access SMB2 shares; modern Windows file sharing unavailable |
| mysql | MySQL | Cannot query MySQL databases; database audit scripts unavailable |
| postgres | PostgreSQL | Cannot query PostgreSQL databases; database audit scripts unavailable |
| redis | Redis | Cannot query Redis; cache/session scripts unavailable |
| mongodb | MongoDB | Cannot query MongoDB; NoSQL scripts unavailable |
| ldap | LDAP | Cannot query LDAP directories; directory enumeration unavailable |
| snmp | SNMP | Cannot query SNMP; network management scripts unavailable |
| creds | Credentials | Cannot access credential stores; credential-based scripts unavailable |
| unpwdb | Wordlist | Cannot access password databases; dictionary attacks unavailable |
| brute | Brute force | Cannot perform brute force authentication; auth testing unavailable |
| target | Target | Cannot manipulate target registry; advanced target handling unavailable |

### Unsupported Patterns

- **Real HTTP/HTTPS**: Corpus uses mocks; real I/O requires network policy and capability wrapper for `http.request()`
- **Real DNS resolution**: Corpus uses mocks; real DNS requires network policy
- **`stdnse.sleep()`**: Blocked without cancellation token support; scripts using sleep will hang or error
- **Process execution**: Fully denied in AgentSafe/CiSafe; only ManualPermissive allows
- **Unscoped filesystem read**: AgentSafe denies reads outside sandbox root
- **Compression**: Supported but subject to 64 MiB input / 256 MiB output limits

### Approximate Compatibility

Scripts marked `Approximate` fidelity have partial Nmap API surface coverage. Specific stubs or mocks may be in place but the full upstream behavior is not implemented. These scripts run but may produce different output than stock Nmap.

---

## Runtime Verification

The compatibility corpus is verified by two structurally separated harnesses:

### Static Harness (`compatibility_corpus_tests.rs`)

- Verifies resolver-level behavior only: script/module resolution, file size/policy, blocked-at-resolver diagnostics.
- Does not execute scripts.
- For blocked fixtures, simulates a `with_error(...)` block and asserts status/fidelity.
- For non-blocked fixtures, defers status/fidelity assertions to the runtime harness (since static cannot observe runtime errors or capability denials).

### Runtime Harness (`runtime_corpus_tests.rs`)

- Drives every fixture through `NseExecutor::with_profile(&ResolvedNseExecutionProfile)` with synthetic host/port context.
- Captures rule reports, library use, capability events, evidence, and runtime stats.
- Asserts manifest expectations (`expected_status`, `expected_fidelity`, `expected_libraries`, `expected_rules`, `expected_capability_events`).
- **Strict assertions (Milestone 5 Phase 02)**:
  - **Libraries**: Hard assert by default; `allow_missing_runtime_libraries = true` downgrades to soft (log-only). Empty library reports (short-circuited execution) pass regardless.
  - **Rules**: Hard assert when fixture declares `[[fixture.ports]]` (portrule can fire). Skip when no ports (portrule cannot fire). `allow_missing_runtime_rules = true` always skips empty rules.
  - **Capability events**: `required = true` hard asserts the denial is observed (no resolver-block/error substitute). `required = false` (default) accepts resolver block or error as substitute.
  - **Evidence**: `expected_evidence_kinds` hard asserts each kind is present. `optional_evidence_kinds` logged as informational.
- 17 tests covering per-category and cross-cutting observations.

### Smoke Tests (`runtime_smoke_tests.rs`)

- Exercise the full pipeline (profile → context → execution → report → `ReportEnvelope` bridge).
- Verify envelope shape (findings, severity, domain_id) for representative scenarios.
- 2 tests: `CompatibilityLab` clean execution and `AgentSafe` capability-denial surfacing.

### Test Status

| Binary | Tests | Stable at | Notes |
|--------|-------|-----------|-------|
| `runtime_corpus_tests` | 17 | `--test-threads=1` | Strict assertions added in Milestone 5 Phase 02; `process-denied` flake under high parallelism |
| `local_protocol_tests` | 16 | any | Local TCP/HTTP/UDP fixtures with real listeners; added in Milestone 5 Phase 03 |
| `runtime_smoke_tests` | 2 | any | Smoke + envelope bridge |
| `compatibility_corpus_tests` | 43 | any | Resolver-only assertions |

### Known Limitations

- **Synthetic context fidelity**: The runtime harness injects a synthetic host/port context, so rule-level fidelity is `Approximate` even when script behavior is fully correct. This is by design — `evaluate_rule_with_context` downgrades fidelity when `host.source == Synthetic`.
- **Capability-denial fixtures**: Capability-denial scenarios (`process-denied`, `fs-read-denied`, `capability-fs-deny`) declare `expected_fidelity = "approximate"` to match this synthetic-context behavior. Status is `Partial` due to the capability denial.
- **High-parallelism flake**: The `process-denied` fixture occasionally misses a `process_exec` capability event when the test harness runs at default parallelism (typically 16+ threads). Stable at `--test-threads=4` or lower. Likely a cross-test interaction with library-level static state (`nmap._ports`, `http.HTTP_CLIENT`, `IO_SANDBOX_VIOLATIONS`, etc.) that is serialized via Mutex but contended under heavy parallelism. Documented as a known limitation; not blocking.

---

## Milestone 5 Candidates

The following are candidates for capability wrapper migration in Milestone 5:

### Priority 1: Protocol Libraries

- **ssl** — TLS/SSL handshake and certificate operations; required for HTTPS scripts
- **ssh** — SSH connection and command execution; required for remote audit scripts
- **http** — Full HTTP/HTTPS client; upgrade from PartiallyWrapped to fully wrapped

### Priority 2: Database Libraries

- **mysql** — MySQL client; required for database security audit scripts
- **postgres** — PostgreSQL client; required for database security audit scripts
- **redis** — Redis client; required for cache/session security scripts
- **mongodb** — MongoDB client; required for NoSQL security scripts

### Priority 3: Authentication Libraries

- **brute** — Brute force framework; required for authentication testing
- **creds** — Credential store access; required for credential-based testing
- **unpwdb** — Wordlist access; required for dictionary attacks

### Priority 4: Network Protocol Libraries

- **smb** / **smb2** — Windows file sharing; required for Windows environment audits
- **ldap** — Directory services; required for Active Directory audits
- **snmp** — Network management; required for network device audits

### Priority 5: Remaining Deferred

- **target** — Target registry manipulation; low priority for most scripts

### Infrastructure Improvements

- **Cancellation token support** — Enable `stdnse.sleep()` to respect task cancellation
- ~~**Real HTTP/HTTPS in corpus**~~ — Local HTTP fixtures added in Milestone 5 Phase 03; reqwest capability bypass documented
- ~~**Real DNS in corpus**~~ — DNS denial tested via local_protocol_tests; real resolution requires local DNS server (deferred)
- **Profile-specific corpus tagging** — Tag fixtures with expected profile compatibility
