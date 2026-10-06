//! Payload corpora — engine-side seam.
//!
//! The 40 payload modules live in the leaf crate `eggsec-payloads`, which owns
//! every [`PayloadType`] variant and resolves all of them through its
//! `get_payloads`. The cross-variant caches live there too, for the same
//! reason: they span all 40 variants, and the corpus now owns all 40.
//!
//! What stays here are the six **probers** — [`graphql`], [`grpc`], [`idor`],
//! [`jwt`], [`oauth`], [`ssti`] — modules holding a `reqwest::Client` that make
//! live requests. They cannot move into a crate with no network surface.
//!
//! The seam is **execution, not payload data**. Each prober module re-exports
//! the static payload builder of the same name from `eggsec-payloads`, so
//! `GraphQLFuzzer` (here) and the GraphQL payload strings (there) are separate
//! things that happen to share a name.
//!
//! Everything is re-exported below, so `eggsec::fuzzer::payloads::*` and
//! `eggsec::fuzzer::{PayloadType, Payload, get_payloads, …}` keep resolving
//! exactly as before. The re-exports are a **permanent** facade, not
//! transitional scaffolding.

// The six live-probe modules: the probers, not the payloads. Each also
// re-exports its corpus module's `get_payloads` (and, for ssti, `TemplateEngine`).
pub mod graphql;
pub mod grpc;
pub mod idor;
pub mod jwt;
pub mod oauth;
pub mod ssti;

// Everything from the corpus crate: the 40 payload modules, `macros`, the
// `Payload` / `PayloadType` / `Severity` types, `PayloadType::all_variants`,
// `is_advanced`, and the cross-variant `get_payloads*` caches.
//
// The six modules of the same name are shadowed by the prober modules declared
// above; a glob import loses to an explicit item, so `payloads::graphql` is the
// prober, not the payload strings. Consumers that want the payload strings ask
// for them through `get_payloads`, which is why nothing needs that path.
pub use eggsec_payloads::*;
