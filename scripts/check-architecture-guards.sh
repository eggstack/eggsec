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

# 16. eggsec-daemon must not depend on TUI crates or engine crate
echo ""
echo "--- Check 16: Daemon free of TUI and engine dependencies ---"
HITS=$(rg -n 'ratatui|crossterm|eggsec-tui|eggsec =|eggsec-core' crates/eggsec-daemon/Cargo.toml 2>/dev/null || true)
if [[ -n "$HITS" ]]; then
  echo "$HITS"
  echo "FAIL: eggsec-daemon has TUI or engine dependencies. It must depend only on eggsec-runtime."
  FAIL=$((FAIL + 1))
else
  echo "PASS: Daemon free of TUI and engine dependencies."
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
  --glob='!wrappers.rs' --glob='!executor_core.rs' --glob='!socket.rs' --glob='!comm.rs' --glob='!dns.rs' --glob='!tests/' 2>/dev/null \
  | grep -v 'tests/' | head -20 || true)
if [[ -n "$NSE_NET_HITS" ]]; then
  echo "$NSE_NET_HITS"
  echo "INFO: Found direct TcpStream/UdpSocket usage in NSE libraries outside wrappers."
  echo "      socket.rs, comm.rs, and dns.rs are migrated (capability checks before connect)."
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
echo "=== Summary ==="
if [[ $FAIL -gt 0 ]]; then
  echo "FAILED: $FAIL check(s) failed."
  exit 1
else
  echo "ALL PASSED: No architecture drift detected."
  exit 0
fi
