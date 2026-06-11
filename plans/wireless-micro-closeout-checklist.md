# Wireless Feature - Micro Close-Out Record

**Status**: Complete
**Date**: 2026-06-11
**Purpose**: Historical record of the final documentation close-out for standalone-complete passive wireless.

---

## Completed In This Pass

- Updated `README.md` wireless quick reference to call out Linux `iwlist`, `wireless-tools`, root/CAP_NET_ADMIN, and the summary-by-default rogue UX.
- Updated `docs/WIRELESS.md` to align the examples, known-good behavior, and `--detect_suspicious` expansion with runtime behavior.
- Updated `docs/CAPABILITIES.md`, `AGENTS.md`, `architecture/wireless.md`, and the wireless skill so they describe the current standalone-complete passive state.
- Superseded the old remaining-work framing with this completed closeout record.

---

## Notes

- Wireless remains passive-only.
- Default human output summarizes rogue candidates; `--detect_suspicious` expands the full findings list.
- Real scans require `--features wireless`, Linux `iwlist` from `wireless-tools`, root or `CAP_NET_ADMIN`, and a managed/up wireless interface.
- No further wireless closeout documentation changes are required.
