//! Web Application Firewall (WAF) detection and bypass
//!
//! This module provides comprehensive WAF testing capabilities including:
//! - Detection of 34 WAF products (Cloudflare, AWS WAF, ModSecurity, etc.)
//! - Bypass techniques (header manipulation, encoding, smuggling)
//! - WAF stress testing with multiple attack vectors
//! - Profile-based testing for specific WAF products
//!
//! ## Key Components
//!
//! - [`WafDetector`] - Detects WAF presence through header and response analysis
//! - [`BypassEngine`] - Attempts various bypass techniques against detected WAFs
//! - [`WafProfile`] - WAF-specific bypass configurations
//! - [`TestType`] - Types of tests to run (SQLi, XSS, SSRF, etc.)
//!
//! ## Supported WAFs
//!
//! Cloudflare, Akamai, AWS WAF, Azure WAF, Google Cloud Armor, Fastly, Imperva,
//! Sucuri, CloudFront, F5 BIG-IP, Barracuda, Fortinet, Citrix NetScaler,
//! ModSecurity, Wordfence, DataDome, PerimeterX, Nginx, Traefik, Kong,
//! Varnish, Radware, Signal Sciences, Wallarm, Reblaze
//!
//! ## Usage
//!
//! ### Basic WAF Detection
//!
//! ```rust,no_run
//! use eggsec::waf::{WafDetector, WafDetectionResult};
//!
//! # async fn example() -> eggsec::error::Result<()> {
//! let detector = WafDetector::new()?;
//! let detection = detector.detect("https://example.com").await?;
//!
//! if let Some(waf_name) = &detection.waf_name {
//!     println!("WAF detected: {} ({}% confidence)", waf_name, detection.confidence);
//!     for header in &detection.matched_headers {
//!         println!("  Indicator: {}", header);
//!     }
//! } else {
//!     println!("No WAF detected");
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### WAF Bypass Testing
//!
//! ```rust,compile_fail
//! use eggsec::waf::{BypassEngine, TestType, get_profile_by_name};
//! use eggsec::cli::WafArgs;
//!
//! # async fn example() -> eggsec::error::Result<()> {
//! let args = WafArgs {
//!     url: "https://example.com".to_string(),
//!     bypass: true,
//!     header_bypass: true,
//!     profile: "cloudflare".to_string(),
//!     ..Default::default()
//! };
//!
//! let profile = get_profile_by_name("cloudflare");
//! let engine = BypassEngine::new(&args, profile, TestType::Sql)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Errors
//!
//! Functions return [`crate::error::Result`] and will fail if:
//! - URL is invalid or unreachable
//! - HTTP client construction fails
//! - Network connectivity issues occur

pub mod bypass;
pub mod data;
pub mod detector;
pub mod output;
pub mod payloads;
pub mod regression_report;
pub mod types;
pub mod waf_patterns;

use crate::error::Result;
use std::time::Instant;

use crate::cli::WafArgs;
use crate::utils::sanitize_for_logging;

pub use bypass::{
    get_auto_profile, get_profile_by_detection_sig, get_profile_by_name, BypassEngine,
    BypassResult, TestType, WafProfile,
};
pub use detector::{WafDetectionResult, WafDetector};
pub use regression_report::{WafBehavior, WafBehaviorSummary, WafRegressionCase, WafRegressionReport};
pub use types::{Finding, OwaspCategory, ScanResults, Severity};
pub use waf_patterns::get_waf_signatures;

/// Run WAF detection and bypass testing from CLI
///
/// # Arguments
///
/// * `args` - WAF testing arguments from CLI
///
/// # Returns
///
/// Result indicating success or failure
pub async fn run_cli(args: WafArgs) -> Result<()> {
    let mut engine = WafEngine::new(args)?;
    engine.run().await
}

pub struct WafEngine {
    args: WafArgs,
    detector: WafDetector,
    bypass_engine: Option<BypassEngine>,
    selected_profile: Option<String>,
    #[cfg(feature = "ai-integration")]
    ai_bypass: Option<crate::ai::SmartWafBypass>,
}

impl WafEngine {
    pub fn new(args: WafArgs) -> Result<Self> {
        let detector = WafDetector::new()?;
        Ok(Self {
            args,
            detector,
            bypass_engine: None,
            selected_profile: None,
            #[cfg(feature = "ai-integration")]
            ai_bypass: None,
        })
    }

    #[cfg(feature = "ai-integration")]
    pub fn set_ai_bypass(&mut self, bypass: crate::ai::SmartWafBypass) {
        self.ai_bypass = Some(bypass);
    }

    #[cfg(feature = "ai-integration")]
    pub fn ai_bypass(&self) -> Option<&crate::ai::SmartWafBypass> {
        self.ai_bypass.as_ref()
    }

