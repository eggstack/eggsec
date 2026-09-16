//! Address classification facade.
//!
//! Canonical owner is `eggsec-policy` (Phase C). Re-exported here so
//! `crate::config::scope_address::{...}` paths keep working.

// Compatibility shim: this deep path is intentionally preserved for
// downstream users even when no in-tree code references it directly.
#[allow(unused_imports)]
pub use eggsec_policy::address::{classify_address, is_private_ip, AddressClass};
