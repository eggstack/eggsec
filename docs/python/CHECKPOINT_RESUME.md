# Checkpoint and Resume

Pipelines can save execution state after each successful step and resume
from the last checkpoint when restarted. This enables long-running
assessments to survive interruptions.

## CheckpointStore Configuration

### In-Memory Store

Checkpoints exist only for the lifetime of the process:

```python
from eggsec import CheckpointStore

store = CheckpointStore()  # No persistence
```

### File-Backed Store

Checkpoints are persisted to disk as JSON and reloaded on creation:

```python
from eggsec import CheckpointStore

store = CheckpointStore.with_persistence("/tmp/eggsec-checkpoints.json")
```

If the file exists, checkpoints are loaded from it on creation. Future saves
are written through to disk atomically.

### Factory Function

```python
from eggsec import create_checkpoint_store

# In-memory
store = create_checkpoint_store()

# File-backed
store = create_checkpoint_store("/path/to/checkpoints.json")
```

### Store API

```python
store = create_checkpoint_store("/tmp/checkpoints.json")

# Save a checkpoint
store.save(checkpoint)

# Load a checkpoint
result = store.load("pipeline-id")
if result:
    cp = result.checkpoint
    print(f"Migrated: {result.migrated}")
    print(f"Original version: {result.original_version}")

# Delete a checkpoint
deleted = store.delete("pipeline-id")

# Find next step to resume
result = store.resume_from("pipeline-id", ["step1", "step2", "step3"])
if result:
    load_result, next_step = result
    print(f"Next step: {next_step}")  # "step2"

# List stored pipelines
ids = store.list_pipeline_ids()

# Check size
print(store.len())      # Number of stored checkpoints
print(store.is_empty())  # True if no checkpoints
```

## Attaching to a Pipeline

```python
from eggsec import Pipeline, create_checkpoint_store, OperationRequest

store = create_checkpoint_store("/tmp/pipeline-state.json")

pipeline = Pipeline("long-scan")
pipeline.set_checkpoint_store(store)
pipeline.add_step("recon", OperationRequest("recon", "example.com"))
pipeline.add_step("tls", OperationRequest("tls-inspect", "example.com"),
                  dependencies=["recon"])

# Each successful step is automatically saved to the store
result = pipeline.run(engine)
```

## Checkpoint Schema

### PipelineCheckpoint Fields

| Field | Type | Description |
|-------|------|-------------|
| `version` | `int` | Schema version (currently 3) |
| `pipeline_id` | `str` | Deterministic pipeline identifier |
| `pipeline_name` | `str` | Human-readable pipeline name |
| `completed_steps` | `list[str]` | Names of successfully completed steps |
| `current_step` | `str?` | Step in progress when checkpoint was created |
| `step_results` | `dict` | Serialized results keyed by step name |
| `created_at_ms` | `int` | Epoch milliseconds when created |
| `updated_at_ms` | `int` | Epoch milliseconds when last updated |
| `operation_schema_version` | `str` | Version of the operation request/result contract |
| `target_set_hash` | `str` | Hash of the target set used by the pipeline |
| `scope_hash` | `str` | Hash of the scope used by the pipeline |
| `execution_profile` | `str` | Enforcement profile name |
| `enabled_features_hash` | `str` | Hash of the compiled feature set |
| `pipeline_definition_hash` | `str` | Hash of the complete pipeline definition |
| `artifact_store_id` | `str?` | Optional external artifact store identity |

### Version Compatibility

| Version | Changes |
|---------|---------|
| 1 | Initial schema |
| 2 | Added `updated_at_ms` (missing in v1) |
| 3 | Added compatibility identity fields (`operation_schema_version`, `target_set_hash`, etc.) |

Checkpoints from older versions are automatically migrated forward on load.
Migration is non-destructive and preserves all existing data.

## What Triggers Checkpoint Creation

A checkpoint is created after **each successful step completion**. Failed or
skipped steps do not produce checkpoints. The checkpoint captures:

1. All previously completed step results
2. The current step's result
3. Updated timestamps
4. Current compatibility identity hashes

The checkpoint is saved to the `CheckpointStore` (in-memory or file-backed)
immediately after each step.

## What Invalidates a Checkpoint

When resuming, the pipeline validates the checkpoint against the current
execution context. A checkpoint is **invalidated** (resume rejected) if any
of these do not match:

