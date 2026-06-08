pub mod block_check;
pub mod compare;
pub mod detect;
pub mod types;

#[cfg(test)]
mod tests;

pub use types::{ResponseDiff, WafDetectionResult};

use crate::constants::waf;
use crate::error::Result;
use crate::utils::circuit_breaker::CircuitBreaker;
use crate::utils::create_insecure_client_with_options;
use rustc_hash::FxHashMap;
use std::sync::Arc;

use super::waf_patterns::{get_waf_signatures, WafSignature};
use types::WafSignatureLower;

pub struct WafDetector {
    client: reqwest::Client,
    signatures: &'static FxHashMap<String, WafSignature>,
    signatures_lower: FxHashMap<String, WafSignatureLower>,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl WafDetector {
    pub fn new() -> Result<Self> {
        let ua = crate::waf::bypass::headers::get_random_ua().to_string();
        let client = create_insecure_client_with_options(waf::SMUGGLING_TIMEOUT_SECS, |builder| {
            builder
                .redirect(reqwest::redirect::Policy::limited(waf::MAX_REDIRECTS))
                .user_agent(ua)
        })?;

        let signatures = get_waf_signatures();
        let signatures_lower = signatures
            .iter()
            .map(|(key, sig)| {
                (
                    key.clone(),
                    WafSignatureLower {
                        headers: sig.headers.iter().map(|h| h.to_lowercase()).collect(),
                        cookies: sig.cookies.iter().map(|c| c.to_lowercase()).collect(),
                        body_patterns: sig.body_patterns.iter().map(|p| p.to_lowercase()).collect(),
                    },
                )
            })
            .collect();

        Ok(Self {
            client,
            signatures,
            signatures_lower,
            circuit_breaker: Arc::new(CircuitBreaker::default()),
        })
    }
}
