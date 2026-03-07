use crate::utils::truncate;
use crate::utils::urlencoding;
use anyhow::Result;
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use super::chain::{ChainAction, ChainExecutor, RequestTemplate};
use super::detection::{PatternMatcher, TimingAnalyzer};
use super::diff::ResponseDiffer;
use super::grammar::{Grammar, GrammarFuzzer};
use super::mutator::generate_mutations;
use super::payloads::{get_all_payloads, get_payloads, Payload, PayloadType};
use super::state::HttpSession;
use super::targets::{get_target_payloads, TargetPayload, TargetType};
use super::advanced::{
    AdvancedFuzzer, 
    GraphQLFuzzer, JwtFuzzer, OAuthFuzzer, IdorFuzzer, 
    SstiFuzzer, WebSocketFuzzer, GrpcFuzzer
};

use crate::cli::{FuzzArgs, FuzzMode, WafStressArgs};
use crate::waf::types::{OwaspCategory, Severity};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzResult {
    pub payload: Payload,
    pub status_code: u16,
    pub response_time_ms: u64,
    pub response_length: Option<u64>,
    pub is_waf_blocked: bool,
    pub is_anomaly: bool,
    pub is_redos_suspected: bool,
    pub leaks_found: Vec<String>,
    pub error: Option<String>,
    pub owasp_category: Option<String>,
    pub detected_severity: Severity,
}

impl FuzzResult {
    pub fn is_vulnerable(&self) -> bool {
        !self.leaks_found.is_empty() || self.is_waf_blocked || self.is_anomaly || self.is_redos_suspected
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwaspSummary {
    pub a01_broken_access_control: usize,
    pub a02_cryptographic_failures: usize,
    pub a03_injection: usize,
    pub a04_insecure_design: usize,
    pub a05_security_misconfiguration: usize,
    pub a06_vulnerable_components: usize,
    pub a07_auth_failures: usize,
    pub a08_software_integrity: usize,
    pub a09_logging_failures: usize,
    pub a10_ssrf: usize,
}

impl OwaspSummary {
    pub fn from_results(results: &[FuzzResult]) -> Self {
        let mut summary = OwaspSummary {
            a01_broken_access_control: 0,
            a02_cryptographic_failures: 0,
            a03_injection: 0,
            a04_insecure_design: 0,
            a05_security_misconfiguration: 0,
            a06_vulnerable_components: 0,
            a07_auth_failures: 0,
            a08_software_integrity: 0,
            a09_logging_failures: 0,
            a10_ssrf: 0,
        };

        for result in results {
            let category = OwaspCategory::from_payload_type(&result.payload.payload_type.to_string());
            match category {
                OwaspCategory::A01_2021_BrokenAccessControl | OwaspCategory::A01_2023_BrokenObjectLevelAuthorization | OwaspCategory::A05_2023_BrokenAccessControl => {
                    summary.a01_broken_access_control += 1;
                }
                OwaspCategory::A02_2021_CryptographicFailures | OwaspCategory::A08_2023_WeakCryptography => {
                    summary.a02_cryptographic_failures += 1;
                }
                OwaspCategory::A03_2021_Injection | OwaspCategory::A03_2023_BrokenObjectPropertyLevelAccessControl => {
                    summary.a03_injection += 1;
                }
                OwaspCategory::A04_2021_InsecureDesign | OwaspCategory::A07_2023_InsecureDesign | OwaspCategory::A04_2023_UnrestrictedResourceConsumption => {
                    summary.a04_insecure_design += 1;
                }
                OwaspCategory::A05_2021_SecurityMisconfiguration | OwaspCategory::A06_2023_SecurityMisconfiguration => {
                    summary.a05_security_misconfiguration += 1;
                }
                OwaspCategory::A06_2021_VulnerableComponents => {
                    summary.a06_vulnerable_components += 1;
                }
                OwaspCategory::A07_2021_AuthFailures | OwaspCategory::A02_2023_BrokenAuthentication => {
                    summary.a07_auth_failures += 1;
                }
                OwaspCategory::A08_2021_SoftwareIntegrity | OwaspCategory::A08_2023_SoftwareIntegrityFailures => {
                    summary.a08_software_integrity += 1;
                }
                OwaspCategory::A09_2021_LoggingFailures | OwaspCategory::A09_2023_LoggingMonitoring => {
                    summary.a09_logging_failures += 1;
                }
                OwaspCategory::A10_2021_SSRF | OwaspCategory::A10_2023_SSRF => {
                    summary.a10_ssrf += 1;
                }
            }
        }

        summary
    }
}

impl std::fmt::Display for OwaspSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "OWASP Top 10")?;
        writeln!(f, "\tA01-BrokenAccessControl: {}", self.a01_broken_access_control)?;
        writeln!(f, "\tA02-CryptographicFailures: {}", self.a02_cryptographic_failures)?;
        writeln!(f, "\tA03-Injection: {}", self.a03_injection)?;
        writeln!(f, "\tA04-InsecureDesign: {}", self.a04_insecure_design)?;
        writeln!(f, "\tA05-SecurityMisconfiguration: {}", self.a05_security_misconfiguration)?;
        writeln!(f, "\tA06-VulnerableComponents: {}", self.a06_vulnerable_components)?;
        writeln!(f, "\tA07-AuthFailures: {}", self.a07_auth_failures)?;
        writeln!(f, "\tA08-SoftwareIntegrity: {}", self.a08_software_integrity)?;
        writeln!(f, "\tA09-LoggingFailures: {}", self.a09_logging_failures)?;
        writeln!(f, "\tA10-SSRF: {}", self.a10_ssrf)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzSession {
    pub target_url: String,
    pub mode: String,
    pub payload_type: String,
    pub total_payloads: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub waf_bypasses: usize,
    pub potential_leaks: usize,
    pub time_anomalies: usize,
    pub redos_suspected: usize,
    pub duration_ms: u64,
    pub total_requests: usize,
    pub findings: usize,
    pub results: Vec<FuzzResult>,
    pub owasp_summary: OwaspSummary,
    pub baseline: Option<BaselineResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineResponse {
    pub status_code: u16,
    pub response_time_ms: u64,
    pub content_length: Option<u64>,
    pub headers: std::collections::HashMap<String, String>,
}

impl std::fmt::Display for FuzzSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Fuzz Results")?;
        writeln!(f, "target: {}", truncate(&self.target_url, 60))?;
        writeln!(f, "mode: {} | payloads: {}", self.mode, self.total_payloads)?;
        writeln!(f, "requests: {} success / {} failed", self.successful_requests, self.failed_requests)?;
        writeln!(f, "waf_bypasses: {} | leaks: {} | anomalies: {} | redos: {}", 
            self.waf_bypasses, self.potential_leaks, self.time_anomalies, self.redos_suspected)?;
        writeln!(f, "duration: {}ms", self.duration_ms)?;
        writeln!(f, "{}", self.owasp_summary)?;
        
        let critical_results: Vec<_> = self.results.iter()
            .filter(|r| r.is_waf_blocked || r.is_anomaly || !r.leaks_found.is_empty())
            .take(10)
            .collect();

        if !critical_results.is_empty() {
            writeln!(f, "findings")?;
            for result in critical_results {
                let severity = if result.is_redos_suspected {
                    "CRITICAL"
                } else if !result.leaks_found.is_empty() {
                    "HIGH"
                } else if result.is_anomaly {
                    "MEDIUM"
                } else {
                    "INFO"
                };
                
                writeln!(f, "\t[{}] {} | {} | {}ms", 
                    severity, result.status_code, truncate(&result.payload.description, 40), result.response_time_ms)?;
                
                if !result.leaks_found.is_empty() {
                    for leak in result.leaks_found.iter().take(2) {
                        writeln!(f, "\t\tleak: {}", truncate(leak, 50))?;
                    }
                }
            }
        }
        
        Ok(())
    }
}