| Check | What Changes |
|-------|-------------|
| `operation_schema_version` | Request/result contract changed between versions |
| `target_set_hash` | Target hosts/paths changed |
| `scope_hash` | Authorization scope changed |
| `execution_profile` | Enforcement profile changed |
| `enabled_features_hash` | Compiled feature set changed |
| `pipeline_definition_hash` | Step names, requests, dependencies, or ordering changed |
| `artifact_store_id` | External artifact store identity changed |

If any check fails, the pipeline raises `checkpoint_incompatible` and does
not resume.

## Resume Flow

### Automatic Resume via Store

When a checkpoint store is attached, `Pipeline.run()` automatically checks
for an existing checkpoint and resumes from the last completed step:

```python
# First run — completes steps 1 and 2, then crashes
pipeline.run(engine)

# Second run — detects checkpoint, skips steps 1 and 2, runs from step 3
pipeline.run(engine)
```

### Manual Resume

```python
from eggsec import Pipeline, Checkpoint

# Load checkpoint from store
result = store.load("pipeline-id")
if result:
    checkpoint = result.checkpoint

    # Create pipeline and resume
    pipeline = Pipeline("my-scan")
    pipeline.add_step("step1", OperationRequest("recon", "example.com"))
    pipeline.add_step("step2", OperationRequest("tls-inspect", "example.com"))

    result = pipeline.resume_from(engine, checkpoint)
```

### Checkpoint Cleanup

On successful pipeline completion, the checkpoint is automatically deleted
from the store:

```python
# After pipeline.run() succeeds:
# - Checkpoint is removed from the store
# - No stale state remains
```

## Secret Redaction

Sensitive fields in step results are automatically redacted before
checkpoint persistence. The redaction logic identifies keys containing:

- `secret`, `password`, `token`
- `api_key`, `apikey`, `authorization`
- `credential`, `client_secret`, `access_key`

Redacted values are replaced with `[REDACTED]` in the serialized checkpoint.

### SensitiveString Handling

Operations like `db_probe` use `SensitiveString` for credentials. The
`SensitiveString` type:

- Never appears in `repr()`, events, or reports
- Is redacted in checkpoint serialization
- Requires explicit `expose_secret()` to access (manual-only operation)

```python
from eggsec import DbProbeRequest

# Password is wrapped in SensitiveString internally
request = DbProbeRequest(
    "10.0.0.1",
    port=5432,
    username="admin",
    password="hunter2",
)

# repr shows redacted password
print(repr(request))
# DbProbeRequest(target=10.0.0.1, password=[REDACTED])

# JSON serialization redacts password
print(request.to_json())
# {"target":"10.0.0.1","port":5432,...,"password":"[REDACTED]",...}
```

## Atomic Write Guarantees

File-backed checkpoint stores use atomic writes:

1. Write to a temporary file (`{path}.tmp-{pid}`)
2. Call `sync_all()` to flush to disk
3. Rename the temporary file to the final path
4. If any step fails, the temporary file is cleaned up

This ensures the checkpoint file is never in a partial/corrupted state.

## Complete Example

```python
from eggsec import (
    Pipeline, Engine, Scope, OperationRequest,
    create_checkpoint_store,
)

scope = Scope.allow_hosts(["10.0.0.1"])
engine = Engine(scope)

# Create file-backed store
store = create_checkpoint_store("/tmp/assessment-checkpoints.json")

# Build pipeline
pipeline = Pipeline("assessment", max_concurrency=2)
pipeline.set_checkpoint_store(store)

pipeline.add_step("recon", OperationRequest("recon", "example.com"))
pipeline.add_step("tls", OperationRequest("tls-inspect", "example.com"))
pipeline.add_step("tech", OperationRequest("tech-detect", "example.com"),
                  parallel_group="passive")
pipeline.add_step("fingerprint", OperationRequest("fingerprint-services", "10.0.0.1"),
                  dependencies=["recon"])

# Run — checkpoints saved after each successful step
result = pipeline.run(engine)

if result.is_success():
    print("Assessment complete")
    # Checkpoint automatically deleted
else:
    print(f"Pipeline failed — checkpoint saved for resume")
    print(f"Completed steps: {[s.step_name for s in result.step_results if s.is_success()]}")
```
