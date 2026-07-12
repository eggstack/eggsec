/// Experimental module marker.
///
/// This module exists as a namespace marker for the `eggsec.experimental` subpackage.
/// The actual experimental APIs are defined in Python-side code under
/// `python/eggsec/experimental/`. This Rust marker ensures the `_core`
/// module documents that experimental APIs are supported.
///
/// See `docs/python/namespace.md` for the experimental namespace policy.
pub const EXPERIMENTAL_MARKER: bool = true;
