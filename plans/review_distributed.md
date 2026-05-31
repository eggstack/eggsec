# Distributed Architecture Review
**Document:** architecture/distributed.md
**Reviewed:** 2026-05-31
**Accuracy:** High
**Lines Reviewed:** 93

## Verified Claims
- TaskType enum has 7 variants: Verified at `mod.rs:58-67`
- Task struct fields: Verified at `queue.rs:7-18`
- TaskResult struct: Verified at `queue.rs:20-27`
- TaskQueue thread-safe queue: Verified at `queue.rs:29-152` (uses `Arc<RwLock<...>>`)
- QueueError enum: Verified at `queue.rs:154-169`
- RemoteListener coordinator server: Verified at `remote.rs:27-467`
- RemoteClient worker client: Verified at `remote.rs:474-941`
- CommandExecutor secure execution: Verified at `command.rs:120-243`
- CommandMessage 6 variants: Verified at `command.rs:28-63`
- Worker struct: Verified at `worker.rs:65-708`
- TlsServer from PEM: Verified at `io.rs:110-161`
- TlsClient: Verified at `io.rs:163-225`
- StreamWrapper unified enum: Verified at `io.rs:19-108`
- LineWriter I/O wrapper: Verified at `io.rs:306-340`
- generate_psk function: Verified at `command.rs:272-277`
- PSK-based authentication: Verified at `remote.rs:286` (constant-time comparison via `subtle::ConstantTimeEq`)
- TLS encryption support: Verified at `io.rs:110-225` (rustls-based)
- Line-based JSON protocol: Verified at `io.rs:315-339` (`LineWriter` writes newline-delimited JSON)
- Task lifecycle (enqueue, dequeue, complete, reassign): Verified at `queue.rs:46-101`
- FxHashMap usage in queue.rs:13, command.rs:36, remote.rs:31: Verified

## Discrepancies
- [Line ranges]: Document claims `queue.rs:8-18` for Task, actual is `queue.rs:7-18` (derive attribute on line 7). Minor offset.
- [Line ranges]: Document claims `queue.rs:21-27` for TaskResult, actual is `queue.rs:20-27`. Minor offset.
- [Line ranges]: Document claims `queue.rs:155-169` for QueueError, actual is `queue.rs:154-169`. Minor offset.
- [Line ranges]: Document claims `command.rs:30-63` for CommandMessage, actual is `command.rs:28-63` (derive on line 28). Minor offset.
- [Heartbeat field]: Document says `Heartbeat { id, status }` with 2 fields, actual `CommandMessage::Heartbeat` at `command.rs:45-46` indeed has only `id` and `status` fields. However, the `Heartbeat` struct in `mod.rs:110-117` has 7 fields (worker_id, status, current_jobs, completed_jobs, failed_jobs, cpu_usage, memory_usage). The `CommandMessage::Heartbeat` variant is a simplified protocol message, not the full struct. This could confuse readers.
- [Missing details]: Document doesn't mention `RemoteListener` features: IP allowlist (`remote.rs:34,70-83`), connection limits (`remote.rs:17,209-213`), rate limiting (`remote.rs:18-19,121-140`), and periodic cleanup (`remote.rs:180-193`).
- [Missing details]: Document doesn't mention `RemoteClient` DNS caching (`remote.rs:514-532`) or the `ResponseMessage` type (`command.rs:65-118`).

## Bugs Found
- [No critical bugs]: The distributed module appears well-structured. The dequeue() fix (documented at queue.rs:57) is verified as correctly setting `worker_id` and `assigned_at_secs`.
- [Potential issue]: `TaskQueue::complete()` at `queue.rs:104-120` removes from in_progress but doesn't verify the task was actually in_progress (could silently no-op if result arrives after reassignment). Low severity since stale tasks are cleaned up.

## Improvement Opportunities
- [Documentation gap]: Add IP allowlist, connection limit, and rate limiting details to the doc. (priority: medium)
- [Documentation gap]: Document the `ResponseMessage` type and the full heartbeat flow including the struct in `mod.rs:110-117`. (priority: low)
- [Documentation gap]: Document the `WorkerRegistration` and `WorkerStatus` types in `mod.rs:93-107`. (priority: low)

## Stale Items
- [Bugs Fixed section]: The 2026-05-22 bug fixes are historical and accurate. No stale items found.
- [Performance Improvements section]: The FxHashMap migration is documented and verified. No stale items.
