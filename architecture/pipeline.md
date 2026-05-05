# Pipeline Module

The Pipeline module allows for the orchestration of complex security assessment workflows by chaining multiple Slapper tasks together.

## Core Concepts (`src/pipeline/`)

### Stage (`stage.rs`)

A `Stage` represents a single discrete task in the pipeline, such as a port scan, a tech detection run, or a targeted fuzzer execution.

- **Configuration**: Each stage can have its own specific configuration that overrides the global settings.
- **Dependencies**: Stages can be configured to run only if previous stages succeed or find specific results.

### Executor (`executor.rs`)

The `executor.rs` file is responsible for running the pipeline from start to finish.

- **Sequential & Parallel Execution**: Supports running stages in a linear sequence or in parallel where dependencies allow.
- **Result Passing**: Output from one stage (e.g., discovered open ports) can be fed as input to a subsequent stage (e.g., fuzzing those ports).

### Pipeline Context (`context.rs`)

Maintains the state of a running pipeline, including intermediate results, shared variables, and the overall status.

### Session (`session.rs`)

Manages the persistence of pipeline runs, allowing them to be paused, resumed, or re-run with different parameters.

## Benefits

- **Automation**: Automate standard pentesting methodologies.
- **Repeatability**: Ensure that complex scans are performed consistently every time.
- **Efficiency**: Reduce manual intervention by automatically triggering the next logical step in a scan.
