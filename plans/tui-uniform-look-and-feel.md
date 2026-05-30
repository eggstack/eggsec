# TUI Uniform Look & Feel Plan

## Goal

Achieve consistent visual appearance across all 28+ TUI tabs by standardizing:
- Results border colors
- Input block wrapping
- Empty state handling
- Error rendering patterns
- Popup text styling
- Notification color consistency
- Scrollbar theming

## Audit Summary

### Current State (All tabs use tc!() - no hardcoded colors)

| Dimension | Pattern A | Pattern B | Pattern C |
|-----------|-----------|-----------|-----------|
| **Results border** | `tc!(success)` (14 tabs) | `None`/`tc!(border)` (10 tabs) | `tc!(info)` (2 tabs) |
| **Input wrapping** | Bordered block (11 tabs) | No block (10 tabs) | FormBuilder (2 tabs) |
| **Empty state** | `empty_state_paragraph` (20 tabs) | None (8 tabs) | Custom (1 tab) |
| **Error rendering** | Early return (8 tabs) | Inline in results (10 tabs) | None (4 tabs) |
| **Popup content text** | Styled (all others) | **Unstyled** (popup.rs only) | - |
| **Warning notification** | `tc!(status_running)` (ui.rs) | `tc!(warning)` (notifications.rs) | - |

### Standards Decision

| Element | Standard | Rationale |
|---------|----------|-----------|
| Results border | `None` (neutral `tc!(border)`) | Results should not imply success/failure at the border level |
| Input wrapping | Bordered block with focus-aware borders | Visual containment matches scan/fuzz/waf/load pattern |
| Empty state | `empty_state_paragraph()` | Already used by 20 tabs; consistent placeholder |
| Error rendering | Early return with bordered error block | Most visible; matches stress/packet/proxy/nse/compliance/report/history/settings |
| Popup text | `tc!(text)` via `.style()` | Matches all other Paragraphs in the codebase |
| Warning notification | `tc!(warning)` | Semantically correct; status_running is for running indicator |
| Scrollbar | Theme-aware via `thumb_style` | Currently uses ratatui defaults |

---

## Priority 1: Critical Fixes

### P1.1 - Popup Content Text Styling
**File:** `tui/components/popup.rs:130`
**Change:** Add `.style(Style::default().fg(tc!(text)))` to the content Paragraph
**Impact:** All popups (help, confirm, info, warning, error) will render body text in theme text color

### P1.2 - Notification Warning Color
**File:** `tui/ui.rs:579`
**Change:** `tc!(status_running)` -> `tc!(warning)` for `NotificationSeverity::Warning`
**Impact:** Warning notifications will be yellow (consistent with the popup notification)

### P1.3 - Results Border Standardization
**Standard:** All tabs pass `None` to `ScrollableText::render()` (falls back to neutral `tc!(border)`)
**Tabs to change** (passing `Some(tc!(success))` currently):
- `recon.rs` - change `Some(tc!(success))` -> `None`
- `packet.rs` - change `Some(tc!(success))` -> `None`
- `load.rs` - change `Some(tc!(success))` -> `None`
- `proxy.rs` - change `Some(tc!(success))` -> `None`
- `hunt.rs` - change `Some(tc!(success))` -> `None`
- `browser.rs` - change `Some(tc!(success))` -> `None`
- `compliance.rs` - change `Some(tc!(success))` -> `None`
- `storage.rs` - change `Some(tc!(success))` -> `None`
- `integrations.rs` - change `Some(tc!(success))` -> `None`
- `workflow.rs` - change `Some(tc!(success))` -> `None`
- `vuln.rs` - change `Some(tc!(success))` -> `None`
**Tabs to change** (passing `Some(tc!(info))` currently):
- `fuzz.rs` - change `Some(tc!(info))` -> `None`
- `resume.rs` - change `Some(tc!(info))` -> `None`
**Tabs already correct** (passing `None`): scan, waf, stress, graphql, oauth, cluster, nse, report, history, dashboard

---

## Priority 2: Input Block Standardization

### P2.1 - Add Bordered Input Blocks
**Standard:** All tabs with input fields wrap them in `Block::default().borders(Borders::ALL).title("...")` with focus-aware `border_style`.
**Pattern to follow:** `scan.rs` or `fuzz.rs` config area rendering

Tabs needing bordered input blocks:
1. `recon.rs` - wrap inputs in block titled " Configuration "
2. `packet.rs` - wrap inputs in block titled " Configuration "
3. `proxy.rs` - wrap inputs in block titled " Configuration "
4. `hunt.rs` - wrap inputs in block titled " Configuration "
5. `browser.rs` - wrap inputs in block titled " Configuration "
6. `compliance.rs` - wrap inputs in block titled " Configuration "
7. `storage.rs` - wrap inputs in block titled " Configuration "
8. `integrations.rs` - wrap inputs in block titled " Configuration "
9. `workflow.rs` - wrap inputs in block titled " Configuration "
10. `vuln.rs` - wrap inputs in block titled " Configuration "

---

## Priority 3: Empty State Standardization

### P3.1 - Add Empty State Paragraphs
**Standard:** All tabs with results areas use `empty_state_paragraph()` when results are empty.
**Pattern:** `use crate::tui::components::empty_state_paragraph;`

Tabs needing empty state paragraphs:
1. `stress.rs` - add empty state in results area
2. `graphql.rs` - add empty state in results area
3. `oauth.rs` - add empty state in results area
4. `cluster.rs` - add empty state in results area
5. `nse.rs` - add empty state in results area
6. `report.rs` - add empty state in results area

---

## Priority 4: Error Rendering Standardization

### P4.1 - Standardize Error Rendering
**Standard:** Early return pattern with full-area bordered error block.
**Pattern:** Match stress/packet/proxy/nse/compliance/report/history/settings

Tabs needing error rendering fixes:
1. `graphql.rs` - add early return error block
2. `oauth.rs` - add early return error block
3. `cluster.rs` - add early return error block

Tabs that already use inline error in results (keep as-is - these show inputs while error is displayed):
- scan, fuzz, waf, load, hunt, browser, storage, integrations, workflow, vuln, resume

---

## Priority 5: Polish

### P5.1 - Scrollbar Theme Styling
**File:** `tui/components/scrollable.rs`
**Change:** Add `.thumb_style(Style::default().fg(tc!(accent)))` and `.track_style(Style::default().fg(tc!(border)))` to the Scrollbar widget
**Impact:** Scrollbars will match the current theme

### P5.2 - Consistent Modifier::BOLD Usage
**Standard:** Only use `Modifier::BOLD` on mode indicators (status bar) and popup active buttons. Remove from section headers in results content.
**Files:** `dashboard.rs`, `history.rs` - remove `Modifier::BOLD` from section header styling in set_results/render_stats

---

## Execution Order

1. **P1.1** popup.rs (1 file, 1 line)
2. **P1.2** ui.rs (1 file, 1 line)
3. **P1.3** 13 tab files (13 lines)
4. **P2.1** 10 tab files (~100 lines - requires layout changes)
5. **P3.1** 6 tab files (~30 lines)
6. **P4.1** 3 tab files (~30 lines)
7. **P5.1** scrollable.rs (2 lines)
8. **P5.2** dashboard.rs, history.rs (4-6 lines)
9. **Verification** - cargo check, cargo test, cargo clippy

## Verification Commands

```bash
cargo check --lib -p slapper
cargo test --lib -p slapper
cargo clippy --lib -p slapper
```
