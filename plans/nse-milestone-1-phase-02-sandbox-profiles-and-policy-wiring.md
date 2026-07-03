# NSE Milestone 1 Phase 02: Sandbox Profiles and Policy Wiring

## Purpose

Introduce explicit NSE execution profiles and wire policy decisions from Eggsec's operation enforcement layer into the `eggsec-nse` runtime.

The goal is to preserve manual CLI/TUI operator discretion while making automated agent/MCP/daemon surfaces fail closed unless a safe profile and scope-derived permissions are selected.

## Current Problem

The current sandbox model is too implicit for production-grade use.

Important issues:

- `SandboxConfig::default()` enables sandboxing only when the `sandbox` feature is compiled.
- Empty `allowed_networks` currently permits all network targets.
- The CLI handler performs central operation enforcement before invoking NSE, but the resulting operation policy does not clearly flow into `eggsec-nse` as a concrete runtime profile.
- CLI help indicates stricter expectations than the operation descriptor currently enforces.
- Manual and automated surfaces are not represented as first-class policy choices inside the NSE runtime.

This makes it too easy for an automated surface to instantiate a permissive executor accidentally.

## Target State

Add explicit `NseExecutionProfile` presets that resolve into `SandboxConfig`, `NseExecutionLimits`, script loading policy, module loading policy, and audit metadata.

The runtime should not need to guess whether it is being used manually or by an agent. That should be selected explicitly by the caller or derived from central operation enforcement.

## Proposed Profiles

### ManualPermissive

Purpose: preserve operator discretion for local CLI/TUI usage.

Default behavior:

- Allows custom script files.
- Allows configured script roots and conventional Nmap `nselib` paths if enabled.
- Allows network to target unless blocked by central scope/policy.
- Allows broader compatibility behavior.
- Logs warnings when sandbox is not compiled or not active.

This profile should be unsuitable for agent/MCP/daemon use.

### ManualStrict

Purpose: safer manual usage without becoming agent-grade restrictive.

Default behavior:

- Allows custom script files only under approved roots.
- Allows network only to explicit target/scope-derived allowlist.
- Disables process execution by default.
- Restricts filesystem to approved read roots and working directory.
- Uses moderate execution limits.

### AgentSafe

Purpose: safe default for agent, MCP, daemon, and autonomous operation.

Default behavior:

- Built-in scripts only unless a trusted script registry is explicitly enabled.
- No ambient Nmap script paths.
- No arbitrary `script_file`.
- No shell/process execution.
- Filesystem access disabled or restricted to an isolated temp work directory.
- Network access limited to explicit scope-derived target IPs/CIDRs.
- Empty network allowlist means deny all, not allow all.
- Strict execution limits.
- Structured audit event required.

### CiSafe

Purpose: deterministic test profile.

Default behavior:

- No external network by default.
- Fixture-only script roots.
- Strict time/output/resource limits.
- Stable diagnostics suitable for snapshot tests.

### CompatibilityLab

Purpose: explicit compatibility testing against broader NSE behavior.

Default behavior:

- May permit conventional script roots and looser module loading.
- Must be clearly labelled as not agent-safe.
- Should require an explicit CLI flag or test-only configuration.

## Proposed Types

Introduce a profile enum:

```rust
pub enum NseExecutionProfileKind {
    ManualPermissive,
    ManualStrict,
    AgentSafe,
    CiSafe,
    CompatibilityLab,
}
```

Introduce a resolved profile:

```rust
pub struct ResolvedNseExecutionProfile {
    pub kind: NseExecutionProfileKind,
    pub sandbox: SandboxConfig,
    pub limits: NseExecutionLimits,
    pub script_policy: NseScriptPolicy,
    pub module_policy: NseModulePolicy,
    pub audit_label: String,
    pub warnings: Vec<String>,
}
```

Introduce script/module policy types:

```rust
pub struct NseScriptPolicy {
    pub allow_builtin_scripts: bool,
    pub allow_script_files: bool,
    pub allowed_script_roots: Vec<PathBuf>,
    pub allow_conventional_nmap_paths: bool,
    pub max_script_bytes: Option<usize>,
}

pub struct NseModulePolicy {
    pub allow_builtin_modules: bool,
    pub allow_filesystem_modules: bool,
    pub allowed_module_roots: Vec<PathBuf>,
    pub max_module_bytes: Option<usize>,
}
```

Names can be changed to match house style, but the separation between profile kind, resolved sandbox, limits, script policy, and module policy should remain.

## Implementation Steps

### Step 1: Add Profile Types

Add the profile types inside `eggsec-nse`, likely near `SandboxConfig` or in a new `profile.rs` module.

Profiles should be serializable/debuggable if existing crate conventions support that. At minimum, they should be printable for audit and CLI diagnostics.

### Step 2: Define Profile Defaults

Implement constructors:

