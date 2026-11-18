# Security Assurance Hardening Milestone 004 — Secret and Credential Egress

Status: closed

Repository baseline: `77188755` (`feat/auto-20261117-7451c367`, 2026-11-17)

Source roadmap:

- `plans/subsystems/security-assurance-hardening-roadmap.md` §7 Milestone 4
- Findings A5, A6, A7 in §4 of the same roadmap

Long-term requirements:

- `plans/000-long-term-specification.md` (credential handling, authorized use)
- `plans/001-terminology-and-domain-model.md` (`SensitiveString`)
- `architecture/distributed.md`
- `architecture/egress_reuse_decision.md` (health checker ownership)

Applicable ADRs: none. No transport or protocol ownership changes.

Primary class: invariant

## 1. Objective

Close the three credential-egress defects: the TUI writing a password into the
process environment, the proxy health probe accepting forged TLS results, and
the distributed worker accepting any `eggsec` argument not on a denylist.

## 2. Why this milestone is ready

- No hard dependency. Soft dependency on Milestone 001 only because both edit
  `eggsec-tui` task-management files; 001 should land first or the two should be
  serialized.
- The wire contract already carries only an environment-variable **name**
  (`password_env`), validated by `normalize_password_env`
  (`crates/eggsec-tool-core/src/operation_request.rs:1418-1436`). No protocol
  change is needed to stop writing the value.
- The proxy health checker is documented as a health-only reqwest owner
  (`crates/eggsec-web-proxy/src/health.rs:33-42`), so narrowing its behavior
  does not touch the production dial path.
- The distributed worker already rejects the `env` field and bounds timeout and
  output; only the argument policy remains.

## 3. Current implementation evidence

### A6 — storage password

```rust
// crates/eggsec-tui/src/app/mod.rs:610-631
let password = self.tabs.storage.password().to_string();
if password.is_empty() {
    std::env::remove_var(crate::app::task_management::EGGSEC_STORAGE_PASSWORD_ENV);
    return;
}
unsafe { std::env::set_var(..., password) };
```

Three independent defects:

1. The variable name diverges from the documented contract:
   `pub(crate) const EGGSEC_STORAGE_PASSWORD_ENV: &str = "EGSEC_STORAGE_PASSWORD";`
   (`crates/eggsec-tui/src/app/task_management.rs:18`) while the CLI defaults to
   `EGGSEC_STORAGE_PASSWORD` (`crates/eggsec/src/cli/storage.rs:49`). Because the
   TUI both writes *and* sends this name, TUI-local dispatch works today; the
   defect is that an operator who exports the documented `EGGSEC_...` before
   launching the TUI is silently ignored, and the two surfaces disagree about
   the contract. (The audit report's phrasing "exposed and never consumed" was
   wrong: `crates/eggsec/src/storage/mod.rs:70-75` looks up whatever name is in
   `password_env`, and the TUI supplies its own.)
2. The value enters the process environment, where child processes inherit it
   and same-UID processes can read it (`/proc/<pid>/environ` on Linux). This
   process spawns `docker`, `git`, `adb`, `frida`, and `iptables`.
3. The `unsafe` justification claims the TUI is single-threaded, but
   `runner::block_on_ambient` builds `tokio::runtime::Runtime::new()`
   (`crates/eggsec-tui/src/app/runner.rs:239`), a multi-threaded runtime.
   `std::env::set_var` is unsound when another thread may read the environment.

The architectural contract is sound: only the variable **name** crosses the
wire, and the engine resolves the value locally (`storage/mod.rs:70-75`). The
TUI deviates by writing the value it should only be naming.

### A5 — proxy health probe (audit claim corrected)

```rust
// crates/eggsec-web-proxy/src/utils.rs:27-45
reqwest::Client::builder().danger_accept_invalid_certs(true)
```

```rust
// crates/eggsec-web-proxy/src/health.rs:111-127
reqwest::Proxy::all(&proxy_url)?.basic_auth(user, pass.expose_secret())
...
let response = client.get(&self.config.test_url).send().await?;
Ok(response.status().is_success())
```

**Correction to the audit report:** certificate verification is disabled for the
*test target* fetched through the proxy, not for the proxy hop. The proxy URL is
built as `http://` for `ProxyType::Http` and `ProxyType::Https`
(`health.rs:104-110`), so `Proxy-Authorization` follows that scheme. The defect
is therefore **health-signal integrity**: a network attacker on the target path
can present a forged certificate and return `2xx`, causing a dead or
intercepting proxy to be reported healthy. Credential disclosure is not caused
by the verification flag.

