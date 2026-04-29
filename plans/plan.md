# Slapper Improvement Plan - Master Consolidated

**Date**: 2026-04-29
**Status**: IN PROGRESS
**Priority**: High

---

## Executive Summary

This document is the single source of truth for all planned improvements to Slapper. It consolidates multiple research phases, security reviews, and TUI deep-dives into a wave-based execution model designed for parallelization.

**Current State**:
- **1,115** passing tests (base library)
- **1,364** passing tests (full features)
- **7** pre-existing AI test failures (ai::planner, ai::waf_bypass, ai::client)
- **~21** clippy warnings (mostly TUI-specific acceptable)
- **503** source files, **30** payload types, **29** TUI tabs.

---

## Parallel Execution Strategy (Waves)

The plan is organized into "Waves" and "Sub-waves" to allow multiple agents to work in parallel on independent domains.

| Wave | Domain | Parallel Agents | Key Items |
|------|--------|-----------------|-----------|
| **0** | **Stabilization** | 1 | Fix pre-existing AI test failures. |
| **1** | **Critical & Security** | 3-4 | Security vulnerabilities, compilation blockers, stress module hardening. |
| **2** | **TUI UX & Features** | 2 | Global search, clipboard, pause/resume, render optimizations. |
| **3** | **Core Quality & Refactor** | 2 | Dead code, error handling, file splitting, **Cookie Management**. |
| **4** | **Performance & Hardening** | 2 | FxHashMap migration, regex cache limits, fuzzer clone reduction. |
| **5** | **Feature Enhancements** | 4-5 | Agent intelligence, plugin API, WAF/Fuzzer gaps, **Observability**, **Hot-Reload**. |
| **6** | **Long-term Capabilities**| 1-2 | Exploitation framework, cloud/mobile security, GUI. |
| **7** | **Documentation** | 1 | README, CAPABILITIES, ARCHITECTURE, **CI Templates**. |

---

## WAVE 0: Stabilization (Prerequisite)
*Goal: Fix pre-existing AI test failures to ensure a reliable baseline.*

### 0.1.1 Fix AI Planner & Client Tests
- **Issue**: 7 tests in `ai::planner`, `ai::waf_bypass`, and `ai::client` fail due to keyword matching logic and line count mismatches.
- **Fix**: Align `test_extract_content_valid_response` with the actual 4-line output. Refactor keyword matching in `ai/planner.rs` to handle flexible wording for "add stage" and "reduce duration".
- **Verification**: `cargo test --lib -p slapper -- ai` should return 100% success.

---

## WAVE 1: Critical Fixes & Security (High Priority)
... (previous Wave 1 content) ...

---

## WAVE 2: TUI UX & Features (High Priority)
... (previous Wave 2 content) ...

---

## WAVE 3: Core Quality & Refactoring

### Wave 3A: Error Handling & Standardization
... (previous Wave 3A content) ...

### Wave 3B: Architecture Refactoring (Large Files)
... (previous Wave 3B content) ...

### Wave 3C: Robust Cookie & Session Management
*Goal: Replace manual Set-Cookie parsing with a standard CookieStore.*

#### 3.3.1 Implement `reqwest::cookie::CookieStore`
- **Issue**: `tool/session.rs` uses manual header parsing for cookies, which fails on complex attributes (Domain, HttpOnly, Path).
- **Fix**: Enable `cookies` feature in `reqwest`. Implement a persistent `Jar` or custom `CookieStore` in `AgentSession`.
- **Note**: Ensure cookies are partitioned by `Target` to prevent cross-site leakage during concurrent scans.

---

## WAVE 4: Performance & Hardening
... (previous Wave 4 content) ...

---

## WAVE 5: Feature Enhancements

### Wave 5.1: Agent Intelligence & Observability
*Goal: Improve agent decisions and provide an audit trail.*

#### 5.1.1 File-Based Agent Observability
- **Fix**: Integrate `tracing-appender` for non-blocking, rotating JSON logs at `~/.config/slapper/logs/agent.log`. 
- **Context**: TUI swallows `stdout`; the autonomous agent needs a dedicated file-based audit trail for security compliance and debugging.

#### 5.1.2 Configuration Hot-Reloading
- **Fix**: Use the `notify` crate to watch `slapper.toml` and `portfolio.json`. 
- **Context**: Allow adding targets or changing agent intensity without restarting the long-running process and losing transient async state.

### Wave 5.2: Plugin & Fuzzer Gaps

#### 5.2.1 Stateful/Chained Fuzzing Payloads
- **Goal**: Enable fuzzing of multi-step business logic (e.g., Create -> Extract ID -> Unauthorized Access).
- **Implementation**: Extend `FuzzEngine` to allow piping outputs from one request into the `FuzzArgs` of the next, leveraging the existing `ChainExecutor`.

---

## WAVE 6: Long-term Capabilities
... (previous Wave 6 content) ...

---

## WAVE 7: Documentation & Integration

### Wave 7.1: CI/CD Pipeline Artifacts
- **Goal**: Provide production-ready pipeline templates.
- **Implementation**: Update `.github/workflows/security-scan.yml` and `.gitlab-ci.yml` to demonstrate ingestion of SARIF output into GitHub/GitLab security dashboards.

---
