# Security Assurance Hardening Milestone 004 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/security-assurance-hardening/004-secret-and-credential-egress.md`

Source subsystem roadmap:

- `plans/subsystems/security-assurance-hardening-roadmap.md` §7 Milestone 4

Repository baseline reviewed: `77188755a41bee15aee9507055ac6b2761ff9271` (implementation remains uncommitted in the worktree)

Implementation commits or pull requests:

- None. No commit or PR was created, per instruction; evidence below describes the current working tree.

## 1. Executive finding

The three scoped egress defects are addressed without constraining human-operated remote command execution. Storage secrets are not written into the process environment; the proxy health probe verifies the test target's TLS certificate; and the PSK-authenticated remote command path remains operator-selected and broad, with its resource bounds and peer trust grant made explicit.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| Storage password value is not process-published | TUI storage/task management and engine storage dispatch changes; `cargo test -q -p eggsec --features rest-api,cli` | pass | The request carries a validated environment-variable name; the TUI no longer calls `set_var` with the password. Embedded mode checks the local process; daemon mode leaves resolution to the daemon. |
| Proxy health does not accept forged target TLS | `crates/eggsec-web-proxy/src/health.rs`, TLS utility, `tests/health_matrix.rs`; `cargo test -q -p eggsec-web-proxy` | pass | Proxy credentials for the configured proxy are separate from target certificate validation. |
| Human-operated remote commands remain flexible but bounded | `crates/eggsec/src/distributed/command.rs`; `cargo test -q -p eggsec --lib distributed` | pass | No hardcoded executable or argument allowlist/denylist; environment field, args, time, and output remain bounded. |
| PSK peer trust is explicit | `architecture/distributed.md` and `.opencode/skills/eggsec-distributed/SKILL.md` | pass | Authenticated peer can run commands as listener account; host isolation is part of the trust boundary. |

## 3. Production implementation evidence

The TUI uses the storage password's environment-variable name as the request contract and no longer writes its secret value into the process environment. Its missing-variable precheck applies only to embedded execution; daemon-attached requests let the daemon resolve the name in its own process, avoiding a required client-side secret copy. The health client uses normal certificate validation for its test target; the result reports reachability rather than pretending the test authenticates the proxy. The distributed executor retains its general `Command::new(program)` behavior for an operator-selected program, rejects custom environment injection, bounds argument count/length, caps command time at 300 seconds and output at 10 MiB, and uses piped output. These bounds do not impose an executable or argument-pattern filter.

## 4. Verification executed

### Commands run

```bash
cargo test -q -p eggsec-web-proxy
cargo test -q -p eggsec --lib distributed
cargo test -q -p eggsec --features rest-api,cli
cargo test -q -p eggsec-tui --features db-pentest,web-proxy,c2 --lib --no-fail-fast
make test-tui-pty
make check
make check-full
```

### Results

- Web proxy suite: 395 unit tests and all reported integration targets passed; two tests were explicitly ignored.
- Distributed engine tests: 35 passed.
- REST/CLI engine suite, representative TUI profile (1,077 passed), `make check`, `make check-full`, and PTY smoke all passed.

## 5. Invariant review

- Only a validated variable name crosses the request boundary; the executing process (embedded engine or daemon) resolves the secret value from its own environment, and the TUI never writes that value into the environment.
- Target TLS verification is enabled; the health check does not use a verification-disabled client to establish a positive signal.
- The distributed worker keeps command and output/time bounds, rejects caller-supplied environment injection, and does not add source-coded command restrictions to the human-operated path.
- No enforcement or scope semantics were changed.

## 6. Failure and recovery review

Command timeout kills/reaps the bounded child; stdout/stderr are capped and collected through pipes. Proxy checks fail closed on unsupported SOCKS4 mapping or request/TLS error. No persistent state migration or new background tasks were introduced. The secret no longer requires cross-thread environment mutation.

## 7. Migration and compatibility review

No wire-schema migration. The TUI field now communicates an environment-variable name rather than a password value; embedded execution checks its local environment while daemon-attached execution relies on the daemon's environment. The CLI's documented `EGGSEC_STORAGE_PASSWORD` path remains intact. The proxy-health result remains a reachability signal and does not promise successful proxy authentication.

## 8. Security review

The change removes process-environment disclosure and the proxy-health false-positive path. The distributed trust grant remains intentionally broad: possession of the PSK authorizes OS-command execution as the worker/listener account. Network isolation and PSK custody remain operational controls; source-code allow/deny lists were intentionally not introduced.

## 9. Documentation and operations

- Remote worker trust boundary and bounds: `architecture/distributed.md`, `.opencode/skills/eggsec-distributed/SKILL.md`.
- Proxy-health behavior: `architecture/web_proxy.md`.
- Regression tests cover distributed argument bounds and proxy health matrix.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| low | The plan notes that `ProxyType::Https` is currently represented as an HTTP proxy URL in the health/effect path; whether this is intentional CONNECT-capable HTTP-proxy semantics or missing TLS-to-proxy remains a separate design question. | Naming/transport semantics may be ambiguous; this milestone does not establish a live credential leak or change the transport contract. | Keep out of this closure; document or create a separate scoped decision if TLS-to-proxy is intended. |

## 11. Roadmap disposition

Milestone closed. The low-severity proxy-type semantics question is explicitly outside this milestone; the PSK trust grant is documented and is not treated as an unbounded resource grant.

## 12. Registry updates

- Record this milestone as closed with this closure reference in the security-assurance-hardening roadmap and `plans/registry.md`. Its soft ordering after 001 is satisfied.
