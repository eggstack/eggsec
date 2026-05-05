# TUI (Terminal User Interface)

Slapper includes a powerful real-time Terminal User Interface (TUI) built with the `ratatui` crate. It provides an interactive way to monitor and control ongoing scans.

## Core Components (`src/tui/`)

### App & UI (`app/`, `ui.rs`)

Manages the overall application state and the rendering of the TUI layout.

- **Tabs (`tabs/`)**: Organizes information into different views (e.g., Overview, Scanner, Fuzzer, Logs, Findings).
- **Components (`components/`)**: Reusable UI elements like progress bars, tables, charts, and modal dialogs.
- **Theme (`theme.rs`)**: Customizable color schemes and styles for the TUI.

### State Management (`state/`)

A reactive state management system that ensures the UI is updated immediately as new data arrives from the background scanning tasks.

### Workers (`workers/`)

Background tasks that bridge the gap between the core scanning engine and the TUI. They collect metrics and findings and push them into the TUI's state.

### Interactive Features

- **Search (`search.rs`)**: Filter and search through findings and logs in real-time.
- **Session Management (`session.rs`)**: Save and load TUI sessions, allowing you to resume work across different terminal sessions.
- **Help (`help.rs`)**: Interactive help system and keybinding reference.

## Usage

The TUI can be launched by adding the `--tui` flag to many Slapper commands or by running the `tui` subcommand.

```bash
slapper scan https://target.com --tui
```
