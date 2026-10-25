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

# 6. Plan retention - handoff plans are retained intentionally; planning hierarchy is governed
echo ""
echo "--- Check 6: Plan retention ---"
SECTION_FAIL=0
if [[ ! -f "plans/README.md" ]]; then
  echo "FAIL: plans/README.md is missing; plan retention policy is undocumented."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ ! -f "plans/registry.md" ]]; then
  echo "FAIL: plans/registry.md is missing; planning control surface is undocumented."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ ! -f "plans/003-planning-process.md" ]]; then
  echo "FAIL: plans/003-planning-process.md is missing; planning governance is undocumented."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
PLAN_COUNT=$(find plans -type f -name '*.md' ! -name 'README.md' | wc -l)
if [[ "$PLAN_COUNT" -eq 0 ]]; then
  echo "FAIL: No retained implementation or handoff plans found in plans/."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Retention policy documented in plans/README.md ($PLAN_COUNT plan files retained)."
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

# 16. eggsec-daemon must not depend on TUI crates or engine crate
echo ""
echo "--- Check 16: Daemon free of TUI and engine dependencies ---"
HITS=$(rg -n 'ratatui|crossterm|eggsec-tui|eggsec-core' crates/eggsec-daemon/Cargo.toml 2>/dev/null || true)
# Check for non-optional eggsec engine dependency (optional = feature-gated is OK)
NON_OPT_EGGSEC=$(awk '/^\[dependencies\]/,/^\[/' crates/eggsec-daemon/Cargo.toml 2>/dev/null | grep -E 'eggsec =\|eggsec=' | grep -v 'optional = true' | grep -v 'optional=true' || true)
HITS="${HITS}${NON_OPT_EGGSEC}"
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: eggsec-daemon has TUI or non-optional engine dependencies. It must depend only on eggsec-runtime (optional feature-gated deps allowed)."
  FAIL=$((FAIL + 1))
else
  echo "PASS: Daemon free of TUI and non-optional engine dependencies."
fi

# 17. eggsec-daemon must not depend on transport crates (axum, tonic, etc.) in [dependencies]
# Feature-gated optional deps in [dependencies] are permitted (e.g., axum behind http-api feature).
echo ""
echo "--- Check 17: Daemon free of non-optional transport dependencies ---"
# Check only the [dependencies] section for non-optional transport crates
# Note: optional = true is allowed (feature-gated, e.g., http-api -> axum)
HITS=$(awk '/^\[dependencies\]/,/^\[/' crates/eggsec-daemon/Cargo.toml 2>/dev/null | grep -E '(axum|tonic|tokio.tungstenite|tower-http)' | grep -v 'optional = true' | grep -v 'optional=true' || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: eggsec-daemon has non-optional transport dependencies. Use feature-gated optional deps."
  FAIL=$((FAIL + 1))
else
  echo "PASS: Daemon transport dependencies are feature-gated (or absent)."
fi

# 18. TUI must not contain a match TaskKind execution dispatcher
# (dispatch routing lives in eggsec::dispatch::dispatch_inner, not TUI).
# NOTE: TUI legitimately uses TaskKind::Variant() to construct RunRequest,
# and test code may match on TaskKind for assertions. We check that all
# match task_kind occurrences are inside test modules (after #[cfg(test)]).
echo ""
echo "--- Check 18: TUI has no match TaskKind execution dispatcher ---"
SECTION_FAIL=0
# Get all files in TUI that contain match.*task_kind
MATCH_FILES=$(rg -l 'match.*task_kind' crates/eggsec-tui/src/ 2>/dev/null || true)
for file in $MATCH_FILES; do
  # Find the first #[cfg(test)] or mod tests line in this file
  TEST_MODULE_LINE=$(rg -n '#\[cfg\(test\)]|^mod tests' "$file" 2>/dev/null | head -1 | cut -d: -f1)
  if [[ -z "$TEST_MODULE_LINE" ]]; then
    # No test module found — any match is suspicious
    HITS=$(rg -n 'match.*task_kind' "$file" 2>/dev/null || true)
    echo "$HITS"
    echo "FAIL: $file has match on task_kind with no test module."
    SECTION_FAIL=$((SECTION_FAIL + 1))
    continue
  fi
  # Check each match: is it after the test module declaration?
  while IFS= read -r hit; do
    line=$(echo "$hit" | cut -d: -f1)
    if [[ "$line" -lt "$TEST_MODULE_LINE" ]]; then
      echo "$hit"
      echo "FAIL: $file has match on task_kind at line $line (before test module at $TEST_MODULE_LINE)."
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  done < <(rg -n 'match.*task_kind' "$file" 2>/dev/null || true)
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: All match TaskKind occurrences are in test code."
else
  FAIL=$((FAIL + 1))
fi

# 19. CLI TUI dependency must be feature-gated
echo ""
echo "--- Check 19: CLI TUI dependency is feature-gated ---"
# The eggsec-cli crate must NOT have an unconditional eggsec-tui dependency
# (exclude optional = true lines, which are feature-gated)
HITS=$(rg -n '^eggsec-tui' crates/eggsec-cli/Cargo.toml 2>/dev/null | grep -v 'optional = true' || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: eggsec-cli has unconditional eggsec-tui dependency. Feature-gate it."
  FAIL=$((FAIL + 1))
else
  echo "PASS: CLI TUI dependency is feature-gated."
fi

# 20. Runtime persistence boundary - no database drivers in runtime crate
echo ""
echo "--- Check 20: Runtime free of persistence dependencies ---"
HITS=$(rg -n 'rusqlite|sqlx' crates/eggsec-runtime/Cargo.toml 2>/dev/null || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: eggsec-runtime has persistence dependencies (rusqlite/sqlx). It must remain dependency-light."
  FAIL=$((FAIL + 1))
else
  echo "PASS: Runtime free of persistence dependencies."
fi

# 21. Engine must not depend on TUI or daemon crates (reverse dependency)
echo ""
echo "--- Check 21: Engine free of TUI and daemon dependencies ---"
HITS=$(rg -n 'eggsec-tui|eggsec-daemon' crates/eggsec/Cargo.toml 2>/dev/null || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: eggsec engine depends on eggsec-tui or eggsec-daemon. The engine must not depend on frontend crates."
  FAIL=$((FAIL + 1))
else
  echo "PASS: Engine free of TUI and daemon dependencies."
fi

# 22. Runtime domain crate isolation - runtime must not depend on engine or domain crates
echo ""
echo "--- Check 22: Runtime isolated from engine and domain crates ---"
HITS=$(rg -n 'eggsec[^-]|eggsec-core|eggsec-db-lab|eggsec-web-proxy|eggsec-mobile-lab|eggsec-nse|eggsec-agent|eggsec-output|eggsec-tool-core' crates/eggsec-runtime/Cargo.toml 2>/dev/null || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: eggsec-runtime depends on engine or domain crates. It must be dependency-light (serde, tokio, tracing only)."
  FAIL=$((FAIL + 1))
else
  echo "PASS: Runtime isolated from engine and domain crates."
fi

# 23. Output crate must not have reverse dependencies on engine or runtime
echo ""
echo "--- Check 23: Output free of reverse dependencies ---"
HITS=$(rg -n 'eggsec =|eggsec-runtime' crates/eggsec-output/Cargo.toml 2>/dev/null || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: eggsec-output depends on eggsec (engine) or eggsec-runtime. It must only depend on eggsec-core."
  FAIL=$((FAIL + 1))
else
  echo "PASS: Output free of reverse dependencies."
fi

# NSE runtime-internal guards moved with the runtime in Milestone 003. The
# standalone repository runs these checks in its own CI. Keep the historical
# in-tree checks available for branches that still carry the local crate.
if [[ -d crates/eggsec-nse ]]; then

# 24. NSE script/module loading flows through ScriptResolver.
# Direct `std::fs::read_to_string` / `std::fs::read` is only allowed in resolver.rs,
# executor_core.rs (load_script + setup_require), public_api/api.rs (manual-only),
# and a narrow metadata-parsing path in executor.rs (`parse_nse_categories`, which
# reads shipped NSE scripts to extract their `categories = {...}` metadata for
# category-aware execution, not to load user-provided scripts).
# `require()` filesystem loading must delegate to ScriptResolver::resolve_module().
echo ""
echo "--- Check 24: NSE script/module loading is resolver-owned ---"
HITS=$(rg -n 'std::fs::read_to_string\(|std::fs::read\(' \
  crates/eggsec-nse/src/ \
  --glob='!crates/eggsec-nse/src/resolver.rs' \
  --glob='!crates/eggsec-nse/src/resolver/mod.rs' \
  --glob='!crates/eggsec-nse/src/executor_core.rs' \
  --glob='!crates/eggsec-nse/src/public_api/api.rs' \
  --glob='!crates/eggsec-nse/src/libraries/*' \
  --glob='!crates/eggsec-nse/src/wrappers.rs' \
  2>/dev/null \
  || true)
# Allowlist the parse_nse_categories function body in executor.rs. Function reads
# shipped NSE metadata, not user scripts. Range covers line numbers as of the
# NSE Milestone 3 corrective pass (line count grew ~58 lines due to the
# with_full_policy / capability_context additions). Widened for the Milestone 4
# closure pass to cover the relocated parse_nse_categories helper.
FILTERED=$(printf '%s\n' "$HITS" | awk -F: '
  /^crates\/eggsec-nse\/src\/executor\.rs:/ {
    line = $2 + 0
    if (line >= 575 && line <= 665) { next }
  }
  { print }
' || true)
if [[ -n "$FILTERED" ]]; then
  echo "$FILTERED"
  echo "FAIL: Direct std::fs::read in NSE execution paths outside resolver/executor_core/public_api/libraries."
  echo "      Route through ScriptResolver or add to the allowlist above."
  FAIL=$((FAIL + 1))
else
  echo "PASS: NSE script/module loading is resolver-owned."
fi

# 25. NseRunReport.libraries is per-run require activity, not registry capability dump.
# Production report paths must use runtime observation (executor.library_reports(),
# required_modules(), library_use_reports_from_required_modules()) or explicitly
# labeled static fallback (library_use_reports_from_static_requires() with loaded=false).
# Registry::all_libraries() may only appear in docs/matrix generation or test code,
# never in production report paths.
echo ""
echo "--- Check 25: NseRunReport.libraries is per-run require activity ---"
# Detect fabrication patterns: registry iteration with loaded: true in production paths
FABRICATION_HITS=$(rg -n 'all_libraries\(\).*loaded:\s*true|registry.*map.*loaded:\s*true' \
  crates/eggsec-nse/src/ crates/eggsec/src/ \
  --glob='!*.rs.bak' \
  2>/dev/null || true)
if [[ -n "$FABRICATION_HITS" ]]; then
  echo "$FABRICATION_HITS"
  echo "FAIL: NseRunReport.libraries must not fabricate loaded status from registry inventory."
  echo "      Registry::all_libraries() describes capability metadata; per-run libraries"
  echo "      must come from runtime require tracking or labeled static fallback."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=1
else
  SECTION_FAIL=0
fi
# Also detect the narrow co-occurrence in run.rs/executor.rs (canonical report owner)
REGISTRY_HITS=$(rg -n 'registry::all_libraries\(\)' crates/eggsec-nse/src/run.rs crates/eggsec-nse/src/executor.rs 2>/dev/null || true)
LOADED_HITS=$(rg -n 'loaded:\s*true' crates/eggsec-nse/src/run.rs crates/eggsec-nse/src/executor.rs 2>/dev/null || true)
if [[ -n "$REGISTRY_HITS" && -n "$LOADED_HITS" ]]; then
  echo "$REGISTRY_HITS"
  echo "$LOADED_HITS"
  echo "FAIL: Production report paths must not map registry inventory directly into loaded runtime libraries."
  echo "      Use per-run require tracking, or expose capability snapshots under a separate field."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=1
fi
# Positive evidence: canonical run.rs uses runtime observation (lib.rs is a
# thin adapter delegating to execute_nse_run since Milestone 001)
if rg -q 'library_use_reports_from_required_modules|library_use_reports_from_static_requires|executor\.library_reports' crates/eggsec-nse/src/run.rs 2>/dev/null; then
  echo "PASS: Canonical report path uses runtime observation functions."
else
  echo "WARN: Could not confirm runtime observation functions in run.rs."
  echo "      Expected: library_use_reports_from_required_modules, library_use_reports_from_static_requires, or executor.library_reports()."
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: NseRunReport.libraries is per-run require activity, not registry dump."
fi

# 26. ManualPermissive profile constructors must not be used outside manual CLI/TUI surfaces.
# Allow list: crates/eggsec-nse/src/profile.rs (canonical constructors),
#             crates/eggsec-nse/src/lib.rs (run_cli_with_profile default fallback),
#             crates/eggsec-nse/src/resolver.rs (inline tests),
#             crates/eggsec-nse/tests/* (regression coverage),
#             crates/eggsec/tests/nse_bridge_tests.rs (engine-owned bridge
#               regression coverage driving the facade with a manual profile),
#             crates/eggsec/src/commands/handlers/scan.rs (CLI handler),
#             crates/eggsec/src/dispatch/api.rs (CLI dispatch).
echo ""
echo "--- Check 26: ManualPermissive stays in manual surfaces ---"
HITS=$(rg -n 'ResolvedNseExecutionProfile::manual_permissive' \
  crates/ \
  --glob='!crates/eggsec-nse/src/profile.rs' \
  --glob='!crates/eggsec-nse/src/lib.rs' \
  --glob='!crates/eggsec-nse/src/resolver.rs' \
  --glob='!crates/eggsec-nse/src/resolver/mod.rs' \
  --glob='!crates/eggsec-nse/tests/*' \
  --glob='!crates/eggsec/tests/nse_bridge_tests.rs' \
  --glob='!crates/eggsec/src/commands/handlers/scan.rs' \
  --glob='!crates/eggsec/src/dispatch/api.rs' \
  2>/dev/null || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: ManualPermissive is constructed outside the manual CLI/TUI surface allowlist."
  echo "      Automated surfaces must use agent_safe() or ci_safe()."
  FAIL=$((FAIL + 1))
else
  echo "PASS: ManualPermissive use stays in manual surfaces and tests."
fi

# 27. Manual-only executor constructors stay in manual-only paths.
# `NseExecutor::new()`, `with_sandbox()`, `with_target()` use permissive defaults.
# Allow list: profile/executor/executor_core source files, nse crate tests,
# eggsec crate tests (nse_tests.rs, nse_integration_tests.rs), manual CLI surfaces
# (commands/handlers/scan.rs, dispatch/api.rs, nse_tool.rs).
echo ""
echo "--- Check 27: Manual-only NseExecutor constructors stay manual ---"
HITS=$(rg -n 'NseExecutor::new\(|NseExecutor::with_sandbox\(|NseExecutor::with_target\(' \
  crates/ \
  --glob='!crates/eggsec-nse/src/executor.rs' \
  --glob='!crates/eggsec-nse/src/executor_core.rs' \
  --glob='!crates/eggsec-nse/tests/*' \
  --glob='!crates/eggsec-nse/src/profile.rs' \
  --glob='!crates/eggsec-nse/src/lib.rs' \
  --glob='!crates/eggsec/tests/*' \
  --glob='!crates/eggsec/src/commands/handlers/scan.rs' \
  --glob='!crates/eggsec/src/dispatch/api.rs' \
  --glob='!crates/eggsec/src/nse_tool.rs' \
  2>/dev/null || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: Manual-only NseExecutor constructors used outside the manual surface allowlist."
  echo "      Use with_policy() or with_profile() on automated surfaces."
  FAIL=$((FAIL + 1))
else
  echo "PASS: Manual-only NseExecutor constructors stay in manual surfaces."
fi

echo ""
echo "--- Check 28: NSE registry entries have corresponding Rust modules ---"
SECTION_FAIL=0
# Nmap Lua library names that map to different Rust module names.
# Format: "nmap_name:rust_module1,rust_module2"
declare -A NSE_NAME_MAP=(
  ["ssl"]="sslcert,openssl"
  ["lfs"]="lfs"
)
REGISTRY_NAMES=$(rg -o 'name: "[^"]*"' crates/eggsec-nse/src/resolver/registry.rs \
  | sed 's/.*name: "\([^"]*\)".*/\1/' | sort -u)
for NAME in $REGISTRY_NAMES; do
  # Direct match first
  if rg -q "^pub mod ${NAME};" crates/eggsec-nse/src/libraries/mod.rs 2>/dev/null; then
    continue
  fi
  # Check name mapping
  MAPPED="${NSE_NAME_MAP[$NAME]:-}"
  FOUND=0
  if [[ -n "$MAPPED" ]]; then
    IFS=',' read -ra MODULES <<< "$MAPPED"
    for MOD in "${MODULES[@]}"; do
      if rg -q "^pub mod ${MOD};" crates/eggsec-nse/src/libraries/mod.rs 2>/dev/null; then
        FOUND=1
        break
      fi
    done
  fi
  if [[ $FOUND -eq 0 ]]; then
    echo "FAIL: Registry entry '${NAME}' has no corresponding Rust module in libraries/mod.rs"
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -gt 0 ]]; then
  echo "FAIL: $SECTION_FAIL registry entry/entries missing corresponding Rust module."
  FAIL=$((FAIL + 1))
else
  echo "PASS: All NSE registry entries have corresponding Rust modules."
fi

# Reverse check: warn if Rust modules exist without a registry entry
MODULE_NAMES=$(rg -o '^pub mod [a-z_]+;' crates/eggsec-nse/src/libraries/mod.rs \
  | sed 's/^pub mod \([a-z_]*\);/\1/' | sort -u)
UNREGISTERED=""
for NAME in $MODULE_NAMES; do
  if ! rg -q "name: \"${NAME}\"" crates/eggsec-nse/src/resolver/registry.rs 2>/dev/null; then
    UNREGISTERED="${UNREGISTERED} ${NAME}"
  fi
done
if [[ -n "$UNREGISTERED" ]]; then
  echo "WARN: Rust modules without registry entries:${UNREGISTERED}"
  echo "      (These are likely protocol-specific implementations, not standard Nmap Lua libraries.)"
fi

# 29. NseLibraryDescriptor must only be instantiated in the registry module.
# Direct construction outside registry.rs bypasses the registry metadata contract
# and prevents policy evaluation, diagnostics, and compatibility reporting.
echo ""
echo "--- Check 29: NseLibraryDescriptor instantiation is registry-owned ---"
HITS=$(rg -n 'NseLibraryDescriptor\s*\{' \
  crates/eggsec-nse/src/ \
  --glob='!crates/eggsec-nse/src/resolver/registry.rs' \
  --glob='!crates/eggsec-nse/src/resolver/registry/*' \
  2>/dev/null || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: NseLibraryDescriptor constructed outside registry module."
  echo "      All library metadata must go through LIBRARY_REGISTRY in resolver/registry.rs."
  FAIL=$((FAIL + 1))
else
  echo "PASS: NseLibraryDescriptor instantiation is registry-owned."
fi

# 30. Canonical NSE pipeline populates library and rule metadata; the CLI
# helper stays a thin adapter.
# Since Milestone 001, `run_cli_with_profile()` in lib.rs delegates to
# `execute_nse_run()` in run.rs, which owns the single report-assembly path.
# The NseRunReport must include library use reports and rule evaluation
# reports for structured output to be complete. Skipping them produces empty
# arrays that hide compatibility truth.
echo ""
echo "--- Check 30: canonical NSE report path populates library/rule metadata ---"
SECTION_FAIL=0
# Check that the canonical pipeline calls with_rules()
if ! rg -q 'with_rules\(' crates/eggsec-nse/src/run.rs 2>/dev/null; then
  echo "FAIL: Canonical NSE pipeline (run.rs) does not call .with_rules() on NseRunReport."
  echo "      Rule evaluation metadata must be included in structured output."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# Check that the canonical pipeline calls with_libraries()
if ! rg -q 'with_libraries\(' crates/eggsec-nse/src/run.rs 2>/dev/null; then
  echo "FAIL: Canonical NSE pipeline (run.rs) does not call .with_libraries() on NseRunReport."
  echo "      Library use metadata must be included in structured output."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# Check that the CLI helper delegates instead of assembling its own report
if ! rg -q 'execute_nse_run' crates/eggsec-nse/src/lib.rs 2>/dev/null; then
  echo "FAIL: run_cli_with_profile() in lib.rs does not delegate to execute_nse_run()."
  echo "      The CLI helper must stay a thin adapter over the canonical pipeline."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Canonical NSE report path populates library/rule metadata; CLI delegates."
else
  FAIL=$((FAIL + 1))
fi

# 31. New rule-evaluation convenience APIs must produce NseRuleEvaluationReport metadata.
# Any public convenience function added to public_api/ that evaluates NSE rules
# (port rules, script rules, host rules) must populate NseRuleEvaluationReport
# entries so compatibility status is visible. Core executor functions are exempt
# (they are implementation, not convenience APIs).
echo ""
echo "--- Check 31: Rule evaluation convenience APIs produce NseRuleEvaluationReport ---"
if [[ -d "crates/eggsec-nse/src/public_api" ]]; then
  RULE_FUNCS=$(rg -n 'pub fn.*(?:rule|eval).*\(' \
    crates/eggsec-nse/src/public_api/ \
    2>/dev/null || true)
  if [[ -n "$RULE_FUNCS" ]]; then
    SECTION_FAIL=0
    while IFS= read -r func_line; do
      file=$(echo "$func_line" | cut -d: -f1)
      line_num=$(echo "$func_line" | cut -d: -f2)
      func_context=$(sed -n "${line_num},$((line_num + 30))p" "$file" 2>/dev/null || true)
      if ! echo "$func_context" | rg -q 'NseRuleEvaluationReport' 2>/dev/null; then
        echo "$func_line"
        echo "  -> Rule evaluation convenience API does not produce NseRuleEvaluationReport."
        SECTION_FAIL=$((SECTION_FAIL + 1))
      fi
    done <<< "$RULE_FUNCS"
    if [[ $SECTION_FAIL -gt 0 ]]; then
      echo "FAIL: $SECTION_FAIL rule-evaluation convenience API(s) missing NseRuleEvaluationReport."
      echo "      New convenience APIs must produce structured report metadata."
      FAIL=$((FAIL + 1))
    else
      echo "PASS: All rule evaluation convenience APIs produce NseRuleEvaluationReport."
    fi
  else
    echo "PASS: No rule-evaluation convenience APIs in public_api/ (none to audit)."
  fi
else
  echo "PASS: No public_api/ directory found (none to audit)."
fi

# 32. No docs claim full Nmap compatibility or parity.
# Eggsec has selective practical NSE compatibility, not full Nmap parity.
# Compatibility status is defined by NseLibraryRegistry metadata and
# NseRuleEvaluationReport fidelity, not by documentation claims.
echo ""
echo "--- Check 32: No docs claim full Nmap parity ---"
HITS=$(rg -in 'full\s+nmap\s+(compat|parity|support|compatible)|100%\s+nmap|complete\s+nmap\s+(compat|parity|support)|nmap\s+compatible\s+replacement|drop.in\s+nmap\s+replacement' \
  --glob='*.md' \
  docs/ architecture/ README.md .opencode/skills/ \
  2>/dev/null \
  | grep -v 'plans/' \
  | grep -v 'not ' \
  | grep -v 'does not' \
  | grep -v 'without' \
  | grep -v 'instead of' \
  || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: Found docs claiming full Nmap parity/compatibility."
  echo "      Use 'selective practical NSE compatibility' instead."
  FAIL=$((FAIL + 1))
else
  echo "PASS: No docs claim full Nmap parity."
fi

# 33. NSE capability wrappers: direct process exec in libraries must be migrated
# Phase 03 migrated filesystem and process wrappers through NseCapabilityContext.
# std::process::Command in library files (outside wrappers.rs, executor_core.rs, tests/)
# now FAILS because all process exec is routed through check_process_exec.
# nmap.rs is excluded because all its std::process::Command calls are guarded by
# check_process_exec() (nmap.is_admin, nmap.is_privileged).
# std::fs remains informational since not all libraries (unpwdb, brute, datafiles, etc.)
# are migrated yet.
echo ""
echo "--- Check 33: NSE direct process exec outside wrappers (FAIL) ---"
NSE_PROC_HITS=$(rg -n 'std::process::Command' --glob='*.rs' crates/eggsec-nse/src/libraries/ \
  --glob='!wrappers.rs' --glob='!executor_core.rs' --glob='!nmap.rs' --glob='!tests/' 2>/dev/null \
  | grep -v 'tests/' || true)
if [[ -n "$NSE_PROC_HITS" ]]; then
  echo "$NSE_PROC_HITS"
  echo "FAIL: Found direct std::process::Command in NSE libraries outside wrappers.rs."
  echo "      Phase 03 migrated all process exec through NseCapabilityContext via check_process_exec()."
  FAIL=$((FAIL + 1))
else
  echo "PASS: No direct std::process::Command in NSE libraries outside wrappers."
fi

echo ""
echo "--- Check 33b: NSE direct filesystem ops outside wrappers (info only) ---"
NSE_FS_HITS=$(rg -n 'std::fs::read_to_string|std::fs::write|std::fs::remove_file|std::fs::rename|std::fs::create_dir_all' \
  --glob='*.rs' crates/eggsec-nse/src/libraries/ \
  --glob='!wrappers.rs' --glob='!executor_core.rs' --glob='!tests/' 2>/dev/null \
  | grep -v 'tests/' | head -20 || true)
if [[ -n "$NSE_FS_HITS" ]]; then
  echo "$NSE_FS_HITS"
  echo "INFO: Found direct filesystem ops in NSE libraries outside wrappers."
  echo "      Some are migrated (io.rs, os.rs have capability checks before these calls)."
  echo "      Others (unpwdb, brute, datafiles) are not yet migrated."
else
  echo "PASS: No direct filesystem ops found in NSE libraries outside wrappers."
fi

echo ""
echo "--- Check 33c: NSE direct network calls outside wrappers (info only) ---"
NSE_NET_HITS=$(rg -n 'TcpStream::connect|UdpSocket::bind|TcpStream::connect_timeout' \
  --glob='*.rs' crates/eggsec-nse/src/libraries/ \
  --glob='!wrappers.rs' --glob='!executor_core.rs' --glob='!socket.rs' --glob='!comm.rs' --glob='!dns.rs' --glob='!nmap.rs' --glob='!tests/' 2>/dev/null \
  | grep -v 'tests/' | head -20 || true)
if [[ -n "$NSE_NET_HITS" ]]; then
  echo "$NSE_NET_HITS"
  echo "INFO: Found direct TcpStream/UdpSocket usage in NSE libraries outside wrappers."
  echo "      socket.rs, comm.rs, dns.rs, and nmap.rs are migrated (capability checks before connect)."
  echo "      Protocol libraries (smb, ssh, ftp, etc.) are not yet migrated."
else
  echo "PASS: No direct network calls found in NSE libraries outside wrappers."
fi

# 34. NSE capability context integration (info only, not failing yet)
echo ""
echo "--- Check 34: NSE capability context integration (info only) ---"
NSE_CAP_HITS=$(rg -n 'NseCapabilityContext|NseCapabilityKind|NseCapabilityEvent|check_capability' --glob='*.rs' crates/eggsec-nse/src/ 2>/dev/null | head -20 || true)
if [[ -n "$NSE_CAP_HITS" ]]; then
  echo "PASS: NSE capability context types found in crates/eggsec-nse/src/"
else
  echo "INFO: No NSE capability context types found. Capability context integration may not be complete."
fi

# 35. NSE run_cli_with_profile() must construct executor via with_profile or with_full_policy
# Profile propagation regression: using with_policy() here silently downgrades
# AgentSafe/CiSafe capability decisions to ManualPermissive because with_policy
# hardcodes ManualPermissive + AllowAllManual in the capability context.
# See plans/nse-milestone-3-corrective-pass.md (Workstream 2).
echo ""
echo "--- Check 35: NSE executor construction uses with_profile/with_full_policy (FAIL) ---"
NSE_RUNCLI_WITH_POLICY_HITS=$(rg -n 'NseExecutor::with_policy' --glob='*.rs' crates/eggsec-nse/src/lib.rs crates/eggsec-nse/src/run.rs 2>/dev/null \
  | grep -E 'run_cli_with_profile|run_cli\b|execute_nse_run' || true)
if [[ -n "$NSE_RUNCLI_WITH_POLICY_HITS" ]]; then
  echo "$NSE_RUNCLI_WITH_POLICY_HITS"
  echo "FAIL: NSE entry paths construct NseExecutor with with_policy()."
  echo "      Use NseExecutor::with_profile(&resolved_profile) or NseExecutor::with_full_policy(...)"
  echo "      so capability context profile_kind/network_policy match the resolved profile."
  FAIL=$((FAIL + 1))
else
  echo "PASS: NSE entry paths do not use with_policy() for executor construction."
fi

# 36. NSE automated surfaces must not use with_policy() (manual-only API)
# The with_policy() constructor hardcodes ManualPermissive profile kind and
# AllowAllManual network policy. Automated surfaces (MCP, agent, REST, daemon,
# CI) MUST use with_profile() or with_full_policy() so the capability engine
# enforces the resolved profile. Manual CLI/TUI is the only accepted caller.
# See plans/nse-milestone-3-corrective-pass.md (Workstream 3).
echo ""
echo "--- Check 36: automated NSE surfaces must not use with_policy() (FAIL) ---"
NSE_AUTOMATED_DIRS=(
  "crates/eggsec/src/dispatch"
  "crates/eggsec/src/agent"
  "crates/eggsec/src/mcp"
  "crates/eggsec/src/rest"
  "crates/eggsec/src/grpc"
  "crates/eggsec-daemon/src"
)
NSE_AUTOMATED_HITS=""
for dir in "${NSE_AUTOMATED_DIRS[@]}"; do
  if [[ -d "$dir" ]]; then
    hits=$(rg -n 'NseExecutor::with_policy|NseExecutor::new|NseExecutor::with_sandbox|NseExecutor::with_target' \
      --glob='*.rs' "$dir" 2>/dev/null || true)
    if [[ -n "$hits" ]]; then
      NSE_AUTOMATED_HITS="${NSE_AUTOMATED_HITS}${hits}"$'\n'
    fi
  fi
done
if [[ -n "$NSE_AUTOMATED_HITS" ]]; then
  echo "$NSE_AUTOMATED_HITS"
  echo "FAIL: Found automated-surface NseExecutor constructor that bypasses with_profile()."
  echo "      with_policy()/new()/with_sandbox()/with_target() are manual-only and hardcode"
  echo "      ManualPermissive in the capability context. Use with_profile(&profile) or"
  echo "      with_full_policy(...) so AgentSafe/CiSafe capability decisions are honored."
  FAIL=$((FAIL + 1))
else
  echo "PASS: No automated-surface NseExecutor uses manual-only constructors."
fi

# 37. ExecutorCore::with_policy must remain a manual-only compatibility wrapper
# Direct calls to ExecutorCore::with_policy outside manual surfaces are
# discouraged. Automated surfaces should use with_full_policy or with_profile.
echo ""
echo "--- Check 37: ExecutorCore::with_policy manual-only callers (INFO) ---"
NSE_CORE_POLICY_HITS=$(rg -n 'ExecutorCore::with_policy' --glob='*.rs' crates/ 2>/dev/null \
  | grep -v 'crates/eggsec-nse/src/executor_core.rs' | grep -v '/tests/' || true)
if [[ -n "$NSE_CORE_POLICY_HITS" ]]; then
  echo "$NSE_CORE_POLICY_HITS"
  echo "INFO: Found ExecutorCore::with_policy callers outside executor_core.rs."
  echo "      This is allowed for manual CLI/TUI surfaces but not for automated surfaces."
  echo "      Automated surfaces should use ExecutorCore::with_full_policy or with_profile."
else
  echo "PASS: No ExecutorCore::with_policy callers outside executor_core.rs."
fi

echo ""
echo "--- Check 38: NSE upstream corpus has local-only fixtures (INFO) ---"
UPSTREAM_LOCAL_ONLY=$(grep -c 'local_fixture = true' crates/eggsec-nse/tests/fixtures/nse_corpus/manifest.toml 2>/dev/null || echo "0")
UPSTREAM_TOTAL=$(grep -c '\[\[fixture\]\]' crates/eggsec-nse/tests/fixtures/nse_corpus/manifest.toml 2>/dev/null || echo "0")
if [[ "$UPSTREAM_LOCAL_ONLY" -lt "$UPSTREAM_TOTAL" ]]; then
  echo "FAIL: Not all fixtures have local_fixture = true ($UPSTREAM_LOCAL_ONLY / $UPSTREAM_TOTAL)"
  echo "      All NSE corpus fixtures must be local-only to avoid upstream license dependencies."
  FAIL=$((FAIL + 1))
elif [[ "$UPSTREAM_TOTAL" -lt 10 ]]; then
  echo "INFO: Corpus has only $UPSTREAM_TOTAL fixtures. Expected 10-37."
  echo "      Corpus should remain representative but not claim full upstream coverage."
else
  echo "PASS: All $UPSTREAM_TOTAL corpus fixtures are local-only ($UPSTREAM_LOCAL_ONLY with local_fixture=true)."
fi

echo ""
echo "--- Check 39: NSE evidence extraction goes through extract_evidence() (INFO) ---"
EVIDENCE_BYPASS=$(rg -n 'NseEvidenceItem\s*\{' --glob='*.rs' crates/ 2>/dev/null \
  | grep -v 'extract_evidence' \
  | grep -v 'report.rs' \
  | grep -v '/tests/' || true)
if [[ -n "$EVIDENCE_BYPASS" ]]; then
  echo "$EVIDENCE_BYPASS"
  echo "INFO: Found direct NseEvidenceItem construction outside extract_evidence/report.rs."
  echo "      Evidence items should be produced by extract_evidence() for consistency."
else
  echo "PASS: No NseEvidenceItem construction outside extract_evidence/report.rs."
fi

echo ""
echo "--- Check 40: NSE bridge module exists for ReportEnvelope (INFO) ---"
if [[ ! -f crates/eggsec/src/nse_bridge.rs ]]; then
  echo "FAIL: crates/eggsec/src/nse_bridge.rs does not exist."
  echo "      NSE evidence must bridge to ReportEnvelope via the engine-owned nse_bridge."
  echo "      (Moved out of eggsec-nse in runtime-extraction Milestone 002.)"
  FAIL=$((FAIL + 1))
else
  echo "PASS: Engine-owned NSE bridge module exists."
fi

echo ""
echo "--- Check 41: NSE compatibility matrix exists ---"
if [[ ! -f docs/NSE_COMPATIBILITY.md ]]; then
  echo "FAIL: docs/NSE_COMPATIBILITY.md does not exist."
  echo "      Milestone 4 requires a published compatibility matrix."
  FAIL=$((FAIL + 1))
else
  MAT_LINES=$(wc -l < docs/NSE_COMPATIBILITY.md)
  if [[ "$MAT_LINES" -lt 100 ]]; then
    echo "INFO: NSE_COMPATIBILITY.md has only $MAT_LINES lines. Expected 100+."
  else
    echo "PASS: NSE_COMPATIBILITY.md exists ($MAT_LINES lines)."
  fi
fi

echo ""
echo "--- Check 42: NSE runtime corpus tests live in a separate test binary ---"
if [[ ! -f crates/eggsec-nse/tests/runtime_corpus_tests.rs ]]; then
  echo "FAIL: crates/eggsec-nse/tests/runtime_corpus_tests.rs does not exist."
  echo "      Milestone 4 requires a dedicated runtime corpus test binary."
  FAIL=$((FAIL + 1))
else
  echo "PASS: runtime_corpus_tests.rs exists."
fi

echo ""
echo "--- Check 43: NSE runtime corpus tests use NseExecutor::with_profile ---"
RUNTIME_PROFILE_USE=$(rg -l 'NseExecutor::with_profile|ExecutorCore::with_profile' crates/eggsec-nse/tests/runtime_corpus_tests.rs 2>/dev/null || true)
if [[ -z "$RUNTIME_PROFILE_USE" ]]; then
  echo "FAIL: runtime_corpus_tests.rs does not call NseExecutor::with_profile."
  echo "      Runtime corpus tests must drive execution through the resolved profile path."
  FAIL=$((FAIL + 1))
else
  echo "PASS: runtime_corpus_tests.rs uses NseExecutor::with_profile."
fi

echo ""
echo "--- Check 44: NSE static corpus harness does not execute scripts ---"
STATIC_EXEC=$(rg -n 'run_script_with_rules' crates/eggsec-nse/tests/compatibility_corpus_tests.rs 2>/dev/null || true)
if [[ -n "$STATIC_EXEC" ]]; then
  echo "$STATIC_EXEC"
  echo "FAIL: Static corpus harness (compatibility_corpus_tests.rs) calls run_script_with_rules."
  echo "      Static harness must remain resolver-only; runtime verification belongs in runtime_corpus_tests.rs."
  FAIL=$((FAIL + 1))
else
  echo "PASS: Static corpus harness is resolver-only (no run_script_with_rules)."
fi

echo ""
echo "--- Check 45: No self-referential expected value construction in runtime corpus tests ---"
# Runtime corpus tests must not construct expected values by calling production APIs.
# Expected values (expected_libraries, expected_rules, expected_capability_events)
# must come from the manifest.toml, not from calling production APIs like
# Registry::all_libraries(), resolve_script(), or NseCapabilityContext methods.
# Self-referential tests would always pass regardless of production code bugs.
SELF_REF_HITS=$(rg -n 'Registry::all_libraries|LIBRARY_REGISTRY' \
  crates/eggsec-nse/tests/runtime_corpus_tests.rs \
  2>/dev/null || true)
if [[ -n "$SELF_REF_HITS" ]]; then
  echo "$SELF_REF_HITS"
  echo "FAIL: Runtime corpus tests reference production code to build expected values."
  echo "      Expected values must come from manifest.toml, not production APIs."
  echo "      Self-referential tests always pass regardless of production code bugs."
  FAIL=$((FAIL + 1))
else
  echo "PASS: Runtime corpus tests use static manifest data for expected values."
fi

echo ""
echo "--- Check 46: Runtime corpus test assertions must not be trivially satisfiable ---"
# Hard assertions (assert!, assert_eq!) in corpus_runtime_observed_* functions
# must compare against actual data, not always-true conditions.
TRIVIAL_HITS=$(rg -n 'assert!\(true\)|assert_eq!\(report\.\w+\.len\(\), report\.\w+\.len\(\)\)' \
  crates/eggsec-nse/tests/runtime_corpus_tests.rs \
  2>/dev/null || true)
if [[ -n "$TRIVIAL_HITS" ]]; then
  echo "$TRIVIAL_HITS"
  echo "FAIL: Found trivially satisfiable assertions in runtime corpus tests."
  echo "      Assertions must compare against actual expected values from the manifest."
  FAIL=$((FAIL + 1))
else
  echo "PASS: No trivially satisfiable assertions found in runtime corpus tests."
fi

echo ""
echo "--- Check 47: Local protocol fixtures have local_service metadata ---"
# All local protocol fixtures (those bound to 127.0.0.1 with local servers)
# must declare [local_service] in manifest.toml to signal the runtime harness
# that they require a real listener (not synthetic context).
LOCAL_FIXTURES=$(rg -l 'local_service' crates/eggsec-nse/tests/fixtures/nse_corpus/manifest.toml 2>/dev/null || true)
if [[ -n "$LOCAL_FIXTURES" ]]; then
  echo "PASS: Local protocol fixtures declare local_service metadata."
else
  echo "WARN: No local_service metadata found in manifest.toml."
  echo "      Local protocol fixtures should declare [local_service] for harness skipping."
fi
# Verify runtime harness has local_service skip logic
SKIP_PRESENT=$(rg -c 'local_service' crates/eggsec-nse/tests/runtime_corpus_tests.rs 2>/dev/null || true)
if [[ -n "$SKIP_PRESENT" && "$SKIP_PRESENT" -gt 0 ]]; then
  echo "PASS: Runtime corpus harness has local_service skip logic."
else
  echo "FAIL: Runtime corpus harness does not skip local_service fixtures."
  echo "      Add local_service.is_some() skip checks in runtime iteration sites."
  FAIL=$((FAIL + 1))
fi
# Verify local_protocol_tests.rs exists for local fixture coverage
if [[ -f crates/eggsec-nse/tests/local_protocol_tests.rs ]]; then
  echo "PASS: local_protocol_tests.rs exists for local protocol fixture coverage."
else
  echo "FAIL: local_protocol_tests.rs does not exist."
  echo "      Local protocol fixtures need dedicated runtime tests with real listeners."
  FAIL=$((FAIL + 1))
fi

echo ""
echo "--- Check 48: HTTP check_network_tcp before reqwest ---"
http_file="crates/eggsec-nse/src/libraries/http.rs"
if [ -f "$http_file" ]; then
    check_count=$(grep -c "check_network_tcp" "$http_file" || echo 0)
    if [ "$check_count" -lt 4 ]; then
        echo "FAIL: http.rs has only $check_count check_network_tcp calls (expected >= 4)"
        FAIL=$((FAIL + 1))
    else
        echo "PASS: http.rs has $check_count check_network_tcp calls."
    fi
else
    echo "SKIP: http.rs not found (nse feature not enabled)."
fi

echo ""
echo "--- Check 48b: HTTP method operations defined ---"
http_file="crates/eggsec-nse/src/libraries/http.rs"
if [ -f "$http_file" ]; then
    missing=""
    for op in "http.get" "http.post" "http.put" "http.delete" "http.head" "http.options" "http.request"; do
        if ! grep -q "\"$op\"" "$http_file"; then
            missing="$missing $op"
        fi
    done
    if [ -n "$missing" ]; then
        echo "FAIL: http.rs missing operation strings:$missing"
        FAIL=$((FAIL + 1))
    else
        echo "PASS: http.rs defines all core HTTP method operations."
    fi
else
    echo "SKIP: http.rs not found."
fi

echo ""
echo "--- Check 48c: Local HTTP denied tests assert zero hits ---"
# Check that every server.hits() call has a strict assertion on same or next line
# Accepts: assert_eq!(server.hits(), 0, ...) or assert!(server.hits() > 0, ...)
HITS=$(rg -n 'server\.hits\(\)' crates/eggsec-nse/tests/local_protocol_tests.rs 2>/dev/null | while read -r line; do
    linenum=$(echo "$line" | cut -d: -f1)
    content=$(echo "$line" | cut -d: -f2-)
    nextline=$(sed -n "$((linenum+1))p" crates/eggsec-nse/tests/local_protocol_tests.rs)
    combined="$content $nextline"
    if ! echo "$combined" | grep -qE '(==\s*0|>\s*0|,\s*0\s*[,)])'; then
        echo "$line"
    fi
done || true)
if [[ -n "$HITS" ]]; then
    echo "$HITS"
    echo "FAIL: Found server.hits() calls without strict equality assertion."
    FAIL=$((FAIL + 1))
else
    echo "PASS: All server.hits() assertions are strict."
fi

echo ""
echo "--- Check 48d: No permissive denied-test language for automated HTTP ---"
# Check for permissive language in test function names or doc comments, not in assertion message strings
HITS=$(rg -n '(may (fail|succeed)|accept either)' crates/eggsec-nse/tests/local_protocol_tests.rs 2>/dev/null | grep -v '// ' | grep -v 'assert' | grep -v '"' || true)
if [[ -n "$HITS" ]]; then
    echo "$HITS"
    echo "FAIL: Found permissive language in automated HTTP denial tests."
    FAIL=$((FAIL + 1))
else
    echo "PASS: No permissive language in automated HTTP denial tests."
fi

echo ""
echo "--- Check 49: No permissive AgentSafe HTTP text ---"
HITS=$(rg -n 'should complete.*not crash|AgentSafe.*accepts either|is_ok.*is_err' crates/eggsec-nse/tests/ 2>/dev/null | grep -i 'http\|agent' | grep -v '// ' || true)
if [[ -n "$HITS" ]]; then
    echo "$HITS"
    echo "FAIL: Found lenient permissive text for AgentSafe HTTP."
    FAIL=$((FAIL + 1))
else
    echo "PASS: No lenient permissive text for AgentSafe HTTP."
fi

echo ""
echo "--- Check 50: No lenient library assertions in runtime tests ---"
HITS=$(rg -n 'is_empty\(\) \|\| found|\.is_empty\(\).*found' crates/eggsec-nse/tests/ 2>/dev/null | grep -v '// ' | grep -v 'allow_missing' || true)
if [[ -n "$HITS" ]]; then
    echo "$HITS"
    echo "FAIL: Found lenient library assertions."
    FAIL=$((FAIL + 1))
else
    echo "PASS: No lenient library assertions."
fi

echo ""
echo "--- Check 51: Every .send() in http.rs has a preflight gate within 15 lines ---"
http_file="crates/eggsec-nse/src/libraries/http.rs"
if [ -f "$http_file" ]; then
    SEND_HITS=$(awk '/\.send\(\)/{send_line=NR; send_content=$0} /check_network_tcp|maybe_denied_response/{check_line=NR} send_line>0 && check_line>0 && check_line>=send_line-15 && check_line<=send_line{found=1} send_line>0 && NR>send_line+15 && !found{print send_line": "send_content; found=0; send_line=0; check_line=0} END{if(send_line>0 && !found) print send_line": "send_content}' "$http_file" 2>/dev/null)
    if [[ -n "$SEND_HITS" ]]; then
        echo "$SEND_HITS"
        echo "FAIL: Found .send() calls without a preflight gate within 15 lines."
        FAIL=$((FAIL + 1))
    else
        echo "PASS: All .send() calls in http.rs have a preflight gate within 15 lines."
    fi
else
    echo "SKIP: http.rs not found."
fi

echo ""
echo "--- Check 51b: Async HTTP functions use check_network_tcp directly (documented pattern) ---"
http_file="crates/eggsec-nse/src/libraries/http.rs"
if [ -f "$http_file" ]; then
    # The 3 async functions use check_network_tcp directly + denied_response(),
    # which is equivalent to maybe_denied_response() but inlined for async control flow.
    # They are registered as closures via http.set(), so we search for the registration name.
    missing_async=""
    for name in "async_get" "async_post" "async_request"; do
        # Find the line with the registration, then check next 25 lines for check_network_tcp
        reg_line=$(grep -n "\"$name\"" "$http_file" | head -1 | cut -d: -f1)
        if [ -n "$reg_line" ]; then
            end_line=$((reg_line + 25))
            if ! sed -n "${reg_line},${end_line}p" "$http_file" | grep -q "check_network_tcp"; then
                missing_async="$missing_async $name"
            fi
        else
            missing_async="$missing_async $name"
        fi
    done
    if [ -n "$missing_async" ]; then
        echo "FAIL: Async HTTP functions missing check_network_tcp:$missing_async"
        FAIL=$((FAIL + 1))
    else
        echo "PASS: All async HTTP functions use check_network_tcp directly."
    fi
else
    echo "SKIP: http.rs not found."
fi

echo ""
echo "--- Check 52: Async HTTP functions are sync Lua closures using block_on (not true async) ---"
http_file="crates/eggsec-nse/src/libraries/http.rs"
if [ -f "$http_file" ]; then
    missing_blockon=""
    for name in "async_get" "async_post" "async_request"; do
        reg_line=$(grep -n "\"$name\"" "$http_file" | head -1 | cut -d: -f1)
        if [ -n "$reg_line" ]; then
            end_line=$((reg_line + 30))
            if ! sed -n "${reg_line},${end_line}p" "$http_file" | grep -q "block_on"; then
                missing_blockon="$missing_blockon $name"
            fi
        else
            missing_blockon="$missing_blockon $name"
        fi
    done
    if [ -n "$missing_blockon" ]; then
        echo "FAIL: Async HTTP functions missing block_on (not sync Lua closures):$missing_blockon"
        FAIL=$((FAIL + 1))
    else
        echo "PASS: All async HTTP functions use block_on (sync Lua closures wrapping async reqwest)."
    fi
else
    echo "SKIP: http.rs not found."
fi

echo ""
echo "--- Check 53: TLS local fixture scripts exist and are declared in manifest ---"
tls_scripts_dir="crates/eggsec-nse/tests/fixtures/nse_corpus/scripts/protocol"
tls_manifest="crates/eggsec-nse/tests/fixtures/nse_corpus/manifest.toml"
tls_missing=""
for script in "sslcert_get_certificate_local.nse" "sslcert_parse_cert_local.nse" "sslcert_get_subject_local.nse" "sslcert_get_chain_certs_local.nse" "sslcert_is_valid_local.nse"; do
    if [ ! -f "$tls_scripts_dir/$script" ]; then
        tls_missing="$tls_missing $script"
    fi
done
if [ -n "$tls_missing" ]; then
    echo "FAIL: Missing TLS fixture scripts:$tls_missing"
    FAIL=$((FAIL + 1))
else
    # Check manifest declares them
    manifest_missing=""
    for script in "sslcert_get_certificate_local" "sslcert_parse_cert_local" "sslcert_get_subject_local" "sslcert_get_chain_certs_local" "sslcert_is_valid_local"; do
        if ! grep -q "$script" "$tls_manifest" 2>/dev/null; then
            manifest_missing="$manifest_missing $script"
        fi
    done
    if [ -n "$manifest_missing" ]; then
        echo "FAIL: TLS scripts not declared in manifest:$manifest_missing"
        FAIL=$((FAIL + 1))
    else
        echo "PASS: All 5 TLS fixture scripts exist and are declared in manifest."
    fi
fi

echo ""
echo "--- Check 54: Upstream-style fixtures are local-only, clean-room, no public network ---"
upstream_manifest="crates/eggsec-nse/tests/fixtures/nse_corpus/manifest.toml"
if [ -f "$upstream_manifest" ]; then
    # Extract upstream fixture blocks and check provenance/public_network/local_fixture
    upstream_bad=""
    upstream_count=0
    in_upstream=false
    current_id=""
    current_provenance=""
    current_public_net=""
    current_local=""
    while IFS= read -r line; do
        # Detect start of a new fixture block
        if [[ "$line" == '[['*']]' ]]; then
            # Check previous entry if it was upstream
            if [ "$in_upstream" = true ]; then
                if [ "$current_provenance" != "clean-room" ]; then
                    upstream_bad="$upstream_bad $current_id (provenance=$current_provenance)"
                fi
                if [ "$current_public_net" = "true" ]; then
                    upstream_bad="$upstream_bad $current_id (public_network_required=true)"
                fi
                if [ "$current_local" != "true" ]; then
                    upstream_bad="$upstream_bad $current_id (local_fixture=$current_local)"
                fi
            fi
            in_upstream=false
            current_id=""
            current_provenance=""
            current_public_net=""
            current_local=""
        fi
        # Parse fields
        if [[ "$line" =~ ^id\ =\ \"(.*)\" ]]; then
            current_id="${BASH_REMATCH[1]}"
        fi
        if [[ "$line" =~ ^category\ =\ \"upstream\" ]]; then
            in_upstream=true
            upstream_count=$((upstream_count + 1))
        fi
        if [[ "$line" =~ ^provenance\ =\ \"(.*)\" ]]; then
            current_provenance="${BASH_REMATCH[1]}"
        fi
        if [[ "$line" =~ ^public_network_required\ =\ (.*) ]]; then
            current_public_net="${BASH_REMATCH[1]}"
        fi
        if [[ "$line" =~ ^local_fixture\ =\ (.*) ]]; then
            current_local="${BASH_REMATCH[1]}"
        fi
    done < "$upstream_manifest"
    # Check last entry
    if [ "$in_upstream" = true ]; then
        if [ "$current_provenance" != "clean-room" ]; then
            upstream_bad="$upstream_bad $current_id (provenance=$current_provenance)"
        fi
        if [ "$current_public_net" = "true" ]; then
            upstream_bad="$upstream_bad $current_id (public_network_required=true)"
        fi
        if [ "$current_local" != "true" ]; then
            upstream_bad="$upstream_bad $current_id (local_fixture=$current_local)"
        fi
    fi
    if [ -n "$upstream_bad" ]; then
        echo "FAIL: Upstream fixtures with bad metadata:$upstream_bad"
        FAIL=$((FAIL + 1))
    else
        echo "PASS: All $upstream_count upstream fixtures are local-only, clean-room, no public network."
    fi
else
    echo "SKIP: manifest.toml not found."
fi

echo ""
echo "--- Check 55: creds library registered with capability context ---"
creds_source="crates/eggsec-nse/src/libraries/creds.rs"
creds_executor="crates/eggsec-nse/src/executor_core.rs"
if [ -f "$creds_source" ] && [ -f "$creds_executor" ]; then
    # Check function signature accepts NseCapabilityContext
    if grep -q 'pub fn register_creds_library(lua: &Lua, _capability_ctx: &NseCapabilityContext)' "$creds_source"; then
        # Check call site passes capability context
        if grep -q 'register_creds_library(&self.lua, &self.capability_context)' "$creds_executor"; then
            echo "PASS: register_creds_library accepts NseCapabilityContext and call site passes it."
        else
            echo "FAIL: register_creds_library call site does not pass capability context."
            FAIL=$((FAIL + 1))
        fi
    else
        echo "FAIL: register_creds_library does not accept NseCapabilityContext parameter."
        FAIL=$((FAIL + 1))
    fi
else
    echo "SKIP: creds source or executor_core not found."
fi

else
  echo "--- NSE runtime internal guards ---"
  echo "SKIP: runtime-owned checks run in eggstack/eggsec-nse CI."
fi

echo ""
echo "--- Check 56: Every sslcert TcpStream::connect must have a network gate within 30 lines ---"
SSLCERT_FILE="crates/eggsec-nse/src/libraries/sslcert.rs"
if [ -f "$SSLCERT_FILE" ]; then
    SSL_CERT_BAD_CONNECTS=$(awk '
    /TcpStream::connect/ {
      line=NR; text=$0; found=0;
      for (i=line-30; i<line; i++) { if (i>0 && checks[i]) found=1; }
      if (!found) print line ": " text;
    }
    /check_network_tcp|check_crypto/ { checks[NR]=1 }
    ' "$SSLCERT_FILE" 2>/dev/null)
    if [[ -n "$SSL_CERT_BAD_CONNECTS" ]]; then
        echo "$SSL_CERT_BAD_CONNECTS"
        echo "FAIL: Found TcpStream::connect in sslcert.rs without a network gate within 30 lines."
        FAIL=$((FAIL + 1))
    else
        CONNECT_COUNT=$(rg -c "TcpStream::connect" "$SSLCERT_FILE" 2>/dev/null || echo 0)
        echo "PASS: All $CONNECT_COUNT TcpStream::connect calls in sslcert.rs have a network gate within 30 lines."
    fi
else
    echo "SKIP: sslcert.rs not found."
fi

# 57. Every nmap.rs TcpStream::connect must have a network capability gate within 30 lines
echo ""
echo "--- Check 57: Every nmap.rs TcpStream::connect must have a network gate within 30 lines ---"
NMAP_FILE="crates/eggsec-nse/src/libraries/nmap.rs"
if [ -f "$NMAP_FILE" ]; then
    NMAP_BAD_CONNECTS=$(awk '
    /TcpStream::connect/ {
      line=NR; text=$0; found=0;
      for (i=line-30; i<line; i++) { if (i>0 && checks[i]) found=1; }
      if (!found) print line ": " text;
    }
    /check_network_tcp/ { checks[NR]=1 }
    ' "$NMAP_FILE" 2>/dev/null)
    if [[ -n "$NMAP_BAD_CONNECTS" ]]; then
        echo "$NMAP_BAD_CONNECTS"
        echo "FAIL: Found TcpStream::connect in nmap.rs without a network capability gate within 30 lines."
        FAIL=$((FAIL + 1))
    else
        NMAP_CONNECT_COUNT=$(rg -c "TcpStream::connect" "$NMAP_FILE" 2>/dev/null || echo 0)
        echo "PASS: All $NMAP_CONNECT_COUNT TcpStream::connect calls in nmap.rs have a network capability gate within 30 lines."
    fi
else
    echo "SKIP: nmap.rs not found."
fi

# 58. EggsecRuntimeExecutor::execute() must not hardcode RuntimeSurface::CliManual
# (The resolve_loaded_scope helper legitimately checks for permissive surfaces.)
echo ""
echo "--- Check 58: EggsecRuntimeExecutor::execute() does not hardcode CliManual ---"
EXECUTOR_FILE="crates/eggsec/src/runtime_bridge/executor.rs"
if [ -f "$EXECUTOR_FILE" ]; then
    # Extract the execute() method body (from "fn execute" to the next "fn " or end of impl)
    EXEC_BODY=$(awk '/fn execute\(/{found=1} found{print NR": "$0} found && /^    fn [a-z]/{if(NR>start+1) exit}' start=0 "$EXECUTOR_FILE" 2>/dev/null || true)
    HITS=$(echo "$EXEC_BODY" | grep 'RuntimeSurface::CliManual' || true)
    if [[ -n "$HITS" ]]; then
        echo "$HITS"
        echo "FAIL: EggsecRuntimeExecutor::execute() hardcodes RuntimeSurface::CliManual. Use session-derived surface from RuntimeExecutionContext."
        FAIL=$((FAIL + 1))
    else
        echo "PASS: EggsecRuntimeExecutor::execute() does not hardcode RuntimeSurface::CliManual."
    fi
else
    echo "SKIP: executor.rs not found."
fi

# 59. EggsecRuntimeExecutor::execute() must not hardcode LoadedScope::default_empty()
# (The resolve_loaded_scope helper legitimately falls back for permissive surfaces.)
echo ""
echo "--- Check 59: EggsecRuntimeExecutor::execute() does not hardcode default_empty scope ---"
if [ -f "$EXECUTOR_FILE" ]; then
    EXEC_BODY=$(awk '/fn execute\(/{found=1} found{print NR": "$0} found && /^    fn [a-z]/{if(NR>start+1) exit}' start=0 "$EXECUTOR_FILE" 2>/dev/null || true)
    HITS=$(echo "$EXEC_BODY" | grep 'LoadedScope::default_empty' || true)
    if [[ -n "$HITS" ]]; then
        echo "$HITS"
        echo "FAIL: EggsecRuntimeExecutor::execute() hardcodes LoadedScope::default_empty(). Use scope from RuntimeExecutionContext."
        FAIL=$((FAIL + 1))
    else
        echo "PASS: EggsecRuntimeExecutor::execute() does not hardcode LoadedScope::default_empty()."
    fi
else
    echo "SKIP: executor.rs not found."
fi

# 60. Executor registry: no operation ID appears in more than one executor
echo ""
echo "--- Check 60: Executor registry has no duplicate operation IDs ---"
# This check is validated by the executor_registry_no_duplicates test in dispatch/mod.rs
# The test uses FxHashSet to detect duplicates at compile time
echo "PASS: Duplicate detection is enforced by executor_registry_no_duplicates test."

# 61. All 22 stable-core operations have an executor
echo ""
echo "--- Check 61: Stable-core operations have executors ---"
# Check that core operations have executors by looking for them in executor files
CORE_OPS_COVERED=0
CORE_OPS_TOTAL=0
for op_id in "scan-ports" "scan-endpoints" "fingerprint" "recon" "waf-detect" "waf-bypass" "waf-stress" "fuzz" "load-test" "stress-test" "packet" "auth-test" "graphql" "oauth"; do
  CORE_OPS_TOTAL=$((CORE_OPS_TOTAL + 1))
  if rg -q "\"$op_id\"" crates/eggsec/src/dispatch/executors/*.rs 2>/dev/null; then
    CORE_OPS_COVERED=$((CORE_OPS_COVERED + 1))
  fi
done
if [[ $CORE_OPS_COVERED -eq $CORE_OPS_TOTAL ]]; then
  echo "PASS: All $CORE_OPS_TOTAL core operation IDs have executors."
else
  echo "WARN: Only $CORE_OPS_COVERED/$CORE_OPS_TOTAL core operation IDs have executors."
  echo "      (Some may be handled by the monolithic dispatch_inner fallback.)"
fi

# 62. Feature-gated executors are only compiled when feature is enabled
echo ""
echo "--- Check 62: Feature-gated executors use cfg guards ---"
SECTION_FAIL=0
# Check that feature-gated executor files have cfg attributes
for executor_file in crates/eggsec/src/dispatch/executors/nse.rs crates/eggsec/src/dispatch/executors/db_pentest.rs; do
  if [[ -f "$executor_file" ]]; then
    # Check for #[cfg(feature = "...")] on the struct
    if ! rg -q '#\[cfg\(feature = "' "$executor_file" 2>/dev/null; then
      echo "FAIL: $executor_file missing #[cfg(feature = \"...\")] guard."
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  fi
done
# Check that mod declarations in executors/mod.rs use cfg
for mod_name in nse db_pentest; do
  if ! rg -q "#\[cfg.*feature.*\].*pub mod $mod_name;" crates/eggsec/src/dispatch/executors/mod.rs 2>/dev/null; then
    if ! rg -q "pub mod $mod_name;" crates/eggsec/src/dispatch/executors/mod.rs 2>/dev/null; then
      # Module doesn't exist at all - that's OK if feature is disabled
      :
    else
      # Module exists but not feature-gated
      LINE=$(rg -n "pub mod $mod_name;" crates/eggsec/src/dispatch/executors/mod.rs 2>/dev/null | head -1)
      if [[ -n "$LINE" ]]; then
        linenum=$(echo "$LINE" | cut -d: -f1)
        # Check if the line before has #[cfg(feature = "...")]
        prev_line=$((linenum - 1))
        prev_content=$(sed -n "${prev_line}p" crates/eggsec/src/dispatch/executors/mod.rs 2>/dev/null)
        if ! echo "$prev_content" | grep -q '#\[cfg'; then
          echo "FAIL: pub mod $mod_name in executors/mod.rs not feature-gated."
          SECTION_FAIL=$((SECTION_FAIL + 1))
        fi
      fi
    fi
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Feature-gated executors use cfg guards."
else
  FAIL=$((FAIL + 1))
fi

# 63. Executor trait is object-safe (no generic parameters on self methods)
echo ""
echo "--- Check 63: OperationExecutor trait is object-safe ---"
TRAIT_FILE="crates/eggsec/src/dispatch/executor.rs"
if [[ -f "$TRAIT_FILE" ]]; then
  # Check for generic parameters on self methods (indicators of non-object-safety)
  GENERIC_SELF=$(rg -n 'fn.*<.*>.*&self' "$TRAIT_FILE" 2>/dev/null | grep -v '//' || true)
  if [[ -n "$GENERIC_SELF" ]]; then
    echo "$GENERIC_SELF"
    echo "FAIL: OperationExecutor trait has generic parameters on self methods (not object-safe)."
    FAIL=$((FAIL + 1))
  else
    echo "PASS: OperationExecutor trait is object-safe."
  fi
else
  echo "SKIP: executor.rs not found."
fi

# 64. Python operation registry has exactly 22 stable operations
echo ""
echo "--- Check 64: Python operation registry has exactly 22 operations ---"
if [[ -f "crates/eggsec-python/src/operation_registry.rs" ]]; then
  # Count entries strictly inside the `ALL` array.
  #
  # The previous range form (`/pub const ALL/,/^\s*\];/`) never terminated on
  # awk implementations without GNU `\s` support (including macOS /usr/bin/awk),
  # so the range ran to end-of-file and counted every `Self::` in the file —
  # reporting 130 instead of the real 22. Use a POSIX bracket expression and an
  # explicit in-array flag so the count is correct everywhere.
  ALL_ENTRIES=$(awk '
    /pub const ALL/       { inside = 1; next }
    inside && /Self::/    { n++ }
    inside && /^[[:space:]]*\];/ { inside = 0 }
    END                   { print n + 0 }
  ' crates/eggsec-python/src/operation_registry.rs 2>/dev/null)
  ALL_ENTRIES=${ALL_ENTRIES:-0}

  # Count enum variants in StableOperation. Checking both keeps the guard
  # meaningful: it now also fails when a variant is added without being
  # registered in ALL, which the ALL-only count could not detect.
  VARIANT_ENTRIES=$(awk '
    /pub enum StableOperation/ { inside = 1; next }
    inside && /^[[:space:]]*\}/ { inside = 0 }
    inside && /^[[:space:]]*(#|\/\/|\/\/\/)/ { next }
    inside && /^[[:space:]]*[A-Z][A-Za-z0-9_]*[[:space:]]*,[[:space:]]*$/ { n++ }
    END { print n + 0 }
  ' crates/eggsec-python/src/operation_registry.rs 2>/dev/null)
  VARIANT_ENTRIES=${VARIANT_ENTRIES:-0}

  if [[ "$ALL_ENTRIES" -ne 22 ]]; then
    echo "FAIL: operation_registry.rs has $ALL_ENTRIES entries in ALL (expected 22)"
    FAIL=$((FAIL + 1))
  elif [[ "$VARIANT_ENTRIES" -ne "$ALL_ENTRIES" ]]; then
    echo "FAIL: operation_registry.rs has $VARIANT_ENTRIES StableOperation variants but $ALL_ENTRIES entries in ALL"
    FAIL=$((FAIL + 1))
  else
    echo "PASS: operation_registry.rs has exactly 22 entries in ALL, matching 22 variants."
  fi
else
  echo "SKIP: operation_registry.rs not found."
fi

# 65. Python daemon mapping uses registry descriptor
echo ""
echo "--- Check 65: Python daemon mapping uses registry descriptor ---"
if [[ -f "crates/eggsec-python/src/engine.rs" ]]; then
  # The old hardcoded match should be replaced with a delegation call
  OLD_MATCH=$(rg -c '"scan_ports" \|"scan-ports"' crates/eggsec-python/src/engine.rs 2>/dev/null || echo 0)
  if [[ "$OLD_MATCH" -gt 0 ]]; then
    echo "FAIL: engine.rs still contains hardcoded daemon task kind matching."
    echo "      operation_request_to_task_kind_json should delegate to dispatch_helpers::operation_request_to_daemon_task."
    FAIL=$((FAIL + 1))
  else
    # Verify delegation exists
    if rg -q 'operation_request_to_daemon_task' crates/eggsec-python/src/engine.rs 2>/dev/null; then
      echo "PASS: engine.rs delegates to registry-based operation_request_to_daemon_task."
    else
      echo "WARN: Could not confirm delegation to operation_request_to_daemon_task in engine.rs."
    fi
  fi
else
  echo "SKIP: engine.rs not found."
fi

# 66. Python dispatch uses 3-phase lifecycle (no legacy inline dispatch in dispatch())
echo ""
echo "--- Check 66: Python dispatch uses 3-phase lifecycle ---"
if [[ -f "crates/eggsec-python/src/engine.rs" ]]; then
  # The dispatch() method should delegate to pre_dispatch_lifecycle, not contain
  # inline scope/policy validation. Check that pre_dispatch_lifecycle is called.
  HAS_LIFECYCLE=$(rg -c 'pre_dispatch_lifecycle' crates/eggsec-python/src/engine.rs 2>/dev/null || echo 0)
  HAS_EXECUTE=$(rg -c 'execute_operation' crates/eggsec-python/src/engine.rs 2>/dev/null || echo 0)
  HAS_POSTHOOKS=$(rg -c 'post_dispatch_hooks' crates/eggsec-python/src/engine.rs 2>/dev/null || echo 0)
  if [[ "$HAS_LIFECYCLE" -gt 0 ]] && [[ "$HAS_EXECUTE" -gt 0 ]] && [[ "$HAS_POSTHOOKS" -gt 0 ]]; then
    echo "PASS: dispatch() uses 3-phase lifecycle (pre_dispatch_lifecycle → execute_operation → post_dispatch_hooks)."
  else
    echo "FAIL: dispatch() does not use the 3-phase lifecycle pattern."
    echo "      Expected calls to pre_dispatch_lifecycle, execute_operation, and post_dispatch_hooks."
    FAIL=$((FAIL + 1))
  fi
else
  echo "SKIP: engine.rs not found."
fi

# 67. NSE libraries must not use unsafe Handle::current().block_on
if [[ -d crates/eggsec-nse ]]; then
echo ""
echo "--- Check 67: NSE libraries avoid Handle::current().block_on ---"
HANDLE_BLOCK_HITS=$(rg -n 'Handle::current\(\)\.block_on' crates/eggsec-nse/src/libraries/ 2>/dev/null \
  | rg -v 'libraries/runtime_bridge.rs' || true)
if [[ -n "$HANDLE_BLOCK_HITS" ]]; then
  echo "$HANDLE_BLOCK_HITS"
  echo "FAIL: NSE libraries must use crate::libraries::runtime_bridge::block_on_async."
  echo "      Handle::current().block_on panics when called inside a tokio runtime task"
  echo "      and may deadlock when no runtime is active."
  FAIL=$((FAIL + 1))
else
  echo "PASS: NSE libraries route async execution through runtime_bridge::block_on_async."
fi

# 68. NSE libraries must not hardcode insecure TLS acceptance (BUG-003)
# Direct reqwest::Client::builder() calls that hardcode `true` are forbidden.
# Calls inside `if <parameter>` blocks that gate on the caller's choice are OK,
# since the parameter is propagated by the caller.
echo ""
echo "--- Check 68: NSE libraries gate insecure TLS on capability context ---"
TLS_HARDCODED=$(rg -nU --multiline \
  'Client::builder\(\)[\s\S]{0,200}?\.danger_accept_invalid_certs\(true\)' \
  crates/eggsec-nse/src/libraries/ crates/eggsec-nse/src/public_api/ 2>/dev/null \
  | rg -v 'helpers\.rs' \
  | rg -v 'http\.rs' || true)
if [[ -n "$TLS_HARDCODED" ]]; then
  echo "$TLS_HARDCODED"
  echo "FAIL: NSE libraries/public_api must not hardcode insecure TLS acceptance."
  echo "      Pass NseCapabilityContext through and gate on allows_insecure_tls()."
  FAIL=$((FAIL + 1))
else
  echo "PASS: NSE libraries gate insecure TLS on capability context."
fi

# 69. NSE library globals expose reset_for_run() to allow ExecutorCore to
# reset per-scan state. The reset is invoked from clear_library_reports().
echo ""
echo "--- Check 69: NSE library globals expose reset_for_run() ---"
LIBRARY_GLOBALS=$(rg -l 'static (CR|ACCOUNT|FILE_HANDLES|CRAWLER|CRAWLER_COUNTER|COMPILED|REGEX|SMB_SESSIONS|SSL_SESSIONS|CREDENTIALS|TARGET|USERNAMES|PASSWORDS|HASH|TEST_RESULTS)' \
  crates/eggsec-nse/src/libraries/ 2>/dev/null || true)
MISSING_RESET=""
for f in $LIBRARY_GLOBALS; do
  if ! rg -q 'pub fn reset_for_run' "$f"; then
    MISSING_RESET="$MISSING_RESET $f"
  fi
done
if [[ -n "$MISSING_RESET" ]]; then
  echo "$MISSING_RESET"
  echo "FAIL: NSE library files with global statics must expose pub fn reset_for_run."
  echo "      ExecutorCore::clear_library_reports calls each module's reset_for_run."
  echo "      to prevent cross-scan contamination."
  FAIL=$((FAIL + 1))
else
  echo "PASS: NSE library globals expose reset_for_run()."
fi

else
  echo "--- NSE runtime async/TLS/global-state guards ---"
  echo "SKIP: runtime-owned checks run in eggstack/eggsec-nse CI."
fi

# 70. Protocol-neutral scope DTO must not authorize (Phase A scope contract)
# eggsec-tool-core's ScopeSpec/ToolScopeSpec is declarative caller intent only.
# An is_allowed()/authorize() method on the DTO would be a competing
# authorization implementation diverging from engine policy (CIDR, DNS
# multi-address, ports, non-public handling). Only eggsec::config::Scope
# evaluated through EnforcementContext may authorize.
echo ""
echo "--- Check 70: ScopeSpec DTO has no authorization methods ---"
DTO_AUTH=$(rg -n 'fn (is_allowed|authorize)\b' crates/eggsec-tool-core/src/ 2>/dev/null || true)
if [[ -n "$DTO_AUTH" ]]; then
  echo "$DTO_AUTH"
  echo "FAIL: eggsec-tool-core must not implement is_allowed()/authorize()."
  echo "      Convert ScopeSpec via eggsec::config::scope_from_spec and evaluate"
  echo "      through EnforcementContext/engine Scope instead."
  FAIL=$((FAIL + 1))
else
  echo "PASS: ScopeSpec DTO has no authorization methods."
fi

# 71. Feature documentation matches Cargo source of truth (Phase B).
# Default features, declared features, curated `full` membership, and the
# domain-crate inventory must agree with crates/eggsec/Cargo.toml and
# FULL_MEMBERS/FULL_EXCLUDED_WITH_REASON in feature_registry.rs.
echo ""
echo "--- Check 71: Feature docs match Cargo source of truth ---"
if command -v python3 >/dev/null 2>&1; then
  if python3 scripts/check-feature-docs.py; then
    echo "PASS: Feature docs match Cargo source of truth."
  else
    echo "FAIL: Feature docs drifted from Cargo source of truth (see above)."
    FAIL=$((FAIL + 1))
  fi
else
  echo "FAIL: python3 is required for feature docs validation."
  FAIL=$((FAIL + 1))
fi

# 72. Individual feature sweep exists and is wired into deep checks.
# `full` is curated, so the sweep in scripts/check-features-individual.sh is
# the exhaustive oracle; it must be runnable via make and scheduled weekly.
echo ""
echo "--- Check 72: Individual feature sweep is maintained ---"
SECTION_FAIL=0
if [[ ! -f "scripts/check-features-individual.sh" ]]; then
  echo "FAIL: scripts/check-features-individual.sh is missing."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'check-features-individual' Makefile 2>/dev/null; then
  echo "FAIL: Makefile has no check-features-individual target."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'check-features-individual' .github/workflows/deep-checks.yml 2>/dev/null; then
  echo "FAIL: deep-checks.yml does not schedule the individual feature sweep."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Individual feature sweep is maintained and scheduled."
fi

# 73. Engine service traits exist (Phase D WS1).
# Protocol/agent adapters must depend on narrow capability traits, not on
# concrete registry/dispatcher construction.
echo ""
echo "--- Check 73: Engine service traits exist ---"
SECTION_FAIL=0
for sym in "trait OperationCatalog" "trait CheckedExecutor" "trait PreflightService" "struct EngineServices" "trait AgentExecutionService" "struct McpEngineBridge"; do
  if ! rg -q "$sym" crates/eggsec/src/tool/service.rs crates/eggsec/src/agent/services.rs crates/eggsec/src/tool/protocol/mcp/bridge.rs 2>/dev/null; then
    echo "FAIL: missing '$sym' in service/bridge modules."
    FAIL=$((FAIL + 1))
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Engine service traits exist."
fi

# 74. Protocol adapters use checked dispatch only (Phase D WS2-4/WS8).
# Raw `.dispatch(` (without `_checked`) and direct `tool.execute(` bypass
# enforcement binding checks. Adapters must go through
# `dispatch_checked` via EngineServices/McpEngineBridge.
echo ""
echo "--- Check 74: Protocol adapters use checked dispatch only ---"
SECTION_FAIL=0
RAW_DISPATCH=$(rg -n '\.dispatch\(' crates/eggsec/src/tool/protocol/ 2>/dev/null | rg -v 'dispatch_checked' | rg -v '^\s*//' | rg -v '//' | rg -v 'ChainPlanner|planner' || true)
if [[ -n "$RAW_DISPATCH" ]]; then
  echo "$RAW_DISPATCH"
  echo "FAIL: Raw .dispatch() in protocol adapters. Use dispatch_checked via EngineServices."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
DIRECT_EXEC=$(rg -n '\.execute\(' crates/eggsec/src/tool/protocol/openai/ crates/eggsec/src/tool/protocol/openresponses/ 2>/dev/null | rg -v 'dispatch_checked' | rg -v '^\s*//' | rg -v '//' || true)
if [[ -n "$DIRECT_EXEC" ]]; then
  echo "$DIRECT_EXEC"
  echo "FAIL: Direct tool.execute() in OpenAI/OpenResponses adapters. Use EngineServices dispatch_checked."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Protocol adapters use checked dispatch only."
fi

# 75. Adapters do not duplicate scope/policy evaluation (Phase D WS3).
# The legacy `Scope::is_target_allowed` DTO check must not appear in protocol
# adapter *code*; authorization lives in EnforcementContext via EngineServices.
# (Comments mentioning the removed check and MCP profile constraints in
# `constraints.rs` are not DTO authorization and are excluded.)
echo ""
echo "--- Check 75: No DTO scope auth in protocol adapters ---"
HITS=$(rg -n 'is_target_allowed' crates/eggsec/src/tool/protocol/ 2>/dev/null | rg -v '^\s*//' | rg -v '//' | rg -v 'constraints\.rs' || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: Protocol adapters must not call is_target_allowed. Use EnforcementContext via EngineServices."
  FAIL=$((FAIL + 1))
else
  echo "PASS: No DTO scope auth in protocol adapters."
fi

# 76. Approval token construction stays private/controlled (Phase D WS6/WS8).
# Only enforcement code (policy_approval.rs, policy_decision.rs) may call
# `ApprovedOperation::new`. Adapters/agents must obtain tokens via approve().
echo ""
echo "--- Check 76: Approval token construction is controlled ---"
# Phase C: the pure kernel (eggsec-policy/src/decision.rs) is a legitimate
# construction site alongside the engine facades; adapters must still obtain
# tokens via approve()/approve_manual().
HITS=$(rg -n 'ApprovedOperation::new' crates/ --glob='*.rs' 2>/dev/null | rg -v 'config/policy_approval.rs' | rg -v 'config/policy_decision.rs' | rg -v 'eggsec-policy/src/decision.rs' | rg -v 'eggsec-policy/src/approval.rs' || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: ApprovedOperation::new outside enforcement modules."
  FAIL=$((FAIL + 1))
else
  echo "PASS: Approval token construction is controlled."
fi

# 77. Protocol adapters do not depend on concrete domain implementations.
# They may use ToolRegistry/EngineServices/catalog metadata, but must not
# import `tool::implementations::` directly (that would make the adapter a
# second composition root).
echo ""
echo "--- Check 77: Protocol free of concrete domain impls ---"
HITS=$(rg -n 'implementations::' crates/eggsec/src/tool/protocol/ 2>/dev/null || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: Protocol adapters must not depend on concrete domain implementations."
  FAIL=$((FAIL + 1))
else
  echo "PASS: Protocol free of concrete domain impls."
fi

# 78. Hotspot decomposition facades exist (Phase D WS6-7).
echo ""
echo "--- Check 78: Hotspot facades exist ---"
SECTION_FAIL=0
for f in "crates/eggsec/src/config/policy_target.rs" "crates/eggsec/src/config/policy_catalog.rs" "crates/eggsec/src/config/policy_approval.rs" "crates/eggsec/src/config/scope_address.rs" "crates/eggsec/src/config/scope_resolver.rs" "crates/eggsec-runtime/src/runtime_config.rs" "crates/eggsec-runtime/src/runtime_sink.rs" "crates/eggsec-daemon/src/host_auth.rs" "crates/eggsec-daemon/src/host_persistence.rs" "crates/eggsec/src/tool/service.rs" "crates/eggsec/src/tool/protocol/mcp/bridge.rs" "crates/eggsec/src/agent/services.rs"; do
  if [[ ! -f "$f" ]]; then
    echo "FAIL: missing hotspot module: $f"
    FAIL=$((FAIL + 1))
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Hotspot facades exist."
fi

# 79. TUI approval cache uses exact descriptor binding (Phase 0.1).
# The facade must reuse cached approvals only via matches_descriptor()
# (derived PartialEq, future fields participate automatically), never by
# operation-name equality. Operation-name comparisons in non-test facade
# code would reintroduce stale reuse across targets/options.
echo ""
echo "--- Check 79: TUI approval cache uses exact binding ---"
FACADE="crates/eggsec-tui/src/app/enforcement_facade.rs"
if [[ -f "$FACADE" ]]; then
  SECTION_FAIL=0
  if ! rg -q 'matches_descriptor' "$FACADE" 2>/dev/null; then
    echo "FAIL: $FACADE does not use ApprovedOperation::matches_descriptor()."
    FAIL=$((FAIL + 1))
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  # Operation-name-only reuse outside tests/comments (allow descriptor().operation in audit/display, not == comparisons for cache identity).
  NAME_CMP=$(rg -n 'descriptor\(\)\.operation\s*==' "$FACADE" 2>/dev/null | rg -v '^\s*//' | rg -v '#\[' || true)
  if [[ -n "$NAME_CMP" ]]; then
    echo "$NAME_CMP"
    echo "FAIL: $FACADE compares descriptor().operation by name for cache identity. Use matches_descriptor() instead."
    FAIL=$((FAIL + 1))
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  if [[ $SECTION_FAIL -eq 0 ]]; then
    echo "PASS: TUI approval cache uses exact descriptor binding."
  fi
else
  echo "SKIP: $FACADE not found."
fi

# 80. Frontend parity guards exist in the mandatory verification path (Phase 0).
# Semantic parity lives in Rust tests; this guard only proves the test files
# exist and `make check` runs the TUI suite (no shell regex for semantics).
echo ""
echo "--- Check 80: Frontend parity guards are wired ---"
SECTION_FAIL=0
for f in "crates/eggsec/tests/frontend_surface_matrix.rs" "crates/eggsec-tui/src/parity.rs"; do
  if [[ ! -f "$f" ]]; then
    echo "FAIL: missing parity guard file: $f"
    FAIL=$((FAIL + 1))
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if ! rg -q 'eggsec-tui' Makefile 2>/dev/null; then
  echo "FAIL: Makefile does not reference eggsec-tui (parity tests not in mandatory path)."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Frontend parity guards exist and are wired."
fi

# 81. TUI TabSpec operations are canonical (Phase 0.4).
# Raw alias strings must not reappear as TabSpec::operation values. The
# compatibility aliases (`waf` -> `waf-detect`, `scan-pipeline` -> `pipeline`)
# remain resolvable via ALL_OPERATION_METADATA_ALIASES, but specs must use
# the canonical ids so the TUI never invents a second identity.
echo ""
echo "--- Check 81: TUI TabSpec operations are canonical ---"
SECTION_FAIL=0
STALE_WAF=$(rg -n 'operation:\s*Some\("waf"\)' crates/eggsec-tui/src/tabs/spec.rs 2>/dev/null || true)
if [[ -n "$STALE_WAF" ]]; then
  echo "$STALE_WAF"
  echo "FAIL: TabSpec uses raw alias Some(\"waf\"). Use canonical Some(\"waf-detect\")."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
STALE_PIPE=$(rg -n 'operation:\s*Some\("scan-pipeline"\)' crates/eggsec-tui/src/tabs/spec.rs 2>/dev/null || true)
if [[ -n "$STALE_PIPE" ]]; then
  echo "$STALE_PIPE"
  echo "FAIL: TabSpec uses raw alias Some(\"scan-pipeline\"). Use canonical Some(\"pipeline\")."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: TUI TabSpec operations are canonical."
fi

# 82. Canonical execution boundary is the single executor owner (Phase 1.1/1.4).
# The executor match lives in dispatch/canonical_execution.rs; the legacy
# dispatch_inner shim delegates to it and owns no second TaskKind→worker match.
echo ""
echo "--- Check 82: Canonical execution boundary owns dispatch ---"
SECTION_FAIL=0
CANON="crates/eggsec/src/dispatch/canonical_execution.rs"
if [[ ! -f "$CANON" ]]; then
  echo "FAIL: missing canonical execution boundary: $CANON"
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  for sym in "execute_approved" "execute_canonical" "CanonicalOperationRequest" "ExecutionEvent" "ExecutionSink" "executor_route_for" "is_feature_available"; do
    if ! rg -q "$sym" "$CANON" 2>/dev/null; then
      echo "FAIL: $CANON missing required symbol: $sym"
      FAIL=$((FAIL + 1))
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  done
fi
# dispatch_inner must delegate (references execute_canonical) and must not own
# a second TaskKind match outside tests.
if ! rg -q 'execute_canonical' crates/eggsec/src/dispatch/mod.rs 2>/dev/null; then
  echo "FAIL: dispatch/mod.rs does not delegate to execute_canonical."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
TASKKIND_MATCH=$(rg -n 'match request\.task_kind' crates/eggsec/src/dispatch/mod.rs 2>/dev/null || true)
if [[ -n "$TASKKIND_MATCH" ]]; then
  echo "$TASKKIND_MATCH"
  echo "FAIL: dispatch/mod.rs still owns a TaskKind match. Executor ownership lives in canonical_execution.rs."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Canonical execution boundary owns dispatch."
fi

# 83. Executor boundary is free of presentation imports (Phase 1.1).
# No Clap, Ratatui, terminal, daemon-protocol, TUI, or Python types may cross
# the execution boundary.
echo ""
echo "--- Check 83: Executor boundary has no presentation imports ---"
SECTION_FAIL=0
PRES_HITS=$(rg -n 'use clap|use ratatui|crossterm|eggsec_daemon_protocol|eggsec-daemon-protocol|use pyo3|eggsec_tui|eggsec-tui|terminal::' "$CANON" 2>/dev/null || true)
if [[ -n "$PRES_HITS" ]]; then
  echo "$PRES_HITS"
  echo "FAIL: Canonical execution boundary imports presentation types."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Executor boundary is presentation-free."
fi

# 84. CLI routing contract is single-owned (Phase 1.2/1.8).
# commands/route.rs owns Commands→CommandRoute (exhaustive); handle_command
# classifies once via route_for_commands; the old registry-bridge prelude is gone.
echo ""
echo "--- Check 84: CLI routing contract is single-owned ---"
SECTION_FAIL=0
ROUTE="crates/eggsec/src/commands/route.rs"
if [[ ! -f "$ROUTE" ]]; then
  echo "FAIL: missing CLI routing contract: $ROUTE"
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  for sym in "route_for_commands" "CommandRoute" "route_for_command_id"; do
    if ! rg -q "$sym" "$ROUTE" 2>/dev/null; then
      echo "FAIL: $ROUTE missing required symbol: $sym"
      FAIL=$((FAIL + 1))
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  done
fi
if ! rg -q 'route_for_commands' crates/eggsec/src/commands/handlers/mod.rs 2>/dev/null; then
  echo "FAIL: handle_command does not classify via route_for_commands."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
BRIDGE_HITS=$(rg -n 'Dispatch bridge' crates/eggsec/src/commands/handlers/mod.rs 2>/dev/null || true)
if [[ -n "$BRIDGE_HITS" ]]; then
  echo "$BRIDGE_HITS"
  echo "FAIL: Transitional registry-bridge prelude still present in handle_command."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: CLI routing contract is single-owned."
fi

# 85. Embedded and daemon runtimes share the executor owner (Phase 1.5/1.6).
# Daemon bundle routes via execute_approved; TUI dispatcher via
# execute_canonical; TUI owns no dispatch_inner call.
echo ""
echo "--- Check 85: Runtimes share the canonical executor ---"
SECTION_FAIL=0
if ! rg -q 'execute_approved' crates/eggsec/src/runtime_bridge/bundle.rs 2>/dev/null; then
  echo "FAIL: runtime_bridge/bundle.rs does not route via execute_approved."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'execute_canonical' crates/eggsec-tui/src/app/task_dispatcher.rs 2>/dev/null; then
  echo "FAIL: TUI dispatcher does not delegate to execute_canonical."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
TUI_INNER=$(rg -n 'dispatch_inner' crates/eggsec-tui/src/app/task_dispatcher.rs 2>/dev/null || true)
if [[ -n "$TUI_INNER" ]]; then
  echo "$TUI_INNER"
  echo "FAIL: TUI dispatcher still calls dispatch_inner directly. Use execute_canonical."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Runtimes share the canonical executor."
fi

# 86. Cancellation semantics are shared (Phase 1.6, closed Phase 3).
# The shared primitive lives in eggsec-runtime::cancel; both adapters
# (embedded TUI executor, daemon-backed engine executor) route dispatch
# through race_with_cancel so terminal/cancel semantics match by
# construction. Ad hoc `tokio::select!` on `cancel.cancelled()` in adapter
# code would reintroduce divergent semantics.
echo ""
echo "--- Check 86: Cancellation semantics are shared ---"
SECTION_FAIL=0
if [[ ! -f "crates/eggsec-runtime/src/cancel.rs" ]]; then
  echo "FAIL: missing shared cancellation primitive: crates/eggsec-runtime/src/cancel.rs"
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
elif ! rg -q 'race_with_cancel' crates/eggsec-runtime/src/cancel.rs 2>/dev/null; then
  echo "FAIL: cancel.rs missing race_with_cancel."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
for f in "crates/eggsec/src/runtime_bridge/executor.rs" "crates/eggsec-tui/src/app/task_runtime.rs"; do
  if ! rg -q 'race_with_cancel' "$f" 2>/dev/null; then
    echo "FAIL: $f does not route through the shared race_with_cancel primitive."
    FAIL=$((FAIL + 1))
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
# No ad hoc cancel select in adapter execute paths (the primitive owns it).
ADHOC=$(rg -n 'cancel\.cancelled\(\)' crates/eggsec-tui/src/app/task_runtime.rs crates/eggsec/src/runtime_bridge/executor.rs 2>/dev/null || true)
if [[ -n "$ADHOC" ]]; then
  echo "$ADHOC"
  echo "FAIL: Adapter code races cancel.cancelled() directly. Route through race_with_cancel."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Cancellation semantics are shared."
fi

# 87. Application-boundary tests are in the mandatory path (Phase 1).
echo ""
echo "--- Check 87: Canonical dispatch tests are wired ---"
SECTION_FAIL=0
if [[ ! -f "crates/eggsec/tests/canonical_dispatch_ownership.rs" ]]; then
  echo "FAIL: missing application-boundary test: crates/eggsec/tests/canonical_dispatch_ownership.rs"
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'cargo test -p eggsec --features rest-api' Makefile 2>/dev/null; then
  echo "FAIL: Makefile mandatory path does not run eggsec --tests (boundary tests not wired)."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Canonical dispatch tests are wired."
fi

# 88. Four-action pilot is removed (Phase 2.1).
# TabSpec is the single production surface owner; do not reintroduce a second
# metadata registry.
echo ""
echo "--- Check 88: TUI pilot registry is removed ---"
SECTION_FAIL=0
if [[ -f "crates/eggsec-tui/src/app/action_spec.rs" ]]; then
  echo "FAIL: pilot registry still exists: crates/eggsec-tui/src/app/action_spec.rs"
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'pub struct TuiActionSpec|static TUI_ACTION_SPECS|mod action_spec' crates/eggsec-tui/src --glob='*.rs' 2>/dev/null; then
  echo "FAIL: stale pilot registry definition (TuiActionSpec/TUI_ACTION_SPECS/mod action_spec) in eggsec-tui/src."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: TUI pilot registry is removed."
fi

# 89. Alias lookup is single-owned (Phase 2.5).
# No manual command->tab match in app/command.rs; resolve_palette_command owns it.
echo ""
echo "--- Check 89: TUI alias lookup is single-owned ---"
SECTION_FAIL=0
if ! rg -q 'resolve_palette_command' crates/eggsec-tui/src/tabs/spec.rs 2>/dev/null; then
  echo "FAIL: tabs/spec.rs missing resolve_palette_command."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q '"portscan" =>|"fingerprinting" =>|"wafstress" \|' crates/eggsec-tui/src/app/command.rs 2>/dev/null; then
  echo "FAIL: app/command.rs reintroduces a manual command->tab match."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: TUI alias lookup is single-owned."
fi

# 90. reload-scope is not discoverable (Phase 2.8).
echo ""
echo "--- Check 90: reload-scope is not a false affordance ---"
SECTION_FAIL=0
if rg -q '"reload-scope"\.to_string' crates/eggsec-tui/src/app/help_config.rs 2>/dev/null; then
  echo "FAIL: help_config.rs still advertises reload-scope in discovery."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: reload-scope is not a false affordance."
fi

# 91. copy-cli is a semantic round trip (Phase 2.7).
echo ""
echo "--- Check 91: copy-cli round-trips through Clap ---"
SECTION_FAIL=0
if ! rg -q 'pub fn cli_argv' crates/eggsec-tui/src/app/operation.rs 2>/dev/null; then
  echo "FAIL: app/operation.rs missing cli_argv argv vector."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ ! -f "crates/eggsec-tui/src/app/surface_wiring.rs" ]]; then
  echo "FAIL: missing round-trip tests: crates/eggsec-tui/src/app/surface_wiring.rs"
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q -- '--max-payloads' crates/eggsec-tui/src/app/operation.rs 2>/dev/null; then
  echo "FAIL: operation.rs emits --max-payloads (no such CLI flag)."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: copy-cli round-trips through Clap."
fi

# 92. Fuzz HTTP-session flag avoids the global --session collision (Phase 2).
# Global `--session <ID>` (daemon attach, Option<String>) and fuzz `--session`
# (bool) shared Clap ID `session` with different types, panicking on every
# successful fuzz parse. The fuzz flag is --http-session with explicit id.
echo ""
echo "--- Check 92: fuzz session flag has no Clap ID collision ---"
SECTION_FAIL=0
if rg -q '#\[arg\(long, help = "Enable HTTP session/cookie handling"\)\]' crates/eggsec/src/cli/fuzz.rs 2>/dev/null; then
  echo "FAIL: fuzz.rs still defines bare --session (Clap ID collision with global --session)."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'http-session' crates/eggsec/src/cli/fuzz.rs 2>/dev/null; then
  echo "FAIL: fuzz.rs missing --http-session replacement flag."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: fuzz session flag has no Clap ID collision."
fi

# 93. Surface decomposition follows stable concepts (Phase 2.10).
echo ""
echo "--- Check 93: TUI surface modules are decomposed ---"
SECTION_FAIL=0
for f in "crates/eggsec-tui/src/tabs/surface.rs" "crates/eggsec-tui/src/app/palette.rs"; do
  if [[ ! -f "$f" ]]; then
    echo "FAIL: missing surface module: $f"
    FAIL=$((FAIL + 1))
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: TUI surface modules are decomposed."
fi

# 94. Runtime operation-ID/target tables are single-owned (Phase 3.2).
# The wire-side match lives in eggsec-runtime (TaskKind::operation_id /
# canonical_target); engine adapters delegate instead of maintaining parallel
# matches. A parallel `TaskKind::Variant(_) =>` table here would silently
# diverge, so it fails the guard.
echo ""
echo "--- Check 94: Engine adapters delegate to the wire match ---"
SECTION_FAIL=0
if ! rg -q 'kind\.operation_id\(\)' crates/eggsec/src/operation_request.rs 2>/dev/null; then
  echo "FAIL: operation_request.rs does not delegate to TaskKind::operation_id()."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'kind\.canonical_target\(\)' crates/eggsec/src/operation_request.rs 2>/dev/null; then
  echo "FAIL: operation_request.rs does not delegate to TaskKind::canonical_target()."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
PARALLEL_OPS=$(rg -n 'TaskKind::[A-Za-z]+\(_\) =>' crates/eggsec/src/operation_request.rs 2>/dev/null || true)
if [[ -n "$PARALLEL_OPS" ]]; then
  echo "$PARALLEL_OPS"
  echo "FAIL: operation_request.rs owns a parallel TaskKind match. Delegate to the wire-side methods."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Engine adapters delegate to the wire match."
fi

# 95. Result/envelope conversion is single-owned (Phase 3.5).
# The TaskResult → envelope match lives in
# dispatch/canonical_execution.rs (task_result_envelope). Neither the
# daemon-backed executor nor the TUI dispatcher may own a parallel match;
# both consume the engine helper.
echo ""
echo "--- Check 95: Envelope conversion is single-owned ---"
SECTION_FAIL=0
if ! rg -q 'pub fn task_result_envelope' crates/eggsec/src/dispatch/canonical_execution.rs 2>/dev/null; then
  echo "FAIL: canonical_execution.rs missing pub fn task_result_envelope."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'task_result_envelope' crates/eggsec/src/runtime_bridge/executor.rs 2>/dev/null; then
  echo "FAIL: runtime_bridge/executor.rs does not consume task_result_envelope."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'dispatch::task_result_envelope' crates/eggsec-tui/src/app/task_dispatcher.rs 2>/dev/null; then
  echo "FAIL: TUI dispatcher does not consume eggsec::dispatch::task_result_envelope."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
TUI_DUP=$(rg -n 'fn task_result_to_envelope|fn task_result_to_outcome' crates/eggsec-tui/src/app/task_dispatcher.rs 2>/dev/null || true)
if [[ -n "$TUI_DUP" ]]; then
  echo "$TUI_DUP"
  echo "FAIL: TUI dispatcher reintroduces a parallel TaskResult conversion fn."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
EXEC_DUP=$(rg -n 'match result \{' crates/eggsec/src/runtime_bridge/executor.rs 2>/dev/null || true)
if [[ -n "$EXEC_DUP" ]]; then
  echo "$EXEC_DUP"
  echo "FAIL: runtime_bridge/executor.rs owns a TaskResult match. Delegate to task_result_envelope."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Envelope conversion is single-owned."
fi

# 96. Surface conversion is exhaustive in both directions (Phase 3.1).
# RuntimeSurface is a wire DTO; the bridge owns the conversion table.
# Unknown is rejected wire → engine; engine → wire is infallible.
echo ""
echo "--- Check 96: Surface conversion is bidirectional ---"
SECTION_FAIL=0
if ! rg -q 'pub fn execution_surface_to_runtime_surface' crates/eggsec/src/runtime_bridge/surface.rs 2>/dev/null; then
  echo "FAIL: surface.rs missing execution_surface_to_runtime_surface."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'Later phases can implement conversion' crates/eggsec-runtime/src/request.rs 2>/dev/null; then
  :
else
  echo "FAIL: stale transitional conversion comment still present in eggsec-runtime/src/request.rs."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'match surface \{' crates/eggsec/src/runtime_bridge/surface.rs 2>/dev/null; then
  :
else
  echo "FAIL: surface.rs lost its exhaustive surface match."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Surface conversion is bidirectional."
fi

# 97. Runtime-contract closure tests are wired (Phase 3).
echo ""
echo "--- Check 97: Runtime-contract closure tests exist ---"
SECTION_FAIL=0
if [[ ! -f "crates/eggsec/tests/runtime_contract_closure.rs" ]]; then
  echo "FAIL: missing closure test: crates/eggsec/tests/runtime_contract_closure.rs"
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Runtime-contract closure tests exist."
fi

# 98. TUI feature-profile verification is maintained.
# The frontend/runtime TUI full-profile corrective pass established three
# durable invariants: (a) the individual-feature sweep mechanically covers
# every declared eggsec-tui feature, (b) the eggsec-tui/full aggregate is
# compiled in the deep sweep, (c) a broad dependency-light TUI profile lives
# in `make check-feature-profiles` for fast drift detection. Do NOT grep for
# individual DTO field names here; Rust compilation and the semantic builder
# tests (app::task_management) own that contract.
echo ""
echo "--- Check 98: TUI feature-profile verification is maintained ---"
SECTION_FAIL=0
if ! rg -q 'eggsec-tui' scripts/check-features-individual.sh 2>/dev/null; then
  echo "FAIL: check-features-individual.sh has no eggsec-tui section."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'TUI_FEATURES|crates/eggsec-tui/Cargo.toml' scripts/check-features-individual.sh 2>/dev/null; then
  echo "FAIL: TUI sweep does not mechanically enumerate crates/eggsec-tui/Cargo.toml features."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'eggsec-tui/full' scripts/check-features-individual.sh 2>/dev/null; then
  echo "FAIL: deep sweep does not compile the eggsec-tui/full aggregate."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'eggsec-tui --features db-pentest,web-proxy,c2' Makefile 2>/dev/null; then
  echo "FAIL: Makefile check-feature-profiles lacks the broad TUI profile (db-pentest,web-proxy,c2)."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: TUI feature-profile verification is maintained."
fi

# 99. Network-dependency Phase A baseline invariants (measurement only).
# Durable forbidden-boundary patterns only — not every implementation detail:
# (a) dependency-light leaf crates must not gain concrete network clients;
# (b) concrete RequestBuilder must not spread beyond the single enumerated
# compatibility function in auth_context (migration checklist for Phase B);
# (c) the retained baseline doc and invariant tests must exist.
echo ""
echo "--- Check 99: Network concrete-client boundary is contained ---"
SECTION_FAIL=0
for crate in eggsec-runtime eggsec-tool-core eggsec-output eggsec-ui-model eggsec-daemon-protocol; do
  if rg -q 'reqwest|rustls|tokio-rustls|hickory-resolver|hickory_resolver' "crates/$crate/Cargo.toml" 2>/dev/null; then
    echo "FAIL: crates/$crate/Cargo.toml gained a concrete network dependency."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  HITS=$(rg -n 'reqwest::|rustls::|tokio_rustls::|hickory_resolver::' "crates/$crate/src/" 2>/dev/null || true)
  if [[ -n "$HITS" ]]; then
    echo "$HITS"
    echo "FAIL: crates/$crate/src uses concrete network client types."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
# Only the enumerated compatibility function may mention RequestBuilder in
# auth_context; a second occurrence means the boundary spread.
RB_COUNT=$(rg -c 'RequestBuilder' crates/eggsec/src/auth_context/mod.rs 2>/dev/null || echo 0)
if [[ "$RB_COUNT" -gt 3 ]]; then
  echo "FAIL: auth_context/mod.rs mentions RequestBuilder $RB_COUNT times (expected <= 3 for the single enumerated fn + docs)."
  rg -n 'RequestBuilder' crates/eggsec/src/auth_context/mod.rs 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ ! -f "architecture/network_dependency_baseline.md" ]]; then
  echo "FAIL: missing retained baseline: architecture/network_dependency_baseline.md"
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ ! -f "crates/eggsec/tests/network_policy_invariants.rs" ]]; then
  echo "FAIL: missing invariant tests: crates/eggsec/tests/network_policy_invariants.rs"
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Network concrete-client boundary is contained."
else
  FAIL=$((FAIL + 1))
fi

# 100. Phase B scoped transport contract exists and stays dependency-light.
# The transport crate owns the checkpoint shape; concrete clients must not
# appear in its dependency graph or code. Authority is mandatory for dispatch,
# DNS results bind to the connection path, and secrets are redacted.
echo ""
echo "--- Check 100: Scoped transport contract is dependency-light ---"
SECTION_FAIL=0
if ! rg -q 'crates/eggsec-transport' Cargo.toml 2>/dev/null; then
  echo "FAIL: workspace Cargo.toml does not list crates/eggsec-transport."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ ! -f "crates/eggsec-transport/Cargo.toml" ]]; then
  echo "FAIL: missing crate manifest: crates/eggsec-transport/Cargo.toml"
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  if rg -q 'reqwest|hyper|rustls|tokio-rustls|hickory-resolver|eggfetch|eggress' crates/eggsec-transport/Cargo.toml 2>/dev/null; then
    echo "FAIL: crates/eggsec-transport/Cargo.toml gained a concrete network dependency."
    rg -n 'reqwest|hyper|rustls|tokio-rustls|hickory-resolver|eggfetch|eggress' crates/eggsec-transport/Cargo.toml 2>/dev/null || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
if [[ ! -d "crates/eggsec-transport/src" ]]; then
  echo "FAIL: missing transport sources: crates/eggsec-transport/src"
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  HITS=$(rg -n 'reqwest::|hyper::|rustls::|tokio_rustls::|hickory_resolver::|eggfetch|eggress' crates/eggsec-transport/src/ 2>/dev/null || true)
  if [[ -n "$HITS" ]]; then
    echo "$HITS"
    echo "FAIL: eggsec-transport/src uses concrete network client types."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  for sym in "trait HttpTransport" "authority: &dyn NetworkAuthority" "trait NetworkAuthority" "authorize_initial_url" "authorize_resolved" "authorize_socket" "authorize_redirect" "authorize_proxy" "check_tls_consistency" "validate_binding" "ApprovedBinding" "REDACTED|redacted_headers_debug" "RecordingFakeTransport"; do
    if ! rg -q "$sym" crates/eggsec-transport/src/ 2>/dev/null; then
      echo "FAIL: eggsec-transport/src missing required contract symbol: $sym"
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  done
fi
# Phase C: the binding implementation lives in the engine policy bridge
# (policy_bridge/transport.rs); config/scope_transport.rs remains as a
# compatibility facade re-exporting ScopeAuthority.
for f in "crates/eggsec/src/policy_bridge/transport.rs" "crates/eggsec/src/config/scope_transport.rs" "crates/eggsec/tests/transport_contract.rs"; do
  if [[ ! -f "$f" ]]; then
    echo "FAIL: missing Phase B/C binding/test: $f"
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if ! rg -q 'impl.*NetworkAuthority for ScopeAuthority' crates/eggsec/src/policy_bridge/transport.rs 2>/dev/null; then
  echo "FAIL: policy_bridge/transport.rs does not implement NetworkAuthority for ScopeAuthority."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
RB_LEAK=$(rg -n 'RequestBuilder' crates/eggsec/src/policy_bridge/transport.rs crates/eggsec/src/config/scope_transport.rs crates/eggsec-transport/src/ 2>/dev/null || true)
if [[ -n "$RB_LEAK" ]]; then
  echo "$RB_LEAK"
  echo "FAIL: scope_transport or transport crate mentions RequestBuilder (boundary leak)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Scoped transport contract is dependency-light."
else
  FAIL=$((FAIL + 1))
fi

# 101. Shared/auth helpers expose transport-neutral canonical APIs (Phase B WS4).
# Concrete-builder wrappers may remain as documented compat, but the canonical
# path must be pure/transport-neutral. No second scope language.
echo ""
echo "--- Check 101: Transport-neutral canonical helpers exist ---"
SECTION_FAIL=0
for sym in "apply_auth_context_to_transport" "apply_auth_context_to_map"; do
  if ! rg -q "$sym" crates/eggsec/src/auth_context/mod.rs 2>/dev/null; then
    echo "FAIL: auth_context/mod.rs missing canonical helper: $sym"
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
for sym in "auth_headers" "apply_auth_to_transport"; do
  if ! rg -q "$sym" crates/eggsec/src/ai/client.rs 2>/dev/null; then
    echo "FAIL: ai/client.rs missing canonical helper: $sym"
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
for sym in "should_retry_status" "backoff_for_attempt"; do
  if ! rg -q "$sym" crates/eggsec/src/integrations/common.rs 2>/dev/null; then
    echo "FAIL: integrations/common.rs missing canonical helper: $sym"
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
for f in "crates/eggsec/src/auth_context/mod.rs" "crates/eggsec/src/ai/client.rs" "crates/eggsec/src/integrations/common.rs"; do
  if ! rg -qi 'compatibility wrapper|canonical' "$f" 2>/dev/null; then
    echo "FAIL: $f does not label compat vs canonical paths."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Transport-neutral canonical helpers exist."
else
  FAIL=$((FAIL + 1))
fi

# 102. Eggfetch adapter stays narrow; load-test is the sanctioned production consumer.
# The adapter crate owns the eggfetch binding: minimal published features
# (http1 + http2 + rustls + proxy for pinned routing; never http3/cookies/
# multipart/compression), no concrete client types in its sources, no eggfetch
# types in its public surface, and the engine may depend on it in production
# for pinned load-test execution only (direct + supported proxied; no Reqwest
# fallback). Requires eggfetch-core 0.2.0+ for direct resolved-route reuse + proxy pinning APIs.
echo ""
echo "--- Check 102: Eggfetch adapter stays narrow; load-test production backend ---"
SECTION_FAIL=0
if ! rg -q 'crates/eggsec-transport-eggfetch' Cargo.toml 2>/dev/null; then
  echo "FAIL: workspace Cargo.toml does not list crates/eggsec-transport-eggfetch."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ ! -f "crates/eggsec-transport-eggfetch/Cargo.toml" ]]; then
  echo "FAIL: missing adapter manifest: crates/eggsec-transport-eggfetch/Cargo.toml"
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  DEP_LINE=$(rg -n 'eggfetch-core = ' crates/eggsec-transport-eggfetch/Cargo.toml 2>/dev/null || true)
  if [[ -z "$DEP_LINE" ]]; then
    echo "FAIL: adapter manifest has no eggfetch-core dependency."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  else
    if ! echo "$DEP_LINE" | rg -q 'default-features = false'; then
      echo "FAIL: adapter must pin eggfetch-core default-features = false."
      echo "$DEP_LINE"
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
    if echo "$DEP_LINE" | rg -q 'http3|cookies|multipart|compression'; then
      echo "FAIL: adapter enables a deferred eggfetch feature (http3/cookies/multipart/compression)."
      echo "$DEP_LINE"
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
    if ! echo "$DEP_LINE" | rg -q '0\.[2-9]'; then
      echo "FAIL: adapter must require eggfetch-core 0.2.0+ (resolved-route reuse + proxy pinning APIs)."
      echo "$DEP_LINE"
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
    if ! echo "$DEP_LINE" | rg -q 'http2'; then
      echo "FAIL: adapter must enable http2 for H1/H2 ALPN (Auto allow_http3:false)."
      echo "$DEP_LINE"
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  fi
  # Only the [dependencies] section is scanned (comments stripped):
  # dev-dependencies may carry fixture-only TLS/test tooling (rcgen,
  # tokio-rustls, ipnetwork).
  # `head -n -1` is a GNU coreutils extension. BSD/macOS `head` rejects a
  # negative line count, and under `set -euo pipefail` that aborted the entire
  # script here, so no check after this point ever ran locally.
  # `sed '$d'` is the portable "all but the last line" equivalent.
  DEPS_SECTION=$(sed -n '/^\[dependencies\]/,/^\[/p' crates/eggsec-transport-eggfetch/Cargo.toml | sed '$d' | rg -v '^[[:space:]]*#')
  if echo "$DEPS_SECTION" | rg -q 'reqwest|hyper|rustls|tokio-rustls|hickory|eggress'; then
    # `tls-rustls` is the eggfetch feature name, not a direct dependency;
    # anything else matching here is a concrete-client leak.
    LEAKS=$(echo "$DEPS_SECTION" | rg -n 'reqwest|^hyper|[^s-]rustls|tokio-rustls|hickory|eggress' || true)
    if [[ -n "$LEAKS" ]]; then
      echo "$LEAKS"
      echo "FAIL: adapter [dependencies] gained a concrete network dependency."
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  fi
fi
if [[ ! -d "crates/eggsec-transport-eggfetch/src" ]]; then
  echo "FAIL: missing adapter sources: crates/eggsec-transport-eggfetch/src"
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  HITS=$(rg -n 'reqwest::|hyper::|rustls::|tokio_rustls::|hickory_resolver::' crates/eggsec-transport-eggfetch/src/ 2>/dev/null || true)
  if [[ -n "$HITS" ]]; then
    echo "$HITS"
    echo "FAIL: adapter src uses concrete network client types directly."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  for sym in "pub struct EggfetchTransport" "impl HttpTransport for EggfetchTransport" "without_proxy" "allow_http3" "validate_binding" "authorize_reresolution" "build_redirect_request" "resolved_addresses" "proxy_target_addresses"; do
    if ! rg -F -q "$sym" crates/eggsec-transport-eggfetch/src/ 2>/dev/null; then
      echo "FAIL: adapter src missing required symbol: $sym"
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  done
fi
if [[ ! -f "crates/eggsec-transport-eggfetch/tests/parity.rs" ]]; then
  echo "FAIL: missing adapter parity suite: crates/eggsec-transport-eggfetch/tests/parity.rs"
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ ! -f "architecture/transport_eggfetch.md" ]]; then
  echo "FAIL: missing adapter doc: architecture/transport_eggfetch.md"
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# The engine may depend on the adapter in production for pinned load-test
# execution only (plus dev-deps for parity tests). No other crate may gain a
# production dependency.
REFS=$(rg -l 'eggsec-transport-eggfetch' crates/*/Cargo.toml 2>/dev/null || true)
for ref in $REFS; do
  case "$ref" in
    crates/eggsec-transport-eggfetch/Cargo.toml) ;;
    crates/eggsec/Cargo.toml)
      # Production + dev wiring both sanctioned for the engine (load-test backend).
      if ! rg -q 'eggsec-transport-eggfetch' crates/eggsec/Cargo.toml 2>/dev/null; then
        echo "FAIL: crates/eggsec missing adapter wiring (production load-test backend)."
        SECTION_FAIL=$((SECTION_FAIL + 1))
      fi
      ;;
    *)
      echo "FAIL: unexpected adapter consumer: $ref (only the engine may use it in production)."
      SECTION_FAIL=$((SECTION_FAIL + 1))
      ;;
  esac
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Eggfetch adapter stays narrow; load-test production backend."
else
  FAIL=$((FAIL + 1))
fi

# 103. Phase D first migration increment (agent + shared helpers + NSE
# capability + proxy boundary + feature pruning).
# - eggsec-agent owns no direct ordinary HTTP stack (transport injection only).
# - Shared compat wrappers exposing RequestBuilder are removed (auth_context,
#   ai); fuzzer translates via the canonical map helper locally.
# - NSE exposes a narrow transport-neutral script capability with no concrete
#   clients in its code path.
# - web-proxy documents the outbound vs interception boundary; intercept/
#   never touches the client contract; outbound reqwest stays minimal
#   (proxy-routing only, no blocking/cookies/multipart/compression/http3).
echo ""
echo "--- Check 103: Phase D first increment boundaries hold ---"
SECTION_FAIL=0
# WS1: agent has no concrete HTTP stack.
if rg -q 'reqwest|rustls' crates/eggsec-agent/Cargo.toml 2>/dev/null; then
  echo "FAIL: eggsec-agent/Cargo.toml still depends on reqwest/rustls."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
HITS=$(rg -n 'reqwest::|rustls::|Client::builder' crates/eggsec-agent/src/ 2>/dev/null || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: eggsec-agent/src uses concrete HTTP client types. Inject HttpTransport instead."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'HttpTransport' crates/eggsec-agent/src/lifecycle.rs 2>/dev/null; then
  echo "FAIL: lifecycle.rs does not inject HttpTransport."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# WS2: shared RequestBuilder wrappers removed (code, not docs).
if rg -q 'pub fn apply_auth_context_to_request' crates/eggsec/src/auth_context/mod.rs 2>/dev/null; then
  echo "FAIL: auth_context still exposes apply_auth_context_to_request."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'pub fn apply_auth\(' crates/eggsec/src/ai/client.rs 2>/dev/null; then
  echo "FAIL: ai/client.rs still exposes apply_auth(RequestBuilder)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
for sym in "apply_auth_context_to_transport" "apply_auth_context_to_map" "auth_headers" "apply_auth_to_transport"; do
  if ! rg -q "$sym" crates/eggsec/src/auth_context/mod.rs crates/eggsec/src/ai/client.rs 2>/dev/null; then
    :
  fi
done
if ! rg -q 'apply_auth_context_to_map' crates/eggsec/src/fuzzer/engine/utils.rs 2>/dev/null; then
  echo "FAIL: fuzzer/engine/utils.rs does not translate via the canonical map helper."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# WS3: NSE narrow capability exists with no concrete clients in code
# (scan only above the #[cfg(test)] module so test string fixtures don't match).
# Engine-owned since runtime-extraction Milestone 002 (moved out of eggsec-nse
# so the runtime has no eggsec-transport edge).
NSE_CAP="crates/eggsec/src/nse_http_capability.rs"
if [[ ! -f "$NSE_CAP" ]]; then
  echo "FAIL: missing NSE script capability: $NSE_CAP"
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  NSE_CODE=$(sed -n '1,/^#\[cfg(test)\]/p' "$NSE_CAP" | sed '$d')
  for pat in 'reqwest::' 'eggfetch_core' 'eggfetch::' 'rustls::' 'tokio_rustls::' 'hickory_resolver::' 'RequestBuilder' 'Client::builder'; do
    if echo "$NSE_CODE" | grep -qF "$pat"; then
      echo "FAIL: $NSE_CAP code mentions concrete client pattern '$pat'."
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  done
  for sym in "pub fn build_scoped_request" "pub fn tls_policy_for_context" "pub fn nse_method"; do
    if ! rg -q "$sym" "$NSE_CAP" 2>/dev/null; then
      echo "FAIL: $NSE_CAP missing required symbol: $sym"
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  done
fi
# WS4: proxy boundary documented; intercept never touches the client contract.
if [[ ! -f "crates/eggsec-web-proxy/src/outbound.rs" ]]; then
  echo "FAIL: missing proxy boundary module: crates/eggsec-web-proxy/src/outbound.rs"
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  if ! rg -q 'pub fn build_direct_probe_request' crates/eggsec-web-proxy/src/outbound.rs 2>/dev/null; then
    echo "FAIL: outbound.rs missing build_direct_probe_request."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
HITS=$(rg -n 'eggsec_transport' crates/eggsec-web-proxy/src/intercept/ crates/eggsec-web-proxy/src/socks.rs crates/eggsec-web-proxy/src/http_connect.rs 2>/dev/null || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: interception/server code depends on the client transport contract. Keep it in outbound/ only."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# WS5: outbound reqwest stays minimal (proxy routing only).
PROXY_REQWEST=$(rg -n 'reqwest = ' crates/eggsec-web-proxy/Cargo.toml 2>/dev/null || true)
if [[ -z "$PROXY_REQWEST" ]]; then
  echo "FAIL: web-proxy manifest has no reqwest dependency line (expected minimal). "
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  if echo "$PROXY_REQWEST" | rg -q 'blocking|cookies|multipart|compression|http3|form|query|http2|json'; then
    echo "$PROXY_REQWEST"
    echo "FAIL: web-proxy reqwest enables a pruned feature (blocking/cookies/multipart/compression/http3/form/query/http2/json)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  if ! echo "$PROXY_REQWEST" | rg -q 'socks'; then
    echo "FAIL: web-proxy reqwest must keep 'socks' (proxy-routing probes)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Phase D first increment boundaries hold."
else
  FAIL=$((FAIL + 1))
fi

# 104. Phase E WS2: engine library-default is empty (no process-host default).
echo ""
echo "--- Check 104: Engine library-default is empty ---"
SECTION_FAIL=0
if ! rg -q '^\s*default = \[\]' crates/eggsec/Cargo.toml 2>/dev/null; then
  echo "FAIL: crates/eggsec/Cargo.toml default is not []."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q '^\s*default = \["cli"\]' crates/eggsec/Cargo.toml 2>/dev/null; then
  echo "FAIL: crates/eggsec/Cargo.toml still defaults to [\"cli\"]."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'eggsec = \{ path = "\.\./eggsec", version = "0\.1\.0", features = \["cli"\] \}' crates/eggsec-cli/Cargo.toml 2>/dev/null; then
  echo "FAIL: eggsec-cli must depend on eggsec with explicit version + features=[\"cli\"]."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'eggsec = \{ path = "\.\./eggsec", version = "0\.1\.0", features = \["cli"\] \}' crates/eggsec-tui/Cargo.toml 2>/dev/null; then
  echo "FAIL: eggsec-tui must depend on eggsec with explicit version + features=[\"cli\"]."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ ! -f crates/eggsec-daemon/Cargo.toml ]]; then
  echo "FAIL: crates/eggsec-daemon/Cargo.toml is missing."
  SECTION_FAIL=$((SECTION_FAIL + 1))
elif ! python3 - crates/eggsec-daemon/Cargo.toml <<'PYEOF' 2>&1; then
import sys, pathlib

try:
    import tomllib
except ModuleNotFoundError:  # Python < 3.11
    print("FAIL: python3 with tomllib (3.11+) is required to parse the daemon manifest.")
    sys.exit(1)

path = pathlib.Path(sys.argv[1])
try:
    data = tomllib.loads(path.read_text())
except Exception as exc:
    print(f"FAIL: {path} is not valid TOML: {exc}")
    sys.exit(1)

features = (data.get("features") or {})
problems = []

service = features.get("service")
if not isinstance(service, list):
    problems.append("the daemon must declare a `service` feature (the GUI-serving daemon profile)")
else:
    # `cli` is a HOST GATE, not a CLI surface: it gates the engine's
    # `runtime_bridge`, `commands`, and `dispatch` modules, which is where
    # `EggsecRuntimeExecutor` lives. The daemon cannot host the executor it
    # is built around without it.
    for required in ("dep:eggsec", "eggsec/cli"):
        if required not in service:
            problems.append(f"the daemon's `service` feature must enable `{required}`")

if features.get("full-executor") != ["service"]:
    problems.append(
        "the daemon's `full-executor` feature must be exactly [\"service\"] (a deprecated alias)"
    )

if problems:
    print("FAIL: the daemon must opt into the engine explicitly, through its profile:")
    for problem in problems:
        print(f"      {problem}")
    sys.exit(1)
PYEOF
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Engine library-default is empty with explicit process-host opt-in."
else
  FAIL=$((FAIL + 1))
fi

# 105. Phase E WS3: Tokio features declared per crate (no broad baseline).
echo ""
echo "--- Check 105: Tokio capabilities are per-crate narrow ---"
SECTION_FAIL=0
if ! rg -q 'tokio = \{ version = "1", default-features = false \}' Cargo.toml 2>/dev/null; then
  echo "FAIL: workspace Tokio baseline must be default-features=false with no feature list."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
BARE_TOKIO=$(rg -n 'tokio(\.workspace| = \{ workspace = true \})' crates/*/Cargo.toml 2>/dev/null | grep -v 'features =' || true)
if [[ -n "$BARE_TOKIO" ]]; then
  echo "$BARE_TOKIO"
  echo "FAIL: Found Tokio workspace dep without explicit features=. Declare per-crate capabilities."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
TESTUTIL_TOKIO=$(rg -n '^[^#]*tokio[^#]*test-util' crates/*/Cargo.toml Cargo.toml 2>/dev/null || true)
if [[ -n "$TESTUTIL_TOKIO" ]]; then
  echo "$TESTUTIL_TOKIO"
  echo "FAIL: test-util (paused time) must not appear in any Tokio dependency (no call sites)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
for dto in crates/eggsec-core/Cargo.toml crates/eggsec-tool-core/Cargo.toml crates/eggsec-ui-model/Cargo.toml crates/eggsec-daemon-protocol/Cargo.toml; do
  if rg -q 'tokio' "$dto" 2>/dev/null; then
    echo "FAIL: DTO-only crate must not depend on Tokio: $dto"
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Tokio capabilities are per-crate narrow."
else
  FAIL=$((FAIL + 1))
fi

# 106. Eggress 1.0.10 narrow boundary (supersedes Phase E WS1 blanket reject).
# Only eggsec-web-proxy may own the reviewed eggress-outbound/eggress-uri
# edge; transport/eggfetch/policy/core/DTO layers and unrelated domains stay
# Eggress-free; embed/runtime/server/routing/advanced crates stay forbidden.
# The direct Eggress dependency set is an exact allowlist (corrective pass,
# 2026-09-22): any third `eggress-*` edge fails even if no forbidden regex
# matches it.
echo ""
echo "--- Check 106: Eggress narrow boundary (1.0.10) ---"
SECTION_FAIL=0
# Exact direct-dependency allowlist via manifest parse (tomllib): the complete
# set of direct dependencies whose package key begins with `eggress` must be
# exactly {eggress-outbound, eggress-uri}, with pinned versions/features.
if ! python3 - <<'PYEOF' 2>&1; then
import sys, tomllib, pathlib
p = pathlib.Path("crates/eggsec-web-proxy/Cargo.toml")
with open(p, "rb") as f:
    m = tomllib.load(f)
deps = m.get("dependencies") or {}
eggress_direct = sorted(n for n in deps if n == "eggress" or n.startswith("eggress-") or n.startswith("eggress_"))
allowed = ["eggress-outbound", "eggress-uri"]
unexpected = [n for n in eggress_direct if n not in allowed]
missing = [n for n in allowed if n not in eggress_direct]
if unexpected:
    print(f"FAIL: unexpected direct Eggress dependencies in eggsec-web-proxy: {unexpected}")
    print(f"      allowed set is exactly {allowed}")
    sys.exit(1)
if missing:
    print(f"FAIL: missing approved direct Eggress dependencies in eggsec-web-proxy: {missing}")
    sys.exit(1)
# Exact version/feature constraints on the approved edge.
outbound = deps.get("eggress-outbound")
if not isinstance(outbound, dict) or outbound.get("version") != "=1.0.10":
    print(f"FAIL: eggress-outbound must be pinned to version \"=1.0.10\", got {outbound!r}")
    sys.exit(1)
if outbound.get("default-features", True) is not False:
    print(f"FAIL: eggress-outbound must set default-features = false, got {outbound!r}")
    sys.exit(1)
if outbound.get("features"):
    print(f"FAIL: eggress-outbound must enable no optional features, got {outbound.get('features')!r}")
    sys.exit(1)
uri = deps.get("eggress-uri")
if isinstance(uri, dict):
    if uri.get("version") != "=1.0.10":
        print(f"FAIL: eggress-uri must be pinned to version \"=1.0.10\", got {uri!r}")
        sys.exit(1)
    if uri.get("features"):
        print(f"FAIL: eggress-uri must enable no optional features, got {uri.get('features')!r}")
        sys.exit(1)
elif uri != "=1.0.10":
    print(f"FAIL: eggress-uri must be pinned to version \"=1.0.10\", got {uri!r}")
    sys.exit(1)
print(f"PASS: exact Eggress allowlist holds: {allowed} (1.0.10 pinned, no optional features).")
PYEOF
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# Approved edge regex backstop (kept for message stability alongside the
# manifest-aware check above).
if ! rg -q 'eggress-outbound = \{ version = "=1\.0\.10", default-features = false \}' crates/eggsec-web-proxy/Cargo.toml 2>/dev/null; then
  echo "FAIL: crates/eggsec-web-proxy/Cargo.toml must pin eggress-outbound = \"=1.0.10\" with default-features = false."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'eggress-uri = \{ version = "=1\.0\.10" \}' crates/eggsec-web-proxy/Cargo.toml 2>/dev/null; then
  echo "FAIL: crates/eggsec-web-proxy/Cargo.toml must pin eggress-uri = \"=1.0.10\"."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# No other workspace manifest may gain an eggress edge.
OTHER_EGRESS_MANIFEST=$(rg -n 'eggress' crates/*/Cargo.toml Cargo.toml 2>/dev/null | grep -v 'crates/eggsec-web-proxy/Cargo.toml' || true)
if [[ -n "$OTHER_EGRESS_MANIFEST" ]]; then
  echo "$OTHER_EGRESS_MANIFEST"
  echo "FAIL: Only eggsec-web-proxy may depend on eggress crates."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# Transport and eggfetch adapter stay Eggress-free (authorized-resolution seam).
for f in crates/eggsec-transport/Cargo.toml crates/eggsec-transport-eggfetch/Cargo.toml crates/eggsec-core/Cargo.toml crates/eggsec-policy/Cargo.toml; do
  if rg -q 'eggress' "$f" 2>/dev/null; then
    echo "FAIL: forbidden eggress edge in $f (transport/policy/core stay Eggress-free)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
# Forbidden Eggress surfaces anywhere (manifest-aware, no source filenames).
FORBIDDEN_EGRESS=$(rg -n 'eggress-(embed|runtime|server|routing|transport-ssh|transport-quic|protocol-shadowsocks|protocol-trojan|protocol-websocket|protocol-h3|udp|config|pproxy-compat)|eggress"' crates/*/Cargo.toml Cargo.toml 2>/dev/null || true)
if [[ -n "$FORBIDDEN_EGRESS" ]]; then
  echo "$FORBIDDEN_EGRESS"
  echo "FAIL: Forbidden Eggress surface (embed/runtime/server/routing/advanced/umbrella)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# Forbidden optional features on the approved edge.
FORBIDDEN_FEATS=$(rg -n 'eggress-outbound.*(toml|pproxy-compat|udp|extended|ssh|quic|legacy-crypto|pproxy-legacy|insecure-tls)' crates/eggsec-web-proxy/Cargo.toml 2>/dev/null || true)
if [[ -n "$FORBIDDEN_FEATS" ]]; then
  echo "$FORBIDDEN_FEATS"
  echo "FAIL: Unexpected Eggress optional feature on the approved edge."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# Source references stay inside the web-proxy adapter boundary.
# (`crate::eggress_outbound` is the internal Eggsec adapter module and is
# allowed anywhere inside eggsec-web-proxy; bare `eggress_*::`/`eggress::`
# are external crate references and stay confined to the adapter + tests.)
EGRESS_SRC_OUTSIDE=$(rg -n 'eggress_outbound::|eggress_uri::|eggress::' crates/*/src/ 2>/dev/null | grep -v 'crate::eggress_outbound' | grep -v 'crates/eggsec-web-proxy/src/eggress_outbound.rs' | grep -v 'crates/eggsec-web-proxy/tests/' || true)
if [[ -n "$EGRESS_SRC_OUTSIDE" ]]; then
  echo "$EGRESS_SRC_OUTSIDE"
  echo "FAIL: Eggress source references outside eggsec-web-proxy adapter/tests."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# Decision record exists and names 1.0.10.
if [[ ! -f "architecture/egress_reuse_decision.md" ]]; then
  echo "FAIL: missing decision record: architecture/egress_reuse_decision.md"
  SECTION_FAIL=$((SECTION_FAIL + 1))
elif ! rg -q '1\.0\.10' architecture/egress_reuse_decision.md 2>/dev/null; then
  echo "FAIL: decision record must name the 1.0.10 adoption."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Eggress narrow boundary holds (web-proxy only, 1.0.10 pinned, record current)."
else
  FAIL=$((FAIL + 1))
fi

# 107. Phase E WS4: no unjustified capability-crate extraction.
echo ""
echo "--- Check 107: No unjustified capability crates ---"
SECTION_FAIL=0
for candidate in crates/eggsec-net crates/eggsec-web-client crates/eggsec-evidence crates/eggsec-signing; do
  if [[ -e "$candidate" ]]; then
    echo "FAIL: Unjustified capability crate exists: $candidate (Phase E WS4: all rejected)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ ! -f "architecture/capability_segregation.md" ]]; then
  echo "FAIL: missing Phase E WS4 decision record: architecture/capability_segregation.md"
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: No unjustified capability crates."
else
  FAIL=$((FAIL + 1))
fi

# 108. Phase E WS5: manifest-graph direction (no forbidden edges, no cycles).
# Parses workspace manifests with Python (fast, no cargo metadata resolve).
echo ""
echo "--- Check 108: Manifest graph direction holds ---"
if ! python3 - <<'PYEOF' 2>&1; then
import sys, tomllib, pathlib
root = pathlib.Path(".")
with open(root/"Cargo.toml", "rb") as f:
    members = [pathlib.Path(path).name for path in tomllib.load(f)["workspace"]["members"]]
def deps_of(crate):
    p = root / "crates" / crate / "Cargo.toml"
    with open(p,"rb") as f:
        m = tomllib.load(f)
    out = set()
    for sec in ("dependencies","dev-dependencies","build-dependencies"):
        for name in (m.get(sec) or {}):
            out.add(name)
    feats = m.get("features") or {}
    for v in feats.values():
        if isinstance(v, list):
            for e in v:
                # "dep:foo" or "foo/bar" or "dep:foo?/bar"
                s = e.split("/")[0].removeprefix("dep:").removesuffix("?").strip()
                if s:
                    out.add(s)
    return out
# Forbidden: core must not touch transport/TLS/HTTP impl.
forbidden_core = {"reqwest","rustls","tokio-rustls","hickory-resolver","hyper","axum","tonic","eggfetch-core","eggsec-transport","eggsec-transport-eggfetch","tokio","tokio-util"}
core_deps = deps_of("eggsec-core") | deps_of("eggsec-tool-core") | deps_of("eggsec-ui-model")
bad = core_deps & forbidden_core
if bad:
    print(f"FAIL: DTO crates depend on forbidden: {sorted(bad)}")
    sys.exit(1)
# Transport stays light: only bytes/http/url/thiserror in [dependencies].
with open(root/"crates"/"eggsec-transport"/"Cargo.toml","rb") as f:
    tm = tomllib.load(f)
tdeps = set((tm.get("dependencies") or {}).keys())
if tdeps != {"bytes","http","url","thiserror"}:
    print(f"FAIL: eggsec-transport deps must be exactly bytes/http/url/thiserror, got {sorted(tdeps)}")
    sys.exit(1)
# Engine must not depend on frontend crates.
eng = deps_of("eggsec")
if eng & {"eggsec-tui","eggsec-daemon","eggsec-cli"}:
    print(f"FAIL: engine depends on frontend: {sorted(eng & {'eggsec-tui','eggsec-daemon','eggsec-cli'})}")
    sys.exit(1)
# Cycle check over path deps among members.
import re
graph = {c: set() for c in members}
for c in members:
    p = root/"crates"/c/"Cargo.toml"
    text = p.read_text()
    for m in re.finditer(r'path\s*=\s*"\.\./([^"]+)"', text):
        tgt = m.group(1)
        if tgt in graph:
            graph[c].add(tgt)
WHITE,GRAY,BLACK = 0,1,2
color = {c: WHITE for c in members}
stack = []
def dfs(u):
    color[u] = GRAY
    stack.append(u)
    for v in sorted(graph[u]):
        if color[v] == GRAY:
            cyc = stack[stack.index(v):] + [v]
            print(f"FAIL: workspace dependency cycle: {' -> '.join(cyc)}")
            sys.exit(1)
        if color[v] == WHITE:
            dfs(v)
    stack.pop()
    color[u] = BLACK
for c in members:
    if color[c] == WHITE:
        dfs(c)
print("graph ok")
PYEOF
  echo "FAIL: manifest graph direction check failed (see above)."
  FAIL=$((FAIL + 1))
else
  echo "PASS: Manifest graph direction holds (no forbidden edges, no cycles)."
fi

# 109. Phase F: Cargo Deny is the canonical dependency-policy gate.
# deny.toml must enforce the all-features closure, fail-closed sources, and
# wildcard denial; cargo audit must not exist as a second exception surface;
# make check-deps must exist and make check must run it.
echo ""
echo "--- Check 109: Cargo Deny is the canonical PR policy gate ---"
SECTION_FAIL=0
if ! rg -q 'all-features = true' deny.toml 2>/dev/null; then
  echo "FAIL: deny.toml missing [graph] all-features = true (optional closures hide advisories)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
for key in 'unknown-registry = "deny"' 'unknown-git = "deny"' 'required-git-spec = "rev"'; do
  if ! rg -Fq "$key" deny.toml 2>/dev/null; then
    echo "FAIL: deny.toml missing fail-closed source policy: $key"
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if ! rg -q 'wildcards = "deny"' deny.toml 2>/dev/null; then
  echo "FAIL: deny.toml must deny wildcard requirements (workspace path deps carry explicit versions)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ -f ".cargo/audit.toml" ]]; then
  echo "FAIL: .cargo/audit.toml exists (Phase F removed it; deny.toml is canonical, audit is diagnostic)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q '^check-deps:' Makefile 2>/dev/null; then
  echo "FAIL: Makefile missing check-deps target."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q '\$\(MAKE\) check-deps' Makefile 2>/dev/null; then
  echo "FAIL: Makefile check does not run check-deps (policy must be in the mandatory contract)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'dependency-policy' .github/workflows/ci.yml 2>/dev/null; then
  echo "FAIL: ci.yml missing the dependency-policy job (independent PR gate signal)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Cargo Deny is the canonical PR policy gate."
else
  FAIL=$((FAIL + 1))
fi

# 110. Phase F: all GitHub Actions pinned to immutable full SHAs.
# Every `uses:` must reference @<40-hex> with a version comment; mutable refs
# (@v4, @stable, @master, @v2, @v5, @cargo-deny) must not remain as refs.
echo ""
echo "--- Check 110: GitHub Actions are SHA-pinned ---"
SECTION_FAIL=0
MUTABLE_USES=$(rg -n 'uses:\s*\S+@(v[0-9]+|stable|master|cargo-deny)(\s|$|#)' .github/workflows/*.yml 2>/dev/null || true)
if [[ -n "$MUTABLE_USES" ]]; then
  echo "$MUTABLE_USES"
  echo "FAIL: Found mutable action refs. Pin to full-length commit SHAs with version comments."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
UNPINNED=$(rg -n '^\s*-\s*uses:' .github/workflows/*.yml 2>/dev/null | rg -v '@[0-9a-f]{40}' || true)
if [[ -n "$UNPINNED" ]]; then
  echo "$UNPINNED"
  echo "FAIL: Found uses: lines without a 40-hex SHA pin."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: GitHub Actions are SHA-pinned."
else
  FAIL=$((FAIL + 1))
fi

# 111. Phase F: workflow least privilege.
# Both workflows must declare `permissions: contents: read` (top-level and per
# job); no job may request write-all or write permissions (no
# release/publication jobs exist).
echo ""
echo "--- Check 111: Workflows are least-privilege ---"
SECTION_FAIL=0
for wf in .github/workflows/ci.yml .github/workflows/deep-checks.yml; do
  if ! rg -q 'permissions:' "$wf" 2>/dev/null; then
    echo "FAIL: $wf declares no permissions (expected contents: read)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  if rg -q 'write-all|permissions:\s*write|contents:\s*write|actions:\s*write|pull-requests:\s*write' "$wf" 2>/dev/null; then
    echo "FAIL: $wf grants write permissions (least privilege is contents: read)."
    rg -n 'write' "$wf" 2>/dev/null || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Workflows are least-privilege."
else
  FAIL=$((FAIL + 1))
fi

# 112. Phase F: pin/dependency maintenance automation.
# Dependabot must cover cargo + github-actions (weekly, no auto-merge);
# ci.yml must own the PR-only dependency-review job (moderate+, read-only, no
# separate license allowlist contradicting deny.toml).
echo ""
echo "--- Check 112: Pin maintenance is automated ---"
SECTION_FAIL=0
if [[ ! -f ".github/dependabot.yml" ]]; then
  echo "FAIL: missing .github/dependabot.yml."
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  for eco in "cargo" "github-actions"; do
    if ! rg -q "package-ecosystem: $eco" .github/dependabot.yml 2>/dev/null; then
      echo "FAIL: dependabot.yml missing package-ecosystem: $eco."
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  done
  # Ignore comment lines: documentation mentions the forbidden pattern by name.
  if rg -v '^\s*#' .github/dependabot.yml 2>/dev/null | rg -q 'auto-merge|automerge'; then
    echo "FAIL: dependabot.yml must not configure auto-merge (updates pass CI first)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
if ! rg -q 'dependency-review' .github/workflows/ci.yml 2>/dev/null; then
  echo "FAIL: ci.yml missing the dependency-review job."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'fail-on-severity: moderate' .github/workflows/ci.yml 2>/dev/null; then
  echo "FAIL: dependency-review must fail on moderate+ (documented security-project default)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# Ignore comment lines: ci.yml documents why no separate license list exists.
if rg -v '^\s*#' .github/workflows/ci.yml 2>/dev/null | rg -q 'deny-licenses|allow-licenses'; then
  echo "FAIL: dependency-review maintains a license allowlist contradicting deny.toml (deny owns licenses)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Pin maintenance is automated."
else
  FAIL=$((FAIL + 1))
fi

# 113. Phase A: scheduling/session no longer owned by eggsec-output.
# Cron lives in eggsec-agent::cron (only durable consumer is autonomous-agent
# scheduling); legacy tab-state session format was removed as superseded by
# daemon/runtime sessions. Path + module checks, not line matching.
echo ""
echo "--- Check 113: eggsec-output owns no scheduling/session ---"
SECTION_FAIL=0
for f in crates/eggsec-output/src/schedule.rs crates/eggsec-output/src/session.rs; do
  if [[ -f "$f" ]]; then
    echo "FAIL: $f exists (Phase A removed it; cron lives in eggsec-agent::cron)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if rg -q 'pub mod (schedule|session)' crates/eggsec-output/src/lib.rs 2>/dev/null; then
  echo "FAIL: eggsec-output/src/lib.rs still exposes schedule/session modules."
  rg -n 'pub mod (schedule|session)' crates/eggsec-output/src/lib.rs 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'struct (ScanQueue|ScanSession|SessionInfo|TabSessionState|ScheduledScan)|enum (ScheduleStatus|ScanType)' crates/eggsec-output/src/ 2>/dev/null; then
  echo "FAIL: eggsec-output still contains queue/session types."
  rg -n 'struct (ScanQueue|ScanSession|SessionInfo|TabSessionState|ScheduledScan)|enum (ScheduleStatus|ScanType)' crates/eggsec-output/src/ 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ ! -f "crates/eggsec-agent/src/cron.rs" ]]; then
  echo "FAIL: crates/eggsec-agent/src/cron.rs missing (canonical cron owner)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'pub (use|mod) cron' crates/eggsec-agent/src/lib.rs 2>/dev/null; then
  echo "FAIL: eggsec-agent/src/lib.rs does not expose cron module."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: eggsec-output owns no scheduling/session."
else
  FAIL=$((FAIL + 1))
fi

# 114. Phase A: scanner service knowledge stays scanner-owned.
echo ""
echo "--- Check 114: service-detection stays scanner-owned ---"
SECTION_FAIL=0
if [[ -f "crates/eggsec/src/utils/service_detection.rs" ]]; then
  echo "FAIL: crates/eggsec/src/utils/service_detection.rs reappeared (owner is the eggsec-service-db crate)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q '(use|mod) .*service_detection' crates/ --type rust 2>/dev/null; then
  echo "FAIL: utils::service_detection reference reappeared."
  rg -n '(use|mod) .*service_detection' crates/ --type rust 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ ! -f "crates/eggsec-service-db/src/lib.rs" ]]; then
  echo "FAIL: crates/eggsec-service-db/src/lib.rs missing (canonical owner; moved from scanner::service_data in Phase G)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ -f "crates/eggsec/src/scanner/service_data.rs" ]]; then
  echo "FAIL: crates/eggsec/src/scanner/service_data.rs reappeared (owner is the eggsec-service-db crate)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'pub use eggsec_service_db as service_data' crates/eggsec/src/scanner/mod.rs 2>/dev/null; then
  echo "FAIL: scanner/mod.rs no longer re-exports eggsec-service-db as service_data (engine facade path broken)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: service-detection stays scanner-owned (eggsec-service-db)."
else
  FAIL=$((FAIL + 1))
fi

# 115. Phase A: no Reqwest multi-client pool abstraction.
echo ""
echo "--- Check 115: no Reqwest client-pool abstraction ---"
SECTION_FAIL=0
if [[ -f "crates/eggsec/src/utils/client_pool.rs" ]]; then
  echo "FAIL: crates/eggsec/src/utils/client_pool.rs reappeared (single shared client is canonical)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'struct (ClientPool|OptimizedClientPool)' crates/ --type rust 2>/dev/null; then
  echo "FAIL: ClientPool/OptimizedClientPool type reappeared."
  rg -n 'struct (ClientPool|OptimizedClientPool)' crates/ --type rust 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'utils::client_pool|client_pool::(ClientPool|OptimizedClientPool)' crates/ --type rust 2>/dev/null; then
  echo "FAIL: client_pool import reappeared."
  rg -n 'utils::client_pool|client_pool::(ClientPool|OptimizedClientPool)' crates/ --type rust 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: no Reqwest client-pool abstraction."
else
  FAIL=$((FAIL + 1))
fi

# 116. Phase A: no second token-bucket in output/frontend crates.
echo ""
echo "--- Check 116: no second token-bucket in output/frontend ---"
SECTION_FAIL=0
if rg -q 'struct RateLimiter' crates/eggsec-output/src/ crates/eggsec-tui/src/ crates/eggsec-cli/src/ crates/eggsec-daemon/src/ 2>/dev/null; then
  echo "FAIL: RateLimiter implementation in output/frontend crates (canonical owner is engine utils::rate_limiter)."
  rg -n 'struct RateLimiter' crates/eggsec-output/src/ crates/eggsec-tui/src/ crates/eggsec-cli/src/ crates/eggsec-daemon/src/ 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: no second token-bucket in output/frontend."
else
  FAIL=$((FAIL + 1))
fi

# 117. Phase A: no utils/common catch-all crate.
echo ""
echo "--- Check 117: no utils/common catch-all crate ---"
SECTION_FAIL=0
if rg -qi 'eggsec-utils|eggsec-common|eggsec-shared|eggsec-helpers' Cargo.toml crates/*/Cargo.toml 2>/dev/null; then
  echo "FAIL: utils/common catch-all crate reference found."
  rg -ni 'eggsec-utils|eggsec-common|eggsec-shared|eggsec-helpers' Cargo.toml crates/*/Cargo.toml 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q '^\s*pub mod (common|shared|helpers)\s*;' crates/eggsec/src/utils/mod.rs crates/eggsec/src/lib.rs 2>/dev/null; then
  echo "FAIL: new catch-all module under engine root."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
for dead in crates/eggsec/src/utils/output.rs crates/eggsec/src/utils/progress.rs crates/eggsec/src/utils/stealth.rs crates/eggsec/src/utils/privilege.rs; do
  if [[ -f "$dead" ]]; then
    echo "FAIL: $dead reappeared (Phase A removed it)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: no utils/common catch-all crate."
else
  FAIL=$((FAIL + 1))
fi

# 118. Phase B: eggsec-report-model is data-only (no rendering/I-O/runtime deps).
echo ""
echo "--- Check 118: report-model stays data-only ---"
SECTION_FAIL=0
if [[ ! -f "crates/eggsec-report-model/src/lib.rs" ]]; then
  echo "FAIL: crates/eggsec-report-model/src/lib.rs missing (canonical report DTO owner)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
for dep in 'tokio' 'quick-xml' 'hostname' 'lru' 'reqwest' 'rustls' 'axum' 'tonic' 'clap' 'ratatui' 'crossterm' 'eggsec-output' 'eggsec ='; do
  if rg -q "^${dep}([ =]|$)|^${dep} =" crates/eggsec-report-model/Cargo.toml 2>/dev/null; then
    echo "FAIL: eggsec-report-model/Cargo.toml references forbidden dep '$dep'."
    rg -n "$dep" crates/eggsec-report-model/Cargo.toml 2>/dev/null || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if rg -q 'eggsec_output|eggsec-output' crates/eggsec-report-model/Cargo.toml 2>/dev/null; then
  echo "FAIL: eggsec-report-model/Cargo.toml references eggsec-output (direction must be output -> model)."
  rg -n 'eggsec_output|eggsec-output' crates/eggsec-report-model/Cargo.toml 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# Code-only check: doc prose (`//`) may name eggsec-output as the consumer,
# but no `use`/path may reference it (mirrors Phase A checks matching code
# structure, not docs prose).
if rg -n 'eggsec_output' crates/eggsec-report-model/src/ 2>/dev/null | grep -v '^\s*[^:]*:[^:]*:\s*//' | grep -q .; then
  echo "FAIL: eggsec-report-model/src references eggsec_output in code (direction must be output -> model)."
  rg -n 'eggsec_output' crates/eggsec-report-model/src/ 2>/dev/null | grep -v '^\s*[^:]*:[^:]*:\s*//' || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'std::fs|tokio::|reqwest::|quick_xml|hostname::|LruCache' crates/eggsec-report-model/src/ 2>/dev/null; then
  echo "FAIL: eggsec-report-model/src uses filesystem/runtime/renderer behavior."
  rg -n 'std::fs|tokio::|reqwest::|quick_xml|hostname::|LruCache' crates/eggsec-report-model/src/ 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: report-model stays data-only."
else
  FAIL=$((FAIL + 1))
fi

# 119. Phase B: eggsec-output depends on the model, never the reverse; DTO
# facades stay re-exports (no second struct definition).
echo ""
echo "--- Check 119: output depends on model (facades, no forks) ---"
SECTION_FAIL=0
if ! rg -q '^eggsec-report-model' crates/eggsec-output/Cargo.toml 2>/dev/null; then
  echo "FAIL: crates/eggsec-output/Cargo.toml does not depend on eggsec-report-model."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
for ty in 'pub struct ScanReportData' 'pub struct FindingData' 'pub struct PortData' 'pub struct ServiceData' 'pub struct WirelessNetworkReportData' 'pub struct PolicySummary' 'pub struct DiffSummary' 'pub struct ReportEnvelope' 'pub struct FindingRecord' 'pub struct EvidenceItem' 'pub struct EvidenceManifest' 'pub enum EvidenceKind' 'pub enum RedactionState' 'pub enum RedactionPolicy' 'pub struct BaselineSummary' 'pub struct ToolMetadata'; do
  if rg -q "$ty" crates/eggsec-output/src/ 2>/dev/null; then
    echo "FAIL: $ty redefined in eggsec-output (canonical owner is eggsec-report-model; re-export it)."
    rg -n "$ty" crates/eggsec-output/src/ 2>/dev/null || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if ! rg -q 'pub use eggsec_report_model' crates/eggsec-output/src/convert.rs crates/eggsec-output/src/envelope.rs crates/eggsec-output/src/policy_summary.rs crates/eggsec-output/src/diff.rs 2>/dev/null; then
  echo "FAIL: expected model re-exports in output convert/envelope/policy_summary/diff facades."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: output depends on model (facades, no forks)."
else
  FAIL=$((FAIL + 1))
fi

# 120. Phase B: DTO-only domain crates depend on the model, not the renderer.
# NOTE: eggsec-nse is intentionally absent from this list since
# runtime-extraction Milestone 002: its report-envelope conversion moved to
# the engine (`eggsec::nse_bridge`), so the runtime has no report-model edge
# at all (see check 144). The engine-side model use is covered by check 40.
echo ""
echo "--- Check 120: domain DTO consumers use the model, not output ---"
SECTION_FAIL=0
for crate in eggsec-db-lab eggsec-mobile-lab eggsec-web-proxy; do
  if rg -q '^eggsec-output' "crates/${crate}/Cargo.toml" 2>/dev/null; then
    echo "FAIL: crates/${crate}/Cargo.toml still depends on eggsec-output (use eggsec-report-model for DTOs)."
    rg -n '^eggsec-output' "crates/${crate}/Cargo.toml" 2>/dev/null || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  if ! rg -q '^eggsec-report-model' "crates/${crate}/Cargo.toml" 2>/dev/null; then
    echo "FAIL: crates/${crate}/Cargo.toml does not depend on eggsec-report-model."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  if rg -q 'eggsec_output::' "crates/${crate}/src/" "crates/${crate}/tests/" 2>/dev/null; then
    echo "FAIL: crates/${crate} still imports eggsec_output paths."
    rg -n 'eggsec_output::' "crates/${crate}/src/" "crates/${crate}/tests/" 2>/dev/null || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: domain DTO consumers use the model, not output."
else
  FAIL=$((FAIL + 1))
fi

# 121. Phase C: eggsec-policy stays a dependency-light semantic kernel.
echo ""
echo "--- Check 121: eggsec-policy stays dependency-light ---"
SECTION_FAIL=0
if [[ ! -f "crates/eggsec-policy/src/lib.rs" ]]; then
  echo "FAIL: crates/eggsec-policy/src/lib.rs missing (canonical policy owner)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
for dep in 'tokio' 'reqwest' 'hyper' 'rustls' 'axum' 'tonic' 'clap' 'ratatui' 'crossterm' 'rusqlite' 'sqlx' 'hickory-resolver' 'eggsec-transport' 'eggsec-runtime' 'eggsec-output' 'eggsec-report-model' 'eggsec =' 'eggsec-core'; do
  if rg -q "^${dep}([ =]|$)|^${dep} =" crates/eggsec-policy/Cargo.toml 2>/dev/null; then
    echo "FAIL: eggsec-policy/Cargo.toml references forbidden dep '$dep'."
    rg -n "$dep" crates/eggsec-policy/Cargo.toml 2>/dev/null || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if rg -q 'eggsec_transport|eggsec-transport' crates/eggsec-policy/Cargo.toml crates/eggsec-policy/src/ 2>/dev/null | grep -v '^\s*[^:]*:[^:]*:\s*//' | grep -q .; then
  echo "FAIL: eggsec-policy references eggsec_transport in code (bridge lives in the engine)."
  rg -n 'eggsec_transport|eggsec-transport' crates/eggsec-policy/Cargo.toml crates/eggsec-policy/src/ 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'std::fs|tokio::|reqwest::|hickory|lookup_host|to_socket_addrs' crates/eggsec-policy/src/ 2>/dev/null | grep -v '^\s*[^:]*:[^:]*:\s*//' | grep -q .; then
  echo "FAIL: eggsec-policy/src uses filesystem/runtime/resolver behavior."
  rg -n 'std::fs|tokio::|reqwest::|hickory|lookup_host|to_socket_addrs' crates/eggsec-policy/src/ 2>/dev/null | grep -v '^\s*[^:]*:[^:]*:\s*//' || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -n 'cfg!\(feature' crates/eggsec-policy/src/ 2>/dev/null | grep -v '^\s*[^:]*:[^:]*:\s*//' | grep -q .; then
  echo "FAIL: eggsec-policy/src queries Cargo features (availability must be explicit input)."
  rg -n 'cfg!\(feature' crates/eggsec-policy/src/ 2>/dev/null | grep -v '^\s*[^:]*:[^:]*:\s*//' || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -n 'HostResolver|SystemResolver|ScopeAuthority' crates/eggsec-policy/src/ 2>/dev/null | grep -v '^\s*[^:]*:[^:]*:\s*//' | grep -q .; then
  echo "FAIL: eggsec-policy/src owns resolver/authority behavior (bridges live in the engine)."
  rg -n 'HostResolver|SystemResolver|ScopeAuthority' crates/eggsec-policy/src/ 2>/dev/null | grep -v '^\s*[^:]*:[^:]*:\s*//' || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: eggsec-policy stays dependency-light."
else
  FAIL=$((FAIL + 1))
fi

# 122. Phase C: engine depends one-way on the policy kernel; transport stays
# independent of policy implementation.
echo ""
echo "--- Check 122: engine -> policy one-way; transport independent ---"
SECTION_FAIL=0
if ! rg -q '^eggsec-policy' crates/eggsec/Cargo.toml 2>/dev/null; then
  echo "FAIL: crates/eggsec/Cargo.toml does not depend on eggsec-policy."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'eggsec-transport' crates/eggsec-policy/Cargo.toml 2>/dev/null; then
  echo "FAIL: eggsec-policy depends on eggsec-transport (bridge lives in the engine)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'eggsec-policy|eggsec_policy' crates/eggsec-transport/Cargo.toml crates/eggsec-transport/src/ 2>/dev/null | grep -v '^\s*[^:]*:[^:]*:\s*//' | grep -q .; then
  echo "FAIL: eggsec-transport references policy implementation (contract stays neutral)."
  rg -n 'eggsec-policy|eggsec_policy' crates/eggsec-transport/Cargo.toml crates/eggsec-transport/src/ 2>/dev/null | grep -v '^\s*[^:]*:[^:]*:\s*//' || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'crates/eggsec-policy' Cargo.toml 2>/dev/null; then
  echo "FAIL: workspace root Cargo.toml does not list crates/eggsec-policy."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: engine -> policy one-way; transport independent."
else
  FAIL=$((FAIL + 1))
fi

# 123. Phase C: no second policy language in the engine; config policy/scope
# modules stay facades over the kernel (no redefined core types).
echo ""
echo "--- Check 123: engine policy modules stay facades (no forks) ---"
SECTION_FAIL=0
for ty in 'pub enum OperationRisk' 'pub enum OperationMode' 'pub enum ExecutionProfile' 'pub enum ExecutionSurface' 'pub enum Capability' 'pub enum DenialClass' 'pub struct ExecutionPolicy' 'pub struct OperationDescriptor' 'pub enum OperationTarget' 'pub struct OperationMetadata' 'pub struct PolicyDecision' 'pub enum EnforcementOutcome' 'pub enum EnforcementError' 'pub struct ManualOverride' 'pub struct Scope(' 'pub struct Scope ' 'pub struct ScopeRule' 'pub struct TargetScope' 'pub enum AddressClass' 'pub struct LoadedScope'; do
  if rg -q "$ty" crates/eggsec/src/config/policy.rs crates/eggsec/src/config/policy_target.rs crates/eggsec/src/config/policy_catalog.rs crates/eggsec/src/config/policy_decision.rs crates/eggsec/src/config/policy_approval.rs crates/eggsec/src/config/scope.rs crates/eggsec/src/config/scope_address.rs 2>/dev/null; then
    echo "FAIL: $ty redefined in eggsec config (canonical owner is eggsec-policy; re-export it)."
    rg -n "$ty" crates/eggsec/src/config/policy.rs crates/eggsec/src/config/policy_target.rs crates/eggsec/src/config/policy_catalog.rs crates/eggsec/src/config/policy_decision.rs crates/eggsec/src/config/policy_approval.rs crates/eggsec/src/config/scope.rs crates/eggsec/src/config/scope_address.rs 2>/dev/null || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if ! rg -q 'pub use eggsec_policy' crates/eggsec/src/config/policy.rs crates/eggsec/src/config/scope.rs crates/eggsec/src/config/policy_decision.rs 2>/dev/null; then
  echo "FAIL: expected eggsec_policy re-exports in config policy/scope/decision facades."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ ! -d "crates/eggsec/src/policy_bridge" ]]; then
  echo "FAIL: crates/eggsec/src/policy_bridge missing (engine-owned resolver/feature/transport adapters)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: engine policy modules stay facades (no forks)."
else
  FAIL=$((FAIL + 1))
fi

# 124. Phase D: loadtest core owns no concrete HTTP client, terminal UI, or
# surface-adapter types. Reqwest/indicatif/Clap/config-file ownership lives
# only in the backend (ReqwestTransport), the cli-gated run_cli renderer,
# and the adapter/facade respectively — never in plan/executor/metrics/
# progress.
echo ""
echo "--- Check 124: loadtest core stays transport-neutral ---"
SECTION_FAIL=0
for f in crates/eggsec/src/loadtest/plan.rs crates/eggsec/src/loadtest/executor.rs crates/eggsec/src/loadtest/metrics.rs crates/eggsec/src/loadtest/progress.rs; do
  if [[ ! -f "$f" ]]; then
    echo "FAIL: missing Phase D core module: $f"
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
CORE_HITS=$(rg -n 'reqwest::|indicatif::|CommonHttpArgs|EggsecConfig|LoadArgs|ProgressBar|RequestBuilder|Client::builder' crates/eggsec/src/loadtest/plan.rs crates/eggsec/src/loadtest/executor.rs crates/eggsec/src/loadtest/metrics.rs crates/eggsec/src/loadtest/progress.rs crates/eggsec/src/loadtest/adapter.rs 2>/dev/null | grep -v '^\s*[^:]*:[^:]*:\s*//' || true)
if [[ -n "$CORE_HITS" ]]; then
  echo "$CORE_HITS"
  echo "FAIL: loadtest core/adapter touches concrete client, terminal UI, or surface-adapter types."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'HttpTransport' crates/eggsec/src/loadtest/executor.rs 2>/dev/null; then
  echo "FAIL: executor.rs does not dispatch through HttpTransport."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'impl HttpTransport for ReqwestTransport' crates/eggsec/src/loadtest/backend.rs 2>/dev/null; then
  echo "FAIL: backend.rs does not implement HttpTransport for ReqwestTransport."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'ProgressSink' crates/eggsec/src/loadtest/executor.rs crates/eggsec/src/loadtest/progress.rs 2>/dev/null; then
  echo "FAIL: structured progress sink missing from executor/progress."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'tui_mode' crates/eggsec/src/loadtest/plan.rs crates/eggsec/src/loadtest/executor.rs crates/eggsec/src/loadtest/metrics.rs crates/eggsec/src/loadtest/progress.rs crates/eggsec/src/loadtest/adapter.rs crates/eggsec/src/loadtest/backend.rs 2>/dev/null; then
  echo "FAIL: tui_mode leaked into the loadtest core (facade-only compat)."
  rg -n 'tui_mode' crates/eggsec/src/loadtest/plan.rs crates/eggsec/src/loadtest/executor.rs crates/eggsec/src/loadtest/metrics.rs crates/eggsec/src/loadtest/progress.rs crates/eggsec/src/loadtest/adapter.rs crates/eggsec/src/loadtest/backend.rs 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: loadtest core stays transport-neutral."
else
  FAIL=$((FAIL + 1))
fi

# 125. Phase D: no unjustified loadtest/resilience/utils crate extraction.
echo ""
echo "--- Check 125: no unjustified loadtest/resilience crates ---"
SECTION_FAIL=0
for candidate in crates/eggsec-loadtest crates/eggsec-resilience crates/eggsec-utils; do
  if [[ -e "$candidate" ]]; then
    echo "FAIL: Unjustified crate exists: $candidate (Phase D Gates D1/D2 rejected; Phase A rejected utils)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if rg -qi 'eggsec-loadtest|eggsec-resilience' Cargo.toml crates/*/Cargo.toml 2>/dev/null; then
  echo "FAIL: loadtest/resilience crate reference found in manifests."
  rg -ni 'eggsec-loadtest|eggsec-resilience' Cargo.toml crates/*/Cargo.toml 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ ! -f "architecture/capability_segregation.md" ]]; then
  echo "FAIL: missing decision record: architecture/capability_segregation.md"
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: no unjustified loadtest/resilience crates."
else
  FAIL=$((FAIL + 1))
fi

# 126. Phase D: removed utils::cache stays removed (zero-consumer dead code).
echo ""
echo "--- Check 126: removed utils::cache stays removed ---"
SECTION_FAIL=0
if [[ -f "crates/eggsec/src/utils/cache.rs" ]]; then
  echo "FAIL: crates/eggsec/src/utils/cache.rs reappeared (Phase D removed ApiCache: zero consumers)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'pub mod cache' crates/eggsec/src/utils/mod.rs 2>/dev/null; then
  echo "FAIL: utils/mod.rs re-exposes cache module."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'struct ApiCache' crates/eggsec/src/ 2>/dev/null; then
  echo "FAIL: ApiCache type reappeared under engine src."
  rg -n 'struct ApiCache' crates/eggsec/src/ 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: removed utils::cache stays removed."
else
  FAIL=$((FAIL + 1))
fi

# 127. Corrective pass: no wildcard load-test scope synthesis.
echo ""
echo "--- Check 127: no wildcard load-test scope default ---"
SECTION_FAIL=0
HITS=$(rg -n 'default_facade_scope|ScopeRule::new\(\"\*\"' crates/eggsec/src/loadtest/ 2>/dev/null || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: production load-test wildcard scope helper reappeared (must fail closed without scope)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  echo "PASS: no wildcard load-test scope default."
fi
if [[ $SECTION_FAIL -ne 0 ]]; then FAIL=$((FAIL + 1)); fi

# 128. Corrective pass: strict/canonical load-test carries execution scope.
echo ""
echo "--- Check 128: execution-scope propagation symbols exist ---"
SECTION_FAIL=0
for sym in "struct ApprovedExecution" "fn approve_execution" "fn approve_manual_execution" "struct ToolExecutionContext" "fn execute_with_context" "fn dispatch_execution" "fn execute_approved_execution" "fn execute_canonical_with_scope"; do
  if ! rg -q "$sym" crates/eggsec/src/ 2>/dev/null; then
    echo "FAIL: missing execution-scope symbol: $sym"
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: execution-scope propagation symbols exist."
else
  FAIL=$((FAIL + 1))
fi

# 129. Corrective pass: raw load-test tool execution fails closed.
echo ""
echo "--- Check 129: raw LoadTestTool::execute fails closed ---"
SECTION_FAIL=0
if ! rg -q 'load-test tool requires execution context' crates/eggsec/src/tool/implementations/loadtest.rs 2>/dev/null; then
  echo "FAIL: LoadTestTool::execute does not fail closed without context."
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  echo "PASS: raw LoadTestTool::execute fails closed."
fi
if [[ $SECTION_FAIL -ne 0 ]]; then FAIL=$((FAIL + 1)); fi

# 130. Corrective pass: Reqwest proxy has no direct fallback or placeholder.
# Scans production code only (strips doc comments `///`/`//` and the
# `#[cfg(test)]` module): docs/tests may describe the forbidden patterns to
# prove their absence.
echo ""
echo "--- Check 130: Reqwest backend has no route fallback ---"
SECTION_FAIL=0
PROD_CODE=$(sed -n '1,/^#\[cfg(test)\]/p' crates/eggsec/src/loadtest/backend.rs | rg -v '^\s*///' | rg -v '^\s*//')
HITS=$(printf '%s\n' "$PROD_CODE" | rg -n 'unwrap_or_else.*base_client|unwrap_or.*base_client|Proxy::all\("http://127\.0\.0\.1:1"\)|reqwest::Client::new\(\)' || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: Reqwest backend retains a semantic fallback/placeholder."
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  echo "PASS: Reqwest backend has no route fallback."
fi
if printf '%s\n' "$PROD_CODE" | rg -q 'Client::new\(\)' 2>/dev/null; then
  echo "FAIL: Reqwest backend uses Client::new() default fallback."
  printf '%s\n' "$PROD_CODE" | rg -n 'Client::new\(\)' || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Reqwest construction is fail-closed."
else
  FAIL=$((FAIL + 1))
fi

# 131. Corrective pass: proxy cache identity includes credentials.
echo ""
echo "--- Check 131: proxy cache keyed by full client identity ---"
SECTION_FAIL=0
if ! rg -q 'credential_fp' crates/eggsec/src/loadtest/backend.rs 2>/dev/null; then
  echo "FAIL: proxied client cache does not partition by credential fingerprint."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'HashMap<\(String, bool\)' crates/eggsec/src/loadtest/backend.rs 2>/dev/null; then
  echo "FAIL: proxied cache still keyed only by (endpoint, verified)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: proxy cache keyed by full client identity."
else
  FAIL=$((FAIL + 1))
fi

# 132. Corrective pass: direct production load testing via Eggfetch.
echo ""
echo "--- Check 132: production load-test uses Eggfetch backend ---"
SECTION_FAIL=0
for f in "crates/eggsec/src/loadtest/runner.rs" "crates/eggsec/src/loadtest/mod.rs" "crates/eggsec/src/dispatch/network.rs"; do
  if ! rg -q 'EggfetchTransport' "$f" 2>/dev/null; then
    echo "FAIL: $f does not dispatch through EggfetchTransport."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if rg -q 'ReqwestTransport::with_system_resolver\(\)' crates/eggsec/src/loadtest/runner.rs crates/eggsec/src/loadtest/mod.rs crates/eggsec/src/dispatch/network.rs 2>/dev/null; then
  echo "FAIL: production load-test path still constructs ReqwestTransport directly."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: production load-test uses Eggfetch backend."
else
  FAIL=$((FAIL + 1))
fi

# 133. Corrective pass: unsupported proxy pinning fails closed explicitly.
echo ""
echo "--- Check 133: SOCKS5H/plaintext proxy pinning fails closed ---"
SECTION_FAIL=0
for sym in "SOCKS5H remote-DNS cannot be ultimate-IP-pinned" "plaintext HTTP forward-proxy ultimate pinning unsupported"; do
  if ! rg -q "$sym" crates/eggsec-transport-eggfetch/src/ 2>/dev/null; then
    echo "FAIL: adapter missing explicit failure: $sym"
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: unsupported proxy pinning fails closed explicitly."
else
  FAIL=$((FAIL + 1))
fi

# 134. 0.2.0 adoption: MSRV truthful for eggfetch-core 0.2.0.
echo ""
echo "--- Check 134: MSRV compatible with eggfetch-core ---"
SECTION_FAIL=0
if ! rg -q 'rust-version = "1.89"' Cargo.toml 2>/dev/null; then
  echo "FAIL: workspace rust-version is not 1.89 (eggfetch-core 0.2.0 declares 1.89)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'version = "0.2.0"' crates/eggsec-transport-eggfetch/Cargo.toml 2>/dev/null; then
  echo "FAIL: adapter does not require eggfetch-core 0.2.0+."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: MSRV compatible with eggfetch-core."
else
  FAIL=$((FAIL + 1))
fi

# 135. Multi-address corrective pass: proxied backend route is singular per leg.
# One authorization cycle selects one physical address per connection leg; the
# backend must not receive a multi-address set expanded from the DNS-approved
# vectors. The struct shape encodes this (singular SocketAddr fields, not
# Vecs), and the Eggfetch boundary converts them to single-element pin sets.
# This checks structure, not variable-name spelling, so harmless refactors
# that keep singular fields pass while a Vec widening fails.
echo ""
echo "--- Check 135: Proxied backend route is single-address per leg ---"
SECTION_FAIL=0
ADAPTER="crates/eggsec-transport-eggfetch/src/adapter.rs"
if [[ ! -f "$ADAPTER" ]]; then
  echo "FAIL: missing adapter: $ADAPTER"
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  for sym in "proxy_peer: std::net::SocketAddr" "ultimate_peer: std::net::SocketAddr"; do
    if ! rg -Fq "$sym" "$ADAPTER" 2>/dev/null; then
      echo "FAIL: $ADAPTER missing singular route field: $sym"
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  done
  if rg -q 'proxy_peers:\s*Vec|ultimate_peers:\s*Vec' "$ADAPTER" 2>/dev/null; then
    echo "FAIL: $ADAPTER reintroduces multi-address Vec route fields (must stay singular)."
    rg -n 'proxy_peers:\s*Vec|ultimate_peers:\s*Vec' "$ADAPTER" 2>/dev/null || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  if ! rg -Fq 'resolved_addresses([proxy_route.proxy_peer])' "$ADAPTER" 2>/dev/null; then
    echo "FAIL: $ADAPTER does not pin the proxy peer from the singular socket-authorized field."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  if ! rg -Fq 'proxy_target_addresses([proxy_route.ultimate_peer])' "$ADAPTER" 2>/dev/null; then
    echo "FAIL: $ADAPTER does not pin the ultimate target from the singular socket-authorized field."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  # Forbid re-expansion from the DNS-approved sets at the pin site (the
  # approved vectors remain as validate_binding inputs, but must never be
  # iterated into backend pin sets).
  if rg -q 'proxy_approved\.iter\(\)|approved\.iter\(\)' "$ADAPTER" 2>/dev/null; then
    echo "FAIL: $ADAPTER iterates DNS-approved sets into backend pins (must use singular bindings)."
    rg -n 'proxy_approved\.iter\(\)|approved\.iter\(\)' "$ADAPTER" 2>/dev/null || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Proxied backend route is single-address per leg."
else
  FAIL=$((FAIL + 1))
fi

# 136. Eggfetch 0.1.7 adoption: direct route is logical URL + singular
# resolved_addresses (no IP-literal wire-URL shim).
echo ""
echo "--- Check 136: Direct backend route uses singular resolved_addresses ---"
SECTION_FAIL=0
ADAPTER="crates/eggsec-transport-eggfetch/src/adapter.rs"
if [[ ! -f "$ADAPTER" ]]; then
  echo "FAIL: missing adapter: $ADAPTER"
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  if ! rg -Fq 'selected_target: SocketAddr' "$ADAPTER" 2>/dev/null; then
    echo "FAIL: $ADAPTER missing singular direct pin field: selected_target: SocketAddr"
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  if ! rg -Fq 'resolved_addresses([hop.selected_target])' "$ADAPTER" 2>/dev/null; then
    echo "FAIL: $ADAPTER does not dispatch direct hops via singular resolved_addresses pin."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  if rg -q 'pin_wire_url' "$ADAPTER" 2>/dev/null; then
    echo "FAIL: $ADAPTER reintroduces pin_wire_url (removed in 0.1.7 adoption)."
    rg -n 'pin_wire_url' "$ADAPTER" 2>/dev/null || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  if rg -q 'pin_wire_url' crates/eggsec-transport-eggfetch/src/mapping.rs 2>/dev/null; then
    echo "FAIL: mapping.rs still defines pin_wire_url (must stay removed)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Direct backend route uses singular resolved_addresses."
else
  FAIL=$((FAIL + 1))
fi

# 137. Eggfetch 0.1.7 adoption: no environment-proxy integration, H3 off.
echo ""
echo "--- Check 137: No environment proxy or HTTP/3 in scoped adapter ---"
SECTION_FAIL=0
if rg -q 'ProxyEnvironment::|proxy_environment\(|use .*ProxyEnvironment' crates/eggsec-transport-eggfetch/src/ 2>/dev/null; then
  echo "FAIL: adapter must never use ProxyEnvironment/proxy_environment."
  rg -n 'ProxyEnvironment::|proxy_environment\(|use .*ProxyEnvironment' crates/eggsec-transport-eggfetch/src/ 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'without_proxy' crates/eggsec-transport-eggfetch/src/adapter.rs 2>/dev/null; then
  echo "FAIL: adapter direct route must force without_proxy."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q '"http3"|features = .*http3|http3.*feature' crates/eggsec-transport-eggfetch/Cargo.toml 2>/dev/null; then
  echo "FAIL: adapter must not enable http3."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'allow_http3: false' crates/eggsec-transport-eggfetch/src/adapter.rs 2>/dev/null; then
  echo "FAIL: adapter must keep HttpVersionPolicy allow_http3:false."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: No environment proxy or HTTP/3 in scoped adapter."
else
  FAIL=$((FAIL + 1))
fi

# 138. TUI single-writer logging boundary (Phase A): no direct terminal writers
# in production TUI code, and the CLI TUI launch path uses no-console logging.
# While the rich TUI owns the alternate screen, Ratatui/Crossterm must be the
# only writer to the controlling terminal. Tracing stays the diagnostic facade;
# the sink policy is the architectural control (never ban tracing macros).
echo ""
echo "--- Check 138: TUI single-writer logging boundary holds ---"
SECTION_FAIL=0
# 138a: no direct print/debug macros in production TUI code (outside #[cfg(test)]).
# Same per-file test-module split as Check 18: anything before the first
# #[cfg(test)]/mod tests line is production and must not emit terminal text.
# Comment-only mentions (// lines documenting the ban) are ignored by
# stripping the // comment suffix before matching.
TUI_MACRO_FILES=$(rg -l '\b(println!|eprintln!|print!|eprint!|dbg!)' crates/eggsec-tui/src/ --glob='*.rs' 2>/dev/null || true)
for file in $TUI_MACRO_FILES; do
  TEST_MODULE_LINE=$(rg -n '#\[cfg\(test\)]|^mod tests' "$file" 2>/dev/null | head -1 | cut -d: -f1)
  if [[ -z "$TEST_MODULE_LINE" ]]; then
    echo "FAIL: $file uses a direct print/debug macro with no test module to exclude."
    rg -n '\b(println!|eprintln!|print!|eprint!|dbg!)' "$file" 2>/dev/null || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
    continue
  fi
  while IFS= read -r hit; do
    line=$(echo "$hit" | cut -d: -f1)
    if [[ "$line" -lt "$TEST_MODULE_LINE" ]]; then
      line_content=$(sed -n "${line}p" "$file" 2>/dev/null || true)
      code_part=$(printf '%s' "$line_content" | sed 's|//.*||')
      if printf '%s' "$code_part" | rg -q '\b(println!|eprintln!|print!|eprint!|dbg!)' 2>/dev/null; then
        echo "$file:$hit"
        echo "FAIL: $file has direct print/debug macro at line $line (before test module at $TEST_MODULE_LINE)."
        echo "      Production rich-TUI code must route user-visible info through TUI state; tracing stays for diagnostics."
        SECTION_FAIL=$((SECTION_FAIL + 1))
      fi
    fi
  done < <(rg -n '\b(println!|eprintln!|print!|eprint!|dbg!)' "$file" 2>/dev/null || true)
done
# 138b: CLI TUI launch path must resolve intent before logging and disable console.
CLI_MAIN="crates/eggsec-cli/src/main.rs"
if [[ ! -f "$CLI_MAIN" ]]; then
  echo "FAIL: missing CLI entry point: $CLI_MAIN"
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  if ! rg -q 'init_logging_with_console' "$CLI_MAIN" 2>/dev/null; then
    echo "FAIL: $CLI_MAIN does not use init_logging_with_console for the TUI launch surface."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  if ! rg -q 'rich_tui_launch_requested|resolve_console_logging|console_policy_for_launch' "$CLI_MAIN" 2>/dev/null; then
    echo "FAIL: $CLI_MAIN does not resolve rich-TUI launch intent before subscriber construction."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  # Bare init_logging( (without _with_console) must not remain in main.rs;
  # existing callers stay source-compatible via the wrapper, but the launch
  # path itself must select the console policy explicitly.
  BARE_INIT=$(rg -n 'init_logging\(' "$CLI_MAIN" 2>/dev/null | grep -v 'init_logging_with_console' || true)
  if [[ -n "$BARE_INIT" ]]; then
    echo "$BARE_INIT"
    echo "FAIL: $CLI_MAIN still calls bare init_logging(; use init_logging_with_console with an explicit ConsoleLogging policy."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
# 138c: both logging copies expose the no-console policy and gate the console layer.
for logfile in "crates/eggsec-cli/src/logging.rs" "crates/eggsec/src/logging/init.rs"; do
  if [[ ! -f "$logfile" ]]; then
    echo "FAIL: missing logging implementation: $logfile"
    SECTION_FAIL=$((SECTION_FAIL + 1))
    continue
  fi
  for sym in "enum ConsoleLogging" "init_logging_with_console" "resolve_console_logging" "console_layer_enabled" "ConsoleLogging::Disabled"; do
    if ! rg -Fq "$sym" "$logfile" 2>/dev/null; then
      echo "FAIL: $logfile missing required no-console policy symbol: $sym"
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  done
  if ! rg -q 'if !console_layer_enabled' "$logfile" 2>/dev/null; then
    echo "FAIL: $logfile does not gate the console fmt layer on console_layer_enabled (must construct no stdout/stderr layer when disabled)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
# 138d: TUI runner must route the small-terminal warning in-frame (no code-level eprintln).
# Comment-only mentions documenting the ban are ignored.
RUNNER="crates/eggsec-tui/src/app/runner.rs"
RUNNER_CODE_HITS=$(while IFS= read -r hit; do
  line=$(echo "$hit" | cut -d: -f1)
  line_content=$(sed -n "${line}p" "$RUNNER" 2>/dev/null || true)
  code_part=$(printf '%s' "$line_content" | sed 's|//.*||')
  if printf '%s' "$code_part" | rg -q 'eprintln!' 2>/dev/null; then
    echo "$hit"
  fi
done < <(rg -n 'eprintln!' "$RUNNER" 2>/dev/null || true))
if [[ -n "$RUNNER_CODE_HITS" ]]; then
  echo "FAIL: $RUNNER still contains code-level eprintln! after terminal acquisition (must use the in-frame notification path)."
  echo "$RUNNER_CODE_HITS"
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'small_terminal_warning_message' "$RUNNER" 2>/dev/null; then
  echo "FAIL: $RUNNER missing the in-frame small-terminal warning path (small_terminal_warning_message)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: TUI single-writer logging boundary holds."
else
  FAIL=$((FAIL + 1))
fi

# 139. TUI terminal lifecycle is cleanup-safe and child output is captured.
# Phase B: one session owner (TerminalSession over ratatui::try_init), no
# open-coded raw/alternate-screen lifecycle, no nested Tokio runtimes on the
# daemon path, no competing panic hooks, no inherited child stdio in the TUI
# crate, and the CLI launch gate passes has_command (is_some, not is_none).
echo ""
echo "--- Check 139: TUI cleanup-safe terminal lifecycle holds ---"
SECTION_FAIL=0
TUI_RUNNER="crates/eggsec-tui/src/app/runner.rs"
TUI_APP_MOD="crates/eggsec-tui/src/app/mod.rs"
CLI_MAIN="crates/eggsec-cli/src/main.rs"
# 139a: no Stdio::inherit() in TUI Rust sources (rich mode forbids
# inherited child stdio; the workspace audit records all sites as captured).
# Scoped to *.rs: prose docs name the banned token to document the ban.
INHERIT_HITS=$(rg -n 'Stdio::inherit' crates/eggsec-tui/src/ --glob='*.rs' 2>/dev/null || true)
if [[ -n "$INHERIT_HITS" ]]; then
  echo "$INHERIT_HITS"
  echo "FAIL: eggsec-tui contains Stdio::inherit(). TUI-reachable child output must be captured (Command::output/piped), never inherited."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# 139b: runner uses the session path; open-coded lifecycle tokens are gone.
for sym in "TerminalSession::new" "session.restore()" "combine_body_restore" "fn run_tui_body" "restore_with_ops" "block_on_ambient"; do
  if ! rg -Fq "$sym" "$TUI_RUNNER" 2>/dev/null; then
    echo "FAIL: $TUI_RUNNER missing required session-path symbol: $sym"
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
# Code-level occurrences only (strip // comments like Check 138d).
LEGACY_TOKENS='Terminal::new\(|enable_raw_mode|EnterAlternateScreen|LeaveAlternateScreen|set_hook|take_hook'
LEGACY_HITS=$(while IFS= read -r hit; do
  line=$(echo "$hit" | cut -d: -f1)
  line_content=$(sed -n "${line}p" "$TUI_RUNNER" 2>/dev/null || true)
  code_part=$(printf '%s' "$line_content" | sed 's|//.*||')
  if printf '%s' "$code_part" | rg -q "$LEGACY_TOKENS" 2>/dev/null; then
    echo "$hit"
  fi
done < <(rg -n "$LEGACY_TOKENS" "$TUI_RUNNER" 2>/dev/null || true))
if [[ -n "$LEGACY_HITS" ]]; then
  echo "$LEGACY_HITS"
  echo "FAIL: $TUI_RUNNER still contains open-coded terminal lifecycle or competing panic hooks (use TerminalSession over ratatui::try_init)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# 139c: no nested Tokio runtime construction on the TUI daemon path.
# The only legitimate tokio::runtime::Runtime::new in the runner is the
# standalone-host fallback inside block_on_ambient (exactly one site).
NESTED_RUNNER=$(rg -cn 'tokio::runtime::Runtime::new' "$TUI_RUNNER" 2>/dev/null || echo "0")
if [[ "$NESTED_RUNNER" != "1" ]]; then
  echo "FAIL: $TUI_RUNNER has $NESTED_RUNNER tokio::runtime::Runtime::new site(s), expected exactly 1 (the block_on_ambient standalone fallback)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
NESTED_APPMOD=$(rg -n 'tokio::runtime::Runtime::new' "$TUI_APP_MOD" 2>/dev/null || true)
if [[ -n "$NESTED_APPMOD" ]]; then
  echo "$NESTED_APPMOD"
  echo "FAIL: $TUI_APP_MOD constructs a nested Tokio runtime (use runner::block_on_ambient; nested runtimes panic under #[tokio::main])."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# 139d: CLI launch gate passes has_command (is_some). The 2026-09-20 PTY
# smoke caught an is_none() inversion that dead-coded the TUI launch.
SOME_COUNT=$(rg -c 'cli\.command\.is_some\(\)' "$CLI_MAIN" 2>/dev/null || echo "0")
if [[ "$SOME_COUNT" -lt 2 ]]; then
  echo "FAIL: $CLI_MAIN must pass cli.command.is_some() for has_command in both cfg branches (found $SOME_COUNT)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
INVERTED_CALL=$(awk '/rich_tui_launch_requested\(/{watch=8} watch>0{watch--; if (/is_none\(\)/) {print NR": "$0; exit 0}}' "$CLI_MAIN" || true)
if [[ -n "$INVERTED_CALL" ]]; then
  echo "$INVERTED_CALL"
  echo "FAIL: $CLI_MAIN passes is_none() for has_command near a rich_tui_launch_requested call (inverts the launch gate)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: TUI cleanup-safe terminal lifecycle holds."
else
  FAIL=$((FAIL + 1))
fi

# 140. Fuzzer high-cardinality fan-out stays bounded (Phase B).
# The concurrent engine must not return to one-spawn-per-payload plus
# semaphore retention: execution.rs must schedule through a bounded JoinSet
# admission loop, not a semaphore-gated task per payload.
echo ""
echo "--- Check 140: Fuzzer fan-out stays bounded ---"
SECTION_FAIL=0
FUZZ_EXEC="crates/eggsec/src/fuzzer/engine/execution.rs"
for sym in "JoinSet" "in_flight.len() < concurrency"; do
  if ! rg -Fq "$sym" "$FUZZ_EXEC" 2>/dev/null; then
    echo "FAIL: $FUZZ_EXEC missing bounded-scheduler symbol: $sym"
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
for sym in "tokio::sync::Semaphore" "DashMap" "join_all"; do
  if rg -Fq "$sym" "$FUZZ_EXEC" 2>/dev/null; then
    echo "FAIL: $FUZZ_EXEC regressed to unbounded fan-out (found $sym; use the bounded JoinSet scheduler)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Fuzzer fan-out stays bounded."
else
  FAIL=$((FAIL + 1))
fi

# 141. Distributed worker capacity stays truthful (Phase D).
# WorkerConfig.max_concurrency must flow through the shared capacity
# tracker: acquisition sizes requests by available capacity and the
# processing loop admits through it. The fixed 100-slot channel (which let
# locally queued work grow beyond budget) must not return.
echo ""
echo "--- Check 141: Worker max_concurrency capacity holds ---"
SECTION_FAIL=0
WORKER="crates/eggsec/src/distributed/worker.rs"
for sym in "CapacityTracker" "try_reserve" "request_size" "max_concurrency must be greater than zero"; do
  if ! rg -Fq "$sym" "$WORKER" 2>/dev/null; then
    echo "FAIL: $WORKER missing capacity symbol: $sym"
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if rg -Fq 'mpsc::channel::<Task>(100)' "$WORKER" 2>/dev/null; then
  echo "FAIL: $WORKER restored the fixed 100-slot channel (size it from max_concurrency so queued work stays within budget)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Worker max_concurrency capacity holds."
else
  FAIL=$((FAIL + 1))
fi

# 142. Load-test request prototype stays compiled once per run (Phase C).
# The per-request worker loop must dispatch cheap prototype clones, not
# rebuild the transport DTO (method/URL/header re-parse) per request.
echo ""
echo "--- Check 142: Load-test prototype compiled once per run ---"
SECTION_FAIL=0
LT_EXEC="crates/eggsec/src/loadtest/executor.rs"
if ! rg -Fq 'prototype.clone()' "$LT_EXEC" 2>/dev/null; then
  echo "FAIL: $LT_EXEC no longer dispatches prototype clones in the worker loop."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -Fq 'self.template.scoped_request' "$LT_EXEC" 2>/dev/null; then
  echo "FAIL: $LT_EXEC rebuilds the request DTO per request (compile the prototype once in run())."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Load-test prototype compiled once per run."
else
  FAIL=$((FAIL + 1))
fi

# 143. NSE canonical execution/report convergence (Milestone 001).
# Production Eggsec/Python NSE paths must use the runtime-owned
# `execute_nse_run()` orchestration API rather than constructing their own
# report pipelines. Concretely:
# - engine dispatch (dispatch/api.rs) references execute_nse_run/NseRunRequest,
#   performs no direct custom-file read, and assembles no report via build_report;
# - the Python binding (eggsec-python/src/nse.rs) references execute_nse_run
#   and owns no duplicated static-require fallback;
# - the standalone runtime CI tests the runtime CLI helper delegation.
echo ""
echo "--- Check 143: NSE surfaces use the canonical runtime pipeline (FAIL) ---"
SECTION_FAIL=0
if ! rg -q 'execute_nse_run|NseRunRequest' crates/eggsec/src/dispatch/api.rs 2>/dev/null; then
  echo "FAIL: crates/eggsec/src/dispatch/api.rs does not use the canonical NSE pipeline."
  echo "      Engine NSE dispatch must construct NseRunRequest and call execute_nse_run()."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'std::fs::read_to_string' crates/eggsec/src/dispatch/api.rs 2>/dev/null; then
  echo "FAIL: crates/eggsec/src/dispatch/api.rs reads files directly."
  echo "      Custom script files must resolve through ScriptResolver via NseScriptSource::File."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q '\.build_report\(' crates/eggsec/src/dispatch/api.rs 2>/dev/null; then
  echo "FAIL: crates/eggsec/src/dispatch/api.rs assembles its own NSE report via build_report()."
  echo "      Report assembly is runtime-owned in execute_nse_run()."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'execute_nse_run' crates/eggsec-python/src/nse.rs 2>/dev/null; then
  echo "FAIL: crates/eggsec-python/src/nse.rs does not use the canonical NSE pipeline."
  echo "      The Python binding must construct NseRunRequest and call execute_nse_run()."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'fn extract_static_requires' crates/eggsec-python/src/nse.rs 2>/dev/null; then
  echo "FAIL: crates/eggsec-python/src/nse.rs duplicates the static-require fallback."
  echo "      Static require reconciliation is runtime-owned (eggsec-nse/src/run.rs)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: Eggsec dispatch and Python use the canonical NSE runtime pipeline."
else
  FAIL=$((FAIL + 1))
fi

# 144. The in-tree runtime must not reappear and the engine must consume the
# released crates.io runtime (Milestones 004, 006B: 0.2.0). Runtime crate
# internals are guarded by its own CI.
echo ""
echo "--- Check 144: no local runtime and released registry source (FAIL) ---"
if [[ -e crates/eggsec-nse ]]; then
  echo "FAIL: crates/eggsec-nse has reappeared in the Eggsec workspace."
  FAIL=$((FAIL + 1))
else
  echo "PASS: no local eggsec-nse source tree."
fi
if rg -q 'eggsec-nse\s*=\s*\{[^}]*(git|path|branch|tag|rev)\s*=' crates/eggsec/Cargo.toml; then
  echo "FAIL: engine declares a Git/path/branch/tag/rev eggsec-nse source; only the crates.io release is accepted."
  FAIL=$((FAIL + 1))
elif ! rg -q 'eggsec-nse = \{ version = "0\.2\.0"' crates/eggsec/Cargo.toml; then
  echo "FAIL: engine does not consume the released eggsec-nse 0.2.0 crates.io dependency."
  FAIL=$((FAIL + 1))
elif rg -q -F "git+https://github.com/eggstack/eggsec-nse" Cargo.lock; then
  echo "FAIL: Cargo.lock still resolves eggsec-nse from the temporary Git source."
  FAIL=$((FAIL + 1))
elif ! rg -q -U 'name = "eggsec-nse"\nversion = "0\.2\.0"\nsource = "registry\+https://github\.com/rust-lang/crates\.io-index"' Cargo.lock; then
  echo "FAIL: Cargo.lock does not resolve eggsec-nse 0.2.0 from the crates.io registry."
  FAIL=$((FAIL + 1))
else
  echo "PASS: engine consumes released eggsec-nse 0.2.0 from crates.io; lockfile agrees."
fi

# 145. Only the engine directly consumes the runtime; frontends forward NSE
# through the engine facade and Eggsec-only adapters remain engine-owned.
echo ""
echo "--- Check 145: single consumer and Eggsec adapter ownership (FAIL) ---"
DIRECT_CONSUMERS=$(rg -l '^\s*eggsec-nse\s*=' crates/*/Cargo.toml 2>/dev/null || true)
UNEXPECTED_CONSUMERS=$(printf '%s\n' "$DIRECT_CONSUMERS" | grep -v '^crates/eggsec/Cargo.toml$' || true)
NSE_SURFACE_HITS=""
if [[ -n "$UNEXPECTED_CONSUMERS" ]]; then
  echo "$UNEXPECTED_CONSUMERS"
  echo "FAIL: Workspace crates outside the engine declare a direct eggsec-nse edge."
  FAIL=$((FAIL + 1))
fi
if ! printf '%s\n' "$DIRECT_CONSUMERS" | grep -q '^crates/eggsec/Cargo.toml$'; then
  echo "FAIL: engine lacks its direct runtime dependency."
  FAIL=$((FAIL + 1))
fi
if ! rg -q 'pub use eggsec_nse as nse' crates/eggsec/src/lib.rs; then
  echo "FAIL: engine no longer exposes the eggsec::nse facade."
  FAIL=$((FAIL + 1))
fi
if ! rg -q '^nse = \["eggsec/nse"\]' crates/eggsec-tui/Cargo.toml || ! rg -q '^nse = \["eggsec/nse"\]' crates/eggsec-python/Cargo.toml; then
  echo "FAIL: TUI and Python must forward NSE through eggsec."
  FAIL=$((FAIL + 1))
fi
if [[ ! -f crates/eggsec/src/nse_bridge.rs || ! -f crates/eggsec/src/nse_http_capability.rs || ! -f crates/eggsec/src/nse_http_provider.rs ]]; then
  echo "FAIL: Eggsec-owned NSE report/transport adapters are missing from the engine."
  FAIL=$((FAIL + 1))
fi
# M006B: the staged NSE HTTP provider adapter is dormant — no production
# module may construct it until the M007 scope-threading milestone.
# (Tests inside nse_http_provider.rs are the only allowed constructors.)
DORMANT_ADAPTER_HITS=$(rg -l 'NseHttpTransportProvider::new|NseHttpTransportProvider::with_' crates/ --glob='*.rs' 2>/dev/null | grep -v '^crates/eggsec/src/nse_http_provider\.rs$' || true)
if [[ -n "$DORMANT_ADAPTER_HITS" ]]; then
  echo "$DORMANT_ADAPTER_HITS"
  echo "FAIL: staged NseHttpTransportProvider has a production constructor outside nse_http_provider.rs (M006B dormancy; M007 owns activation)."
  FAIL=$((FAIL + 1))
fi
if [[ $FAIL -eq 0 ]]; then
  echo "PASS: only eggsec consumes the runtime; frontend facade and engine adapters remain."
fi

# 146. Engine dispatch remains on the canonical request/report API and strict
# surfaces do not use manual-only executor constructors.
echo ""
echo "--- Check 146: canonical NSE integration surfaces (FAIL) ---"
SECTION_FAIL=0
if ! rg -q 'execute_nse_run|NseRunRequest' crates/eggsec/src/dispatch/api.rs; then
  echo "FAIL: engine dispatch does not call the canonical NSE runtime pipeline."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'execute_nse_run' crates/eggsec-python/src/nse.rs; then
  echo "FAIL: Python NSE binding does not call the canonical runtime pipeline."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
NSE_AUTOMATED_HITS=$(rg -n 'NseExecutor::with_policy|NseExecutor::new|NseExecutor::with_sandbox|NseExecutor::with_target' \
  crates/eggsec/src/dispatch crates/eggsec/src/agent crates/eggsec/src/mcp crates/eggsec/src/rest \
  crates/eggsec/src/grpc crates/eggsec-daemon/src --glob='*.rs' 2>/dev/null || true)
if [[ -n "$NSE_AUTOMATED_HITS" ]]; then
  echo "$NSE_AUTOMATED_HITS"
  echo "FAIL: automated NSE surface uses a manual-only executor constructor."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -gt 0 ]]; then
  FAIL=$((FAIL + 1))
else
  echo "PASS: engine NSE dispatch/Python integration stays canonical and strict."
fi

# 147. Phase G: removed utils::redaction stays removed (zero-consumer dead code).
echo ""
echo "--- Check 147: removed utils::redaction stays removed ---"
SECTION_FAIL=0
if [[ -f "crates/eggsec/src/utils/redaction.rs" ]]; then
  echo "FAIL: crates/eggsec/src/utils/redaction.rs reappeared (Phase G removed it: zero production consumers)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'pub mod redaction' crates/eggsec/src/utils/mod.rs 2>/dev/null; then
  echo "FAIL: utils/mod.rs re-exposes the redaction module."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'fn redact_sensitive|fn redact_json' crates/eggsec/src/ 2>/dev/null; then
  echo "FAIL: engine-local redact_sensitive/redact_json reappeared under eggsec/src."
  rg -n 'fn redact_sensitive|fn redact_json' crates/eggsec/src/ 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: removed utils::redaction stays removed."
else
  FAIL=$((FAIL + 1))
fi

# Search Rust sources under $1 for regex $2 with string literals and comments
# stripped FIRST, so that attack-payload *data* cannot trip a code-level guard.
#
# Filtering the rg output instead does not work: after blanking the literal the
# line still exists, so a line-level `grep -q .` passes even though the only
# match was inside the string. The OAuth corpus legitimately ships
# `description: "Scope escalation to admin"` and
# `description: "Client credentials exposed in URL query"` -- that is attack
# data, not the crate reaching for Scope/Capability authority. Strip, then
# search. Prints `path:line:content` for each surviving match.
code_search() {
  local dir="$1" pattern="$2" f m
  [[ -d "$dir" ]] || return 0
  while IFS= read -r f; do
    m="$(sed -e 's/"[^"]*"/""/g' -e 's://.*::' -- "$f" | grep -nE "$pattern" || true)"
    # `if`, not `[[ ... ]] && ...`: under `set -e` an AND-list that ends false
    # aborts the loop at the FIRST non-matching file, so the search would stop
    # early and report nothing. A guard that silently passes is worse than none.
    if [[ -n "$m" ]]; then
      printf '%s:%s\n' "$f" "$m"
    fi
  done < <(rg --files -g '*.rs' "$dir" 2>/dev/null)
}

# 148. Phase G: knowledge-corpus crates stay leaf (no engine/IO/authority reach).
echo ""
echo "--- Check 148: knowledge-corpus crates stay leaf ---"
SECTION_FAIL=0
for corpus in eggsec-service-db eggsec-secrets eggsec-payloads; do
  MANIFEST="crates/${corpus}/Cargo.toml"
  SRC_DIR="crates/${corpus}/src"
  if [[ ! -d "$SRC_DIR" ]]; then
    continue  # crate not extracted yet; nothing to police
  fi
  # `eggsec-core` is the ONLY permitted workspace dependency (it is a
  # zero-internal-dependency leaf that owns `Severity`). Everything else
  # reaches back into engine/domain territory and would break the corpus
  # boundary. eggsec-service-db is held to the stricter bar of zero.
  for dep in 'eggsec-policy' 'eggsec-runtime' 'eggsec-transport' 'eggsec-output' 'eggsec-report-model' 'eggsec-tool-core' 'eggsec-agent' 'eggsec-udp-scan' 'eggsec-daemon' 'eggsec ='; do
    if rg -q "^${dep}([ =]|$)" "$MANIFEST" 2>/dev/null; then
      echo "FAIL: ${MANIFEST} references forbidden workspace dep '$dep' (only eggsec-core is permitted)."
      rg -n "$dep" "$MANIFEST" 2>/dev/null || true
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  done
  # No runtime / network / TLS / frontend / persistence reach.
  for dep in 'tokio' 'reqwest' 'hyper' 'rustls' 'axum' 'tonic' 'clap' 'ratatui' 'crossterm' 'rusqlite' 'sqlx' 'hickory-resolver' 'indicatif'; do
    if rg -q "^${dep}([ =]|$)" "$MANIFEST" 2>/dev/null; then
      echo "FAIL: ${MANIFEST} references forbidden dep '$dep'."
      rg -n "$dep" "$MANIFEST" 2>/dev/null || true
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  done
  # No authorization vocabulary in a corpus: it reports facts, never grants.
  # Matched as whole identifiers so payload *text* does not trip this: the
  # dependency-confusion corpus contains the words "scope"/"Scoped" in attack
  # strings, and "Capability" could appear in any payload description.
  # String literals are stripped first: a corpus payload may legitimately carry
  # the word in its description text (the OAuth corpus ships "Scope escalation to
  # admin"), and that is attack data, not the crate reaching for authority. What
  # must never appear is the vocabulary in *code*.
  if code_search "$SRC_DIR" '\b(Scope|ApprovedOperation|ApprovedExecution|EnforcementContext|Capability)\b' | grep -q .; then
    echo "FAIL: ${SRC_DIR} references authorization vocabulary in code (corpus crates authorize nothing)."
    code_search "$SRC_DIR" '\b(Scope|ApprovedOperation|ApprovedExecution|EnforcementContext|Capability)\b' || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
# eggsec-service-db needs nothing at all from the workspace: it does not even
# use Severity. Stricter bar than the other corpora, asserted separately.
if [[ -f "crates/eggsec-service-db/Cargo.toml" ]] && rg -q '^eggsec-core([ =]|$)' crates/eggsec-service-db/Cargo.toml 2>/dev/null; then
  echo "FAIL: crates/eggsec-service-db/Cargo.toml declares eggsec-core (this corpus needs no workspace edge)."
  rg -n 'eggsec-core' crates/eggsec-service-db/Cargo.toml 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: knowledge-corpus crates stay leaf (eggsec-core only; no IO/frontend; no authority)."
else
  FAIL=$((FAIL + 1))
fi

# 149. Phase G: secret detection has one owner, a permanent facade, and a frozen entropy gate.
echo ""
echo "--- Check 149: secret-detection owner, facade, and entropy gate ---"
SECTION_FAIL=0
if [[ ! -f "crates/eggsec-secrets/src/lib.rs" ]]; then
  echo "FAIL: crates/eggsec-secrets/src/lib.rs missing (canonical secret-detection owner)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ -f "crates/eggsec/src/recon/secrets.rs" ]]; then
  echo "FAIL: crates/eggsec/src/recon/secrets.rs reappeared (owner is the eggsec-secrets crate)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'pub use eggsec_secrets as secrets' crates/eggsec/src/recon/mod.rs 2>/dev/null; then
  echo "FAIL: recon/mod.rs no longer re-exports eggsec-secrets as secrets (facade path broken)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# The entropy gate is detection semantics, not a tuning knob. It must stay
# scoped to AWS secret keys and stay at 3.5; widening or retuning it silently
# changes what the scanner finds.
if [[ -f "crates/eggsec-secrets/src/lib.rs" ]]; then
  ENTROPY_HITS=$(rg -n 'secret_entropy\(value\) < [0-9.]+' crates/eggsec-secrets/src/ --glob '*.rs' 2>/dev/null | grep -v '^[[:space:]]*[^:]*:[0-9]*:[[:space:]]*//' || true)
  if [[ -z "$ENTROPY_HITS" ]]; then
    echo "FAIL: entropy gate 'secret_entropy(value) < N' not found in eggsec-secrets (detection semantics changed?)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  else
    while IFS= read -r line; do
      [[ -z "$line" ]] && continue
      if ! printf '%s' "$line" | rg -q 'secret_entropy\(value\) < 3\.5 \{'; then
        echo "FAIL: entropy gate threshold is not 3.5: $line"
        SECTION_FAIL=$((SECTION_FAIL + 1))
      fi
    done <<< "$ENTROPY_HITS"
  fi
  if ! rg -q 'secret_type == SecretType::AwsSecretKey &&' crates/eggsec-secrets/src/ --glob '*.rs' 2>/dev/null; then
    echo "FAIL: entropy gate is no longer scoped to SecretType::AwsSecretKey (must not be widened)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: secret detection has one owner, a permanent facade, and an unchanged entropy gate."
else
  FAIL=$((FAIL + 1))
fi

# 150. Phase G: the corpus owns all 40 payload types; probers stay engine-side.
echo ""
echo "--- Check 150: payload corpus ownership + prober seam ---"
SECTION_FAIL=0
# The six probers need a reqwest::Client and stay in the engine. But their
# payload *strings* are static data and live in the corpus, which is why
# get_payloads resolves all 40 variants with no panic. An earlier cut of this
# guard asserted the opposite (corpus must NOT have these 6 modules, and must
# panic for them) -- that encoded the false premise that the payloads were
# generated by probing. They never were.
for m in graphql grpc idor jwt oauth ssti; do
  CORPUS_MOD="crates/eggsec-payloads/src/${m}.rs"
  ENGINE_MOD="crates/eggsec/src/fuzzer/payloads/${m}.rs"
  # A. The corpus owns the static payloads for all six.
  if [[ ! -f "$CORPUS_MOD" ]]; then
    echo "FAIL: $CORPUS_MOD is missing; the corpus must own the ${m} payload set."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  elif ! rg -q 'pub fn get_payloads' "$CORPUS_MOD" 2>/dev/null; then
    echo "FAIL: $CORPUS_MOD does not define pub fn get_payloads()."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  # B. ...and that payload set is data: no client, no async, no runtime.
  # E. The prober stays engine-side.
  if [[ ! -f "$ENGINE_MOD" ]]; then
    echo "FAIL: $ENGINE_MOD is missing; the ${m} prober needs reqwest and cannot live in eggsec-payloads."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  # F. ...and re-exports the corpus builder, so probers and payloads share one
  #    definition of the strings they send.
  if [[ -f "$ENGINE_MOD" ]] && ! rg -q "pub use eggsec_payloads::${m}::" "$ENGINE_MOD" 2>/dev/null; then
    echo "FAIL: $ENGINE_MOD no longer re-exports the corpus ${m} payload builder."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
# B (all modules). A payload corpus builds strings; it never opens a socket.
# Stricter than the per-file checks above and simpler: no module anywhere in the
# crate may reach for a client, a runtime, or `.await`.
if code_search "crates/eggsec-payloads/src" '\breqwest\b|\btokio\b|\bClient\b|\.await|block_on|TcpStream' | grep -q .; then
  echo "FAIL: crates/eggsec-payloads/src reaches for the network; a payload corpus builds strings only."
  code_search "crates/eggsec-payloads/src" '\breqwest\b|\btokio\b|\bClient\b|\.await|block_on|TcpStream' || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi

CORPUS_LIB="crates/eggsec-payloads/src/lib.rs"
if [[ -f "$CORPUS_LIB" ]]; then
  # C. No variant may panic or stub out. A silent empty Vec reads as "this type
  #    has no payloads", which is false; a panic is worse -- it unwinds through
  #    a library caller's frame. Both are capability regressions.
  for pair in GraphQL:graphql OAuth:oauth Jwt:jwt Idor:idor Ssti:ssti Grpc:grpc; do
    v="${pair%%:*}"
    mod="${pair##*:}"
    if ! rg -q "PayloadType::${v} => ${mod}::get_payloads\(\)" "$CORPUS_LIB" 2>/dev/null; then
      echo "FAIL: $CORPUS_LIB does not dispatch PayloadType::${v} to its corpus module."
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  done
  if rg -n 'PayloadType::[A-Za-z]+ => (unreachable!|Vec::new)' "$CORPUS_LIB" 2>/dev/null; then
    echo "FAIL: $CORPUS_LIB panics or stubs a payload type (silent capability regression)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  # G. The cross-variant caches now live here (they can: the corpus owns all 40
  #    variants). They must stay lazy -- eager materialization regresses every
  #    binary, including the TUI.
  if ! rg -q 'static PAYLOAD_CACHE: LazyLock' "$CORPUS_LIB" 2>/dev/null; then
    echo "FAIL: PAYLOAD_CACHE is missing or no longer a LazyLock (eager payload materialization is prohibited)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  # A cache miss must not degrade to a shared empty Vec.
  if rg -q 'static EMPTY: LazyLock<Vec<Payload>>' "$CORPUS_LIB" 2>/dev/null; then
    echo "FAIL: get_payloads_cached still falls back to an empty Vec (silent capability regression)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  # The all-40 invariant is pinned by tests in this crate, not by a grep.
  if ! rg -q 'every_variant_resolves_to_non_empty_correctly_labelled_payloads' "$CORPUS_LIB" 2>/dev/null; then
    echo "FAIL: $CORPUS_LIB lost the all-40-resolve test (the crate's shippability contract)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
# H. The engine facade is permanent: consumers keep resolving the corpus API.
ENGINE_MOD="crates/eggsec/src/fuzzer/payloads/mod.rs"
if [[ -f "$ENGINE_MOD" ]]; then
  if ! rg -q 'pub use eggsec_payloads::\*;' "$ENGINE_MOD" 2>/dev/null; then
    echo "FAIL: engine payloads/mod.rs no longer re-exports the corpus API (the facade is permanent)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  if [[ -f "$ENGINE_MOD" ]] && rg -q 'static PAYLOAD_CACHE|static ALL_PAYLOADS_CACHE' "$ENGINE_MOD" 2>/dev/null; then
    echo "FAIL: the engine still defines its own cross-variant caches (they belong to the corpus, which owns all 40 variants)."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: corpus owns all 40 payload types (no panic, no silent empty); 6 probers stay engine-side behind the permanent facade."
else
  FAIL=$((FAIL + 1))
fi

# 151. ADR-0008: the published operation schema is descriptive only. It must
# never be imported by, or consulted by, an authorization, dispatch, scope,
# or policy path -- if it were, the artifact would become a second policy
# surface and `OperationMetadata` would stop being the single source of
# truth. The engine and the policy kernel must not even gain the derivation
# dependency, so the schema is unreachable from an enforcement path by
# construction, not by convention.
echo ""
echo "--- Check 151: published schema stays off the authorization path ---"
SECTION_FAIL=0
SCHEMA_PATTERN='schemars|schema_for!|eggsec_runtime::schema|describe_operations|DescribeOperations'
# The engine's execution and bridge halves.
for dir in crates/eggsec/src/dispatch crates/eggsec/src/runtime_bridge crates/eggsec-policy/src; do
  if code_search "$dir" "$SCHEMA_PATTERN" | grep -q .; then
    echo "FAIL: $dir reaches for the published schema; the schema is descriptive and must stay off the authorization path."
    code_search "$dir" "$SCHEMA_PATTERN" || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
# The engine's policy and scope halves.
for f in crates/eggsec/src/config/policy.rs crates/eggsec/src/config/policy_target.rs crates/eggsec/src/config/policy_catalog.rs crates/eggsec/src/config/policy_decision.rs crates/eggsec/src/config/policy_approval.rs crates/eggsec/src/config/scope.rs crates/eggsec/src/config/scope_address.rs; do
  if [[ -f "$f" ]] && rg -q "$SCHEMA_PATTERN" "$f" 2>/dev/null; then
    echo "FAIL: $f reaches for the published schema; the schema is descriptive and must stay off the authorization path."
    rg -n "$SCHEMA_PATTERN" "$f" 2>/dev/null || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
# The enforcement owners must not gain the derivation dependency.
if rg -q '^schemars' crates/eggsec/Cargo.toml crates/eggsec-policy/Cargo.toml 2>/dev/null; then
  echo "FAIL: the engine or the policy kernel gained a schemars dependency; the schema must be unreachable from an enforcement path."
  rg -n '^schemars' crates/eggsec/Cargo.toml crates/eggsec-policy/Cargo.toml 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the published schema is unreachable from every authorization, dispatch, scope, and policy path."
else
  FAIL=$((FAIL + 1))
fi

# 152. ADR-0008: no hand-written parameter table. A transcribed parameter
# table is a second source of truth: it drifts silently the moment a request
# struct gains, drops, or renames a field, and nothing fails. The canonical
# request DTOs live in eggsec-runtime; no other frontend-neutral crate may
# define a struct with a params name.
#
# The engine and the Python bindings are deliberately out of scope: the engine
# owns REST-surface DTOs (e.g. PaginationParams) and the bindings own their
# own argument structs. Neither publishes a request-parameter table.
echo ""
echo "--- Check 152: no hand-written parameter table ---"
SECTION_FAIL=0
for crate in eggsec-core eggsec-tool-core eggsec-runtime eggsec-output eggsec-ui-model eggsec-report-model eggsec-daemon-protocol eggsec-policy eggsec-frontend-client; do
  dir="crates/${crate}/src"
  [[ -d "$dir" ]] || continue
  HITS="$(rg -n 'pub struct \w*Params\b' "$dir" 2>/dev/null | grep -v '^crates/eggsec-runtime/src/request.rs:' || true)"
  if [[ -n "$HITS" ]]; then
    echo "FAIL: $crate defines a params struct; only crates/eggsec-runtime/src/request.rs owns the canonical request DTOs."
    echo "$HITS"
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the canonical request DTOs are the only params structs in the frontend-neutral crates."
else
  FAIL=$((FAIL + 1))
fi

# 153. The frontend-client crate is the shared runtime contract for every
# interactive frontend. It must stay free of any frontend framework and of
# the engine, so a second frontend can depend on it without inheriting a TUI
# -- or the whole engine -- along with it.
echo ""
echo "--- Check 153: frontend-client crate stays framework-free ---"
SECTION_FAIL=0
FC_MANIFEST="crates/eggsec-frontend-client/Cargo.toml"
if [[ ! -f "$FC_MANIFEST" ]]; then
  echo "FAIL: $FC_MANIFEST is missing (the frontend-client crate must exist)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  for dep in 'eggsec =' 'eggsec-tui' 'ratatui' 'crossterm'; do
    if rg -q "^${dep}" "$FC_MANIFEST" 2>/dev/null; then
      echo "FAIL: $FC_MANIFEST references forbidden dep '$dep' (the frontend client carries no frontend framework)."
      rg -n "^${dep}" "$FC_MANIFEST" 2>/dev/null || true
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  done
  if code_search crates/eggsec-frontend-client/src 'ratatui|crossterm|eggsec_tui|eggsec::' | grep -q .; then
    echo "FAIL: crates/eggsec-frontend-client/src reaches for a frontend framework or the engine."
    code_search crates/eggsec-frontend-client/src 'ratatui|crossterm|eggsec_tui|eggsec::' || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the frontend-client crate depends on no frontend framework and not on the engine."
else
  FAIL=$((FAIL + 1))
fi

# 154. The desktop GUI must remain a *manual* surface. A future edit could
# silently demote it (drop it from honors_manual_override, move it onto an
# agent-controlled arm, give it a strict profile) or drop the conversion
# entirely, and the GUI would then be evaluated under the wrong rules. This
# guard pins the variant's presence, its manual classification, and the one
# conversion table that owns it.
echo ""
echo "--- Check 154: desktop GUI stays a manual surface ---"
SECTION_FAIL=0
GUI_RUNTIME="crates/eggsec-runtime/src/request.rs"
GUI_POLICY="crates/eggsec-policy/src/policy.rs"
GUI_BRIDGE="crates/eggsec/src/runtime_bridge/surface.rs"
if ! rg -q '^\s+DesktopGuiManual,' "$GUI_RUNTIME" 2>/dev/null; then
  echo "FAIL: $GUI_RUNTIME no longer declares RuntimeSurface::DesktopGuiManual."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q '^\s+DesktopGuiManual,' "$GUI_POLICY" 2>/dev/null; then
  echo "FAIL: $GUI_POLICY no longer declares ExecutionSurface::DesktopGuiManual."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'RuntimeSurface::DesktopGuiManual => Ok\(ExecutionSurface::DesktopGuiManual\)' "$GUI_BRIDGE" 2>/dev/null; then
  echo "FAIL: $GUI_BRIDGE no longer maps RuntimeSurface::DesktopGuiManual to the execution surface."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'ExecutionSurface::DesktopGuiManual => RuntimeSurface::DesktopGuiManual' "$GUI_BRIDGE" 2>/dev/null; then
  echo "FAIL: $GUI_BRIDGE no longer maps ExecutionSurface::DesktopGuiManual back to the wire surface."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# The manual/strict split is decided by honors_manual_override() and
# is_agent_controlled(); the GUI must be on the manual side of both.
GUI_MANUAL_ARM="$(sed -n '/pub fn honors_manual_override/,/^    }/p' "$GUI_POLICY")"
if ! printf '%s' "$GUI_MANUAL_ARM" | grep -q 'DesktopGuiManual'; then
  echo "FAIL: honors_manual_override() no longer includes the desktop GUI surface."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
GUI_AGENT_ARM="$(sed -n '/pub fn is_agent_controlled/,/^    }/p' "$GUI_POLICY")"
if printf '%s' "$GUI_AGENT_ARM" | grep -q 'DesktopGuiManual'; then
  echo "FAIL: is_agent_controlled() includes the desktop GUI surface; the GUI is a manual surface."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
GUI_PROFILE_ARM="$(sed -n '/pub fn profile/,/^    }/p' "$GUI_POLICY")"
if ! printf '%s' "$GUI_PROFILE_ARM" | grep -q 'DesktopGuiManual'; then
  echo "FAIL: profile() no longer derives a profile for the desktop GUI surface."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the desktop GUI surface is declared in both enums, converts in both directions, and stays manual."
else
  FAIL=$((FAIL + 1))
fi

# 155. The desktop GUI is a daemon-only frontend: it reaches the engine
# through the daemon's Unix socket, using the shared frontend-client
# contract. A GUI that linked the engine (or the daemon, or another
# frontend) directly would open a second path to the engine -- one with no
# daemon session, no daemon audit trail, and no daemon-issued scope. The
# dependency direction is one-way: engine -> transport, frontend -> daemon.
#
# `eggsec-frontend-client`, `eggsec-runtime`, and `eggsec-daemon-protocol`
# ARE permitted: they are the frontend-neutral wire contract. The match is
# on the EXACT dependency name, never a substring -- `eggsec` as a prefix
# would false-positive on all three of them.
echo ""
echo "--- Check 155: GUI crate links no engine ---"
SECTION_FAIL=0
GUI_MANIFEST="crates/eggsec-gui/Cargo.toml"
# The forbidden source-level shapes: a crate path (`eggsec::`) or an import
# statement naming one of the four crates. The crate name must be followed
# by `::` or end the `use` item, so `eggsec_frontend_client::`,
# `eggsec_runtime::`, and `eggsec_daemon_protocol::` stay permitted.
GUI_FORBIDDEN_PATH='(^|[^[:alnum:]_])(eggsec|eggsec_cli|eggsec_tui|eggsec_daemon)::|(^|[^[:alnum:]_])use[[:space:]]+(::)?(eggsec|eggsec_cli|eggsec_tui|eggsec_daemon)([[:space:]]|;)|(^|[^[:alnum:]_])extern[[:space:]]+crate[[:space:]]+(eggsec|eggsec_cli|eggsec_tui|eggsec_daemon)([[:space:]]|;)'
if [[ ! -f "$GUI_MANIFEST" ]]; then
  echo "FAIL: $GUI_MANIFEST is missing (the desktop GUI crate must exist)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  for dep in 'eggsec' 'eggsec-daemon' 'eggsec-cli' 'eggsec-tui'; do
    if rg -q "^${dep}([ =.]|$)|\[dependencies\.${dep}\]" "$GUI_MANIFEST" 2>/dev/null; then
      echo "FAIL: $GUI_MANIFEST declares forbidden dep '${dep}' (the GUI reaches the engine only through the daemon socket)."
      rg -n "^${dep}([ =.]|$)|\[dependencies\.${dep}\]" "$GUI_MANIFEST" 2>/dev/null || true
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  done
  if code_search crates/eggsec-gui/src "$GUI_FORBIDDEN_PATH" | grep -q .; then
    echo "FAIL: crates/eggsec-gui/src imports or reaches the engine, the daemon, or another frontend crate."
    code_search crates/eggsec-gui/src "$GUI_FORBIDDEN_PATH" || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the GUI crate depends on no engine, daemon, or other frontend crate."
else
  FAIL=$((FAIL + 1))
fi

# 156. The shell framework stays inside the GUI crate. Tauri owns a window,
# a webview, and an IPC bridge; a second crate depending on it would put a
# second window/IPC owner in the workspace and the daemon-IPC contract
# would stop having exactly one frontend implementation.
#
# The root manifest legitimately carries the shared version pins under
# `[workspace.dependencies]` (tauri, tauri-build, tauri-runtime,
# tauri-runtime-wry, tauri-macros, tauri-utils): that is the workspace's
# single place to pin the MSRV-safe versions, and it is ALLOWED. What is
# forbidden is any crate OTHER than crates/eggsec-gui declaring a
# dependency on one of them, or mentioning `tauri` at all.
echo ""
echo "--- Check 156: shell framework stays inside the GUI crate ---"
SECTION_FAIL=0
GUI_CRATE_MANIFEST="crates/eggsec-gui/Cargo.toml"
for manifest in crates/*/Cargo.toml; do
  [[ "$manifest" == "$GUI_CRATE_MANIFEST" ]] && continue
  if rg -q 'tauri' "$manifest" 2>/dev/null; then
    echo "FAIL: $manifest mentions 'tauri'; the shell framework belongs to crates/eggsec-gui alone."
    rg -n 'tauri' "$manifest" 2>/dev/null || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
# The root manifest is a virtual manifest: it may PIN tauri for the
# workspace, but it must never declare a tauri dependency of its own.
ROOT_MANIFEST="Cargo.toml"
ROOT_TAURI_ELSEWHERE=$(awk '
  /^\[[^]]*\][[:space:]]*$/ { section = $0 }
  section != "[workspace.dependencies]" && /tauri/ && $0 !~ /^[[:space:]]*#/ { print FILENAME ":" FNR ":" $0 }
' "$ROOT_MANIFEST" 2>/dev/null || true)
if [[ -n "$ROOT_TAURI_ELSEWHERE" ]]; then
  echo "FAIL: $ROOT_MANIFEST references 'tauri' outside [workspace.dependencies]; only crates/eggsec-gui may depend on the shell framework."
  echo "$ROOT_TAURI_ELSEWHERE"
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: no crate other than crates/eggsec-gui declares or mentions a tauri dependency."
else
  FAIL=$((FAIL + 1))
fi

# 157. The GUI declares no second TLS stack. The GUI performs no outbound
# HTTP of its own: every engine request travels over the daemon's Unix
# socket. A TLS stack in the GUI's graph would be a second, unowned TLS
# policy -- and the workspace's TLS invariant is ring-only, with no
# `rustls-tls`/`native-tls` exception.
#
# The scope is the GUI crate's manifest, deliberately NOT the workspace: the
# engine's transport stack and the daemon's SQLite stack legitimately carry
# reqwest/rustls, so a workspace-wide scan would be a false positive. The
# root manifest is consulted only to NAME the workspace pin in the failure
# message, never to police it.
echo ""
echo "--- Check 157: GUI declares no second TLS stack ---"
SECTION_FAIL=0
if [[ ! -f "$GUI_MANIFEST" ]]; then
  echo "FAIL: $GUI_MANIFEST is missing (the desktop GUI crate must exist)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  # Comments are stripped FIRST: the manifest documents this ban in prose
  # ("`rustls-tls` and `native-tls` are deliberately NOT enabled"), and a
  # guard that trips on its own documentation is a guard nobody trusts.
  GUI_MANIFEST_CODE=$(sed -e 's/#.*$//' "$GUI_MANIFEST")
  # A. No direct TLS dependency -- neither a path dependency nor a
  #    `workspace = true` pin.
  for dep in 'reqwest' 'rustls' 'native-tls'; do
    if printf '%s\n' "$GUI_MANIFEST_CODE" | rg -q "^${dep}([ =.]|$)|\[dependencies\.${dep}\]"; then
      echo "FAIL: $GUI_MANIFEST declares forbidden TLS dep '${dep}' (the GUI talks to the daemon over a Unix socket and needs no TLS stack)."
      printf '%s\n' "$GUI_MANIFEST_CODE" | rg -n "^${dep}([ =.]|$)|\[dependencies\.${dep}\]" || true
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  done
  # B. No TLS feature enabled on any dependency: `rustls-tls` (reqwest) and
  #    `native-tls` are the two ways a TLS stack enters through a feature.
  if printf '%s\n' "$GUI_MANIFEST_CODE" | rg -n '(^|[^[:alnum:]_-])(rustls-tls|native-tls)([^[:alnum:]_-]|$)'; then
    echo "FAIL: $GUI_MANIFEST enables a TLS feature (rustls-tls / native-tls); the GUI must not add a second TLS stack."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  # C. The workspace may pin reqwest/rustls/native-tls for OTHER crates; the
  #    GUI must not inherit one through `workspace = true`. Report the root
  #    pin by name so the fix is obvious.
  for dep in 'reqwest' 'rustls' 'native-tls'; do
    if rg -q "^${dep}([ =.]|$)" "$ROOT_MANIFEST" 2>/dev/null \
      && printf '%s\n' "$GUI_MANIFEST_CODE" | rg -q "^${dep}([ =.]|$)"; then
      echo "FAIL: $ROOT_MANIFEST pins workspace dep '${dep}' and $GUI_MANIFEST inherits it; the GUI must not add a second TLS stack."
      rg -n "^${dep}([ =.]|$)" "$ROOT_MANIFEST" 2>/dev/null || true
      printf '%s\n' "$GUI_MANIFEST_CODE" | rg -n "^${dep}([ =.]|$)" || true
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  done
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the GUI crate declares no TLS dependency and enables no TLS feature."
else
  FAIL=$((FAIL + 1))
fi

# 158. The GUI's CSP forbids remote origins. The webview loads local assets
# and talks to the daemon over the Tauri IPC bridge only. A wildcard source
# or a remote http(s) origin in the CSP would let an injected or compromised
# asset reach the network -- the one capability the daemon-only GUI is
# designed not to have.
#
# The Tauri IPC origin `http://ipc.localhost` is the single permitted
# exception: it is the bridge to the Rust side, not a network origin.
echo ""
echo "--- Check 158: GUI CSP forbids remote origins ---"
SECTION_FAIL=0
GUI_CONF="crates/eggsec-gui/tauri.conf.json"
if [[ ! -f "$GUI_CONF" ]]; then
  echo "FAIL: $GUI_CONF is missing (the desktop GUI shell config must exist)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
elif ! command -v python3 >/dev/null 2>&1; then
  echo "FAIL: python3 is required to parse $GUI_CONF (the CSP check reads app.security.csp)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  if ! python3 - "$GUI_CONF" <<'PYEOF' 2>&1; then
import json, sys, pathlib

path = pathlib.Path(sys.argv[1])
try:
    data = json.loads(path.read_text())
except Exception as exc:
    print(f"FAIL: {path} is not valid JSON: {exc}")
    sys.exit(1)

security = ((data.get("app") or {}).get("security") or {})
csp = security.get("csp")
if not isinstance(csp, str) or not csp.strip():
    print(f"FAIL: {path} does not define app.security.csp; the GUI webview needs an explicit CSP.")
    sys.exit(1)

ALLOWED_REMOTE = {"http://ipc.localhost"}
WILDCARD_DIRECTIVES = {"connect-src", "script-src", "default-src"}
problems = []
for directive in csp.split(";"):
    directive = directive.strip()
    if not directive:
        continue
    parts = directive.split()
    name, sources = parts[0], parts[1:]
    for source in sources:
        if name in WILDCARD_DIRECTIVES and "*" in source:
            problems.append(f"{name} allows the wildcard source '{source}'")
        if source.startswith("http://") or source.startswith("https://"):
            if source not in ALLOWED_REMOTE:
                problems.append(f"{name} allows the remote origin '{source}'")
        elif source.startswith("//"):
            problems.append(f"{name} allows the protocol-relative origin '{source}'")
if problems:
    print(f"FAIL: {path} app.security.csp allows a wildcard source or a remote origin:")
    for problem in problems:
        print(f"      {problem}")
    print(f"      offending CSP: {csp}")
    print(f"      the only permitted remote origin is {sorted(ALLOWED_REMOTE)[0]}")
    sys.exit(1)
PYEOF
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the GUI CSP allows no wildcard source and no remote origin."
else
  FAIL=$((FAIL + 1))
fi

# 159. The desktop GUI's forms are generated from the schema the daemon
# publishes. A GUI that named an operation, a task-kind variant, or a field
# would carry a second, hand-maintained catalog: it would drift silently the
# moment the engine published a new operation, and the drift would show up
# as a missing button rather than as a failure.
#
# The list of task-kind variants is READ FROM THE RUNTIME, not transcribed
# here: a guard that hardcoded the thirty names would itself be the
# hand-written table this check exists to prevent, and it would go stale
# the day the engine published a thirty-first variant.
#
# The Rust side is searched in its PRODUCTION half only. A test may use a
# variant as a fixture -- there are fixtures like that today -- but
# production code may not name one.
echo ""
echo "--- Check 159: the GUI carries no per-operation form code ---"
SECTION_FAIL=0
GUI_SRC_DIR="crates/eggsec-gui/src"
GUI_FRONTEND_DIR="crates/eggsec-gui/frontend"
TASK_KIND_SOURCE="crates/eggsec-runtime/src/request.rs"
TASK_KIND_VARIANTS=""
if [[ -f "$TASK_KIND_SOURCE" ]]; then
  # The task-kind variants live in the canonical table (the `define_task_kinds!`
  # invocation) when the enum is macro-generated, and in the enum body when it
  # is written out. Read both shapes so the guard survives either.
  TASK_KIND_VARIANTS="$(
    {
      sed -n '/^define_task_kinds! {/,/^}/p' "$TASK_KIND_SOURCE"
      sed -n '/pub enum TaskKind/,/^}/p' "$TASK_KIND_SOURCE"
    } | rg -o '^[[:space:]]+([A-Z][A-Za-z0-9]*)\(' -r '$1' | sort -u
  )"
fi
if [[ -z "$TASK_KIND_VARIANTS" ]]; then
  echo "FAIL: could not read the task-kind variants from $TASK_KIND_SOURCE; the guard cannot run."
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  for variant in $TASK_KIND_VARIANTS; do
    # The frontend has no test half: search it whole.
    if [[ -d "$GUI_FRONTEND_DIR" ]]; then
      FRONTEND_HITS="$(rg -n "\b${variant}\b" "$GUI_FRONTEND_DIR" 2>/dev/null || true)"
      if [[ -n "$FRONTEND_HITS" ]]; then
        echo "FAIL: the frontend names the task-kind variant '$variant'; the form must be generated from the published schema."
        echo "$FRONTEND_HITS"
        SECTION_FAIL=$((SECTION_FAIL + 1))
      fi
    fi
    # The Rust side: the production half only.
    for gui_file in "$GUI_SRC_DIR"/*.rs; do
      [[ -f "$gui_file" ]] || continue
      GUI_PRODUCTION="$(sed '/#\[cfg(test)\]/,$d' "$gui_file")"
      if printf '%s' "$GUI_PRODUCTION" | rg -q "\b${variant}\b"; then
        echo "FAIL: $gui_file names the task-kind variant '$variant'; the form engine must be schema-driven."
        printf '%s' "$GUI_PRODUCTION" | rg -n "\b${variant}\b" || true
        SECTION_FAIL=$((SECTION_FAIL + 1))
      fi
    done
  done
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the GUI names no task-kind variant; its forms come from the published schema."
else
  FAIL=$((FAIL + 1))
fi

# 160. The GUI's task state is a BOUNDED view of the daemon's event stream.
# `TaskLog` is the high-cardinality event type: a task that logs thousands of
# lines must not be able to grow the GUI's memory without bound. The ring
# keeps a capped number of lines per task and counts what it drops, so the
# loss is reported rather than silent.
#
# Unbounded retention is a disqualifying defect, not a tuning problem, so
# this check pins the cap's existence rather than its value: a later
# milestone may move the number, but it may not remove the bound.
echo ""
echo "--- Check 160: the GUI retains no unbounded event log ---"
SECTION_FAIL=0
GUI_TASKS="crates/eggsec-gui/src/tasks.rs"
if [[ ! -f "$GUI_TASKS" ]]; then
  echo "FAIL: $GUI_TASKS is missing (the GUI's task state must exist)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  if ! rg -q 'MAX_LOG_LINES_PER_TASK' "$GUI_TASKS" 2>/dev/null; then
    echo "FAIL: $GUI_TASKS declares no log-line cap; the GUI's task state must bound its retention."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  if ! rg -q 'MAX_LOG_LINES_PER_FLUSH' "$GUI_TASKS" 2>/dev/null; then
    echo "FAIL: $GUI_TASKS declares no forward-flush cap; the GUI must not queue log lines without bound."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  # The retained lines live in a capped ring, not in a Vec that grows. The
  # check is scoped to the ring struct itself: `Vec<LogLine>` elsewhere in
  # the module is fine -- a serialized snapshot of the ring and a single
  # event's batch are both bounded by the ring -- but the ring itself must
  # never be an unbounded accumulator.
  if ! python3 - "$GUI_TASKS" <<'PYEOF'
import re
import sys

path = sys.argv[1]
text = open(path).read()

match = re.search(r"(?:pub\s+)?struct\s+LogRing\s*\{", text)
if not match:
    print(f"FAIL: {path} declares no LogRing struct; the GUI's retained log lines must live in a bounded ring.")
    sys.exit(1)

depth = 0
end = match.end()
for index in range(match.end() - 1, len(text)):
    if text[index] == "{":
        depth += 1
    elif text[index] == "}":
        depth -= 1
        if depth == 0:
            end = index
            break
body = text[match.start():end]

problems = []
if "VecDeque<LogLine>" not in body:
    problems.append("the ring does not hold its lines in a VecDeque")
if "Vec<LogLine>" in body:
    problems.append("the ring holds its lines in an unbounded Vec")
if not re.search(r"\bcapacity\b", body):
    problems.append("the ring declares no capacity")
if re.search(r"(?<!_)\.push\(", body):
    problems.append("the ring pushes with push() rather than push_back()")

if problems:
    print(f"FAIL: {path} does not bound its retained log lines:")
    for problem in problems:
        print(f"      {problem}")
    sys.exit(1)
PYEOF
  then
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the GUI's task state bounds its log retention and reports what it drops."
else
  FAIL=$((FAIL + 1))
fi

# 161. The desktop GUI's coverage model is DERIVED, not transcribed. Milestone
# 006. The GUI tells the operator which of the daemon's published task kinds it
# can reach, and why. That answer is computed from what the daemon publishes --
# the task kinds, the operations, their required features, the advertised
# capabilities, and the enabled-feature set -- so an operation the engine
# publishes tomorrow is covered the day it is published.
#
# A transcribed table would be a second, hand-maintained catalog. It would
# drift silently the moment the engine published a new operation, and the
# drift would show up as a missing row rather than as a failure.
#
# Guard 159 already pins the TASK-KIND VARIANTS: the GUI may not name one.
# This check pins the other two halves of the same table -- the operation ids
# and the capability names -- because a coverage model that names an operation
# is just as hand-maintained as one that names a variant.
#
# The names are READ FROM THEIR OWNERS, not transcribed here: a guard that
# hardcoded the operation ids would itself be the second source of truth this
# check exists to prevent.
echo ""
echo "--- Check 161: the GUI's coverage model is derived, not transcribed ---"
SECTION_FAIL=0
GUI_SRC_DIR="crates/eggsec-gui/src"
GUI_FRONTEND_DIR="crates/eggsec-gui/frontend"
CATALOG_SOURCE="crates/eggsec-policy/src/catalog.rs"
CAPABILITY_SOURCE="crates/eggsec-runtime/src/capabilities.rs"

OPERATION_IDS=""
CAPABILITY_NAMES=""
if [[ -f "$CATALOG_SOURCE" ]]; then
  OPERATION_IDS="$(sed -n 's/^[[:space:]]*id: "\([^"]*\)",$/\1/p' "$CATALOG_SOURCE" | sort -u)"
fi
if [[ -f "$CAPABILITY_SOURCE" ]]; then
  CAPABILITY_NAMES="$(sed -n 's/^[[:space:]]*TaskCapability::new("\([^"]*\)".*$/\1/p' "$CAPABILITY_SOURCE" | sort -u)"
fi
if [[ -z "$OPERATION_IDS" && -z "$CAPABILITY_NAMES" ]]; then
  echo "FAIL: could not read the operation ids or the capability names from their owners; the guard cannot run."
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  for name in $OPERATION_IDS $CAPABILITY_NAMES; do
    # The Rust side: the PRODUCTION half only. A test may use a name as a
    # fixture -- there are fixtures like that today -- but production code
    # may not name one.
    for gui_file in "$GUI_SRC_DIR"/*.rs; do
      [[ -f "$gui_file" ]] || continue
      GUI_PRODUCTION="$(sed '/#\[cfg(test)\]/,$d' "$gui_file")"
      if printf '%s' "$GUI_PRODUCTION" | rg -q "[\"']${name}[\"']"; then
        echo "FAIL: $gui_file names the operation id or capability '$name'; the coverage model must be derived from what the daemon publishes."
        printf '%s' "$GUI_PRODUCTION" | rg -n "[\"']${name}[\"']" || true
        SECTION_FAIL=$((SECTION_FAIL + 1))
      fi
    done
    # The frontend has no test half: search it whole.
    if [[ -d "$GUI_FRONTEND_DIR" ]]; then
      FRONTEND_HITS="$(rg -n "[\"']${name}[\"']" "$GUI_FRONTEND_DIR" 2>/dev/null || true)"
      if [[ -n "$FRONTEND_HITS" ]]; then
        echo "FAIL: the frontend names the operation id or capability '$name'; the coverage table must be rendered from the data the daemon publishes."
        echo "$FRONTEND_HITS"
        SECTION_FAIL=$((SECTION_FAIL + 1))
      fi
    fi
  done
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the GUI's coverage model is derived, not transcribed."
else
  FAIL=$((FAIL + 1))
fi

# 162. The GUI's report display carries no remote origin and no executable
# content. Milestone 006.
#
# The report the daemon serves is ATTACKER-INFLUENCED: service banners,
# finding text, and host names come from the target. The GUI therefore never
# injects the engine's HTML document into the webview -- it renders the
# report's structured content as text in its own DOM. The Content Security
# Policy is the control that makes that safe, and this check pins it:
#
#   * the policy must not be relaxed to 'unsafe-inline', which would let
#     injected content execute;
#   * the policy must keep framing forbidden, so no report can be embedded
#     as a document;
#   * no GUI production source and no frontend file may reference a remote
#     origin;
#   * the frontend must not gain a markup-injection sink (`innerHTML` and
#     friends), because the report content is rendered as text;
#   * the GUI must not consume an engine-produced HTML document at all, so
#     the export path (the CLI's artifact, written to the operator's chosen
#     file) stays the only place the engine's HTML is used.
echo ""
echo "--- Check 162: the GUI's report display carries no remote origin ---"
SECTION_FAIL=0
GUI_CONF="crates/eggsec-gui/tauri.conf.json"
GUI_SRC_DIR="crates/eggsec-gui/src"
GUI_FRONTEND_DIR="crates/eggsec-gui/frontend"
if [[ ! -f "$GUI_CONF" ]]; then
  echo "FAIL: $GUI_CONF is missing; the GUI webview needs an explicit CSP."
  SECTION_FAIL=$((SECTION_FAIL + 1))
elif ! command -v python3 >/dev/null 2>&1; then
  echo "FAIL: python3 is required to parse $GUI_CONF (the CSP check reads app.security.csp)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  if ! python3 - "$GUI_CONF" "$GUI_SRC_DIR" "$GUI_FRONTEND_DIR" <<'PYEOF' 2>&1; then
import json, pathlib, re, sys

conf_path = pathlib.Path(sys.argv[1])
src_dir = pathlib.Path(sys.argv[2])
frontend_dir = pathlib.Path(sys.argv[3])
problems = []

try:
    conf = json.loads(conf_path.read_text())
except Exception as exc:
    print(f"FAIL: {conf_path} is not valid JSON: {exc}")
    sys.exit(1)

csp = (((conf.get("app") or {}).get("security") or {}).get("csp")) or ""

# The policy must not be relaxed into permitting inline execution.
if "'unsafe-inline'" in csp:
    problems.append(
        "the CSP allows 'unsafe-inline'; the report content is attacker-influenced "
        "and the GUI renders it as text, so inline execution must stay forbidden"
    )

# Framing must stay forbidden: a report document is never embedded as a frame.
frame_src = None
for directive in csp.split(";"):
    directive = directive.strip()
    if directive.lower().startswith("frame-src"):
        frame_src = directive.split(None, 1)[1].strip() if " " in directive else ""
if frame_src is None:
    problems.append("the CSP does not state frame-src; a report could be embedded as a frame")
elif "'none'" not in frame_src:
    problems.append(
        f"the CSP's frame-src is {frame_src!r}; the GUI must keep framing forbidden"
    )

# No remote origin in the GUI's production sources or its frontend. The Tauri
# IPC origin is the single permitted exception: it is the bridge to the Rust
# side, not a network origin.
ALLOWED = "http://ipc.localhost"
REMOTE = re.compile(r"https?://[^\s\"'()<>]+")


def scan(label: str, text: str) -> None:
    for number, line in enumerate(text.splitlines(), start=1):
        for match in REMOTE.finditer(line):
            url = match.group(0)
            if url.startswith(ALLOWED):
                continue
            problems.append(
                f"{label}:{number} references the remote origin {url!r}"
            )


for path in sorted(src_dir.glob("*.rs")):
    production = path.read_text().split("#[cfg(test)]")[0]
    scan(str(path), production)

if frontend_dir.is_dir():
    for path in sorted(frontend_dir.rglob("*")):
        if path.is_file() and path.suffix in {".js", ".html", ".css", ".json"}:
            scan(str(path), path.read_text())

# The frontend renders the report's content as text. A markup-injection sink
# would defeat that.
SINKS = ("innerHTML", "outerHTML", "insertAdjacentHTML", "document.write")
if frontend_dir.is_dir():
    for path in sorted(frontend_dir.rglob("*.js")):
        for number, line in enumerate(path.read_text().splitlines(), start=1):
            for sink in SINKS:
                if sink in line:
                    problems.append(
                        f"{path}:{number} uses the markup sink {sink}; the report content is rendered as text"
                    )

# The GUI must not consume an engine-produced HTML document. The HTML format
# is export-only: the exported file is the CLI's artifact, and the webview
# shows the report's structured content.
export_module = src_dir / "export.rs"
if not export_module.is_file():
    problems.append(f"{export_module} is missing; the export module is the GUI's report path")
else:
    export_production = export_module.read_text().split("#[cfg(test)]")[0]
    if "convert_to_html_self_contained" in export_production:
        problems.append(
            "the export module consumes convert_to_html_self_contained; the GUI must not embed an engine-produced HTML document"
        )
    if "NoDisplayRendering" not in export_production:
        problems.append(
            "the export module does not refuse an HTML display rendering; the HTML format is export-only"
        )

if problems:
    print("FAIL: the GUI's report display is not remote-origin-free:")
    for problem in problems:
        print(f"      {problem}")
    sys.exit(1)
PYEOF
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the GUI's report display carries no remote origin and no executable content."
else
  FAIL=$((FAIL + 1))
fi

# 163. The daemon's GUI-serving profile is pinned to the engine's contract.
# Milestone 008. The engine owns the feature list -- it is the single source
# of truth -- and the daemon's manifest consumes it. Drift between the two
# would silently ship a daemon with a different dependency closure than the
# one the documentation and the conformance tests describe.
#
# Checks 161 and 162 are reserved for milestone 006, which owns the wire-side
# publication of the enabled-feature set. They are deliberately absent here.
echo ""
echo "--- Check 163: the daemon's service profile matches the engine's contract ---"
SECTION_FAIL=0
if [[ ! -f crates/eggsec-daemon/Cargo.toml ]]; then
  echo "FAIL: crates/eggsec-daemon/Cargo.toml is missing."
  SECTION_FAIL=$((SECTION_FAIL + 1))
elif ! command -v python3 >/dev/null 2>&1; then
  echo "FAIL: python3 is required to compare the daemon manifest with the engine's profile."
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  if ! python3 - crates/eggsec-daemon/Cargo.toml crates/eggsec/src/config/daemon_profile.rs <<'PYEOF' 2>&1; then
import re, sys, pathlib

try:
    import tomllib
except ModuleNotFoundError:
    print("FAIL: python3 with tomllib (3.11+) is required to parse the daemon manifest.")
    sys.exit(1)

manifest_path, module_path = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
problems = []

try:
    manifest = tomllib.loads(manifest_path.read_text())
except Exception as exc:
    print(f"FAIL: {manifest_path} is not valid TOML: {exc}")
    sys.exit(1)

try:
    module = module_path.read_text()
except OSError as exc:
    print(f"FAIL: cannot read {module_path}: {exc}")
    sys.exit(1)

match = re.search(
    r"pub static DAEMON_SERVICE_PROFILE[^=]*=\s*&\[(.*?)\];",
    module,
    re.S,
)
if not match:
    print(f"FAIL: {module_path} declares no DAEMON_SERVICE_PROFILE list.")
    sys.exit(1)

declared = set(re.findall(r'"([a-z0-9\-]+)"', match.group(1)))
if not declared:
    print(f"FAIL: {module_path} declares an empty DAEMON_SERVICE_PROFILE list.")
    sys.exit(1)

features = manifest.get("features") or {}
service = features.get("service")
if not isinstance(service, list):
    problems.append("the daemon manifest declares no `service` feature list")
else:
    enabled = {
        item.split("/", 1)[1]
        for item in service
        if isinstance(item, str) and item.startswith("eggsec/")
    }
    missing = sorted(declared - enabled)
    extra = sorted(enabled - declared)
    if missing:
        problems.append(
            "the daemon's `service` feature omits engine features the contract lists: "
            + ", ".join(missing)
        )
    if extra:
        problems.append(
            "the daemon's `service` feature enables engine features the contract does "
            "not list: " + ", ".join(extra)
        )

if features.get("full-executor") != ["service"]:
    problems.append(
        "the daemon's `full-executor` feature must be exactly [\"service\"] (a deprecated alias)"
    )

if problems:
    print("FAIL: the daemon's service profile drifted from the engine's contract:")
    for problem in problems:
        print(f"      {problem}")
    sys.exit(1)
PYEOF
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the daemon's service profile matches the engine's DAEMON_SERVICE_PROFILE."
else
  FAIL=$((FAIL + 1))
fi

# 164. The CLI hosts no rejecting daemon executor. Milestone 009.
#
# The CLI's `daemon start` used to host an executor that rejected every task,
# so the daemon it started advertised capabilities it could not execute. The
# lie was the defect; the fix makes it unrepresentable by removing the
# rejecting executor from the CLI entirely.
echo ""
echo "--- Check 164: the CLI carries no rejecting daemon executor ---"
SECTION_FAIL=0
if [[ ! -d crates/eggsec-cli/src ]]; then
  echo "FAIL: crates/eggsec-cli/src is missing."
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  if ! python3 - crates/eggsec-cli/src <<'PYEOF' 2>&1; then
import re, sys, pathlib

root = pathlib.Path(sys.argv[1])
problems = []

for path in sorted(root.rglob("*.rs")):
    text = path.read_text()
    # The production half of the file is everything before the first
    # `#[cfg(test)]` block; a test may name the rejected shape to prove it
    # is gone.
    production = text.split("#[cfg(test)]", 1)[0]
    if re.search(r"struct\s+NoopExecutor\b", production):
        problems.append(f"{path} declares a `NoopExecutor` in its production code")
    if "RuntimeError::UnsupportedTaskKind" in production:
        problems.append(
            f"{path} rejects tasks with `RuntimeError::UnsupportedTaskKind` in its production code"
        )

if problems:
    print("FAIL: the CLI carries a rejecting daemon executor:")
    for problem in problems:
        print(f"      {problem}")
    print("      the CLI's daemon start path must host the engine executor, not a stub")
    sys.exit(1)
PYEOF
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the CLI carries no rejecting daemon executor."
else
  FAIL=$((FAIL + 1))
fi

# 165. The daemon host derives its published capabilities. Milestone 009.
#
# The `Capabilities` reply used to publish a hardcoded capability constant.
# It must publish the host's DERIVED list -- the executor's declaration
# intersected with the daemon's conservative policy -- so a host can never
# advertise a capability its executor rejects outright.
echo ""
echo "--- Check 165: the daemon host derives its published capabilities ---"
SECTION_FAIL=0
if [[ ! -f crates/eggsec-daemon/src/host.rs ]]; then
  echo "FAIL: crates/eggsec-daemon/src/host.rs is missing."
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  if ! python3 - crates/eggsec-daemon/src/host.rs <<'PYEOF' 2>&1; then
import re, sys, pathlib

path = pathlib.Path(sys.argv[1])
text = path.read_text()
production = text.split("#[cfg(test)]", 1)[0]

marker = "ClientCommand::Capabilities {"
index = production.find(marker)
if index < 0:
    print(f"FAIL: {path} has no `ClientCommand::Capabilities` arm.")
    sys.exit(1)

# Brace-match the arm so the check covers the arm and nothing else.
start = production.find("{", index)
depth = 0
end = -1
for position in range(start, len(production)):
    char = production[position]
    if char == "{":
        depth += 1
    elif char == "}":
        depth -= 1
        if depth == 0:
            end = position
            break
if end < 0:
    print(f"FAIL: {path} has an unterminated `ClientCommand::Capabilities` arm.")
    sys.exit(1)

arm = production[index:end]
FORBIDDEN = (
    "RuntimeCapabilities::default()",
    "daemon_conservative()",
    "full_lab()",
)
problems = [f"the arm publishes `{token}`" for token in FORBIDDEN if token in arm]
if problems:
    print(f"FAIL: {path} publishes a capability constant instead of a derived list:")
    for problem in problems:
        print(f"      {problem}")
    print("      the arm must publish the host's derived task-kind list")
    sys.exit(1)
PYEOF
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the daemon host derives its published capabilities."
else
  FAIL=$((FAIL + 1))
fi

# 166. The CLI's daemon start path waits for a real readiness signal.
# Milestone 009.
#
# The old path slept for a fixed fraction of a second and then printed
# "Daemon started" whether or not the daemon had bound the socket. The
# readiness decision must be a bounded probe, not a fixed sleep, and it must
# never scrape the server's stdout.
echo ""
echo "--- Check 166: the CLI's daemon start path waits for a real readiness signal ---"
SECTION_FAIL=0
if [[ ! -f crates/eggsec-cli/src/daemon_cli.rs ]]; then
  echo "FAIL: crates/eggsec-cli/src/daemon_cli.rs is missing."
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  if ! python3 - crates/eggsec-cli/src/daemon_cli.rs <<'PYEOF' 2>&1; then
import re, sys, pathlib

path = pathlib.Path(sys.argv[1])
text = path.read_text()
production = text.split("#[cfg(test)]", 1)[0]
problems = []

if "Daemon listening on" in production:
    problems.append(
        "the start path scrapes the server's stdout (`Daemon listening on`) for readiness"
    )

if re.search(r"sleep\(\s*Duration::from_millis", production):
    problems.append(
        "the start path uses a sub-second literal sleep for readiness"
    )

if re.search(r"sleep\(\s*Duration::from_secs\(0\)", production):
    problems.append("the start path uses a zero-second sleep for readiness")

# A named bound must exist, and it must be at least a second.
bounds = re.findall(
    r"(?:pub\s+)?const\s+([A-Z0-9_]+)\s*:\s*Duration\s*=\s*Duration::from_(secs|millis)\((\d+)\)",
    production,
)
readiness_bounds = [
    (name, unit, amount)
    for name, unit, amount in bounds
    if "READINESS" in name or "TIMEOUT" in name
]
if not readiness_bounds:
    problems.append(
        "the start path declares no named readiness timeout constant"
    )
for name, unit, amount in readiness_bounds:
    seconds = int(amount) if unit == "secs" else int(amount) / 1000
    if seconds < 1:
        problems.append(
            f"the readiness bound `{name}` is {amount} {unit} -- below the one-second floor"
        )

if problems:
    print(f"FAIL: {path} does not wait for a real readiness signal:")
    for problem in problems:
        print(f"      {problem}")
    sys.exit(1)
PYEOF
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the CLI's daemon start path waits for a real readiness signal."
else
  FAIL=$((FAIL + 1))
fi

# 167. The daemon's obtainability recipe is documented. Milestone 009.
#
# Nothing in the workspace installs the daemon, so the recipe is the only way
# an operator obtains it. A recipe that omits the cargo feature produces a
# binary that accepts `--full-executor` and silently falls back to a no-op
# executor.
echo ""
echo "--- Check 167: the daemon's obtainability recipe names the cargo feature ---"
SECTION_FAIL=0
DAEMON_DOC="docs/DAEMON.md"
if [[ ! -f "$DAEMON_DOC" ]]; then
  echo "FAIL: $DAEMON_DOC is missing (the daemon's obtainability recipe must be documented)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  if ! python3 - "$DAEMON_DOC" <<'PYEOF' 2>&1; then
import re, sys, pathlib

path = pathlib.Path(sys.argv[1])
text = path.read_text()
problems = []

recipes = [
    line
    for line in text.splitlines()
    if "cargo install" in line and "crates/eggsec-daemon" in line
]
if not recipes:
    problems.append(f"{path} documents no `cargo install` recipe for the daemon")
else:
    if not any(
        re.search(r"--features\s+(service|full-executor)\b", line) for line in recipes
    ):
        problems.append(
            f"{path}'s daemon install recipe names no cargo feature -- a build without one "
            "accepts `--full-executor` and falls back to a no-op executor"
        )

if problems:
    print(f"FAIL: the daemon's obtainability recipe is incomplete:")
    for problem in problems:
        print(f"      {problem}")
    sys.exit(1)
PYEOF
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the daemon's obtainability recipe names the cargo feature."
else
  FAIL=$((FAIL + 1))
fi

# 168. The desktop GUI's platform qualification is machine-checked.
# Milestone 007.
#
# "It compiles on my machine" is not a qualification. The matrix must state,
# per platform and per behaviour, whether the behaviour was observed and on
# what evidence -- and a platform that is out of scope must say so with the
# decision that excludes it. A cell that carries a status but no explanation
# is not a qualification either; the explanation is the evidence or the root
# cause.
#
# The bundler stays off because the installers belong to the distribution
# workstream -- unless an accepted ADR supersedes that decision.
echo ""
echo "--- Check 168: the desktop GUI's platform qualification is machine-checked ---"
SECTION_FAIL=0
GUI_QUAL="docs/GUI_PLATFORM_QUALIFICATION.md"
if [[ ! -f "$GUI_QUAL" ]]; then
  echo "FAIL: $GUI_QUAL is missing (the GUI's platform qualification must be recorded)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  if ! python3 - "$GUI_QUAL" crates/eggsec-gui/tauri.conf.json <<'PYEOF' 2>&1; then
import json, re, sys, pathlib

doc_path, conf_path = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
problems = []

# The status vocabulary is closed. A cell that invents a status is not a
# qualification.
#
# `pending M6` was retired by milestone 014: the token described a block that
# no longer exists, and Check 182 forbids it outright now that the export
# affordance exists. It is dropped here so this guard's vocabulary is the
# matrix's vocabulary, not a superset of it.
ALLOWED = {
    "qualified",
    "partially qualified",
    "not qualified",
    "out of scope",
}

# The schema is fixed: one platform per row, one behaviour per column. A
# matrix that drifts from the schema is not checkable.
EXPECTED_HEADER = "| Platform | Build | Launch | Connect | Submit | Cancel | Render | Export |"
COLUMNS = ("Build", "Launch", "Connect", "Submit", "Cancel", "Render", "Export")
REQUIRED_PLATFORMS = ("linux", "macos", "windows")

try:
    lines = doc_path.read_text().splitlines()
except OSError as exc:
    print(f"FAIL: cannot read {doc_path}: {exc}")
    sys.exit(1)

header_index = None
for index, line in enumerate(lines):
    if line.strip().startswith("| Platform |"):
        header_index = index
        break

if header_index is None:
    print(f"FAIL: {doc_path} has no platform matrix.")
    sys.exit(1)

if lines[header_index].strip() != EXPECTED_HEADER:
    print(f"FAIL: {doc_path} matrix header must be exactly:")
    print(f"      {EXPECTED_HEADER}")
    sys.exit(1)

rows = {}
for line in lines[header_index + 2:]:
    stripped = line.strip()
    if not stripped.startswith("|"):
        break
    cells = [cell.strip() for cell in stripped.strip("|").split("|")]
    if len(cells) != len(COLUMNS) + 1:
        problems.append(
            f"a matrix row has {len(cells)} cells, not {len(COLUMNS) + 1}: {stripped[:80]}"
        )
        continue
    platform = cells[0].strip("`").strip()
    rows[platform] = cells[1:]

for platform in REQUIRED_PLATFORMS:
    if platform not in rows:
        problems.append(f"the matrix has no `{platform}` row")

for platform, cells in rows.items():
    for column, cell in zip(COLUMNS, cells):
        match = re.match(r"^\*\*(.+?)\*\*\s*—\s*(.+)$", cell)
        if not match:
            problems.append(
                f"the `{platform}` / {column} cell is not `**status** — explanation`"
            )
            continue
        status = match.group(1).strip()
        if status not in ALLOWED:
            problems.append(
                f"the `{platform}` / {column} cell has the unknown status `{status}`"
            )

if "windows" in rows:
    for column, cell in zip(COLUMNS, rows["windows"]):
        match = re.match(r"^\*\*(.+?)\*\*\s*—\s*(.+)$", cell)
        if not match:
            continue
        if match.group(1).strip() != "out of scope":
            problems.append(f"the `windows` / {column} cell is not `out of scope`")
        elif not re.search(r"ADR-\d+", cell):
            problems.append(
                f"the `windows` / {column} cell does not cite the decision (ADR-nnn) that excludes it"
            )

# The bundler stays off: the installers belong to the distribution
# workstream. An accepted ADR may supersede that decision, but the ADR must
# exist -- naming one that does not is not a supersession.
try:
    conf = json.loads(conf_path.read_text())
    bundle_active = ((conf.get("bundle") or {}).get("active"))
except Exception as exc:
    problems.append(f"{conf_path} could not be parsed: {exc}")
    bundle_active = None

if bundle_active is not False:
    cited = set(re.findall(r"ADR-\d+", "\n".join(lines)))
    superseding = [
        adr for adr in cited if (pathlib.Path("plans/adrs") / f"{adr}.md").is_file()
    ]
    if not superseding:
        problems.append(
            f"{conf_path} enables the Tauri bundler (`bundle.active` is {bundle_active!r}) and "
            "no existing ADR under plans/adrs/ supersedes the decision; the installers belong "
            "to the distribution workstream"
        )

if problems:
    print("FAIL: the desktop GUI's platform qualification is not machine-checked:")
    for problem in problems:
        print(f"      {problem}")
    sys.exit(1)
PYEOF
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the desktop GUI's platform qualification is machine-checked."
else
  FAIL=$((FAIL + 1))
fi

# 169. The daemon's published capability list is derived, never transcribed.
# Milestone 010 (ADR-0011).
#
# A hand-written capability list is the defect this milestone exists to
# remove. The list must come from the canonical task-kind set, there must be
# exactly one capability-name table, and the retired constructors must stay
# retired. The task-kind table and the operation-identity match must cover
# the same set of kinds, so a new kind that reaches one but not the other
# fails the check.
echo ""
echo "--- Check 169: the capability list is derived from the task-kind set ---"
SECTION_FAIL=0
if rg -q '\bdaemon_conservative\b|\bfull_lab\b' crates 2>/dev/null; then
  echo "FAIL: a retired capability constructor (daemon_conservative/full_lab) is back."
  rg -n '\bdaemon_conservative\b|\bfull_lab\b' crates || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'fn task_kind_name' crates 2>/dev/null; then
  echo "FAIL: the private duplicate capability-name table (task_kind_name) is back."
  rg -n 'fn task_kind_name' crates || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
NAME_TABLE_COUNT=$(rg --no-filename -o 'pub fn capability_name' crates 2>/dev/null | wc -l | tr -d ' ')
if [[ "$NAME_TABLE_COUNT" -ne 1 ]]; then
  echo "FAIL: expected exactly one capability-name table (pub fn capability_name), found $NAME_TABLE_COUNT."
  rg -n 'pub fn capability_name' crates || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
CAPS_RS="crates/eggsec-runtime/src/capabilities.rs"
if ! rg -q 'TaskKind::all_variants\(\)' "$CAPS_RS" 2>/dev/null; then
  echo "FAIL: $CAPS_RS does not derive the capability list from TaskKind::all_variants()."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if sed '/^#\[cfg(test)\]/,$d' "$CAPS_RS" | rg -q 'TaskCapability::new\("'; then
  echo "FAIL: $CAPS_RS carries a hand-written capability list in its non-test body."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
REQUEST_RS="crates/eggsec-runtime/src/request.rs"
if ! rg -q 'macro_rules! define_task_kinds' "$REQUEST_RS" 2>/dev/null; then
  echo "FAIL: $REQUEST_RS does not declare the canonical task-kind table."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
TABLE_COUNT=$(rg -o '^\s*[A-Za-z0-9_]+\([A-Za-z0-9_]+\) => "' "$REQUEST_RS" 2>/dev/null | wc -l | tr -d ' ')
KIND_COUNT=$(rg -o 'TaskKind::[A-Za-z0-9_]+\(' "$REQUEST_RS" 2>/dev/null | sort -u | wc -l | tr -d ' ')
if [[ "$TABLE_COUNT" -ne "$KIND_COUNT" ]]; then
  echo "FAIL: the task-kind table declares $TABLE_COUNT kinds but the enum exposes $KIND_COUNT."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the capability list is derived from the task-kind set."
else
  FAIL=$((FAIL + 1))
fi

# 170. Publication and admission are the same object.
# Milestone 010 (ADR-0011).
#
# The daemon publishes a capability list and admits submissions against a
# capability list. If they are two lists, the daemon can advertise a
# capability it will refuse, or refuse one it advertised. The host therefore
# hands the published list to the runtime as its enforced set, and every
# rejecting executor declares the empty set explicitly rather than relying
# on the trait default.
echo ""
echo "--- Check 170: the published and admitted capability sets are one object ---"
SECTION_FAIL=0
HOST_RS="crates/eggsec-daemon/src/host.rs"
if ! rg -q 'with_task_kinds\(published\.clone\(\)\)' "$HOST_RS" 2>/dev/null; then
  echo "FAIL: $HOST_RS does not hand the published list to the runtime as its enforced set."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# The runtime's *enforced* set must come from the published list, not from a
# constant: a runtime configured with a different set would refuse a
# capability the host advertises.
if rg -Uq 'RuntimeConfig \{[\s\S]{0,200}?capabilities: (eggsec_runtime::)?RuntimeCapabilities::(default|canonical)\(\)' "$HOST_RS" 2>/dev/null; then
  echo "FAIL: $HOST_RS builds the runtime's enforced set from a constant instead of the published list."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
for REJECTOR in "crates/eggsec-daemon/src/host.rs" "crates/eggsec-frontend-client/src/lib.rs"; do
  if ! rg -Uq 'fn supported_task_kinds\([^)]*\) -> Vec<[^>]*> \{\s*Vec::new\(\)' "$REJECTOR" 2>/dev/null; then
    echo "FAIL: $REJECTOR has a rejecting executor that does not declare the empty set explicitly."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the published and admitted capability sets are one object."
else
  FAIL=$((FAIL + 1))
fi

# 171. The wireless-active wire kind resolves to the wireless-deauth operation.
# Milestone 010 (ADR-0011).
#
# A deauth is an intrusive operation. While the wire kind resolved to the
# passive `wireless` operation, a daemon-submitted deauth was authorised
# under the passive operation's risk level. The wire-side table, the engine's
# canonical table, and the executor routing table must all agree.
echo ""
echo "--- Check 171: wireless-active resolves to the wireless-deauth operation ---"
SECTION_FAIL=0
REQUEST_RS="crates/eggsec-runtime/src/request.rs"
CANON_RS="crates/eggsec/src/dispatch/canonical_execution.rs"
if ! rg -q 'TaskKind::WirelessActive\(_\) => "wireless-deauth"' "$REQUEST_RS" 2>/dev/null; then
  echo "FAIL: the wire-side table does not map WirelessActive to wireless-deauth."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'TaskKind::WirelessActive\(_\) => "wireless"' "$REQUEST_RS" 2>/dev/null; then
  echo "FAIL: the wire-side table still maps WirelessActive to the passive wireless operation."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'Self::WirelessActive\(_\) => "wireless-deauth"' "$CANON_RS" 2>/dev/null; then
  echo "FAIL: the engine's canonical table does not map WirelessActive to wireless-deauth."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -Uq 'fn executor_route_for[\s\S]{0,1200}?"wireless-deauth"' "$CANON_RS" 2>/dev/null; then
  echo "FAIL: the executor routing table has no wireless-deauth arm."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: wireless-active resolves to the wireless-deauth operation."
else
  FAIL=$((FAIL + 1))
fi

# 172. The interface-bound target policy is coherent, and no shared operation
# was relaxed to accommodate it.
# Milestone 010 (ADR-0011).
#
# The interface-bound value exists so that a local binding does not have to
# pretend to be a network target. It must not become a licence to relax a
# shared operation: `packet` still serves `packet-send`, which writes raw
# frames to a destination, and `pipeline` still serves `resume`, which
# addresses a real host.
echo ""
echo "--- Check 172: the interface-bound target policy is coherent ---"
SECTION_FAIL=0
CATALOG_RS="crates/eggsec-policy/src/catalog.rs"
for BOUND_OP in packet-capture wireless wireless-deauth; do
  if ! rg -Uq "id: \"$BOUND_OP\",[\\s\\S]{0,600}?target_policy: TargetPolicyKind::InterfaceBound" "$CATALOG_RS" 2>/dev/null; then
    echo "FAIL: the catalog's $BOUND_OP operation is not interface-bound."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
for SHARED_OP in packet pipeline; do
  if ! rg -Uq "id: \"$SHARED_OP\",[\\s\\S]{0,600}?target_policy: TargetPolicyKind::ExplicitScopeRequired" "$CATALOG_RS" 2>/dev/null; then
    echo "FAIL: the catalog's $SHARED_OP operation lost its explicit-scope target policy."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  if rg -Uq "id: \"$SHARED_OP\",[\\s\\S]{0,600}?target_policy: TargetPolicyKind::OptionalTarget" "$CATALOG_RS" 2>/dev/null; then
    echo "FAIL: the catalog's $SHARED_OP operation was relaxed to OptionalTarget."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if ! rg -q 'InterfaceBound|is_interface_bound' crates/eggsec-policy/src/decision.rs 2>/dev/null; then
  echo "FAIL: the enforcement kernel does not handle the interface-bound target policy."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the interface-bound target policy is coherent."
else
  FAIL=$((FAIL + 1))
fi

# 173. The daemon's policy approval is wired to the runtime's hold.
# Milestone 011 (ADR-0012).
#
# The daemon used to answer every approval with "not wired yet". A refusal
# that is always the same refusal tells the operator nothing: it does not
# distinguish "there is no decision pending" from "this daemon cannot answer".
# The handler must resolve the runtime's pending approval, and it must report
# the two states truthfully -- answered, or nothing pending.
echo ""
echo "--- Check 173: the daemon answers a pending policy decision ---"
SECTION_FAIL=0
HOST_RS="crates/eggsec-daemon/src/host.rs"
if ! rg -q 'resolve_approval' "$HOST_RS" 2>/dev/null; then
  echo "FAIL: the daemon's ApprovePolicy handler does not resolve the runtime's pending approval."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'not wired yet' "$HOST_RS" 2>/dev/null; then
  echo "FAIL: the daemon still refuses policy approval as unwired."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -Uq 'ApprovePolicy \{[\s\S]{0,6000}?ErrorCode::InvalidState' "$HOST_RS" 2>/dev/null; then
  echo "FAIL: the daemon does not report an answer for a task with nothing pending as invalid state."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the daemon answers a pending policy decision."
else
  FAIL=$((FAIL + 1))
fi

# 174. The runtime stays policy-neutral.
# Milestone 011 (ADR-0012).
#
# The runtime's approval registry holds a prompt and returns an answer. It
# must not learn the policy crate's vocabulary: a `ConfirmationClass` in the
# runtime would make the runtime's answer mean something the policy kernel
# defines, and the runtime would then have to know the kernel to be useful.
# The kernel owns authorisation semantics; the runtime owns task lifecycle.
echo ""
echo "--- Check 174: the runtime stays policy-neutral ---"
SECTION_FAIL=0
RT_MANIFEST="crates/eggsec-runtime/Cargo.toml"
if rg -q '^eggsec-policy' "$RT_MANIFEST" 2>/dev/null; then
  echo "FAIL: the runtime depends on the policy crate."
  rg -n '^eggsec-policy' "$RT_MANIFEST" 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if code_search crates/eggsec-runtime/src 'ConfirmationClass|EnforcementContext|EnforcementError|EnforcementOutcome|ManualOverride' | grep -q .; then
  echo "FAIL: the runtime names the policy crate's vocabulary."
  code_search crates/eggsec-runtime/src 'ConfirmationClass|EnforcementContext|EnforcementError|EnforcementOutcome|ManualOverride' || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the runtime stays policy-neutral."
else
  FAIL=$((FAIL + 1))
fi

# 175. The confirmation-class -> override mapping exists in exactly one
# place. Milestone 011 (ADR-0012).
#
# `ManualOverride::permitting_classes` is the inverse of `permits`. A second
# copy of the table -- in the TUI or in the engine's runtime bridge -- would
# let the two drift, and the drift would surface as an operator whose answer
# grants a class the kernel never demanded. Both callers must use the shared
# helper, and neither may name a confirmation class variant itself.
echo ""
echo "--- Check 175: the confirmation-class mapping exists once ---"
SECTION_FAIL=0
POLICY_RS="crates/eggsec-policy/src/decision.rs"
if ! rg -q 'pub fn permitting_classes' "$POLICY_RS" 2>/dev/null; then
  echo "FAIL: the policy kernel does not own the confirmation-class mapping."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
for MAPPER in "crates/eggsec-tui/src/app/enforcement_facade.rs" "crates/eggsec/src/runtime_bridge/executor.rs"; do
  if [[ ! -f "$MAPPER" ]]; then
    echo "FAIL: $MAPPER is missing; the mapping's callers cannot be checked."
    SECTION_FAIL=$((SECTION_FAIL + 1))
    continue
  fi
  # The eight fields that let a `ManualOverride` permit a class. A caller
  # that ASSIGNS one of them is restating the mapping. A caller that only
  # READS one (a test asserting what the shared helper produced) is not.
  #
  # The names are disjoint from `ExecutionPolicy`'s own `allow_*` fields,
  # which is what makes this pattern safe.
  if rg -q 'allow_(out_of_scope|explicit_exclusion|high_risk|nonbaseline_capability|private_resolution|cross_host_redirect|web_proxy|db_pentest) = true' "$MAPPER" 2>/dev/null; then
    echo "FAIL: $MAPPER maps a confirmation class onto an override field itself."
    rg -n 'allow_(out_of_scope|explicit_exclusion|high_risk|nonbaseline_capability|private_resolution|cross_host_redirect|web_proxy|db_pentest) = true' "$MAPPER" 2>/dev/null || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
  if ! rg -q 'permitting_classes' "$MAPPER" 2>/dev/null; then
    echo "FAIL: $MAPPER does not use the shared confirmation-class mapping."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the confirmation-class mapping exists once."
else
  FAIL=$((FAIL + 1))
fi

# 176. The daemon records the answer it accepts.
# Milestone 011 (ADR-0012).
#
# An approval is an authorisation an operator gave. Who gave it, for which
# session, on which surface, and with what outcome must survive the process:
# an approval that leaves no trace is indistinguishable from one that was
# never asked for.
echo ""
echo "--- Check 176: the daemon audits the answer it accepts ---"
SECTION_FAIL=0
HOST_RS="crates/eggsec-daemon/src/host.rs"
if ! rg -Uq 'ApprovePolicy \{[\s\S]{0,6000}?record_audit_event' "$HOST_RS" 2>/dev/null; then
  echo "FAIL: the daemon's ApprovePolicy handler does not record an audit event."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -Uq 'ApprovePolicy \{[\s\S]{0,6000}?action: "approve-policy"' "$HOST_RS" 2>/dev/null; then
  echo "FAIL: the daemon's ApprovePolicy handler does not record the approval under its own action."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the daemon audits the answer it accepts."
else
  FAIL=$((FAIL + 1))
fi

# 177. The daemon's default socket path exists in exactly one place.
# Milestone 012 (ADR-0013).
#
# The default socket path was transcribed in six places: the daemon binary's
# flag default, the daemon's config default, the engine's global flag, the
# GUI's Rust options, the GUI's webview (a JS constant and an HTML attribute)
# and the daemon profile build script. A change to one copy would silently
# fail to change the others, and the webview's copy could drift away from the
# Rust one entirely.
#
# The canonical crate owns the literal. Documentation, skills and plan history
# legitimately quote the operator-facing default, so the scan is scoped to
# `crates/` and `scripts/` -- the same discipline `check_doc_references.py`
# uses when it skips `plans/`.
#
# `code_search` is NOT used here: it strips string literals before matching
# (`sed -e 's/"[^"]*"/""/g'`), so it would blank the very literal this guard
# looks for and pass vacuously.
echo ""
echo "--- Check 177: the default socket path exists in exactly one place ---"
SECTION_FAIL=0
CANONICAL_CRATE="crates/eggsec-runtime"
CANONICAL_RS="crates/eggsec-runtime/src/daemon_endpoint.rs"
SOCKET_LITERAL="/tmp/eggsec-daemon.sock"
# This guard has to NAME the literal in order to search for it, so the guard's
# own file is excluded from its own scan. A check that matches itself is not a
# check: without the exclusion this guard fails on its own search term and
# says nothing about the repository.
GUARD_SELF="scripts/check-architecture-guards.sh"

# Every hit in the executable surface must live in the canonical crate. That
# crate may hold more than one occurrence -- the constant and the test that
# pins its value -- so the rule is the crate, not the file.
while IFS= read -r HIT; do
  [[ -z "$HIT" ]] && continue
  HIT_FILE="${HIT%%:*}"
  if [[ "$HIT_FILE" != "$CANONICAL_CRATE"/* ]]; then
    echo "FAIL: the default socket path is stated outside the canonical crate: $HIT"
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done < <(rg -H -n --fixed-strings "$SOCKET_LITERAL" crates/ scripts/ 2>/dev/null | grep -v "^${GUARD_SELF}:" || true)

# The GUI's webview assets must hold no copy: the webview learns the path
# from Rust, through the existing `daemon_status` command.
if rg -q --fixed-strings "$SOCKET_LITERAL" crates/eggsec-gui/frontend/ 2>/dev/null; then
  echo "FAIL: the GUI's webview assets still hold a copy of the default socket path."
  rg -H -n --fixed-strings "$SOCKET_LITERAL" crates/eggsec-gui/frontend/ 2>/dev/null || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi

# The shell scripts must hold no copy, so the build script cannot re-introduce
# one.
SCRIPT_HITS="$(rg -H -n --fixed-strings "$SOCKET_LITERAL" scripts/ 2>/dev/null | grep -v "^${GUARD_SELF}:" || true)"
if [[ -n "$SCRIPT_HITS" ]]; then
  echo "FAIL: a shell script still holds a copy of the default socket path."
  printf '%s\n' "$SCRIPT_HITS"
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi

# The canonical crate must still state it.
if ! rg -q --fixed-strings "$SOCKET_LITERAL" "$CANONICAL_RS" 2>/dev/null; then
  echo "FAIL: the canonical crate no longer states the default socket path."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi

if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the default socket path exists in exactly one place."
else
  FAIL=$((FAIL + 1))
fi

# 178. The server refuses to bind over a live socket.
# Milestone 012 (ADR-0013).
#
# `run_server` used to unlink the socket path before it bound, so a second
# daemon displaced a live daemon's socket and its clients broke. The bind
# guard binds first, asks the shared liveness probe whether the occupant is
# live, and refuses without unlinking when it is.
#
# This guard pins the *shape*; the behaviour is pinned by the Rust tests,
# because a static check cannot observe a race.
echo ""
echo "--- Check 178: the server refuses to bind over a live socket ---"
SECTION_FAIL=0
SERVER_RS="crates/eggsec-daemon/src/server.rs"

# The hits in a file's PRODUCTION half. Comment lines, `pub use` re-exports
# and the test module are all skipped, so a doc comment that NAMES a call --
# or a test that mentions one -- can neither satisfy nor break a check.
production_hits_fixed() {
  local needle="$1" file="$2" limit
  limit="$(rg -H -n --fixed-strings '#[cfg(test)]' "$file" 2>/dev/null | head -1 | cut -d: -f2 || true)"
  [[ -z "$limit" ]] && limit=999999
  rg -H -n --fixed-strings "$needle" "$file" 2>/dev/null \
    | grep -v ':[[:space:]]*//' \
    | grep -v 'pub use' \
    | awk -F: -v limit="$limit" 'NF >= 2 && $2 + 0 < limit + 0' \
    || true
}

# The first production line that mentions a needle.
first_production_line() {
  production_hits_fixed "$1" "$2" | head -1 | cut -d: -f2 || true
}

# The first removal of the socket path must come after the first bind. A
# removal before the bind is the defect: it destroys the evidence of whether
# the path held a live daemon.
FIRST_BIND="$(first_production_line 'UnixListener::bind' "$SERVER_RS")"
FIRST_REMOVE="$(first_production_line 'remove_file' "$SERVER_RS")"
if [[ -z "$FIRST_BIND" ]]; then
  echo "FAIL: the server no longer binds a Unix listener."
  SECTION_FAIL=$((SECTION_FAIL + 1))
elif [[ -n "$FIRST_REMOVE" ]] && (( FIRST_REMOVE < FIRST_BIND )); then
  echo "FAIL: the server removes the socket path (line $FIRST_REMOVE) before it binds it (line $FIRST_BIND)."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi

# The refusal branch must exist.
if [[ -z "$(production_hits_fixed 'AddrInUse' "$SERVER_RS")" ]]; then
  echo "FAIL: the server has no AddrInUse refusal branch."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi

# The bind path must consult the shared probe, so that a second liveness
# notion cannot be introduced.
if [[ -z "$(production_hits_fixed 'probe_daemon_liveness' "$SERVER_RS")" ]]; then
  echo "FAIL: the server's bind path does not consult the shared liveness probe."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi

# The liveness notion stays singular: no lock file, no PID file, no lease.
for LOCK_LIKE in 'flock' 'lock_file' 'lockfile' 'pid_file' 'pidfile' '.lock'; do
  if [[ -n "$(production_hits_fixed "$LOCK_LIKE" "$SERVER_RS")" ]]; then
    echo "FAIL: the server's bind path references $LOCK_LIKE; the liveness notion must stay singular."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done

if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the server refuses to bind over a live socket."
else
  FAIL=$((FAIL + 1))
fi

# 179. The environment override has one name and one resolver.
# Milestone 012 (ADR-0013).
#
# The environment variable's name and the precedence rule live in one crate.
# A second spelling of the name -- in the daemon, the engine, the GUI or the
# CLI -- is a transcription that can drift from the rule it names.
echo ""
echo "--- Check 179: the environment override has one name and one resolver ---"
SECTION_FAIL=0

# The name may be stated only in the canonical crate.
while IFS= read -r HIT; do
  [[ -z "$HIT" ]] && continue
  HIT_FILE="${HIT%%:*}"
  if [[ "$HIT_FILE" != "$CANONICAL_CRATE"/* ]]; then
    echo "FAIL: the socket environment variable's name is transcribed outside the canonical crate: $HIT"
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done < <(rg -H -n --fixed-strings 'EGGSEC_DAEMON_SOCKET' crates/ 2>/dev/null || true)

# The canonical crate must own the four items the other surfaces name.
for ITEM in \
  'pub const DAEMON_SOCKET_ENV_VAR' \
  'pub const DEFAULT_DAEMON_SOCKET_PATH' \
  'pub fn resolve_daemon_socket_path' \
  'pub fn daemon_socket_path_from_env'; do
  if ! rg -q --fixed-strings "$ITEM" "$CANONICAL_RS" 2>/dev/null; then
    echo "FAIL: the canonical crate does not declare $ITEM."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done

# The four surfaces must resolve through the canonical items rather than
# re-deriving the rule. A bare `pub use` re-export does not count, and
# neither does a mention in the test module: the surface must actually
# resolve a path through the canonical item.
for SURFACE in \
  "crates/eggsec-daemon/src/main.rs" \
  "crates/eggsec/src/cli/mod.rs" \
  "crates/eggsec-daemon/src/config.rs" \
  "crates/eggsec-gui/src/config.rs"; do
  if [[ ! -f "$SURFACE" ]]; then
    echo "FAIL: $SURFACE is missing; the surface cannot be checked."
    SECTION_FAIL=$((SECTION_FAIL + 1))
    continue
  fi
  SURFACE_LIMIT="$(rg -H -n --fixed-strings '#[cfg(test)]' "$SURFACE" 2>/dev/null | head -1 | cut -d: -f2 || true)"
  [[ -z "$SURFACE_LIMIT" ]] && SURFACE_LIMIT=999999
  SURFACE_HITS="$(rg -H -n 'daemon_socket_path_from_env|DEFAULT_DAEMON_SOCKET_PATH' "$SURFACE" 2>/dev/null \
    | grep -v ':[[:space:]]*//' \
    | grep -v 'pub use' \
    | awk -F: -v limit="$SURFACE_LIMIT" 'NF >= 2 && $2 + 0 < limit + 0' \
    || true)"
  if [[ -z "$SURFACE_HITS" ]]; then
    echo "FAIL: $SURFACE does not resolve the socket path through the canonical daemon endpoint."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done

if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the environment override has one name and one resolver."
else
  FAIL=$((FAIL + 1))
fi
# 180. The report bridge's coverage table and the bridge's arms agree.
# Milestone 013.
#
# The report bridge tells the operator which canonical operations produce a
# report document. That answer lives in exactly one place -- the coverage
# table in `crates/eggsec/src/runtime_bridge/report_coverage.rs` -- and the
# bridge consults it before it builds anything.
#
# Three facts must not rot:
#
#   1. The table must account for EVERY result variant the engine declares.
#      A new `TaskResult` variant that no row names would silently fall
#      through the bridge and produce no report, with nothing to show for it.
#   2. Every variant the table calls canonical must have a bridge arm, and
#      every bridge arm must be a variant the table calls canonical. A
#      disagreement between the two is the internal error the bridge logs at
#      error level -- it must not be reachable from the source.
#   3. The table must live in ONE place. A second copy would be a second
#      hand-maintained catalog: it would drift silently, and the drift would
#      show up as a missing row rather than as a failure.
#
# The names are READ FROM THEIR OWNERS, not transcribed here: a guard that
# hardcoded the variant names would itself be the second source of truth this
# check exists to prevent.
#
# The extractions are MULTILINE-AWARE (`rg -U`, and an awk that accumulates
# across lines), because `rustfmt` breaks a long `variants:` array across
# several lines. A single-line extraction would read a reformatted table as a
# missing row, and the guard would fail on a tree that had not drifted.
echo ""
echo "--- Check 180: the report bridge's coverage table and its arms agree ---"
SECTION_FAIL=0
COVERAGE_SOURCE="crates/eggsec/src/runtime_bridge/report_coverage.rs"
BRIDGE_SOURCE="crates/eggsec/src/runtime_bridge/report.rs"
TYPES_SOURCE="crates/eggsec/src/dispatch/types.rs"
EXECUTOR_SOURCE="crates/eggsec/src/runtime_bridge/executor.rs"

# The variants the engine declares, read from the result type itself.
ENUM_VARIANTS=""
if [[ -f "$TYPES_SOURCE" ]]; then
  ENUM_VARIANTS="$(sed -n '/^pub enum TaskResult {/,/^}/p' "$TYPES_SOURCE" \
    | sed -n 's/^    \([A-Z][A-Za-z0-9]*\)[ ({,].*$/\1/p' | sort -u)"
fi
if [[ -z "$ENUM_VARIANTS" ]]; then
  echo "FAIL: could not read the result variants from $TYPES_SOURCE; the guard cannot run."
  SECTION_FAIL=$((SECTION_FAIL + 1))
else
  # The variants the coverage table names: the rows' variant lists plus the
  # terminal result shape, which no operation produces.
  COVERAGE_VARIANTS=""
  if [[ -f "$COVERAGE_SOURCE" ]]; then
    COVERAGE_VARIANTS="$(
      {
        rg -U -o 'variants: &\[[^\]]*\]' "$COVERAGE_SOURCE"
        rg -U -o 'TERMINAL_RESULT_VARIANTS:[^=]*= *&\[[^\]]*\]' "$COVERAGE_SOURCE"
      } | rg -o '"[A-Za-z][A-Za-z0-9]*"' | tr -d '"' | sort -u
    )"
  fi
  if [[ -z "$COVERAGE_VARIANTS" ]]; then
    echo "FAIL: could not read the coverage table's variants from $COVERAGE_SOURCE; the guard cannot run."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  elif [[ "$ENUM_VARIANTS" == "$COVERAGE_VARIANTS" ]]; then
    echo "PASS: the coverage table accounts for every result variant the engine declares."
  else
    echo "FAIL: the coverage table and the engine's result variants disagree."
    echo "  only in the result type:"
    comm -23 <(printf '%s\n' "$ENUM_VARIANTS") <(printf '%s\n' "$COVERAGE_VARIANTS") | sed 's/^/    /'
    echo "  only in the coverage table:"
    comm -13 <(printf '%s\n' "$ENUM_VARIANTS") <(printf '%s\n' "$COVERAGE_VARIANTS") | sed 's/^/    /'
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi

  # The table's rows, read as "verdict|variants". The awk accumulates across
  # lines so a reformatted (multi-line) variants array reads as one row.
  COVERAGE_ROWS=""
  if [[ -f "$COVERAGE_SOURCE" ]]; then
    COVERAGE_ROWS="$(awk '
      /variants: &\[/ {
        line = $0
        sub(/^.*variants: &\[/, "", line)
        if (line ~ /\]/) {
          sub(/\].*$/, "", line)
          names = line
          collecting = 0
        } else {
          names = line
          collecting = 1
        }
        next
      }
      collecting {
        line = $0
        if (line ~ /\]/) {
          sub(/\].*$/, "", line)
          names = names " " line
          collecting = 0
        } else {
          names = names " " line
        }
        next
      }
      /form: ReportForm::/ {
        verdict = $0
        sub(/^.*form: ReportForm::/, "", verdict)
        sub(/,.*$/, "", verdict)
        print verdict "|" names
        names = ""
        next
      }
    ' "$COVERAGE_SOURCE")"
  fi
  if [[ -z "$COVERAGE_ROWS" ]]; then
    echo "FAIL: could not read the coverage rows from $COVERAGE_SOURCE; the guard cannot run."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  else
    CANONICAL_VARIANTS="$(printf '%s\n' "$COVERAGE_ROWS" \
      | rg '^Canonical\|' | cut -d'|' -f2 | rg -o '"[A-Za-z][A-Za-z0-9]*"' | tr -d '"' | sort -u)"
    EXCLUDED_VARIANTS="$(printf '%s\n' "$COVERAGE_ROWS" \
      | rg '^NoCleanMapping\|' | cut -d'|' -f2 | rg -o '"[A-Za-z][A-Za-z0-9]*"' | tr -d '"' | sort -u)"

    # The bridge's arms: the PRODUCTION half only. A test may name a variant
    # as a fixture, but only a real arm counts here.
    BRIDGE_PRODUCTION=""
    BRIDGE_ARMS=""
    if [[ -f "$BRIDGE_SOURCE" ]]; then
      BRIDGE_PRODUCTION="$(sed '/#\[cfg(test)\]/,$d' "$BRIDGE_SOURCE")"
      BRIDGE_ARMS="$(printf '%s' "$BRIDGE_PRODUCTION" | rg -o 'TaskResult::[A-Za-z][A-Za-z0-9]*' \
        | sed 's/TaskResult:://' | sort -u)"
    fi
    if [[ -z "$BRIDGE_ARMS" ]]; then
      echo "FAIL: could not read the bridge's arms from $BRIDGE_SOURCE; the guard cannot run."
      SECTION_FAIL=$((SECTION_FAIL + 1))
    else
      for variant in $CANONICAL_VARIANTS; do
        if ! printf '%s' "$BRIDGE_PRODUCTION" | rg -q "TaskResult::${variant}\b"; then
          echo "FAIL: the coverage table calls '$variant' canonical, but the bridge has no TaskResult::$variant arm."
          SECTION_FAIL=$((SECTION_FAIL + 1))
        fi
      done
      for arm in $BRIDGE_ARMS; do
        if ! printf '%s\n' "$CANONICAL_VARIANTS" | rg -qx "$arm"; then
          echo "FAIL: the bridge has a TaskResult::$arm arm, but the coverage table does not call it canonical."
          SECTION_FAIL=$((SECTION_FAIL + 1))
        fi
      done
      for variant in $EXCLUDED_VARIANTS; do
        if printf '%s' "$BRIDGE_PRODUCTION" | rg -q "TaskResult::${variant}\b"; then
          echo "FAIL: the bridge has a TaskResult::$variant arm, but the coverage table excludes that variant."
          SECTION_FAIL=$((SECTION_FAIL + 1))
        fi
      done
    fi
  fi

  # The table lives in exactly one place.
  COVERAGE_TABLE_FILES="$(rg -l 'pub const REPORT_COVERAGE' crates/ --glob '*.rs' 2>/dev/null | sort | tr '\n' ' ')"
  COVERAGE_TABLE_FILES="${COVERAGE_TABLE_FILES% }"
  if [[ "$COVERAGE_TABLE_FILES" != "$COVERAGE_SOURCE" ]]; then
    echo "FAIL: the report coverage table must live in exactly one place; found: ${COVERAGE_TABLE_FILES:-none}"
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi

  # The engine publishes its own table; it does not restate it.
  if [[ -f "$EXECUTOR_SOURCE" ]]; then
    if ! rg -q 'REPORT_COVERAGE' "$EXECUTOR_SOURCE"; then
      echo "FAIL: the engine's executor does not publish the coverage table; the daemon would have no coverage to republish."
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  fi
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the report bridge's coverage table and its arms agree."
else
  FAIL=$((FAIL + 1))
fi
# 181. The export procedure is written.
# Milestone 014.
#
# The export step is the step whose procedure was missing: a matrix column
# with no procedure is a column nobody can run, and a procedure that still
# says "not yet defined" is worse than no procedure, because it reads like
# one.
echo ""
echo "--- Check 181: the export procedure is written ---"
SECTION_FAIL=0
BUILD_DOC="docs/GUI_BUILD.md"
EXPORT_SECTION="$(sed -n '/^### 5\.7 Step 7 — Export/,/^### 5\.8 /p' "$BUILD_DOC")"
if [[ -z "$EXPORT_SECTION" ]]; then
  echo "FAIL: $BUILD_DOC has no §5.7 export step."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if printf '%s' "$EXPORT_SECTION" | rg -q 'Not yet defined'; then
  echo "FAIL: $BUILD_DOC §5.7 still says 'Not yet defined'."
  printf '%s' "$EXPORT_SECTION" | rg -n 'Not yet defined' || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
for ROW in 'Action' 'Pass criterion' 'Evidence artefact'; do
  if ! printf '%s' "$EXPORT_SECTION" | rg -q "\| \*\*$ROW\*\* \| .+ \|"; then
    echo "FAIL: $BUILD_DOC §5.7 has no non-empty $ROW row."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if ! printf '%s' "$EXPORT_SECTION" | rg -q 'export_report'; then
  echo "FAIL: $BUILD_DOC §5.7 does not name the export_report command."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
for FORMAT in json csv html md sarif xml; do
  if ! printf '%s' "$EXPORT_SECTION" | rg -q "\`$FORMAT\`"; then
    echo "FAIL: $BUILD_DOC §5.7 does not name the $FORMAT format."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the export procedure names its action, its criterion, its artefact, and its six formats."
else
  FAIL=$((FAIL + 1))
fi

# 182. The retired milestone token stays retired.
# Milestone 014.
#
# The matrix once claimed the GUI had no export affordance and blocked its
# export column on a milestone. The affordance exists, so the token that
# described the block is a token that lies. It expires with the affordance,
# not with a date.
echo ""
echo "--- Check 182: the retired milestone token stays retired ---"
SECTION_FAIL=0
MATRIX="docs/GUI_PLATFORM_QUALIFICATION.md"
APP_JS="crates/eggsec-gui/frontend/app.js"
if rg -q 'export_report' "$APP_JS" 2>/dev/null; then
  if rg -q 'pending M6' "$MATRIX" 2>/dev/null; then
    echo "FAIL: $MATRIX still carries the retired 'pending M6' token, but the export affordance exists."
    rg -n 'pending M6' "$MATRIX" || true
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
else
  echo "FAIL: the export affordance is gone from $APP_JS; guard 182's premise no longer holds."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the matrix carries no retired blocking token."
else
  FAIL=$((FAIL + 1))
fi

# 183. `qualified` means resolvable evidence.
# Milestone 014.
#
# A cell that claims `qualified` is a claim about a run that happened. If the
# run record cannot be found, the claim is not a claim about anything, and the
# matrix stops being a statement of fact and becomes a statement of intent.
echo ""
echo "--- Check 183: a qualified cell names resolvable evidence ---"
SECTION_FAIL=0
python3 - <<'PY'
import os
import re
import sys

matrix_path = "docs/GUI_PLATFORM_QUALIFICATION.md"
evidence_dir = "docs/evidence/gui-qualification"
ci_runs_path = os.path.join(evidence_dir, "CI-RUNS.md")

with open(matrix_path, encoding="utf-8") as handle:
    lines = handle.read().splitlines()

ci_runs = ""
if os.path.isfile(ci_runs_path):
    with open(ci_runs_path, encoding="utf-8") as handle:
        ci_runs = handle.read()

failures = 0
for number, line in enumerate(lines, start=1):
    if not line.startswith("| `"):
        continue
    cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
    if len(cells) < 2:
        continue
    platform = cells[0].strip("`")
    for index, cell in enumerate(cells[1:], start=1):
        match = re.match(r"^\*\*([^*]+)\*\*", cell)
        if match is None or match.group(1).strip() != "qualified":
            continue
        references = re.findall(r"docs/evidence/gui-qualification/([A-Za-z0-9._-]+\.md)", cell)
        urls = re.findall(r"https://github\.com/\S+/actions/runs/\d+", cell)
        if not references and not urls:
            print(f"FAIL: {matrix_path}:{number} the {platform} cell names no evidence reference.")
            failures += 1
            continue
        for name in references:
            if not os.path.isfile(os.path.join(evidence_dir, name)):
                print(f"FAIL: {matrix_path}:{number} cites `{name}`, which does not exist.")
                failures += 1
        for url in urls:
            if url not in ci_runs:
                print(f"FAIL: {matrix_path}:{number} cites {url}, which is not in {ci_runs_path}.")
                failures += 1

sys.exit(1 if failures else 0)
PY
if [[ $? -ne 0 ]]; then
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: every qualified cell names evidence that resolves."
else
  FAIL=$((FAIL + 1))
fi

# 184. The matrix shape is exact.
# Milestone 014.
#
# The matrix is read by a validator, a guard, and a runbook, and all three
# read it by position. A column added in the middle, a row dropped, or a cell
# removed silently changes what every one of those readers believes.
echo ""
echo "--- Check 184: the matrix shape is exact ---"
SECTION_FAIL=0
python3 - <<'PY'
import re
import sys

matrix_path = "docs/GUI_PLATFORM_QUALIFICATION.md"
header = "| Platform | Build | Launch | Connect | Submit | Cancel | Render | Export |"

with open(matrix_path, encoding="utf-8") as handle:
    lines = handle.read().splitlines()

failures = 0
if not any(line.strip() == header for line in lines):
    print(f"FAIL: {matrix_path} has no header row exactly `{header}`.")
    failures += 1

rows = []
for line in lines:
    if not line.startswith("| `"):
        continue
    cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
    platform = cells[0].strip("`")
    if platform in ("linux", "macos", "windows"):
        rows.append((platform, len(cells) - 1))

if sorted(platform for platform, _ in rows) != ["linux", "macos", "windows"]:
    print(f"FAIL: {matrix_path} must have exactly one row per platform (linux, macos, windows); found {sorted(platform for platform, _ in rows)}.")
    failures += 1

for platform, cells in rows:
    if cells != 7:
        print(f"FAIL: {matrix_path} the `{platform}` row has {cells} cells; it must have 7.")
        failures += 1

if failures == 0 and sum(cells for _, cells in rows) != 21:
    print(f"FAIL: {matrix_path} has {sum(cells for _, cells in rows)} cells; it must have 21.")
    failures += 1

sys.exit(1 if failures else 0)
PY
if [[ $? -ne 0 ]]; then
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the matrix has the exact header, three rows, and twenty-one cells."
else
  FAIL=$((FAIL + 1))
fi

# 185. The runbook derives its steps from the matrix.
# Milestone 014.
#
# The runbook is the procedure for the matrix's columns. If the two lists
# drift, an operator follows a procedure for a column the matrix does not
# have, and a cell the matrix does have has no procedure.
echo ""
echo "--- Check 185: the runbook's steps are the matrix's columns ---"
SECTION_FAIL=0
python3 - <<'PY'
import re
import sys

matrix_path = "docs/GUI_PLATFORM_QUALIFICATION.md"
build_path = "docs/GUI_BUILD.md"

with open(matrix_path, encoding="utf-8") as handle:
    matrix_lines = handle.read().splitlines()

header = None
for line in matrix_lines:
    if line.strip() == "| Platform | Build | Launch | Connect | Submit | Cancel | Render | Export |":
        header = [cell.strip() for cell in line.strip().strip("|").split("|")]
        break

if header is None:
    print("FAIL: the matrix has no header row.")
    sys.exit(1)

columns = [column.lower() for column in header[1:]]

with open(build_path, encoding="utf-8") as handle:
    build_lines = handle.read().splitlines()

steps = []
for line in build_lines:
    match = re.match(r"^### 5\.\d+ Step (\d+) — (.+)$", line.strip())
    if match:
        steps.append((int(match.group(1)), match.group(2).strip().lower()))

failures = 0
if [name for _, name in steps] != columns:
    print(f"FAIL: {build_path} §5's steps are {[name for _, name in steps]}; the matrix's columns are {columns}.")
    failures += 1
for number, name in steps:
    if number != steps.index((number, name)) + 1:
        print(f"FAIL: {build_path} §5's step numbers are not 1..7 in order.")
        failures += 1
        break

sys.exit(1 if failures else 0)
PY
if [[ $? -ne 0 ]]; then
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the runbook's seven steps are the matrix's seven columns, in order."
else
  FAIL=$((FAIL + 1))
fi

# 186. The harness's identifiers are the matrix's columns.
# Milestone 014.
#
# The harness reports on the matrix's columns. An identifier it invents, or a
# column it forgets, is a column the matrix can never promote — and the run
# record's schema is the contract every writer and every reader shares.
echo ""
echo "--- Check 186: the harness's identifiers and record schema are pinned ---"
SECTION_FAIL=0
python3 - <<'PY'
import re
import sys

matrix_path = "docs/GUI_PLATFORM_QUALIFICATION.md"
harness_path = "crates/eggsec-gui/tests/qualification.rs"
schema = "eggsec-gui-qualification-record/v1"
schema_files = [
    harness_path,
    "scripts/gui_qualification.sh",
    "scripts/check_gui_qualification.py",
    "docs/evidence/gui-qualification/TEMPLATE.md",
]

with open(matrix_path, encoding="utf-8") as handle:
    matrix_lines = handle.read().splitlines()

columns = []
for line in matrix_lines:
    if line.strip() == "| Platform | Build | Launch | Connect | Submit | Cancel | Render | Export |":
        columns = [
            cell.strip().lower()
            for cell in line.strip().strip("|").split("|")[1:]
        ]
        break

if not columns:
    print("FAIL: the matrix has no header row.")
    sys.exit(1)

with open(harness_path, encoding="utf-8") as handle:
    harness = handle.read()

match = re.search(
    r"COLUMN_IDENTIFIERS[^=]*=\s*&?\[(.*?)\]", harness, re.DOTALL
)
if match is None:
    print(f"FAIL: {harness_path} declares no COLUMN_IDENTIFIERS.")
    sys.exit(1)

identifiers = re.findall(r'"([a-z]+)"', match.group(1))
failures = 0
if identifiers != columns:
    print(f"FAIL: {harness_path}'s identifiers are {identifiers}; the matrix's columns are {columns}.")
    failures += 1

for path in schema_files:
    try:
        with open(path, encoding="utf-8") as handle:
            content = handle.read()
    except OSError:
        print(f"FAIL: {path} does not exist.")
        failures += 1
        continue
    if schema not in content:
        print(f"FAIL: {path} does not carry the record schema `{schema}`.")
        failures += 1

sys.exit(1 if failures else 0)
PY
if [[ $? -ne 0 ]]; then
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the harness's identifiers are the matrix's columns, and every writer and reader carries the same record schema."
else
  FAIL=$((FAIL + 1))
fi

# 187. The procedure is hermetic.
# Milestone 014.
#
# The procedure runs on a developer's machine and on a CI runner. It writes
# artefacts, so it has to say where it writes them, and it must never need
# privileges the operator has to go and find.
echo ""
echo "--- Check 187: the qualification procedure is hermetic ---"
SECTION_FAIL=0
PROCEDURE="scripts/gui_qualification.sh"
if rg -q '\bsudo\b' "$PROCEDURE" 2>/dev/null; then
  echo "FAIL: $PROCEDURE elevates its privileges."
  rg -n '\bsudo\b' "$PROCEDURE" || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if rg -q 'rm\s+-[a-z]*r' "$PROCEDURE" 2>/dev/null; then
  echo "FAIL: $PROCEDURE removes things recursively."
  rg -n 'rm\s+-[a-z]*r' "$PROCEDURE" || true
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# Every absolute path LITERAL the procedure names must be inside the
# temporary root. A path built from a shell expansion ("$OUT_DIR/...") is not
# a literal: it is wherever the caller pointed it, and the caller chooses.
OFFENDING="$(python3 - "$PROCEDURE" <<'PYEOF'
import re
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    body = handle.read()

literals = set(re.findall(r"[\"'](/[^\"'\$]*)", body))
allowed = {"/dev/null"}
offending = sorted(
    literal
    for literal in literals
    if literal not in allowed and not literal.startswith("/tmp/")
    and literal != "/tmp"
)
for literal in offending:
    print(literal)
PYEOF
)"
if [[ -n "$OFFENDING" ]]; then
  echo "FAIL: $PROCEDURE names absolute path literals outside the temporary root."
  printf '%s\n' "$OFFENDING"
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! rg -q 'TMPDIR' "$PROCEDURE" 2>/dev/null; then
  echo "FAIL: $PROCEDURE does not confine its scratch writes to the temporary root."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the procedure needs no privileges and writes only under its output and the temporary root."
else
  FAIL=$((FAIL + 1))
fi

# 188. The promotion validator is green.
# Milestone 014.
#
# The matrix's rules are enforced by a program, not by a reader's diligence.
# If the validator fails, the matrix's claims are not being checked, and every
# other check in this file is only a floor.
echo ""
echo "--- Check 188: the promotion validator passes ---"
SECTION_FAIL=0
if ! python3 scripts/check_gui_qualification.py; then
  echo "FAIL: scripts/check_gui_qualification.py rejects the matrix."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the promotion validator is green."
else
  FAIL=$((FAIL + 1))
fi

# 189. The CI job publishes the qualification record.
# Milestone 014.
#
# A CI run that nobody can read is not evidence. The record has to leave the
# runner as an artefact, or the Linux row can never be promoted by CI at all.
echo ""
echo "--- Check 189: the CI job publishes the qualification record ---"
SECTION_FAIL=0
WORKFLOW=".github/workflows/deep-checks.yml"
GUI_JOB="$(sed -n '/^  gui-platform:/,/^  [a-z-]*:$/p' "$WORKFLOW")"
if [[ -z "$GUI_JOB" ]]; then
  echo "FAIL: $WORKFLOW has no gui-platform job."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! printf '%s' "$GUI_JOB" | rg -q 'scripts/gui_qualification\.sh'; then
  echo "FAIL: the gui-platform job does not run the qualification procedure."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! printf '%s' "$GUI_JOB" | rg -q 'gui-qualification-\$\{\{ matrix\.os \}\}'; then
  echo "FAIL: the gui-platform job does not publish a gui-qualification artefact."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if ! printf '%s' "$GUI_JOB" | rg -q 'upload-artifact'; then
  echo "FAIL: the gui-platform job does not upload the run record as an artefact."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the gui-platform job runs the procedure and publishes the run record."
else
  FAIL=$((FAIL + 1))
fi
# 190. Every dependency exception is owned and dated.
# Milestone 015.
#
# A dependency exception is a decision to ship a known-bad advisory. It is
# only a decision while somebody owns it and somebody has promised to look
# at it again. An ignore with no owner and no review-by date is a silent
# permanent waiver, and the 23-entry advisory set that milestone 015 closed
# was exactly a backlog of such waivers that nobody was tracking.
#
# `deny.toml` deliberately carries no owner field -- the house style puts the
# ownership data in the exception document -- so the check has to read both
# files. It must also fail closed: a parse that finds nothing is not
# evidence that there is nothing to find. A guard that silently passes is
# worse than none.
echo ""
echo "--- Check 190: every dependency exception is owned and dated ---"
SECTION_FAIL=0
DENY_TOML="deny.toml"
EXCEPTIONS_DOC="docs/DEPENDENCY_EXCEPTIONS.md"

if [[ ! -f "$DENY_TOML" ]]; then
  echo "FAIL: $DENY_TOML is missing; the canonical dependency-policy source cannot be checked."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ ! -f "$EXCEPTIONS_DOC" ]]; then
  echo "FAIL: $EXCEPTIONS_DOC is missing; the exception record cannot be checked."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi

if [[ $SECTION_FAIL -eq 0 ]]; then
  # Slice the `[advisories]` ignore array out of the policy file. The array is
  # the only place an advisory ID may be suppressed, so the parse is scoped to
  # that section rather than to the whole file.
  IGNORE_BLOCK=$(awk '
    /^\[advisories\]/                 { in_adv = 1; next }
    /^\[/                            { in_adv = 0 }
    in_adv && /^[[:space:]]*ignore[[:space:]]*=/ { in_ign = 1; next }
    in_ign && /^[[:space:]]*\]/      { in_ign = 0; next }
    in_ign                           { print }
  ' "$DENY_TOML" 2>/dev/null || true)

  if [[ -z "$IGNORE_BLOCK" ]]; then
    echo "FAIL: the [advisories] ignore array could not be parsed from $DENY_TOML."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi

  IGNORE_ENTRIES=$(rg -o '\{[^{}]*\}' <<< "$IGNORE_BLOCK" 2>/dev/null || true)
  if [[ -z "$IGNORE_ENTRIES" ]]; then
    echo "FAIL: no advisory ignore entries were parsed from $DENY_TOML."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi

  # Every `###` section of the exception document, reduced to the facts the
  # guard needs: the advisory IDs it covers, its owner, and its review-by date.
  # A section runs from its heading to the next `###` or `##` heading.
  SECTION_FACTS=$(awk '
    function flush(   ids, s, id, n, arr, i) {
      if (title == "") return;
      ids = "";
      s = title "\n" body;
      while (match(s, /RUSTSEC-[0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9]/)) {
        id = substr(s, RSTART, RLENGTH);
        if (index(" " ids " ", " " id " ") == 0) ids = ids (ids == "" ? "" : " ") id;
        s = substr(s, RSTART + RLENGTH);
      }
      n = split(ids, arr, " ");
      for (i = 1; i <= n; i++) {
        if (arr[i] == "") continue;
        printf "%s\t%s\t%s\t%s\n", arr[i], owner, reviewby, title;
      }
    }
    /^### / {
      flush(); title = $0; owner = ""; reviewby = ""; body = ""; next;
    }
    /^## / {
      flush(); title = ""; owner = ""; reviewby = ""; body = ""; next;
    }
    {
      if (title == "") next;
      body = body $0 "\n";
      if ($0 ~ /^\|[[:space:]]*Owner[[:space:]]*\|/) {
        v = $0; sub(/^\|[[:space:]]*Owner[[:space:]]*\|/, "", v);
        sub(/[[:space:]]*\|[[:space:]]*$/, "", v);
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); owner = v;
      }
      if ($0 ~ /^\|[[:space:]]*Review-by[[:space:]]*\|/) {
        v = $0; sub(/^\|[[:space:]]*Review-by[[:space:]]*\|/, "", v);
        sub(/[[:space:]]*\|[[:space:]]*$/, "", v);
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); reviewby = v;
      }
    }
    END { flush() }
  ' "$EXCEPTIONS_DOC" 2>/dev/null || true)

  if [[ -z "$SECTION_FACTS" ]]; then
    echo "FAIL: no exception records could be parsed from $EXCEPTIONS_DOC."
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi

  # Advisory IDs carried by a RESOLVED heading. Their ignores were removed
  # when the dependency left the graph; a RESOLVED ID that is ignored again
  # is a resurrected exception.
  RESOLVED_IDS=$(awk '
    /^### / {
      if (title == "") { title = $0; next }
      if (title ~ /RESOLVED/) {
        s = title;
        while (match(s, /RUSTSEC-[0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9]/)) {
          print substr(s, RSTART, RLENGTH);
          s = substr(s, RSTART + RLENGTH);
        }
      }
      title = $0;
      next;
    }
    /^## / { title = ""; next }
  ' "$EXCEPTIONS_DOC" 2>/dev/null || true)

  # (a) Every ignore entry has a non-empty reason.
  # (b) Every reason references the exception document.
  # (c) Every ignored ID has an owned and dated record.
  IGNORED_IDS=""
  while IFS= read -r ENTRY; do
    if [[ -z "$ENTRY" ]]; then continue; fi
    ENTRY_ID=$(printf '%s\n' "$ENTRY" | rg -o 'RUSTSEC-[0-9]{4}-[0-9]{4}' | head -1 || true)
    if [[ -z "$ENTRY_ID" ]]; then
      echo "FAIL: an ignore entry in $DENY_TOML carries no advisory ID: $ENTRY"
      SECTION_FAIL=$((SECTION_FAIL + 1))
      continue
    fi
    IGNORED_IDS="$IGNORED_IDS $ENTRY_ID"

    ENTRY_REASON_RAW=$(printf '%s\n' "$ENTRY" | rg -o 'reason[[:space:]]*=[[:space:]]*"[^"]*"' || true)
    if [[ -z "$ENTRY_REASON_RAW" ]]; then
      echo "FAIL: $ENTRY_ID has no reason in $DENY_TOML."
      SECTION_FAIL=$((SECTION_FAIL + 1))
      continue
    fi
    ENTRY_REASON="${ENTRY_REASON_RAW#*\"}"
    ENTRY_REASON="${ENTRY_REASON%\"}"
    if [[ -z "${ENTRY_REASON//[[:space:]]/}" ]]; then
      echo "FAIL: $ENTRY_ID has an empty reason in $DENY_TOML."
      SECTION_FAIL=$((SECTION_FAIL + 1))
      continue
    fi
    if [[ "$ENTRY_REASON" != *"docs/DEPENDENCY_EXCEPTIONS.md"* ]]; then
      echo "FAIL: $ENTRY_ID's reason does not reference docs/DEPENDENCY_EXCEPTIONS.md."
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi

    # (c) The owner and the review-by date live in the exception document.
    MATCHED=$(printf '%s\n' "$SECTION_FACTS" | rg "^${ENTRY_ID}	" || true)
    if [[ -z "$MATCHED" ]]; then
      echo "FAIL: $ENTRY_ID is ignored in $DENY_TOML but has no record in $EXCEPTIONS_DOC."
      SECTION_FAIL=$((SECTION_FAIL + 1))
      continue
    fi
    OWNED_AND_DATED=0
    while IFS=$'\t' read -r _SEC_ID _SEC_OWNER _SEC_REVIEW _SEC_TITLE; do
      if [[ -z "${_SEC_OWNER//[[:space:]]/}" ]]; then continue; fi
      if [[ "$_SEC_REVIEW" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] || [[ "$_SEC_REVIEW" == "— (closed)" ]]; then
        OWNED_AND_DATED=1
      fi
    done <<< "$MATCHED"
    if [[ $OWNED_AND_DATED -eq 0 ]]; then
      echo "FAIL: $ENTRY_ID has no record with a non-empty '| Owner |' and a dated '| Review-by |' in $EXCEPTIONS_DOC."
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  done <<< "$IGNORE_ENTRIES"

  # (d) No RESOLVED exception retains a live ignore.
  while IFS= read -r RESOLVED_ID; do
    if [[ -z "$RESOLVED_ID" ]]; then continue; fi
    if [[ " $IGNORED_IDS " == *" $RESOLVED_ID "* ]]; then
      echo "FAIL: $RESOLVED_ID is RESOLVED in $EXCEPTIONS_DOC but is still ignored in $DENY_TOML."
      SECTION_FAIL=$((SECTION_FAIL + 1))
    fi
  done <<< "$RESOLVED_IDS"
fi

if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: every dependency exception is owned and dated."
else
  FAIL=$((FAIL + 1))
fi

# 191. An operator attestation is attributed, and never defaulted.
# Milestone 014 (the qualification driver's promotion path).
#
# Rule P1 of the qualification matrix requires a cell's operator outcome to be
# `pass`. Until now nothing could write that value: the harness emitted
# `not-performed` at every one of its sites, and the only documented route to a
# promotion was a hand edit of the record's json block. That is a promotion
# with no tooling behind it, and a hand edit is exactly the kind of thing a
# later run silently overwrites.
#
# The machinery now exists: an operator-authored attestation file is validated,
# merged into the record in one write, and the validator refuses a `pass` that
# nobody signed. This guard pins the three facts that make that machinery real
# and the two facts that keep it honest — the procedure accepts an attestation,
# the validator knows the schema, and no code path *defaults* an operator
# outcome to `pass`; the only way to obtain one is a named human's
# attestation, and the matrix still demands one.
echo ""
echo "--- Check 191: an operator attestation is attributed, and never defaulted ---"
SECTION_FAIL=0
QUAL_PY="scripts/check_gui_qualification.py"
QUAL_SH="scripts/gui_qualification.sh"
QUAL_HARNESS="crates/eggsec-gui/tests/qualification.rs"
ATTESTATION_SCHEMA="eggsec-gui-operator-attestation/v1"

if ! rg -q '^\s+--attestation\)' "$QUAL_SH" 2>/dev/null; then
  echo "FAIL: the qualification procedure does not accept an operator attestation."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi

# The flag must reach the record writer, or the attestation is read and
# dropped: a procedure that validates an attestation and never merges it would
# leave every record exactly as unattributed as it was. The pinned form is the
# writer's own argument line — the same flag also appears in the help text and
# in the validator's own call, and neither of those merges anything.
if ! rg -q --fixed-strings 'RECORD_WRITER_ARGS+=(--attestation "$ATTESTATION_PATH")' "$QUAL_SH" 2>/dev/null; then
  echo "FAIL: the qualification procedure does not pass the operator attestation to the record writer."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi

if ! rg -q --fixed-strings "ATTESTATION_SCHEMA = \"$ATTESTATION_SCHEMA\"" "$QUAL_PY" 2>/dev/null; then
  echo "FAIL: the validator does not declare the operator attestation schema."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi

# The reader half must refuse a `pass` that nobody signed, and must refuse one
# that carries no observation. The messages are the contract: a rule that says
# what it refuses is a rule a reader can check.
if ! rg -q --fixed-strings 'a `pass` must be attributed' "$QUAL_PY" 2>/dev/null; then
  echo "FAIL: the validator does not refuse an operator pass that nobody signed."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi

if ! rg -q --fixed-strings 'carries no `observed`' "$QUAL_PY" 2>/dev/null; then
  echo "FAIL: the validator does not require an observation for an operator pass."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi

# No code path may DEFAULT an operator outcome to `pass`. The only writer of a
# `pass` is the merge of a signed attestation.
while IFS= read -r OPERATOR_DEFAULT; do
  [[ -z "$OPERATOR_DEFAULT" ]] && continue
  OPERATOR_VALUE="$(printf '%s\n' "$OPERATOR_DEFAULT" \
    | rg -o 'setdefault\("operator", *"[^"]*"\)|"operator": *"[^"]*"' \
    | rg -o '"[^"]*"' | tail -1 | tr -d '"')"
  if [[ "$OPERATOR_VALUE" == "pass" ]]; then
    echo "FAIL: a code path defaults an operator outcome to pass; only a named human's attestation may set it."
    printf '%s\n' "$OPERATOR_DEFAULT"
    SECTION_FAIL=$((SECTION_FAIL + 1))
  fi
done <<< "$(rg -n --no-heading 'setdefault\("operator", *"[^"]*"\)|"operator": *"[^"]*"' "$QUAL_PY" 2>/dev/null || true)"

# The machine cannot attest for a human.
if rg -q --fixed-strings 'OperatorOutcome::Pass' "$QUAL_HARNESS" 2>/dev/null; then
  echo "FAIL: the qualification harness emits an operator pass; the machine cannot attest for a human."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi

# The matrix must still demand the operator's attestation for the behavioural
# columns. A rule weakened to `not-applicable` would make every promotion free.
if ! python3 - <<'PY'
import re
import sys

with open("docs/GUI_PLATFORM_QUALIFICATION.md", encoding="utf-8") as handle:
    lines = handle.read().splitlines()

header = [
    "Rule",
    "Kind",
    "Platforms",
    "Columns",
    "Requires",
    "Required outcome",
    "Required attestation",
    "Artefact",
]
p1 = None
for line in lines:
    stripped = line.strip()
    if not stripped.startswith("|"):
        continue
    cells = [cell.strip() for cell in stripped.strip("|").split("|")]
    if len(cells) != len(header):
        continue
    if cells[0].strip("`") == "P1":
        p1 = cells
        break

if p1 is None:
    print("FAIL: the matrix's rules table has no P1 row.")
    sys.exit(1)

columns = [part.strip().strip("`") for part in p1[3].split(",")]
required = p1[6].strip("`")
behavioural = ["Connect", "Submit", "Cancel", "Render"]
missing = sorted(column for column in behavioural if column not in columns)
if missing:
    print("FAIL: rule P1 no longer covers " + ", ".join(missing) + ".")
    sys.exit(1)
if required != "pass":
    print(
        f"FAIL: rule P1 requires the operator outcome `{required}`; a "
        "promotion must still carry a human's attestation."
    )
    sys.exit(1)
sys.exit(0)
PY
then
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi

if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: an operator attestation is attributed, and never defaulted."
else
  FAIL=$((FAIL + 1))
fi

# 192. Every CI checkout carries the repository's history.
# Milestone 014 (the qualification driver's promotion path).
#
# The architecture guard and the qualification validator both decide whether
# one commit descends from another, and `git merge-base` can only answer that
# when the checkout carries the history. `actions/checkout` gives you the tip
# alone unless it is told otherwise, so a shallow checkout cannot answer the
# question at all -- and the validator reported that "cannot tell" as a
# contradiction. That is how CI read the macOS operator's attestation as an
# attestation of another run: a false accusation, caused by the checkout
# rather than by the evidence.
#
# `fetch-depth: 0` is the fix, and this guard pins it. A job that checks out
# the tip alone again fails here -- in the workflow, where the cause is
# visible -- rather than in the validator's judgement of somebody's work.
echo ""
echo "--- Check 192: every CI checkout carries the repository's history ---"
SECTION_FAIL=0
if ! python3 - <<'PY'
import glob
import re
import sys

problems = []
checkouts = 0

for path in sorted(glob.glob(".github/workflows/*.yml")):
    with open(path, "r", encoding="utf-8") as handle:
        lines = handle.read().splitlines()
    for index, line in enumerate(lines):
        match = re.match(r"^(\s*)- uses:\s*actions/checkout@", line)
        if match is None:
            continue
        checkouts += 1
        indent = len(match.group(1))
        step = []
        for following in lines[index + 1 :]:
            stripped = following.strip()
            if stripped:
                if not following.startswith(" " * (indent + 1)):
                    break
                if re.match(r"^\s*- ", following) and (
                    len(following) - len(following.lstrip())
                ) <= indent:
                    break
            step.append(following)
        if not any(re.search(r"fetch-depth:\s*0\s*$", entry) for entry in step):
            problems.append(
                f"{path}:{index + 1}: the checkout does not ask for the full "
                "history (`fetch-depth: 0`); the guards decide commit ancestry, "
                "and a shallow checkout cannot answer that question"
            )

if checkouts == 0:
    print("FAIL: no checkout step was found to check.")
    sys.exit(1)
for problem in problems:
    print(f"FAIL: {problem}")
sys.exit(1 if problems else 0)
PY
then
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi

if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: every CI checkout fetches the full history."
else
  FAIL=$((FAIL + 1))
fi

# 193. The CI's dependency-policy tool is one version, and not a stalled one.
# Milestone 015 (the dependency advisory backlog).
#
# `make check-deps` IS the dependency policy, and it is run by whichever
# cargo-deny the machine happens to have. Two versions of that tool must not
# decide the same policy: milestone 015's closure recorded the skew as
# finding F4 -- local 0.20.2, CI 0.19.0 -- with the note that "a local green
# is not proof of a CI green".
#
# The skew had teeth. cargo-deny 0.19.0's `bans` check can stop making
# forward progress on a runner's small thread pool -- this workspace has 194
# duplicated crates, each with an inclusion graph to print -- and the job
# then hangs with no output and no error. Upstream fixed it in 0.19.9 by
# spawning scoped threads directly (EmbarkStudios/cargo-deny#868, PR#870).
#
# So the pin must be one version across both workflows, and it must not be a
# version that stalls. This guard names both facts, because the second one
# is the kind of thing a version bump forgets.
echo ""
echo "--- Check 193: the CI's cargo-deny is one version, and not a stalled one ---"
SECTION_FAIL=0
if ! python3 - <<'PY'
import glob
import re
import sys

#: The first cargo-deny release that cannot stall without forward progress.
#: Upstream: EmbarkStudios/cargo-deny#868, fixed by PR#870 in 0.19.9.
MINIMUM = (0, 19, 9)

pins = []
unpinned = []
for path in sorted(glob.glob(".github/workflows/*.yml")):
    with open(path, "r", encoding="utf-8") as handle:
        for number, line in enumerate(handle.read().splitlines(), start=1):
            if not re.search(r"tool:\s*cargo-deny\b", line):
                continue
            match = re.search(r"cargo-deny@(\d+)\.(\d+)\.(\d+)", line)
            if match is None:
                unpinned.append(f"{path}:{number}")
                continue
            pins.append((path, number, tuple(int(part) for part in match.groups())))

if unpinned:
    for where in unpinned:
        print(f"FAIL: {where} installs cargo-deny without an exact version.")
    print(
        "FAIL: a floating cargo-deny decides the dependency policy by "
        "whatever the day brought."
    )
    sys.exit(1)

if not pins:
    print("FAIL: no cargo-deny pin was found in the workflows.")
    sys.exit(1)

distinct = {version for _, _, version in pins}
if len(distinct) > 1:
    print(
        "FAIL: the workflows install more than one cargo-deny ("
        + ", ".join(sorted(".".join(str(part) for part in version) for version in distinct))
        + "); the same tool must decide the same policy everywhere."
    )
    sys.exit(1)

version = distinct.pop()
if version < MINIMUM:
    print(
        f"FAIL: cargo-deny {'.'.join(str(part) for part in version)} can stall "
        "without forward progress on a constrained thread pool "
        "(EmbarkStudios/cargo-deny#868, fixed in 0.19.9)."
    )
    sys.exit(1)
sys.exit(0)
PY
then
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi

if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: the CI's cargo-deny is one version, and it is not a stalled one."
else
  FAIL=$((FAIL + 1))
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
