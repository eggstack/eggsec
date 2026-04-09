
use crate::scanner::spoof::{format_spoof_warning, SpoofConfig};
use crate::utils::preserve_all;
use crate::utils::sanitize_for_logging;
use crate::error::Result;
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::cli::EndpointScanArgs;
use crate::config::SlapperConfig;

#[derive(Clone)]
pub struct EndpointScanConfig {
    pub base_url: String,
    pub endpoints: Vec<String>,
    pub concurrency: usize,
    pub timeout_duration: Duration,
    pub include_404: bool,
    pub tui_mode: bool,
    pub spoof_config: SpoofConfig,
    pub verify_tls: bool,
    pub progress_tx: Option<tokio::sync::mpsc::Sender<(u64, u64)>>,
}

pub static DEFAULT_ENDPOINTS: &[&str] = &[
    "/admin",
    "/admin/login",
    "/admin/admin",
    "/administrator",
    "/api",
    "/api/v1",
    "/api/v2",
    "/api/admin",
    "/api/users",
    "/api/config",
    "/api/keys",
    "/api/secrets",
    "/api/internal",
    "/api/debug",
    "/api/swagger.json",
    "/api/openapi.json",
    "/api-docs",
    "/api-docs/swagger.json",
    "/.env",
    "/.git",
    "/.git/config",
    "/.git/HEAD",
    "/.gitignore",
    "/.htaccess",
    "/.htpasswd",
    "/.svn",
    "/.svn/entries",
    "/.DS_Store",
    "/backup",
    "/backup.sql",
    "/backup.zip",
    "/backups",
    "/config",
    "/config.php",
    "/config.json",
    "/config.yml",
    "/config.yaml",
    "/configuration.php",
    "/conf",
    "/console",
    "/dashboard",
    "/debug",
    "/dump",
    "/error",
    "/errors",
    "/graphql",
    "/health",
    "/healthz",
    "/info",
    "/install",
    "/install.php",
    "/login",
    "/login.php",
    "/logout",
    "/logs",
    "/metrics",
    "/phpinfo.php",
    "/phpmyadmin",
    "/pma",
    "/private",
    "/robots.txt",
    "/root",
    "/s3",
    "/secret",
    "/secrets",
    "/server-status",
    "/server-info",
    "/setup",
    "/shell",
    "/signin",
    "/signup",
    "/sitemap.xml",
    "/status",
    "/test",
    "/testing",
    "/tmp",
    "/upload",
    "/uploads",
    "/user",
    "/users",
    "/web.config",
    "/webadmin",
    "/wp-admin",
    "/wp-login.php",
    "/wp-config.php",
    "/wp-content",
    "/xmlrpc.php",
    "/actuator",
    "/actuator/health",
    "/actuator/env",
    "/actuator/metrics",
    "/actuator/mappings",
    "/actuator/configprops",
    "/actuator/heapdump",
    "/actuator/threaddump",
    "/actuator/loggers",
    "/actuator/auditevents",
    "/actuator/beans",
    "/actuator/info",
    "/actuator/sessions",
    "/.aws/credentials",
    "/.docker/config.json",
    "/.kube/config",
    "/.npmrc",
    "/.pgpass",
    "/.my.cnf",
    "/id_rsa",
    "/id_rsa.pub",
    "/.ssh/authorized_keys",
    "/.ssh/id_rsa",
    "/.ssh/config",
    "/credentials.json",
    "/secrets.json",
    "/keys.json",
    "/tokens.json",
    "/auth.json",
    "/service-account.json",
    "/.well-known/openid-configuration",
    "/.well-known/jwks.json",
    "/.well-known/security.txt",
    "/swagger",
    "/swagger-ui",
    "/swagger-ui.html",
    "/swagger-resources",
    "/v2/api-docs",
    "/v3/api-docs",
    "/redoc",
    "/graphiql",
    "/console/sql",
    "/elmah.axd",
    "/trace.axd",
    "/__route.js",
    "/__webpack_hmr",
    "/.nuxt",
    "/.nuxt/dist",
    "/.nuxt/views",
    "/jenkins",
    "/jenkins/script",
    "/jenkins/queue",
    "/hudson",
    "/jolokia",
    "/solr",
    "/solr/admin",
    "/kibana",
    "/elasticsearch",
    "/.kibana",
    "/rabbitmq",
    "/rabbitmq/api",
    "/activemq",
    "/adminer",
    "/adminer.php",
    "/myadmin",
    "/mysql",
    "/pgadmin",
    "/pgadmin4",
    "/oracle",
    "/oraclectrl",
    "/websphere",
    "/weblogic",
    "/jmx-console",
    "/jmx-console/HtmlAdaptor",
    "/invoker",
    "/invoker/JMXInvokerServlet",
    "/web-console",
    "/web-console/Invoker",
    "/system/console",
    "/system/console/configMgr",
    "/manager",
    "/manager/html",
    "/manager/status",
    "/host-manager",
    "/host-manager/html",
    "/zabbix",
    "/zabbix/api_jsonrpc.php",
    "/nagios",
    "/nagios/cgi-bin/status.cgi",
    "/prometheus",
    "/prometheus/metrics",
    "/grafana",
    "/grafana/login",
    "/grafana/api",
    "/alertmanager",
    "/consul",
    "/consul/ui",
    "/nomad",
    "/nomad/ui",
    "/vault",
    "/vault/ui",
    "/traefik",
    "/traefik/dashboard",
    "/traefik/api",
    "/haproxy",
    "/haproxy?stats",
    "/haproxy-status",
    "/nginx-status",
    "/nginx_status",
    "/fpm-status",
    "/fpm_status",
    "/php-fpm-status",
    "/couchdb",
    "/couchdb/_utils",
    "/couchdb/_all_dbs",
    "/_all_dbs",
    "/_utils",
    "/redis",
    "/redis/command",
    "/mongo",
    "/mongo/execute",
    "/mongodb",
    "/.mongodb",
    "/mongoclient",
    "/.env.local",
    "/.env.development",
    "/.env.production",
    "/.env.test",
    "/.env.staging",
    "/.env.backup",
    "/.env.old",
    "/.env.save",
    "/.env.bak",
    "/.env~",
    "/.env.swp",
    "/.env.swo",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointResult {
    pub path: String,
    pub status_code: u16,
    pub status_text: String,
    pub content_length: Option<u64>,
    pub response_time_ms: u64,
    pub redirect: Option<String>,
    pub interesting: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EndpointScanResults {
    pub base_url: String,
    pub endpoints_scanned: usize,
    pub endpoints_found: usize,
    pub interesting_findings: usize,
    pub duration_ms: u64,
    pub results: Vec<EndpointResult>,
}

impl std::fmt::Display for EndpointScanResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Endpoint Scan Results")?;
        writeln!(f, "target: {}", preserve_all(&self.base_url, 60))?;
        writeln!(f, "scanned: {} endpoints", self.endpoints_scanned)?;
        writeln!(f, "found: {} endpoints", self.endpoints_found)?;

        if self.results.is_empty() {
            writeln!(f, "no endpoints found")?;
        } else {
            let _ = writeln!(f, "endpoints");
            for result in &self.results {
                let marker = if result.interesting { "[!]" } else { "   " };
                let size = result
                    .content_length
                    .map(|l| l.to_string())
                    .unwrap_or_else(|| "-".to_string());
                writeln!(
                    f,
                    "{}\t{}\t{}\t{}\t{}",
                    marker, result.status_code, size, result.response_time_ms, result.path
                )?;
            }
        }

        writeln!(f, "\nLegend: [!] = Interesting finding")?;
        Ok(())
    }
}

