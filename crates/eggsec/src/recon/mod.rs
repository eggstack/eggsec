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
//! - `git_secrets` - Git repository secret detection
//! - `api_schema` - API schema discovery
//!
//! ## Additional Recon Utilities (standalone)
//!
//! Exported modules like `email_security`, `git_secrets`,
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
//! | `cloud` | `cloud` |
//!
//! ## Usage
//!
//! ```rust,no_run
//! use eggsec::recon::techdetect::TechDetector;
//!
//! # async fn example() -> eggsec::error::Result<()> {
//! let detector = TechDetector::new()?;
//! let result = detector.detect("https://example.com").await?;
//! println!("Detected {} server technologies", result.tech_stack.servers.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Errors
//!
//! Recon operations may fail with [`EggsecError`](crate::error::EggsecError) for:
//! - Invalid target domains or IPs
//! - Network connectivity issues
//! - DNS resolution failures
//! - External API rate limiting (crt.sh, Shodan, etc.)
//! - Timeout during long-running enumeration

pub mod api_schema;
#[cfg(feature = "cloud")]
pub mod cloud;
pub mod containers;
pub mod content;
pub mod cors;
pub mod cve;
pub mod dns_records;
pub mod email;
pub mod email_security;
pub mod geolocation;
#[cfg(feature = "git-secrets")]
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

#[cfg(feature = "cli")]
use crate::cli::ReconArgs;
use crate::config::EggsecConfig;
use crate::error::Result;
use parking_lot::Mutex;
#[cfg(feature = "tool-api")]
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub use spinner::Spinner;

#[cfg(feature = "cli")]
struct SpinnerGuard {
    stop: Arc<AtomicBool>,
    has_spinner: bool,
    thread: Option<std::thread::JoinHandle<()>>,
}

#[cfg(feature = "cli")]
impl SpinnerGuard {
    fn start(args: &ReconArgs, stage: &Arc<Mutex<String>>) -> Self {
        let has_spinner = !args.quiet;
        let stop = Arc::new(AtomicBool::new(false));
        let mut thread = None;

        if has_spinner {
            let stop_clone = stop.clone();
            let stage_clone = stage.clone();
            thread = Some(std::thread::spawn(move || {
                let mut spinner = Spinner::new(stop_clone, stage_clone);
                while !spinner.stop.load(Ordering::Relaxed) {
                    spinner.tick();
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                spinner.stop();
            }));
            runner::set_stage(stage, "init");
        }

        Self {
            stop,
            has_spinner,
            thread,
        }
    }

    async fn stop(&mut self) {
        if self.has_spinner {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(thread) = self.thread.take() {
                let join = tokio::time::timeout(
                    std::time::Duration::from_secs(1),
                    tokio::task::spawn_blocking(move || thread.join()),
                )
                .await;
                match join {
                    Ok(Ok(Ok(()))) => {}
                    Ok(Ok(Err(_))) => tracing::warn!("Recon spinner thread panicked"),
                    Ok(Err(e)) => tracing::warn!("Failed to join recon spinner: {}", e),
                    Err(_) => tracing::warn!("Timed out joining recon spinner thread"),
                }
            }
        }
    }
}

#[cfg(feature = "cli")]
async fn write_recon_output(
    recon: &FullReconResult,
    args: &ReconArgs,
    has_spinner: bool,
) -> Result<()> {
    let output = if args.json {
        serde_json::to_string_pretty(recon)?
    } else {
        let mut buf = Vec::new();
        if !has_spinner {
            buf.extend_from_slice(b"\n");
        }
        buf.extend_from_slice(runner::print_recon_results_string(recon).as_bytes());
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
    #[cfg(feature = "cloud")]
    pub cloud: Option<cloud::CloudDiscovery>,
    #[cfg(feature = "cloud")]
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
    pub secrets: Option<Vec<secrets::SecretFinding>>,
}

impl FullReconResult {
    pub fn new(target: &str) -> Self {
        Self {
            target: target.to_string(),
            ..Default::default()
        }
    }
}

/// Plain reconnaissance request (no Clap derives).
///
/// This is the engine-facing contract used by the pipeline, Python bindings,
/// and tool/API consumers. CLI parsing converts `ReconArgs` into this type.
#[derive(Debug, Clone, Default)]
pub struct ReconRequest {
    pub target: String,
    pub concurrency: Option<usize>,
    pub no_tech: bool,
    pub no_dns: bool,
    pub no_geo: bool,
    pub no_whois: bool,
    pub no_subdomains: bool,
    pub no_ssl: bool,
    pub no_dns_records: bool,
    pub no_js: bool,
    pub no_content: bool,
    pub no_cloud: bool,
    pub no_wayback: bool,
    pub no_cors: bool,
    pub no_threat: bool,
    pub no_cve: bool,
    pub no_email: bool,
    pub no_takeover: bool,
}

impl ReconRequest {
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            ..Default::default()
        }
    }

    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = Some(concurrency);
        self
    }
}

