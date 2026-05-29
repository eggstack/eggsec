# Slapper Improvement Handoff Plan

Repository: `dbowm91/slapper`

Intended implementer: a smaller coding model such as MiMo v2.5, working in incremental PR-sized changes.

Primary objective: improve Slapper’s consistency, safety, maintainability, and extensibility without rewriting the project or adding high-risk offensive behavior. The repo already has broad capability; the highest-value work is tightening the feature contract, centralizing scope/policy enforcement, standardizing findings/evidence, and improving workflow primitives such as baselines, API testing, auth contexts, and agent-safe tool exposure.

## Global Constraints

Work incrementally. Prefer small, reviewable changes. Do not rewrite the CLI, pipeline, fuzzer, scanner, or output architecture unless a specific phase requires a small targeted refactor.

Do not add aggressive offensive functionality. Slapper is for legitimate, authorized testing. Any feature that can stress targets, send raw packets, credential-test, bypass WAFs, perform remote execution, or run autonomously must be policy-gated and fail closed by default.

Do not silently make aspirational features appear production-ready. If a feature is documented but not implemented, either implement the minimal viable wiring or document it as planned/partial.

Prefer explicit errors over panics. Avoid `unwrap()` in library code. Follow the existing convention: library modules should prefer project error types such as `SlapperError`; command handlers may use `anyhow::Result` at the boundary.

All new target-bearing functionality must go through scope and operation policy enforcement. All new result-producing functionality should emit or convert into canonical findings. All evidence that may contain secrets must be redacted by default.

## Phase 0: Repository Orientation and Baseline Checks

Before editing, inspect these files and directories:

```text
Cargo.toml
crates/slapper/Cargo.toml
crates/slapper/src/lib.rs
crates/slapper/src/main.rs
crates/slapper/src/cli/mod.rs
crates/slapper/src/commands/handlers/mod.rs
crates/slapper/src/config/
crates/slapper/src/types.rs
crates/slapper/src/output/
crates/slapper/src/pipeline/
crates/slapper/src/fuzzer/
crates/slapper/src/recon/
crates/slapper/src/scanner/
crates/slapper/src/waf/
architecture/overview.md
architecture/cli_commands.md
README.md
```

Run baseline checks where possible:

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

If feature-related build failures occur, record the exact failure and begin with Phase 1. Do not attempt large feature work before the feature matrix is coherent.

## Phase 1: Fix Cargo Feature Contract and Documentation Drift

### Problem

The repository documentation and `lib.rs` advertise feature flags that are not consistently declared in `crates/slapper/Cargo.toml`. Examples may include `full`, `headless-browser`, `database`, `container`, `sbom`, `cloud`, `api-schema`, `git-secrets`, `pdf`, `wireless`, and `websocket`. There may also be naming ambiguity such as `ws-api` versus `websocket`.

This is the first priority because it affects buildability, user trust, CI reliability, and downstream agent behavior.

### Goal

Make feature flags internally consistent across:

```text
crates/slapper/Cargo.toml
crates/slapper/src/lib.rs
README.md
architecture/overview.md
crates/slapper/src/cli/
crates/slapper/src/commands/
```

### Tasks

Create a feature inventory table by comparing every feature mentioned in docs or `#[cfg(feature = "...")]` against features declared in `crates/slapper/Cargo.toml`.

Add a new document:

```text
architecture/feature_matrix.md
```

The table should include these columns:

```text
Feature name
Declared in Cargo.toml: yes/no
Used in #[cfg]: yes/no
Exposes CLI command: yes/no
Primary module/crate
Status: implemented, partial, planned, docs-only
Build command
Notes
```

Fix the manifest. Add missing feature entries only when corresponding code or dependency wiring exists. If a feature is aspirational, do not declare it as complete. Mark it planned/partial in docs or implement the minimal wiring.

Recommended cleanup:

Add a real `full` feature only after confirming all included features compile.

Example:

```toml
full = [
    "tool-api",
    "rest-api",
    "ws-api",
    "grpc-api",
    "python-plugins",
    "ruby-plugins",
    "stress-testing",
    "packet-inspection",
    "nse",
    "nse-sandbox",
    "advanced-hunting",
    "compliance",
    "external-integrations",
    "finding-workflow",
    "vuln-management",
]
```

Only include `nse-ssh2`, `database`, `container`, `sbom`, `headless-browser`, or similar features in `full` if those features compile cleanly in the current repo and their dependencies are correctly declared.

Add missing feature entries when clearly wired:

```toml
database = ["sqlx"]
sbom = ["cyclonedx-bom", "spdx"]
pdf = ["printpdf"]
api-schema = ["openapiv3"]
headless-browser = ["headless_chrome"]
container = ["kube", "k8s-openapi"]
```

Only add the above if corresponding modules exist and compile.

Resolve naming ambiguity:

`ws-api` should mean WebSocket API server support.

`websocket` should mean WebSocket security testing.

Do not merge these unless the code proves they are the same.

Useful search commands:

```bash
rg '#\[cfg.*feature|cfg\(feature' crates/slapper/src crates/*/src
rg 'feature|--features|headless|websocket|database|container|sbom|cloud|api-schema|git-secrets|pdf|wireless|full' README.md architecture docs crates/slapper/src/lib.rs crates/slapper/Cargo.toml
```

### Acceptance Criteria

`cargo check --workspace` passes or all failures are unrelated and documented.

`cargo check -p slapper --features full` passes if README recommends `--features full`. Otherwise README must stop recommending it.

Every feature mentioned in `README.md` and `architecture/overview.md` exists in `crates/slapper/Cargo.toml` or is clearly marked planned/partial.

`architecture/feature_matrix.md` exists and is accurate.

No `#[cfg(feature = "...")]` uses an undeclared feature.

## Phase 2: Build a CLI/Feature Surface Audit Test

### Problem

The feature surface is large. Commands, docs, Cargo features, and module gates can drift easily.

### Goal

Add a lightweight internal audit test that catches obvious feature inconsistencies.

### Preferred Implementation

Add a test rather than a runtime command unless diagnostics infrastructure already exists.

Possible locations:

```text
crates/slapper/tests/feature_surface.rs
crates/slapper/src/cli/tests.rs
```

The test may be string-based. It does not need to parse all Rust syntax.

Test approach:

Read `crates/slapper/Cargo.toml`.

Extract keys under `[features]`.

Search source files for `feature = "..."`.

Assert every cfg feature exists in the manifest, except feature names that intentionally belong to workspace subcrates and are not used by `crates/slapper`.

Optionally verify README build examples only use declared features.

### Acceptance Criteria

A test fails when a future developer adds `#[cfg(feature = "new-feature")]` without declaring it in the manifest.

The test is simple and maintainable.

No heavy dependency is added solely for this test unless already present.

## Phase 3: Centralize Operation Risk and Scope Enforcement

### Problem

Scope enforcement appears handler-driven. Command handlers call scope validation methods, which is fragile because new handlers can forget to call them. Slapper has commands with very different risk levels, including passive recon, active scanning, fuzzing, load testing, stress testing, raw packets, proxy rotation, credential testing, remote execution, and autonomous agent/MCP execution.

### Goal

Introduce an `ExecutionPolicy` or `OperationPolicy` abstraction that classifies operations by risk tier and enforces scope and explicit permission before dangerous operations run.

### New Types

Add an operation risk enum. Candidate location: `crates/slapper/src/config/policy.rs` or `crates/slapper/src/types.rs`.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OperationRisk {
    Passive,
    ActiveScan,
    IntrusiveFuzz,
    LoadTest,
    StressTest,
    RawPacket,
    CredentialTesting,
    RemoteExecution,
    AgentAutonomous,
}
```

Add policy config with conservative defaults:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPolicy {
    pub require_explicit_scope: bool,
    pub allow_intrusive_fuzzing: bool,
    pub allow_load_testing: bool,
    pub allow_stress_testing: bool,
    pub allow_raw_packets: bool,
    pub allow_credential_testing: bool,
    pub allow_remote_execution: bool,
    pub allow_agent_autonomous: bool,
    pub max_risk_without_confirm: OperationRisk,
}
```

