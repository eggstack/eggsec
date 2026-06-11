---
name: tui_clipboard
description: "Clipboard support using arboard crate for copy/paste in TUI"
triggers:
  - clipboard
  - copy
  - paste
  - copy paste
metadata:
  category: TUI
  tools: [tui]
  scope: local
---

## Overview

Eggsec's TUI provides clipboard functionality using the `arboard` crate (pure Rust clipboard access). This enables copy/paste operations in the terminal UI.

## Usage

```rust
use eggsec::tui::utils::clipboard::Clipboard;

// Create clipboard instance
let clipboard = Clipboard::new();

// Check if clipboard is available
if clipboard.is_available() {
    // Get clipboard contents
    if let Ok(content) = clipboard.get() {
        println!("Clipboard: {}", content);
    }

    // Set clipboard contents
    if let Ok(()) = clipboard.set("Hello, World!") {
        println!("Content copied to clipboard");
    }

    // Clear clipboard
    let _ = clipboard.clear();
}
```

## Implementation

- `crates/eggsec-tui/src/utils/clipboard.rs` - Clipboard utility
- Depends on `arboard = "3.4"` crate

## Key Methods

- `Clipboard::new()` - Creates new clipboard instance
- `clipboard.is_available()` - Check if system clipboard is accessible
- `clipboard.get()` - Read content from clipboard (returns `Result<String, Error>`)
- `clipboard.set(content)` - Write content to clipboard (returns `Result<(), Error>`)
- `clipboard.clear()` - Clear clipboard contents

## Error Handling

```rust
match clipboard.get() {
    Ok(content) => println!("{}", content),
    Err(e) => eprintln!("Clipboard read failed: {:?}", e),
}
```

## Integration Points

- Use in input fields for paste support
- Use in output displays for copy support
- Use in selection components for copy selection

## Verification

```bash
cargo test --lib -p eggsec -- clipboard
```