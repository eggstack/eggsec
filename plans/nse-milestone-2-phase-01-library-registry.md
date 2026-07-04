# NSE Milestone 2 Phase 01: Declarative Library Registry

## Purpose

Create a declarative registry for NSE library modules so compatibility, side effects, sandbox posture, and known gaps are machine-readable instead of scattered across code comments and docs.

This phase does not require rewriting every library implementation. It establishes the registry model, migrates high-value libraries first, and creates guardrails so new libraries cannot be added without metadata.

## Background

The `eggsec-nse` crate now has a large NSE library surface. Milestone 1 closed loader/profile policy, but library capability state is still difficult to inspect. Some libraries are close to compatibility, some are partial shims, some perform direct network/filesystem/process actions, and some exist mainly to satisfy common `require()` patterns.

A production-grade compatibility layer needs to tell the truth about this state.

## Non-Goals

Do not add new NSE libraries in this phase.

Do not perform a broad behavior rewrite of all library modules.

Do not implement capability wrappers for every side-effecting helper; that belongs to Milestone 3.

Do not remove partial libraries simply because they are partial. Mark them accurately.

## Target State

By the end of this phase:

- `eggsec-nse` has a central `NseLibraryRegistry` or equivalent.
- Each registered library can expose metadata including status, capabilities, side effects, and known gaps.
- Library registration in `ExecutorCore::register_libraries()` can be checked against registry metadata.
- Structured reports in later phases can consume the registry.
- Architecture guards prevent unregistered library files from silently appearing.

## Proposed Data Model

Add a registry module, likely:

```text
crates/eggsec-nse/src/library_registry.rs
```

Suggested core types:

```rust
pub struct NseLibraryDescriptor {
    pub name: &'static str,
    pub module_path: &'static str,
    pub status: NseLibraryStatus,
    pub compatibility: NseCompatibilityLevel,
    pub side_effects: &'static [NseSideEffect],
    pub requires_features: &'static [&'static str],
    pub sandbox_posture: NseSandboxPosture,
    pub deterministic: bool,
    pub known_gaps: &'static [&'static str],
    pub notes: &'static str,
}

pub enum NseLibraryStatus {
    Implemented,
    Partial,
    Shim,
    Stub,
    Unsupported,
    Experimental,
}

pub enum NseCompatibilityLevel {
    Exact,
    Practical,
    Approximate,
    Partial,
    StubOnly,
    Unknown,
}

pub enum NseSideEffect {
    None,
    NetworkTcp,
    NetworkUdp,
    FilesystemRead,
    FilesystemWrite,
    ProcessExec,
    Time,
    Randomness,
    DnsResolution,
    Crypto,
    Compression,
}

pub enum NseSandboxPosture {
    Pure,
    SandboxChecked,
    ProfileChecked,
    ManualOnly,
    NeedsMilestone3Wrapper,
    Unknown,
}
```

Keep the model small enough to maintain. Add fields later only if needed by reports or guards.

## Workstream 1: Build the Registry Skeleton

### Steps

1. Add `library_registry.rs` with descriptor types.
2. Export the registry types from `lib.rs` when `nse` is enabled.
3. Add `pub fn all_library_descriptors() -> &'static [NseLibraryDescriptor]`.
4. Add `pub fn library_descriptor(name: &str) -> Option<&'static NseLibraryDescriptor>`.
5. Add `pub fn known_library_names() -> impl Iterator<Item = &'static str>` or an equivalent static slice.
6. Add unit tests for lookup, uniqueness, and stable ordering.

### Acceptance Criteria

- Registry compiles under `--features nse`.
- Library names are unique.
- Lookup by name works for representative libraries.
- Registry APIs do not instantiate Lua or perform side effects.

## Workstream 2: Seed High-Value Metadata

### Initial Libraries

Start with the most common and most policy-relevant modules:

- `stdnse`
- `nmap`
- `shortport`
- `http`
- `comm`
- `socket`
- `sslcert`
- `tls`
- `dns`
- `ssh`
- `smb`
- `vulns`
- `json`
- `bin`
- `bit`
- `strbuf`
- `tableaux`
- `lfs`
- `io`
- `os`

For each, record:

- compatibility level,
- whether it is pure or side-effecting,
- whether it should be considered safe for automated profiles,
- whether it needs Milestone 3 capability-wrapper work,
- obvious known gaps.

### Acceptance Criteria

- The seeded set covers common `require()` paths and high-risk side effects.
- Unknowns are marked `Unknown` rather than guessed.
- Known partial behavior is documented in `known_gaps`.

## Workstream 3: Connect Registration to Metadata

### Problem

`ExecutorCore::register_libraries()` currently registers many libraries directly. Without a cross-check, metadata can drift from actual registration.

### Steps

1. Add a lightweight test that compares registered library names against registry entries.
2. If direct introspection of registered Lua globals is awkward, create a static `REGISTERED_LIBRARY_NAMES` slice near `register_libraries()` and compare it to the registry.
3. Ensure every direct `register_*_library()` call has a corresponding descriptor.
4. Do not require every descriptor to be `Implemented`; partial/stub descriptors are acceptable.

### Acceptance Criteria

- Adding a new library registration without metadata fails a test or guard.
- Removing a library registration while leaving stale metadata fails a test or is clearly detected.

## Workstream 4: Guard New Library Files

### Steps

1. Update `scripts/check-architecture-guards.sh` with an NSE library metadata check.
2. The guard should detect `.rs` files under `crates/eggsec-nse/src/libraries/` that have no descriptor entry.
3. Allow non-module support files explicitly if they exist.
4. Keep the guard readable and documented inline.

### Acceptance Criteria

- Architecture guard fails on an unregistered new library file.
- Guard output tells the developer where to add metadata.

## Workstream 5: Documentation

### Steps

1. Add a `Library Registry` section to `architecture/nse_integration.md`.
2. Document that registry metadata is the source of truth for compatibility status.
3. Link from `.opencode/skills/eggsec-nse/SKILL.md` to the registry section.
4. Add a short note that Milestone 3 will upgrade `NeedsMilestone3Wrapper` libraries with capability wrappers.

### Acceptance Criteria

- Docs identify the registry as canonical.
- Docs do not claim exact compatibility unless metadata says exact.

## Tests

Required tests:

- registry has unique names;
- seeded libraries are present;
- every registered library has metadata;
- every metadata entry refers to a known or intentionally future library;
- side-effecting seeded libraries are not marked `Pure`;
- manual-only libraries are not marked automated-safe.

## Verification

Run:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
bash scripts/check-architecture-guards.sh
```

## Final Acceptance Criteria

Phase 01 is complete when:

- A central library registry exists.
- High-value libraries have truthful descriptors.
- Registration and metadata cannot silently drift.
- Architecture guards catch new library files without metadata.
- Docs point future work to the registry rather than ad hoc claims.