fn is_interesting(path: &str, status_code: u16) -> bool {
    let sensitive_patterns = [
        ".env",
        ".git",
        ".ssh",
        ".aws",
        ".kube",
        "credentials",
        "secrets",
        "keys",
        "tokens",
        "auth",
        "password",
        "id_rsa",
        "backup",
        "dump",
        "config",
        "private",
        "admin",
        "phpmyadmin",
        "wp-admin",
        "wp-config",
        "actuator/heapdump",
        "actuator/threaddump",
        "actuator/env",
        "swagger",
        "api-docs",
        "graphql",
        "graphiql",
        "debug",
        "jenkins",
        "jolokia",
        "solr/admin",
        "manager/html",
    ];

    let path_lower = path.to_lowercase();

    if status_code == 200 || status_code == 403 || status_code == 401 {
        for pattern in &sensitive_patterns {
            if path_lower.contains(pattern) {
                return true;
            }
        }
    }

    false
}

pub async fn run_cli(args: EndpointScanArgs, config: &SlapperConfig) -> Result<()> {
    if args.verbose {
        eprintln!("Starting endpoint enumeration on {}", sanitize_for_logging(&args.url));
    }

    let endpoints = if let Some(wordlist_path) = args.wordlist {
        let content = tokio::fs::read_to_string(&wordlist_path).await?;
        content
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        DEFAULT_ENDPOINTS.iter().map(|s| s.to_string()).collect()
    };

    let timeout_secs = if args.timeout == 10 {
        config.http.timeout_secs
    } else {
        args.timeout
    };

    let spoof_config = SpoofConfig::from_args(
        args.spoof_ip.clone(),
        args.spoof_range.clone(),
        false,
        args.decoy.clone(),
        args.decoy_range.clone(),
        args.decoy_count,
        args.decoy_mode.clone(),
        args.include_me,
        None,
        false,
        false,
        None,
        None,
        None,
        None,
    )?;

    if spoof_config.enabled {
        eprintln!("{}", format_spoof_warning(&spoof_config));
    }

    let results = scan_endpoints(EndpointScanConfig {
        base_url: args.url.clone(),
        endpoints,
        concurrency: args.concurrency,
        timeout_duration: Duration::from_secs(timeout_secs),
        include_404: args.include_404,
        tui_mode: false,
        spoof_config,
        verify_tls: config.http.verify_tls,
        progress_tx: None,
    })
    .await?;

    if args.verbose {
        eprintln!(
            "Endpoint scan complete: {} endpoints found",
            results.endpoints_found
        );
    }

    let output = if args.json {
        serde_json::to_string_pretty(&results)?
    } else {
        format!("{}", results)
    };

    if let Some(ref output_file) = args.output {
        tokio::fs::write(output_file, &output).await?;
        if args.verbose {
            eprintln!("Results written to {}", output_file);
        }
    } else {
        println!("{}", output);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_interesting_sensitive_path_200() {
        assert!(is_interesting("/.env", 200));
        assert!(is_interesting("/admin", 200));
        assert!(is_interesting("/config.php", 200));
        assert!(is_interesting("/.git/HEAD", 200));
        assert!(is_interesting("/.ssh/authorized_keys", 200));
        assert!(is_interesting("/backup.sql", 200));
        assert!(is_interesting("/phpmyadmin", 200));
        assert!(is_interesting("/actuator/heapdump", 200));
        assert!(is_interesting("/swagger-ui", 200));
        assert!(is_interesting("/jenkins/script", 200));
    }

    #[test]
    fn test_is_interesting_sensitive_path_403() {
        assert!(is_interesting("/admin", 403));
        assert!(is_interesting("/.env", 403));
        assert!(is_interesting("/api/config", 403));
    }

    #[test]
    fn test_is_interesting_sensitive_path_401() {
        assert!(is_interesting("/admin", 401));
        assert!(is_interesting("/private", 401));
    }

    #[test]
    fn test_is_interesting_not_sensitive_200() {
        assert!(!is_interesting("/about", 200));
        assert!(!is_interesting("/contact", 200));
        assert!(!is_interesting("/faq", 200));
        assert!(!is_interesting("/pricing", 200));
    }

    #[test]
    fn test_is_interesting_sensitive_wrong_status() {
        assert!(!is_interesting("/admin", 404));
        assert!(!is_interesting("/.env", 500));
        assert!(!is_interesting("/.git", 301));
        assert!(!is_interesting("/config", 201));
    }

    #[test]
    fn test_is_interesting_case_insensitive() {
        assert!(is_interesting("/ADMIN", 200));
        assert!(is_interesting("/Admin", 200));
        assert!(is_interesting("/CONFIG.PHP", 200));
    }

    #[test]
    fn test_is_interesting_partial_path_match() {
        assert!(is_interesting("/api/admin/users", 200));
        assert!(is_interesting("/my/admin/panel", 200));
        assert!(is_interesting("/path/config/file", 200));
    }

    #[test]
    fn test_endpoint_result_serialization() {
        let result = EndpointResult {
            path: "/admin".to_string(),
            status_code: 200,
            status_text: "OK".to_string(),
            content_length: Some(1024),
            response_time_ms: 42,
            redirect: None,
            interesting: true,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: EndpointResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, "/admin");
        assert_eq!(deserialized.status_code, 200);
        assert_eq!(deserialized.content_length, Some(1024));
        assert!(deserialized.interesting);
    }

    #[test]
    fn test_endpoint_result_display() {
        let results = EndpointScanResults {
            base_url: "https://example.com".to_string(),
            endpoints_scanned: 100,
            endpoints_found: 2,
            interesting_findings: 1,
            duration_ms: 5000,
            results: vec![
                EndpointResult {
                    path: "/admin".to_string(),
                    status_code: 200,
                    status_text: "OK".to_string(),
                    content_length: Some(1024),
                    response_time_ms: 42,
                    redirect: None,
                    interesting: true,
                },
                EndpointResult {
                    path: "/about".to_string(),
                    status_code: 200,
                    status_text: "OK".to_string(),
                    content_length: Some(512),
                    response_time_ms: 30,
                    redirect: None,
                    interesting: false,
                },
            ],
        };
        let output = format!("{}", results);
        assert!(output.contains("Endpoint Scan Results"));
        assert!(output.contains("example.com"));
        assert!(output.contains("[!]"));
        assert!(output.contains("/admin"));
        assert!(output.contains("/about"));
    }

    #[test]
    fn test_endpoint_scan_results_empty_display() {
        let results = EndpointScanResults {
            base_url: "https://example.com".to_string(),
            endpoints_scanned: 100,
            endpoints_found: 0,
            interesting_findings: 0,
            duration_ms: 5000,
            results: vec![],
        };
        let output = format!("{}", results);
        assert!(output.contains("no endpoints found"));
    }

    #[test]
    fn test_default_endpoints_not_empty() {
        assert!(!DEFAULT_ENDPOINTS.is_empty());
        assert!(DEFAULT_ENDPOINTS.contains(&"/admin"));
        assert!(DEFAULT_ENDPOINTS.contains(&"/.env"));
        assert!(DEFAULT_ENDPOINTS.contains(&"/robots.txt"));
    }

    #[test]
    fn test_default_endpoints_no_duplicates() {
        let mut sorted: Vec<&str> = DEFAULT_ENDPOINTS.to_vec();
        sorted.sort();
        let before_len = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), before_len, "DEFAULT_ENDPOINTS contains duplicates");
    }
}