pub struct FuzzEngine {
    args: FuzzArgs,
    client: Client,
    timing_analyzer: Arc<Mutex<TimingAnalyzer>>,
    pattern_matcher: PatternMatcher,
    user_agent: String,
    tui_mode: bool,
    grammar_fuzzer: Option<GrammarFuzzer>,
    http_session: Option<HttpSession>,
    differ: Option<ResponseDiffer>,
    baseline_captured: bool,
}

impl FuzzEngine {
    pub fn new(args: FuzzArgs) -> Self {
        Self::new_with_tui_mode(args, false)
    }

    pub fn new_with_tui_mode(args: FuzzArgs, tui_mode: bool) -> Self {
        let insecure = args.common.insecure;
        let user_agent = args.common.user_agent.clone()
            .unwrap_or_else(crate::utils::stealth::tool_user_agent);
        
        let concurrency = args.concurrency.max(100);
        
        let mut client_builder = Client::builder()
            .timeout(Duration::from_secs(args.timeout))
            .danger_accept_invalid_certs(insecure)
            .redirect(reqwest::redirect::Policy::limited(10))
            .pool_max_idle_per_host(concurrency / 2)
            .pool_idle_timeout(Duration::from_secs(30))
            .tcp_nodelay(true);

        if let Some(proxy_url) = &args.common.proxy {
            if let Ok(mut proxy) = reqwest::Proxy::all(proxy_url) {
                if let Some(auth) = &args.common.proxy_auth {
                    let parts: Vec<&str> = auth.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        proxy = proxy.basic_auth(parts[0], parts[1]);
                    }
                }
                client_builder = client_builder.proxy(proxy);
            }
        }

