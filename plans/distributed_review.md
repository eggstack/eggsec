# Distributed Architecture Review

**Document:** architecture/distributed.md
**Reviewed:** 2026-05-31
**Accuracy:** High

## Verified Claims

- [TaskType enum]: 7 variants (PortScan, ServiceFingerprint, EndpointDiscovery, Fuzz, WafTest, LoadTest, Recon) verified at `crates/slapper/src/distributed/mod.rs:59-67`
- [Task struct]: Fields match actual implementation at `crates/slapper/src/distributed/queue.rs:8-18`
- [TaskResult struct]: Fields match actual implementation at `crates/slapper/src/distributed/queue.rs:21-27`
- [TaskQueue]: Thread-safe queue with pending/in_progress/completed collections verified at `crates/slapper/src/distributed/queue.rs:29-152`
- [QueueError]: QueueFull/TaskNotFound variants verified at `crates/slapper/src/distributed/queue.rs:154-169`
- [RemoteListener]: Coordinator server with PSK auth, rate limiting, TLS support verified at `crates/slapper/src/distributed/remote.rs:27-467`
- [RemoteClient]: Worker client with DNS caching verified at `crates/slapper/src/distributed/remote.rs:474-941`
- [CommandExecutor]: Secure command execution with allowlist verified at `crates/slapper/src/distributed/command.rs:120-243`
- [CommandMessage]: 6 message types (Execute, Register, Heartbeat, Result, RequestTasks, AssignTasks) verified at `crates/slapper/src/distributed/command.rs:30-63`
- [Worker]: Worker node with heartbeat, task request, and task processing loops verified at `crates/slapper/src/distributed/worker.rs:65-708`
- [TlsServer]: TLS server from PEM files verified at `crates/slapper/src/distributed/io.rs:110-161`
- [TlsClient]: TLS client with `insecure-tls` feature gate verified at `crates/slapper/src/distributed/io.rs:163-225`
- [StreamWrapper]: Unified plain/TLS stream enum verified at `crates/slapper/src/distributed/io.rs:19-108`
- [LineWriter]: Line-based JSON I/O verified at `crates/slapper/src/distributed/io.rs:306-340`
- [generate_psk]: PSK generation (32 random bytes, hex-encoded) verified at `crates/slapper/src/distributed/command.rs:272-277`
- [Task lifecycle]: Enqueue -> dequeue -> execute -> complete -> reassign flow verified across queue.rs methods
- [dequeue() sets worker_id]: `task.worker_id = Some(worker_id.to_string())` verified at `queue.rs:65`
- [dequeue() sets assigned_at_secs]: `task.assigned_at_secs = Some(now)` verified at `queue.rs:66`
- [reassign_stale_tasks()]: Moves stale tasks back to pending verified at `queue.rs:74-101`
- [FxHashMap in queue.rs]: Uses `rustc_hash::FxHashMap` for `in_progress` and `Task.payload` verified at `queue.rs:1,13`
- [FxHashMap in command.rs]: Uses `FxHashMap` for `CommandMessage::Execute.env` verified at `command.rs:1,37`
- [FxHashMap in remote.rs]: Uses `FxHashMap` for `rate_limits` verified at `remote.rs:1,31`
- [PSK auth]: Constant-time comparison via `subtle::ConstantTimeEq` verified at `remote.rs:286`
- [Line-based protocol]: Newline-delimited JSON via `LineWriter` verified at `io.rs:315-329`
- [Override file reference]: `crates/slapper/src/distributed/AGENTS.override.md` exists

## Discrepancies

- [TaskType line range]: Documented as "59-67" which matches current code (`mod.rs:59-67`). Accurate.
- [Task struct line range]: Documented as "8-18" but actual is lines 7-18 (struct starts at line 7 with `#[derive...]`). Minor offset.
- [TaskResult line range]: Documented as "21-27" but actual is lines 20-27. Minor offset.
- [TaskQueue line range]: Documented as "29-154" but actual ends at line 152. Minor offset.
- [QueueError line range]: Documented as "155-169" but actual starts at line 154. Minor offset.
- [RemoteListener line range]: Documented as "27-390" but actual ends at line 467. The struct definition ends at line 37, and impl block extends to 467. Document appears to reference an older version where the impl was shorter.
- [RemoteClient line range]: Documented as "407-767" but actual starts at line 474 and ends at 941. Significant line drift from code additions.
- [CommandExecutor line range]: Documented as "106-229" but actual starts at line 120 and ends at 243. Line drift.
- [CommandMessage line range]: Documented as "30-48" but actual enum definition spans lines 30-63 (includes all variants). The doc only counted through line 48, missing Result/RequestTasks/AssignTasks variants.
- [Worker line range]: Documented as "64-557" but actual starts at line 65 and ends at 708. Significant line drift.
- [generate_psk line range]: Documented as "258-264" but actual is at lines 272-277. Line drift.
- [HashMap migration table]: The "Performance Improvements (2026-05-22)" table lists specific line numbers for FxHashMap migration. These line numbers are stale due to subsequent code changes. For example, `queue.rs:13` for `Task.payload` is now at `queue.rs:13` (still correct for the FxHashMap import, but the struct field is at line 13).

## Bugs Found

- None found in the documented architecture.

## Improvement Opportunities

- [Line number ranges]: The "Key Components" table line ranges should be updated to reflect current code positions. Many have drifted by 10-150 lines due to code additions. (priority: medium)
- [RemoteListener description]: The table says "27-390" for RemoteListener but the actual implementation extends to line 467. The doc should reflect the full range. (priority: low)
- [CommandMessage variants]: The table says "30-48" for CommandMessage but the enum has 6 variants spanning to line 63. The doc should include all variants. (priority: low)

## Stale Items

- [Key Components table line ranges]: All line ranges in the "Key Components" table are from an earlier version and have drifted. The components themselves are accurate but line references need updating. Recommended action: Update line ranges to current positions, or remove line ranges and use file references only.
- [Bug fix table line numbers]: The "Bugs Fixed (2026-05-22)" table references line numbers at time of fix. `queue.rs:57` for dequeue() is now at `queue.rs:57` (still correct). `worker.rs:132-161` for heartbeat is now at `worker.rs:133-184`. Minor drift. Recommended action: Keep table but note line numbers are approximate.
- [Performance Improvements table line numbers]: The FxHashMap migration table line numbers are mostly stale. `queue.rs:13` is approximately correct. `command.rs:36` is now at `command.rs:37`. `remote.rs:30` is now at `remote.rs:31`. Recommended action: Verify and update line numbers or remove them.
