//! Scope facade.
//!
//! Canonical owner of the scope data model and pure matching is
//! `eggsec-policy` (Phase C). Re-exported here so `crate::config::scope::*`
//! and `crate::config::{Scope, TargetScope, ...}` paths keep working.
//!
//! DNS acquisition and filesystem loading live in the engine policy bridge
//! (`crate::policy_bridge::resolver`); import
//! [`ScopeResolution`](crate::policy_bridge::resolver::ScopeResolution) for
//! the historical `is_target_allowed*` method syntax. The transport checkpoint
//! adapter is [`ScopeAuthority`](crate::policy_bridge::transport::ScopeAuthority).

pub use eggsec_policy::scope::{
    LoadedScope, Scope, ScopeError, ScopeRule, ScopeSource, TargetScope,
};
pub use eggsec_policy::{classify_address, is_private_ip, AddressClass};

// Historical resolver paths now owned by the bridge; re-exported for compat.
pub use crate::policy_bridge::resolver::{
    default_resolver, resolve_hostname_facts, resolve_hostname_facts_with, resolve_target_facts,
    resolve_target_facts_with, HostResolver, ResolutionResult, ScopeResolution, SystemResolver,
};

/// Convert a [`LoadedScope`] into its runtime session view.
///
/// Engine-side free function (replacing the pre-extraction
/// `From<&LoadedScope> for SessionScope` impl, which cannot live here after
/// `LoadedScope` moved to `eggsec-policy` under orphan rules).
pub fn session_scope_from_loaded(loaded: &LoadedScope) -> eggsec_runtime::SessionScope {
    let source = match loaded.source {
        ScopeSource::DefaultEmpty => "default-empty",
        ScopeSource::ConfigFile => "config",
        ScopeSource::CliScopeFile => "cli",
        ScopeSource::GeneratedPreset => "preset",
    };
    eggsec_runtime::SessionScope {
        is_explicit: loaded.is_explicit_manifest(),
        source: source.to_string(),
        path: loaded.path.clone(),
    }
}
