//! Reconnaissance and intelligence gathering module
//!
//! Provides comprehensive reconnaissance capabilities for gathering information
//! about target systems before security testing.
//!
//! ## Key Components
//!
//! - [`FullReconResult`] - Aggregated results from all recon modules
//! - [`techdetect::TechDetector`] - Technology stack detection
//! - [`subdomain::SubdomainEnumerator`] - Subdomain enumeration
//! - [`ssl::SslAnalyzer`] - SSL/TLS certificate analysis
//! - [`cors::CorsAnalyzer`] - CORS misconfiguration detection
//! - [`cve::CveMapper`] - CVE mapping for detected technologies
//!
//! ## Modules
//!
//! - `techdetect` - Technology stack detection (servers, frameworks, CMS)
//! - `subdomain` - Subdomain enumeration via crt.sh, DNS, and brute force
//! - `ssl` - SSL/TLS certificate and configuration analysis
//! - `cors` - CORS policy testing and misconfiguration detection
//! - `cve` - CVE mapping for detected technologies
//! - `dns_records` - DNS record enumeration (A, AAAA, MX, TXT, etc.)
//! - `whois` - WHOIS information gathering
//! - `geolocation` - IP geolocation lookup
//! - `secrets` - Secret detection in responses (API keys, tokens)

pub mod cloud;
pub mod content;
pub mod cors;
pub mod cve;
pub mod dependency_scan;
pub mod dns_records;
pub mod email;
pub mod email_security;
pub mod geolocation;
pub mod git_secrets;
pub mod api_schema;
pub mod js;
pub mod reverse_dns;
pub mod runner;
pub mod secrets;
pub mod spinner;
pub mod ssl;
pub mod subdomain;
pub mod takeover;
pub mod techdetect;
pub mod threatintel;
pub mod wayback;
pub mod whois;

use crate::cli::ReconArgs;
use crate::config::SlapperConfig;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub use spinner::Spinner;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FullReconResult {
    pub target: String,
    pub domain: Option<String>,
    pub ip_address: Option<String>,
    pub tech_stack: Option<techdetect::TechStack>,
    pub reverse_dns: Option<reverse_dns::ReverseDnsResult>,
    pub geolocation: Option<geolocation::GeoLocation>,
    pub geoip_error: Option<String>,
    pub whois: Option<whois::WhoisResult>,
    pub subdomains: Option<subdomain::SubdomainResult>,
    pub ssl_analysis: Option<ssl::SslAnalysis>,
    pub dns_records: Option<dns_records::DnsRecords>,
    pub js_analysis: Option<js::JsAnalysis>,
    pub wayback: Option<wayback::WaybackResult>,
    pub cloud: Option<cloud::CloudDiscovery>,
    pub content: Option<content::ContentDiscovery>,
    pub cors: Option<cors::CorsAnalysis>,
    pub email_discovery: Option<email::EmailDiscovery>,
    pub threat_intel: Option<threatintel::ThreatIntel>,
    pub cve_mapping: Option<cve::CveMapping>,
    pub takeover: Option<Vec<takeover::TakeoverResult>>,
}

impl FullReconResult {
    pub fn new(target: &str) -> Self {
        Self {
            target: target.to_string(),
            ..Default::default()
        }
    }
}

pub async fn run_cli(args: ReconArgs, config: &SlapperConfig) -> Result<()> {
    let stage = Arc::new(Mutex::new(String::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let has_spinner = !args.quiet;
    let verbose = args.verbose;

    if has_spinner {
        let stop_clone = stop.clone();
        let stage_clone = stage.clone();
        tokio::task::spawn_blocking(move || {
            let mut spinner = Spinner::new(stop_clone, stage_clone);
            while !spinner.stop.load(Ordering::Relaxed) {
                spinner.tick();
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            spinner.stop();
        });
        runner::set_stage(&stage, "init");
    }

    let recon = runner::run_full_recon(&args, config, stage, verbose).await?;

    if has_spinner {
        stop.store(true, Ordering::Relaxed);
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    let output = if args.json {
        serde_json::to_string_pretty(&recon)?
    } else {
        let mut buf = Vec::new();
        if !has_spinner {
            buf.extend_from_slice(b"\n");
        }
        buf.extend_from_slice(runner::print_recon_results_string(&recon).as_bytes());
        String::from_utf8(buf)?
    };

    if let Some(ref output_file) = args.output {
        tokio::fs::write(output_file, &output).await?;
        if !args.quiet && !args.json {
            eprintln!("Results written to {}", output_file);
        }
    } else {
        println!("{}", output);
    }

    Ok(())
}

pub use runner::{print_recon_results_string, run_full_recon};
