# Test Matrix and Pre-Handoff Checklist

Every extension must pass a targeted set of tests before merging. This
document maps extension types to the required test commands and describes
the final pre-handoff gate.

## Test-to-Extension Mapping

| Extension type | Required tests |
|----------------|----------------|
| Operation metadata | `metadata_consistency`, `feature_matrix` |
| Domain descriptor | `metadata_consistency`, `tool_registration`, `feature_matrix` |
| Command | `command_registry`, `enforcement_matrix` when side-effecting |
| Tool exposure | `tool_registration`, `enforced_dispatch_regression` |
| TUI action | `eggsec-tui --lib`, TUI action spec tests |
| Report output | `eggsec-output --test report_envelope` |
| Feature | `feature_matrix`, representative `cargo check --features ...` |

## Required Local Commands

Run every command below from the workspace root. All must pass before
opening a pull request.

```bash
cargo fmt --all --check

cargo check --workspace --no-default-features

cargo test -p eggsec --lib

cargo test -p eggsec --test metadata_consistency

cargo test -p eggsec --test command_registry

cargo test -p eggsec --test tool_registration --features rest-api

cargo test -p eggsec --test feature_matrix

cargo test -p eggsec --test enforcement_matrix

cargo test -p eggsec --test enforced_dispatch_regression

cargo test -p eggsec-output --test report_envelope

bash scripts/check-architecture-guards.sh

make check-feature-profiles

make check-architecture-ci
```

### What each command verifies

- **`cargo fmt --all --check`** -- formatting. Must be clean.
- **`cargo check --workspace --no-default-features`** -- compiles with no
  features enabled. Catches unconditional imports of feature-gated items.
- **`cargo test -p eggsec --lib`** -- unit tests across the main crate.
- **`cargo test -p eggsec --test metadata_consistency`** -- cross-references
  `DomainDescriptor` and `OperationMetadata` for consistency.
- **`cargo test -p eggsec --test command_registry`** -- validates the static
  command registry: duplicate IDs, missing metadata, stale entries.
- **`cargo test -p eggsec --test tool_registration --features rest-api`**
  -- validates tool registration metadata for MCP, REST, gRPC, and agent
  surfaces.
- **`cargo test -p eggsec --test feature_matrix`** -- checks that feature
  strings in `OperationMetadata` and `DomainDescriptor` match actual Cargo
  features and that `KNOWN_EGGSEC_FEATURES` is current.
- **`cargo test -p eggsec --test enforcement_matrix`** -- verifies that
  side-effecting commands have correct enforcement expectations.
- **`cargo test -p eggsec --test enforced_dispatch_regression`** -- ensures
  `EnforcedDispatcher` cannot be bypassed.
- **`cargo test -p eggsec-output --test report_envelope`** -- validates
  `ReportEnvelope` serialization and the `to_report_envelope()` bridge.
- **`bash scripts/check-architecture-guards.sh`** -- static grep checks
  for stale terminology, MCP exposure splits, raw dispatch prevention, and
  docs currency. Requires ripgrep.
- **`make check-feature-profiles`** -- runs `cargo check` for
  representative feature profiles. Catches missing deps or broken cfg
  paths in common build configurations.
- **`make check-architecture-ci`** -- reproduces the full architecture
  guard CI job locally. This is the final gate.

## Feature-Profile Checks

Feature-profile checks verify that every representative build
configuration compiles. The canonical profiles tested by CI are:

- `--no-default-features`
- `--features rest-api`
- `--features db-pentest`
- `--features mobile`
- `--features web-proxy`
- `--features wireless`
- `--features nse`
- `--features evasion,postex`
- `--features full`

If your extension adds a new feature, add a representative profile check
that includes it. The `make check-feature-profiles` target runs the full
set.

## Platform-Sensitive and Deep Checks

Some features require platform-specific system dependencies or elevated
privileges and are not tested in the default CI matrix. Before merging
changes to these features, run the checks locally on a compatible
platform.

| Feature | System dependency | Local command |
|---------|-------------------|---------------|
| `mobile-dynamic` | ADB + Android device | `cargo check -p eggsec --features mobile-dynamic` |
| `wireless` | iwlist, CAP_NET_ADMIN | `cargo check -p eggsec --features wireless` |
| `wireless-advanced` | wireless + root | `cargo check -p eggsec --features wireless-advanced` |
| `packet-inspection` | libpcap-dev | `cargo check -p eggsec --features packet-inspection` |
| `db-pentest` | database drivers | `cargo test -p eggsec-db-lab` |
| `nse` | libssl-dev | `cargo test -p eggsec-nse` |

These checks are excluded from the default `make check-feature-profiles`
target because they fail on platforms without the required system
libraries. Document any platform-sensitive test steps in the PR
description.

## Pre-Handoff Checklist

Before opening a pull request, confirm every item.

- [ ] `cargo fmt --all --check` passes
- [ ] `cargo check --workspace --no-default-features` passes
- [ ] `cargo test -p eggsec --lib` passes
- [ ] Extension-specific tests from the table above pass
- [ ] `bash scripts/check-architecture-guards.sh` passes
- [ ] `make check-feature-profiles` passes
- [ ] `make check-architecture-ci` passes
- [ ] If the extension adds a feature: representative `cargo check --features <new-feature>` passes
- [ ] If the extension touches platform-sensitive code: local check on a compatible platform passes
- [ ] No new clippy warnings introduced (`cargo clippy --lib -p eggsec`)
- [ ] AGENTS.md updated if the extension adds commands, features, or changes the capability matrix