A second, separate question is recorded but not decided here:
`ProxyType::Https` displays as `"https"` (`config.rs:24-31`) but is dialed as an
`http://` proxy for health and mapped to `ProtocolSpec::Http` in the egress path
(`eggress_outbound.rs:76`). Either that is the intended "CONNECT-capable HTTP
proxy" semantic and needs documentation, or TLS-to-proxy is missing and needs a
separate milestone.

### A7 — distributed worker command policy

```rust
// crates/eggsec/src/distributed/command.rs:8,18-32,190
const ALLOWED_COMMANDS: &[&str] = &["eggsec"];
const FORBIDDEN_PATTERNS: &[&str] = &["../", "/etc/", "~/.ssh/", "--config", ...];
...
let mut cmd = Command::new(program);
```

A peer holding the PSK may pass any `eggsec` argument not matching those
substrings. Paths such as `/home/<user>/.bashrc` or `/app/results/...` are
accepted, so a compromised or malicious coordinator can direct file writes as
the worker user. `Command::new("eggsec")` also resolves through `PATH`.

## 4. Invariants that must not regress

- Only an environment-variable **name** crosses the daemon/tool boundary;
  never a secret value. `password_env` keeps its validated shape.
- `SensitiveString` remains zeroize-on-drop, redacted in `Debug`/`Display`, and
  compared with `ct_eq`.
- The distributed PSK stays constant-time compared
  (`crates/eggsec/src/distributed/remote.rs:484`), and the `env` field stays
  rejected.
- Remote command execution keeps the args-count, arg-length, output-cap, and
  `1..=300s` timeout bounds.
- Proxy health keeps its documented meaning: an HTTP(S) request **through** the
  selected proxy to `test_url` returning `2xx`; never mere TCP success.
- No secret value is logged, including in new refusal messages.
- The production egress path (Eggress) is not modified.

## 5. Scope

### In scope

- `eggsec-tui` storage-credential handoff and the misspelled constant.
- `eggsec-web-proxy` health-probe TLS behavior and its test.
- `eggsec` distributed worker argument policy and executable resolution.
- Tests, refusal messages, and the docs listed in §12.

### Explicitly out of scope

- A secret manager, vault, or keyring integration.
- Changing the PSK wire format or adding mutual TLS to the worker protocol.
- Making `ProxyType::Https` dial TLS (separate decision; record it).
- The production Eggress dial path.
- Any new environment variable as a credential channel.

## 6. Required production changes

### Core/domain

Distributed worker (`crates/eggsec/src/distributed/command.rs`):

- Do not apply a source-coded executable, subcommand, flag, or argument
  allowlist/denylist. Human-driven remote execution intentionally accepts the
  operator-selected program and argument tokens, including path-like strings.
- Retain resource bounds: at most 50 arguments (in addition to the program),
  1000 bytes per token, a 1–300 second timeout (300 seconds when omitted), and
  a 10 MB output cap. Continue rejecting custom environment variables.
- Keep platform executable resolution through the selected program name; do not
  require the Eggsec binary or an operator-configured absolute path.
- Document the accepted trust boundary: any peer that authenticates with the
  listener PSK can execute arbitrary programs as the listener account. PSK/TLS
  authentication grants that capability; it is not a command sandbox. Use TLS
  and a dedicated host reachable only by trusted operators.

### Storage and migrations

None. No secret is persisted by this milestone.

### Protocol and DTOs

`password_env: Option<String>` is unchanged on the wire. The TUI change alters
which value is placed in the field (a variable name, not a password), so
`normalize_password_env` validation applies to operator input for the first
time.

### Runtime and concurrency

Deleting the `set_var` call removes an unsound operation; no replacement
synchronization is needed. If the fallback design in §7 WP A is chosen, the
secret lives in a local variable moved into the dispatch context, not in a
global.

### Frontend or operator surface

TUI storage tab: the field stops being a password entry and becomes an
environment-variable name entry with the documented default
`EGGSEC_STORAGE_PASSWORD`. In embedded mode, show a clear diagnostic when the
variable is unset in the TUI process. In daemon mode, do not require a client-side
copy: the daemon resolves the name in its own process and reports absence through
the normal task error path. Neither mode may print or copy the secret value.

