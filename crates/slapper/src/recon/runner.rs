use crate::cli::ReconArgs;
use crate::config::SlapperConfig;
use crate::error::Result;
use crate::recon::{cloud, content, cors, cve, dns_records, email, geolocation, js, reverse_dns, ssl, subdomain, takeover, techdetect, threatintel, wayback, whois, FullReconResult};
use crate::types::SensitiveString;
use std::sync::{Arc, Mutex};

/// Resolves the target domain to an IP address.
///
/// Strips protocol prefixes, extracts the domain, and performs DNS resolution
/// if the target is not already an IP address.
async fn resolve_target(target: &str, verbose: bool) -> (String, Option<String>, Option<String>) {
    let target_clean = if target.starts_with("http://") {
        target.strip_prefix("http://").unwrap_or(target)
    } else if target.starts_with("https://") {
        target.strip_prefix("https://").unwrap_or(target)
    } else {
        target
    };

    let domain = target_clean.split('/').next().map(|s| s.to_string());
    let url = if target.starts_with("http://") || target.starts_with("https://") {
        target.to_string()
    } else {
        format!("https://{}", target)
    };

    let resolved_ip = if let Some(ref d) = domain {
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

    (url, domain, resolved_ip)
}

/// Performs reverse DNS lookup for the given IP address.
///
/// Returns `None` if `no_dns` is true, no IP is provided, or the lookup fails.
async fn run_reverse_dns(
    ip: Option<&String>,
    no_dns: bool,
) -> Option<reverse_dns::ReverseDnsResult> {
    if no_dns {
        return None;
    }
    let ip = ip?;
    match reverse_dns::reverse_dns_lookup(ip).await {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("reverse DNS lookup failed: {}", e);
            None
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
) -> Option<geolocation::GeoLocation> {
    if no_geo {
        return None;
    }
    let ip = ip?;
    match geolocation::geolocation_lookup_with_config(ip, ipapi_key, maxmind_settings).await {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("geolocation lookup failed: {}", e);
            None
        }
    }
}

/// Performs threat intelligence lookup for an IP address.
///
/// Returns `None` if `no_threat` is true, no IP is provided, or the lookup fails.
async fn run_threat_intel(
    ip: Option<&String>,
    no_threat: bool,
    virustotal_key: Option<&SensitiveString>,
    alienvault_key: Option<&SensitiveString>,
    shodan_key: Option<&SensitiveString>,
) -> Option<threatintel::ThreatIntel> {
    if no_threat {
        return None;
    }
    let ip = ip?;
    let is_ip = ip.parse::<std::net::IpAddr>().is_ok();
    match threatintel::check_threat_intel(ip, is_ip, virustotal_key, alienvault_key, shodan_key).await {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("threat intel lookup failed: {}", e);
            None
        }
    }
}

/// Performs SSL/TLS certificate analysis.
///
/// Returns `None` if `no_ssl` is true, no host is provided, or the analysis fails.
async fn run_ssl_recon(
    host: Option<&String>,
    url: &str,
    no_ssl: bool,
) -> Option<ssl::SslAnalysis> {
    if no_ssl {
        return None;
    }
    let host = host?;
    let port = if url.contains("https://") { 443 } else { 80 };
    match ssl::analyze_ssl(host, port).await {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("SSL analysis failed: {}", e);
            None
        }
    }
}

/// Performs WHOIS lookup for a domain.
///
/// Returns `None` if `no_whois` is true, no domain is provided, or the lookup fails.
async fn run_whois_lookup(
    domain: Option<&String>,
    no_whois: bool,
) -> Option<whois::WhoisResult> {
    if no_whois {
        return None;
    }
    let domain = domain?;
    match whois::whois_lookup(domain).await {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("whois lookup failed: {}", e);
            None
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
) -> Option<subdomain::SubdomainResult> {
    if no_subdomains {
        return None;
    }
    let domain = domain?;
    match subdomain::enumerate_subdomains(domain, concurrency).await {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("subdomain enumeration failed: {}", e);
            None
        }
    }
}

