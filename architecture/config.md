# Configuration & Enforcement System

Deep-dive into the configuration, scope enforcement, and policy evaluation system.

> Parent: [overview.md](overview.md)
> Related: [runtime_bridge.md](runtime_bridge.md), [dispatch.md](dispatch.md), [audit.md](audit.md)

## Role & Responsibilities

The `config` module owns all configuration loading/validation and the **mandatory pre-dispatch enforcement gate** that every execution surface must pass before running an operation.

**Responsibilities:**

- Load, parse, and validate `EggsecConfig` from TOML/YAML files
- Load, parse, and validate `Scope` manifests (allowed/excluded targets, port rules)
- Track scope provenance via `LoadedScope` (source of scope for strict-profile enforcement)
- Resolve hostnames to IP addresses via the `HostResolver` trait
- Classify IP addresses into typed address classes
- Produce typed `PolicyDecision` records with denial/warning classification
- Enforce per-operation risk, feature, capability, and scope policy via `EnforcementContext::evaluate()`
- Gate strict automated surfaces with the `ApprovedOperation` token
- Provide preflight policy preview without dispatching
- Define `ExecutionBudget` constraints for stress/load/packet operations
- Define defense-lab `DefenseLabPreset` presets
- Maintain the canonical `ALL_OPERATION_METADATA` registry (31 operations, 42 aliases)
- Maintain the authoritative compile-time feature registry

**Non-responsibilities:**

- Dispatching operations to tool implementations (owned by `tool/`, `commands/handlers/`)
- TUI rendering (owned by `eggsec-tui`)
- Report formatting (owned by `eggsec-output`)
- Task lifecycle management (owned by `eggsec-runtime`)
- Session persistence (owned by `eggsec-daemon`)

## Location & Feature Gating

All source files live under `crates/eggsec/src/config/`:

| File | Lines | Feature Gate | Purpose |
|------|-------|:---:|---------|
| `mod.rs` | 127 | — | Re-exports, `ENV_PREFIX`, default config template |
| `policy.rs` | 2394 | — | `OperationRisk`, `ExecutionPolicy`, `OperationMode`, `IntendedUse`, `ExecutionSurface`, `ExecutionProfile`, `Capability`, `DenialClass`, `OperationDescriptor`, `OperationMetadata`, `ALL_OPERATION_METADATA` (31), `ALL_OPERATION_METADATA_ALIASES` (42) |
| `policy_decision.rs` | 3759 | — | `PolicyDecision`, `EnforcementOutcome`, `EnforcementContext`, `ApprovedOperation`, `ConfirmationClass` (8), `ManualOverride`, `PreflightResult`, `EnforcementError`, `classify_denial_reasons()`, `evaluate_enforcement()`, `evaluate_operation_policy()`, `confirmation_classes_for()` |
| `scope.rs` | ~1570 | — | `Scope`, `ScopeRule`, `ScopeSource` (4), `LoadedScope`, `AddressClass` (7), `HostResolver` trait, `SystemResolver`, `TargetScope`, `classify_address()`, `is_private_ip()` |
| `settings.rs` | 744 | — | `EggsecConfig`, `ScanConfig`, `HttpConfig`, `OutputConfig`, `NotificationConfig`, `AiConfig`, `ReconConfig`, `RemoteConfig`, `SearchConfig`, `AlertChannelsConfig`, `ProxyConfigEntry`, `ConfigError`, validation impls |
| `loader.rs` | 478 | — | `load_config()`, `load_scope()`, `load_scope_with_source()`, `find_config_file()`, `find_scope_file()`, config search order |
| `scan.rs` | 267 | — | `ScanConfig`, `ScanProfile`, `FuzzProfile`, `OutputConfig`, `NotificationConfig`, `WebhookConfig`, `WebhookEvent` |
| `http.rs` | 75 | — | `HttpConfig`, `Verbosity` (4 variants) |
| `api.rs` | 76 | — | `ApiConfig`, `ApiKeyConfig`, `IpApiConfig`, `MaxMindConfig`, `NvdConfig`, `WaybackConfig` |
| `budget.rs` | 217 | — | `ExecutionBudget`, `BudgetError` (3 variants) |
| `discovery.rs` | 70 | — | `DiscoveredTargetStatus` (4 variants) |
| `feature_registry.rs` | 531 | — | `FeatureEntry`, `FeatureState`, `FeatureCategory`, `ALL_FEATURES` (~48 entries), `feature_state()`, `is_feature_enabled()`, `is_known_feature()`, `feature_missing_hint()` |
| `presets.rs` | 233 | — | `DefenseLabPreset` (7 built-in presets) |

