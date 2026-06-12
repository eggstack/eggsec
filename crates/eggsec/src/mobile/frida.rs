//! Frida-based runtime instrumentation for mobile dynamic analysis (Phase 3).
//!
//! **Phase 3 vision** (per plans/mobile-dynamic-phase2-closeout-and-phase3-kickoff-plan.md
//! and architecture/mobile.md):
//! Introduce gated Frida integration for deep runtime observation and manipulation on
//! Android (iOS deferred). This enables high-value capabilities beyond logcat/ADB/proxy:
//! - Method hooking + argument/return tracing (e.g. crypto, keystore, auth flows)
//! - Runtime secret/key material extraction
//! - API call inspection and parameter tampering in lab
//! - Validation of client-side protections / root-detection / Frida-detection bypasses
//!
//! **Current state**: This is the *first-pass kickoff scaffolding* (design + core primitives).
//! - No real Frida client implementation.
//! - No external crate dependency added (see Cargo.toml; `mobile-frida` is a pure feature gate).
//! - All public functions are explicit stubs returning "not yet implemented" errors.
//! - Provides documented intended surface + one high-value built-in capability *sketch*
//!   (method tracing plan only; no codegen or execution yet).
//! - Compiles cleanly under `--features mobile-frida` (implies mobile-dynamic) and has
//!   zero impact on builds/tests using only `mobile-dynamic` (or lower).
//!
//! **Safety model (standalone defense-lab, consistent with mobile-dynamic + wireless)**:
//! - All Frida functionality is strictly lab-only / defense-validation.
//! - Requires rooted device **or** Frida-injected emulator (user supplies frida-server;
//!   e.g. on test AVD or custom image).
//! - Real (non-dry) execution will require explicit `--allow-frida` (or equivalent) flag
//!   plus EnforcementContext policy confirmation (via `OperationRisk::Intrusive` or
//!   dedicated tier; handler wiring + flag deferred to later Phase 3a steps per plan).
//! - No production data, no production devices. Test builds must be provenance-controlled
//!   and securely destroyed after use.
//! - Audit trail will be recorded (future extension of actions_performed + dedicated
//!   frida_instrumentation section).
//! - Best-effort cleanup for any injected state.
//! - **No MCP / autonomous agent exposure** (intentionally absent, same as rest of mobile).
//! - Android-first; iOS support is explicitly out of scope for the initial passes.
//!
//! **Intended public surface** (stubs document the contract; real impl will fill bodies):
//! - `FridaSession`: opaque handle representing a connection to frida-server on a device.
//! - `connect(device: &str) -> Result<FridaSession>`
//! - `execute_script(&self, script: &str) -> Result<FridaScriptResult>` (user scripts or built-ins)
//! - `basic_method_trace(...)` : sketch of the first high-value built-in capability.
//!
//! **High-value built-in capability sketch (documented intent only)**:
//! `basic_method_trace` is planned as a safe, high-signal primitive that would:
//!   - Target common sensitive Java/Android methods (e.g. Cipher.doFinal, SecretKeyFactory,
//!     KeyStore.getEntry, custom app login / token handling, root-detection checks).
//!   - Use Frida's Java.perform + Interceptor to log calls, args (redacted where possible),
//!     return values, and exceptions.
//!   - Emit structured output convertible to `DynamicMobileFinding` with categories such as
//!     "frida-method-trace", "frida-secret-extract", "frida-bypass-validation".
//!   - Feed the (future) `frida_instrumentation` field on `DynamicMobileReport` and the
//!     reporting bridge (new categories under mobile-dynamic-android-frida-*).
//!   - Remain under the same explicit-allow + policy gate as the rest of dynamic mobile.
//!
//! Future steps (post this scaffolding): CLI flag wiring (e.g. --frida-script, --allow-frida),
//! EnforcementContext integration in handler, actual frida crate (or CLI) integration,
//! built-in script library, evidence formatting, and extension of `to_scan_report_data_dynamic`.
//!
//! See: dynamic.rs (DynamicMobileReport + run_dynamic_cli + bridge), mobile/mod.rs,
//! crates/eggsec/src/mobile/AGENTS.override.md, architecture/mobile.md, docs/MOBILE.md,
//! and Section 3/4 of the combined closeout+kickoff plan for the full roadmap.

use serde::{Deserialize, Serialize};

