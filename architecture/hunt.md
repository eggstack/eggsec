# Hunt Module — Deep Dive

## Role & Responsibilities

The `hunt` module performs business-logic and authorization-flaw hunting via real HTTP probing. It detects attack chains, business logic flaws, race conditions, authorization bypasses, and session management vulnerabilities by correlating findings across sub-modules.

This is an **advanced threat hunting** surface — all sub-modules make real HTTP requests to the target using `reqwest` via `HuntClient`. The `chain` sub-module is purely correlational (no additional HTTP requests).

## Location & Feature Gating

| Aspect | Detail |
|--------|--------|
| Root module | `crates/eggsec/src/hunt/mod.rs` |
| Cargo feature | `advanced-hunting` (marker-only, no deps) — `Cargo.toml:305` |
| Feature gate in lib.rs | `#[cfg(feature = "advanced-hunting")] pub mod hunt;` at `lib.rs:111`; `#[cfg(not(feature = "advanced-hunting"))] #[allow(dead_code)] mod hunt;` at `lib.rs:113-115` |
| When disabled | Private stub module with `#[allow(dead_code)]` — compiles but is unused |
| Included in | `full` feature set (`Cargo.toml:327`) |

## Architecture

### File Inventory (6 files)

| File | Lines | Types Defined |
|------|-------|---------------|
| `hunt/mod.rs` | 276 | `HuntClient`, `HuntReport`, `HuntConfig`, `run_hunt()` |
| `hunt/authz.rs` | 314 | `AuthzBypass`, `BypassType`, `check_authz_bypass()` |
| `hunt/business.rs` | 389 | `BusinessLogicFlaw`, `FlawType`, `check_business_logic()` |
| `hunt/chain.rs` | 338 | `AttackChain`, `ChainType`, `ChainStep`, `detect_attack_chains()` |
| `hunt/race.rs` | 251 | `RaceCondition`, `RaceType`, `check_race_conditions()` |
| `hunt/session.rs` | 297 | `SessionIssue`, `SessionIssueType`, `check_session_security()` |

### Type Table

| Type | File:Line | Role |
|------|-----------|------|
| `HuntClient` | `mod.rs:27` | HTTP client wrapper: `reqwest::Client`, target URL, timeout; methods: `get()`, `post_json()`, `head()`, `request()` |
| `HuntReport` | `mod.rs:134` | Aggregated results: 5 category vectors + `total_findings` counter |
| `HuntConfig` | `mod.rs:223` | 7 fields: 5 boolean category toggles, `concurrency`, `timeout_ms` |
| `AuthzBypass` | `authz.rs:7` | Finding: `BypassType`, severity, endpoint, evidence, CVSS |
| `BypassType` | `authz.rs:20` | Enum of 7 variants |
| `BusinessLogicFlaw` | `business.rs:7` | Finding: `FlawType`, severity, location, evidence, CVSS |
| `FlawType` | `business.rs:20` | Enum of 10 variants |
| `AttackChain` | `chain.rs:7` | Multi-step chain: `ChainType`, steps, severity, CVSS |
| `ChainType` | `chain.rs:20` | Enum of 6 variants |
| `ChainStep` | `chain.rs:29` | Step: number, vulnerability, prerequisite, impact, evidence, severity |
| `RaceCondition` | `race.rs:9` | Finding: `RaceType`, severity, endpoint, evidence, CVSS |
| `RaceType` | `race.rs:22` | Enum of 8 variants |
| `SessionIssue` | `session.rs:7` | Finding: `SessionIssueType`, severity, evidence, CVSS |
| `SessionIssueType` | `session.rs:19` | Enum of 9 variants |

### Enum Variant Counts

| Enum | Variants | File:Line |
|------|----------|-----------|
| `BypassType` | 7 | `authz.rs:20-28` |
| `FlawType` | 10 | `business.rs:20-31` |
| `ChainType` | 6 | `chain.rs:20-27` |
| `RaceType` | 8 | `race.rs:22-31` |
| `SessionIssueType` | 9 | `session.rs:19-29` |

