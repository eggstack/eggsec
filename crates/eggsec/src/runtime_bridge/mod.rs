//! Runtime-to-enforcement bridge.
//!
//! Converts frontend-neutral [`eggsec_runtime`] DTOs into canonical
//! [`crate::config`] enforcement types. This is the security boundary
//! between the daemon/runtime layer and the engine's policy model.
//!
//! # Dependency direction
//!
//! The bridge lives in the `eggsec` crate (which may depend on `eggsec-runtime`).
//! The `eggsec-runtime` crate must **never** depend on `eggsec`. This module
//! is the only place where runtime DTOs are converted to enforcement types.
//!
//! # Invariants
//!
//! - [`RuntimeSurface::Unknown`](eggsec_runtime::RuntimeSurface::Unknown) is never executable.
//! - Manual surfaces (`CliManual`, `TuiManual`) retain operator-directed semantics
//!   whether embedded or daemon-backed.
//! - Automated surfaces (`McpServer`, `RestApi`, `GrpcApi`, `SecurityAgent`, `Ci`)
//!   never honor manual overrides.
//! - Any new [`RuntimeSurface`](eggsec_runtime::RuntimeSurface) variant must update
//!   the conversion tests in [`surface`].

mod bundle;
mod descriptor;
mod executor;
mod manual;
mod surface;

pub use bundle::{
    approve_run_request_bundle, dispatch_approved_runtime_request, ApprovedRunRequest,
};
pub use descriptor::descriptor_for_run_request;
pub use executor::EggsecRuntimeExecutor;
pub use manual::{approve_run_request, preflight_run_request};
pub use surface::{runtime_surface_to_execution_surface, RuntimeBridgeError};
