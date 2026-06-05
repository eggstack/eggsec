use std::sync::Arc;
use std::time::Instant;

use crate::error::Result;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use super::context::PipelineContext;
use super::report::PipelineReport;
use super::session::{save, PipelineSession};
use super::stage::{parse_stages, Stage, EXTENDED_SCAN_PORTS};
use crate::cli::{CommonHttpArgs, ScanArgs, ScanProfile};
use crate::config::SlapperConfig;
use crate::probe::ProbeRisk;
use crate::scanner::endpoints::EndpointScanConfig;
use crate::scanner::spoof::SpoofConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub stage: Stage,
    #[serde(skip)]
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

impl StageResult {
    pub fn new(stage: Stage, duration_ms: u64, success: bool, error: Option<String>) -> Self {
        Self {
            stage,
            duration_ms,
            success,
            error,
        }
    }
}

pub struct Pipeline {
    target: String,
    stages: Vec<Stage>,
    profile: ScanProfile,
    risk_budget: ProbeRisk,
    concurrency: usize,
    concurrent_stages: bool,
    common: CommonHttpArgs,
    spoof_config: SpoofConfig,
    context: Arc<Mutex<PipelineContext>>,
    session_path: Option<String>,
    tui_mode: bool,
    config: Option<SlapperConfig>,
}

impl Pipeline {
    pub fn new(target: &str) -> Self {
        let profile = ScanProfile::Quick;
        Self {
            target: target.to_string(),
            stages: Vec::new(),
            risk_budget: profile.max_risk_budget(),
            profile,
            concurrency: 10,
            concurrent_stages: false,
            common: CommonHttpArgs::default(),
            spoof_config: SpoofConfig::default(),
            context: Arc::new(Mutex::new(PipelineContext::new(target))),
            session_path: None,
            tui_mode: false,
            config: None,
        }
    }

    pub fn from_args(args: ScanArgs) -> Self {
        Self::from_args_with_tui_mode(args, None, false)
    }

    pub fn from_args_with_config(args: ScanArgs, config: &SlapperConfig) -> Self {
        Self::from_args_with_tui_mode(args, Some(config), false)
    }