### HuntClient Construction

`HuntClient::new()` (`mod.rs:34-60`):

- Validates target starts with `http://` or `https://`
- Builds `reqwest::Client` with: timeout from config, cookie store enabled, same-host redirect policy (max 10 hops), pool max idle per host from `DEFAULT_POOL_MAX_IDLE_PER_HOST`, pool idle timeout from `DEFAULT_POOL_IDLE_TIMEOUT_SECS`, TCP nodelay
- All request methods set `User-Agent: Eggsec/1.0 Security Testing`

### HuntConfig Defaults

| Field | Default | Source |
|-------|---------|--------|
| `check_attack_chains` | `true` | `mod.rs:236` |
| `check_business_logic` | `true` | `mod.rs:237` |
| `check_race_conditions` | `true` | `mod.rs:238` |
| `check_authz_bypass` | `true` | `mod.rs:239` |
| `check_session` | `true` | `mod.rs:240` |
| `concurrency` | `10` | `mod.rs:241` |
| `timeout_ms` | `DEFAULT_TOOL_TIMEOUT_MS` | `mod.rs:242` |

## Behavior & Flow

### `run_hunt()` Orchestrator

`run_hunt()` (`mod.rs:178-220`) executes sub-modules sequentially:

1. **session** → `session::check_session_security()`
2. **authz** → `authz::check_authz_bypass()`
3. **race** → `race::check_race_conditions()`
4. **business** → `business::check_business_logic()`
5. **chain** → `chain::detect_attack_chains(&report)` (operates on accumulated report, no HTTP)

Each sub-module is gated by its `HuntConfig` boolean. Findings are added to `HuntReport` via `add_*()` methods that increment `total_findings`.

### AuthZ Bypass Detection

`check_authz_bypass()` (`authz.rs:62-76`) runs 4 checks:

1. **Admin access** (`authz.rs:78-146`): Tests 17 admin paths (`ADMIN_PATHS` at `authz.rs:30-49`: `/admin`, `/admin/`, `/api/admin`, `/api/admin/users`, `/api/admin/config`, `/dashboard`, `/manage`, `/management`, `/internal`, `/api/internal`, `/debug`, `/api/debug`, `/actuator`, `/actuator/health`, `/actuator/env`, `/swagger-ui.html`, `/api-docs`, `/graphql`) — concurrent with semaphore. HTTP 200 + body contains `"admin"`, `"dashboard"`, `"management"`, or `"users"` → `MissingAuthorization` (Critical, CVSS 9.0).

2. **IDOR** (`authz.rs:148-212`): Tests 8 IDOR-prone paths (`IDOR_PATHS` at `authz.rs:51-60`: `/api/users/{1,2}`, `/api/users/{1,2}/profile`, `/api/accounts/{1,2}`, `/api/documents/{1,2}`) — concurrent with semaphore. HTTP 200 + body > 50 bytes → `Idor` (High, CVSS 7.5).

3. **Force browsing** (`authz.rs:214-244`): Tests 4 paths (`/admin/config`, `/settings`, `/profile/admin`, `/user/roles`) sequentially. HTTP 200 → `ForceBrowsing` (Medium, CVSS 5.3).

4. **HTTP methods** (`authz.rs:246-299`): Tests `OPTIONS` (checks `Allow` header for PUT/DELETE → Low, CVSS 3.0) and `TRACE` (HTTP 200 → Medium, CVSS 5.0, XST risk).

All findings use UUID-based IDs (`az-{uuid[..8]}`).

### Business Logic Flaw Detection

`check_business_logic()` (`business.rs:86-100`) runs 4 checks:

1. **API discovery** (`business.rs:102-156`): Tests 15 API paths (`API_PATHS` at `business.rs:33-49`). HTTP 200 + body > 100 bytes + API keywords → `InsufficientValidation` (Low, CVSS 3.0). HTTP 401/403 → `PrivilegeEscalation` (Info).

