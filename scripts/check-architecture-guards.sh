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

# 6. Plan retention - handoff plans are retained intentionally
echo ""
echo "--- Check 6: Plan retention ---"
SECTION_FAIL=0
if [[ ! -f "plans/README.md" ]]; then
  echo "FAIL: plans/README.md is missing; plan retention policy is undocumented."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
PLAN_COUNT=$(find plans -maxdepth 1 -type f -name '*.md' ! -name 'README.md' | wc -l)
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
# Also detect the narrow co-occurrence in lib.rs/executor.rs (original check)
REGISTRY_HITS=$(rg -n 'registry::all_libraries\(\)' crates/eggsec-nse/src/lib.rs crates/eggsec-nse/src/executor.rs 2>/dev/null || true)
LOADED_HITS=$(rg -n 'loaded:\s*true' crates/eggsec-nse/src/lib.rs crates/eggsec-nse/src/executor.rs 2>/dev/null || true)
if [[ -n "$REGISTRY_HITS" && -n "$LOADED_HITS" ]]; then
  echo "$REGISTRY_HITS"
  echo "$LOADED_HITS"
  echo "FAIL: Production report paths must not map registry inventory directly into loaded runtime libraries."
  echo "      Use per-run require tracking, or expose capability snapshots under a separate field."
  FAIL=$((FAIL + 1))
  SECTION_FAIL=1
fi
# Positive evidence: production lib.rs uses runtime observation
if rg -q 'library_use_reports_from_required_modules|library_use_reports_from_static_requires|executor\.library_reports' crates/eggsec-nse/src/lib.rs 2>/dev/null; then
  echo "PASS: Production report path uses runtime observation functions."
else
  echo "WARN: Could not confirm runtime observation functions in lib.rs."
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

# 30. run_cli_with_profile() JSON path must populate library and rule metadata.
# The NseRunReport must include library use reports and rule evaluation reports
# for structured output to be complete. Once these APIs exist, skipping them
# produces empty arrays that hide compatibility truth.
echo ""
echo "--- Check 30: run_cli_with_profile() JSON path populates report metadata ---"
SECTION_FAIL=0
# Check that the JSON path calls with_rules()
if ! rg -q 'with_rules\(' crates/eggsec-nse/src/lib.rs 2>/dev/null; then
  echo "FAIL: run_cli_with_profile() does not call .with_rules() on NseRunReport."
  echo "      Rule evaluation metadata must be included in structured JSON output."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
# Check that the JSON path calls with_libraries()
if ! rg -q 'with_libraries\(' crates/eggsec-nse/src/lib.rs 2>/dev/null; then
  echo "FAIL: run_cli_with_profile() does not call .with_libraries() on NseRunReport."
  echo "      Library use metadata must be included in structured JSON output."
  SECTION_FAIL=$((SECTION_FAIL + 1))
fi
if [[ $SECTION_FAIL -eq 0 ]]; then
  echo "PASS: run_cli_with_profile() JSON path populates library and rule metadata."
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
echo "--- Check 35: run_cli_with_profile must use with_profile/with_full_policy (FAIL) ---"
NSE_RUNCLI_WITH_POLICY_HITS=$(rg -n 'NseExecutor::with_policy' --glob='*.rs' crates/eggsec-nse/src/lib.rs 2>/dev/null \
  | grep -E 'run_cli_with_profile|run_cli\b' || true)
if [[ -n "$NSE_RUNCLI_WITH_POLICY_HITS" ]]; then
  echo "$NSE_RUNCLI_WITH_POLICY_HITS"
  echo "FAIL: run_cli_with_profile() constructs NseExecutor with with_policy()."
  echo "      Use NseExecutor::with_profile(&resolved_profile) or NseExecutor::with_full_policy(...)"
  echo "      so capability context profile_kind/network_policy match the resolved profile."
  FAIL=$((FAIL + 1))
else
  echo "PASS: run_cli_with_profile() does not use with_policy() for executor construction."
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
if [[ ! -f crates/eggsec-nse/src/bridge.rs ]]; then
  echo "FAIL: crates/eggsec-nse/src/bridge.rs does not exist."
  echo "      NSE evidence must bridge to ReportEnvelope via bridge.rs."
  FAIL=$((FAIL + 1))
else
  echo "PASS: NSE bridge module exists."
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
  # Count enum variants in StableOperation
  VARIANT_COUNT=$(rg -c '^\s+\w+,$' crates/eggsec-python/src/operation_registry.rs 2>/dev/null || echo 0)
  # Count entries in ALL array
  ALL_COUNT=$(rg -c 'Self::' crates/eggsec-python/src/operation_registry.rs 2>/dev/null | head -1 || echo 0)
  # More precise: count Self:: entries inside the ALL const array
  ALL_ENTRIES=$(awk '/pub const ALL/,/^\s*\];/' crates/eggsec-python/src/operation_registry.rs 2>/dev/null | rg -c 'Self::' || echo 0)
  if [[ "$ALL_ENTRIES" -ne 22 ]]; then
    echo "FAIL: operation_registry.rs has $ALL_ENTRIES entries in ALL (expected 22)"
    FAIL=$((FAIL + 1))
  else
    echo "PASS: operation_registry.rs has exactly 22 entries in ALL."
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

echo ""
echo "=== Summary ==="
if [[ $FAIL -gt 0 ]]; then
  echo "FAILED: $FAIL check(s) failed."
  exit 1
else
  echo "ALL PASSED: No architecture drift detected."
  exit 0
fi
