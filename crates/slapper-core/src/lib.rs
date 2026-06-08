//! Slapper Core - dependency-light domain types and shared primitives.
//!
//! This crate contains stable shared types and constants used across the
//! Slapper workspace. It intentionally avoids runtime, UI, network, API,
//! database, packet, browser, and agent dependencies.
//!
//! Keep this crate small. Subsystem-specific behavior belongs in subsystem
//! crates or the main `slapper` engine crate.

pub mod constants;
pub mod types;

pub use types::Severity;
