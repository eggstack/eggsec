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
//! ## Usage
//!
//! ```rust,no_run
//! use slapper::recon;
//! use slapper::cli::ReconArgs;
//! use slapper::config::SlapperConfig;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let args = ReconArgs {
//!     target: "example.com".to_string(),
//!     concurrency: 10,
//!     ..Default::default()
//! };
//!
//! let config = SlapperConfig::default();
//! recon::run_cli(args, &config).await?;
//! # Ok(())
//! # }
//! ```
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
pub mod dns_records;
pub mod email;
pub mod geolocation;
pub mod js;
pub mod reverse_dns;
pub mod secrets;
pub mod ssl;
pub mod subdomain;
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

struct Spinner {
    chars: &'static [&'static str],
    idx: usize,
    stop: Arc<AtomicBool>,
    stage: Arc<Mutex<String>>,
}

impl Spinner {
    fn new(stop: Arc<AtomicBool>, stage: Arc<Mutex<String>>) -> Self {
        Self {
            chars: &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            idx: 0,
            stop,
            stage,
        }
    }

    fn tick(&mut self) {
        if !self.stop.load(Ordering::Relaxed) {
            if let Ok(stage) = self.stage.lock() {
                eprint!("\r{} {}", self.chars[self.idx], stage);
                self.idx = (self.idx + 1) % self.chars.len();
            }
        }
    }

    fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        eprint!("\r                                                      \r");
    }
}

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
        set_stage(&stage, "init");
    }

    let recon = run_full_recon(&args, config, stage, verbose).await?;

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
        buf.extend_from_slice(print_recon_results_string(&recon).as_bytes());
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

fn set_stage(stage: &Arc<Mutex<String>>, text: &str) {
    if let Ok(mut s) = stage.lock() {
        *s = text.to_string();
    }
}