CLI behavior is unchanged: `EGGSEC_STORAGE_PASSWORD=... eggsec storage ...`
already works.

### Security and authorization

This milestone changes no policy evaluation. Human-operated remote command
execution has no source-coded executable or argument allow/deny policy; a valid
PSK-authenticated peer is trusted with OS-command execution under the listener
account. Existing authentication, scope controls in nested Eggsec operations,
timeout, argument-size, environment, and output bounds remain distinct controls.

### Documentation and static guards

- `architecture/distributed.md` and the distributed skill — arbitrary
  operator-selected commands, retained resource bounds, and the PSK/TLS trust boundary.
- `architecture/egress_reuse_decision.md` or `architecture/web_proxy.md` —
  health-probe semantics and the `ProxyType::Https` open question.
- `docs/python/` not affected.
- Guard: fail if `std::env::set_var` appears in `eggsec-tui` outside tests.
  Check whether an existing guard already covers it before adding a duplicate.

## 7. Ordered work packages

### Work package A — Storage credential handoff

Intent: stop writing secrets into the environment.

**Decision (preferred):** delete `publish_storage_password_env`; change the
storage tab field to an environment-variable **name**; validate it with the same
rules as `normalize_password_env`; document that embedded operators set the
variable before launching the TUI, while daemon-attached operators set it in the
daemon's execution environment. The embedded TUI may precheck its own environment;
daemon mode must not require a client-side secret copy. This removes the misspelled
constant, inheritance exposure, and unsound `set_var` in one change.

**Alternative (only if the product owner requires inline secret entry):** add an
in-memory, non-serialized handoff to the execution context that the engine
consults before `std::env::var`. If this path is chosen, stop and confirm the
design first — it adds a new engine API and must not put the secret into
`ToolRequest.params` (params are recorded in execution history and can reach
reports).

Required changes: whichever option is chosen, `EGGSEC_STORAGE_PASSWORD_ENV` is
either renamed to the correct spelling or deleted; no code path calls
`set_var` with a secret.

Acceptance evidence: a test asserting that after a storage dispatch, the process
environment contains no storage password (query the variable directly); the
existing storage dispatch tests still pass.

### Work package B — Proxy health integrity

Intent: make a forged certificate fail the probe.

Required changes:

- Build the health client with normal certificate verification. Keep the
  `test_url` and proxy configuration.
- If a self-signed *proxy test target* is a supported scenario, add an explicit,
  documented opt-in on the checker config; default off.
- With verification restored, the existing `basic_auth` stays. As defence in
  depth for any future relaxation, never combine proxy credentials with a
  verification-disabled client: make that combination unrepresentable in the
  checker's constructor rather than a caller convention.
- Define health for an authenticating proxy: a `407` proves reachability. Decide
  and document whether `407` counts as healthy or as "auth required" (preferred:
  distinct enum outcome rather than a boolean, if the type is local to this
  crate; otherwise document that `407` is not `2xx` and therefore unhealthy).

Acceptance evidence: a test with a self-signed test server proving the probe now
fails; a test proving a `2xx` from a valid server still passes.

### Work package C — Distributed remote-command trust boundary

Intent: keep human-driven remote execution useful without source-coded command
filters, while making the trust grant and resource bounds explicit.

Required changes: remove the executable/subcommand/argument-pattern filters;
retain the bounded argument count/length, environment rejection, timeout, and
output handling. Document that an authenticated peer may run arbitrary programs
as the listener account.

Acceptance evidence: an executor test launches a harmless non-Eggsec program
with a path-like argument and verifies the argument is passed through; focused
tests still prove each retained resource/environment bound.

## 8. Failure, cancellation, restart, and contention semantics

- Empty commands, excessive argument counts/lengths, and custom environments
  return a `ResponseMessage::error`; the connection stays usable and the queue
  is unaffected.
- No executable allowlist or path-pattern rejection runs before spawn.
- Health failure keeps the existing `max_failures` accounting; no new retry
  loop.
- Storage-credential absence is a connection-time error surfaced to the
  operator; it must not silently connect without credentials.

## 9. Compatibility and migration

- TUI: the storage field's meaning changes from secret to variable name. This is
  a deliberate UX change. Embedded execution resolves the value from the TUI
  process environment; daemon-attached execution resolves it from the daemon's
  environment and must not require duplicating the value on the client. The field
  defaults to the documented `EGGSEC_STORAGE_PASSWORD`; embedded mode reports an
  unset variable before submission, and daemon mode reports it through the normal
  task error path. Test both paths before claiming the workflow is preserved.
