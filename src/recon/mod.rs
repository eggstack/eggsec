#![allow(dead_code)]

pub mod techdetect;
pub mod reverse_dns;
pub mod geolocation;
pub mod whois;
pub mod subdomain;
pub mod ssl;
pub mod dns_records;
pub mod js;
pub mod wayback;
pub mod cloud;
pub mod content;
pub mod cors;
pub mod email;
pub mod cve;
pub mod threatintel;

pub use cloud::CloudDiscovery;
pub use threatintel::ThreatIntel;

use crate::cli::ReconArgs;
use crate::config::SlapperConfig;
use serde::{Deserialize, Serialize};
use anyhow::Result;
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
            let stage = self.stage.lock().unwrap();
            eprint!("\r{}{} {}", self.chars[self.idx], " ", stage);
            self.idx = (self.idx + 1) % self.chars.len();
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
        std::thread::spawn(move || {
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
        std::thread::sleep(std::time::Duration::from_millis(200));
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
        std::fs::write(output_file, &output)?;
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

pub async fn run_full_recon(args: &ReconArgs, config: &SlapperConfig, stage: Arc<Mutex<String>>, verbose: bool) -> Result<FullReconResult> {
    let target = &args.target;
    let concurrency = args.concurrency.unwrap_or(config.recon.dns_concurrency.max(10));
    
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
    
    let resolved_ip: Option<String> = if let Some(ref d) = domain {
        if d.parse::<std::net::IpAddr>().is_ok() {
            Some(d.clone())
        } else {
            match reverse_dns::resolve_domain(d).await {
                Ok(ips) if !ips.is_empty() => {
                    if verbose { eprintln!("Resolved to {}", ips[0]); }
                    Some(ips[0].clone())
                }
                _ => None,
            }
        }
    } else {
        None
    };
    
    if let Some(ref ip_addr) = resolved_ip {
        recon.ip_address = Some(ip_addr.clone());
        
        if !args.no_dns {
            set_stage(&stage, "reverse dns");
            if let Ok(rdns) = reverse_dns::reverse_dns_lookup(ip_addr).await {
                let rdns_info = rdns.hostname.clone();
                recon.reverse_dns = Some(rdns);
                if verbose { eprintln!("Reverse DNS: {:?}", rdns_info); }
            }
        }
        
        if !args.no_geo {
            set_stage(&stage, "geolocation");
            let ipapi_key = config.recon.apis.ipapi.api_key.clone();
            
            let maxmind_settings = if config.recon.apis.maxmind.enabled {
                Some(geolocation::MaxMindSettings {
                    account_id: config.recon.apis.maxmind.account_id,
                    license_key: config.recon.apis.maxmind.license_key.clone(),
                    edition_ids: config.recon.apis.maxmind.edition_ids.clone(),
                    data_dir: shellexpand::tilde(&config.recon.apis.maxmind.data_dir).into_owned().into(),
                    auto_update: config.recon.apis.maxmind.auto_update,
                })
            } else {
                None
            };

            match geolocation::geolocation_lookup_with_config(ip_addr, ipapi_key, maxmind_settings).await {
                Ok(geo) => {
                    let geo_info = format!("{} / {}", geo.country.as_deref().unwrap_or("?"), geo.city.as_deref().unwrap_or("?"));
                    recon.geolocation = Some(geo);
                    if verbose { eprintln!("Geo: {}", geo_info); }
                }
                Err(e) => {
                    recon.geoip_error = Some(format!("{}", e));
                }
            }
        }
        
        if !args.no_threat {
            set_stage(&stage, "threat intel");
            let is_ip = ip_addr.parse::<std::net::IpAddr>().is_ok();
            if let Ok(intel) = threatintel::check_threat_intel(
                ip_addr,
                is_ip,
                config.recon.apis.virustotal.api_key.clone(),
                config.recon.apis.alienvault.api_key.clone(),
                config.recon.apis.shodan.api_key.clone(),
            ).await {
                recon.threat_intel = Some(intel);
            }
        }
    }
    
    if !args.no_tech {
        set_stage(&stage, "tech detection");
        if let Ok(tech_result) = techdetect::detect_tech_stack(&url).await {
            recon.tech_stack = Some(tech_result.tech_stack.clone());
            if verbose { eprintln!("Tech: {:?}", tech_result.tech_stack.servers); }
            
            if !args.no_cve {
                if let Ok(cve) = cve::map_cves(&tech_result.tech_stack, None).await {
                    let cve_count = cve.vulnerabilities.len();
                    recon.cve_mapping = Some(cve);
                    if verbose { eprintln!("CVEs: {} found", cve_count); }
                }
            }
        }
    }
    
    if !args.no_whois {
        set_stage(&stage, "whois");
        if let Some(ref d) = domain {
            if let Ok(whois_result) = whois::whois_lookup(d).await {
                recon.whois = Some(whois_result);
                if verbose { eprintln!("WHOIS complete"); }
            }
        }
    }
    
    if !args.no_subdomains {
        set_stage(&stage, "subdomains");
        if let Some(ref d) = domain {
            if let Ok(subdomains) = subdomain::enumerate_subdomains(d, concurrency).await {
                let count = subdomains.subdomains.len();
                recon.subdomains = Some(subdomains);
                if verbose { eprintln!("Subdomains: {} found", count); }
            }
        }
    }
    
    if !args.no_ssl {
        set_stage(&stage, "ssl");
        if let Some(ref host) = resolved_ip {
            let port = if url.contains("https://") { 443 } else { 80 };
            if let Ok(ssl) = ssl::analyze_ssl(host, port).await {
                recon.ssl_analysis = Some(ssl);
                if verbose { eprintln!("SSL analyzed"); }
            }
        }
    }
    
    if !args.no_dns_records {
        if let Some(ref d) = domain {
            if let Ok(records) = dns_records::enumerate_dns_records(d).await {
                recon.dns_records = Some(records);
                if verbose { eprintln!("DNS records complete"); }
            }
        }
    }
    
    if !args.no_js {
        if let Ok(js) = js::analyze_js(&url).await {
            recon.js_analysis = Some(js);
        }
    }
    
    if !args.no_wayback {
        if let Some(ref d) = domain {
            if let Ok(wayback) = wayback::get_wayback_snapshots(d, config.recon.apis.wayback_machine.api_key.clone(), 100).await {
                recon.wayback = Some(wayback);
            }
        }
    }
    
    if !args.no_cloud {
        if let Some(ref d) = domain {
            if let Ok(cloud) = cloud::scan_cloud(d, concurrency).await {
                recon.cloud = Some(cloud);
            }
        }
    }
    
    if !args.no_content {
        if let Ok(content) = content::scan_content(&url, concurrency).await {
            recon.content = Some(content);
        }
    }
    
    if !args.no_cors {
        if let Ok(cors) = cors::analyze_cors(&url).await {
            recon.cors = Some(cors);
        }
    }
    
    if !args.no_email {
        if let Ok(email) = email::discover_contacts(&url).await {
            recon.email_discovery = Some(email);
        }
    }
    
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
                s.push_str(&format!("\t{} ({})\n", sub.subdomain, sub.ip_addresses.join(", ")));
            }
        }
    }
    if let Some(ref content) = recon.content {
        if !content.sensitive_files.is_empty() {
            s.push_str("sensitive\n");
            for file in &content.sensitive_files {
                s.push_str(&format!("\t[{}] {}\n", file.severity.to_uppercase(), file.url));
            }
        }
    }
    if let Some(ref cve) = recon.cve_mapping {
        if cve.total_critical > 0 || cve.total_high > 0 || cve.total_medium > 0 {
            s.push_str("vulnerabilities\n");
            s.push_str(&format!("\t{} critical, {} high, {} medium\n", cve.total_critical, cve.total_high, cve.total_medium));
            for vuln in &cve.vulnerabilities {
                s.push_str(&format!("\t[{}] {}\n", vuln.severity.to_uppercase(), vuln.cve_id));
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
                s.push_str(&format!("\t[{}] {}\n", issue.severity.to_uppercase(), issue.description));
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
            s.push_str(&format!("\tip-reputation: {} ({})\n", ip_rep.score, ip_rep.category));
        }
        if let Some(ref dom_rep) = intel.domain_reputation {
            s.push_str(&format!("\tdomain-reputation: {} ({})\n", dom_rep.score, dom_rep.category));
        }
    }
    
    s
}
