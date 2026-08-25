# Phase F Plan: Engine/Application Boundary and Library-Size Reduction

## Status


Status: Executed.
Executed (partial). Phase F workstream 4 (logging subscriber), workstream 5
(notifications/integrations), and workstream 6 (config watching) completed.
Workstream 2 (CLI definitions) partially completed: `clap` and `clap_complete`
are now optional behind a `cli` feature; `Cli` and `Commands` remain in the engine
but are feature-gated; `CommonHttpArgs` extracted to `types.rs` as a plain struct;
CLI-specific `CommonHttpArgsCli` wraps it with clap derives. Workstream 3
(command orchestration) partially completed: dispatch, fuzzer, and loadtest
modules are gated behind `cli` feature. Workstream 7-10 deferred to follow-up
phase. Acceptance criteria 1 (clap optional), 4 (indicatif conditional), 7
(config watching optional) met. Criteria 2-3 (CLI types in eggsec-cli, typed
non-Clap APIs) require larger follow-up refactor.

## Objective

Make the main `eggsec` library an engine and policy composition layer rather than
an application bundle. Move CLI parsing, operator presentation, notification
transports, file watching, and other application adapters to frontend or adapter
crates so Python and headless Rust consumers link only the capabilities they use.

The phase must preserve every supported capability while reducing unconditional
dependency reachability and clarifying ownership.

## Current boundary problem

The central engine crate currently owns or unconditionally depends on facilities
such as:

```text
Clap and shell completion
Indicatif progress presentation
tracing-subscriber and tracing-appender
SMTP/Lettre notifications
configuration file watching/debouncing
HTML scraping utilities
GeoIP database support
certificate generation
broad reqwest features including blocking/cookies/SOCKS/HTTP2
application directory expansion and shell path helpers
```

Some of these are legitimate engine capabilities in specific domains. They are
not all legitimate unconditional dependencies of the library used by Python or
headless consumers.

## Preconditions

- Phase D provides a stable typed operation catalog and service boundary.
- Phase E has addressed urgent direct dependency advisories so movement does not
  preserve known vulnerable versions under new crate names.

## Scope

Primary manifests and source areas:

```text
Cargo.toml
crates/eggsec/Cargo.toml
crates/eggsec/src/lib.rs
crates/eggsec/src/cli/
crates/eggsec/src/commands/
crates/eggsec/src/logging/
crates/eggsec/src/notify/
crates/eggsec/src/config/
crates/eggsec/src/output/
crates/eggsec-cli/Cargo.toml
crates/eggsec-cli/src/
crates/eggsec-tui/Cargo.toml
crates/eggsec-tui/src/
crates/eggsec-python/Cargo.toml
crates/eggsec-python/src/
crates/eggsec-output/
crates/eggsec-core/
crates/eggsec-tool-core/
```

New small adapter crates are allowed only when they create a real dependency
boundary used by more than one consumer. Prefer moving code into existing
`eggsec-cli`, `eggsec-tui`, `eggsec-output`, or domain crates.

## Non-goals

This phase does not:

- remove CLI or TUI commands;
- split every module into a crate;
- rewrite network engines for abstract purity;
- move domain execution out of `eggsec` when no dependency benefit exists;
- replace Clap, tracing, Lettre, notify, scraper, MaxMind, or reqwest solely for
  stylistic reasons;
- optimize the `full` aggregate as the primary artifact;
- introduce a plugin ABI;
- combine daemon topology work from Phase G unless required for compilation;
- alter manual release behavior.

## Target dependency principles

### Engine crate

The engine may depend on:

- policy, scope, operation catalog, and core DTOs;
- async runtime and networking needed for default assessment capabilities;
- domain execution dependencies used by default engine behavior;
- serialization and error primitives;
- tracing facade, but not necessarily subscriber/output configuration.

The engine should not unconditionally own:

- command-line parsing;
- terminal presentation/progress bars;
- shell completion;
- application log subscriber setup;
- SMTP/webhook/PagerDuty/Slack transport implementations unless a notification
  adapter feature is enabled;
- config filesystem watch loops unless a watch adapter is enabled;
- TUI clipboard/theme/compression dependencies;
- daemon persistence/server implementation;
- PDF generation unless the PDF feature is enabled;
- headless browser, Kubernetes, DB, proxy, or other optional-domain dependencies
  outside their features.

### Frontend crates

