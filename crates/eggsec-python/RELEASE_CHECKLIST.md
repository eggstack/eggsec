# Release Checklist

Use this checklist before any PyPI publication.

## Pre-Release Gates

- [ ] `cargo check -p eggsec-python` passes
- [ ] `cargo test -p eggsec-python` passes
- [ ] `cargo check -p eggsec-python --features full-no-system` passes
- [ ] Selected optional feature checks pass (see feature matrix)
- [ ] `maturin develop` succeeds
- [ ] `pytest` default suite passes (no `|| true`)
- [ ] `maturin build --release` succeeds
- [ ] Clean venv wheel install succeeds
- [ ] `import eggsec` smoke test passes from installed wheel
- [ ] Scanner smoke test passes from installed wheel
- [ ] Report serialization smoke test passes from installed wheel
- [ ] Stub consistency check passes
- [ ] Documentation reviewed and accurate
- [ ] TestPyPI dry run succeeds
- [ ] Install from TestPyPI succeeds
- [ ] CI workflow passes (correct platform wheel selection)

## Post-Release

- [ ] PyPI publish (only after ALL above gates pass)
- [ ] Verify `pip install eggsec` works
- [ ] Update documentation status from "experimental" if appropriate
