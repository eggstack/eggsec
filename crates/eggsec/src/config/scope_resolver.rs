//! Host resolution facade.
//!
//! Canonical owner is the engine policy bridge
//! (`crate::policy_bridge::resolver`, Phase C): concrete DNS acquisition must
//! not live in the policy kernel. Re-exported here so
//! `crate::config::{HostResolver, SystemResolver, ResolutionResult,
//! default_resolver}` and `crate::config::scope_resolver::{...}` paths keep
//! working.

// Compatibility shim: this deep path is intentionally preserved for
// downstream users even when no in-tree code references it directly.
#[allow(unused_imports)]
pub use crate::policy_bridge::resolver::{
    default_resolver, resolve_hostname_facts, resolve_hostname_facts_with, resolve_target_facts,
    resolve_target_facts_with, HostResolver, ResolutionResult, ScopeResolution, SystemResolver,
};
