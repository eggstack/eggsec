# Security Assurance Hardening Milestone 002 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/security-assurance-hardening/002-python-buffer-protocol-lifetime.md`

Source subsystem roadmap:

- `plans/subsystems/security-assurance-hardening-roadmap.md` §7 Milestone 2

Repository baseline reviewed: `77188755a41bee15aee9507055ac6b2761ff9271` (implementation remains uncommitted in the worktree)

Implementation commits or pull requests:

- None. No commit or PR was created, per instruction; evidence below describes the current working tree.

## 1. Executive finding

Both Python buffer exporters now transfer a real owner reference through `Py_buffer.obj`. The pointer-backed shape remains a field of the exporting object and is therefore kept alive for the exporter's lifetime. The Python API and serialized formats are unchanged.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| `BinaryBuffer` export owns its exporter | `crates/eggsec-python/src/buffer_support.rs`; `test_buffer_exposes_owner_reference`, `test_buffer_survives_exporter_drop` | pass | `memoryview.obj` is the exporter; view remains readable after exporter-local reference is dropped. |
| Content-bearing `Artifact` export owns its exporter | `crates/eggsec-python/src/artifact.rs`; Artifact owner-reference and exporter-drop tests in `tests/test_buffer.py` | pass | Missing content still errors before ownership transfer. |
| Buffer layout/read-only contract unchanged | `tests/test_buffer.py` layout, bytes, and readonly checks | pass | No writable view or ABI change. |
| Full Python binding contract remains consistent | `make check-python` | pass | 4,437 passed, 1,728 skipped, 23 deselected, 16 xfailed; capability matrix, architecture guards, stub parity, and type checks passed. |

## 3. Production implementation evidence

`buffer_support.rs` and `artifact.rs` set `(*view).obj = slf.into_ptr()` only after fallible checks and pointer/layout preparation. The existing shape field continues to back `view.shape`; `__releasebuffer__` remains a no-op because CPython releases the owned exporter reference.

## 4. Verification executed

### Commands run

```bash
make check-python
make check
```

### Results

Both commands exited 0. The Python suite result was 4,437 passed, 1,728 skipped, 23 deselected, and 16 xfailed. Six non-fatal pytest/runtime warnings were reported; capability matrix, Python architecture guards, stubs, importability, and type checks passed. The full Python suite includes the buffer tests; no separate one-file pytest invocation was run.

## 5. Invariant review

- `readonly = 1`, format, item size, dimensionality, strides, and no-suboffset layout remain unchanged.
- Export failure occurs before ownership transfer; an artifact without content does not publish `obj`.
- The transfer adds one owning reference per successful export; CPython's normal release path balances it.
- This memory-safety fix does not alter authorization or policy code.

## 6. Failure and recovery review

Null-view and missing-content paths remain errors and do not transfer the reference. No asynchronous work, persistence, cancellation, restart, or shared mutable state was added. The existing full Python checks passed.

## 7. Migration and compatibility review

No public Python constructor/method, serialized representation, or stability tier changed. The additional reference count is the expected PEP 3118 ownership cost.

## 8. Security review

Consumers can rely on `Py_buffer.obj` to keep the Rust exporter alive; the prior unmanaged-pointer lifetime hazard is removed. Read-only semantics remain enforced.

## 9. Documentation and operations

- Exporter comments explain the PEP 3118 owner-reference rule.
- Python API context remains in `architecture/python_api.md`.
- `make check-python` builds in the configured CI virtual environment and runs the Python guards and type checks.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| — | No unresolved milestone finding identified by executed checks. | — | — |

## 11. Roadmap disposition

Milestone closed; no migration or follow-on dependency is blocked.

## 12. Registry updates

- Record this milestone as closed with this closure reference in the security-assurance-hardening roadmap and `plans/registry.md`.