Frontends translate external input into typed engine operations and render
results. They do not duplicate authorization or domain execution.

## Workstream 1 — Establish artifact baselines

Before moving code, record for at least:

```bash
cargo tree -p eggsec -e features
cargo tree -p eggsec-cli -e features
cargo tree -p eggsec-python -e features
cargo build -p eggsec-cli --release
cargo build -p eggsec-cli --release --no-default-features
```

Record:

- release artifact sizes;
- direct and transitive crate counts;
- duplicate dependency generations;
- native/build dependencies;
- largest crates from `cargo bloat --crates` when available;
- which dependencies are reachable from engine-only and Python builds.

Do not block implementation if `cargo bloat` is unavailable. File size and
`cargo tree` baselines are mandatory; symbol-level analysis is optional.

## Workstream 2 — Move CLI definitions into `eggsec-cli`

Relocate Clap-specific types, subcommands, global flags, help text, and shell
completion ownership from `eggsec` to `eggsec-cli`.

The engine should expose typed input structures or service calls that are not
annotated with Clap. The CLI may define conversion layers:

```rust
impl TryFrom<ScanArgs> for ScanRequest
impl TryFrom<FuzzArgs> for FuzzRequest
```

Keep conversion validation close to the CLI while target/policy validation
remains in the engine.

Migration requirements:

- preserve command names, flags, defaults, help, and completion output;
- preserve feature-gated command availability;
- avoid duplicating domain request types solely for CLI use;
- keep public Rust library consumers able to invoke operations without parsing
  CLI structs;
- update examples/docs that import `eggsec::cli` if that path was public.

Where external compatibility requires it, provide a temporary deprecated
re-export behind an explicit `cli-compat` feature or document the breaking
change. Do not keep Clap unconditional in the engine solely for compatibility.

## Workstream 3 — Move command orchestration presentation out of the engine

Separate:

```text
operation/service execution
operator prompts and confirmations
progress bar rendering
stdout/stderr formatting
shell-oriented path expansion
command-specific report presentation
```

The engine may return progress events and structured results. CLI/TUI adapters
render them.

Do not move enforcement decisions into frontends. Manual confirmation input is
translated into `ManualOverride`; policy evaluation remains centralized.

If `commands/handlers` currently combines execution and presentation, migrate
one command family at a time. Avoid a flag-day rewrite.

## Workstream 4 — Isolate logging subscriber setup

Keep `tracing` instrumentation in engine code. Move subscriber selection,
pretty/JSON formatting, rolling file appenders, and application log-directory
setup to the CLI, daemon, or agent host that owns process startup.

Target result:

- `eggsec` depends on `tracing` facade;
- `eggsec-cli` owns CLI subscriber configuration;
- `eggsec-daemon` owns daemon subscriber configuration;
- Python either exposes no subscriber or provides a binding-specific bridge;
- `tracing-appender` is linked only by artifacts that write rolling log files.

## Workstream 5 — Isolate notifications and external integrations

Inventory notification channels and determine ownership:

```text
SMTP email
webhook
Slack
PagerDuty
Jira/GitHub/GitLab integrations
```

Options, in preferred order:

1. move transport implementations to an existing integration adapter module
   behind the relevant feature;
2. create one `eggsec-notify` adapter crate only if multiple frontends consume it
   independently;
3. keep protocol-neutral notification event types in a dependency-light crate.

`lettre` must not remain unconditional in the engine if email notification is
not part of every library use.

Preserve configuration serialization and command behavior. Do not add new
channels.

## Workstream 6 — Isolate configuration watching

Separate config parsing/loading from filesystem watch/debounce behavior.

- parsing and explicit file loading may remain in engine/config support;
- `notify` and `notify-debouncer-mini` belong behind a `config-watch` adapter
  feature or in process-host crates;
- Python and short-lived headless commands should not link a watcher by default;
- daemon/TUI hot reload can explicitly enable the adapter.

Preserve current config file syntax and reload semantics for artifacts that use
watching.

## Workstream 7 — Move output-only dependencies

Review `eggsec-output` and engine output facades. Ensure:

- PDF generation remains behind `pdf`;
- XML/HTML/SARIF/JUnit formatting lives in output adapters, not scanner logic;
- terminal styling/progress does not leak into report DTOs;
- hostname/UUID/time dependencies are retained only where report envelopes
  require them;
- async Tokio is removed from output-only code if not required by actual APIs.

