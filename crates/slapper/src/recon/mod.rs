//! Reconnaissance and intelligence gathering module
//!
//! Provides comprehensive reconnaissance capabilities for gathering information
//! about target systems before security testing.
//!
//! ## Key Components
//!
//! - [`FullReconResult`] - Aggregated results from all recon modules
//! - [`run_full_recon`] - Main entry point for full recon execution
//! - [`TechDetector`](techdetect::TechDetector) - Technology stack detection
//! - [`SubdomainEnumerator`](subdomain::SubdomainEnumerator) - Subdomain enumeration
//! - [`SslAnalyzer`](ssl::SslAnalyzer) - SSL/TLS certificate analysis
//! - [`CorsAnalyzer`](cors::CorsAnalyzer) - CORS misconfiguration detection
//! - [`CveMapper`](cve::CveMapper) - CVE mapping for detected technologies
//!
//! ## Full Recon Pipeline Modules
//!
//! `run_full_recon` is a curated pipeline, not an invocation of every module in
//! `src/recon`.
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
//! - `cloud` - Cloud service discovery (AWS, GCP, Azure)
//! - `content` - Content and directory discovery
//! - `js` - JavaScript file analysis for endpoints and secrets
//! - `wayback` - Wayback Machine historical URL discovery
//! - `takeover` - Subdomain takeover detection
//! - `threatintel` - Threat intelligence lookup
//! - `email` / `email_security` - Email discovery and security analysis
//! - `dependency_scan` - Dependency vulnerability scanning
//! - `git_secrets` - Git repository secret detection
//! - `api_schema` - API schema discovery
//!
//! ## Additional Recon Utilities (standalone)
//!
//! Exported modules like `email_security`, `dependency_scan`, `git_secrets`,
//! and `api_schema` are available for direct invocation, but are not currently
//! part of `run_full_recon`.
//!
//! See [`FULL_RECON_PIPELINE_MODULES`] for the exact module list used by
//! `run_full_recon`.
//!
//! ## Feature Flags
//!
//! | Feature | Modules Enabled |
//! |---------|----------------|
//! | `git-secrets` | `git_secrets` |
//! | `api-schema` | `api_schema` |
//! | `cloud` | `cloud` |
//!
//! ## Usage
//!
//! ```rust,no_run
//! use slapper::recon::{FullReconResult, TechDetector};
//! use slapper::config::SlapperConfig;
//!
//! # async fn example() -> slapper::error::Result<()> {
//! let config = SlapperConfig::default();
//! let detector = TechDetector::new("example.com".to_string(), config.into());
//! let tech_stack = detector.detect().await?;
//! println!("Detected {} technologies", tech_stack.technologies.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Errors
//!
//! Recon operations may fail with [`SlapperError`](crate::error::SlapperError) for:
//! - Invalid target domains or IPs
//! - Network connectivity issues
//! - DNS resolution failures
//! - External API rate limiting (crt.sh, Shodan, etc.)
//! - Timeout during long-running enumeration

pub mod api_schema;
pub mod cloud;
pub mod containers;
pub mod content;
pub mod cors;
pub mod cve;
pub mod dependency_scan;
pub mod dns_records;
pub mod email;
pub mod email_security;
pub mod geolocation;
pub mod git_secrets;
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
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub use spinner::Spinner;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FullReconResult {
    pub target: String,
    pub domain: Option<String>,
    pub ip_address: Option<String>,
    pub tech_stack: Option<techdetect::TechStack>,
    pub tech_error: Option<String>,
    pub reverse_dns: Option<reverse_dns::ReverseDnsResult>,
    pub reverse_dns_error: Option<String>,
    pub geolocation: Option<geolocation::GeoLocation>,
    pub geoip_error: Option<String>,
    pub whois: Option<whois::WhoisResult>,
    pub whois_error: Option<String>,
    pub subdomains: Option<subdomain::SubdomainResult>,
    pub subdomains_error: Option<String>,
    pub ssl_analysis: Option<ssl::SslAnalysis>,
    pub ssl_error: Option<String>,
    pub dns_records: Option<dns_records::DnsRecords>,
    pub dns_records_error: Option<String>,
    pub js_analysis: Option<js::JsAnalysis>,
    pub js_error: Option<String>,
    pub wayback: Option<wayback::WaybackResult>,
    pub wayback_error: Option<String>,
    pub cloud: Option<cloud::CloudDiscovery>,
    pub cloud_error: Option<String>,
    pub content: Option<content::ContentDiscovery>,
    pub content_error: Option<String>,
    pub cors: Option<cors::CorsAnalysis>,
    pub cors_error: Option<String>,
    pub email_discovery: Option<email::EmailDiscovery>,
    pub email_error: Option<String>,
    pub threat_intel: Option<threatintel::ThreatIntel>,
    pub threat_intel_error: Option<String>,
    pub cve_mapping: Option<cve::CveMapping>,
    pub cve_error: Option<String>,
    pub takeover: Option<Vec<takeover::TakeoverResult>>,
    pub takeover_error: Option<String>,
}