- Distributed: command selection remains human-driven and intentionally broad.
  A peer holding the listener PSK is trusted to execute arbitrary programs as
  the listener account; operators must protect the PSK, use TLS, and isolate the
  listener host. The closure record must state this boundary and the retained
  resource limits.
- Health: a self-signed test target now reports unhealthy unless the explicit
  opt-in is set.
- No on-disk or protocol migration.

## 10. Required tests

### Focused unit tests

- `password_env` name validation accepts/rejects the documented patterns (reuse
  existing tests in `eggsec-tool-core`).
- Remote executor runs a harmless non-Eggsec executable with path-like and
  option-like arguments unchanged.
- Argument count/length, custom environment, timeout, and output bounds remain
  covered; no executable or argument-pattern allow/deny table is asserted.

### Integration tests

- TUI storage dispatch in embedded mode with the variable unset produces a
  diagnostic and does not connect. A daemon-attached submission must not be
  blocked by an unset client-side variable; the daemon resolves the name.
- Proxy health against a self-signed server fails; against a valid server
  passes.
- Distributed `Execute` round-trip with an operator-selected harmless command
  and a bounded-input rejection (for example, custom environment variables).

### Restart and recovery tests

- Worker remains usable after a rejected command.

### Contention and cancellation tests

- Existing distributed timeout test still passes; no new spawn.

### Security and negative tests

- No storage password in `std::env::vars()` after dispatch.
- No secret in log output at `debug` level (assert on captured tracing where the
  harness allows it).
- Rejected worker command message does not echo a full argument vector.

### Migration and compatibility tests

- CLI `EGGSEC_STORAGE_PASSWORD=... eggsec storage ...` unchanged.

## 11. Required verification commands

```bash
make check
cargo test -p eggsec-tui
cargo test -p eggsec-web-proxy
cargo test -p eggsec --lib distributed
```

`make test-tui-pty` is expected if the storage tab behavior changes; run it when
the environment supports PTY tests and record the result.

Do not claim commands that were not actually run in the closure record.

## 12. Documentation updates

- `architecture/distributed.md` — §command policy.
- `architecture/egress_reuse_decision.md` and/or `architecture/web_proxy.md` —
  health semantics, `ProxyType::Https` open question.
- TUI storage tab help text and the operator-facing docs that mention the
  storage password.
- `SECURITY.md` if it makes a claim about secret handling that changes.

## 13. Acceptance criteria

1. No code path writes a credential into the process environment.
2. The misspelled `EGSEC_` constant no longer exists.
3. The proxy health probe verifies the target certificate by default.
4. The distributed worker accepts operator-selected programs and arguments
   without source-coded executable or argument-pattern filters; it documents
   that a PSK-authenticated peer can execute as the listener account.
5. Argument count/length, custom environment rejection, timeout, and output
   bounds remain enforced.
6. `make check` and the three crate-scoped test commands pass.

## 14. Stop conditions

Stop and report rather than improvise when:

- the storage-credential fix appears to require a daemon protocol change (it
  must not);
- a required authentication or resource bound cannot be retained without
  breaking the human-operated remote execution workflow;
- restoring TLS verification breaks a documented self-signed test fixture in a
  way that needs a new configuration surface the plan did not anticipate;
- a fix would place a secret in `ToolRequest.params`, `RunRequest`, or any
  serialized structure.

## 15. Closure evidence required

- Commit/PR per work package (or one PR with three clearly separated commits).
- The accepted remote-command trust boundary and retained resource limits,
  quoted in the closure record.
- Test output for each package.
- `make check` output.
- A statement of the `ProxyType::Https` decision (documented semantic vs
  deferred).
- The storage-field UX change called out explicitly.

## 16. Handoff notes

- `publish_storage_password_env` is called immediately before the request is
  built (`crates/eggsec-tui/src/app/task_management.rs:567`); read that call
  site before changing the field.
- Do not "fix" the typo by changing the CLI to read `EGSEC_`; the correct
  spelling is the one the CLI and docs already publish.
- The health checker's comment block at `health.rs:33-42` explains why reqwest
  stays in the tree; do not replace it with a hand-rolled client.
- Preserve unrelated user changes in the worktree.
