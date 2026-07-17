# Compatibility Policy

This document defines the compatibility guarantees and breaking-change
policies for the `eggsec` Python package. It is the authoritative reference
for how stability classifications translate into real-world guarantees.

> This package is pre-1.0. The only stable execution boundary is the
> twenty-two-operation stable core described in
> [domain-maturity.md](domain-maturity.md). The guarantees below apply to
> the stable-core boundary only. Provisional and experimental APIs are
> explicitly excluded.

## Compatibility guarantees by maturity level

### Stable APIs

Symbols classified as `stable` in `api_surface()` carry the following
guarantees:

- **Semantic compatibility**: the operation semantics, typed result shape,
  and error contract will not change in incompatible ways within the current
  major version.
- **Deprecation window**: before any stable symbol is removed, it will be
  marked deprecated for at least one minor release. The deprecation message
  identifies the replacement and the planned removal version.
- **Type stub parity**: `.pyi` stubs will remain in sync with the compiled
  extension for all stable symbols.
- **Audit decision**: every stable-core dispatch continues to emit a
  structured audit decision through `EnforcementContext`.

Stable APIs include:
- The twenty-two operations and their typed request/result DTOs
- `Engine`, `AsyncEngine`, `Scope`, `Client`, `AsyncClient`
- Configuration types (`EggsecConfig` and its frozen children)
- Enforcement and preflight types
- Event protocol types
- Introspection functions (`api_surface`, `domain_maturity`, `features`)
- Version constants

### Provisional APIs

Symbols classified as `provisional` in `api_surface()` carry:

- **Best-effort compatibility**: we will avoid gratuitous breakage, but the
  API surface may change without a deprecation window if needed to advance
  toward graduation.
- **Migration notes**: when a provisional type undergoes a material change
  (renamed, moved, or structurally altered), a migration note will be added
  to [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md) in the same release.
- **Graduation path**: provisional types that complete the graduation
  checklist (see [domain-maturity.md](domain-maturity.md)) are promoted to
  stable with a deprecation window for the old path.

Provisional APIs include:
- Daemon client types (`eggsec.daemon`)
- Session types (`eggsec.sessions`)
- Network types (`eggsec.net`)
- Storage types (`eggsec.storage`)
- Reporting types (`eggsec.reporting`)

### Experimental APIs

Symbols classified as `experimental` in `api_surface()` carry:

- **No compatibility promise**: the API may change or be removed in any
  release without notice.
- **Change documentation**: material changes are noted in release notes but
  do not require migration guides.
- **Namespace isolation**: experimental types live exclusively under
  `eggsec.experimental` and must not be imported from the top level.

Experimental APIs include:
- Wireless, evasion, postex, C2, hunt, browser, AI, stress types
- Packet inspection (live capture, raw crafting)
- Any type accessed via `from eggsec.experimental import ...`

### Deprecated APIs

Deprecated symbols are retained until the declared removal floor:

- The removal floor is stated in the `DeprecationWarning` message emitted
  when the symbol is accessed.
- A deprecated symbol may be removed after the removal floor has shipped.
- Removal is announced in release notes with a migration path.

### Internal APIs

Internal symbols (e.g., `_core.BinaryBufferPy`, `_core.ArtifactMetaPy`):

- **Excluded from public inventories**: they do not appear in
  `api_surface()` with a stable or provisional classification.
- **No compatibility guarantee**: they may be renamed, restructured, or
  removed at any time.
- **Not exported at top level**: they are accessed only via `eggsec._core`
  or other private modules.

## Breaking change policy

A breaking change is any modification to a stable symbol that would cause
existing code to fail or behave differently without explicit action from the
user. This includes:

- Renaming or removing a stable symbol
- Changing a function signature (parameter types, return type, defaults)
- Changing the semantic behavior of an operation
- Changing the error kind for an existing failure mode
- Adding required fields to a previously optional request type
- Changing the serialization format of a typed result

### Requirements for breaking changes

Every breaking change to a stable symbol requires:

1. **Allowlist entry**: the change must be recorded in a breaking-changes
   allowlist with a rationale (e.g., "correctness fix", "security fix",
   "API clarity"). The allowlist is reviewed during release gating.