2. **Sensitive files** (`business.rs:158-223`): Tests 32 sensitive paths (`SENSITIVE_PATHS` at `business.rs:51-84`: `.env`, `.git/config`, `.git/HEAD`, various config files, `private_key.pem`, `id_rsa`, `.ssh/authorized_keys`, backup/dump paths, debug endpoints, actuator, metrics) — concurrent with semaphore. Classification (`business.rs:225-247`):
   - `.env`, `credentials`, `private_key`, `id_rsa`, body contains `password`/`secret`/`api_key` → Critical (CVSS 9.0), `TrustBoundaryViolation`
   - `.git`, `config`, `backup`, `dump` → High (CVSS 7.5), `TrustBoundaryViolation`
   - Otherwise → Medium (CVSS 5.0), `TrustBoundaryViolation`

3. **Error handling** (`business.rs:259-322`): Tests 6 malicious paths (`/api/test%00`, `/api/test%0d%0a`, SQL injection, XSS, path traversal, `%ff%ff`). Verbose error messages (stack trace, exception, traceback, etc.) → `InsufficientValidation` (Medium, CVSS 5.0). Path traversal success (body contains `"root:"`) → `TrustBoundaryViolation` (Critical, CVSS 9.0).

4. **Rate limiting** (`business.rs:324-368`): Sends 20 rapid GET requests. Zero 429 responses → `RateLimitBypass` (Medium, CVSS 5.0).

### Race Condition Detection

`check_race_conditions()` (`race.rs:51-63`) runs 2 checks:

1. **Concurrent requests** (`race.rs:65-159`): Tests 15 state-changing paths (`STATE_CHANGING_PATHS` at `race.rs:33-49`: `/api/checkout`, `/api/cart`, `/api/transfer`, `/api/payment`, `/api/order`, `/api/coupon`, `/api/discount`, `/api/vote`, `/api/like`, `/api/comment`, `/api/purchase`, `/api/redeem`, `/api/claim`, `/api/book`, `/api/reserve`). For each path, sends `concurrency` (default 10) concurrent POST requests with JSON body `{"action":"test","quantity":1,"amount":100}`. Detection:
   - **ResponseInconsistency** (`race.rs:113-133`): Unique status codes include both success (200/201) and error (≥400) → High (CVSS 7.0)
   - **TOCTOU** (`race.rs:135-155`): Multiple concurrent successes → Medium (CVSS 6.0)
   - Uses `FxHashSet` for unique status tracking (`race.rs:106`)

2. **Response inconsistency** (`race.rs:161-233`): Tests 3 endpoints (`/api/user/profile`, `/api/cart`, `/api/balance`) sequentially with 5 GET requests each. Detection:
   - **TimingAnomaly** (`race.rs:195-211`): Max deviation > 3× average → Low (CVSS 3.0)
   - **ResponseInconsistency** (`race.rs:214-230`): Unique status codes across sequential requests → Medium (CVSS 5.0)

### Chain Detection

`detect_attack_chains()` (`chain.rs:39-50`) runs 4 correlation detectors on the accumulated report (no HTTP):

1. **Privilege escalation** (`chain.rs:52-145`):
   - IDOR + MissingAuthorization → Critical (CVSS 9.0), chain steps from matching bypasses
   - MissingAuthorization + SessionIssue (Fixation/MissingHttpOnly/InsufficientEntropy) → Critical (CVSS 8.5), 2-step chain

2. **Data exfiltration** (`chain.rs:147-197`):
   - TrustBoundaryViolation (sensitive files) + IDOR → Critical (CVSS 9.5), 2-step chain

3. **Session exploitation** (`chain.rs:199-253`):
   - Weak session (InsufficientEntropy/Fixation/MissingHttpOnly/MissingSecure) + RateLimitBypass → High (CVSS 7.5), 2-step chain

