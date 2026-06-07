use crate::cli::ReconArgs;
use crate::config::SlapperConfig;
use crate::error::Result;
#[cfg(feature = "cloud")]
use crate::recon::cloud;
use crate::recon::{
    content, cors, cve, dns_records, email, geolocation, js, reverse_dns, secrets, ssl, subdomain,
    takeover, techdetect, threatintel, wayback, whois, FullReconResult,
};
use crate::types::SensitiveString;
use crate::utils::sanitize_for_logging;
use parking_lot::Mutex;
use reqwest::Url;
use std::net::IpAddr;
use std::sync::Arc;
use url::Host;

enum ReconStep<T> {
    Skipped,
    Completed(T),
    Failed,
}

impl<T> ReconStep<T> {
    fn into_option(self) -> Option<T> {
        match self {
            Self::Completed(value) => Some(value),
            Self::Skipped | Self::Failed => None,
        }
    }

    fn is_failed(&self) -> bool {
        matches!(self, Self::Failed)
    }
}

/// Resolves the target domain to an IP address.
///
/// Strips protocol prefixes, extracts the domain, and performs DNS resolution
/// if the target is not already an IP address.
async fn resolve_target(
    target: &str,
    verbose: bool,
) -> (String, Option<String>, Option<String>, Option<u16>) {
    let looks_like_ipv6 = target
        .parse::<IpAddr>()
        .map(|ip| ip.is_ipv6())
        .unwrap_or(false);
    let url = if target.starts_with("http://") || target.starts_with("https://") {
        target.to_string()
    } else if looks_like_ipv6 {
        format!("https://[{}]", target)
    } else {
        format!("https://{}", target)
    };

    let parsed = Url::parse(&url).ok();
    if parsed.is_none() {
        tracing::warn!("failed to parse target as URL: {}", sanitize_for_logging(target));
    }
    let host = parsed.as_ref().and_then(|u| {
        u.host().map(|h| match h {
            Host::Domain(d) => d.to_string(),
            Host::Ipv4(ip) => ip.to_string(),
            Host::Ipv6(ip) => ip.to_string(),
        })
    });
    let port = parsed.as_ref().and_then(Url::port_or_known_default);

    let resolved_ip = if let Some(ref host) = host {
        if host.parse::<IpAddr>().is_ok() {
            Some(host.clone())
        } else {
            match reverse_dns::resolve_domain(host).await {
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

    (url, host, resolved_ip, port)
}

/// Performs reverse DNS lookup for the given IP address.
///
/// Returns `None` if `no_dns` is true, no IP is provided, or the lookup fails.
async fn run_reverse_dns(
    ip: Option<&String>,
    no_dns: bool,
) -> ReconStep<reverse_dns::ReverseDnsResult> {
    if no_dns {
        return ReconStep::Skipped;
    }
    let Some(ip) = ip else {
        return ReconStep::Skipped;
    };
    match reverse_dns::reverse_dns_lookup(ip).await {
        Ok(v) => ReconStep::Completed(v),
        Err(e) => {
            tracing::warn!("reverse DNS lookup failed: {}", e);
            ReconStep::Failed
        }
    }
}

/// Performs IP geolocation lookup.
///
/// Returns `None` if `no_geo` is true, no IP is provided, or the lookup fails.
async fn run_geo_lookup(
    ip: Option<&String>,
    no_geo: bool,
    ipapi_key: Option<&SensitiveString>,
    maxmind_settings: Option<geolocation::MaxMindSettings>,
) -> ReconStep<geolocation::GeoLocation> {
    if no_geo {
        return ReconStep::Skipped;
    }
    let Some(ip) = ip else {
        return ReconStep::Skipped;
    };
    match geolocation::geolocation_lookup_with_config(ip, ipapi_key, maxmind_settings).await {
        Ok(v) => ReconStep::Completed(v),
        Err(e) => {
            tracing::warn!("geolocation lookup failed: {}", e);
            ReconStep::Failed
        }
    }
}

/// Performs threat intelligence lookup for an IP address.
///
/// Returns `None` if `no_threat` is true, no IP is provided, or the lookup fails.
async fn run_threat_intel(
    target: Option<&String>,
    no_threat: bool,
    virustotal_key: Option<&SensitiveString>,
    alienvault_key: Option<&SensitiveString>,
    shodan_key: Option<&SensitiveString>,
) -> ReconStep<threatintel::ThreatIntel> {
    if no_threat {
        return ReconStep::Skipped;
    }
    let Some(target) = target else {
        return ReconStep::Skipped;
    };
    let is_ip = target.parse::<std::net::IpAddr>().is_ok();
    match threatintel::check_threat_intel(target, is_ip, virustotal_key, alienvault_key, shodan_key)
        .await
    {
        Ok(v) => ReconStep::Completed(v),
        Err(e) => {
            tracing::warn!("threat intel lookup failed: {}", e);
            ReconStep::Failed
        }
    }
}

/// Performs SSL/TLS certificate analysis.
///
/// Returns `None` if `no_ssl` is true, no host is provided, or the analysis fails.
async fn run_ssl_recon(
    host: Option<&String>,
    port: Option<u16>,
    no_ssl: bool,
) -> ReconStep<ssl::SslAnalysis> {
    if no_ssl {
        return ReconStep::Skipped;
    }
    let Some(host) = host else {
        return ReconStep::Skipped;
    };
    let port = port.unwrap_or(443);
    match ssl::analyze_ssl(host, port).await {
        Ok(v) => ReconStep::Completed(v),
        Err(e) => {
            tracing::warn!("SSL analysis failed: {}", e);
            ReconStep::Failed
        }
    }
}

/// Performs WHOIS lookup for a domain.
///
/// Returns `None` if `no_whois` is true, no domain is provided, or the lookup fails.
async fn run_whois_lookup(
    domain: Option<&String>,
    no_whois: bool,
) -> ReconStep<whois::WhoisResult> {
    if no_whois {
        return ReconStep::Skipped;
    }
    let Some(domain) = domain else {
        return ReconStep::Skipped;
    };
    match whois::whois_lookup(domain).await {
        Ok(v) => ReconStep::Completed(v),
        Err(e) => {
            tracing::warn!("whois lookup failed: {}", e);
            ReconStep::Failed
        }
    }
}

/// Performs subdomain enumeration.
///
/// Returns `None` if `no_subdomains` is true, no domain is provided, or enumeration fails.
async fn run_subdomain_enum(
    domain: Option<&String>,
    concurrency: usize,
    no_subdomains: bool,
) -> ReconStep<subdomain::SubdomainResult> {
    if no_subdomains {
        return ReconStep::Skipped;
    }
    let Some(domain) = domain else {
        return ReconStep::Skipped;
    };
    match subdomain::enumerate_subdomains(domain, concurrency).await {
        Ok(v) => ReconStep::Completed(v),
        Err(e) => {
            tracing::warn!("subdomain enumeration failed: {}", e);
            ReconStep::Failed
        }
    }
}

/// Enumerates DNS records for a domain.
///
/// Returns `None` if `no_dns_records` is true, no domain is provided, or enumeration fails.
async fn run_dns_records(
    domain: Option<&String>,
    no_dns_records: bool,
) -> ReconStep<dns_records::DnsRecords> {
    if no_dns_records {
        return ReconStep::Skipped;
    }
    let Some(domain) = domain else {
        return ReconStep::Skipped;
    };
    match dns_records::enumerate_dns_records(domain).await {
        Ok(v) => ReconStep::Completed(v),
        Err(e) => {
            tracing::warn!("DNS records enumeration failed: {}", e);
            ReconStep::Failed
        }
    }
}

/// Detects the technology stack used by a web application.
///
/// Returns `None` if `no_tech` is true or detection fails.
async fn run_tech_detection(
    url: &str,
    no_tech: bool,
) -> ReconStep<techdetect::TechDetectionResult> {
    if no_tech {
        return ReconStep::Skipped;
    }
    match techdetect::detect_tech_stack(url).await {
        Ok(v) => ReconStep::Completed(v),
        Err(e) => {
            tracing::warn!("tech detection failed: {}", e);
            ReconStep::Failed
        }
    }
}

/// Analyzes JavaScript files for endpoints and secrets.
///
/// Returns `None` if `no_js` is true or analysis fails.
async fn run_js_analysis(url: &str, no_js: bool) -> ReconStep<js::JsAnalysis> {
    if no_js {
        return ReconStep::Skipped;
    }
    match js::analyze_js(url).await {
        Ok(v) => ReconStep::Completed(v),
        Err(e) => {
            tracing::warn!("JS analysis failed: {}", e);
            ReconStep::Failed
        }
    }
}

/// Queries the Wayback Machine for historical snapshots.
///
/// Returns `None` if `no_wayback` is true, no domain is provided, or the lookup fails.
async fn run_wayback_check(
    domain: Option<&String>,
    no_wayback: bool,
    wayback_key: Option<&SensitiveString>,
) -> ReconStep<wayback::WaybackResult> {
    if no_wayback {
        return ReconStep::Skipped;
    }
    let Some(domain) = domain else {
        return ReconStep::Skipped;
    };
    match wayback::get_wayback_snapshots(domain, wayback_key, 100).await {
        Ok(v) => ReconStep::Completed(v),
        Err(e) => {
            tracing::warn!("wayback lookup failed: {}", e);
            ReconStep::Failed
        }
    }
}

/// Scans for cloud infrastructure misconfigurations.
///
/// Returns `None` if `no_cloud` is true, no domain is provided, or the scan fails.
#[cfg(feature = "cloud")]
async fn run_cloud_detection(
    domain: Option<&String>,
    concurrency: usize,
    no_cloud: bool,
) -> ReconStep<cloud::CloudDiscovery> {
    if no_cloud {
        return ReconStep::Skipped;
    }
    let Some(domain) = domain else {
        return ReconStep::Skipped;
    };
    match cloud::scan_cloud(domain, concurrency).await {
        Ok(v) => ReconStep::Completed(v),
        Err(e) => {
            tracing::warn!("cloud scan failed: {}", e);
            ReconStep::Failed
        }
    }
}

/// Scans web content for sensitive files and directories.
///
/// Returns `None` if `no_content` is true or the scan fails.
async fn run_content_analysis(
    url: &str,
    concurrency: usize,
    no_content: bool,
) -> ReconStep<content::ContentDiscovery> {
    if no_content {
        return ReconStep::Skipped;
    }
    match content::scan_content(url, concurrency).await {
        Ok(v) => ReconStep::Completed(v),
        Err(e) => {
            tracing::warn!("content scan failed: {}", e);
            ReconStep::Failed
        }
    }
}

/// Analyzes CORS configuration for misconfigurations.
///
/// Returns `None` if `no_cors` is true or the analysis fails.
async fn run_cors_check(url: &str, no_cors: bool) -> ReconStep<cors::CorsAnalysis> {
    if no_cors {
        return ReconStep::Skipped;
    }
    match cors::analyze_cors(url).await {
        Ok(v) => ReconStep::Completed(v),
        Err(e) => {
            tracing::warn!("CORS analysis failed: {}", e);
            ReconStep::Failed
        }
    }
}

/// Discovers email addresses associated with a target.
///
/// Returns `None` if `no_email` is true or discovery fails.
async fn run_email_security(url: &str, no_email: bool) -> ReconStep<email::EmailDiscovery> {
    if no_email {
        return ReconStep::Skipped;
    }
    match email::discover_contacts(url).await {
        Ok(v) => ReconStep::Completed(v),
        Err(e) => {
            tracing::warn!("email discovery failed: {}", e);
            ReconStep::Failed
        }
    }
}

/// Checks for subdomain takeover vulnerabilities.
///
/// This runs sequentially after subdomain enumeration since it depends on those results.
/// Returns `None` if `no_takeover` is true, no subdomains were found, or no vulnerabilities detected.
async fn run_takeover_check(
    subdomain_result: Option<&subdomain::SubdomainResult>,
    no_takeover: bool,
) -> Option<Vec<takeover::TakeoverResult>> {
    if no_takeover {
        return None;
    }
    let sub_result = subdomain_result?;
    if sub_result.subdomains.is_empty() {
        return None;
    }
    let subdomains: Vec<String> = sub_result
        .subdomains
        .iter()
        .map(|s| s.subdomain.clone())
        .collect();
    match takeover::detect_takeovers(&subdomains, None, 10).await {
        Ok(results) if !results.is_empty() => {
            let vulnerable: Vec<_> = results
                .iter()
                .filter(|r| r.status == takeover::TakeoverStatus::Vulnerable)
                .cloned()
                .collect();
            if vulnerable.is_empty() {
                None
            } else {
                Some(vulnerable)
            }
        }
        _ => None,
    }
}

/// Maps detected technologies to known CVEs.
///
/// Returns `None` if `no_cve` is true, no tech stack was detected, or mapping fails.
async fn run_cve_check(
    tech_result: Option<&techdetect::TechDetectionResult>,
    no_cve: bool,
    nvd_api_key: Option<&str>,
) -> Option<cve::CveMapping> {
    if no_cve {
        return None;
    }
    let tech = tech_result?;
    let nvd_api_key = nvd_api_key.map(ToString::to_string);
    match cve::map_cves(&tech.tech_stack, nvd_api_key).await {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("CVE mapping failed: {}", e);
            None
        }
    }
}

/// Scans content for exposed secrets and sensitive information.
///
/// Returns `None` if content discovery failed or no secrets were found.
async fn run_secrets_check(
    content_result: Option<&content::ContentDiscovery>,
) -> Option<Vec<secrets::SecretFinding>> {
    let content = content_result?;
    if content.sensitive_files.is_empty() {
        return None;
    }
    let mut all_findings = Vec::new();
    for sensitive_file in &content.sensitive_files {
        match secrets::SecretScanner::new().scan_file(&sensitive_file.url) {
            Ok(file_findings) => all_findings.extend(file_findings),
            Err(e) => {
                tracing::debug!("failed to scan file for secrets {}: {}", sensitive_file.url, e);
            }
        }
    }
    if all_findings.is_empty() {
        None
    } else {
        Some(all_findings)
    }
}

fn nvd_api_key_from_config(config: &SlapperConfig) -> Option<String> {
    config
        .recon
        .apis
        .nvd
        .api_key
        .as_ref()
        .map(|k| k.expose_secret().to_string())
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
        .unwrap_or(config.recon.dns_concurrency)
        .max(1);

    if verbose {
        eprintln!("Starting recon on {}", sanitize_for_logging(target));
    }

    set_stage(&stage, "resolving");

    let (url, domain, resolved_ip, port) = resolve_target(target, verbose).await;

    let mut recon = FullReconResult::new(target);

    if let Some(ref d) = domain {
        recon.domain = Some(d.clone());
    }

    if let Some(ref ip) = resolved_ip {
        recon.ip_address = Some(ip.clone());
    }

    set_stage(&stage, "recon (parallel)");

    let ipapi_key = config.recon.apis.ipapi.api_key.as_ref();
    let maxmind_settings = if config.recon.apis.maxmind.enabled {
        Some(geolocation::MaxMindSettings {
            account_id: config.recon.apis.maxmind.account_id,
            license_key: config.recon.apis.maxmind.license_key.clone(),
            edition_ids: config.recon.apis.maxmind.edition_ids.clone(),
            data_dir: config.recon.apis.maxmind.data_dir.clone(),
            auto_update: config.recon.apis.maxmind.auto_update,
        })
    } else {
        None
    };
    let virustotal_key = config.recon.apis.virustotal.api_key.as_ref();
    let alienvault_key = config.recon.apis.alienvault.api_key.as_ref();
    let shodan_key = config.recon.apis.shodan.api_key.as_ref();
    let wayback_key = config.recon.apis.wayback_machine.api_key.as_ref();
    let nvd_api_key = nvd_api_key_from_config(config);

    let threat_target = domain.as_ref().or(resolved_ip.as_ref());
    let ssl_host = domain.as_ref().or(resolved_ip.as_ref());

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
        content_result,
        cors_result,
        email_result,
    ) = tokio::join!(
        run_reverse_dns(resolved_ip.as_ref(), args.no_dns),
        run_geo_lookup(
            resolved_ip.as_ref(),
            args.no_geo,
            ipapi_key,
            maxmind_settings
        ),
        run_threat_intel(
            threat_target,
            args.no_threat,
            virustotal_key,
            alienvault_key,
            shodan_key
        ),
        run_ssl_recon(ssl_host, port, args.no_ssl),
        run_whois_lookup(domain.as_ref(), args.no_whois),
        run_subdomain_enum(domain.as_ref(), concurrency, args.no_subdomains),
        run_dns_records(domain.as_ref(), args.no_dns_records),
        run_tech_detection(&url, args.no_tech),
        run_js_analysis(&url, args.no_js),
        run_wayback_check(domain.as_ref(), args.no_wayback, wayback_key),
        run_content_analysis(&url, concurrency, args.no_content),
        run_cors_check(&url, args.no_cors),
        run_email_security(&url, args.no_email),
    );

    let reverse_dns_failed = reverse_dns_result.is_failed();
    let geo_failed = geolocation_result.is_failed();
    let threat_failed = threat_intel_result.is_failed();
    let ssl_failed = ssl_result.is_failed();
    let whois_failed = whois_result.is_failed();
    let subdomains_failed = subdomain_result.is_failed();
    let dns_records_failed = dns_records_result.is_failed();
    let js_failed = js_result.is_failed();
    let wayback_failed = wayback_result.is_failed();
    let content_failed = content_result.is_failed();
    let cors_failed = cors_result.is_failed();
    let email_failed = email_result.is_failed();

    #[cfg(feature = "cloud")]
    let (cloud_result, cloud_failed) = {
        let result = run_cloud_detection(domain.as_ref(), concurrency, args.no_cloud).await;
        let failed = result.is_failed();
        (result, failed)
    };

    let subdomain_result = subdomain_result.into_option();
    let takeover_result = run_takeover_check(subdomain_result.as_ref(), args.no_takeover).await;
    let takeover_failed = !args.no_takeover
        && matches!(subdomain_result.as_ref(), Some(s) if !s.subdomains.is_empty())
        && takeover_result.is_none();

    recon.reverse_dns = reverse_dns_result.into_option();
    recon.geolocation = geolocation_result.into_option();
    recon.threat_intel = threat_intel_result.into_option();
    recon.ssl_analysis = ssl_result.into_option();
    recon.whois = whois_result.into_option();
    recon.subdomains = subdomain_result;
    recon.dns_records = dns_records_result.into_option();
    recon.js_analysis = js_result.into_option();
    recon.wayback = wayback_result.into_option();
    #[cfg(feature = "cloud")]
    {
        recon.cloud = cloud_result.into_option();
    }
    recon.cors = cors_result.into_option();
    recon.email_discovery = email_result.into_option();
    recon.takeover = takeover_result;

    let content_result_opt = content_result.into_option();
    recon.secrets = run_secrets_check(content_result_opt.as_ref()).await;
    recon.content = content_result_opt;

    if reverse_dns_failed {
        recon.reverse_dns_error =
            Some("Reverse DNS lookup failed (see logs for the underlying error)".to_string());
    }
    if geo_failed {
        recon.geoip_error =
            Some("Geolocation lookup failed (see logs for the underlying error)".to_string());
    }
    if threat_failed {
        recon.threat_intel_error =
            Some("Threat intel lookup failed (see logs for the underlying error)".to_string());
    }
    if ssl_failed {
        recon.ssl_error =
            Some("SSL analysis failed (see logs for the underlying error)".to_string());
    }
    if whois_failed {
        recon.whois_error =
            Some("WHOIS lookup failed (see logs for the underlying error)".to_string());
    }
    if subdomains_failed {
        recon.subdomains_error =
            Some("Subdomain enumeration failed (see logs for the underlying error)".to_string());
    }
    if dns_records_failed {
        recon.dns_records_error =
            Some("DNS records enumeration failed (see logs for the underlying error)".to_string());
    }
    if js_failed {
        recon.js_error = Some("JS analysis failed (see logs for the underlying error)".to_string());
    }
    if wayback_failed {
        recon.wayback_error =
            Some("Wayback lookup failed (see logs for the underlying error)".to_string());
    }
    #[cfg(feature = "cloud")]
    if cloud_failed {
        recon.cloud_error =
            Some("Cloud scan failed (see logs for the underlying error)".to_string());
    }
    if content_failed {
        recon.content_error =
            Some("Content discovery failed (see logs for the underlying error)".to_string());
    }
    if cors_failed {
        recon.cors_error =
            Some("CORS analysis failed (see logs for the underlying error)".to_string());
    }
    if email_failed {
        recon.email_error =
            Some("Email discovery failed (see logs for the underlying error)".to_string());
    }
    if takeover_failed {
        recon.takeover_error = Some(
            "Takeover check failed on discovered subdomains (see logs for the underlying error)"
                .to_string(),
        );
    }

    let techdetect_failed = techdetect_result.is_failed();
    let techdetect_result = techdetect_result.into_option();

    recon.cve_mapping = run_cve_check(
        techdetect_result.as_ref(),
        args.no_cve,
        nvd_api_key.as_deref(),
    )
    .await;
    if recon.cve_mapping.is_none() && !args.no_cve && techdetect_result.is_some() {
        recon.cve_error =
            Some("CVE mapping failed (see logs for the underlying error)".to_string());
    }

    recon.tech_stack = techdetect_result.map(|t| t.tech_stack);
    if techdetect_failed {
        recon.tech_error = Some("Technology detection failed".to_string());
    }

    if verbose {
        eprintln!("Recon complete");
    }

    Ok(recon)
}

pub fn set_stage(stage: &Arc<Mutex<String>>, text: &str) {
    let mut s = stage.lock();
    *s = text.to_string();
}

pub fn print_recon_results_string(recon: &FullReconResult) -> String {
    let mut s = String::new();

    if let Some(ref ip) = recon.ip_address {
        s.push_str(&format!("ip: {}\n", ip));
    }
    if let Some(ref domain) = recon.domain {
        s.push_str(&format!("domain: {}\n", domain));
    }
    if let Some(ref geo) = recon.geolocation {
        let country = geo.country.as_deref().unwrap_or("");
        let city = geo.city.as_deref().unwrap_or("");
        if !country.is_empty() || !city.is_empty() {
            if !country.is_empty() && !city.is_empty() {
                s.push_str(&format!("geo: {}, {}\n", country, city));
            } else if !country.is_empty() {
                s.push_str(&format!("geo: {}\n", country));
            } else {
                s.push_str(&format!("geo: {}\n", city));
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
    if let Some(ref dns) = recon.dns_records {
        if !dns.a.is_empty() || !dns.aaaa.is_empty() || !dns.mx.is_empty()
            || !dns.txt.is_empty() || !dns.ns.is_empty() || dns.soa.is_some()
        {
            s.push_str("dns\n");
            if !dns.a.is_empty() {
                s.push_str(&format!("\ta: {}\n", dns.a.join(", ")));
            }
            if !dns.aaaa.is_empty() {
                s.push_str(&format!("\taaaa: {}\n", dns.aaaa.join(", ")));
            }
            if !dns.ns.is_empty() {
                s.push_str(&format!("\tns: {}\n", dns.ns.join(", ")));
            }
            if !dns.mx.is_empty() {
                let mx_str: Vec<String> = dns
                    .mx
                    .iter()
                    .map(|m| format!("{} {}", m.preference, m.exchange))
                    .collect();
                s.push_str(&format!("\tmx: {}\n", mx_str.join(", ")));
            }
            if !dns.txt.is_empty() {
                for txt in &dns.txt {
                    s.push_str(&format!("\ttxt: {}\n", txt));
                }
            }
            if let Some(ref soa) = dns.soa {
                s.push_str(&format!("\tsoa: {} {} {}\n", soa.mname, soa.rname, soa.serial));
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
        if ssl.certificate.is_some() || !ssl.issues.is_empty() {
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
                if finding.is_vulnerable {
                    s.push_str(&format!(
                        "\t[VULN] {} ({})\n",
                        finding.origin,
                        finding.vulnerability_type.as_deref().unwrap_or("unknown")
                    ));
                } else if finding.allows_origin {
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
        if intel.ip_reputation.is_some() || intel.domain_reputation.is_some() {
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
    }
    if let Some(ref takeovers) = recon.takeover {
        if !takeovers.is_empty() {
            s.push_str("takeover\n");
            for t in takeovers {
                s.push_str(&format!(
                    "\t[{}] {} ({})\n",
                    t.severity.as_str().to_uppercase(),
                    t.target.subdomain,
                    t.service.as_deref().unwrap_or("unknown")
                ));
            }
        }
    }

    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SensitiveString;

    #[tokio::test]
    async fn test_resolve_target_http_prefix() {
        let (url, domain, _, port) = resolve_target("http://example.com/path", false).await;
        assert_eq!(url, "http://example.com/path");
        assert_eq!(domain, Some("example.com".to_string()));
        assert_eq!(port, Some(80));
    }

    #[tokio::test]
    async fn test_resolve_target_https_prefix() {
        let (url, domain, _, port) = resolve_target("https://example.com/page?q=1", false).await;
        assert_eq!(url, "https://example.com/page?q=1");
        assert_eq!(domain, Some("example.com".to_string()));
        assert_eq!(port, Some(443));
    }

    #[tokio::test]
    async fn test_resolve_target_no_prefix() {
        let (url, domain, resolved_ip, port) = resolve_target("example.com", false).await;
        assert_eq!(url, "https://example.com");
        assert_eq!(domain, Some("example.com".to_string()));
        assert_eq!(port, Some(443));
        let _ = resolved_ip;
    }

    #[tokio::test]
    async fn test_resolve_target_ip_address() {
        let (url, domain, resolved_ip, _) = resolve_target("8.8.8.8", false).await;
        assert_eq!(url, "https://8.8.8.8");
        assert_eq!(domain, Some("8.8.8.8".to_string()));
        assert_eq!(resolved_ip, Some("8.8.8.8".to_string()));
    }

    #[tokio::test]
    async fn test_resolve_target_ipv6() {
        let (url, domain, resolved_ip, _) = resolve_target("::1", false).await;
        assert_eq!(url, "https://[::1]");
        assert_eq!(domain, Some("::1".to_string()));
        assert_eq!(resolved_ip, Some("::1".to_string()));
    }

    #[tokio::test]
    async fn test_resolve_target_with_port() {
        let (url, domain, _, port) = resolve_target("http://example.com:8080/admin", false).await;
        assert_eq!(url, "http://example.com:8080/admin");
        assert_eq!(domain, Some("example.com".to_string()));
        assert_eq!(port, Some(8080));
    }

    #[tokio::test]
    async fn test_resolve_target_strips_path_from_domain() {
        let (url, domain, _, _) = resolve_target("https://example.com/a/b/c", false).await;
        assert_eq!(url, "https://example.com/a/b/c");
        assert_eq!(domain, Some("example.com".to_string()));
    }

    #[test]
    fn test_full_recon_result_new() {
        use crate::recon::FullReconResult;
        let result = FullReconResult::new("example.com");
        assert_eq!(result.target, "example.com");
        assert!(result.domain.is_none());
        assert!(result.ip_address.is_none());
    }

    #[test]
    fn test_full_recon_result_default() {
        use crate::recon::FullReconResult;
        let result = FullReconResult::default();
        assert!(result.target.is_empty());
    }

    #[test]
    fn test_full_recon_result_serialization() {
        use crate::recon::FullReconResult;
        let result = FullReconResult {
            target: "example.com".to_string(),
            domain: Some("example.com".to_string()),
            ip_address: Some("93.184.216.34".to_string()),
            ..Default::default()
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("example.com"));
        assert!(json.contains("93.184.216.34"));
        let decoded: FullReconResult = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.target, "example.com");
    }

    #[test]
    fn test_full_recon_result_with_subdomains() {
        use crate::recon::subdomain::{SubdomainInfo, SubdomainResult};
        use crate::recon::FullReconResult;
        let result = FullReconResult {
            target: "example.com".to_string(),
            domain: Some("example.com".to_string()),
            subdomains: Some(SubdomainResult {
                domain: "example.com".to_string(),
                subdomains: vec![SubdomainInfo {
                    subdomain: "www".to_string(),
                    ip_addresses: vec!["93.184.216.34".to_string()],
                    has_mx: false,
                    has_cname: false,
                    has_txt: false,
                }],
                sources: vec!["crt.sh".to_string()],
            }),
            ..Default::default()
        };
        assert!(result.subdomains.is_some());
        let subs = result.subdomains.unwrap();
        assert_eq!(subs.subdomains.len(), 1);
        assert_eq!(subs.subdomains[0].subdomain, "www");
    }

    #[test]
    fn test_nvd_api_key_from_config() {
        let mut config = SlapperConfig::default();
        config.recon.apis.nvd.api_key = Some(SensitiveString::new("nvd-test-key".to_string()));
        let extracted = nvd_api_key_from_config(&config);
        assert_eq!(extracted, Some("nvd-test-key".to_string()));
    }
}
