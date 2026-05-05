//! Security fuzzing engine with 22+ payload types
//!
//! This module provides a comprehensive fuzzing engine for discovering security vulnerabilities
//! in web applications. It supports 22 different payload types including SQL injection, XSS,
//! SSRF, path traversal, and more.
//!
//! ## Key Components
//!
//! - [`FuzzEngine`] - Main fuzzing engine that orchestrates payload generation and testing
//! - [`Payload`] - Individual test payload with type, severity, and description
//! - [`PayloadType`] - Enumeration of all supported payload types (22 types)
//! - [`Severity`] - Vulnerability severity levels (Critical, High, Medium, Low, Info)
//! - [`FuzzResult`] - Result of a single fuzzing test
//!
//! ## Feature Flags
//!
//! - `stress-testing` - Enables advanced evasion and WAF bypass features
//!
//! ## Usage
//!
//! ### Getting Payloads
//!
//! ```rust,no_run
//! use slapper::fuzzer::{FuzzEngine, PayloadType, get_payloads, Severity};
//!
//! // Get SQL injection payloads
//! let payloads = get_payloads(PayloadType::Sqli);
//! for payload in payloads.iter().take(5) {
//!     println!("[{}] {}", payload.severity, payload.payload);
//! }
//! ```
//!
//! ### Running a Fuzz Session
//!
//! ```rust,compile_fail
//! use slapper::cli::{FuzzArgs, FuzzMode, CommonHttpArgs};
//! use slapper::fuzzer::FuzzEngine;
//!
//! # async fn example() -> slapper::error::Result<()> {
//! let args = FuzzArgs {
//!     url: "https://example.com/api?id=1".to_string(),
//!     payload_type: "sqli".to_string(),
//!     mode: FuzzMode::Sequential,
//!     concurrency: 10,
//!     timeout: 30,
//!     ..Default::default()
//! };
//!
//! let mut engine = FuzzEngine::new(args);
//! let session = engine.run_return_session().await?;
//!
//! println!("Found {} potential vulnerabilities", session.results.len());
//! # Ok(())
//! # }
//! ```
//!
//! ### Mutation-Based Fuzzing
//!
//! ```rust,no_run
//! use slapper::fuzzer::generate_mutations;
//!
//! let original = "' OR 1=1--";
//! let mutations = generate_mutations(original, 5);
//! for mutation in &mutations {
//!     println!("Mutated: {}", mutation);
//! }
//! ```
//!
//! ## Payload Types
//!
//! The fuzzer supports these payload categories:
//! - **Injection**: SQLi, XSS, SSTI, Command Injection, LDAP, XXE
//! - **Access Control**: IDOR, JWT vulnerabilities
//! - **Server-Side**: SSRF, ReDoS, Deserialization
//! - **Client-Side**: Open Redirect, CSV Injection
//! - **API Security**: GraphQL, OAuth/OIDC, gRPC
//! - **Infrastructure**: Host Header Injection, Cache Poisoning, Compression Bombs
//!
//! ## Errors
//!
//! Functions return [`crate::error::Result`] and will fail if:
//! - URL parsing fails
//! - HTTP client construction fails
//! - Network connectivity issues occur
//! - Invalid payload type is specified

pub mod advanced;
pub mod api_schema;
pub mod calibration;
pub mod chain;
pub mod detection;
pub mod diff;
pub mod engine;
pub mod filters;
pub mod grammar;
pub mod mutator;
pub mod payloads;
pub mod rate_limit;
pub mod redos_detect;
pub mod state;
pub mod targets;
pub mod waf_fingerprint;
pub use advanced::{
    AdvancedFuzzer, GraphQLFuzzer, GrpcFuzzer, IdorFuzzer, JwtFuzzer, OAuthFuzzer, SstiFuzzer,
    WebSocketFuzzer,
};

use crate::error::Result;

#[cfg(feature = "tool-api")]
use crate::tool::response::Finding;

use crate::cli::FuzzArgs;

pub use calibration::{BaselineStats, CalibrationResult, Calibrator};
pub use chain::{
    AutoExploiter, ChainAction, ChainExecutionResult, ChainExecutor, ChainedFuzzResult,
};
pub use diff::{DiffResult, ResponseDiff, ResponseDiffer};
pub use engine::{
    ChainedFuzzInput, ChainedFuzzOutput, FuzzChainStep, FuzzEngine, FuzzResult, FuzzSession,
    OwaspSummary, StatefulFuzzer, StepResults,
};
pub use filters::{FilterChain, PayloadFilter};
pub use grammar::{Grammar, GrammarFuzzer};
pub use mutator::generate_mutations;
pub use payloads::{
    get_all_payloads_cached, get_payloads, get_payloads_cached, Payload, PayloadType, Severity,
};
pub use rate_limit::{AdaptiveRateLimiter, RateLimiterTokenBucket};
pub use redos_detect::{PayloadReDosChecker, ReDosDetector, ReDosResult, RegexExecutor};
pub use state::{AuthCredentials, AuthHandler, AuthType, HttpSession, SessionManager};
pub use targets::{get_target_payloads, TargetPayload, TargetType};
pub use waf_fingerprint::{WafDetectionResult, WafFingerprint, WafFingerprinter};

/// Run the fuzzer CLI with the given arguments
///
/// # Arguments
///
/// * `args` - Fuzzing arguments from the CLI
///
/// # Returns
///
/// Result indicating success or failure of the fuzzing operation
pub async fn run_cli(args: FuzzArgs) -> Result<()> {
    let mut engine = engine::FuzzEngine::new(args.clone())?;
    engine.run().await
}

/// Run the fuzzer CLI with a callback for streaming findings
///
/// # Arguments
///
/// * `args` - Fuzzing arguments from the CLI
/// * `callback` - A mutable function called for each vulnerable finding
///
/// # Returns
///
/// Result indicating success or failure of the fuzzing operation
#[cfg(feature = "tool-api")]
pub async fn run_cli_with_callback<F>(args: FuzzArgs, mut callback: F) -> Result<()>
where
    F: FnMut(Finding) + Send + 'static,
{
    let mut engine = engine::FuzzEngine::new(args)?;
    let session = engine.run_return_session().await?;
    for result in session.results {
        if result.is_vulnerable() {
            callback(Finding::from(result));
        }
    }
    Ok(())
}

/// Run WAF stress testing
///
/// Tests WAF effectiveness by sending various attack payloads and measuring detection rates.
///
/// # Arguments
///
/// * `args` - WAF stress testing arguments
///
/// # Returns
///
/// Result indicating success or failure of the stress test
pub async fn run_waf_stress(args: crate::cli::WafStressArgs) -> Result<()> {
    let mut engine = engine::FuzzEngine::new_from_waf_args(args.clone())?;
    engine.run_all_types().await
}