        let client = client_builder
            .build()
            .expect("Failed to create HTTP client");

        let grammar_fuzzer = if args.grammar_fuzz {
            let grammar = match args.grammar_type.as_deref() {
                Some("json") => Grammar::json(),
                Some("graphql") => Grammar::graphql(),
                Some("xml") => Grammar::xml(),
                Some("jwt") => Grammar::jwt(),
                Some("ssti") => Grammar::ssti(),
                _ => Grammar::json(),
            };
            Some(GrammarFuzzer::new(grammar))
        } else {
            None
        };

        let http_session = if args.session {
            Some(HttpSession::new())
        } else {
            None
        };

        let differ = if args.diffing {
            Some(ResponseDiffer::new())
        } else {
            None
        };

        Self {
            args,
            client,
            timing_analyzer: Arc::new(Mutex::new(TimingAnalyzer::new())),
            pattern_matcher: PatternMatcher::new(),
            user_agent,
            tui_mode,
            grammar_fuzzer,
            http_session,
            differ,
            baseline_captured: false,
        }
    }

    pub fn new_from_waf_args(args: WafStressArgs) -> Self {
        let fuzz_args = FuzzArgs {
            url: args.url.clone(),
            payload_type: "all".to_string(),
            mode: FuzzMode::Sequential,
            mutate: false,
            mutation_count: 0,
            grammar_fuzz: false,
            grammar_type: None,
            adaptive_rate: false,
            session: false,
            diffing: false,
            capture_baseline: false,
            enhanced_redos: false,
            waf_fingerprint: false,
            chaining: false,
            chain_file: None,
            method: "GET".to_string(),
            param: None,
            concurrency: args.concurrency,
            timeout: args.timeout,
            json: args.json,
            output: None,
            verbose: false,
            format: None,
            target: None,
            jwt_token: None,
            oauth_issuer: None,
            oauth_client_id: None,
            oauth_client_secret: None,
            idor_base_id: None,
            idor_user_ids: None,
            ssti_param: None,
            graphql_introspection: true,
            graphql_depth_bypass: true,
            graphql_alias_overload: true,
            oauth_redirect: true,
            oauth_scope: true,
            oauth_state: true,
            oauth_grant: true,
            common: args.common,
        };
        Self::new(fuzz_args)
    }

    pub async fn run(&mut self) -> Result<()> {
        if self.args.verbose {
            eprintln!("Starting fuzz against {}", self.args.url);
        }
        
        let session = self.run_return_session().await?;
        
        let output = if self.args.json {
            serde_json::to_string_pretty(&session)?
        } else {
            session.to_string()
        };
        
        if let Some(ref output_file) = self.args.output {
            std::fs::write(output_file, &output)?;
            if self.args.verbose {
                eprintln!("Results written to {}", output_file);
            }
        } else {
            println!("{}", output);
        }
        
        if self.args.verbose {
            eprintln!("Fuzz complete: {} requests, {} findings", 
                session.total_requests, session.findings);
        }

        Ok(())
    }

    pub async fn run_return_session(&mut self) -> Result<FuzzSession> {
        if self.args.capture_baseline && self.differ.is_some() {
            self.capture_baseline_for_diffing().await?;
            self.baseline_captured = true;
        }

        let payload_types = self.parse_payload_types()?;
        let mut all_results = Vec::with_capacity(2048);
        let start = Instant::now();

        let advanced_types = ["graphql", "oauth", "jwt", "idor", "ssti", "websocket", "grpc"];
        
        for pt in payload_types {
            let pt_str = format!("{:?}", pt).to_lowercase();
            
            if advanced_types.contains(&pt_str.as_str()) {
                let advanced_results = self.run_advanced_fuzzer(&pt_str).await?;
                all_results.extend(advanced_results);
            } else {
                let mut payloads = if self.args.mutate {
                    self.mutate_payloads(get_payloads(pt))
                } else {
                    get_payloads(pt)
                };

                if self.args.grammar_fuzz {
                    if let Some(ref mut grammar_fuzzer) = self.grammar_fuzzer {
                        let grammar_payloads = grammar_fuzzer.generate_batch(self.args.mutation_count.max(10));
                        payloads.extend(grammar_payloads.into_iter().map(|p| Payload {
                            payload_type: PayloadType::Xss,
                            payload: p,
                            description: "Grammar-generated payload".to_string(),
                            severity: Severity::Medium,
                            tags: vec!["grammar".to_string()],
                        }));
                    }
                }

                let results = match self.args.mode {
                    FuzzMode::Sequential => self.run_sequential_with_session(payloads).await?,
                    FuzzMode::Burst => self.run_burst_with_session(payloads).await?,
                    FuzzMode::Adaptive => self.run_adaptive_with_session(payloads).await?,
                };

                if self.args.diffing && self.differ.is_some() {
                    let diffed_results = self.apply_diffing(results).await?;
                    all_results.extend(diffed_results);
                } else {
                    all_results.extend(results);
                }
            }
        }

        if let Some(ref target_str) = self.args.target {
            if let Ok(target_type) = target_str.parse::<TargetType>() {
                let target_payloads = get_target_payloads(target_type);
                let payloads: Vec<Payload> = target_payloads.into_iter().map(|tp| Payload {
                    payload_type: PayloadType::Traversal,
                    payload: tp.payload,
                    description: tp.description,
                    severity: Severity::High,
                    tags: vec![target_type.to_string(), tp.category],
                }).collect();
                
                let results = match self.args.mode {
                    FuzzMode::Sequential => self.run_sequential(payloads).await?,
                    FuzzMode::Burst => self.run_burst(payloads).await?,
                    FuzzMode::Adaptive => self.run_adaptive(payloads).await?,
                };
                all_results.extend(results);
            }
        }

        Ok(self.build_session(all_results, start.elapsed(), None))
    }

    pub async fn run_all_types(&mut self) -> Result<()> {
        let payloads = if self.args.mutate {
            self.mutate_payloads(get_all_payloads())
        } else {
            get_all_payloads()
        };

        let start = Instant::now();
        let results = match self.args.mode {
            FuzzMode::Sequential => self.run_sequential(payloads).await?,
            FuzzMode::Burst => self.run_burst(payloads).await?,
            FuzzMode::Adaptive => self.run_adaptive(payloads).await?,
        };

        let session = self.build_session(results, start.elapsed(), None);
        
        if self.args.json {
            println!("{}", serde_json::to_string_pretty(&session)?);
        } else {
            println!("{}", session);
        }

        Ok(())
    }

    async fn run_advanced_fuzzer(&self, fuzzer_type: &str) -> Result<Vec<FuzzResult>> {
        let insecure = self.args.common.insecure;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(self.args.timeout))
            .danger_accept_invalid_certs(insecure)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()?;

        match fuzzer_type {
            "graphql" => {
                let mut fuzzer = GraphQLFuzzer::new(self.args.url.clone())
                    .with_introspection(self.args.graphql_introspection)
                    .with_depth_bypass(self.args.graphql_depth_bypass)
                    .with_alias_overload(self.args.graphql_alias_overload);
                Ok(fuzzer.fuzz(&client).await)
            }
            "jwt" => {
                let mut fuzzer = JwtFuzzer::new()
                    .with_target_url(self.args.url.clone());
                
                if let Some(ref token) = self.args.jwt_token {
                    fuzzer = fuzzer.with_original_token(token.clone());
                }
                
                Ok(fuzzer.fuzz(&client).await)
            }
            "oauth" => {
                let client_id = self.args.oauth_client_id.clone()
                    .unwrap_or_else(|| "test-client-id".to_string());
                let client_secret = self.args.oauth_client_secret.clone()
                    .unwrap_or_else(|| "test-client-secret".to_string());
                let redirect_uri = "http://localhost/callback".to_string();
                
                let mut fuzzer = OAuthFuzzer::new(client_id, redirect_uri)
                    .with_client_secret(client_secret)
                    .with_redirect_test(self.args.oauth_redirect)
                    .with_scope_test(self.args.oauth_scope)
                    .with_state_test(self.args.oauth_state)
                    .with_grant_test(self.args.oauth_grant);
                
                if let Some(ref issuer) = self.args.oauth_issuer {
                    fuzzer = fuzzer.with_issuer(issuer.clone());
                }
                
                Ok(fuzzer.fuzz(&client).await)
            }
            "idor" => {
                let mut fuzzer = IdorFuzzer::new(self.args.url.clone());
                
                if let Some(ref base_id) = self.args.idor_base_id {
                    fuzzer = fuzzer.with_base_user_id(base_id.clone());
                }
                
                if let Some(ref user_ids_str) = self.args.idor_user_ids {
                    let ids: Vec<String> = user_ids_str.split(',').map(|s| s.trim().to_string()).collect();
                    fuzzer = fuzzer.with_user_ids(ids);
                }
                
                Ok(fuzzer.fuzz(&client).await)
            }
            "ssti" => {
                let mut fuzzer = SstiFuzzer::new()
                    .with_target_url(self.args.url.clone());
                
                if let Some(ref param) = self.args.ssti_param {
                    fuzzer = fuzzer.with_param_name(param.clone());
                }
                
                Ok(fuzzer.fuzz(&client).await)
            }
            "websocket" => {
                let mut fuzzer = WebSocketFuzzer::new(self.args.url.clone());
                fuzzer.fuzz(&client).await
            }
            "grpc" => {
                let mut fuzzer = GrpcFuzzer::new(self.args.url.clone());
                fuzzer.fuzz(&client).await
            }
            _ => Ok(Vec::new()),
        }
    }

    fn parse_payload_types(&self) -> Result<Vec<PayloadType>> {
        if self.args.payload_type == "all" {
            return Ok(vec![
                PayloadType::Sqli,
                PayloadType::Xss,
                PayloadType::Traversal,
                PayloadType::Ssrf,
                PayloadType::Redirect,
                PayloadType::Redos,
                PayloadType::Headers,
                PayloadType::Compression,
                PayloadType::GraphQL,
                PayloadType::OAuth,
                PayloadType::Jwt,
                PayloadType::Idor,
                PayloadType::Ssti,
                PayloadType::Grpc,
                PayloadType::Xxe,
                PayloadType::Ldap,
                PayloadType::Cmd,
                PayloadType::Deser,
                PayloadType::Host,
                PayloadType::Cache,
                PayloadType::Csv,
                PayloadType::Soap,
            ]);
        }

        let types: Vec<PayloadType> = self.args.payload_type
            .split(',')
            .filter_map(|s| match s.trim().to_lowercase().as_str() {
                "sqli" | "sql" => Some(PayloadType::Sqli),
                "xss" => Some(PayloadType::Xss),
                "traversal" | "lfi" | "path" => Some(PayloadType::Traversal),
                "ssrf" => Some(PayloadType::Ssrf),
                "redirect" | "open-redirect" => Some(PayloadType::Redirect),
                "redos" | "regex" => Some(PayloadType::Redos),
                "headers" | "header" => Some(PayloadType::Headers),
                "compression" | "gzip" | "zip-bomb" => Some(PayloadType::Compression),
                "graphql" | "gql" => Some(PayloadType::GraphQL),
                "oauth" | "oidc" => Some(PayloadType::OAuth),
                "jwt" => Some(PayloadType::Jwt),
                "idor" | "auth" => Some(PayloadType::Idor),
                "ssti" | "template" => Some(PayloadType::Ssti),
                "grpc" | "protobuf" => Some(PayloadType::Grpc),
                "xxe" | "xml" => Some(PayloadType::Xxe),
                "ldap" => Some(PayloadType::Ldap),
                "cmd" | "command" | "rce" => Some(PayloadType::Cmd),
                "deser" | "deserialization" => Some(PayloadType::Deser),
                "host" | "host-header" => Some(PayloadType::Host),
                "cache" | "cache-poisoning" => Some(PayloadType::Cache),
                "csv" | "formula" => Some(PayloadType::Csv),
                "soap" => Some(PayloadType::Soap),
                _ => None,
            })
            .collect();

        if types.is_empty() {
            anyhow::bail!("No valid payload types specified");
        }

        Ok(types)
    }

    fn mutate_payloads(&self, payloads: Vec<Payload>) -> Vec<Payload> {
        let mut mutated = Vec::new();
        
        for payload in payloads {
            mutated.push(payload.clone());
            
            let mutations = generate_mutations(&payload.payload, self.args.mutation_count);
            for mutated_payload in mutations {
                if mutated_payload != payload.payload {
                    mutated.push(Payload {
                        payload_type: payload.payload_type,
                        payload: mutated_payload,
                        description: format!("{} (mutated)", payload.description),
                        severity: payload.severity,
                        tags: payload.tags.iter().cloned().chain(std::iter::once("mutated".to_string())).collect(),
                    });
                }
            }
        }
        
        mutated
    }

    async fn run_sequential(&self, payloads: Vec<Payload>) -> Result<Vec<FuzzResult>> {
        let progress = if self.tui_mode {
            None
        } else {
            let pb = Arc::new(ProgressBar::new(payloads.len() as u64));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} payloads ({eta})")
                    .unwrap()
                    .progress_chars("#>-"),
            );
            Some(pb)
        };

        let results = Arc::new(Mutex::new(Vec::new()));

        for payload in payloads {
            let result = self.send_payload(&payload).await?;
            results.lock().await.push(result);
            if let Some(ref pb) = progress {
                pb.inc(1);
            }
        }

        if let Some(ref pb) = progress {
            pb.finish_and_clear();
        }
        let results = results.lock().await.clone();
        Ok(results)
    }

    async fn run_burst(&self, payloads: Vec<Payload>) -> Result<Vec<FuzzResult>> {
        self.run_concurrent(payloads, "BURST MODE").await
    }

    async fn run_adaptive(&self, payloads: Vec<Payload>) -> Result<Vec<FuzzResult>> {
        self.run_concurrent(payloads, "ADAPTIVE MODE").await
    }

    async fn run_concurrent(&self, payloads: Vec<Payload>, mode_name: &str) -> Result<Vec<FuzzResult>> {
        let payload_count = payloads.len();
        
        let progress = if self.tui_mode {
            None
        } else {
            let pb = Arc::new(ProgressBar::new(payload_count as u64));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(&format!("{{spinner:.green}} [{{elapsed_precise}}] [{{bar:40.cyan/blue}}] {{pos}}/{{len}} - {}", mode_name))
                    .unwrap()
                    .progress_chars("#>-"),
            );
            Some(pb)
        };

        let results: Arc<Mutex<Vec<FuzzResult>>> = Arc::new(Mutex::new(Vec::with_capacity(payload_count)));
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.args.concurrency));
        let mut handles = Vec::new();

        for payload in payloads {
            let permit = semaphore.clone().acquire_owned().await?;
            let client = self.client.clone();
            let url = self.args.url.clone();
            let method = self.args.method.clone();
            let param = self.args.param.clone();
            let timing_analyzer = self.timing_analyzer.clone();
            let pattern_matcher = self.pattern_matcher.clone();
            let results = results.clone();
            let progress = progress.clone();
            let payload_clone = payload.clone();
            let user_agent = self.user_agent.clone();

            let handle = tokio::spawn(async move {
                let result = send_payload_async(
                    client,
                    &url,
                    &method,
                    param.as_deref(),
                    &payload_clone,
                    timing_analyzer,
                    pattern_matcher,
                    &user_agent,
                ).await;

                if let Ok(r) = result {
                    results.lock().await.push(r);
                }
                
                if let Some(ref pb) = progress {
                    pb.inc(1);
                }
                drop(permit);
            });

            handles.push(handle);
        }

        join_all(handles).await;
        if let Some(ref pb) = progress {
            pb.finish_and_clear();
        }
        let results = results.lock().await.clone();
        Ok(results)
    }

    async fn send_payload(&self, payload: &Payload) -> Result<FuzzResult> {
        send_payload_async(
            self.client.clone(),
            &self.args.url,
            &self.args.method,
            self.args.param.as_deref(),
            payload,
            self.timing_analyzer.clone(),
            self.pattern_matcher.clone(),
            &self.user_agent,
        ).await
    }

    fn build_session(&self, results: Vec<FuzzResult>, duration: Duration, baseline: Option<BaselineResponse>) -> FuzzSession {
        let successful = results.iter().filter(|r| r.error.is_none()).count();
        let failed = results.len() - successful;
        let waf_bypasses = results.iter().filter(|r| r.is_waf_blocked).count();
        let leaks = results.iter().filter(|r| !r.leaks_found.is_empty()).count();
        let anomalies = results.iter().filter(|r| r.is_anomaly).count();
        let redos = results.iter().filter(|r| r.is_redos_suspected).count();
        let findings = results.iter().filter(|r| r.is_vulnerable()).count();
        let owasp_summary = OwaspSummary::from_results(&results);

        FuzzSession {
            target_url: self.args.url.clone(),
            mode: format!("{:?}", self.args.mode),
            payload_type: self.args.payload_type.clone(),
            total_payloads: results.len(),
            successful_requests: successful,
            failed_requests: failed,
            waf_bypasses,
            potential_leaks: leaks,
            time_anomalies: anomalies,
            redos_suspected: redos,
            duration_ms: duration.as_millis() as u64,
            total_requests: results.len(),
            findings,
            results,
            owasp_summary,
            baseline,
        }
    }

    #[allow(dead_code)]
    async fn capture_baseline(&self) -> Result<BaselineResponse> {
        let start = Instant::now();
        let response = self.client
            .request(reqwest::Method::GET, self.args.url.clone())
            .header("User-Agent", &self.user_agent)
            .send()
            .await?;

        let response_time = start.elapsed();
        let status_code = response.status().as_u16();
        let content_length = response.content_length();
        
        let mut headers = std::collections::HashMap::new();
        for (name, value) in response.headers() {
            if let Ok(v) = value.to_str() {
                headers.insert(name.to_string(), v.to_string());
            }
        }

        Ok(BaselineResponse {
            status_code,
            response_time_ms: response_time.as_millis() as u64,
            content_length,
            headers,
        })
    }

    async fn capture_baseline_for_diffing(&mut self) -> Result<()> {
        if let Some(ref mut differ) = self.differ {
            let start = Instant::now();
            let response = self.client
                .get(&self.args.url)
                .header("User-Agent", &self.user_agent)
                .send()
                .await?;

            let status_code = response.status().as_u16();
            let body = response.bytes().await.unwrap_or_default();
            let headers = reqwest::header::HeaderMap::new();
            let timing_ms = start.elapsed().as_millis() as u64;

            differ.capture_baseline(status_code, &headers, &body, timing_ms);
        }
        Ok(())
    }

    async fn run_sequential_with_session(&mut self, payloads: Vec<Payload>) -> Result<Vec<FuzzResult>> {
        let mut results = Vec::with_capacity(payloads.len());
        
        for payload in payloads {
            let result = self.send_fuzz_request(&payload, Method::GET).await?;
            results.push(result);
        }
        
        if self.args.session {
            self.update_session_from_results(&results).await;
        }
        
        Ok(results)
    }

    async fn run_burst_with_session(&mut self, payloads: Vec<Payload>) -> Result<Vec<FuzzResult>> {
        let mut futures_vec = Vec::new();
        for _p in payloads {
            let client = self.client.clone();
            let url = self.args.url.clone();
            let user_agent = self.user_agent.clone();
            futures_vec.push(async move {
                let start = Instant::now();
                let response = client.get(&url)
                    .header("User-Agent", &user_agent)
                    .send()
                    .await;
                (start.elapsed(), response)
            });
        }
        
        let responses = join_all(futures_vec).await;
        
        let results: Vec<FuzzResult> = responses.into_iter().map(|(elapsed, response)| {
            let dummy_payload = Payload {
                payload_type: PayloadType::Xss,
                payload: "burst".to_string(),
                description: "Burst mode request".to_string(),
                severity: Severity::Info,
                tags: vec![],
            };
            match response {
                Ok(resp) => FuzzResult {
                    payload: dummy_payload,
                    status_code: resp.status().as_u16(),
                    response_time_ms: elapsed.as_millis() as u64,
                    response_length: resp.content_length(),
                    is_waf_blocked: false,
                    is_anomaly: false,
                    is_redos_suspected: false,
                    leaks_found: vec![],
                    error: None,
                    owasp_category: None,
                    detected_severity: Severity::Info,
                },
                Err(_) => FuzzResult {
                    payload: dummy_payload,
                    status_code: 0,
                    response_time_ms: elapsed.as_millis() as u64,
                    response_length: None,
                    is_waf_blocked: false,
                    is_anomaly: false,
                    is_redos_suspected: false,
                    leaks_found: vec![],
                    error: Some("Request failed".to_string()),
                    owasp_category: None,
                    detected_severity: Severity::Info,
                },
            }
        }).collect();

        if self.args.session {
            self.update_session_from_results(&results).await;
        }
        
        Ok(results)
    }

    async fn run_adaptive_with_session(&mut self, payloads: Vec<Payload>) -> Result<Vec<FuzzResult>> {
        self.run_sequential_with_session(payloads).await
    }

    async fn send_fuzz_request(&self, payload: &Payload, method: Method) -> Result<FuzzResult> {
        let url = self.build_fuzz_url(&payload.payload);
        
        let start = Instant::now();
        let mut request = self.client.request(method.clone(), &url)
            .header("User-Agent", &self.user_agent);

        if self.args.session {
            if let Some(ref session) = self.http_session {
                for (name, cookie) in &session.cookies {
                    request = request.header("Cookie", format!("{}={}", name, cookie.value));
                }
            }
        }

        if let Some(ref bearer) = self.args.common.bearer {
            request = request.bearer_auth(bearer);
        }

        let response = request.send().await;
        let elapsed = start.elapsed();

        match response {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let length = resp.content_length();
                
                Ok(FuzzResult {
                    payload: payload.clone(),
                    status_code: status,
                    response_time_ms: elapsed.as_millis() as u64,
                    response_length: length,
                    is_waf_blocked: false,
                    is_anomaly: false,
                    is_redos_suspected: false,
                    leaks_found: vec![],
                    error: None,
                    owasp_category: None,
                    detected_severity: payload.severity,
                })
            }
            Err(e) => {
                Ok(FuzzResult {
                    payload: payload.clone(),
                    status_code: 0,
                    response_time_ms: elapsed.as_millis() as u64,
                    response_length: None,
                    is_waf_blocked: false,
                    is_anomaly: false,
                    is_redos_suspected: false,
                    leaks_found: vec![],
                    error: Some(e.to_string()),
                    owasp_category: None,
                    detected_severity: Severity::Info,
                })
            }
        }
    }

    fn build_fuzz_url(&self, payload: &str) -> String {
        let url = &self.args.url;
        if let Some(param) = &self.args.param {
            if url.contains('?') {
                format!("{}&{}={}", url, param, urlencoding::encode(payload))
            } else {
                format!("{}?{}={}", url, param, urlencoding::encode(payload))
            }
        } else {
            url.clone()
        }
    }

    async fn update_session_from_results(&mut self, results: &[FuzzResult]) {
        if let Some(ref mut _session) = self.http_session {
            for result in results {
                if result.status_code == 200 || result.status_code == 302 {
                }
            }
        }
    }

    async fn apply_diffing(&mut self, results: Vec<FuzzResult>) -> Result<Vec<FuzzResult>> {
        if let Some(ref mut differ) = self.differ {
            let mut diffed_results = Vec::new();
            
            for result in results {
                let mut updated_result = result.clone();
                
                let start = Instant::now();
                if let Ok(resp) = self.client.get(&self.args.url)
                    .header("User-Agent", &self.user_agent)
                    .send()
                    .await 
                {
                    let status_code = resp.status().as_u16();
                    let body = resp.bytes().await.unwrap_or_default();
                    let headers = reqwest::header::HeaderMap::new();
                    let timing_ms = start.elapsed().as_millis() as u64;
                    
                    let diff = differ.diff(status_code, &headers, &body, timing_ms);
                    
                    if diff.diff.status_changed {
                        updated_result.is_anomaly = true;
                        updated_result.leaks_found.push(format!("Status changed: {}", diff.diff.status_changed));
                    }
                    if diff.diff.body_length_diff.abs() > 100 {
                        updated_result.is_anomaly = true;
                        updated_result.leaks_found.push(format!("Body length diff: {}", diff.diff.body_length_diff));
                    }
                }
                
                diffed_results.push(updated_result);
            }
            
            Ok(diffed_results)
        } else {
            Ok(results)
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn send_payload_async(
    client: Client,
    base_url: &str,
    method: &str,
    param: Option<&str>,
    payload: &Payload,
    timing_analyzer: Arc<Mutex<TimingAnalyzer>>,
    pattern_matcher: PatternMatcher,
    user_agent: &str,
) -> Result<FuzzResult> {
    let url = build_url(base_url, param, &payload.payload)?;
    let http_method = parse_method(method);

    let start = Instant::now();
    
    let response = client
        .request(http_method, url.clone())
        .header("User-Agent", user_agent)
        .send()
        .await;

    let response_time = start.elapsed();
    let mut timing = timing_analyzer.lock().await;
    let timing_result = timing.record(response_time);

    match response {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let content_length = resp.content_length();
            
            let body = resp.text().await.unwrap_or_default();
            let leaks = pattern_matcher.scan(&body);
            
            let is_waf_blocked = status == 403 || status == 406 || status == 429;
            
            let owasp_str = payload.payload_type.to_string();
            let detected_severity = compute_severity(&payload.severity, is_waf_blocked, timing_result.is_redos_suspected, !leaks.is_empty());
            
            Ok(FuzzResult {
                payload: payload.clone(),
                status_code: status,
                response_time_ms: timing_result.response_time_ms,
                response_length: content_length,
                is_waf_blocked,
                is_anomaly: timing_result.is_anomaly,
                is_redos_suspected: timing_result.is_redos_suspected,
                leaks_found: leaks.iter().map(|l| format!("{}: {}", l.category, l.pattern)).collect(),
                error: None,
                owasp_category: Some(owasp_str),
                detected_severity,
            })
        }
        Err(e) => {
            let owasp_str = payload.payload_type.to_string();
            Ok(FuzzResult {
                payload: payload.clone(),
                status_code: 0,
                response_time_ms: timing_result.response_time_ms,
                response_length: None,
                is_waf_blocked: false,
                is_anomaly: timing_result.is_anomaly,
                is_redos_suspected: timing_result.is_redos_suspected,
                leaks_found: Vec::new(),
                error: Some(e.to_string()),
                owasp_category: Some(owasp_str),
                detected_severity: Severity::Info,
            })
        }
    }
}

fn build_url(base_url: &str, param: Option<&str>, payload: &str) -> Result<url::Url> {
    let mut url = url::Url::parse(base_url)?;
    
    if let Some(p) = param {
        {
            let mut query_pairs = url.query_pairs_mut();
            query_pairs.append_pair(p, payload);
        }
    } else {
        let path = url.path();
        let new_path = if path.ends_with('/') {
            format!("{}{}", path, urlencoding::encode(payload))
        } else {
            format!("{}/{}", path, urlencoding::encode(payload))
        };
        url.set_path(&new_path);
    }
    
    Ok(url)
}

fn parse_method(method: &str) -> Method {
    match method.to_uppercase().as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        "PATCH" => Method::PATCH,
        "HEAD" => Method::HEAD,
        "OPTIONS" => Method::OPTIONS,
        _ => Method::GET,
    }
}

fn compute_severity(
    base_severity: &Severity,
    is_waf_blocked: bool,
    is_redos: bool,
    has_leak: bool,
) -> Severity {
    if is_redos || (is_waf_blocked && has_leak) {
        Severity::Critical
    } else if has_leak {
        Severity::High
    } else if is_waf_blocked {
        Severity::Medium
    } else {
        *base_severity
    }
}


