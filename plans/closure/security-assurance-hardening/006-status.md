# Security Assurance Hardening Milestone 006 — Closure Status

Status: conditionally closed

Source implementation plan:

- `plans/implementation/security-assurance-hardening/006-supply-chain-and-output-hygiene.md`

Source subsystem roadmap:

- `plans/subsystems/security-assurance-hardening-roadmap.md` §7 Milestone 6

Repository baseline reviewed: `77188755a41bee15aee9507055ac6b2761ff9271` (implementation remains uncommitted in the worktree)

Implementation commits or pull requests:

- None. No commit or PR was created, per instruction; evidence below describes the current working tree.

## 1. Executive finding

The static supply-chain and output-hygiene changes are implemented: Dockerfile and Compose definitions were corrected, required lab credentials are parameterized, root artifacts from another project were removed, report JavaScript is vendored, and dependency exceptions were reviewed. The image build/run and Compose set/unset behavior could not be executed because neither Docker nor Docker Compose is installed. M6 is therefore **conditionally**, not unconditionally, closed.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| Docker build uses the workspace MSRV and complete crate set | `Dockerfile`: `rust:1.89-bookworm`, `COPY . .`, curated `eggsec-cli --features full` build | partial | Static source inspection only; no image build or `eggsec --version` run. |
| Compose credentials fail closed and host ports stay local | `docker-compose.yml`, `DOCKER.md`, `DOCKER_COMPOSE.md`; required-variable substitutions and every published host port binds to `127.0.0.1` | partial | Static inspection only; `docker compose config` set/unset cases were unavailable. |
| Local Compose env files stay out of Docker build context | Root `.dockerignore` excludes `.env`, `.env.*`, and Git metadata before the Dockerfile's `COPY . .` | partial | Pattern content is statically inspected; Docker was unavailable to test actual context handling. |
| Root artifacts are dispositioned | `config.toml`, `package.json`, and `package-lock.json` are deleted; `.github`, `scripts`, and Makefile reference checks found no build/CI dependency; `architecture/supply_chain.md` records the rationale | pass | No npm fixture was retained because tests use temporary manifests. |
| Report runs offline without remote script code | Vendored Chart.js 4.5.1 asset (208,522 bytes) with adjacent MIT license; HTML tests and `cargo test -q -p eggsec-output` | pass | Output crate tests: 77 + 12 passed. |
| Dependency exception state is current and policy-enforced | `docs/DEPENDENCY_EXCEPTIONS.md`, `deny.toml`, `Cargo.lock`; `make check-deps` | pass | `lettre` 0.11.23→0.11.24 resolves RUSTSEC-2026-0560 through -0565; retained exceptions have owners/dates. |
| `db-pentest` and MSSQL feature boundary is accurately represented | `cargo tree -p eggsec-cli --features full -i rustls@0.21.12` (no package match; expected) and explicit MSSQL feature graph | pass | `db-pentest-mssql-tiberius` remains opt-in; this result is not Docker runtime evidence. |

## 3. Production implementation evidence

- `Dockerfile` targets Rust 1.89, copies the complete workspace instead of maintaining a partial manifest list, retains the curated `full` feature aggregate, and keeps the non-root runtime user.
- Compose substitutions require lab-specific values for Redis, Elasticsearch, Tor, Postgres, MySQL, and MSSQL passwords. Published services and lab ports are loopback-bound. Elasticsearch's security-off setting is explicitly described as lab-only.
- The root `.dockerignore` excludes root/nested `.env` and `.env.*` files and Git metadata from Docker build contexts; Compose still reads the project `.env` for interpolation. This is statically reviewed only because Docker is unavailable.
- The accidental root `config.toml`, empty `package.json`, and empty `package-lock.json` are deleted and were not restored. The supply-chain architecture note now says npm manifests belong in test fixtures, not the repository root.
- HTML reports embed the pinned Chart.js build and guard chart construction; summary fields remain available without remote script retrieval.
- The dependency record distinguishes the curated `full` feature graph from the all-features dependency-policy graph. It records the `lettre` update and the current 2027-02-15 review-by date for the retained `chrono` exception. The 7 Dependabot alerts are described as a dated 2026-11-17 baseline snapshot, not as a freshly queried live count.

