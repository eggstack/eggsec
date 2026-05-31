# Diff Architecture Review
**Document:** architecture/diff.md
**Reviewed:** 2026-05-31
**Accuracy:** High
**Lines Reviewed:** 23

## Verified Claims
- `DiffResult` struct: Verified at `crates/slapper/src/diff/mod.rs:9` with fields `new`, `resolved`, `persisting`, `changed`, `summary`
- `FindingChange` struct: Verified at `crates/slapper/src/diff/mod.rs:18` with fields `fingerprint`, `title`, `old_severity`, `new_severity`, `old_confidence`, `new_confidence`, `evidence_changed`
- `DiffSummary` struct: Verified at `crates/slapper/src/diff/mod.rs:29` with fields `new_count`, `resolved_count`, `persisting_count`, `changed_count`, `old_total`, `new_total`
- `diff_findings()` function: Verified at `crates/slapper/src/diff/mod.rs:39` - fingerprint-based comparison using `HashMap<&str, &Finding>`
- `diff_findings_from_files()` function: Documented but actual implementation is `load_findings_from_file()` (`crates/slapper/src/diff/mod.rs:103`) which loads findings from a JSON file (no file-to-file diff function exists)
- File-based comparison: Documented as `diff_findings_from_files()`. Actual: `load_findings_from_file()` loads a single file; there is no function that diffs two files directly. The user must load both files and call `diff_findings()` manually
- Only one file (`mod.rs`): Verified

## Discrepancies
- **`diff_findings_from_files()` function name**: Documented as `diff_findings_from_files()`. Actual: No such function exists. The closest is `load_findings_from_file()` (`crates/slapper/src/diff/mod.rs:103`) which loads findings from a single file. A file-to-file diff function was never implemented.
- **`format_diff_text()` undocumented**: The codebase contains `format_diff_text()` (`crates/slapper/src/diff/mod.rs:110`) which formats a `DiffResult` as human-readable text. This is not mentioned in the architecture document.

## Bugs Found
- None

## Improvement Opportunities
- Implement the documented `diff_findings_from_files()` function that loads two files and diffs them, or update the document to accurately describe `load_findings_from_file()` + manual `diff_findings()` usage
- Document `format_diff_text()` in the architecture

## Stale Items
- `diff_findings_from_files()` claim is stale - the function does not exist
