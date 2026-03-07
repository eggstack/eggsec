#![allow(unused_imports)]

pub mod chain;
pub mod detection;
pub mod diff;
pub mod engine;
pub mod grammar;
pub mod mutator;
pub mod payloads;
pub mod rate_limit;
pub mod redos_detect;
pub mod state;
pub mod targets;
pub mod waf_fingerprint;
pub mod advanced;
pub use advanced::{
    AdvancedFuzzer, 
    GraphQLFuzzer, JwtFuzzer, OAuthFuzzer, IdorFuzzer, 
    SstiFuzzer, WebSocketFuzzer, GrpcFuzzer,
};

use anyhow::Result;

use crate::cli::FuzzArgs;
use crate::config::SlapperConfig;

pub use chain::{AutoExploiter, ChainAction, ChainExecutor, ChainExecutionResult, ChainedFuzzResult};
pub use diff::{ResponseDiff, ResponseDiffer, DiffResult};
pub use engine::{FuzzEngine, FuzzResult, FuzzSession, OwaspSummary};
pub use grammar::{Grammar, GrammarFuzzer};
pub use mutator::generate_mutations;
pub use payloads::{get_all_payloads, get_payloads, Payload, PayloadType, Severity};
pub use rate_limit::{AdaptiveRateLimiter, RateLimiterTokenBucket};
pub use redos_detect::{ReDosDetector, ReDosResult, RegexExecutor, PayloadReDosChecker};
pub use state::{AuthHandler, AuthType, AuthCredentials, HttpSession, SessionManager};
pub use targets::{TargetType, TargetPayload, get_target_payloads};
pub use waf_fingerprint::{WafDetectionResult, WafFingerprinter, WafFingerprint};

pub async fn run_cli(args: FuzzArgs, _config: &SlapperConfig) -> Result<()> {
    let mut engine = engine::FuzzEngine::new(args.clone());
    engine.run().await
}

pub async fn run_waf_stress(args: crate::cli::WafStressArgs, _config: &SlapperConfig) -> Result<()> {
    let mut engine = engine::FuzzEngine::new_from_waf_args(args.clone());
    engine.run_all_types().await
}