4. **Rate limit chain** (`chain.rs:255-302`):
   - RateLimitBypass + MissingAuthorization → Critical (CVSS 8.0), 2-step chain

### Session Security Analysis

`check_session_security()` (`session.rs:31-55`) runs 4 checks on the initial `/` response:

1. **Cookie flags** (`session.rs:57-114`): Iterates all `Set-Cookie` headers. Checks:
   - Missing `HttpOnly` → `MissingHttpOnly` (Medium, CVSS 5.0)
   - Missing `Secure` on HTTPS site → `MissingSecure` (Medium, CVSS 5.0)
   - Missing `SameSite` → `MissingSameSite` (Low, CVSS 3.0)

2. **Security headers** (`session.rs:116-149`):
   - Missing `X-Frame-Options` → `Csrf` (Low, CVSS 3.0)
   - Missing `Content-Security-Policy` → `Csrf` (Low, CVSS 3.0)

3. **Token entropy** (`session.rs:151-192`): Checks session-like cookie names (`session`, `sid`, `token`) — value length < 16 chars → `InsufficientEntropy` (High, CVSS 7.0).

4. **Session fixation** (`session.rs:194-279`): Sends 5 concurrent GET requests (semaphore-gated), filters for session-like cookies, compares across all requests. All identical → `SessionFixation` (High, CVSS 6.5).

## Public API

| Function/Method | File:Line | Signature |
|-----------------|-----------|-----------|
| `HuntClient::new()` | `mod.rs:34` | `(target: &str, config: &HuntConfig) -> Result<Self>` |
| `HuntClient::get()` | `mod.rs:78` | `(&self, path: &str) -> Result<reqwest::Response>` |
| `HuntClient::post_json()` | `mod.rs:89` | `(&self, path: &str, body: &Value) -> Result<reqwest::Response>` |
| `HuntClient::head()` | `mod.rs:106` | `(&self, path: &str) -> Result<reqwest::Response>` |
| `HuntClient::request()` | `mod.rs:117` | `(&self, method: Method, path: &str) -> Result<reqwest::Response>` |
| `HuntClient::base_url()` | `mod.rs:128` | `(&self) -> &str` |
| `HuntReport::new()` | `mod.rs:145` | `(target: &str) -> Self` |
| `HuntReport::add_chain()` | `mod.rs:152` | `(&mut self, chain: AttackChain)` |
| `HuntReport::add_business_flaw()` | `mod.rs:157` | `(&mut self, flaw: BusinessLogicFlaw)` |
| `HuntReport::add_race_condition()` | `mod.rs:162` | `(&mut self, race: RaceCondition)` |
| `HuntReport::add_authz_bypass()` | `mod.rs:167` | `(&mut self, bypass: AuthzBypass)` |
| `HuntReport::add_session_issue()` | `mod.rs:172` | `(&mut self, issue: SessionIssue)` |
| `run_hunt()` | `mod.rs:179` | `(target: &str, config: HuntConfig) -> Result<HuntReport>` |
| `check_authz_bypass()` | `authz.rs:63` | `(client: &HuntClient, config: &HuntConfig) -> Result<Vec<AuthzBypass>>` |
| `check_business_logic()` | `business.rs:87` | `(client: &HuntClient, config: &HuntConfig) -> Result<Vec<BusinessLogicFlaw>>` |
| `detect_attack_chains()` | `chain.rs:40` | `(report: &HuntReport) -> Result<Vec<AttackChain>>` |
| `check_race_conditions()` | `race.rs:52` | `(client: &HuntClient, config: &HuntConfig) -> Result<Vec<RaceCondition>>` |
| `check_session_security()` | `session.rs:32` | `(client: &HuntClient, config: &HuntConfig) -> Result<Vec<SessionIssue>>` |

## Integration Points

### CLI

