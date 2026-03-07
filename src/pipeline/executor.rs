use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use super::context::PipelineContext;
use super::report::PipelineReport;
use super::session::{save, PipelineSession};
use super::stage::{parse_stages, Stage};
use crate::cli::{CommonHttpArgs, ScanArgs, ScanProfile};
use crate::config::SlapperConfig;
use crate::scanner::spoof::SpoofConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub stage: Stage,
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

pub struct Pipeline {
    target: String,
    stages: Vec<Stage>,
    profile: ScanProfile,
    concurrency: usize,
    common: CommonHttpArgs,
    spoof_config: SpoofConfig,
    context: Arc<Mutex<PipelineContext>>,
    session_path: Option<String>,
    tui_mode: bool,
}

impl Pipeline {
    pub fn new(target: &str) -> Self {
        Self {
            target: target.to_string(),
            stages: Vec::new(),
            profile: ScanProfile::Quick,
            concurrency: 10,
            common: CommonHttpArgs::default(),
            spoof_config: SpoofConfig::default(),
            context: Arc::new(Mutex::new(PipelineContext::new(target))),
            session_path: None,
            tui_mode: false,
        }
    }

    pub fn from_args(args: ScanArgs) -> Self {
        Self::from_args_with_tui_mode(args, None, false)
    }

    pub fn from_args_with_config(args: ScanArgs, config: &SlapperConfig) -> Self {
        Self::from_args_with_tui_mode(args, Some(config), false)
    }

    pub fn from_args_with_tui_mode(args: ScanArgs, config: Option<&SlapperConfig>, tui_mode: bool) -> Self {
        let stages = if let Some(stages_str) = &args.stages {
            parse_stages(stages_str)
        } else {
            Stage::from_profile(args.profile)
        };

        let concurrency = if let Some(cfg) = config {
            if args.concurrency == 10 {
                cfg.scan.default_concurrency
            } else {
                args.concurrency
            }
        } else {
            args.concurrency
        };

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
        ).unwrap_or_default();

        Self {
            target: args.target.clone(),
            stages,
            profile: args.profile,
            concurrency,
            common: args.common,
            spoof_config,
            context: Arc::new(Mutex::new(PipelineContext::new(&args.target))),
            session_path: args.output.clone(),
            tui_mode,
        }
    }

    pub fn from_session(session: PipelineSession) -> Self {
        let mut pipeline = Self::new(&session.target);
        pipeline.stages = session.remaining_stages;
        pipeline.context = Arc::new(Mutex::new(session.context));
        pipeline.spoof_config = SpoofConfig::default();
        pipeline
    }

    pub fn with_spoof_config(mut self, spoof_config: SpoofConfig) -> Self {
        self.spoof_config = spoof_config;
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

    pub fn has_stages(&self) -> bool {
        !self.stages.is_empty()
    }

    pub fn get_stages(&self) -> &[Stage] {
        &self.stages
    }

    pub async fn run(&self) -> Result<PipelineReport> {
        let start = Instant::now();
        let mut stage_results = Vec::new();
        
        let progress = if self.tui_mode {
            None
        } else {
            let pb = Arc::new(ProgressBar::new(self.stages.len() as u64));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} stages: {msg}")
                    .unwrap()
                    .progress_chars("#>-"),
            );
            Some(pb)
        };

        for stage in &self.stages {
            if let Some(ref pb) = progress {
                pb.set_message(stage.to_string());
            }
            let stage_start = Instant::now();

            let result = self.execute_stage(stage).await;

            let stage_result = StageResult {
                stage: stage.clone(),
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
                    completed_stages: stage_results.iter().map(|r| r.stage.clone()).collect(),
                    remaining_stages: self.stages[stage_results.len()..].to_vec(),
                    context: self.context.lock().await.clone(),
                };
                let _ = save(path, &session);
            }
        }

        if let Some(ref pb) = progress {
            pb.finish_and_clear();
        }

        let context = self.context.lock().await.clone();

        Ok(PipelineReport {
            target: self.target.clone(),
            total_duration_ms: start.elapsed().as_millis() as u64,
            stage_results,
            open_ports: context.port_results,
            services: context.services.into_values().collect(),
            endpoints: context.endpoints,
        })
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
        }
    }

    async fn run_port_scan(&self) -> Result<()> {
        let ports = crate::utils::parsing::parse_ports(&get_extended_ports())?;
        
        if self.spoof_config.enabled {
            eprintln!("{}", crate::scanner::spoof::format_spoof_warning(&self.spoof_config));
        }
        
        let results = crate::scanner::ports::scan_ports(
            &self.target,
            ports,
            self.concurrency,
            std::time::Duration::from_secs(2),
            self.tui_mode,
            self.spoof_config.clone(),
        )
        .await?;

        let mut context = self.context.lock().await;
        context.update_ports(results.open_ports);

        Ok(())
    }

    async fn run_fingerprint(&self) -> Result<()> {
        let context = self.context.lock().await;
        let ports: Vec<u16> = if context.open_ports.is_empty() {
            vec![
                21, 22, 23, 25, 53, 80, 110, 143, 443, 445, 993, 995, 1433, 1521, 3306, 3389,
                5432, 5900, 6379, 8080, 8443, 27017, 9092, 9200, 5672, 2181, 2375, 2376, 6443,
                10250,
            ]
        } else {
            context.open_ports.clone()
        };
        drop(context);

        let results =
            crate::scanner::fingerprint::fingerprint_services(&self.target, ports, std::time::Duration::from_secs(5), self.tui_mode)
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
            eprintln!("{}", crate::scanner::spoof::format_spoof_warning(&self.spoof_config));
        }

        let results = crate::scanner::endpoints::scan_endpoints(
            &base_url,
            get_default_endpoints(),
            self.concurrency,
            std::time::Duration::from_secs(10),
            false,
            self.tui_mode,
            self.spoof_config.clone(),
        )
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
            common: crate::cli::CommonHttpArgs {
                stealth,
                ..self.common.clone()
            },
        };

        let mut engine = crate::fuzzer::engine::FuzzEngine::new_with_tui_mode(args, self.tui_mode);
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

        let args = crate::cli::LoadArgs {
            url: base_url,
            requests: 100,
            concurrency: self.concurrency,
            method: "GET".to_string(),
            body: None,
            headers: Vec::new(),
            timeout: 30,
            json: false,
            verbose: false,
            output: None,
            common: self.common.clone(),
        };

        let runner = crate::loadtest::runner::LoadTestRunner::from_args_with_tui_mode(args, self.tui_mode)?;
        runner.run().await?;

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
            output: None,
            common: self.common.clone(),
        };

        crate::waf::run_cli(args, &crate::config::SlapperConfig::default()).await?;

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
            concurrency: Some(self.concurrency),
            json: false,
            quiet: false,
            verbose: false,
            output: None,
        };

        let config = crate::config::SlapperConfig::default();
        crate::recon::run_cli(args, &config).await?;

        Ok(())
    }
}

fn get_extended_ports() -> String {
    "21,22,23,25,53,80,110,143,443,445,993,995,1433,1521,3306,3389,5432,5900,6379,8080,8443,27017,9092,9200,5672,2181,2375,2376,6443,10250,3000,5000,8000,9000,4200,5601,9090".to_string()
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
        }
    }
}