**Feature gating:** The config module itself requires no feature gates. Individual `OperationMetadata` entries declare `required_features` that are checked at evaluation time via `feature_registry::is_feature_enabled()`.

## Architecture

### Key Types — Enum Variant Counts

| Type | File:Line | Variants | Purpose |
|------|-----------|----------|---------|
| `OperationRisk` | `policy.rs:9` | 15 | Risk tier ordering (Passive → AgentAutonomous) |
| `ExecutionSurface` | `policy.rs:357` | 9 | Caller origin identity |
| `ExecutionProfile` | `policy.rs:461` | 5 | Trust boundary for enforcement |
| `OperationMode` | `policy.rs:179` | 3 | Session safety boundary |
| `Capability` | `policy.rs:507` | 19 | Operation capability declarations |
| `IntendedUse` | `policy.rs:227` | 8 | Operation use-case classification |
| `DenialClass` | `policy.rs:573` | 8 | Typed denial classification |
| `ConfirmationClass` | `policy_decision.rs:402` | 8 | Manual discretion trigger categories |
| `EnforcementOutcome` | `policy_decision.rs:236` | 4 | Profile-aware enforcement result |
| `AddressClass` | `scope.rs:14` | 7 | IP address classification |
| `ScopeSource` | `scope.rs:201` | 4 | Scope provenance |
| `DescriptorError` | `policy.rs:1175` | 2 | Target-policy violation errors |
| `DiscoveredTargetStatus` | `discovery.rs:10` | 4 | Discovery promotion model |
| `BudgetError` | `budget.rs:107` | 3 | Budget validation errors |
| `ConfigError` | `settings.rs:708` | 4 | Config loading/parsing errors |
| `ScopeError` | `scope.rs:931` | 7 | Scope validation/loading errors |

### Key Types — Structs

| Type | File:Line | Purpose |
|------|-----------|---------|
| `EggsecConfig` | `settings.rs:92` | Main configuration struct |
| `ExecutionPolicy` | `policy.rs:33` | Operation policy controls (14 boolean flags + risk + capabilities) |
| `OperationDescriptor` | `policy.rs:279` | Unit of policy evaluation |
| `OperationMetadata` | `policy.rs:1202` | Static metadata for one operation (17 fields) |
| `PolicyDecision` | `policy_decision.rs:11` | Fully-populated enforcement decision record (17 fields) |
| `EnforcementContext` | `policy_decision.rs:472` | Bundles profile + policy + scope for shared evaluation |
| `ApprovedOperation` | `policy_decision.rs:331` | Proof-of-enforcement token (private fields) |
| `ManualOverride` | `policy_decision.rs:432` | Manual override flags (10 fields) |
| `PreflightResult` | `policy_decision.rs:733` | Read-only pre-dispatch policy preview |
| `Scope` | `scope.rs:274` | Allowed/excluded targets + port rules |
| `ScopeRule` | `scope.rs:554` | Single scope rule (pattern or CIDR) |
| `LoadedScope` | `scope.rs:217` | Scope + provenance metadata |
| `TargetScope` | `scope.rs:677` | Parsed target with resolved addresses |
| `ExecutionBudget` | `budget.rs:8` | Execution constraints (10 fields) |
| `DefenseLabPreset` | `presets.rs:7` | Preset lab constraints (15 fields) |
| `HttpConfig` | `http.rs:19` | HTTP client settings (10 fields) |
| `ScanConfig` | `scan.rs:17` | Scanning settings (9 fields) |

### Static Registries

| Registry | File:Line | Count | Purpose |
|----------|-----------|-------|---------|
| `ALL_OPERATION_METADATA` | `policy.rs:1494` | 31 | Canonical operation definitions |
| `ALL_OPERATION_METADATA_ALIASES` | `policy.rs:2027` | 42 | Tool-ID → canonical-ID mappings |
| `ALL_FEATURES` | `feature_registry.rs:110` (generated) | ~48 | Compile-time feature registry |

### ExecutionSurface → ExecutionProfile Mapping

