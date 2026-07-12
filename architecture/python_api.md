# Python API execution contract

`eggsec-python` is a host-language binding over the Rust engine. Its scoped
pre-1.0 stable core is intentionally smaller than the importable package.

## Stable dispatch

The ten stable engine operations are represented by the Rust
`StableOperation` enum in `crates/eggsec-python/src/operation_registry.rs`.
`Engine.run()` and `AsyncEngine.run()` parse the same enum before dispatch, so
unknown IDs cannot silently reach a domain executor. Historical aliases are
accepted only as compatibility inputs and resolve to the same enum variant.

## Mandatory gate

Every stable engine operation evaluates an `eggsec::config::EnforcementContext`
before entering a domain executor. The gate evaluates scope, risk, required
features, capabilities, and execution profile. It records an allow, warning,
confirmation, or deny decision in the engine’s structured audit sink. Tracing
is supplemental and is not the audit contract.

Successful results include `policy_decision=allow` and the policy schema
version in metadata. Failed results carry `OperationError`, which includes a
schema version, kind, code, operation ID, retryability, denial class, source,
details, and causes. `error_message` is a compatibility view.

## Event delivery

`EventEnvelope.sequence` is monotonic for events produced by one stream.
`BackpressureChannel` has a bounded best-effort queue for progress and a
reliable queue for planning, preflight, stage lifecycle, finding, artifact,
cancellation, failure, and completion events. `EventDeliveryStats` reports
emission, delivery, drops by event kind, maximum depth, consumer lag, and
terminal delivery failures. The guarantee is exactly-once within the in-process
queue model; daemon replay requires the daemon contract to be completed.

## Domain boundary

Use Python `domain_maturity()` for whole-domain state and `api_surface()` for
individual symbol state. A Cargo feature only controls compilation. It does
not promote a domain to stable-core. See
[`docs/python/domain-maturity.md`](../docs/python/domain-maturity.md).
