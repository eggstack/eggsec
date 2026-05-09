mod adaptive;
mod cache;
mod client;
mod errors;
mod payloads;
#[cfg(feature = "ai-integration")]
mod planner;
#[cfg(feature = "ai-integration")]
mod script_gen;
mod types;
mod waf_bypass;

pub use adaptive::AdaptiveScanEngine;
pub use cache::{AiCache, CacheKeyBuilder, CacheStats};
pub use client::{AiClient, Provider};
pub use errors::{AiError, Result};
pub use payloads::AiPayloadGenerator;
#[cfg(feature = "ai-integration")]
pub use planner::AiPlanner;
#[cfg(feature = "ai-integration")]
pub use script_gen::{GeneratedScript, PluginLanguage, ScriptGenerator, ScriptMetadata, ScriptTarget};
pub use types::*;
pub use waf_bypass::SmartWafBypass;
