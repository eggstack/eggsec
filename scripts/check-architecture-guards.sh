#!/usr/bin/env bash
# Architecture drift guards - static checks for invariant regressions
# Part of CI architecture guards (Phase 11)
set -euo pipefail

if ! command -v rg >/dev/null 2>&1; then
  echo "FAIL: ripgrep (rg) is required for architecture guard checks." >&2
  echo "Install ripgrep locally or add it to the CI image before running this script." >&2
  exit 1
fi

FAIL=0

echo "=== Architecture Drift Guards ==="

# 1. No stale "manual_only" in command registry docs/tests (excluding plans/, TUI action specs, and this script's own docs)
echo ""
echo "--- Check 1: No stale 'manual_only' in command registry ---"
HITS=$(rg -n 'manual_only' --glob='*.md' docs/ 2>/dev/null | grep -v 'plans/' | grep -v 'action_spec' | grep -v 'CI_ARCHITECTURE_GUARDS' | grep -v 'docs/extending/' || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: Found stale 'manual_only' in docs. Use 'cli_interactive_only' instead."
  FAIL=$((FAIL + 1))
else
  echo "PASS: No stale 'manual_only' in command registry docs."
fi

# 2. No ambiguous "interactive_only" (should be "cli_interactive_only")
echo ""
echo "--- Check 2: No ambiguous 'interactive_only' ---"
HITS=$(rg -n '\binteractive_only\b' --glob='*.rs' --glob='*.md' crates/ docs/ scripts/ 2>/dev/null | grep -v 'plans/' | grep -v 'cli_interactive_only' || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: Found ambiguous 'interactive_only'. Use 'cli_interactive_only' instead."
  FAIL=$((FAIL + 1))
else
  echo "PASS: No ambiguous 'interactive_only' found."
fi

# 3. MCP exposure terminology stays split
echo ""
echo "--- Check 3: MCP exposure terminology split ---"
HITS=$(rg -n 'mcp_metadata_exposable' --glob='*.rs' crates/eggsec/src/tool/registration.rs 2>/dev/null || true)
if [[ -z "$HITS" ]]; then
  echo "FAIL: 'mcp_metadata_exposable' not found in tool/registration.rs"
  FAIL=$((FAIL + 1))
else
  echo "PASS: 'mcp_metadata_exposable' present in tool/registration.rs."
fi

HITS=$(rg -n 'mcp_default_visible' --glob='*.rs' crates/eggsec/src/tool/registration.rs 2>/dev/null || true)
if [[ -z "$HITS" ]]; then
  echo "FAIL: 'mcp_default_visible' not found in tool/registration.rs"
  FAIL=$((FAIL + 1))
else
  echo "PASS: 'mcp_default_visible' present in tool/registration.rs."
fi

# 4. OpsAgent is not equated with conservative default in code
# (Test files and docs that discuss the relationship are expected to mention both terms;
#  the tool_registration tests enforce the actual invariant.)
echo ""
echo "--- Check 4: OpsAgent not equated with conservative default in source ---"
HITS=$(rg -n 'OpsAgent.*conservative.*default|conservative.*default.*OpsAgent' --glob='*.rs' crates/ 2>/dev/null | grep -v 'plans/' | grep -v 'tests/' | grep -v 'not ' || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: Found source code equating OpsAgent with conservative default."
  FAIL=$((FAIL + 1))
else
  echo "PASS: No source code equates OpsAgent with conservative default."
fi

# 5. Raw dispatch not used by strict surfaces (grep for direct calls)
echo ""
echo "--- Check 5: Raw dispatch not in strict surfaces ---"
# Check gRPC for raw .dispatch( calls (excluding known internals)
HITS=$(rg -n '\.dispatch\(' --glob='*.rs' crates/eggsec/src/commands/grpc.rs 2>/dev/null | grep -v 'EnforcedDispatcher' | grep -v 'dispatch_checked' | grep -v 'orchestrator' | grep -v 'test' || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: Found raw .dispatch() calls in gRPC surface."
  FAIL=$((FAIL + 1))
else
  echo "PASS: No raw dispatch in gRPC surface."
fi

# Agent self.dispatch() is internal implementation (trait method), not a bypass
# The enforced_dispatch_regression test validates the actual invariant

# 6. Plan retention - key phase files exist
echo ""
echo "--- Check 6: Plan retention ---"
SECTION_FAIL=0
REQUIRED_PLANS=(
  "plans/architecture-extensibility-roadmap.md"
  "plans/architecture-extensibility-phase-06-command-registry.md"
  "plans/architecture-extensibility-phase-07-tool-mcp-registration.md"
  "plans/architecture-extensibility-phase-08-tui-tightening.md"
  "plans/architecture-extensibility-phase-09-report-evidence-unification.md"
  "plans/architecture-extensibility-phase-10-feature-matrix-build-profiles.md"
  "plans/architecture-extensibility-phase-11-ci-architecture-guards.md"
)
for plan in "${REQUIRED_PLANS[@]}"; do
  if [[ ! -f "$plan" ]]; then
    echo "FAIL: Missing required plan file: $plan"
    FAIL=$((FAIL + 1))
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: All required plan files exist."
fi

# 7. Documentation currency - required docs exist
echo ""
echo "--- Check 7: Required documentation exists ---"
SECTION_FAIL=0
REQUIRED_DOCS=(
  "docs/COMMAND_REGISTRY.md"
  "docs/TOOL_REGISTRATION.md"
  "docs/FEATURE_MATRIX.md"
  "docs/METADATA_OWNERSHIP.md"
  "docs/CI_ARCHITECTURE_GUARDS.md"
)
for doc in "${REQUIRED_DOCS[@]}"; do
  if [[ ! -f "$doc" ]]; then
    echo "FAIL: Missing required doc: $doc"
    FAIL=$((FAIL + 1))
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: All required docs exist."
fi

# 7b. Extensibility handoff guides exist
echo ""
echo "--- Check 7b: Extensibility handoff guides exist ---"
SECTION_FAIL=0
REQUIRED_EXT_DOCS=(
  "docs/EXTENSIBILITY.md"
  "docs/extending/operations.md"
  "docs/extending/domains.md"
  "docs/extending/commands.md"
  "docs/extending/tool-exposure.md"
  "docs/extending/tui-actions.md"
  "docs/extending/report-evidence.md"
  "docs/extending/features.md"
  "docs/extending/testing.md"
  "docs/extending/templates.md"
)
for doc in "${REQUIRED_EXT_DOCS[@]}"; do
  if [[ ! -f "$doc" ]]; then
    echo "FAIL: Missing extensibility doc: $doc"
    FAIL=$((FAIL + 1))
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: All extensibility handoff guides exist."
fi

# 8. Extensibility guide links in EXTENSIBILITY.md resolve to existing files
echo ""
echo "--- Check 8: Extensibility guide links resolve ---"
SECTION_FAIL=0
EXPECTED_EXT_LINKS=(
  "docs/extending/operations.md"
  "docs/extending/domains.md"
  "docs/extending/commands.md"
  "docs/extending/tool-exposure.md"
  "docs/extending/tui-actions.md"
  "docs/extending/report-evidence.md"
  "docs/extending/features.md"
  "docs/extending/testing.md"
  "docs/extending/templates.md"
)
for link in "${EXPECTED_EXT_LINKS[@]}"; do
  if ! rg -F "$link" docs/EXTENSIBILITY.md >/dev/null 2>&1; then
    echo "FAIL: docs/EXTENSIBILITY.md missing link: $link"
    FAIL=$((FAIL + 1))
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  if [[ ! -f "$link" ]]; then
    echo "FAIL: linked extensibility doc missing: $link"
    FAIL=$((FAIL + 1))
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: All extensibility guide links in EXTENSIBILITY.md resolve."
fi

# 9. No stale field names in current docs
echo ""
echo "--- Check 9: No stale field names in current docs ---"
HITS=$(rg -n 'mcp_listing_does_not_check|mcp_exposed_by_default.*false.*hardcoded' --glob='*.md' docs/ 2>/dev/null | grep -v 'plans/' || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: Found stale field names/phrases in current docs."
  FAIL=$((FAIL + 1))
else
  echo "PASS: No stale field names in current docs."
fi

# 10. TUI workers directory must not exist
echo ""
echo "--- Check 10: TUI workers directory absent ---"
if [[ -d "crates/eggsec-tui/src/workers" ]]; then
  echo "FAIL: crates/eggsec-tui/src/workers/ still exists. Dispatch moved to eggsec::dispatch."
  FAIL=$((FAIL + 1))
else
  echo "PASS: TUI workers directory is absent."
fi

# 11. eggsec-runtime must not depend on TUI crates
echo ""
echo "--- Check 11: Runtime free of TUI dependencies ---"
HITS=$(rg -n 'ratatui|crossterm|eggsec.tui' crates/eggsec-runtime/Cargo.toml 2>/dev/null || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: eggsec-runtime has TUI dependencies."
  FAIL=$((FAIL + 1))
else
  echo "PASS: Runtime free of TUI dependencies."
fi

# 12. eggsec-runtime must not depend on daemon transport crates
echo ""
echo "--- Check 12: Runtime free of transport dependencies ---"
HITS=$(rg -n 'axum|tonic|tokio.tungstenite|tower-http' crates/eggsec-runtime/Cargo.toml 2>/dev/null || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: eggsec-runtime has daemon transport dependencies."
  FAIL=$((FAIL + 1))
else
  echo "PASS: Runtime free of transport dependencies."
fi

# 13. Runtime capabilities must not advertise unimplemented transports
echo ""
echo "--- Check 13: No unimplemented transports in capabilities ---"
HITS=$(rg -n '"stdio"|"unix-socket"|"websocket"' crates/eggsec-runtime/src/capabilities.rs 2>/dev/null || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: Capabilities advertise unimplemented transports. Use 'in-process'."
  FAIL=$((FAIL + 1))
else
  echo "PASS: No unimplemented transports in capabilities."
fi

# 14. eggsec-tui must not define a canonical TaskConfig or TaskResult enum
echo ""
echo "--- Check 14: TUI has no canonical TaskConfig/TaskResult enums ---"
HITS=$(rg -n 'pub enum TaskConfig|pub enum TaskResult' crates/eggsec-tui/src/ 2>/dev/null || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: TUI defines canonical TaskConfig/TaskResult enums. Use eggsec::dispatch::TaskResult."
  FAIL=$((FAIL + 1))
else
  echo "PASS: No canonical TaskConfig/TaskResult in TUI."
fi

echo ""
echo "=== Summary ==="
if [[ $FAIL -gt 0 ]]; then
  echo "FAILED: $FAIL check(s) failed."
  exit 1
else
  echo "ALL PASSED: No architecture drift detected."
  exit 0
fi
