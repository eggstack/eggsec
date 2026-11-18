# Security Assurance Hardening Milestone 002 — Python Buffer Protocol Lifetime

Status: closed

Repository baseline: `77188755` (`feat/auto-20261117-7451c367`, 2026-11-17)

Source roadmap:

- `plans/subsystems/security-assurance-hardening-roadmap.md` §7 Milestone 2
- Finding A2 in §4 of the same roadmap

Long-term requirements:

- `plans/000-long-term-specification.md` (Python programmability boundary)
- `plans/001-terminology-and-domain-model.md`
- `architecture/python_api.md`
- `docs/python/domain-maturity.md` (stability tier of the affected classes)

Applicable ADRs: none.

Primary class: invariant (memory safety)

## 1. Objective

Make every `Py_buffer` export from the Python bindings transfer a real owner
reference, so a consumer that trusts `Py_buffer.obj` can never observe a pointer
into a freed `BinaryBuffer` or `Artifact`.

## 2. Why this milestone is ready

- No dependencies; two self-contained `unsafe` blocks.
- The correct pattern is documented by the pinned pyo3 version: pyo3 `0.29.2`
  tests set `(*view).obj = slf.into_ptr()` (`pyo3-0.29.2/tests/test_buffer.rs:82`).
- Both classes are already exposed and tested
  (`crates/eggsec-python/tests/test_buffer.py`), so the regression harness exists.

## 3. Current implementation evidence

Both exporters null the owner and point `shape` into the Rust object:

```rust
// crates/eggsec-python/src/buffer_support.rs:84-91
(*view).obj = std::ptr::null_mut(); // PyO3 will set this
...
(*view).shape = std::ptr::addr_of!(slf.shape) as *mut _;
```

```rust
// crates/eggsec-python/src/artifact.rs:159-165
(*view).obj = std::ptr::null_mut();
...
(*view).shape = std::ptr::addr_of!(slf.shape) as *mut _;
```

The inline comment is false: pyo3 forwards the `__getbuffer__` slot and does not
synthesize `obj`. PEP 3118 defines `obj` as "a new reference to the exporting
object", owned by the consumer and released by `PyBuffer_Release`. Setting it to
NULL is legal only when the consumer is told to manage the lifetime itself —
which is exactly what every generic consumer assumes it does not have to do.

The `shape` field was moved into the struct specifically so it outlives a stack
frame (`crates/eggsec-python/src/artifact.rs:33-37` comment). That reasoning is
sound but incomplete: field lifetime is bounded by the *object's* lifetime, which
is only pinned by `obj`.

Impact nuance — report honestly in closure evidence:

- CPython's `memoryview` keeps its own `mv->base` reference, so existing
  `memoryview(buf)` tests pass today and do **not** catch this.
- The hazard is for consumers that rely on `view.obj` for keepalive (raw
  `Py_buffer` users, `numpy.frombuffer`, C extensions). For those, the exporter
  can be collected while `buf`/`shape` are still read.

Existing tests that must keep passing:
`crates/eggsec-python/tests/test_buffer.py::TestBinaryBuffer::test_memoryview`
and `::test_buffer_protocol`.

## 4. Invariants that must not regress

- `readonly = 1` stays: `data` is reachable only through a shared borrow, and
  advertising writability would let Python write through `&self`.
- `itemsize`, `format` (`b\0`), `ndim`, `strides` (NULL / contiguous) and
  `suboffsets` (NULL) keep their current values.
- `__releasebuffer__` stays a no-op: the buffer does not allocate separate
  memory, and CPython now owns the decref of `obj`.
- `Artifact` with no embedded content keeps returning `PyBufferError` and must
  not set `obj`.
- No `BinaryBuffer` API surface change; the class stays `provisional`.
- No global allocator, GC, or pyo3 version change.

## 5. Scope

### In scope

- `crates/eggsec-python/src/buffer_support.rs` `__getbuffer__`.
- `crates/eggsec-python/src/artifact.rs` `__getbuffer__`.
- Owner-reference regression tests.
- A short comment replacing the incorrect "PyO3 will set this" claim.

### Explicitly out of scope

- Redesigning `BinaryBuffer` into a zero-copy view over borrowed data.
- Adding `__getbuffer__` to other pyclasses.
- Changing the stability tier or the Python API surface snapshot.
- Vendoring or patching pyo3.
- Any `maturin` packaging change.

## 6. Required production changes

### Core/domain

In both exporters, replace the null assignment with the canonical ownership
transfer:

```rust
(*view).obj = slf.into_ptr();
```

Placement matters: set `obj` **after** all fallible work in the function, so an
error path never transfers ownership. In `artifact.rs` the `None` arm returns
`Err` before the pointer writes; keep that ordering. In `buffer_support.rs` the
null-`view` check must also precede the transfer.

`slf` is consumed by `into_ptr()`. If the remaining field reads (`shape`,
`readonly`) need `slf` afterwards, capture the needed values into locals
*before* the transfer — but note that `shape` must remain a pointer into the
object, so capture only the `isize` value into a local is **not** acceptable for
`shape`; the pointer must still target the object field. Recommended ordering:

1. compute `ptr`, `len`, and the `shape` pointer from `&slf`;
2. write all fields except `obj`;
3. `(*view).obj = slf.into_ptr();`

This keeps `shape` aimed at the live object while transferring the reference
last.

### Storage and migrations

None.

### Protocol and DTOs

None. The Python-visible `BinaryBuffer` constructor and methods are unchanged.

### Runtime and concurrency

Exports may happen from any thread; pyo3's `PyRef` guarantees the GIL. No new
synchronization.

### Frontend or operator surface

