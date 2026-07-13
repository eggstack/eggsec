# eggsec-python release checklist

This checklist governs a scoped pre-1.0 stable-core release. It must not be
read as a claim that every importable domain is stable.

## Semantic gates

- [x] Stable operation IDs and dispatch are backed by `StableOperation`.
- [x] Stable engine results carry versioned `OperationError` payloads.
- [x] `raise_for_status()` reconstructs the documented exception hierarchy.
- [x] Sync and async engine state use the same policy gate and audit model.
- [x] Event envelopes carry monotonic sequence numbers.
- [x] Backpressure statistics account for drops and preserve reliable events.
- [x] `domain_maturity()` exposes the provisional/experimental boundary.
- [x] Every stable operation has a deterministic non-skipping integration fixture.
- [x] Daemon execution is explicitly provisional and excluded from the first-release contract.
- [x] Pipeline checkpoint/resume equivalence is demonstrated for a stable-core operation, including typed restoration.
- [x] Secret sentinel coverage includes checkpoint persistence and structured result serialization.

## Verification gates

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --lib -p eggsec`
- [x] `cargo check -p eggsec-python`
- [x] `cargo check -p eggsec-python --features full-no-system`
- [x] `pytest crates/eggsec-python/tests/ crates/eggsec-python/python/tests/`
- [x] Export/stub parity checker passes against the rebuilt extension.
- [x] Repository-wide architecture guards pass.
- [x] Release wheel profiles build and install in clean virtual environments.
- [x] Stable-core fixture smoke test passes from the installed wheel on Linux x86_64.
- [ ] Linux, macOS arm64, and the declared experimental Windows profile have current CI evidence.

## Publication gates

- [ ] TestPyPI dry run and clean-environment installation succeed (manual CI gate).
- [ ] Changelog, migration notes, security policy, and vulnerability route are current.
- [ ] Release is cut from a commit with all required CI checks passing.
- [ ] PyPI publication is manually approved after all prior gates pass.
