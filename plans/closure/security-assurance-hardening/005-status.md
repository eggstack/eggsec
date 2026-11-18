# Security Assurance Hardening Milestone 005 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/security-assurance-hardening/005-daemon-http-adapter-safety.md`

Source subsystem roadmap:

- `plans/subsystems/security-assurance-hardening-roadmap.md` §7 Milestone 5

Repository baseline reviewed: `77188755a41bee15aee9507055ac6b2761ff9271` (implementation remains uncommitted in the worktree)

Implementation commits or pull requests:

- None. No commit or PR was created, per instruction; evidence below describes the current working tree.

## 1. Executive finding

The dormant daemon HTTP adapter now has explicit authentication and bind modes, Host/Origin validation, and fail-closed anonymous ownership behavior. It remains unwired: no CLI/listener path was added, and future wiring still requires an ADR.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| No public bind without credential | `crates/eggsec-daemon/src/http.rs`; configuration matrix tests | pass | `AuthMode` is explicit; `HttpConfig` has no `Default`. |
| Host and Origin are validated | HTTP middleware tests in daemon feature target | pass | Mismatches are rejected before route work. |
| Anonymous callers cannot access owned state | HTTP route/ownership tests | pass | `/health` and `/capabilities` are the only routes exempt from token auth; neither accesses session-owned state. Session/action routes remain authenticated and ownership-checked. |
| Adapter remains dormant | Workspace search finds only the `run_http_server` definition; no caller; daemon module still feature-gated | pass | No CLI flag or process wiring added. |
| Default dependency and strict-policy boundaries remain intact | `make check` and `make check-full` | pass | No daemon default-dependency expansion; policy remains engine-owned. |

## 3. Production implementation evidence

The configuration uses `AuthMode::Token(AuthToken)` or loopback-only trust and is built through a validating constructor rather than `Default`. Bind validation couples non-loopback exposure to explicit opt-in and a credential. Host/Origin middleware rejects invalid requests; anonymous session ownership is denied. The adapter compiles only under its optional `http-api` feature and is not called by the daemon executable or CLI.

## 4. Verification executed

### Commands run

```bash
cargo test -q -p eggsec-daemon --features http-api
make check
make check-full
```

### Results

The HTTP-feature daemon suite passed 105 tests. The mandatory check, including workspace no-default compilation, and the optional deep check both exited 0. No live listener or runtime-network test was performed because the adapter is intentionally unwired.

## 5. Invariant review

- `EnforcementContext::evaluate()` remains the pre-dispatch gate; the adapter adds transport admission, not policy semantics.
- No scope-insensitive or anonymous route receives session ownership authority.
- The Unix-socket daemon path was not changed.
- No HTTP API dependency was added to the daemon's default dependency set.

## 6. Failure and recovery review

Invalid configuration fails before listener startup. Invalid Host/Origin, missing token, and anonymous owner access fail closed. Since the adapter remains dormant, restart, persistence, and in-flight cancellation semantics of the supported daemon path are unchanged.

## 7. Migration and compatibility review

The HTTP configuration type is an internal, dormant API with no callers; there is no deployed configuration migration. No Unix-socket protocol or client identity contract changed.

## 8. Security review

Non-loopback unauthenticated operation is not constructible through the validated config path. Tokens are redacted and compared with the intended constant-time primitive. Host/Origin checks defend the latent browser-reachable path. In token mode, `/health` and `/capabilities` are explicitly exempt from bearer-token checks and return non-session health/capability metadata; session/action routes remain authenticated. The adapter does not claim that a client UUID is itself an authenticated identity.

## 9. Documentation and operations

- Dormant adapter and future wiring gate: `architecture/daemon.md`.
- Feature gating and test command are documented; no operator-facing command was added.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| — | No unresolved finding within the unwired-adapter safety boundary. | — | — |

## 11. Roadmap disposition

Milestone closed as a latent-surface hardening change. Any future wiring requires a separate ADR; the current adapter remains unsupported and unwired.

## 12. Registry updates

- Record this milestone as closed with this closure reference in the security-assurance-hardening roadmap and `plans/registry.md`.