## 4. Verification executed

### Commands run

```bash
cargo test -q -p eggsec-output
make check-deps
make check
make check-full
cargo tree -p eggsec-cli --features full -i rustls@0.21.12
cargo tree -p eggsec --features db-pentest-mssql-tiberius -i rustls@0.21.12
command -v docker
command -v docker-compose
```

### Results

- Output crate: 77 unit tests and 12 integration tests passed.
- `make check-deps`, `make check`, and `make check-full` exited 0. Cargo Deny emitted non-fatal duplicate-version/license-metadata warnings; dependency bans, licenses, and sources checks passed.
- The `full` feature graph command returned no `rustls 0.21.12` package (expected); the separate MSSQL marker graph resolves `tiberius 0.12.3` → `tokio-rustls 0.24.1` → `rustls 0.21.12`.
- Docker and docker-compose were unavailable. No container build, image runtime, or Compose configuration command was run; no such success is claimed.

## 5. Invariant review

- `deny.toml` remains canonical; all-features policy checking is fail-closed and passes.
- Ring-only TLS and the `aws-lc-rs` ban were not weakened.
- `db-pentest` stays in the curated full aggregate; the optional Tiberius path stays separately gated.
- Report output retains escaping and CSV formula neutralization; no remote chart script remains.
- The test/lab Compose profile is not presented as production configuration, and no lab service is bound to all host interfaces.

## 6. Failure and recovery review

Docker/Compose validation was blocked by unavailable executables, not by a failed build or configuration test. No image or container was started, so there is no runtime state to recover. Compose's required substitutions are visible in source, but whether the current CLI accepts the full configuration remains unverified. No concurrency, cancellation, persistence, or rollback behavior changed.

## 7. Migration and compatibility review

- Compose users must provide all documented lab credentials; unset or empty values are intended to fail configuration.
- Removing root npm manifests intentionally removes the false repository-level npm surface; npm parser tests use temporary manifests.
- The Docker image's full feature set remains the existing curated aggregate. Actual image size and binary runtime behavior are unverified until Docker is available.
- Chart.js is now embedded (208,522 bytes) and the generated HTML remains self-contained/offline.

## 8. Security review

Static review shows no hardcoded service passwords in Compose, required-value substitutions, loopback host binds, a non-root Docker runtime user, a vendored report script, and documented residual dependency exceptions. These findings do not substitute for Compose parser behavior or image execution, which remain outstanding.

## 9. Documentation and operations

- Container use and required values: `DOCKER.md`, `DOCKER_COMPOSE.md`.
- Dependency policy and dated exception mapping: `docs/DEPENDENCY_EXCEPTIONS.md`, `deny.toml`.
- Root manifest disposition: `architecture/supply_chain.md`.
- Report asset and offline behavior: `crates/eggsec-output/assets/chart.umd.min.js`, `CHARTJS-LICENSE.md`.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| medium | Docker build/runtime and Compose set/unset validation were not executable in this environment. | The Dockerfile and Compose behavior are statically reviewed but not proven by a real parser, build, or runtime. | On a Docker-capable environment, run `docker build -t eggsec:audit-006 .`; run `docker run --rm eggsec:audit-006 --version`; run `docker compose config` with all required values set, then omit each required variable in turn and confirm a clear failure. Update this record only with actual output. |

## 11. Roadmap disposition

Milestone conditionally closed with named operational evidence outstanding. Static implementation and dependency-policy checks pass; the required container and Compose runtime evidence must be added before unconditional closure.

## 12. Registry updates

- Mark M6 conditionally closed and reference this record in the security-assurance-hardening roadmap and `plans/registry.md`.
- Keep the subsystem roadmap conditionally closed until the evidence listed in §10 is captured.
