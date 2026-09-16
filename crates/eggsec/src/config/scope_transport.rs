//! Transport authority facade.
//!
//! Canonical owner is the engine policy bridge
//! (`crate::policy_bridge::transport`, Phase C): the `NetworkAuthority`
//! adapter sits between the pure policy kernel and `eggsec-transport`, owned
//! by neither. Re-exported here so `crate::config::scope_transport::*` and
//! `crate::config::ScopeAuthority` paths keep working.

pub use crate::policy_bridge::transport::ScopeAuthority;
