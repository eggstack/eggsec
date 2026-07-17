# Release 5 Phase F — Compatibility Enforcement and Release Hardening

Phase F strengthens release readiness through automated compatibility
enforcement, resource budget controls, comprehensive redaction coverage,
and domain graduation review.

## Overview

Phase F introduces four enforcement mechanisms that gate release
publication:

1. **Compatibility baseline and checker** — detects accidental API breaks
   by comparing the current build against a recorded baseline.
2. **Resource budget enforcement** — prevents unbounded growth of the
   stable-core surface through hard limits on modules, symbols, and wheel
   size.
3. **Comprehensive redaction testing** — verifies that `SensitiveString`
   values are redacted in all serialization, repr, and persistence paths.
4. **Domain graduation review** — structured template for evaluating
   provisional domains against stable-core promotion criteria.

## Enforcement Model

A `breaking` compatibility violation or resource budget failure blocks
release publication. Warnings and info-level violations are recorded in
the evidence bundle but do not block.

| Gate | Failure Mode | Blocking |
|------|-------------|----------|
| Compatibility checker | `breaking` severity violation | Yes |
| Resource budget | Any budget exceeded | Yes |
| Redaction test | Any path not redacted | Yes |
| Graduation review | Checklist incomplete | No (informational) |

## Key Documents

| Document | Purpose |
|----------|---------|
| [COMPATIBILITY_POLICY.md](COMPATIBILITY_POLICY.md) | Compatibility violation taxonomy and enforcement model |
| [GRADUATION_AUDIT.md](GRADUATION_AUDIT.md) | Domain graduation checklist and review template |
| [STABILITY_CLASSIFICATIONS.md](STABILITY_CLASSIFICATIONS.md) | Per-symbol stability mapping with maturity-aware severity rules |
| [domain-maturity.md](domain-maturity.md) | The twenty-two-operation stable-core boundary |

## Key Scripts

| Script | Purpose |
|--------|---------|
| `scripts/build_compatibility_baseline.py` | Generate compatibility baseline manifests |
| `scripts/compatibility_check.py` | Semantic compatibility checker |
| `scripts/build_python_release_evidence.py` | Aggregate all evidence into release bundle |

## Key Test Files

| Test File | Purpose |
|-----------|---------|
| `tests/test_resource_budgets.py` | Resource budget enforcement |
| `tests/test_redaction.py` | Redaction coverage verification |

## Running Phase F Checks

```bash
# Generate compatibility baseline
python scripts/build_compatibility_baseline.py --commit <sha> \
    --output validation/compatibility/baseline.json

# Check compatibility
python scripts/compatibility_check.py \
    --baseline validation/compatibility/baseline.json

# Run resource budget tests
pytest crates/eggsec-python/tests/test_resource_budgets.py

# Run redaction tests
pytest crates/eggsec-python/tests/test_redaction.py

# Full evidence bundle (includes Phase F artifacts)
python scripts/build_python_release_evidence.py --commit <sha>
```