pub async fn scan_endpoints(config: EndpointScanConfig) -> Result<EndpointScanResults> {
    let client = Client::builder()
        .timeout(config.timeout_duration)
        .danger_accept_invalid_certs(!config.verify_tls)
        .redirect(reqwest::redirect::Policy::limited(5))
        .build().map_err(|e| crate::error::SlapperError::from(e).with_timeout(config.timeout_duration.as_millis() as u64))?;

    let results: Arc<Mutex<Vec<EndpointResult>>> = Arc::new(Mutex::new(Vec::new()));
    let scanned_count = Arc::new(tokio::sync::Mutex::new(0u64));
    let total_endpoints = config.endpoints.len() as u64;

    let progress = if config.tui_mode {
        None
    } else {
        let pb = Arc::new(ProgressBar::new(config.endpoints.len() as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} endpoints ({eta})")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("#>-"),
        );
        Some(pb)
    };

    let semaphore = Arc::new(tokio::sync::Semaphore::new(config.concurrency));
    let mut handles = Vec::new();
    let start = std::time::Instant::now();
    let base = config.base_url.trim_end_matches('/');
    let endpoints_count = config.endpoints.len();

    for endpoint in config.endpoints {
        let permit = semaphore.clone().acquire_owned().await?;
        let client = client.clone();
        let results = results.clone();
        let progress = progress.clone();
        let url = format!("{}{}", base, endpoint);
        let endpoint_path = endpoint;
        let spoof_config = config.spoof_config.clone();
        let scanned_count = scanned_count.clone();
        let progress_tx = config.progress_tx.clone();

        let handle = tokio::spawn(async move {
            let request_start = std::time::Instant::now();

            let mut request = client.get(&url);

            if spoof_config.enabled {
                if let Ok(Some(spoof_ip)) = spoof_config.header_value() {
                    request = request
                        .header("X-Forwarded-For", &spoof_ip)
                        .header("X-Real-IP", &spoof_ip)
                        .header("X-Originating-IP", &spoof_ip);
                }
            }

            if let Ok(response) = request.send().await {
                let status = response.status();
                let status_code = status.as_u16();

                if config.include_404 || status_code != 404 {
                    let content_length = response.content_length();
                    let redirect = if status.is_redirection() {
                        response
                            .headers()
                            .get("location")
                            .and_then(|h| h.to_str().ok())
                            .map(|s| s.to_string())
                    } else {
                        None
                    };

                    let interesting = is_interesting(&endpoint_path, status_code);

                    let mut results = results.lock().await;
                    results.push(EndpointResult {
                        path: endpoint_path,
                        status_code,
                        status_text: status.canonical_reason().unwrap_or("Unknown").to_string(),
                        content_length,
                        response_time_ms: request_start.elapsed().as_millis() as u64,
                        redirect,
                        interesting,
                    });
                }
            }

            if let Some(ref pb) = progress {
                pb.inc(1);
            }
            if let Some(ref tx) = progress_tx {
                let count = {
                    let mut c = scanned_count.lock().await;
                    *c += 1;
                    *c
                };
                let _ = tx.send((count, total_endpoints)).await;
            }
            drop(permit);
        });

        handles.push(handle);
    }

    join_all(handles).await;
    if let Some(ref pb) = progress {
        pb.finish_and_clear();
    }

    let mut results = results.lock().await.clone();
    results.sort_by(|a, b| {
        b.interesting
            .cmp(&a.interesting)
            .then_with(|| a.status_code.cmp(&b.status_code))
            .then_with(|| a.path.cmp(&b.path))
    });

    let endpoints_found = results.len();
    let interesting = results.iter().filter(|r| r.interesting).count();

    Ok(EndpointScanResults {
        base_url: config.base_url.clone(),
        endpoints_scanned: endpoints_count,
        endpoints_found,
        interesting_findings: interesting,
        duration_ms: start.elapsed().as_millis() as u64,
        results,
    })
}