If adding this to existing config is disruptive, add backwards-compatible defaults and make the policy optional in config.

### Implementation Steps

Add a method to `CommandContext`:

```rust
pub fn enforce_operation_policy(
    &self,
    risk: OperationRisk,
    target: Option<&str>,
) -> anyhow::Result<()>;
```

This method should:

Validate target against scope if target exists.

Reject operations above allowed policy unless config explicitly enables them.

Produce clear errors.

Avoid interactive prompts in CI/JSON mode.

Require explicit config for high-risk tiers.

Add a mapping from CLI command to risk tier. This can be implemented inside each handler or centrally near `handle_command`. Prefer central mapping if practical.

Suggested risk classification:

```text
Passive:
  plan, config, doctor, report, vuln scoring without network, passive-only recon

ActiveScan:
  scan-ports, fingerprint, scan-endpoints, basic recon, basic waf detection

IntrusiveFuzz:
  fuzz, GraphQL injection modes, OAuth negative testing, WAF bypass

LoadTest:
  load

StressTest:
  stress, waf-stress

RawPacket:
  packet send, SYN/UDP/ICMP raw modes, raw-socket traceroute

CredentialTesting:
  auth-test brute force, credential stuffing, MFA tests

RemoteExecution:
  remote, exec

AgentAutonomous:
  agent run, MCP-exposed autonomous operations
```

### Acceptance Criteria

All target-bearing commands go through one policy method, even if handlers still do local validation.

Dangerous operations fail closed unless explicitly enabled.

Tests cover:

```text
Allowed target succeeds.
Excluded target fails.
Out-of-scope target fails when require_explicit_scope = true.
Stress/raw packet/credential/remote operation fails unless config enables that risk class.
JSON mode does not prompt.
```

## Phase 4: Standardize Canonical Finding and Evidence Schema

### Problem

Slapper emits many result types. Long-term usefulness requires consistent findings and evidence. This enables deduplication, baseline diffing, SARIF/JUnit/HTML/PDF output, storage, CI gating, agent memory, and finding lifecycle management.

### Goal

Create or refine a canonical `Finding` model and migrate output-producing modules toward it.

### Design Guidance

Inspect existing `types.rs`, `output/`, `vuln/`, and `storage/` before adding new types. If a good `Finding` type already exists, extend it rather than duplicating it.

Target shape:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub fingerprint: String,
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub confidence: Confidence,
    pub finding_type: FindingType,
    pub cwe: Option<String>,
    pub owasp: Option<String>,
    pub cve: Option<String>,
    pub affected_asset: AffectedAsset,
    pub location: FindingLocation,
    pub evidence: Vec<Evidence>,
    pub reproduction: Option<Reproduction>,
    pub remediation: Option<String>,
    pub discovered_at: DateTime<Utc>,
    pub source: FindingSource,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
}
```

Supporting types:

```rust
pub enum Confidence {
    Confirmed,
    High,
    Medium,
    Low,
    Informational,
}

pub enum EvidenceKind {
    HttpRequest,
    HttpResponse,
    Header,
    BodySnippet,
    Timing,
    Diff,
    Banner,
    DnsRecord,
    Certificate,
    PortState,
    Screenshot,
    FilePath,
    LogLine,
}