2. **Migration documentation**: a migration example must be added to
   [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md) showing the before/after
   pattern.
3. **Versioning decision**: a statement of whether the change warrants a
   major version bump or is absorbed under the pre-1.0 minor version
   contract.
4. **Removal timeline**: if the change removes a symbol, the deprecation
   window and removal floor must be stated.

### Emergency exceptions

Security fixes that require immediate breaking changes may bypass the
deprecation window, but still require:

- An allowlist entry with "security fix" rationale
- Migration documentation in the same release
- A note in the security advisory

## Stability expansion prevention

Maturity is not automatically conferred by importability or re-export.

### Top-level re-export does not imply stability

A type or function available at the top level of `eggsec` is not stable
unless `api_surface()` classifies it as such. The top-level namespace
includes backward-compatible re-exports of provisional types (e.g.,
`TargetPy`); these remain provisional even though they are importable from
the top level.

### Graduation checklist is mandatory

A symbol or domain is promoted from provisional to stable only when it
satisfies the graduation checklist in [domain-maturity.md](domain-maturity.md):

1. Canonical operation ID and request/result DTO
2. Sync and async dispatch through the common policy gate
3. Structured errors, events, cancellation, and serialization tests
4. Deterministic fixtures and contract coverage
5. Documentation, type stubs, and wheel-profile coverage

### Feature-gated stability

A feature-gated type is stable **only when the feature is compiled**. When
the feature is not compiled:

- Accessing the type raises `FeatureUnavailableError`
- The type is not present in `api_surface()` for that build
- Stability classification for feature-gated types is conditional on the
  build profile

This means `db_probe` is stable when built with `--features db-pentest` but
is not present (and not classified) in a default-wheel build.

## Machine-readable enforcement

The compatibility policy is enforced programmatically through two primary
instruments.

### `api_surface()` — per-symbol classification

`api_surface()` (defined in `src/lib.rs`) returns a dict mapping each
exported symbol to its stability metadata:

```python
import eggsec

surface = eggsec.api_surface()
print(surface["scan_ports"]["stability"])    # "stable"
print(surface["TargetPy"]["stability"])      # "provisional"
print(surface["wireless_scan"]["stability"]) # "experimental"
```

Each entry contains:

| Field | Type | Description |
|-------|------|-------------|
| `stability` | str | `"stable"`, `"provisional"`, `"experimental"`, or `"deprecated"` |
| `deprecated` | bool | Whether the symbol is deprecated |
| `feature_gate` | str or None | Required feature flag, if any |
| `module` | str | Canonical submodule path |

### `domain_maturity()` — per-domain classification

`domain_maturity()` returns a dict mapping domain names to their maturity
level and associated operation IDs:

```python
import eggsec

maturity = eggsec.domain_maturity()
print(maturity["stable-core"]["level"])      # "stable"
print(maturity["daemon"]["level"])           # "provisional"
print(maturity["wireless"]["level"])         # "experimental"
```

### Compatibility checker

The compatibility checker (run during release gating) applies
maturity-aware severity rules:

| Maturity | Unintended change severity | Required gate |
|----------|---------------------------|---------------|
| stable | **blocking** — release cannot proceed | Breaking-change allowlist + migration doc |
| provisional | **warning** — logged, not blocking | Migration note in release notes |
| experimental | **informational** — logged only | None |
| deprecated | **blocking if before removal floor** | Allowlist entry required |

The checker compares the current `api_surface()` snapshot against the
baseline snapshot tracked in `tests/api_surface_snapshot.json`. Any stable
symbol that is removed, renamed, or has its signature changed without an
allowlist entry causes a gating failure.

## Related documents

- [STABILITY_CLASSIFICATIONS.md](STABILITY_CLASSIFICATIONS.md) — per-symbol
  classification inventory
- [domain-maturity.md](domain-maturity.md) — domain maturity table and
  graduation checklist
- [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md) — migration examples for
  breaking and material changes
- `tests/api_surface_snapshot.json` — machine-readable baseline for
  compatibility checking