- **Command**: `eggsec hunt <target>` — handler at `commands/handlers/hunt.rs:5`
- **Feature gate**: `advanced-hunting` required (`commands/handlers/hunt.rs:15`)
- **Policy gate**: `evaluate_and_enforce_operation()` with `OperationRisk::Intrusive` (`hunt.rs:6-20`)
- **Args**: `HuntArgs` with `--skip-chains`, `--skip-business`, `--skip-race`, `--skip-authz`, `--skip-session`, `--concurrency`, `--timeout`, `--format` (json/pretty), `--output`
- **Output**: JSON (default) or pretty text via `format_hunt_report()` (`hunt.rs:74-189`)

### Dispatch

- `TaskKind::Hunt(HuntParams)` dispatched at `dispatch/mod.rs:255-258`
- Executor: `security::run_hunt_task()` (`dispatch/security.rs:14-34`) — wraps in 60s `tokio::time::timeout`
- `TaskResult::Hunt(HuntReport)` at `dispatch/types.rs:114-115` (feature-gated)

### Pipeline

- No pipeline integration — hunt is standalone

### Output

- `AttackGraphBuilder::from_chains()` in `output/attack_graph.rs` converts `AttackChain` values to graph structures (feature-gated on `advanced-hunting`)

## Safety

- **Intrusive risk tier**: `OperationRisk::Intrusive` — requires explicit scope authorization
- **Scope requirement**: Target URL must pass `EnforcementContext::evaluate()` with loaded scope
- **Concurrency control**: Semaphore-based (`tokio::sync::Semaphore`) limits concurrent requests to `config.concurrency` (default 10)
- **Request timeout**: Per-request timeout from `config.timeout_ms` via `reqwest::Client::timeout()`
- **Task timeout**: 60s hard timeout in dispatch executor (`dispatch/security.rs:22-30`)
- **Same-host redirects**: `HuntClient` uses `same_host_redirect_policy(10)` — max 10 hops, same host only
- **No credential handling**: Hunt only probes unauthenticated endpoints; no login/credential logic

## Testing

| File | Test Count | Coverage |
|------|------------|----------|
| `mod.rs` | 3 | Client creation, config defaults, report creation |
| `authz.rs` | 1 | BypassType variant equality |
| `business.rs` | 1 | FlawType creation |
| `chain.rs` | 2 | AttackChain creation, empty report detection |
| `race.rs` | 1 | RaceType variant equality |
| `session.rs` | 1 | SessionIssueType variant equality |

Total: 9 tests across 6 files.

## Invariants & Gotchas

1. **Feature-gated**: Hunt compiles as a private stub when `advanced-hunting` is disabled — no dead code warnings, no runtime cost.
2. **Chain detection is correlational**: `detect_attack_chains()` makes zero HTTP requests — it only inspects findings accumulated by other sub-modules.
3. **Concurrent vs sequential**: Admin access, IDOR, sensitive files, and session fixation use semaphore-based concurrency. API discovery, error handling, rate limiting, force browsing, HTTP methods, and response inconsistency are sequential.
4. **IDOR detection is heuristic**: Any HTTP 200 response with >50 bytes body on IDOR-prone paths is flagged — high false-positive rate by design.
5. **Rate-limit test is simple**: 20 sequential GET requests; no 429 = no rate limiting. Does not test POST-specific limits.
6. **Race condition detection**: Concurrent requests use a uniform JSON body; real race conditions may require specific state (e.g., inventory count, balance). The test is a detection heuristic, not a proof.
7. **`FxHashSet` for status codes**: `race.rs:106` uses `rustc_hash::FxHashSet` for performance — not `std::collections::HashSet`.
8. **No `auth/` module interaction**: Hunt's `session` sub-module checks HTTP cookie/header security; `auth/session.rs` tests session fixation via login flow. They are independent.
9. **`HuntClient::build_url()`** (`mod.rs:62-76`): Handles absolute paths, relative paths, and full URLs — used by all sub-modules.

*Last verified against source: 2026-08-25*
