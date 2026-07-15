# Checkpoints and Resume

This guide covers checkpoint persistence for pipeline resumption.

## Checkpoint Store

```python
from eggsec import CheckpointStore, PipelineCheckpoint

# Create a file-backed checkpoint store
store = CheckpointStore("/tmp/checkpoints")

# Create a checkpoint
checkpoint = PipelineCheckpoint(
    pipeline_id="pipe-1",
    step_results={"recon": '{"status":"complete"}'},
    artifact_refs=["art-1"],
)

# Save
store.save(checkpoint)

# Load
loaded = store.load("pipe-1")
if loaded:
    print(f"Pipeline: {loaded.pipeline_id}")
    print(f"Steps: {loaded.step_results}")
```

## Pipeline Checkpoint Fields

- `pipeline_id`: Unique identifier for the pipeline
- `step_results`: Map of step name to result JSON
- `artifact_refs`: List of artifact IDs produced
- `schema_version`: Checkpoint schema version for compatibility
- `created_at_ms`: Timestamp of checkpoint creation

## Resume After Restart

```python
store = CheckpointStore("/tmp/checkpoints")
checkpoint = store.load("pipe-1")

if checkpoint:
    # Resume from where we left off
    completed_steps = set(checkpoint.step_results.keys())
    remaining = [s for s in all_steps if s not in completed_steps]
    for step in remaining:
        # Execute step and update checkpoint
        result = execute_step(step)
        checkpoint.step_results[step] = result
        store.save(checkpoint)
```

## Atomic Writes

Checkpoints are written atomically to prevent corruption during process restarts.