    pub fn from_args_with_tui_mode(
        args: ScanArgs,
        config: Option<&SlapperConfig>,
        tui_mode: bool,
    ) -> Self {
        let stages = if let Some(stages_str) = &args.stages {
            parse_stages(stages_str)
        } else {
            Stage::from_profile(args.profile)
        };

        let default_concurrency = config
            .map(|c| c.scan.default_concurrency)
            .unwrap_or(10);
        let concurrency = args.concurrency.unwrap_or(default_concurrency);

        let spoof_config = SpoofConfig::from_args(
            args.source_ip.clone(),
            args.spoof_range.clone(),
            false,
            args.decoy.clone(),
            args.decoy_range.clone(),
            args.decoy_count,
            args.decoy_mode.clone(),
            args.include_me,
            args.source_port,
            args.random_source_port,
            args.fragment,
            args.scan_type.clone(),
            args.packet_trace.clone(),
            args.max_rate,
            args.ttl,
        )
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "Failed to parse spoof config, using defaults");
            SpoofConfig::default()
        });

        let session_path = args
            .output
            .clone()
            .filter(|p| p.ends_with(".session.json") || p.ends_with(".session"));

        Self {
            target: args.target.clone(),
            stages,
            profile: args.profile,
            risk_budget: args.profile.max_risk_budget(),
            concurrency,
            concurrent_stages: args.concurrent_stages,
            common: args.common,
            spoof_config,
            context: Arc::new(Mutex::new(PipelineContext::new(&args.target))),
            session_path,
            tui_mode,
            config: config.cloned(),
        }
    }

    pub fn from_session(session: PipelineSession) -> Self {
        let mut pipeline = Self::new(&session.target);
        pipeline.stages = session.remaining_stages;
        pipeline.context = Arc::new(Mutex::new(session.context));
        pipeline.spoof_config = session.spoof_config;
        if let Some(concurrency) = session.concurrency {
            pipeline.concurrency = concurrency;
        }
        if let Some(concurrent_stages) = session.concurrent_stages {
            pipeline.concurrent_stages = concurrent_stages;
        }
        if let Some(config) = session.config {
            pipeline.risk_budget = pipeline.profile.max_risk_budget();
            pipeline.config = Some(config);
        }
        pipeline
    }

    pub fn with_spoof_config(mut self, spoof_config: SpoofConfig) -> Self {
        self.spoof_config = spoof_config;
        self
    }

    pub fn with_config(mut self, config: SlapperConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn add_stage(mut self, stage: Stage) -> Self {
        self.stages.push(stage);
        self
    }

    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }

    pub fn with_concurrent_stages(mut self, enabled: bool) -> Self {
        self.concurrent_stages = enabled;
        self
    }

    pub fn has_stages(&self) -> bool {
        !self.stages.is_empty()
    }

    pub fn get_stages(&self) -> &[Stage] {
        &self.stages
    }

    /// Validate that defense-lab profiles target private/loopback addresses only.
    fn validate_defense_lab_scope(&self) -> Result<()> {
        if !self.profile.requires_private_scope() {
            return Ok(());
        }

        let target = &self.target;
        let host = target
            .strip_prefix("http://")
            .or_else(|| target.strip_prefix("https://"))
            .unwrap_or(target);
        let host = host.split('/').next().unwrap_or(host);
        let host = host.split(':').next().unwrap_or(host);
        let host = host
            .strip_prefix('[')
            .and_then(|h| h.strip_suffix(']'))
            .unwrap_or(host);

        let is_private = if let Ok(ip) = host.parse::<std::net::IpAddr>() {
            match ip {
                std::net::IpAddr::V4(v4) => {
                    v4.is_loopback()
                        || v4.is_private()
                        || v4.octets()[0] == 10
                        || (v4.octets()[0] == 172 && (15..=31).contains(&v4.octets()[1]))
                        || (v4.octets()[0] == 192 && v4.octets()[1] == 168)
                }
                std::net::IpAddr::V6(v6) => {
                    v6.is_loopback()
                        || (v6.segments()[0] & 0xfe00) == 0xfc00
                        || (v6.segments()[0] & 0xffc0) == 0xfe80
                }
            }
        } else {
            let lower = host.to_lowercase();
            lower == "localhost" || lower == "local" || lower.ends_with(".local")
        };

        if !is_private {
            return Err(crate::error::SlapperError::ScopeViolation(format!(
                "Defense-lab profile '{}' requires a local or private-lab target. \
                 Target '{}' appears to be a public address. Use a localhost or \
                 private CIDR target (e.g., 127.0.0.1, 10.0.0.0/8, 192.168.0.0/16).",
                self.profile, self.target
            )));
        }

        Ok(())
    }

    /// Validate that required compile-time features are enabled for the selected profile.
    fn validate_feature_gates(&self) -> Result<()> {
        if self.profile.requires_packet_inspection() && !cfg!(feature = "packet-inspection") {
            return Err(crate::error::SlapperError::Config(format!(
                "Profile '{}' requires the 'packet-inspection' feature. \
                 Rebuild with: cargo build --features packet-inspection",
                self.profile
            )));
        }

        if self.profile.requires_nse() && !cfg!(feature = "nse") {
            return Err(crate::error::SlapperError::Config(format!(
                "Profile '{}' requires the 'nse' feature. \
                 Rebuild with: cargo build --features nse",
                self.profile
            )));
        }

        Ok(())
    }

    /// Check whether a stage's risk level fits within the profile's risk budget.
    ///
    /// Returns `Ok(true)` if the stage should run, `Ok(false)` if it should
    /// be skipped (risk exceeds budget), or `Err` on hard blocks.
    fn validate_stage_risk(&self, stage: Stage) -> Result<bool> {
        let stage_risk = stage.to_probe_risk();
        if stage_risk.risk_level() > self.risk_budget.risk_level() {
            tracing::info!(
                stage = %stage,
                stage_risk = ?stage_risk,
                budget = ?self.risk_budget,
                "Skipping stage: risk level exceeds profile budget"
            );
            return Ok(false);
        }
        Ok(true)
    }

    pub async fn run(&self) -> Result<PipelineReport> {
        self.validate_defense_lab_scope()?;
        self.validate_feature_gates()?;
        let start = Instant::now();

        if self.concurrent_stages {
            return self.run_concurrent().await;
        }

        let mut stage_results = Vec::new();
        let mut checkpoint_error = None;

        let progress = if self.tui_mode || self.stages.is_empty() {
            None
        } else {
            let pb = Arc::new(ProgressBar::new(self.stages.len() as u64));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} stages: {msg}")
                    .unwrap_or_else(|_| ProgressStyle::default_bar())
                    .progress_chars("#>-"),
            );
            Some(pb)
        };

        for stage in &self.stages {
            if let Some(ref pb) = progress {
                pb.set_message(stage.to_string());
            }
            let stage_start = Instant::now();

            let allowed = self.validate_stage_risk(*stage)?;
            let result = if allowed {
                self.execute_stage(stage).await
            } else {
                tracing::info!(stage = %stage, "Stage skipped due to risk budget");
                Ok(())
            };

            let stage_result = StageResult {
                stage: *stage,
                duration_ms: stage_start.elapsed().as_millis() as u64,
                success: result.is_ok(),
                error: result.as_ref().err().map(|e| e.to_string()),
            };

            stage_results.push(stage_result);
            if let Some(ref pb) = progress {
                pb.inc(1);
            }

            if let Some(ref path) = self.session_path {
                let session = PipelineSession {
                    target: self.target.clone(),
                    completed_stages: stage_results.iter().map(|r| r.stage).collect(),
                    remaining_stages: self.stages[stage_results.len()..].to_vec(),
                    context: self.context.lock().await.clone(),
                    spoof_config: self.spoof_config.clone(),
                    concurrency: Some(self.concurrency),
                    concurrent_stages: Some(self.concurrent_stages),
                    config: self.config.clone(),
                };
                if let Err(e) = save(path, &session).await {
                    tracing::warn!(
                        path = %path,
                        error = %e,
                        "Failed to save session checkpoint - progress may be lost on interrupt"
                    );
                    checkpoint_error = Some(e.to_string());
                }
            }
        }

        if let Some(ref pb) = progress {
            pb.finish_and_clear();
        }

        let context = self.context.lock().await.clone();

        let mut report = PipelineReport {
            target: self.target.clone(),
            total_duration_ms: start.elapsed().as_millis() as u64,
            stage_results,
            open_ports: context.port_results,
            services: context.services.into_values().collect(),
            endpoints: context.endpoints,
            checkpoint_error,
            manifest: None,
            vuln_assessment: context.vuln_assessment,
            load_test_results: context.load_test_results,
        };

        let mut manifest =
            crate::output::RunManifest::from_report(&report, "pipeline", self.risk_budget);
        manifest.populate_findings_from_report(&report);
        report.manifest = Some(manifest);

        Ok(report)
    }

    async fn run_concurrent(&self) -> Result<PipelineReport> {
        self.validate_defense_lab_scope()?;
        self.validate_feature_gates()?;
        let start = Instant::now();

        let progress = if self.tui_mode || self.stages.is_empty() {
            None
        } else {
            let pb = Arc::new(ProgressBar::new(self.stages.len() as u64));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] {msg}")
                    .unwrap_or_else(|_| ProgressStyle::default_bar())
                    .progress_chars("#>-"),
            );
            pb.set_message(format!(
                "running {} stages concurrently",
                self.stages.len()
            ));
            Some(pb)
        };

        let risk_budget = self.risk_budget;
        let stage_futures: Vec<_> = self
            .stages
            .iter()
            .map(|stage| async move {
                let stage_start = Instant::now();
                let stage_risk = stage.to_probe_risk();
                let allowed = stage_risk.risk_level() <= risk_budget.risk_level();
                let result = if allowed {
                    self.execute_stage(stage).await
                } else {
                    tracing::info!(stage = %stage, "Stage skipped due to risk budget");
                    Ok(())
                };
                let stage_result = StageResult {
                    stage: *stage,
                    duration_ms: stage_start.elapsed().as_millis() as u64,
                    success: result.is_ok(),
                    error: result.as_ref().err().map(|e| e.to_string()),
                };
                stage_result
            })
            .collect();

        let stage_results = futures::future::join_all(stage_futures).await;

        if let Some(ref pb) = progress {
            pb.finish_and_clear();
        }

        let context = self.context.lock().await.clone();

        let mut checkpoint_error = None;
        if let Some(ref path) = self.session_path {
            let session = PipelineSession {
                target: self.target.clone(),
                completed_stages: stage_results.iter().map(|r| r.stage).collect(),
                remaining_stages: Vec::new(),
                context: context.clone(),
                spoof_config: self.spoof_config.clone(),
                concurrency: Some(self.concurrency),
                concurrent_stages: Some(self.concurrent_stages),
                config: self.config.clone(),
            };
            if let Err(e) = save(path, &session).await {
                tracing::warn!(
                    path = %path,
                    error = %e,
                    "Failed to save session checkpoint after concurrent execution"
                );
                checkpoint_error = Some(e.to_string());
            }
        }

        let mut report = PipelineReport {
            target: self.target.clone(),
            total_duration_ms: start.elapsed().as_millis() as u64,
            stage_results,
            open_ports: context.port_results,
            services: context.services.into_values().collect(),
            endpoints: context.endpoints,
            checkpoint_error,
            manifest: None,
            vuln_assessment: context.vuln_assessment,
            load_test_results: context.load_test_results,
        };

        let mut manifest =
            crate::output::RunManifest::from_report(&report, "pipeline", self.risk_budget);
        manifest.populate_findings_from_report(&report);
        report.manifest = Some(manifest);

        Ok(report)
    }

    async fn execute_stage(&self, stage: &Stage) -> Result<()> {
        match stage {
            Stage::PortScan => self.run_port_scan().await,
            Stage::Fingerprint => self.run_fingerprint().await,
            Stage::EndpointScan => self.run_endpoint_scan().await,
            Stage::Fuzz => self.run_fuzz().await,
            Stage::LoadTest => self.run_load_test().await,
            Stage::Waf => self.run_waf().await,
            Stage::Recon => self.run_recon().await,
            Stage::Vuln => self.run_vuln().await,
        }
    }

    async fn run_port_scan(&self) -> Result<()> {
        let ports = crate::utils::parsing::parse_ports(&get_extended_ports())?;

        if self.spoof_config.enabled {
            eprintln!(
                "{}",
                crate::scanner::spoof::format_spoof_warning(&self.spoof_config)
            );
        }

        let results = crate::scanner::ports::scan_ports(
            &self.target,
            crate::scanner::ports::PortScanConfig {
                ports,
                concurrency: self.concurrency,
                timeout_duration: std::time::Duration::from_secs(2),
                tui_mode: self.tui_mode,
                spoof_config: self.spoof_config.clone(),
                progress_tx: None,
                max_results: None,
            },
        )
        .await?;

        let mut context = self.context.lock().await;
        context.update_ports(results.open_ports);

        Ok(())
    }

    async fn run_fingerprint(&self) -> Result<()> {
        let context = self.context.lock().await;
        let ports: Vec<u16> = if context.open_ports.is_empty() {
            crate::utils::parsing::parse_ports(EXTENDED_SCAN_PORTS)?
        } else {
            context.open_ports.clone()
        };
        drop(context);

        let results = crate::scanner::fingerprint::fingerprint_services(
            &self.target,
            ports,
            std::time::Duration::from_secs(5),
            self.tui_mode,
            20,
            None,
            None,
        )
        .await?;

        let mut context = self.context.lock().await;
        context.update_services(results.results);

        Ok(())
    }

    async fn run_endpoint_scan(&self) -> Result<()> {
        let context = self.context.lock().await;
        let base_url = context.get_base_url().unwrap_or_else(|| {
            if self.target.starts_with("http") {
                self.target.clone()
            } else {
                format!("http://{}", self.target)
            }
        });
        drop(context);

        if self.spoof_config.enabled {
            eprintln!(
                "{}",
                crate::scanner::spoof::format_spoof_warning(&self.spoof_config)
            );
        }

        let verify_tls = self
            .config
            .as_ref()
            .map(|c| c.http.verify_tls)
            .unwrap_or(true);

        let results = crate::scanner::endpoints::scan_endpoints(EndpointScanConfig {
            base_url: base_url.clone(),
            endpoints: get_default_endpoints(),
            concurrency: self.concurrency,
            timeout_duration: std::time::Duration::from_secs(10),
            include_404: false,
            tui_mode: self.tui_mode,
            spoof_config: std::sync::Arc::new(self.spoof_config.clone()),
            verify_tls,
            progress_tx: None,
            max_results: None,
        })
        .await?;

        let mut context = self.context.lock().await;
        context.update_endpoints(results.results);

        Ok(())
    }

    async fn run_fuzz(&self) -> Result<()> {
        let context = self.context.lock().await;
        let base_url = context.get_base_url().unwrap_or_else(|| {
            if self.target.starts_with("http") {
                self.target.clone()
            } else {
                format!("http://{}", self.target)
            }
        });
        drop(context);

        let (payload_type, mutate, mutation_count) = match self.profile {
            ScanProfile::Api => ("graphql,jwt,oauth".to_string(), false, 0),
            ScanProfile::Stealth => ("all".to_string(), false, 0),
            ScanProfile::Deep => ("all".to_string(), true, 5),
            ScanProfile::Vuln => ("all".to_string(), false, 0),
            ScanProfile::Auth => ("jwt,oauth,idor".to_string(), false, 0),
            _ => ("all".to_string(), false, 0),
        };

        let stealth = self.profile == ScanProfile::Stealth || self.common.stealth;

        let args = crate::cli::FuzzArgs {
            url: base_url,
            payload_type,
            mode: crate::cli::FuzzMode::Sequential,
            mutate,
            mutation_count,
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
            concurrency: self.concurrency,
            timeout: 10,
            json: false,
            output: None,
            verbose: false,
            quiet: false,
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
            schema: None,
            discover_only: false,
            auto_discover_schema: false,
            calibrate: false,
            fc: None,
            fs: None,
            fw: None,
            fl: None,
            ft: None,
            fr: None,
            common: crate::cli::CommonHttpArgs {
                stealth,
                ..self.common.clone()
            },
        };

        let mut engine = crate::fuzzer::engine::FuzzEngine::new_with_tui_mode(args, self.tui_mode)?;
        engine.run().await?;

        Ok(())
    }

    async fn run_load_test(&self) -> Result<()> {
        let context = self.context.lock().await;
        let base_url = context.get_base_url().unwrap_or_else(|| {
            if self.target.starts_with("http") {
                self.target.clone()
            } else {
                format!("http://{}", self.target)
            }
        });
        drop(context);

        let default_config = SlapperConfig::default();
        let config = self.config.as_ref().unwrap_or(&default_config);
        let args = crate::cli::LoadArgs {
            url: base_url,
            requests: 100,
            concurrency: self.concurrency,
            method: "GET".to_string(),
            body: None,
            headers: Vec::new(),
            timeout: None,
            json: false,
            verbose: false,
            quiet: false,
            output: None,
            common: self.common.clone(),
        };

        let runner = crate::loadtest::runner::LoadTestRunner::from_args_with_config(args, config)?;
        let results = runner.run().await?;

        let mut context = self.context.lock().await;
        context.update_load_test_results(results);

        Ok(())
    }

    async fn run_waf(&self) -> Result<()> {
        let context = self.context.lock().await;
        let base_url = context.get_base_url().unwrap_or_else(|| {
            if self.target.starts_with("http") {
                self.target.clone()
            } else {
                format!("http://{}", self.target)
            }
        });
        drop(context);

        let args = crate::cli::WafArgs {
            url: base_url,
            detect_only: false,
            bypass: true,
            header_bypass: true,
            smuggling: true,
            evasion: true,
            profile: "auto".to_string(),
            test_type: None,
            concurrency: self.concurrency,
            timeout: 15,
            json: false,
            verbose: false,
            quiet: false,
            output: None,
            common: self.common.clone(),
        };

        crate::waf::run_cli(args).await?;

        Ok(())
    }

    async fn run_recon(&self) -> Result<()> {
        let args = crate::cli::ReconArgs {
            target: self.target.clone(),
            no_tech: false,
            no_dns: false,
            no_geo: false,
            no_whois: false,
            no_subdomains: false,
            no_ssl: false,
            no_dns_records: false,
            no_js: false,
            no_content: false,
            no_cloud: false,
            no_wayback: false,
            no_cors: false,
            no_threat: false,
            no_cve: false,
            no_email: false,
            no_takeover: false,
            concurrency: Some(self.concurrency),
            json: false,
            quiet: false,
            verbose: false,
            output: None,
        };

        let default_config = SlapperConfig::default();
        let config = self.config.as_ref().unwrap_or(&default_config);
        crate::recon::run_cli(args, config).await?;

        Ok(())
    }

    async fn run_vuln(&self) -> Result<()> {
        use crate::types::Severity;
        use crate::vuln::VulnAssessment;
        use crate::vuln::asset::assess_asset;
        use crate::vuln::prioritizer::prioritize_findings;

        let context = self.context.lock().await;
        let mut assessment = VulnAssessment::new("pipeline");

        let mut findings: Vec<(String, String, Severity, Option<f32>)> = Vec::new();

        for endpoint in &context.endpoints {
            if endpoint.interesting {
                let severity = match endpoint.status_code {
                    200 => Severity::Medium,
                    301 | 302 => Severity::Low,
                    401 | crate::constants::STATUS_FORBIDDEN => Severity::Medium,
                    500..=599 => Severity::High,
                    _ => Severity::Info,
                };
                findings.push((
                    format!("endpoint-{}", endpoint.path),
                    format!("Interesting endpoint: {}", endpoint.path),
                    severity,
                    None,
                ));
            }
        }

        for (port, service) in &context.services {
            if service.service == "HTTP" || service.service == "HTTPS" {
                findings.push((
                    format!("service-{}", port),
                    format!("{} service on port {}", service.service, port),
                    Severity::Info,
                    None,
                ));
            }
        }

        if findings.is_empty() {
            assessment.summary.push("No findings to assess".to_string());
        } else {
            let prioritized = prioritize_findings(&findings);
            assessment.summary.push(format!("Assessed {} finding(s):", prioritized.len()));
            for f in &prioritized {
                assessment.summary.push(format!(
                    "  #{} [{}] {} - Risk: {:.1} ({:?})",
                    f.priority_rank, f.severity, f.title, f.risk_score.combined_score, f.risk_score.priority_level
                ));
            }
            assessment.prioritized_findings = prioritized;
        }

        let target_str = self.target.clone();
        let asset = assess_asset(&target_str, "web_server");
        assessment.summary.push(format!("Asset criticality: {:.1}", asset.overall_score));
        assessment.asset_criticality = Some(asset);

        drop(context);

        let mut context = self.context.lock().await;
        context.update_vuln_assessment(assessment);

        Ok(())
    }
}

fn get_extended_ports() -> String {
    EXTENDED_SCAN_PORTS.to_string()
}

fn get_default_endpoints() -> Vec<String> {
    vec![
        "/admin".to_string(),
        "/admin/login".to_string(),
        "/api".to_string(),
        "/api/v1".to_string(),
        "/.env".to_string(),
        "/.git/config".to_string(),
        "/config".to_string(),
        "/debug".to_string(),
        "/health".to_string(),
        "/login".to_string(),
        "/robots.txt".to_string(),
        "/status".to_string(),
        "/swagger".to_string(),
        "/swagger-ui".to_string(),
        "/actuator".to_string(),
        "/actuator/health".to_string(),
        "/actuator/env".to_string(),
        "/metrics".to_string(),
        "/phpinfo.php".to_string(),
        "/server-status".to_string(),
    ]
}

#[allow(clippy::derivable_impls)]
impl Default for CommonHttpArgs {
    fn default() -> Self {
        Self {
            insecure: false,
            proxy: None,
            proxy_auth: None,
            auth: None,
            bearer: None,
            cookie: None,
            api_key: None,
            user_agent: None,
            stealth: false,
            rate_limit: None,
            jitter: None,
            auth_context: None,
            auth_role: None,
        }
    }
}