pub struct Evidence {
    pub kind: EvidenceKind,
    pub redacted: bool,
    pub summary: String,
    pub data: serde_json::Value,
}
```

Evidence must support redaction. Do not store raw credentials, bearer tokens, session cookies, API keys, private keys, or sensitive request bodies by default.

### Fingerprinting

Add stable finding fingerprints. A fingerprint should be deterministic across scan runs when the same issue is rediscovered.

Suggested fingerprint inputs:

```text
normalized target/asset
finding type
location path/param/port/header
CWE or vulnerability class
normalized evidence signature
```

Do not include timestamps or random IDs in fingerprints.

### Migration Strategy

Do not migrate every module at once. Start with one or two:

```text
Fuzzer findings
Scanner/fingerprint findings
WAF detection findings
```

Create conversion methods from legacy result types into canonical `Finding`.

Update output layer to prefer canonical findings.

Preserve backwards compatibility where existing JSON output tests depend on old structures.

### Acceptance Criteria

Canonical finding type exists and is documented.

At least fuzzer and scanner or WAF can emit canonical findings.

Output layer can serialize findings as JSON.

SARIF output uses stable rule IDs/fingerprints when possible.

Evidence redaction helper exists and is tested.

Tests verify fingerprint stability.

## Phase 5: Add Baseline and Differential Scan Support

### Problem

Slapper has sessions, storage concepts, and an autonomous agent, but needs a direct workflow for answering: what changed between scans?

### Goal

Implement a minimal `diff` or `baseline` capability comparing two result files or stored scan runs.

### MVP Command Options

Prefer whichever fits the existing CLI best:

```bash
slapper report diff old.json new.json
```

or

```bash
slapper diff old.json new.json
```

or

```bash
slapper storage diff --baseline prod --current latest
```

If a `report` command group already exists and accepts conversion/reporting subcommands, prefer `report diff`.

### Comparison Types

At minimum, compare canonical findings by fingerprint:

```text
New findings
Resolved findings
Persisting findings
Severity changes
Confidence changes
Evidence changes
```

Later expansions may compare assets, open ports, DNS records, headers, technologies, endpoints, WAF identity, and TLS certificate changes.

### Output

Support JSON and human-readable text.

Example JSON:

```json
{
  "new": [],
  "resolved": [],
  "persisting": [],
  "changed": [],
  "summary": {
    "new_count": 0,
    "resolved_count": 0,
    "persisting_count": 0,
    "changed_count": 0
  }
}
```

### Acceptance Criteria

Can compare two JSON result files.

Uses canonical finding fingerprints.

Produces deterministic output.

Has tests with fixture files.

Does not require database feature.

## Phase 6: API Testing Unification

### Problem

API security functionality is spread across `fuzz`, `graphql`, `oauth`, schema fuzzing docs, and scan profiles. Slapper should have a coherent API workflow.

### Goal

Create a unified API assessment workflow that can import schemas, discover endpoints, model auth, and generate type-aware fuzz targets.

### Scope

Do not build a full Burp/Postman replacement. Start with schema ingestion and target generation.

### Command Shape

Option A, new command group:

```bash
slapper api import openapi.yaml
slapper api discover https://example.com
slapper api fuzz --schema openapi.yaml --base-url https://api.example.com
slapper api diff old-openapi.yaml new-openapi.yaml
```

Option B, integrate with existing `fuzz`:

```bash
slapper fuzz --schema openapi.yaml --schema-type openapi --base-url https://api.example.com
```

Prefer a new `api` command group only if CLI complexity remains manageable.

### MVP Features

Parse OpenAPI 3 JSON/YAML using optional `openapiv3`.

Generate endpoint/method/parameter inventory.

Generate fuzz targets based on parameter location:

```text
path
query
header
cookie
body
```

Generate type-aware payloads for:

```text
string
integer
number
boolean
array
object
enum
date/time format
uuid format
email format
uri format
```

Extract auth scheme metadata:

```text
bearer
basic
apiKey header/query/cookie
OAuth metadata when present
```

Do not defeat authentication automatically. Only use provided credentials or auth context.

### Acceptance Criteria

Given a small OpenAPI fixture, Slapper can enumerate operations.

Given that fixture, Slapper can generate fuzz targets.

Tests cover JSON and YAML OpenAPI files.

Feature flag `api-schema` is declared and works.

If the feature is disabled, the command is hidden or returns a clear error.

## Phase 7: Auth Context and Role-Based Testing

### Problem

IDOR/BOLA, session handling, CSRF, tenant isolation, and privilege escalation testing require multiple authenticated contexts. Generic bearer/cookie flags are insufficient.

### Goal

Add an auth context file format representing users, roles, tokens/cookies, login recipes, and refresh behavior.

### MVP File Format

Add `auth-context.yaml` support:

```yaml
version: 1
contexts:
  user:
    description: "Normal user"
    headers:
      Authorization: "Bearer ${USER_TOKEN}"
    cookies: {}
  admin:
    description: "Admin user"
    headers:
      Authorization: "Bearer ${ADMIN_TOKEN}"
    cookies: {}
