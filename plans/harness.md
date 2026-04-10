# MCP/Agentic Capabilities Harness Plan

This plan addresses issues in the MCP protocol implementation and autonomous agent system, focusing on making the "harness" (tool-to-finding propagation) functional.

## Overview

The core problem is that the autonomous agent and MCP tools are **disconnected** - findings discovered by security tools never propagate back to the agent. The agent only receives empty `findings: vec![]` responses.

| Category | Issues | Priority |
|----------|--------|----------|
| **Harness** | Tool findings not propagated to agent/MCP | CRITICAL |
| Event System | `trigger_event()` stub, handlers never invoked | CRITICAL |
| Severity Types | `ResponseSeverity` vs `crate::types::Severity` mismatch | HIGH |
| Memory | Path collision, memory leak, empty `resolved_findings` | HIGH |
| MCP Protocol | Dead code, hardcoded catalog, missing pagination | MEDIUM |
| Alerting | Email/PagerDuty stubs, unused infrastructure | MEDIUM |
| Infrastructure | Scheduler queue, delegation, lifecycle - unused/partial | MEDIUM |

---

## Critical Issue: Tool Findings Not Propagated

### Root Cause Analysis

**The Problem**: All tool implementations discard actual findings and return `findings: vec![]`.

**Affected Files**:
- `tool/implementations/scanner.rs:184,202`
- `tool/implementations/fuzzer.rs:178,196`
- `tool/implementations/recon.rs:154,172`
- `tool/implementations/pipeline.rs:119,137`

**Why Findings Are Lost**:

1. Tool implementations call CLI run functions (e.g., `fuzzer::run_cli()`)
2. These functions return `Result<()>` - just success/failure
3. Tool implementations create empty `ToolResponse { findings: vec![] }`
4. The actual `FuzzSession` (which contains `results: Vec<FuzzResult>` and `findings: usize`) is dropped

**Key Discovery**: `FuzzEngine` has `run_return_session()` method that returns `FuzzSession` with all findings, but `run_cli()` doesn't use it.

---

## Implementation Strategy: Callback Approach

### Design

Add a `FindingCallback` type and modify tool implementations to use callback-enabled run functions:

```rust
/// Callback for streaming findings during tool execution
pub type FindingCallback = Box<dyn FnMut(tool::response::Finding) + Send + 'static>;

/// Run fuzzer with findings callback
pub async fn run_cli_with_callback(
    args: FuzzArgs,
    mut callback: FindingCallback
) -> Result<()> {
    let mut engine = engine::FuzzEngine::new(args)?;
    
    // Instrument the engine to call callback for each finding
    let findings_cb = Arc::new(Mutex::new(callback));
    
    // ... modified run logic that invokes callback for each finding
}
```

### Benefits

1. **Non-blocking**: Findings can be streamed as they're discovered
2. **Memory efficient**: Don't need to store all findings if not needed
3. **Flexible**: Tools can choose to ignore findings if not needed
4. **Backward compatible**: `run_cli()` can call `run_cli_with_callback()` with no-op callback

---

## Wave 1: Implement Finding Callback Infrastructure

### 1.1 Create Finding Types in tool/response.rs

**Location**: `tool/response.rs`

**Add conversion from fuzzer types to tool types**:

```rust
impl From<FuzzResult> for Finding {
    fn from(result: FuzzResult) -> Self {
        let severity = match result.detected_severity {
            crate::waf::types::Severity::Critical => ResponseSeverity::Critical,
            crate::waf::types::Severity::High => ResponseSeverity::High,
            crate::waf::types::Severity::Medium => ResponseSeverity::Medium,
            crate::waf::types::Severity::Low => ResponseSeverity::Low,
            crate::waf::types::Severity::Info => ResponseSeverity::Info,
        };

        Finding {
            id: uuid::Uuid::new_v4().to_string(),
            finding_type: FindingType::Vulnerability,
            severity,
            title: result.payload.description.clone(),
            description: result.leaks_found.join(", "),
            location: format!("{} - {}", result.payload.payload_type, result.payload.payload),
            evidence: None,
            cve_ids: vec![],
            remediation: None,
            references: vec![],
            metadata: serde_json::json!({
                "status_code": result.status_code,
                "response_time_ms": result.response_time_ms,
                "is_waf_blocked": result.is_waf_blocked,
                "is_anomaly": result.is_anomaly,
                "payload": result.payload.payload,
            }),
        }
    }
}
```