```rust
impl ResolvedNseExecutionProfile {
    pub fn manual_permissive(target: Option<&str>) -> Self;
    pub fn manual_strict(target: Option<&str>, scope: ScopeInput) -> Result<Self>;
    pub fn agent_safe(target: &str, scope: ScopeInput) -> Result<Self>;
    pub fn ci_safe() -> Self;
    pub fn compatibility_lab(target: Option<&str>) -> Self;
}
```

Do not require these exact names. The important part is that profile defaults are explicit and reviewed.

### Step 3: Change Empty Network Allowlist Semantics Per Profile

Do not rely on one global meaning for empty `allowed_networks`.

For manual permissive compatibility, empty may continue to mean unrestricted if that is intentional.

For `AgentSafe`, `ManualStrict`, and `CiSafe`, empty must mean deny all. Prefer representing this explicitly instead of overloading an empty vector:

```rust
pub enum NseNetworkPolicy {
    AllowAllManual,
    DenyAll,
    AllowCidrs(Vec<IpNetwork>),
    AllowResolvedTargetSet(Vec<IpAddr>),
}
```

If changing `SandboxConfig` directly is too disruptive, add profile-level network policy first and adapt it into the existing sandbox checks.

### Step 4: Derive Network Allowlist From Target and Scope

For strict profiles, derive allowed networks from:

- exact target IP.
- resolved target IP set.
- explicit scope CIDR.
- localhost/private-only policy if selected.

Avoid hostname-only allow decisions for strict automated mode unless DNS rebinding risk is explicitly handled. Prefer resolving once, pinning the resulting IP set for the run, and connecting only to pinned IPs.

### Step 5: Wire CLI Handler to Profile Selection

Update the NSE command handler so central operation enforcement produces or selects an NSE profile before calling `eggsec-nse::run_cli` or equivalent.

Required behavior:

- Manual CLI defaults to `ManualPermissive` or `ManualStrict`, whichever matches project UX preference.
- A CLI flag may allow explicit profile selection later, but this phase can hardcode the current intended manual behavior.
- If help text says private/localhost/scope is required, the operation descriptor must enforce that. Otherwise update the help text to match actual manual behavior.
- Agent/MCP/daemon paths must not call the manual default constructor.

### Step 6: Pass Profile Into the NSE Runtime

Replace runtime construction paths that only pass target/script args with runtime construction paths that include the resolved profile.

Suggested flow:

1. CLI parses `NseArgs`.
2. CLI central enforcement evaluates operation descriptor.
3. Handler selects `ResolvedNseExecutionProfile`.
4. Handler constructs `NseRunRequest` with script source, target, args, profile.
5. `eggsec-nse` executor is created from that request.

### Step 7: Add Audit and Diagnostics

Every NSE run should expose:

- selected profile.
- sandbox enabled/disabled.
- network policy summary.
- script loading policy summary.
- execution limits summary.
- warnings if compiled without `sandbox` but a strict profile was requested.

For JSON output, include this in structured metadata once the report model lands. For this phase, text diagnostics or tracing events are acceptable.

### Step 8: Add Tests

Required tests:

- `AgentSafe` rejects arbitrary script files.
- `AgentSafe` rejects empty network allowlist.
- `AgentSafe` allows an explicitly scoped target.
- `ManualPermissive` preserves existing manual compatibility behavior.
- `ManualStrict` rejects traversal/out-of-root script paths once Phase 3 lands.
- CLI handler selects a profile and does not bypass profile construction.
- Help text and operation descriptor agree on whether scope/private/local target is required.

## Documentation Updates

Update CLI help and crate docs to explain:

- manual profile behavior.
- automated profile behavior.
- how sandbox feature gating affects runtime behavior.
- how network allowlists are derived.
- how script files are handled by profile.

Avoid language implying full Nmap behavior. Use "selective NSE compatibility" consistently.

## Verification Commands

Run at least:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
make test-nse
```

Add CLI integration tests if the repo already has a pattern for them.

## Acceptance Criteria

This phase is complete when:

- NSE runtime construction requires or derives an explicit profile for new call paths.
- Manual and automated surfaces no longer share implicit permissive defaults.
- Empty network allowlist denies all in strict/agent/CI profiles.
- Agent/MCP/daemon use cannot run arbitrary script files by default.
- Sandbox disabled-at-compile-time is visible as a warning or hard error depending on requested profile.
- CLI help and operation enforcement agree.
- Tests cover profile selection and strict-profile denial behavior.

## Reviewer Checklist

- Verify all non-test `NseExecutor::new()` or `with_target()` uses are intentional.
- Verify automated surfaces do not use `ManualPermissive`.
- Verify strict profiles fail closed when target/scope cannot produce an allowlist.
- Verify docs do not overstate sandbox guarantees when the feature is not compiled.
- Verify the profile model will compose with the Phase 3 script resolver.
