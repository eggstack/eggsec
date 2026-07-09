# Release Checklist

Use this checklist before any PyPI publication.

**Validation results:** See `VALIDATION.md` for command output and platform details.

## Pre-Release Gates

### Rust validation (all pass as of 2026-07-09)

- [x] `cargo check -p eggsec-python` passes
- [x] `cargo test -p eggsec-python` passes
- [x] `cargo check -p eggsec-python --features full-no-system` passes
- [x] Selected optional feature checks pass (11 features, all pass — see VALIDATION.md)

### Python build and smoke (all pass as of 2026-07-09)

- [x] `maturin develop` succeeds
- [x] `pytest` default suite passes (0 failures, 9 network tests skipped via marker)
- [x] `maturin build --release` succeeds
- [x] Clean venv wheel install succeeds
- [x] `import eggsec` smoke test passes from installed wheel
- [x] Scanner smoke test passes from installed wheel
- [x] Report serialization smoke test passes from installed wheel
- [x] `__all__` consistency check passes (all 94 names resolve)

### Documentation and tooling (as of 2026-07-09)

- [x] Stub consistency check passes (scope enforced on free functions, instance-bound on Client/AsyncClient)
- [x] Export checker script exists (`scripts/check_eggsec_python_exports.py`)
- [x] Documentation reviewed and accurate
- [x] VALIDATION.md records results

### CI and publish (as of 2026-07-09)

- [x] CI workflow exists (`.github/workflows/python-wheels.yml`)
- [x] Workflow YAML validates
- [x] Publish job gated by `workflow_dispatch` (no accidental push/PR publish)

### Not yet PyPI-ready

- [ ] TestPyPI dry run succeeds (requires manual workflow dispatch)
- [ ] Install from TestPyPI succeeds (requires manual workflow dispatch)
- [ ] Final PyPI publish (only after ALL above gates pass)

## Post-Release

- [ ] PyPI publish (only after ALL above gates pass)
- [ ] Verify `pip install eggsec` works
- [ ] Update documentation status from "experimental" if appropriate