    fn select_profile(&mut self, detection: &WafDetectionResult) -> Option<WafProfile> {
        let profile_name = &self.args.profile;
        if profile_name.to_lowercase() == "auto" {
            if let Some(ref waf_name) = detection.waf_name {
                let waf_lower = waf_name.to_lowercase();

                if let Some(profile) = get_profile_by_name(&waf_lower) {
                    self.selected_profile = Some(profile.name.clone());
                    return Some(profile.clone());
                }

                for sig in get_waf_signatures().keys() {
                    let sig_lower = sig.to_lowercase();
                    if waf_lower == sig_lower
                        || waf_lower.starts_with(&sig_lower)
                        || waf_lower.ends_with(&sig_lower)
                        || waf_lower.contains(&format!(" {}", &sig_lower))
                    {
                        if let Some(profile) = bypass::get_profile_by_detection_sig(sig) {
                            self.selected_profile = Some(profile.name.clone());
                            return Some(profile.clone());
                        }
                    }
                }
                tracing::warn!(
                    "Auto-detected WAF '{}' but no matching profile found, using generic profile",
                    waf_name
                );
            }
            let profile = get_auto_profile();
            self.selected_profile = Some(profile.name.clone());
            return Some(profile);
        } else if let Some(profile) = get_profile_by_name(profile_name) {
            self.selected_profile = Some(profile.name.clone());
            return Some(profile.clone());
        }

        tracing::warn!(
            "Unknown profile '{}', falling back to auto profile",
            profile_name
        );
        let profile = get_auto_profile();
        self.selected_profile = Some(profile.name.clone());
        Some(profile)
    }

    pub async fn run(&mut self) -> Result<()> {
        let start = Instant::now();

        if self.args.verbose {
            tracing::info!("Detecting WAF on {}", sanitize_for_logging(&self.args.url));
        }

        let detection = self.detector.detect(&self.args.url).await?;

        if !self.args.bypass
            && !self.args.header_bypass
            && !self.args.smuggling
            && !self.args.evasion
        {
            self.print_detection(&detection);

            if self.args.verbose {
                if let Some(ref waf_name) = detection.waf_name {
                    tracing::info!("WAF detected: {}", waf_name);
                } else {
                    tracing::info!("No WAF detected");
                }
            }
            return Ok(());
        }

        let profile = self.select_profile(&detection);
        let test_type = self
            .args
            .test_type
            .as_ref()
            .map(|t| TestType::parse(t))
            .unwrap_or(TestType::All);
        self.bypass_engine = Some(BypassEngine::new(&self.args, profile, test_type)?);

        if self.args.verbose {
            tracing::info!("Attempting WAF bypasses...");
        }

        let bypass_results = self
            .bypass_engine
            .as_ref()
            .ok_or_else(|| {
                crate::error::EggsecError::Internal("bypass engine not initialized".to_string())
            })?
            .run_bypasses(&detection)
            .await?;

        #[cfg(feature = "ai-integration")]
        let bypass_results = self.run_ai_bypasses(&detection, bypass_results).await?;

        let findings = bypass_results
            .iter()
            .map(|br| {
                let severity = if br.success {
                    Severity::High
                } else {
                    Severity::Low
                };
                let owasp = OwaspCategory::A05_2021_SecurityMisconfiguration;

                Finding::new(
                    format!("WAF Bypass - {:?}", br.technique),
                    br.description.clone(),
                    severity,
                    owasp,
                    detection.waf_name.clone(),
                    br.success,
                    format!("{:?}", br.technique),
                    br.payload.clone().unwrap_or_default(),
                    br.status_code,
                )
            })
            .collect::<Vec<_>>();

        let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        let scan_results = ScanResults::new(
            self.args.url.clone(),
            duration_ms,
            Some(detection.clone()),
            findings,
        );

        if self.args.verbose {
            let successful = bypass_results.iter().filter(|r| r.success).count();
            let total = bypass_results.len();
            tracing::info!(
                "WAF bypass complete: {} successful out of {} attempts",
                successful,
                total
            );
        }

        let output = if self.args.json {
            serde_json::to_string_pretty(&scan_results)?
        } else {
            output::format_results(&detection, &bypass_results, self.selected_profile.as_ref())
        };

        if let Some(ref output_file) = self.args.output {
            tokio::fs::write(output_file, &output).await?;
            if self.args.verbose {
                tracing::info!("Results written to {}", output_file);
            }
        } else {
            if self.args.json {
                println!("{}", output);
            } else {
                self.print_results(&detection, &bypass_results);
            }
        }

        Ok(())
    }

    fn print_detection(&self, detection: &WafDetectionResult) {
        if self.args.json {
            output::print_detection_json(detection);
            return;
        }

        if let Some(ref profile_name) = self.selected_profile {
            println!("profile: {}", profile_name);
        }

        output::print_detection(detection);
    }

    fn print_results(&self, detection: &WafDetectionResult, bypass_results: &[BypassResult]) {
        if self.args.json {
            output::print_results_json(detection, bypass_results);
            return;
        }

        output::print_results(detection, bypass_results, self.selected_profile.as_ref());
    }

    #[cfg(feature = "ai-integration")]
    async fn run_ai_bypasses(
        &mut self,
        detection: &WafDetectionResult,
        bypass_results: Vec<BypassResult>,
    ) -> Result<Vec<BypassResult>> {
        if let Some(ref mut ai_bypass_engine) = self.ai_bypass {
            let waf_name = detection.waf_name.as_deref().unwrap_or("unknown");

            for br in &bypass_results {
                if !br.success {
                    if let Ok(suggestion) = ai_bypass_engine
                        .find_bypass(waf_name, &br.description)
                        .await
                    {
                        if let Some(suggestion) = suggestion {
                            tracing::info!(
                                "[AI] Suggested bypass for {:?}: {}",
                                br.technique,
                                suggestion
                            );
                        }
                    }
                }
            }
        }
        Ok(bypass_results)
    }
}
