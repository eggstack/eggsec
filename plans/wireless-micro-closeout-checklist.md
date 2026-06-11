# Wireless Feature - Micro Close-Out Checklist

**Status**: Final Polish / Close-Out  
**Date**: 2026-06-11  
**Purpose**: Quick, focused checklist to finish the last minor items and mark wireless as complete in standalone form.

---

## Current State

- Core functionality is strong and stable.
- `docs/WIRELESS.md` is comprehensive.
- `CAPABILITIES.md` and `SAFETY.md` have been meaningfully updated.
- Remaining work is light documentation polish and consistency.

---

## Micro Close-Out Checklist

### 1. README.md Update (Quick Win)
- [ ] Add a short entry for `eggsec wireless <iface>` in the CLI command reference section.
- [ ] Optionally add one sentence in the "Lab / Defense Validation" or "Wireless" section of the README.

### 2. Rogue Output UX Review (Optional but Recommended)
- [ ] Review current default behavior in `run_cli()`:
  - Rogue candidates are summarized by default with a hint to use `--detect_suspicious`.
  - Decide if this is still the best UX or if a short list of candidate SSIDs would be clearer.
- [ ] Ensure the behavior is clearly documented in both `WIRELESS.md` and CLI help text.

### 3. Documentation Consistency Pass
- [ ] Do a quick scan of `docs/WIRELESS.md` for any outdated examples or missing flags.
- [ ] Verify that all CLI flags (`--repeat`, `--dry-run`, `--known-good`, `--detect_suspicious`) are mentioned consistently in:
  - `WIRELESS.md`
  - CLI help text (`WIRELESS_ABOUT`)
  - `CAPABILITIES.md`
- [ ] Check that error messages during repeated scans are clear and user-friendly.

### 4. Final Sanity Checks
- [ ] Run `cargo test --features wireless` (or equivalent) to ensure all wireless tests still pass.
- [ ] Build with `--features wireless` and verify `eggsec wireless --help` looks clean.
- [ ] Optionally run a quick manual test with `--dry-run --repeat 3 --json` to confirm output shape.

---

## Recommended Approach

This checklist is intentionally small. Most items can be completed in one short focused session:

1. Start with **README.md** (fastest visible improvement).
2. Do the **consistency pass** while reviewing the docs.
3. Spend a few minutes on **rogue UX** if it feels worth tweaking.
4. Finish with the **sanity checks**.

---

## Success Criteria

- README.md mentions the wireless command.
- Documentation is consistent across `WIRELESS.md`, CLI help, and `CAPABILITIES.md`.
- Rogue output behavior is intentional and well-documented.
- All tests pass and the command feels polished.

Once this checklist is complete, wireless can be considered finished in its current standalone form.

---

**This is the final micro close-out checklist. Keep it lightweight and pragmatic.**