| Surface | Profile | Manual Override | File:Line |
|---------|---------|:---:|-----------|
| `CliManual` | `ManualPermissive` | Yes | `policy.rs:382` |
| `TuiManual` | `ManualPermissive` | Yes | `policy.rs:382` |
| `CliManualStrict` | `ManualGuarded` | No | `policy.rs:383` |
| `TuiManualStrict` | `ManualGuarded` | No | `policy.rs:383` |
| `McpServer` | `McpStrict` | No | `policy.rs:384` |
| `SecurityAgent` | `AgentStrict` | No | `policy.rs:385` |
| `Ci` | `CiStrict` | No | `policy.rs:386` |
| `RestApi` | `McpStrict` | No | `policy.rs:387` |
| `GrpcApi` | `McpStrict` | No | `policy.rs:388` |

### Capability Classification

Baseline capabilities allowed by default for strict profiles (no explicit allow needed):
- `PassiveFingerprint`, `ActiveProbe`, `Crawl`, `WafDetect`

Defined by `baseline_allowed_capability()` at `policy.rs:557`. All other capabilities require explicit listing in `ExecutionPolicy::allowed_capabilities` for strict automated profiles.

## Enforcement Flow

### `EnforcementContext::evaluate()` — Central Entry Point

`EnforcementContext::evaluate()` at `policy_decision.rs:561` is the **mandatory pre-dispatch gate** for all surfaces. The step-by-step flow:

1. **Inner evaluation**: Calls `evaluate_enforcement(descriptor, policy, Some(&scope), profile)` at `policy_decision.rs:562`.

2. **Provenance gate** (`policy_decision.rs:569`): For automated profiles (`CiStrict`, `McpStrict`, `AgentStrict`) with target-bearing operations that set `requires_explicit_scope`:
   - If `loaded_scope.is_explicit_manifest() == false` (i.e. `DefaultEmpty`):
     - Returns `EnforcementOutcome::Deny` with `DenialClass::ScopeMissing`

3. **Feature checks** (`evaluate_operation_policy` at `policy_decision.rs:935`): For each `required_feature` in the descriptor:
   - If `is_feature_enabled(feature) == false`: pushes `DenialClass::FeatureMissing`, sets `allowed = false`

4. **Scope evaluation** (`policy_decision.rs:947`): If target and scope are provided:
   - Resolves target to addresses via `TargetScope`
   - Checks exclusion rules first (exclusion wins)
   - Checks allowed rules via `evaluate_addresses()` for all-resolved-address evaluation
   - Returns `DenialClass::ExplicitExclusion`, `DenialClass::TargetOutOfScope`, or `DenialClass::InvalidTarget` as appropriate
   - If scope is missing and operation requires it: `DenialClass::ScopeMissing`

5. **Risk check** (`policy_decision.rs:1017`): If `descriptor.risk.is_allowed_by(policy) == false`:
   - Returns `DenialClass::RiskPolicyDenied`

6. **Policy flag checks** (`policy_decision.rs:1029`): For each `required_policy_flags` entry.

7. **Capability checks** (`evaluate_enforcement` at `policy_decision.rs:1180`):
   - Denied capabilities always deny (hard)
   - Strict profiles: non-baseline capabilities require explicit allow; missing = `DenialClass::CapabilityDenied`

8. **ManualPermissive downgrade logic** (`policy_decision.rs:1214`):
   - For safe (Passive/SafeActive), StandardAssessment ops with only ScopeMissing/TargetOutOfScope:
     - If no positive scope rules declared: **downgrade to `Warn`** (allowed with warnings)
     - If positive rules declared but target missed: **`RequireConfirmation`** (operator discretion)
   - For explicit exclusion, high-risk, non-baseline capability: **`RequireConfirmation`**
   - Hard denials (feature missing, invalid target, capability denied, risk-policy denied) **stay Deny**

9. **Strict profiles** (`policy_decision.rs:1323`):
   - Missing scope for networked ops → `Deny`
   - Scope ambiguity (target present, no matched rules) → `Deny`
   - Any warnings → `Deny`
   - Otherwise → `Allow`

### EnforcementOutcome Variants

| Variant | Meaning | Proceed? |
|---------|---------|:---:|
| `Allow(PolicyDecision)` | Operation authorized | Yes |
| `Warn(PolicyDecision)` | Authorized with warnings | Yes (surface-dependent) |
| `RequireConfirmation(PolicyDecision)` | Manual-only; needs override flags | ManualPermissive only |
| `Deny(PolicyDecision)` | Operation blocked | No |

### Approval Methods