```

Do not store secrets in plaintext in examples. Support environment variable interpolation.

Possible later extension:

```yaml
login:
  method: POST
  url: "https://example.com/login"
  body:
    username: "${USERNAME}"
    password: "${PASSWORD}"
  extract:
    token:
      from: json
      path: "$.access_token"
```

### CLI Integration

Add common HTTP args:

```bash
--auth-context auth-context.yaml
--auth-role user
```

For role-differential tests:

```bash
slapper auth compare --context auth-context.yaml --roles user,admin --url https://api.example.com/users/123
```

Or integrate with fuzzer:

```bash
slapper fuzz https://api.example.com/users/123 -t idor --auth-context auth.yaml --roles user,admin
```

### Acceptance Criteria

Auth context parser exists.

Environment variable interpolation works.

HTTP client builder can apply selected context.

Secrets are redacted from logs and output.

At least one command can use `--auth-context` and `--auth-role`.

Tests verify headers/cookies are applied and redacted.

## Phase 8: Browser/SPA Crawl as Request Corpus Generator

### Problem

Modern web apps hide endpoints behind client-side routing and JavaScript fetch/XHR calls. Browser crawling should produce useful input for endpoint discovery and fuzzing.

### Goal

Make browser crawling produce a normalized request corpus that can feed scanner and fuzzer workflows.

### Feature Flag

Use `headless-browser`, wired to `headless_chrome`, only if the module exists and compiles.

### MVP Command

```bash
slapper browser crawl https://example.com --output corpus.json
```

If adding a new command is too much, expose through endpoint scanning:

```bash
slapper scan-endpoints https://example.com --browser --output corpus.json
```

### Corpus Contents

```text
Visited routes
Observed network requests
HTTP methods
URLs
Headers with sensitive values redacted
Content types
Request body shape, not raw secrets
Detected forms
Detected GraphQL endpoint candidates
Detected OpenAPI/Swagger links
JavaScript source URLs
Potential DOM sinks if available
```

### Acceptance Criteria

Can crawl a basic SPA fixture or simple local test page.

Outputs JSON corpus.

Fuzzer or scanner integration can be a later phase; MVP may only generate corpus.

Secrets are redacted.

Feature-gated build passes.

## Phase 9: Storage and Finding Lifecycle

### Problem

The repo has storage/vuln/workflow concepts, but the most useful workflow is persistent finding history and triage.

### Goal

Add or refine storage for canonical findings and scan runs.

### MVP

If `database` is wired to Postgres/SQLx, define tables for:

```text
scan_runs
assets
findings
evidence
finding_events
```

If database support is too heavy, start with local JSONL unless another lightweight dependency already exists. Do not add unnecessary database complexity.

### Finding Lifecycle States

```rust
pub enum FindingStatus {
    New,
    Confirmed,
    AcceptedRisk,
    FalsePositive,
    Remediated,
    Reopened,
}
```

Candidate commands:

```bash
slapper vuln list
slapper vuln show <fingerprint>
slapper vuln mark <fingerprint> --status false-positive
slapper vuln export --format json
```

### Acceptance Criteria

Can persist canonical findings.

Can list findings by severity/status.

Can update status.

Can diff latest scan against previous stored scan.

Feature-gated if database-backed.

## Phase 10: Report Quality and Redaction

### Problem

Security reports can leak tokens, cookies, API keys, private keys, or sensitive request bodies if evidence is stored raw.

### Goal

Make reporting safer and more useful.

### Tasks

Add a central redaction utility:

```rust
pub fn redact_sensitive(input: &str) -> String
```

It should handle:

```text
Authorization headers
Bearer tokens
Basic auth
Cookies
API keys
JWT-looking strings
Private key blocks
Common secret key names
```

Add structured report summary:

```text
Counts by severity
Counts by confidence
Counts by finding type
New/resolved/persisting counts if diff context exists
Top affected assets
Risk narrative
Remediation summary
```

### Acceptance Criteria

Report formats use redacted evidence by default.

There is an explicit unsafe flag if raw evidence output is ever needed, named clearly, for example `--include-sensitive-evidence`.

Tests cover common secret redaction cases.

JSON report schema is stable enough for CI use.

## Phase 11: Agent and MCP Safety Boundary

### Problem

Slapper exposes powerful behavior through autonomous agent and MCP/REST/gRPC layers. Agent-accessible tools need stricter risk controls than human CLI use.

### Goal

Make agent-exposed tools policy-aware and safe by default.

### Tasks

Ensure all MCP/tool registry operations declare metadata:

```text
Name
Description
Input schema
Output schema
Risk tier
Requires target scope: yes/no
Requires explicit enablement: yes/no
Can mutate state: yes/no
Can send network traffic: yes/no
Can stress/load test: yes/no
Can run raw packet operations: yes/no
```

Create a checked tool execution guard:

```rust
ToolRegistry::execute_checked(tool_name, input, policy, scope)
```

This should enforce the same policy system from Phase 3.

Disable high-risk tools from agent/MCP by default:

```text
stress
raw packet send
credential testing
remote exec
WAF bypass if considered intrusive
load test above low rate
```

### Acceptance Criteria

Agent/MCP tools cannot bypass CLI scope/policy checks.

High-risk tools require explicit config.

Tool metadata includes risk tier.

Tests verify blocked tool execution when policy disallows it.

## Phase 12: Supply-Chain and Repository Analysis

### Problem

Slapper already mentions dependency scanning and SBOM features. This is a high-value, CI-friendly expansion and safer than live-target intrusive testing.

### Goal

Create a coherent `supply-chain` or `repo` workflow.

### MVP Commands

```bash
slapper supply-chain scan /path/to/repo
slapper supply-chain sbom /path/to/repo --format cyclonedx
slapper supply-chain secrets /path/to/repo
```

Alternatively integrate with existing `ci` if that fits better.

Detect:

```text
Cargo.toml/Cargo.lock
package.json/package-lock.json/yarn.lock/pnpm-lock.yaml
go.mod/go.sum
Dockerfile
GitHub Actions workflows
```

Basic checks:

```text
Dependency manifest inventory
Known risky script hooks
GitHub Actions overly broad permissions
Hardcoded secrets using existing secret detector
SBOM generation if sbom feature enabled
```

Do not build a full SCA database unless already present. Prefer inventory generation and integration with existing CVE mapping.

### Acceptance Criteria

Can scan a local test repo fixture.

Produces canonical findings.

Can emit SARIF for CI.

Feature flags are coherent.

## Phase 13: Service Fingerprinting Refinement

### Problem

Service fingerprinting is useful only when confidence, evidence, and uncertainty are explicit.

### Goal

Improve fingerprint output quality rather than merely adding more probes.

### Tasks

Add confidence scores to service fingerprints.

Capture probe evidence:

```text
banner
TLS certificate fields
ALPN result
HTTP headers
protocol negotiation behavior
```

Add normalized service identity:

```text
service name
version
product
vendor
protocol
transport
port
confidence
```

Handle uncertainty:

```text
unknown version
conflicting fingerprints
possible services
```

Map to CVEs only when version confidence is high enough. Otherwise emit possible exposure or informational evidence, not confirmed vulnerability.

### Acceptance Criteria

Fingerprint result type includes confidence and evidence.

Output distinguishes confirmed versus inferred service/version.

CVE mapping does not overclaim.

Tests cover ambiguous fingerprints.

## Phase 14: Documentation and Examples Cleanup

### Problem

The README is broad, but it should clearly distinguish stable, feature-gated, experimental, partial, and planned capabilities.

### Goal

Make docs accurately reflect current capability.

### Tasks

Update README sections:

```text
Build features
Quick start
Advanced features
Safety/scope
Agent/MCP usage
Output formats
```

Add or update docs:

```text
architecture/feature_matrix.md
docs/SAFETY.md
docs/FINDINGS_SCHEMA.md
docs/AUTH_CONTEXT.md
docs/API_TESTING.md
docs/BASELINES_AND_DIFFS.md
```

Add examples:

```text
examples/scope.toml
examples/auth-context.yaml
examples/openapi.yaml
examples/baseline-diff/old.json
examples/baseline-diff/new.json
```

### Acceptance Criteria

All documented commands compile or are clearly marked as requiring a feature.

All feature build examples are valid.

Dangerous operations are documented with explicit authorization requirements.

Docs do not imply planned features are production-ready.

## Phase 15: CI Matrix

### Problem

Feature-heavy Rust projects regress easily when optional features are not checked.

### Goal

Add or refine CI checks for important feature combinations.

### Suggested Matrix

Default:

```bash
cargo check --workspace
cargo test --workspace
```

Minimal CLI:

```bash
cargo check -p slapper
```

Feature groups:

```bash
cargo check -p slapper --features rest-api
cargo check -p slapper --features grpc-api
cargo check -p slapper --features packet-inspection
cargo check -p slapper --features stress-testing
cargo check -p slapper --features nse
cargo check -p slapper --features nse,nse-sandbox
cargo check -p slapper --features all-plugins
cargo check -p slapper --features full
```

Only include expensive or system-dependent features if CI supports them. If Ruby/Python/NSE dependencies are painful, separate them into optional CI jobs.

### Acceptance Criteria

CI catches undeclared/miswired features.

CI checks formatting and clippy.

CI does not require privileged raw sockets.

High-risk integration tests are mocked or ignored by default.

## Suggested PR Breakdown

### PR 1: Feature matrix and Cargo feature cleanup

Likely files:

```text
crates/slapper/Cargo.toml
crates/slapper/src/lib.rs
README.md
architecture/overview.md
architecture/feature_matrix.md
```

### PR 2: Feature surface audit test

Likely files:

```text
crates/slapper/tests/feature_surface.rs
```

Avoid new dependencies if possible.

### PR 3: Execution policy and risk tiers

Likely files:

```text
crates/slapper/src/config/
crates/slapper/src/commands/
crates/slapper/src/types.rs
README.md
docs/SAFETY.md
```

### PR 4: Canonical finding schema and redaction utility

Likely files:

```text
crates/slapper/src/types.rs
crates/slapper/src/output/
crates/slapper/src/utils/
docs/FINDINGS_SCHEMA.md
```

### PR 5: Migrate fuzzer and scanner/WAF findings to canonical schema

Likely files:

```text
crates/slapper/src/fuzzer/
crates/slapper/src/scanner/
crates/slapper/src/waf/
crates/slapper/src/output/
```

### PR 6: Baseline diff MVP

Likely files:

```text
crates/slapper/src/cli/
crates/slapper/src/commands/handlers/
crates/slapper/src/output/
crates/slapper/src/diff/
examples/baseline-diff/
```

### PR 7: API schema MVP

Likely files:

```text
crates/slapper/Cargo.toml
crates/slapper/src/cli/
crates/slapper/src/fuzzer/api_schema/
crates/slapper/src/api/
docs/API_TESTING.md
```

### PR 8: Auth context MVP

Likely files:

```text
crates/slapper/src/config/
crates/slapper/src/cli/mod.rs
HTTP client builder/helper modules
docs/AUTH_CONTEXT.md
```

### PR 9: Agent/MCP policy enforcement

Likely files:

```text
crates/slapper/src/tool/
crates/slapper/src/agent/
crates/slapper/src/config/policy.rs
```

## Highest-Value Initial Milestone

Start with Phases 1 through 4. These are foundational and will make every later feature easier.

Milestone statement:

```text
Make Cargo features, docs, and cfg gates internally consistent; add an audit test; introduce central operation risk policy; define canonical findings and redaction.
```

This improves trust, safety, maintainability, CI stability, and agent-readiness without requiring speculative feature work.