**Estimated**: 1 hour

---

### 1.2 Add `run_cli_with_callback` to Fuzzer Module

**Location**: `fuzzer/mod.rs`

**Add new function**:

```rust
/// Run the fuzzer CLI with a callback for findings
///
/// This allows the caller to receive findings as they're discovered,
/// rather than having them printed to stdout and discarded.
pub async fn run_cli_with_callback<F>(args: FuzzArgs, mut callback: F) -> Result<()>
where
    F: FnMut(tool::response::Finding) + Send + 'static,
{
    let mut engine = engine::FuzzEngine::new(args)?;
    let session = engine.run_return_session().await?;
    
    // Convert FuzzResults to Findings and invoke callback
    for result in session.results.iter().filter(|r| r.is_vulnerable()) {
        let finding = result.clone().into();
        callback(finding);
    }
    
    Ok(())
}
```

**Estimated**: 1-2 hours

---

### 1.3 Update FuzzerTool to Use Callback

**Location**: `tool/implementations/fuzzer.rs`

**Modify execute method**:

```rust
async fn execute(&self, request: ToolRequest) -> ToolResult<ToolResponse> {
    // ... existing setup code ...
    
    // Collect findings via callback
    let mut findings: Vec<Finding> = Vec::new();
    let findings_cb = |f: Finding| {
        findings.push(f);
    };
    
    // Call the callback-enabled run function
    let result = crate::fuzzer::run_cli_with_callback(args, findings_cb).await;
    
    // Return findings in response
    match result {
        Ok(_) => Ok(ToolResponse {
            // ...
            findings,  // Now populated!
        }),
        // ... error handling ...
    }
}
```

**Estimated**: 1 hour

---

### 1.4 Add Finding Conversion for Other Tools

**Location**: `tool/implementations/*.rs`

**For ScannerTool**: Convert `PortResult`, `ServiceFingerprint`, `EndpointResult` to `Finding`

**For ReconTool**: The recon module already has structured output - convert to `Finding` types

**For PipelineTool**: The pipeline runs multiple tools - need to aggregate findings from each stage

**Approach**: For now, focus on FuzzerTool (most findings). Scanner/Recon/Pipeline can be done in Wave 2.

**Estimated**: 4-6 hours

---

## Wave 2: Fix Event System

### 2.1 Implement `trigger_event()` in Agent

**Severity**: CRITICAL  
**Location**: `agent/mod.rs:248-251`

**Current stub**:
```rust
pub async fn trigger_event(&mut self, event: SecurityEvent) -> Result<()> {
    tracing::debug!("Event triggered: {:?}", event.event_type());
    Ok(())  // STUB - does nothing!
}
```

**Fix**:
```rust
pub async fn trigger_event(&mut self, event: SecurityEvent) -> Result<()> {
    tracing::debug!("Event triggered: {:?}", event.event_type());
    
    let handlers: Vec<_> = self.event_handlers.iter().collect();
    for handler in handlers {
        if handler.handles(&event) {
            handler.handle(&event, self).await?;
        }
    }
    Ok(())
}
```

**Estimated**: 30 minutes

---

### 2.2 Fix Severity Type Mismatch

**Severity**: HIGH  
**Location**: `agent/mod.rs:209-234`

**Issue**: `ResponseSeverity` (tool/response.rs) vs `crate::types::Severity` (types.rs) are different enums.

**Fix**: Add conversion method in `tool/response.rs`:

```rust
impl ResponseSeverity {
    pub fn to_agent_severity(&self) -> crate::types::Severity {
        match self {
            ResponseSeverity::Critical => crate::types::Severity::Critical,
            ResponseSeverity::High => crate::types::Severity::High,
            ResponseSeverity::Medium => crate::types::Severity::Medium,
            ResponseSeverity::Low => crate::types::Severity::Low,
            ResponseSeverity::Info => crate::types::Severity::Info,
            ResponseSeverity::None => crate::types::Severity::Info,
        }
    }
}
```

