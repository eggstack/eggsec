//! Slapper Core - Dependency-light domain types and primitives
//!
//! This crate contains the shared domain types and constants used across
//! the Slapper workspace. It is designed to have a small dependency set
//! and no heavy optional dependencies (no tokio, reqwest, ratatui, etc.).
//!
//! The canonical error type (`SlapperError`) and CLI-specific types
//! (`OutputFormat`) remain in the main `slapper` crate.

pub mod constants;
pub mod types;

pub use types::Severity;