Do not force every report adapter into the minimal engine build.

## Workstream 8 — Narrow engine HTTP client ownership

Create explicit client profiles or constructors instead of one globally broad
reqwest feature set. Examples:

```text
core async HTTP: rustls + json + redirects
proxy client: SOCKS
session/auth adapter: cookies + form/query
legacy compatibility: blocking, only if still required
```

Prefer async paths and remove blocking reqwest from the engine when no supported
public API requires it. Keep one shared policy-aware client builder where TLS,
redirect, proxy, timeout, and scope behavior must remain consistent.

Feature declarations should activate optional reqwest capabilities only for the
owning adapter/domain.

## Workstream 9 — Reduce Python extension reachability

After movement, verify `eggsec-python` does not pull frontend-only dependencies.

Review direct binding dependencies for duplication with the engine. Remove
direct dependencies that are used only through engine types or can be replaced
with small standard-library/PyO3 conversions.

Preserve:

- stable operation APIs;
- sync/async behavior;
- result and error serialization;
- feature introspection;
- maturin packaging.

Do not change Python maturity classification in this phase.

## Workstream 10 — Reassess small crate boundaries

After dependency movement, evaluate—but do not automatically perform—small
consolidations:

- `eggsec-ui-model` into `eggsec-runtime` if it has no independent consumer and
  does not create dependency inversion;
- overlap between `eggsec-core` and `eggsec-tool-core`;
- output compatibility facades that only re-export another crate.

Consolidate only when it reduces maintenance without reintroducing heavy
dependencies into leaf crates. Document any deliberate retention.

## Tests

Required behavior checks:

- CLI parsing snapshots/help for representative commands;
- command-to-engine request conversion;
- manual override propagation;
- structured progress/result behavior;
- notification adapter configuration and one fake transport test;
- config watch behavior with temporary files when retained;
- Python installed-package smoke and stable API tests;
- no-default engine and headless CLI compilation;
- feature-gated optional adapters compile.

Add manifest-boundary tests only when Cargo itself cannot express the intended
optional dependency relation. Avoid new grep guards for module placement.

## Validation commands

```bash
cargo fmt --all -- --check
cargo check -p eggsec --no-default-features
cargo test -p eggsec --lib
cargo check -p eggsec-cli --no-default-features
cargo check -p eggsec-cli
cargo check -p eggsec-tui
cargo check -p eggsec-python
cargo test -p eggsec-output
make check-python
```

Feature-specific checks should correspond to moved adapters:

```bash
cargo check -p eggsec-cli --features rest-api
cargo check -p eggsec --features external-integrations
cargo check -p eggsec --features pdf
```

Do not add a broad all-features gate solely for this phase.

## Migration sequence

1. Add engine request/service APIs where commands still depend on Clap structs.
2. Move CLI definitions and conversion one command family at a time.
3. Move process logging setup.
4. Isolate notifications/integrations.
5. Isolate config watching.
6. Narrow output dependencies.
7. Narrow HTTP client feature ownership.
8. Remove old engine re-exports/dependencies.
9. measure Python/headless/default artifact changes.
10. consider small crate consolidation only after the primary boundary is clean.

## Rollback considerations

If a moved adapter causes compatibility problems, restore a thin compatibility
crate or feature-gated re-export. Do not restore unconditional application
dependencies to the engine.

## Acceptance criteria

1. `eggsec` no longer unconditionally depends on Clap or `clap_complete`.
2. CLI argument types and help are owned by `eggsec-cli`.
3. Engine operations are invokable through typed non-Clap APIs.
4. `indicatif` is linked only by artifacts that render progress bars.
5. tracing subscriber/appender setup is owned by process-host crates.
6. SMTP/Lettre is optional and owned by a notification/integration adapter.
7. config watching dependencies are optional and not linked by Python/headless
   consumers by default.
8. PDF, browser, DB, proxy, and other optional domains remain feature-scoped.
9. broad reqwest features are split by owning capability; blocking is removed
   where unused.
10. `eggsec-python` no longer inherits CLI/operator-only dependencies.
11. standard CLI/TUI behavior and public Python behavior remain intact.
12. no-default engine and headless CLI builds pass.
13. before/after dependency and artifact measurements show the effect of the
    boundary changes.
14. new crates are introduced only for demonstrated reusable dependency
    boundaries.
15. no package or release is published.