**Estimated**: 30 minutes

---

## Wave 3: Fix Memory Issues

### 3.1 Fix Target Path Collision

**Severity**: HIGH  
**Location**: `agent/memory.rs:107-115`

**Issue**: `https://example.com` and `http://example.com` both become `http__example.com`

**Fix**:
```rust
fn get_target_path(&self, target: &str) -> PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    // Normalize: ensure single scheme prefix
    let normalized = if target.starts_with("http://") || target.starts_with("https://") {
        target.to_string()
    } else {
        format!("https://{}", target)
    };
    
    // Use hash for collision avoidance
    let mut hasher = DefaultHasher::new();
    normalized.hash(&mut hasher);
    let hash = format!("{:x}", hasher.finish());
    
    // Extract a readable prefix from the target
    let safe_name: String = normalized
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("unknown")
        .chars()
        .take(50)
        .collect();
    
    self.storage_dir
        .join("targets")
        .join(format!("{}_{}.json", safe_name, &hash[..16]))
}
```

**Estimated**: 1 hour

---

### 3.2 Implement `resolved_findings` Detection

**Severity**: HIGH  
**Location**: `agent/memory.rs:242-270`

**Issue**: `resolved_findings` is always `Vec::new()`

**Fix**: Compare baseline IDs against current scan:

```rust
pub fn compare_with_baseline(
    &self,
    target: &str,
    findings: &[Finding],
) -> Result<BaselineComparison> {
    let target_path = self.get_target_path(target);

    let (baseline_ids, all_baseline_findings) = if target_path.exists() {
        let content = fs::read_to_string(&target_path)?;
        let memory: TargetMemory = serde_json::from_str(&content)?;
        let ids: HashSet<_> = memory.baselines.iter().cloned().collect();
        // Collect all findings from all baseline scans
        let all_findings: Vec<_> = memory.scans
            .iter()
            .flat_map(|s| s.findings.iter())
            .cloned()
            .collect();
        (ids, all_findings)
    } else {
        (HashSet::new(), Vec::new())
    };

    let current_ids: HashSet<_> = findings.iter().map(|f| f.id.clone()).collect();

    // Find resolved: in baseline but not in current
    let resolved_findings: Vec<Finding> = all_baseline_findings
        .into_iter()
        .filter(|f| baseline_ids.contains(&f.id) && !current_ids.contains(&f.id))
        .collect();

    // Find new: not in baseline but in current
    let new_findings: Vec<Finding> = findings
        .iter()
        .filter(|f| !baseline_ids.contains(&f.id))
        .cloned()
        .collect();

    let unchanged_count = findings.len() - new_findings.len();

    Ok(BaselineComparison {
        new_findings,
        resolved_findings,
        unchanged_count,
    })
}
```

**Estimated**: 1-2 hours

---

### 3.3 Fix Memory Leak in AlertRouter

**Severity**: HIGH  
**Location**: `agent/alerts.rs:70-104`

**Issue**: `recent_alerts` HashMap grows indefinitely

**Fix**: Add cleanup:

```rust
pub async fn send(&mut self, alert: &Alert) -> Result<()> {
    // Periodic cleanup when map gets large
    if self.recent_alerts.len() > 1000 {
        let cutoff = Instant::now() - Duration::from_secs(self.dedup_window_secs * 2);
        self.recent_alerts.retain(|_, instant| *instant > cutoff);
    }
    
    let dedup_key = self.make_dedup_key(alert);
    // ... rest unchanged
}
```

**Estimated**: 15 minutes

---

## Wave 4: MCP Protocol Improvements

### 4.1 Remove Dead Sampling Types

**Severity**: LOW  
**Location**: `tool/protocol/mcp/sampling.rs`

**Issue**: `SamplingRequest`/`SamplingResponse` defined but never used

**Fix**: Delete the file and remove from mod.rs exports (unless MCP spec requires them)

**Estimated**: 15 minutes

---

### 4.2 Make Vulnerability Catalog Dynamic

**Severity**: LOW  
**Location**: `tool/protocol/mcp/handlers.rs:465-488`

**Issue**: Hardcoded 18 vulnerabilities

