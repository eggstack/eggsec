# CLI & Commands

The CLI and Commands layer is responsible for parsing user input, managing global state (CommandContext), and dispatching execution to the appropriate handlers.

## CLI Parsing (`src/cli/`)

Slapper uses `clap` for command-line argument parsing. The CLI is organized into several modules, each defining the arguments for a specific category of commands:

- **`mod.rs`**: Defines the main `Cli` entry point and common arguments.
- **`scan.rs`**: Arguments for the `scan` command (port scanning, endpoint discovery).
- **`fuzz.rs`**: Arguments for the `fuzz` command (security fuzzing).
- **`http.rs`**: Arguments for HTTP-specific operations.
- **`packet.rs` & `stress.rs`**: Arguments for low-level networking and stress testing.
- **`agent.rs` & `ai_analyze.rs`**: Arguments for AI-driven features.

## Command Dispatch (`src/commands/`)

Once arguments are parsed, the `main` function initializes a `CommandContext` and calls `handle_command` in `src/commands/mod.rs`.

- **`CommandContext`**: Carries global state including the loaded `SlapperConfig`, `Scope`, and output preferences.
- **`handle_command`**: A large match statement that dispatches to the correct handler based on the subcommand.

## Handlers (`src/commands/handlers/`)

Actual command execution logic resides in the `handlers` directory. Each handler is typically an `async` function that takes the parsed arguments and the `CommandContext`.

Examples:
- **`scan.rs`**: Entry point for port scanning and reconnaissance.
- **`fuzz.rs`**: Entry point for the security fuzzing engine.
- **`cluster.rs`**: Manages distributed scanning nodes.
- **`plugin.rs`**: Handles execution of external Python/Ruby plugins.

## Workflow

1. `main.rs` parses arguments using `Cli::parse()`.
2. Logging is initialized.
3. Configuration and Scope are loaded.
4. `CommandContext` is created.
5. `handle_command` dispatches to a specific handler in `src/commands/handlers/`.
6. The handler executes the requested operation, often interacting with other core modules like `scanner` or `fuzzer`.