| Method | Accepts | Rejects | Use Case |
|--------|---------|---------|----------|
| `approve(surface, descriptor)` | `Allow` only | `Warn`, `RequireConfirmation`, `Deny` | REST, MCP, Agent, CI |
| `approve_manual(surface, descriptor, override)` | `Allow`, `Warn`, `RequireConfirmation` (with matching override) | `Deny` | CLI, TUI |

Both methods verify that the caller-provided `surface` derives the same profile as the context was constructed with. Mismatches return `EnforcementError::SurfaceProfileMismatch`.

### ConfirmationClass Mapping

Under `ManualPermissive`, `RequireConfirmation` is produced for these operator-discretion cases:

| ConfirmationClass | Trigger | CLI Override Flag |
|-------------------|---------|-------------------|
| `OutOfScope` | Positive rules exist, target misses | `--allow-out-of-scope` or `--yes` |
| `TargetExpansion` | Discovered target outside original | `--allow-out-of-scope` or `--yes` |
| `ExplicitExclusion` | Target matches exclusion rule | `--allow-excluded-target` |
| `HighRisk` | Intrusive/LoadTest/StressTest/etc. | `--allow-high-risk` or `--allow-db-pentest` |
| `NonBaselineCapability` | Non-baseline capability required | `--allow-nonbaseline-capability` |
| `PrivateResolution` | Public input resolved to private | `--allow-private-resolution` |
| `CrossHostRedirect` | Cross-host redirect detected | `--allow-cross-host-redirect` |
| `TrafficInterception` | MITM proxy interception | `--allow-web-proxy` |

`--yes` (`assume_yes`) is **narrow**: it only covers `OutOfScope` and `TargetExpansion`. Dedicated `--allow-*` flags are required for all other classes. Automated profiles never honor overrides.

## Scope Model

### Scope vs LoadedScope

- **`Scope`** (`scope.rs:274`): The raw scope rules — `allowed_targets`, `excluded_targets`, `allowed_ports`, `excluded_ports`, `max_requests_per_second`, `require_explicit_scope`.

- **`LoadedScope`** (`scope.rs:217`): Wraps `Scope` with provenance metadata:
  - `scope: Scope` — the underlying rules (public field, accessed directly)
  - `source: ScopeSource` — where the scope came from (public field)
  - `path: Option<String>` — optional file path

- **`is_explicit_manifest()`** (`scope.rs:226`): Returns `true` if source is `ConfigFile`, `CliScopeFile`, or `GeneratedPreset`. Returns `false` for `DefaultEmpty`. This is the critical check for strict-profile enforcement.

### ScopeSource Variants

| Variant | Meaning | is_explicit |
|---------|---------|:---:|
| `DefaultEmpty` | No scope provided | No |
| `ConfigFile` | Loaded from `eggsec.toml` or `scope.toml` | Yes |
| `CliScopeFile` | Loaded from `--scope` CLI flag | Yes |
| `GeneratedPreset` | Generated from a preset | Yes |

### Address Classification

`classify_address(ip)` at `scope.rs:65` reports facts only; policy decides authorization.

| Class | IPv4 | IPv6 | is_non_public |
|-------|------|------|:---:|
| `Public` | 8.8.8.8 | 2001:4860:4860::8888 | No |
| `Loopback` | 127.0.0.0/8 | ::1 | Yes |
| `Private` | 10/8, 172.16/12, 192.168/16 | fc00::/7 | Yes |
| `LinkLocal` | 169.254/16 | fe80::/10 | Yes |
| `IPv4MappedLoopback` | — | ::ffff:127.0.0.1 | Yes |
| `Unspecified` | 0.0.0.0 | :: | Yes |
| `Multicast` | 224/4 | ff00::/8 | Yes |

`is_non_public()` at `scope.rs:50` returns `true` for all classes except `Public`. Loopback is exempted from scope blocking when no scope rules are defined (`scope.rs:411`).

### HostResolver Trait and SystemResolver

The `HostResolver` trait (`scope.rs:142`) decouples DNS resolution from policy:

```rust
pub trait HostResolver: Send + Sync {
    fn resolve_all(&self, host: &str) -> ResolutionResult;
}
```

- **`SystemResolver`** (`scope.rs:156`): Default implementation using `std::net::ToSocketAddrs`. Collects unique addresses, returns sorted for deterministic ordering. Does **not** reject any address classes — policy decides authorization.
- **`ResolutionResult`** (`scope.rs:117`): Contains `hostname`, `addresses: Vec<IpAddr>`, `error: Option<String>`.
- **`default_resolver()`** (`scope.rs:190`): Factory returning `Arc<dyn HostResolver>`.