**Fix**: Derive from tool capabilities

**Estimated**: 1 hour

---

### 4.3 Add Session Pagination

**Severity**: LOW  
**Location**: `tool/protocol/mcp/handlers.rs:849-881`

**Fix**: Add `offset` and `limit` params to `session/list`

**Estimated**: 1 hour

---

## Wave 5: Alerting Infrastructure

### 5.1 Implement Email Alert Channel

**Severity**: MEDIUM  
**Location**: `agent/alerts.rs:119-127`

**Current**: Only logs "Would send email..."

**Fix**: Use `letter` or `lettre` crate for SMTP

**Estimated**: 2-3 hours

---

### 5.2 Implement PagerDuty Alert Channel

**Severity**: MEDIUM  
**Location**: `agent/alerts.rs:128-134`

**Fix**: Use PagerDuty Events API v2 via `reqwest`

**Estimated**: 1-2 hours

---

### 5.3 Add Scheduler Queue Consumer

**Severity**: MEDIUM  
**Location**: `tool/agents/scheduler.rs`

**Issue**: Tasks queued but never processed

**Fix**: Add `TaskWorker` that dequeues and dispatches

**Estimated**: 3-4 hours

---

### 5.4 Lifecycle Manager - Add HTTP Ping

**Severity**: MEDIUM  
**Location**: `tool/agents/lifecycle.rs`

**Fix**: Ping `callback_url` in health checks

**Estimated**: 1-2 hours

---

## Implementation Order

### Phase 1: Harness (Critical) - START HERE
- [x] 1.1 Add `From<FuzzResult> for Finding` conversion
- [x] 1.2 Add `run_cli_with_callback` to fuzzer module
- [x] 1.3 Update FuzzerTool to use callback
- [ ] 1.4 (Optional) Add for Scanner/Recon/Pipeline

### Phase 2: Event System (Critical)
- [x] 2.1 Implement `trigger_event()`
- [x] 2.2 Fix severity type mismatch

### Phase 3: Memory (High)
- [ ] 3.1 Fix target path collision
- [ ] 3.2 Implement resolved_findings
- [ ] 3.3 Fix AlertRouter memory leak

### Phase 4: MCP Improvements (Medium)
- [ ] 4.1 Remove dead sampling types
- [ ] 4.2 Dynamic vulnerability catalog
- [ ] 4.3 Session pagination

### Phase 5: Alerting (Medium)
- [ ] 5.1 Email alerts
- [ ] 5.2 PagerDuty alerts
- [ ] 5.3 Scheduler consumer
- [ ] 5.4 Lifecycle ping

---

## Parallelization

### Phase 1 (Harness) - Sequential
- 1.1 must complete before 1.2
- 1.2 must complete before 1.3

### Phase 2 (Event) - Parallel with Phase 1
- 2.1 and 2.2 are independent of Phase 1

### Phase 3 (Memory) - Parallel with Phase 2
- 3.1, 3.2, 3.3 are independent

### Phase 4 (MCP) - Independent
- All items independent

### Phase 5 (Alerting) - Independent
- All items independent

---

## Dependencies

New dependencies may be needed:
```toml
# For Email (if implementing)
letter = "0.9"  # or lettre = "0.11"

# No new deps for path hashing (std::collections::hash_map::DefaultHasher)
# No new deps for PagerDuty (reqwest already available)
```

---

## Verification

All changes require:
```bash
cargo check --lib -p slapper
cargo test --lib -p slapper  
cargo clippy --lib -p slapper
```

Key tests to add:
- `test_fuzz_result_to_finding_conversion` in `tool/response.rs`
- `test_trigger_event_invokes_handlers` in `agent/mod.rs`
- `test_severity_conversion` in `tool/response.rs`
- `test_target_path_no_collision` in `agent/memory.rs`
- `test_resolved_findings_detection` in `agent/memory.rs`
- `test_alert_router_cleanup` in `agent/alerts.rs`

---

## Notes

- **prompts/read handler**: Already wired up at `handlers.rs:139` - no fix needed
- **Sampling types**: Defined but unused - candidate for removal
- **OpenAI handler**: Uses template strings, not real LLM - lower priority unless AI features are actively used