impl FullReconResult {
    pub fn new(target: &str) -> Self {
        Self {
            target: target.to_string(),
            ..Default::default()
        }
    }
}

#[cfg(feature = "tool-api")]
pub async fn run_cli_with_callback<F>(
    args: ReconArgs,
    config: &SlapperConfig,
    mut callback: F,
) -> Result<()>
where
    F: FnMut(crate::tool::response::Finding) + Send + 'static,
{
    let stage = Arc::new(Mutex::new(String::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let has_spinner = !args.quiet;
    let verbose = args.verbose;

    if has_spinner {
        let stop_clone = stop.clone();
        let stage_clone = stage.clone();
        std::thread::spawn(move || {
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

    if let Some(ref cve_mapping) = recon.cve_mapping {
        for vuln in &cve_mapping.vulnerabilities {
            callback(crate::tool::response::Finding::from(vuln.clone()));
        }
    }

    if let Some(ref tech_stack) = recon.tech_stack {
        for server in &tech_stack.servers {
            callback(crate::tool::response::Finding {
                id: uuid::Uuid::new_v4().to_string(),
                finding_type: crate::tool::response::FindingType::Technology,
                severity: crate::tool::response::ResponseSeverity::Info,
                title: format!("Technology detected: {}", server),
                description: format!("Detected server technology: {}", server),
                location: server.clone(),
                evidence: None,
                cve_ids: vec![],
                remediation: None,
                references: vec![],
                metadata: {
                    let mut m = std::collections::HashMap::new();
                    m.insert(
                        "technology".to_string(),
                        serde_json::Value::String(server.clone()),
                    );
                    m
                },
            });
        }
    }

    if let Some(ref takeover_results) = recon.takeover {
        for result in takeover_results {
            let title = format!(
                "Potential subdomain takeover: {} ({})",
                result.target.subdomain,
                result.service.as_deref().unwrap_or("unknown service")
            );
            callback(crate::tool::response::Finding {
                id: uuid::Uuid::new_v4().to_string(),
                finding_type: crate::tool::response::FindingType::Vulnerability,
                severity: crate::tool::response::ResponseSeverity::High,
                title,
                description: result.evidence.clone(),
                location: result.target.subdomain.clone(),
                evidence: result.target.cname.clone(),
                cve_ids: vec![],
                remediation: Some(
                    "Register the dormant subdomain or remove the DNS record".to_string(),
                ),
                references: vec![],
                metadata: {
                    let mut m = std::collections::HashMap::new();
                    m.insert(
                        "cname".to_string(),
                        serde_json::to_value(&result.target.cname).unwrap_or(serde_json::Value::Null),
                    );
                    m.insert(
                        "ns".to_string(),
                        serde_json::to_value(&result.target.ns).unwrap_or(serde_json::Value::Null),
                    );
                    m.insert(
                        "service".to_string(),
                        serde_json::to_value(&result.service).unwrap_or(serde_json::Value::Null),
                    );
                    m
                },
            });
        }
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

pub async fn run_cli(args: ReconArgs, config: &SlapperConfig) -> Result<()> {
    let stage = Arc::new(Mutex::new(String::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let has_spinner = !args.quiet;
    let verbose = args.verbose;

    if has_spinner {
        let stop_clone = stop.clone();
        let stage_clone = stage.clone();
        std::thread::spawn(move || {
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

pub const FULL_RECON_PIPELINE_MODULES: &[&str] = &[
    "reverse_dns",
    "geolocation",
    "threatintel",
    "ssl",
    "whois",
    "subdomain",
    "dns_records",
    "techdetect",
    "js",
    "wayback",
    "cloud",
    "content",
    "cors",
    "email",
    "takeover",
    "cve",
    "secrets",
];

#[cfg(test)]
mod module_registration_tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::Path;

    #[test]
    fn recon_modules_match_filesystem() {
        let mod_src = include_str!("mod.rs");
        let declared: BTreeSet<String> = mod_src
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("pub mod ") {
                    return rest.strip_suffix(';').map(str::to_string);
                }
                None
            })
            .collect();

        let recon_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/recon");
        let mut discovered = BTreeSet::new();

        for entry in fs::read_dir(&recon_dir).expect("read src/recon") {
            let entry = entry.expect("read_dir entry");
            let path = entry.path();

            if path.is_file() {
                if path.extension().and_then(|ext| ext.to_str()) == Some("rs")
                    && path.file_name().and_then(|n| n.to_str()) != Some("mod.rs")
                {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        discovered.insert(stem.to_string());
                    }
                }
            } else if path.is_dir() && path.join("mod.rs").is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    discovered.insert(name.to_string());
                }
            }
        }

        let intentionally_detached: BTreeSet<String> = [
            "asn",
            "cve_lookup",
            "dns_enhanced",
            "ftp_auth",
            "smtp_auth",
            "ssh_auth",
            "ssl_audit",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();

        let discovered: BTreeSet<String> = discovered
            .into_iter()
            .filter(|m| !intentionally_detached.contains(m))
            .collect();

        assert_eq!(
            declared, discovered,
            "recon module declarations are out of sync with src/recon filesystem"
        );
    }
}
