//! Engine-owned policy bridge (Phase C).
//!
//! Composition layer between two independent contracts:
//!
//! - the pure `eggsec-policy` authorization kernel (no I/O);
//! - the `eggsec-transport` scoped HTTP contract (neutral DTOs).
//!
//! Dependency direction is strictly one-way:
//!
//! ```text
//! eggsec-policy      eggsec-transport
//!        ^                 ^
//!         \               /
//!          \             /
//!               eggsec
//!         composition/bridge
//! ```
//!
//! Never introduce `eggsec-policy -> eggsec-transport` or the reverse merely
//! to avoid a small adapter. Concrete DNS acquisition, feature-registry
//! mapping, and `ScopeAuthority` all live here, never in the policy kernel.

pub mod features;
pub mod resolver;
pub mod transport;

pub use features::current_enabled_features;
pub use resolver::{resolve_target_facts, resolve_target_facts_with, ScopeResolution};
pub use transport::ScopeAuthority;
