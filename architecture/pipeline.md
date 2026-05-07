# Pipeline Module

The Pipeline module allows for the orchestration of complex security assessment workflows by chaining multiple Slapper tasks together.

## Core Concepts (`src/pipeline/`)

### Stage (`stage.rs`)

A `Stage` represents a single discrete task in the pipeline, such as a port scan, a tech detection run, or a targeted fuzzer execution.

- **Selection**: Stages are selected from a profile (for example `quick`, `web`, `full`) or from an explicit comma-separated list.
- **Aliases**: User-facing aliases such as `portscan`, `fp`, `endpoint-scan`, `graphql`, `oauth`, and `jwt` are normalized into canonical stages.

### Executor (`executor.rs`)

The `executor.rs` file is responsible for running the pipeline from start to finish.

- **Sequential Execution**: Stages run in linear order (`for stage in &self.stages`).
- **Result Passing**: Output from one stage (for example open ports and detected HTTP services) is persisted into `PipelineContext` and consumed by later stages.
- **Failure Recording**: Stage errors are recorded per stage in `StageResult` and surfaced in the report. CLI entrypoints return `ScanFailed` if any stage failed.

### Pipeline Context (`context.rs`)

Maintains the state of a running pipeline, including intermediate results, shared variables, and the overall status.

### Session (`session.rs`)

Manages persistence for resumable pipeline runs via JSON snapshots (`PipelineSession`).
Session checkpoints are written only when output path is explicitly a session-like file name (`*.session` or `*.session.json`) to avoid colliding with report outputs.

## Benefits

- **Automation**: Automate standard pentesting methodologies.
- **Repeatability**: Ensure that complex scans are performed consistently every time.
- **Efficiency**: Reduce manual intervention by automatically triggering the next logical step in a scan.