#[cfg(feature = "cli")]
impl From<crate::cli::ReconArgs> for ReconRequest {
    fn from(args: crate::cli::ReconArgs) -> Self {
        Self {
            target: args.target,
            concurrency: args.concurrency,
            no_tech: args.no_tech,
            no_dns: args.no_dns,
            no_geo: args.no_geo,
            no_whois: args.no_whois,
            no_subdomains: args.no_subdomains,
            no_ssl: args.no_ssl,
            no_dns_records: args.no_dns_records,
            no_js: args.no_js,
            no_content: args.no_content,
            no_cloud: args.no_cloud,
            no_wayback: args.no_wayback,
            no_cors: args.no_cors,
            no_threat: args.no_threat,
            no_cve: args.no_cve,
            no_email: args.no_email,
            no_takeover: args.no_takeover,
        }
    }
}

#[cfg(all(feature = "tool-api", feature = "cli"))]
pub async fn run_cli_with_callback<F>(
    args: ReconArgs,
    config: &EggsecConfig,
    callback: F,
) -> Result<()>
where
    F: FnMut(crate::tool::response::Finding) + Send + 'static,
{
    let verbose = args.verbose;
    let stage = Arc::new(Mutex::new(String::new()));
    let mut spinner = SpinnerGuard::start(&args, &stage);

    let recon_result =
        runner::run_full_recon_from_request(&args.clone().into(), config, stage, verbose).await;
    spinner.stop().await;
    let recon = recon_result?;

    emit_recon_findings(&recon, callback);

    write_recon_output(&recon, &args, spinner.has_spinner).await
}

#[cfg(feature = "tool-api")]
pub async fn run_with_callback<F>(
    request: &ReconRequest,
    config: &EggsecConfig,
    callback: F,
) -> Result<FullReconResult>
where
    F: FnMut(crate::tool::response::Finding) + Send + 'static,
{
    let stage = Arc::new(Mutex::new(String::new()));
    let recon = runner::run_full_recon_from_request(request, config, stage, false).await?;
    emit_recon_findings(&recon, callback);
    Ok(recon)
}

#[cfg(feature = "tool-api")]
fn emit_recon_findings<F>(recon: &FullReconResult, mut callback: F)
where
    F: FnMut(crate::tool::response::Finding) + Send + 'static,
{
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
                    let mut m = FxHashMap::default();
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
                    let mut m = FxHashMap::default();
                    m.insert(
                        "cname".to_string(),
                        serde_json::to_value(&result.target.cname)
                            .unwrap_or(serde_json::Value::Null),
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
}

#[cfg(feature = "cli")]
pub async fn run_cli(args: ReconArgs, config: &EggsecConfig) -> Result<()> {
    let stage = Arc::new(Mutex::new(String::new()));
    let mut spinner = SpinnerGuard::start(&args, &stage);
    let verbose = args.verbose;

    let recon_result =
        runner::run_full_recon_from_request(&args.clone().into(), config, stage, verbose).await;
    spinner.stop().await;
    let recon = recon_result?;

    write_recon_output(&recon, &args, spinner.has_spinner).await
}

pub use runner::print_recon_results_string;
#[cfg(feature = "cli")]
pub use runner::run_full_recon;
pub use runner::run_full_recon_from_request;

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