### TargetScope Resolution

`TargetScope` (`scope.rs:677`) holds the parsed target with all resolved addresses:

```rust
pub struct TargetScope {
    pub host: String,
    pub ip: Option<IpAddr>,
    pub resolved_addresses: Vec<IpAddr>,
}
```

Key methods:
- `parse_with_resolver(target, resolver)` at `scope.rs:695`: Full DNS resolution. Literal IPs skip resolution; URLs extract host; bare hostnames resolve via the resolver.
- `parse_hostname_only_with_resolver(target, resolver)` at `scope.rs:788`: Hostname-only matching; resolution failures are non-fatal.
- `evaluate_addresses(allowed, excluded)` at `scope.rs:849`: Returns `(all_allowed, any_excluded, classes)`. Every resolved address must match at least one allowed rule; no address may match an exclusion.

### Scope Evaluation — `is_target_allowed()`

`Scope::is_target_allowed()` at `scope.rs:368` delegates to `is_target_allowed_with_resolver()`:

1. If scope has CIDR rules: use full `TargetScope::parse_with_resolver()` (requires DNS)
2. Otherwise: use `TargetScope::parse_hostname_only_with_resolver()` (DNS failures non-fatal)
3. Check exclusions first (exclusion always wins)
4. If no allowed targets: block non-public addresses (except loopback); allow public
5. If allowed targets exist: use `evaluate_addresses()` for all-address evaluation
6. Returns `Result<bool, ScopeError>`

### Scope Loading

- `load_config()` at `loader.rs:14`: Searches 5 locations in order: `--config` arg → `./eggsec.toml` → `./.eggsec/eggsec.toml` → `./config/eggsec.toml` → `~/.config/eggsec/eggsec.toml`
- `load_scope()` at `loader.rs:58`: Loads scope without provenance
- `load_scope_with_source()` at `loader.rs:102`: Loads scope with `ScopeSource` tracking. `--scope` → `CliScopeFile`; found file → `ConfigFile`; not found → `DefaultEmpty`
- Config format: TOML (primary), YAML (`.yaml`/`.yml`)
- Permissions: `check_config_file_permissions()` warns about world/group-readable files

## Integration Points

### Upstream Consumers

| Consumer | How it uses config |
|----------|-------------------|
| CLI (`eggsec-cli`) | `load_config()`, `load_scope_with_source()`, constructs `EnforcementContext::manual_permissive()` or `manual_guarded()` |
| TUI (`eggsec-tui`) | Reads `EggsecConfig` for settings tab; constructs `EnforcementContext` for operation dispatch |
| REST API | Forces `McpStrict` profile; `McpServer::with_enforcement()` receives `EnforcementContext` |
| MCP server | Forces `McpStrict` profile; `self.enforcement.evaluate()` called before every tool dispatch |
| Agent | Forces `AgentStrict` profile; handler defensively rebuilds `AgentStrict` enforcement |
| gRPC API | Forces `McpStrict` profile |
| `eggsec-runtime` | `runtime_bridge` converts `RuntimeSurface` → `ExecutionSurface` → `EnforcementContext` |
| Preflight | `preflight_operation()` at `policy_decision.rs:776` provides read-only policy preview |

### OperationMetadata → OperationDescriptor Flow

1. External surfaces (REST, MCP, TUI) look up metadata via `metadata_for_tool_id(tool_id)` at `policy.rs:2078`
2. Alias resolution: 42 aliases in `ALL_OPERATION_METADATA_ALIASES` at `policy.rs:2027` map alternative IDs to canonical operation IDs
3. Descriptor generation: `metadata.try_descriptor_for_target(target)` at `policy.rs:1283` (validated) or `metadata.descriptor_for_target(target)` at `policy.rs:1226` (unchecked)
4. Policy evaluation: `enforcement.evaluate(&descriptor)` or `enforcement.approve(surface, descriptor)`

### EnforcedDispatcher

REST, MCP, gRPC, and Agent surfaces use `EnforcedDispatcher` which requires an `ApprovedOperation` token before `dispatch_checked()`. This enforces type-level access control — strict programmatic surfaces cannot accidentally bypass policy.

## Testing

### Unit Tests by File