None beyond Python.

### Security and authorization

This is a memory-safety fix, not an authorization change. Do not add
scope/policy logic here.

### Documentation and static guards

- Replace the false inline comment with a one-line statement of the PEP 3118
  ownership rule.
- Update `architecture/python_api.md` only if it documents buffer internals.
- Consider a grep guard that fails on `obj = std::ptr::null_mut()` in
  `eggsec-python` outside tests.

## 7. Ordered work packages

### Work package A — `BinaryBuffer` export

Intent: correct ownership for the general binary buffer.

Required changes: field ordering as in §6, then `obj` transfer.

Acceptance evidence: `memoryview(buf).obj is buf`; `bytes(memoryview(buf))`
unchanged; `readonly` still true.

### Work package B — `Artifact` export

Intent: correct ownership for artifact content, preserving the `None` error path.

Required changes: same ordering; `obj` set only when `content` is `Some`.

Acceptance evidence: same assertions for an `Artifact` with content; an empty
artifact still raises `BufferError` and leaves no dangling export.

### Work package C — Regression tests

Intent: prove the contract in a way the existing tests cannot.

Required changes in `crates/eggsec-python/tests/test_buffer.py`:

- `test_buffer_exposes_owner_reference`: `mv = memoryview(buf); assert mv.obj is buf`.
- `test_buffer_survives_exporter_drop`: build the buffer inside a helper, take a
  memoryview, drop the buffer, `gc.collect()`, then assert the bytes still read
  correctly. This is a consumer-oriented guard, not the primary proof.
- Equivalent `test_artifact_buffer_exposes_owner_reference` using the fixture
  artifact helper already used by the suite (inspect the file before writing).
- If `memoryview.obj` proves ambiguous on the CI CPython, add a `ctypes` check
  that calls `PyObject_GetBuffer` and asserts `view.obj` is non-NULL, then
  releases it. Prefer the high-level assertion; only add the ctypes probe if it
  is genuinely needed, and record why.

Acceptance evidence: new tests fail on the pre-fix tree (verify by reverting the
two lines locally) and pass after.

## 8. Failure, cancellation, restart, and contention semantics

- Export failure (null view, missing content) returns `PyBufferError` with no
  reference transferred and no partially initialized view.
- No cancellation, restart, or contention surface: exports are synchronous under
  the GIL.
- Double release is CPython's responsibility once `obj` is set; the no-op
  `__releasebuffer__` remains correct.

## 9. Compatibility and migration

- No Python API change, no serialized format change, no stability-tier change.
- Refcount behavior changes by exactly one increment per live export, which is
  the intended contract. Watch for a class of bug where a consumer never calls
  `PyBuffer_Release`; that would now leak the object rather than dangling. This
  is the correct trade (leak is safe, UAF is not) and must be stated in the
  closure record.

## 10. Required tests

### Focused unit tests

Rust-side: none required (the export is exercised through Python).

### Integration tests

- `crates/eggsec-python/tests/test_buffer.py` full file.
- New owner-reference and drop tests.

### Restart and recovery tests

Not applicable.

### Contention and cancellation tests

Not applicable.

### Security and negative tests

- Empty artifact raises `BufferError`.
- `readonly` remains set (attempting a writable buffer request fails or returns
  a read-only view, per current behavior).
- Exporter collected while a memoryview is alive still yields correct bytes.

### Migration and compatibility tests

- `api_surface_snapshot.json` unchanged.
- `compatibility_allowlist.json` unchanged.

## 11. Required verification commands

```bash
make check
make check-python
```

`make check-python` builds into `.venv-ci/` (override:
`EGGSEC_PYTHON_VENV`). Record whether the new pytest file ran and its result.

Do not claim commands that were not actually run.

## 12. Documentation updates

- Inline comments in both exporters.
- `architecture/python_api.md` only if it makes a claim about buffer internals.
- No changelog entry unless the subsystem's convention requires one for a
  safety fix.

## 13. Acceptance criteria

1. Both exporters set `(*view).obj` to a new owned reference on success.
2. No error path transfers ownership.
3. `memoryview(obj).obj is obj` for `BinaryBuffer` and content-bearing
   `Artifact`.
4. `readonly`, `itemsize`, `format`, `ndim`, `strides`, `suboffsets` unchanged.
5. `make check` and `make check-python` pass.
6. The new tests demonstrably fail on the pre-fix code.

## 14. Stop conditions

Stop and report rather than improvise when:

- `PyRef::into_ptr` is unavailable or behaves differently on the pinned pyo3;
- the ordering constraint cannot be satisfied without leaking or double-freeing
  (e.g. a borrow-checker dead end that tempts `unsafe` pointer arithmetic);
- the Python test harness cannot build the extension in the environment;
- a fix would require changing the pyo3 version or the public Python API.

## 15. Closure evidence required

- Commit/PR and the exact two-line production diff (plus comments).
- Pre-fix failure output for the new tests.
- Post-fix `make check` and `make check-python` output.
- Confirmation that the API surface snapshot and compatibility allowlist are
  unchanged.
- The refcount-leak tradeoff stated explicitly.

## 16. Handoff notes

- Do not touch `Artifact::shape` initialization
  (`crates/eggsec-python/src/artifact.rs:66,81`); it is correct.
- `BinaryBufferPy::memoryview()` returns a `PyMemoryView` over a `PyBytes` copy,
  which is unrelated to `__getbuffer__`; leave it alone.
- The `provisional` tier at `crates/eggsec-python/src/lib.rs:1687` is unrelated
  to this fix.
- If a reviewer asks for a `ctypes` proof, keep it in the test file only; do not
  add FFI helpers to the crate.