/// Placeholder for a connected Frida session (scaffolding only; no real handle yet).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FridaSession {
    pub device_id: String,
    /// Reserved for future frida-rs session/device objects (kept private to this module).
    _stub: (),
}

/// Structured result from running a Frida script (user-provided or built-in).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FridaScriptResult {
    pub script_source: String,
    pub output: String,        // raw or JSON payload from Frida
    pub findings: Vec<String>, // extracted high-level signals (to become DynamicMobileFinding)
    pub duration_ms: u64,
}

/// Extension point for Frida data on `DynamicMobileReport` (cfg-gated behind mobile-frida).
/// Currently a minimal placeholder so the report type can evolve without breaking changes.
/// Real data will include sessions, script results, and mapped findings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FridaInstrumentation {
    pub note: String,
    // Planned (future passes):
    // pub sessions: Vec<FridaSession>,
    // pub script_results: Vec<FridaScriptResult>,
    // pub enabled_builtins: Vec<String>,
}

/// Connect to the Frida server on the target device/emulator (stub).
///
/// Intended: requires frida-server reachable (ADB forwarded port or direct on emulator).
/// Returns a session handle for subsequent script execution.
///
/// This is scaffolding: always errors with a clear "not yet implemented" message.
/// Real implementation will be added after CLI/policy wiring in later Phase 3a work.
pub fn connect(_device: &str) -> crate::error::Result<FridaSession> {
    Err(crate::error::EggsecError::Validation(
        "mobile-frida: connect() is scaffolding only (Phase 3 kickoff pass). \
         Not yet implemented. See module docs in frida.rs and the \
         mobile-dynamic-phase2-closeout-and-phase3-kickoff-plan.md for vision + next steps."
            .to_string(),
    ))
}

/// Execute a Frida script (JavaScript) in the context of an existing session (stub).
///
/// Supports both user-supplied scripts (future --frida-script) and built-in high-signal
/// scripts (e.g. the method tracer sketch).
///
/// Scaffolding only: returns a "not yet implemented" error. No Frida runtime is invoked.
pub fn execute_script(
    _session: &FridaSession,
    _script: &str,
) -> crate::error::Result<FridaScriptResult> {
    Err(crate::error::EggsecError::Validation(
        "mobile-frida: execute_script() is scaffolding only; not yet implemented.".to_string(),
    ))
}

/// Sketch / documented stub for the first high-value built-in capability: basic method tracing.
///
/// **Intended behavior (not implemented in this pass)**:
/// Hook the listed methods inside the target package, log invocations + args/returns,
/// and return structured results that can be turned into findings (e.g. secret material,
/// crypto usage, auth flows, or bypass detections).
///
/// Example target list for a real implementation:
///   ["javax.crypto.Cipher.doFinal", "android.security.KeyStore.getKey", ...]
///
/// This function exists as a design marker and to lock the planned signature.
/// It currently returns a clear stub error and performs no work.
pub fn basic_method_trace(
    _session: &FridaSession,
    _package: &str,
    _methods: &[&str],
) -> crate::error::Result<FridaScriptResult> {
    Err(crate::error::EggsecError::Validation(
        "mobile-frida: basic_method_trace() is a Phase 3 design marker (stub). \
         High-value target: tracing crypto/keystore/auth methods + bypass validation. \
         Real implementation deferred.".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frida_module_present_and_stubs_construct_without_side_effects() {
        // Verifies the module is visible under the mobile-frida feature gate,
        // placeholder types are constructible, and stubs error cleanly with no
        // real Frida side-effects, device access, or external dependencies.
        let sess = FridaSession::default();
        assert!(sess.device_id.is_empty());

        let res: FridaScriptResult = Default::default();
        assert!(res.script_source.is_empty());
        assert!(res.findings.is_empty());

        let instr = FridaInstrumentation::default();
        assert!(instr.note.is_empty());

        // Stubs must fail fast with our documented scaffolding message.
        let err = connect("emulator-5554").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("scaffolding only"));
        assert!(msg.contains("mobile-frida"));
        assert!(msg.contains("Phase 3 kickoff"));

        let err2 =
            basic_method_trace(&sess, "com.example.vuln", &["javax.crypto.Cipher.doFinal"])
                .unwrap_err();
        assert!(err2.to_string().contains("basic_method_trace() is a Phase 3 design marker"));
    }
}
