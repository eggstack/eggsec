# Python API Milestone C — Core Missing Assessment Domains

## Goal

Expose the highest-value ordinary assessment domains still absent from Python: consolidated reconnaissance, GraphQL, OAuth/OIDC, authentication testing, headless-browser assessment, and advanced hunting. All implementations must use the Milestone A engine model and Milestone B policy model.

## Dependencies

- Milestone A request/result/event/cancellation/pipeline infrastructure.
- Milestone B configuration, execution context, authorization, preflight, audit, and credential-provider primitives.
- Stable Rust domain implementations and shared finding/evidence types.

## Workstream C1 — Consolidated reconnaissance

Add `ReconRequest`, `ReconConfig`, and `ReconResult` with selectable modules for DNS, WHOIS, TLS/certificates, SAN extraction, technology detection, service/version mapping, CVE correlation, and passive host discovery.

The API should allow callers to select passive-only versus active modules, configure provider/timeouts, and receive one consolidated result with domain-specific subresults and normalized findings. Avoid duplicating already-bound DNS/TLS/technology implementations; route them through the new request model.

## Workstream C2 — GraphQL security assessment

Add `GraphQLRequest`, `GraphQLConfig`, `GraphQLResult`, `GraphQLSchemaInfo`, `GraphQLOperation`, and `GraphQLFinding`.

Bind existing Rust checks for introspection exposure, schema discovery, depth/complexity handling, aliasing/batching, authorization boundaries, error disclosure, mutation exposure, rate limiting, and input-handling issues. Support headers and credential providers without serializing secrets.

Planning/preflight must identify active tests, request counts, authentication requirements, target expansion, and risk classification.

## Workstream C3 — OAuth/OIDC assessment

Add OAuth/OIDC request and result types, issuer metadata, endpoint findings, redirect-URI findings, PKCE findings, token-validation observations, and discovery-document analysis.

Support explicit flow selection and distinguish passive metadata analysis from active authorization flows. Active flows must require appropriate scope and authorization. Validate issuer/audience assumptions, state/nonce behavior, grant exposure, redirect handling, TLS posture, and client configuration where supported by Rust.

## Workstream C4 — Authentication assessment

Add `AuthAssessmentRequest`, `AuthFlow`, `AuthConfig`, and structured findings for sessions, cookies, JWTs, rate limiting, lockout, MFA, password policy, fixation/rotation, logout, and invalidation.

All credential-bearing operations must consume `CredentialProvider`/`SecretReference`. Ensure logs, events, checkpoints, findings, and artifacts remain redacted.

## Workstream C5 — Headless browser assessment

Feature-gated bindings should expose `BrowserSession`, `BrowserAssessmentRequest`, `BrowserConfig`, `BrowserAssessmentResult`, `BrowserArtifact`, `DomFinding`, and `RouteDiscoveryResult`.

Support SPA route discovery, DOM injection observations, JavaScript-generated endpoint discovery, console/network events, screenshots, cookies/storage inspection, redirect tracking, and bounded browser lifecycle. Hide underlying browser-crate implementation details.

Ensure cancellation terminates tabs, processes, and temporary profiles deterministically.

## Workstream C6 — Advanced hunting

Add `HuntRequest`, `HuntConfig`, `HuntResult`, `HuntTechnique`, and `HuntEvidence` under the feature gate. Bind stable Rust capabilities only; do not expose internal experimental types directly.

Integrate hunting into common planning, progress, cancellation, findings, artifacts, and policy semantics.

## Workstream C7 — Namespace and API organization

Organize new domains under stable Python namespaces while retaining ergonomic top-level convenience functions where justified. Suggested layout:

- `eggsec.recon`
- `eggsec.graphql`
- `eggsec.oauth`
- `eggsec.auth`
- `eggsec.browser`
- `eggsec.hunt`

Keep class names explicit enough to avoid collision with generic Python or third-party concepts.

## Workstream C8 — Documentation and fixtures

Create controlled local fixtures for each domain. Documentation should include a minimal sync example, minimal async example, preflight example, pipeline composition example, and credential-safe example where applicable.

## Testing

- Domain request validation and serialization.
- Sync/async parity.
- Planning and preflight coverage.
- Cancellation during active work.
- Feature-unavailable behavior.
- Secret redaction for OAuth/auth/browser sessions.
- Browser cleanup under exceptions and task cancellation.
- Finding schema and artifact contract tests.
- Local fixture integration tests with no external network dependency.

## Acceptance criteria

- Each domain executes through `Engine`/`AsyncEngine` and supports planning, preflight, events, cancellation, and structured results.
- No domain introduces a separate policy or credential model.
- Consolidated recon supersedes fragmented orchestration without breaking existing functions.
- Browser resources are reliably cleaned up.
- Domain results use common finding/evidence/artifact/status types.
- Runtime exports and stubs remain synchronized for all feature combinations.

## Risks

- Authentication flows can become stateful and brittle: model flows explicitly and maintain deterministic fixtures.
- Browser dependencies increase wheel complexity: isolate feature-gated builds and CI jobs.
- CVE mapping freshness may depend on data sources: expose provenance and avoid implying guaranteed vulnerability presence.
- GraphQL/OAuth tests may be intrusive: classify and preflight each test family separately.

## Handoff notes

Implement consolidated recon first because it reuses existing bindings and validates the new engine pattern. Follow with GraphQL and OAuth/OIDC, then authentication, browser, and hunting. Each domain should land with stubs, docs, fixtures, and contract tests in the same change set.