| File | Test Module | Count | Key Coverage |
|------|-------------|-------|--------------|
| `policy.rs` | `tests` | 15 | Risk ordering, profile display, capability display, serialization roundtrips |
| `policy.rs` | `operation_metadata_tests` | 13 | Metadata uniqueness, alias resolution, descriptor generation, feature-gated ops |
| `policy_decision.rs` | `tests` | 48 | Manual-mode invariants, downgrade logic, confirmation classes, approval tokens, preflight |
| `scope.rs` | `tests` | ~40 | Scope rules, CIDR matching, address classification, resolver tests, loaded scope provenance |
| `settings.rs` | `tests` | 1 | Config default path |
| `loader.rs` | `tests` | 12 | Config/scope loading, format support, source tracking |
| `budget.rs` | `tests` | 8 | Budget validation, defaults, serialization |
| `discovery.rs` | `tests` | 3 | Status display, scannability, serialization |
| `feature_registry.rs` | `tests` | 4 | Registry coverage, category matching |
| `presets.rs` | `tests` | 4 | Built-in count, find by name, serialization |

### Integration Tests

- `crates/eggsec/tests/enforcement_matrix.rs`: 105+ tests providing systematic cross-surface coverage for the dual-mode enforcement contract
- Tests cover: all execution surfaces, manual permissive/guarded/strict behavior, capability matrix, override isolation, scope state matrix

### Running Tests

```bash
cargo test -p eggsec --lib config::              # config module unit tests
cargo test --test enforcement_matrix -p eggsec    # enforcement matrix
```

## Invariants & Gotchas

### Invariants

1. **`EnforcementContext::evaluate()` is mandatory.** Every execution surface (CLI, TUI, REST, MCP, agent, gRPC) must pass through it. Never bypass it. (`policy_decision.rs:561`)

2. **`ApprovedOperation` is the only valid dispatch token.** Strict programmatic surfaces (REST, MCP, Agent, CI) require it before `dispatch_checked()`. (`policy_decision.rs:331`)

3. **Scope provenance matters.** Strict profiles (`CiStrict`, `McpStrict`, `AgentStrict`) require `LoadedScope::is_explicit_manifest() == true` for networked operations with `requires_explicit_scope`. `DefaultEmpty` blocks these. (`policy_decision.rs:569`)

4. **OperationMetadata is the single source of truth.** All surfaces use `metadata_for_tool_id()` to look up canonical operation definitions. Don't build policy checks inline. (`policy.rs:1494`)

5. **ManualOverride is CLI-only.** Never part of MCP/agent schemas or automated paths. Automated profiles never honor overrides. (`policy_decision.rs:432`)

6. **`--yes` is narrow.** Only covers `OutOfScope` and `TargetExpansion`. Dedicated `--allow-*` flags required for all other confirmation classes. (`policy_decision.rs:453`)

7. **Feature registry is fail-closed.** Unknown feature names return `false` from `is_feature_enabled()`. (`feature_registry.rs:141`)

8. **Resolver reports facts; policy decides.** `HostResolver` never rejects addresses. `classify_address()` never authorizes. (`scope.rs:62`)

### Gotchas

- **TOCTOU on scope checks.** Scope checks occur at dispatch time, not per-connection. `reqwest` may re-resolve DNS between scope check and connection. Strict surfaces should pin connections when possible.

- **`is_explicitly_excluded()` checks all resolved addresses.** A hostname that resolves to multiple IPs, where one IP matches an exclusion rule, will be excluded. (`scope.rs:500`)

- **Empty scope with positive rules + target miss = `RequireConfirmation` in ManualPermissive** (not a silent warn). This was a deliberate 2026-06-10 hardening decision. (`policy_decision.rs:1230`)

- **Strict profiles deny on warnings.** If `evaluate_operation_policy` produces warnings for a strict profile, the outcome is `Deny`. (`policy_decision.rs:1345`)

- **`classify_denial_reasons()` dual path.** When `denial_classes` is populated (new code path), returns typed classes directly. Falls back to string inspection for legacy. (`policy_decision.rs:1058`)

- **`ExecutionBudget::from_preset()` doesn't propagate `max_bytes`.** The `from_preset()` constructor at `budget.rs:90` sets `max_bytes: None` regardless of the preset's potential byte limits.

### Bug Sweep — Confirmed Findings

| Location | Finding | Severity |
|----------|---------|----------|
| `policy_decision.rs:727` | `expect("ExecutionPolicy is JSON-serializable")` in `policy_hash()` — will panic on serialization failure | Low (in practice, always serializable) |

*Last verified against source: 2026-08-25*
