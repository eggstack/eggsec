pub mod types;
pub mod detect;
pub mod block_check;
pub mod compare;

#[cfg(test)]
mod tests;

pub use types::{WafDetectionResult, ResponseDiff};

use crate::error::Result;
use crate::utils::create_insecure_client_with_options;

use super::waf_patterns::{get_waf_signatures, WafSignature};
use types::WafSignatureLower;

pub struct WafDetector {
    client: reqwest::Client,
    signatures: std::collections::HashMap<String, WafSignature>,
    signatures_lower: std::collections::HashMap<String, WafSignatureLower>,
}

impl WafDetector {
    pub fn new() -> Result<Self> {
        let ua = crate::waf::bypass::headers::get_random_ua().to_string();
        let client = create_insecure_client_with_options(15, |builder| {
            builder
                .redirect(reqwest::redirect::Policy::limited(5))
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
        })
    }
}
