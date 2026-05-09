//! Shared test utilities for integration tests.
//!
//! Provides wiremock helpers, test server setup, serialization helpers,
//! and assertion macros used across multiple test files.

pub mod wiremock_helpers;

pub use wiremock_helpers::*;

use serde::de::DeserializeOwned;
use serde::Serialize;

/// Asserts that a value can be serialized to JSON and deserialized back
/// to an equivalent value.
///
/// # Panics
///
/// Panics if serialization or deserialization fails, or if the
/// deserialized value is not equal to the original.
///
/// # Example
///
/// ```rust
/// use slapper::types::Severity;
///
/// assert_serialize_roundtrip(&Severity::High);
/// ```
pub fn assert_serialize_roundtrip<T>(value: &T)
where
    T: Serialize + DeserializeOwned + Eq + std::fmt::Debug,
{
    let json = serde_json::to_string(value).unwrap();
    let decoded: T = serde_json::from_str(&json).unwrap();
    assert_eq!(value, &decoded);
}

/// Asserts that a value can be serialized to JSON and deserialized back
/// with the same string representation (for String types).
pub fn assert_string_serialize_roundtrip(value: &str) {
    let json = serde_json::to_string(value).unwrap();
    let decoded: &str = serde_json::from_str(&json).unwrap();
    assert_eq!(value, decoded);
}