pub async fn run_full_recon(
    args: &ReconArgs,
    config: &SlapperConfig,
    stage: Arc<Mutex<String>>,
    verbose: bool,
) -> Result<FullReconResult> {
    let target = &args.target;
    let concurrency = args
        .concurrency
        .unwrap_or(config.recon.dns_concurrency.max(10));

    if verbose {
        eprintln!("Starting recon on {}", target);
    }

    set_stage(&stage, "resolving");

    let target_clean = if target.starts_with("http://") {
        target.strip_prefix("http://").unwrap_or(target)
    } else if target.starts_with("https://") {
        target.strip_prefix("https://").unwrap_or(target)
    } else {
        target
    };

    let domain = target_clean.split('/').next().map(|s| s.to_string());
    let url = if target.starts_with("http://") || target.starts_with("https://") {
        target.clone()
    } else {
        format!("https://{}", target)
    };

    let mut recon = FullReconResult::new(target);

    if let Some(ref d) = domain {
        recon.domain = Some(d.clone());
    }

    // Phase 0: DNS resolution (must complete first — other modules need the IP)
    set_stage(&stage, "resolving");

    let resolved_ip: Option<String> = if let Some(ref d) = domain {
        if d.parse::<std::net::IpAddr>().is_ok() {
            Some(d.clone())
        } else {
            match reverse_dns::resolve_domain(d).await {
                Ok(ips) if !ips.is_empty() => {
                    if verbose {
                        eprintln!("Resolved to {}", ips[0]);
                    }
                    Some(ips[0].clone())
                }
                _ => None,
            }
        }
    } else {
        None
    };

    if let Some(ref ip) = resolved_ip {
        recon.ip_address = Some(ip.clone());
    }

    // Phase 1: Run all independent recon modules concurrently.
    // Each module runs only if its --no_* flag is not set.
    set_stage(&stage, "recon (parallel)");

    let ipapi_key = config.recon.apis.ipapi.api_key.as_ref().map(|s| s.expose_secret().to_string());
    let maxmind_settings = if config.recon.apis.maxmind.enabled {
        Some(geolocation::MaxMindSettings {
            account_id: config.recon.apis.maxmind.account_id,
            license_key: config.recon.apis.maxmind.license_key.as_ref().map(|s| s.expose_secret().to_string()),
            edition_ids: config.recon.apis.maxmind.edition_ids.clone(),
            data_dir: shellexpand::tilde(&config.recon.apis.maxmind.data_dir)
                .into_owned()
                .into(),
            auto_update: config.recon.apis.maxmind.auto_update,
        })
    } else {
        None
    };
    let virustotal_key = config.recon.apis.virustotal.api_key.as_ref().map(|s| s.expose_secret().to_string());
    let alienvault_key = config.recon.apis.alienvault.api_key.as_ref().map(|s| s.expose_secret().to_string());
    let shodan_key = config.recon.apis.shodan.api_key.as_ref().map(|s| s.expose_secret().to_string());
    let wayback_key = config.recon.apis.wayback_machine.api_key.as_ref().map(|s| s.expose_secret().to_string());
    let ip_for_threat = resolved_ip.clone();
    let ip_for_ssl = resolved_ip.clone();
    let ip_for_geo = resolved_ip.clone();
    let ip_for_rdns = resolved_ip.clone();
    let domain_for_whois = domain.clone();
    let domain_for_sub = domain.clone();
    let domain_for_dns = domain.clone();
    let domain_for_wayback = domain.clone();
    let domain_for_cloud = domain.clone();

    let (
        reverse_dns_result,
        geolocation_result,
        threat_intel_result,
        ssl_result,
        whois_result,
        subdomain_result,
        dns_records_result,
        techdetect_result,
        js_result,
        wayback_result,
        cloud_result,
        content_result,
        cors_result,
        email_result,
    ) = tokio::join!(
        async {
            if !args.no_dns {
                if let Some(ref ip) = ip_for_rdns {
                    match reverse_dns::reverse_dns_lookup(ip).await {
                        Ok(v) => Some(v),
                        Err(e) => {
                            tracing::warn!("reverse DNS lookup failed: {}", e);
                            None
                        }
                    }
                } else { None }
            } else { None }
        },
        async {
            if !args.no_geo {
                if let Some(ref ip) = ip_for_geo {
                    match geolocation::geolocation_lookup_with_config(ip, ipapi_key, maxmind_settings).await {
                        Ok(v) => Some(v),
                        Err(e) => {
                            tracing::warn!("geolocation lookup failed: {}", e);
                            None
                        }
                    }
                } else { None }
            } else { None }
        },
        async {
            if !args.no_threat {
                if let Some(ref ip) = ip_for_threat {
                    let is_ip = ip.parse::<std::net::IpAddr>().is_ok();
                    match threatintel::check_threat_intel(ip, is_ip, virustotal_key, alienvault_key, shodan_key).await {
                        Ok(v) => Some(v),
                        Err(e) => {
                            tracing::warn!("threat intel lookup failed: {}", e);
                            None
                        }
                    }
                } else { None }
            } else { None }
        },
        async {
            if !args.no_ssl {
                if let Some(ref host) = ip_for_ssl {
                    let port = if url.contains("https://") { 443 } else { 80 };
                    match ssl::analyze_ssl(host, port).await {
                        Ok(v) => Some(v),
                        Err(e) => {
                            tracing::warn!("SSL analysis failed: {}", e);
                            None
                        }
                    }
                } else { None }
            } else { None }
        },
        async {
            if !args.no_whois {
                if let Some(ref d) = domain_for_whois {
                    match whois::whois_lookup(d).await {
                        Ok(v) => Some(v),
                        Err(e) => {
                            tracing::warn!("whois lookup failed: {}", e);
                            None
                        }
                    }
                } else { None }
            } else { None }
        },
        async {
            if !args.no_subdomains {
                if let Some(ref d) = domain_for_sub {
                    match subdomain::enumerate_subdomains(d, concurrency).await {
                        Ok(v) => Some(v),
                        Err(e) => {
                            tracing::warn!("subdomain enumeration failed: {}", e);
                            None
                        }
                    }
                } else { None }
            } else { None }
        },
        async {
            if !args.no_dns_records {
                if let Some(ref d) = domain_for_dns {
                    match dns_records::enumerate_dns_records(d).await {
                        Ok(v) => Some(v),
                        Err(e) => {
                            tracing::warn!("DNS records enumeration failed: {}", e);
                            None
                        }
                    }
                } else { None }
            } else { None }
        },
        async {
            if !args.no_tech {
                match techdetect::detect_tech_stack(&url).await {
                    Ok(v) => Some(v),
                    Err(e) => {
                        tracing::warn!("tech detection failed: {}", e);
                        None
                    }
                }
            } else { None }
        },
        async {
            if !args.no_js {
                match js::analyze_js(&url).await {
                    Ok(v) => Some(v),
                    Err(e) => {
                        tracing::warn!("JS analysis failed: {}", e);
                        None
                    }
                }
            } else { None }
        },
        async {
            if !args.no_wayback {
                if let Some(ref d) = domain_for_wayback {
                    match wayback::get_wayback_snapshots(d, wayback_key, 100).await {
                        Ok(v) => Some(v),
                        Err(e) => {
                            tracing::warn!("wayback lookup failed: {}", e);
                            None
                        }
                    }
                } else { None }
            } else { None }
        },
        async {
            if !args.no_cloud {
                if let Some(ref d) = domain_for_cloud {
                    match cloud::scan_cloud(d, concurrency).await {
                        Ok(v) => Some(v),
                        Err(e) => {
                            tracing::warn!("cloud scan failed: {}", e);
                            None
                        }
                    }
                } else { None }
            } else { None }
        },
        async {
            if !args.no_content {
                match content::scan_content(&url, concurrency).await {
                    Ok(v) => Some(v),
                    Err(e) => {
                        tracing::warn!("content scan failed: {}", e);
                        None
                    }
                }
            } else { None }
        },
        async {
            if !args.no_cors {
                match cors::analyze_cors(&url).await {
                    Ok(v) => Some(v),
                    Err(e) => {
                        tracing::warn!("CORS analysis failed: {}", e);
                        None
                    }
                }
            } else { None }
        },
        async {
            if !args.no_email {
                match email::discover_contacts(&url).await {
                    Ok(v) => Some(v),
                    Err(e) => {
                        tracing::warn!("email discovery failed: {}", e);
                        None
                    }
                }
            } else { None }
        },
    );

    // Assign results
    recon.reverse_dns = reverse_dns_result;
    recon.geolocation = geolocation_result;
    recon.threat_intel = threat_intel_result;
    recon.ssl_analysis = ssl_result;
    recon.whois = whois_result;
    recon.subdomains = subdomain_result;
    recon.dns_records = dns_records_result;
    recon.js_analysis = js_result;
    recon.wayback = wayback_result;
    recon.cloud = cloud_result;
    recon.content = content_result;
    recon.cors = cors_result;
    recon.email_discovery = email_result;

    // Phase 2: CVE mapping depends on techdetect result
    if !args.no_cve {
        if let Some(ref tech) = techdetect_result {
            if let Ok(cve) = cve::map_cves(&tech.tech_stack, None).await {
                recon.cve_mapping = Some(cve);
            }
        }
    }
    recon.tech_stack = techdetect_result.map(|t| t.tech_stack);

    if verbose {
        eprintln!("Recon complete");
    }

    Ok(recon)
}