/// Enumerates DNS records for a domain.
///
/// Returns `None` if `no_dns_records` is true, no domain is provided, or enumeration fails.
async fn run_dns_records(
    domain: Option<&String>,
    no_dns_records: bool,
) -> Option<dns_records::DnsRecords> {
    if no_dns_records {
        return None;
    }
    let domain = domain?;
    match dns_records::enumerate_dns_records(domain).await {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("DNS records enumeration failed: {}", e);
            None
        }
    }
}

/// Detects the technology stack used by a web application.
///
/// Returns `None` if `no_tech` is true or detection fails.
async fn run_tech_detection(
    url: &str,
    no_tech: bool,
) -> Option<techdetect::TechDetectionResult> {
    if no_tech {
        return None;
    }
    match techdetect::detect_tech_stack(url).await {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("tech detection failed: {}", e);
            None
        }
    }
}

/// Analyzes JavaScript files for endpoints and secrets.
///
/// Returns `None` if `no_js` is true or analysis fails.
async fn run_js_analysis(
    url: &str,
    no_js: bool,
) -> Option<js::JsAnalysis> {
    if no_js {
        return None;
    }
    match js::analyze_js(url).await {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("JS analysis failed: {}", e);
            None
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
) -> Option<wayback::WaybackResult> {
    if no_wayback {
        return None;
    }
    let domain = domain?;
    match wayback::get_wayback_snapshots(domain, wayback_key, 100).await {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("wayback lookup failed: {}", e);
            None
        }
    }
}

/// Scans for cloud infrastructure misconfigurations.
///
/// Returns `None` if `no_cloud` is true, no domain is provided, or the scan fails.
async fn run_cloud_detection(
    domain: Option<&String>,
    concurrency: usize,
    no_cloud: bool,
) -> Option<cloud::CloudDiscovery> {
    if no_cloud {
        return None;
    }
    let domain = domain?;
    match cloud::scan_cloud(domain, concurrency).await {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("cloud scan failed: {}", e);
            None
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
) -> Option<content::ContentDiscovery> {
    if no_content {
        return None;
    }
    match content::scan_content(url, concurrency).await {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("content scan failed: {}", e);
            None
        }
    }
}

/// Analyzes CORS configuration for misconfigurations.
///
/// Returns `None` if `no_cors` is true or the analysis fails.
async fn run_cors_check(
    url: &str,
    no_cors: bool,
) -> Option<cors::CorsAnalysis> {
    if no_cors {
        return None;
    }
    match cors::analyze_cors(url).await {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("CORS analysis failed: {}", e);
            None
        }
    }
}

/// Discovers email addresses associated with a target.
///
/// Returns `None` if `no_email` is true or discovery fails.
async fn run_email_security(
    url: &str,
    no_email: bool,
) -> Option<email::EmailDiscovery> {
    if no_email {
        return None;
    }
    match email::discover_contacts(url).await {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("email discovery failed: {}", e);
            None
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
) -> Option<cve::CveMapping> {
    if no_cve {
        return None;
    }
    let tech = tech_result?;
    match cve::map_cves(&tech.tech_stack, None).await {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("CVE mapping failed: {}", e);
            None
        }
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

    let (url, domain, resolved_ip) = resolve_target(target, verbose).await;

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
        run_reverse_dns(resolved_ip.as_ref(), args.no_dns),
        run_geo_lookup(resolved_ip.as_ref(), args.no_geo, ipapi_key, maxmind_settings),
        run_threat_intel(resolved_ip.as_ref(), args.no_threat, virustotal_key, alienvault_key, shodan_key),
        run_ssl_recon(resolved_ip.as_ref(), &url, args.no_ssl),
        run_whois_lookup(domain.as_ref(), args.no_whois),
        run_subdomain_enum(domain.as_ref(), concurrency, args.no_subdomains),
        run_dns_records(domain.as_ref(), args.no_dns_records),
        run_tech_detection(&url, args.no_tech),
        run_js_analysis(&url, args.no_js),
        run_wayback_check(domain.as_ref(), args.no_wayback, wayback_key),
        run_cloud_detection(domain.as_ref(), concurrency, args.no_cloud),
        run_content_analysis(&url, concurrency, args.no_content),
        run_cors_check(&url, args.no_cors),
        run_email_security(&url, args.no_email),
    );

    let takeover_result = run_takeover_check(subdomain_result.as_ref(), args.no_takeover).await;

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
    recon.takeover = takeover_result;

    recon.cve_mapping = run_cve_check(techdetect_result.as_ref(), args.no_cve).await;
    recon.tech_stack = techdetect_result.map(|t| t.tech_stack);

    if verbose {
        eprintln!("Recon complete");
    }

    Ok(recon)
}

