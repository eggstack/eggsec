# Macros

## Overview

Public macros defined in `crates/slapper/src/macros.rs` for reducing code duplication.

## Exported Macros

### `run_if_enabled!`

Conditional async task execution with stage tracking:
```rust
run_if_enabled!(condition, "stage_name", stage, async_task)
```
Executes `task` only if `condition` is true, setting the recon stage first.

### `stage_task!`

Named stage task with skip support:
```rust
stage_task!("name", skip_condition, stage, body)
```
Returns `None` if skipped, `Some(result)` otherwise.

### `recon_stage!`

Recon-specific stage wrapper:
```rust
recon_stage!(skip, "stage_name", stage, { body })
```
Locks the stage mutex, executes body, returns `Default` on error.

### `print_if_some!`

Conditional println for optional values:
```rust
print_if_some!("Label", optional_value)
```

### `option_as_result!`

Convert `Option` to `Result` with error message:
```rust
option_as_result!(option_expr, "error message")
```

## Helper Functions

- `format_optional_field()` - append labeled optional field to string
- `format_list_field()` - append labeled list field to string