fn print_recon_results_string(recon: &FullReconResult) -> String {
    let mut s = String::new();

    if let Some(ref ip) = recon.ip_address {
        s.push_str(&format!("ip: {}\n", ip));
    }
    if let Some(ref domain) = recon.domain {
        s.push_str(&format!("domain: {}\n", domain));
    }
    if let Some(ref geo) = recon.geolocation {
        if let (Some(country), Some(city)) = (&geo.country, &geo.city) {
            if !country.is_empty() || !city.is_empty() {
                s.push_str(&format!("geo: {}, {}\n", country, city));
            }
        }
    }
    if let Some(ref tech) = recon.tech_stack {
        if !tech.frameworks.is_empty() || !tech.servers.is_empty() || !tech.cms.is_empty() {
            s.push_str("tech\n");
            if !tech.frameworks.is_empty() {
                s.push_str(&format!("\tframeworks: {}\n", tech.frameworks.join(", ")));
            }
            if !tech.servers.is_empty() {
                s.push_str(&format!("\tservers: {}\n", tech.servers.join(", ")));
            }
            if !tech.cms.is_empty() {
                s.push_str(&format!("\tcms: {}\n", tech.cms.join(", ")));
            }
            if !tech.languages.is_empty() {
                s.push_str(&format!("\tlanguages: {}\n", tech.languages.join(", ")));
            }
            if !tech.cdns.is_empty() {
                s.push_str(&format!("\tcdns: {}\n", tech.cdns.join(", ")));
            }
        }
    }
    if let Some(ref subdomains) = recon.subdomains {
        if !subdomains.subdomains.is_empty() {
            s.push_str("subdomains\n");
            for sub in &subdomains.subdomains {
                s.push_str(&format!(
                    "\t{} ({})\n",
                    sub.subdomain,
                    sub.ip_addresses.join(", ")
                ));
            }
        }
    }
    if let Some(ref content) = recon.content {
        if !content.sensitive_files.is_empty() {
            s.push_str("sensitive\n");
            for file in &content.sensitive_files {
                s.push_str(&format!(
                    "\t[{}] {}\n",
                    file.severity.to_uppercase(),
                    file.url
                ));
            }
        }
    }
    if let Some(ref cve) = recon.cve_mapping {
        if cve.total_critical > 0 || cve.total_high > 0 || cve.total_medium > 0 {
            s.push_str("vulnerabilities\n");
            s.push_str(&format!(
                "\t{} critical, {} high, {} medium\n",
                cve.total_critical, cve.total_high, cve.total_medium
            ));
            for vuln in &cve.vulnerabilities {
                s.push_str(&format!(
                    "\t[{}] {}\n",
                    vuln.severity.to_uppercase(),
                    vuln.cve_id
                ));
            }
        }
    }
    if let Some(ref ssl) = recon.ssl_analysis {
        s.push_str("ssl\n");
        if let Some(ref cert) = ssl.certificate {
            s.push_str(&format!("\tsubject: {}\n", cert.subject));
            s.push_str(&format!("\tissuer: {}\n", cert.issuer));
            s.push_str(&format!("\texpires: {}\n", cert.valid_until));
        }
        if !ssl.issues.is_empty() {
            for issue in &ssl.issues {
                s.push_str(&format!(
                    "\t[{}] {}\n",
                    issue.severity.to_uppercase(),
                    issue.description
                ));
            }
        }
    }
    if let Some(ref wayback) = recon.wayback {
        if !wayback.snapshots.is_empty() {
            s.push_str("wayback\n");
            for snap in wayback.snapshots.iter().take(10) {
                s.push_str(&format!("\t{} {}\n", snap.timestamp, snap.url));
            }
        }
    }
    if let Some(ref js) = recon.js_analysis {
        if !js.extracted_endpoints.is_empty() {
            s.push_str("js endpoints\n");
            for ep in js.extracted_endpoints.iter().take(10) {
                s.push_str(&format!("\t{}\n", ep));
            }
        }
    }
    if let Some(ref cors) = recon.cors {
        if !cors.findings.is_empty() {
            s.push_str("cors\n");
            for finding in &cors.findings {
                if finding.allows_origin {
                    s.push_str(&format!("\t[+] {}\n", finding.origin));
                }
            }
        }
    }
    if let Some(ref email) = recon.email_discovery {
        if !email.emails.is_empty() {
            s.push_str("email\n");
            for e in &email.emails {
                s.push_str(&format!("\t{}\n", e.email));
            }
        }
    }
    if let Some(ref intel) = recon.threat_intel {
        s.push_str("threat\n");
        if let Some(ref ip_rep) = intel.ip_reputation {
            s.push_str(&format!(
                "\tip-reputation: {} ({})\n",
                ip_rep.score, ip_rep.category
            ));
        }
        if let Some(ref dom_rep) = intel.domain_reputation {
            s.push_str(&format!(
                "\tdomain-reputation: {} ({})\n",
                dom_rep.score, dom_rep.category
            ));
        }
    }

    s
}