pub fn set_stage(stage: &Arc<Mutex<String>>, text: &str) {
    if let Ok(mut s) = stage.lock() {
        *s = text.to_string();
    }
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

    #[tokio::test]
    async fn test_resolve_target_http_prefix() {
        let (url, domain, _) = resolve_target("http://example.com/path", false).await;
        assert_eq!(url, "http://example.com/path");
        assert_eq!(domain, Some("example.com".to_string()));
    }

    #[tokio::test]
    async fn test_resolve_target_https_prefix() {
        let (url, domain, _) = resolve_target("https://example.com/page?q=1", false).await;
        assert_eq!(url, "https://example.com/page?q=1");
        assert_eq!(domain, Some("example.com".to_string()));
    }

    #[tokio::test]
    async fn test_resolve_target_no_prefix() {
        let (url, domain, resolved_ip) = resolve_target("example.com", false).await;
        assert_eq!(url, "https://example.com");
        assert_eq!(domain, Some("example.com".to_string()));
        let _ = resolved_ip;
    }

    #[tokio::test]
    async fn test_resolve_target_ip_address() {
        let (url, domain, resolved_ip) = resolve_target("8.8.8.8", false).await;
        assert_eq!(url, "https://8.8.8.8");
        assert_eq!(domain, Some("8.8.8.8".to_string()));
        assert_eq!(resolved_ip, Some("8.8.8.8".to_string()));
    }

    #[tokio::test]
    async fn test_resolve_target_ipv6() {
        let (url, domain, resolved_ip) = resolve_target("::1", false).await;
        assert_eq!(url, "https://::1");
        assert_eq!(domain, Some("::1".to_string()));
        assert_eq!(resolved_ip, Some("::1".to_string()));
    }

    #[tokio::test]
    async fn test_resolve_target_with_port() {
        let (url, domain, _) = resolve_target("http://example.com:8080/admin", false).await;
        assert_eq!(url, "http://example.com:8080/admin");
        assert_eq!(domain, Some("example.com:8080".to_string()));
    }

    #[tokio::test]
    async fn test_resolve_target_strips_path_from_domain() {
        let (url, domain, _) = resolve_target("https://example.com/a/b/c", false).await;
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
            tech_stack: None,
            reverse_dns: None,
            geolocation: None,
            geoip_error: None,
            whois: None,
            subdomains: None,
            ssl_analysis: None,
            dns_records: None,
            js_analysis: None,
            wayback: None,
            cloud: None,
            content: None,
            cors: None,
            email_discovery: None,
            threat_intel: None,
            cve_mapping: None,
            takeover: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("example.com"));
        assert!(json.contains("93.184.216.34"));
        let decoded: FullReconResult = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.target, "example.com");
    }

    #[test]
    fn test_full_recon_result_with_subdomains() {
        use crate::recon::FullReconResult;
        use crate::recon::subdomain::{SubdomainInfo, SubdomainResult};
        let result = FullReconResult {
            target: "example.com".to_string(),
            domain: Some("example.com".to_string()),
            subdomains: Some(SubdomainResult {
                domain: "example.com".to_string(),
                subdomains: vec![
                    SubdomainInfo {
                        subdomain: "www".to_string(),
                        ip_addresses: vec!["93.184.216.34".to_string()],
                        has_mx: false,
                        has_cname: false,
                        has_txt: false,
                    },
                ],
                sources: vec!["crt.sh".to_string()],
            }),
            ..Default::default()
        };
        assert!(result.subdomains.is_some());
        let subs = result.subdomains.unwrap();
        assert_eq!(subs.subdomains.len(), 1);
        assert_eq!(subs.subdomains[0].subdomain, "www");
    }
}
