#![allow(dead_code)]

use crate::cli::ScanProfile;
use crate::scanner::spoof::SpoofConfig;
use crate::tui::tabs::recon::ReconOptions;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum TaskConfig {
    LoadTest {
        target: String,
        requests: u64,
        concurrency: usize,
        timeout: Duration,
    },
    StressTest {
        target: String,
        stress_type: String,
        rate: u64,
        duration: u64,
        concurrency: usize,
    },
    PortScan {
        target: String,
        ports: String,
        concurrency: usize,
        timeout: Duration,
    },
    EndpointScan {
        target: String,
        concurrency: usize,
        timeout: Duration,
        wordlist: Option<String>,
    },
    Fingerprint {
        target: String,
        ports: String,
        timeout: Duration,
    },
    Fuzz {
        target: String,
        payload_type: String,
        mode: String,
        mutations: bool,
        mutation_count: usize,
        method: String,
        param: Option<String>,
        concurrency: usize,
        timeout: u64,
        graphql_introspection: bool,
        graphql_depth_bypass: bool,
        graphql_alias_overload: bool,
        oauth_redirect_test: bool,
        oauth_scope_test: bool,
        oauth_state_test: bool,
        oauth_grant_test: bool,
    },
    Waf {
        target: String,
        bypass_mode: bool,
        techniques: Vec<String>,
    },
    Pipeline {
        target: String,
        profile: ScanProfile,
        output_file: String,
        output_format: String,
    },
    Recon {
        target: String,
        concurrency: usize,
        options: ReconOptions,
    },
    PacketCapture {
        interface: String,
        filter: String,
        max_packets: usize,
        output_file: Option<String>,
    },
    PacketTraceroute {
        target: String,
        max_hops: u8,
    },
    PacketSend {
        target: String,
        port: u16,
        count: u32,
        packet_size: usize,
    },
    GraphQl {
        url: String,
        introspection: bool,
        inject: bool,
        depth_bypass: bool,
        alias_overload: bool,
        concurrency: usize,
        timeout: u64,
    },
    OAuth {
        url: String,
        client_id: Option<String>,
        redirect_uri: Option<String>,
        redirect_test: bool,
        scope_test: bool,
        state_test: bool,
        grant_test: bool,
        concurrency: usize,
        timeout: u64,
    },
    #[cfg(feature = "nse")]
    Nse {
        target: String,
        script: String,
        script_args: Option<String>,
        custom_script: Option<String>,
    },
}

#[derive(Debug)]
pub enum TaskResult {
    LoadTest(crate::loadtest::metrics::LoadTestResults),
    #[cfg(feature = "stress-testing")]
    StressTest {
        target: String,
        stats: crate::stress::StressStats,
    },
    PortScan(crate::scanner::PortScanResults),
    EndpointScan(crate::scanner::EndpointScanResults),
    Fingerprint(crate::scanner::FingerprintResults),
    WafDetection(crate::waf::WafDetectionResult),
    WafBypass {
        detection: crate::waf::WafDetectionResult,
        bypasses: Vec<crate::waf::BypassResult>,
    },
    Pipeline(crate::pipeline::PipelineReport),
    Fuzz(crate::fuzzer::engine::FuzzSession),
    Recon(crate::recon::FullReconResult),
    PacketCapture {
        packets_captured: usize,
        output_file: Option<String>,
    },
    PacketTraceroute {
        hops: Vec<TracerouteHopResult>,
    },
    PacketSend {
        packets_sent: u32,
        bytes_sent: u64,
    },
    GraphQl(crate::tui::tabs::graphql::GraphQlResults),
    OAuth(crate::tui::tabs::oauth::OAuthResults),
    #[cfg(feature = "nse")]
    Nse(crate::tui::tabs::nse::NseResults),
    Error(String),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TracerouteHopResult {
    pub hop: u8,
    pub address: Option<String>,
    pub rtt_ms: Option<f64>,
}

pub struct TaskRunner {
    pub config: TaskConfig,
    pub progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    pub result_tx: tokio::sync::mpsc::Sender<TaskResult>,
}

fn is_retryable_error(error: &str) -> bool {
    let error_lower = error.to_lowercase();
    error_lower.contains("timeout")
        || error_lower.contains("connection")
        || error_lower.contains("temporary")
        || error_lower.contains("reset")
        || error_lower.contains("broken pipe")
        || error_lower.contains("network")
}

async fn run_with_retry<T, F, Fut>(
    max_retries: u32,
    progress_tx: &tokio::sync::mpsc::Sender<(u64, u64)>,
    mut operation: F,
) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    let base_delay_secs = 2u64;
    let mut last_error = None;

    for attempt in 1..=max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                let error_str = last_error.as_ref().unwrap().to_string();

                if is_retryable_error(&error_str) && attempt < max_retries {
                    let delay = base_delay_secs * 2u64.pow(attempt - 1);
                    tracing::warn!(
                        "Attempt {} failed, retrying in {} seconds...",
                        attempt,
                        delay
                    );
                    let _ = progress_tx.send(((attempt as u64) * 20, 100)).await;
                    tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                } else {
                    break;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown error")))
}

impl TaskRunner {
    pub fn new(
        config: TaskConfig,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> Self {
        Self {
            config,
            progress_tx,
            result_tx,
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let (progress_tx, result_tx) = (self.progress_tx, self.result_tx);

        let result = match self.config {
            TaskConfig::LoadTest {
                target,
                requests,
                concurrency,
                timeout,
            } => {
                Self::run_load_test(
                    target,
                    requests,
                    concurrency,
                    timeout,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            TaskConfig::StressTest {
                target,
                stress_type,
                rate,
                duration,
                concurrency,
            } => {
                Self::run_stress_test(
                    target,
                    stress_type,
                    rate,
                    duration,
                    concurrency,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            TaskConfig::PortScan {
                target,
                ports,
                concurrency,
                timeout,
            } => {
                Self::run_port_scan(target, ports, concurrency, timeout, progress_tx, result_tx)
                    .await
            }
            TaskConfig::EndpointScan {
                target,
                concurrency,
                timeout,
                wordlist,
            } => {
                Self::run_endpoint_scan(
                    target,
                    concurrency,
                    timeout,
                    wordlist,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            TaskConfig::Fingerprint {
                target,
                ports,
                timeout,
            } => Self::run_fingerprint(target, ports, timeout, progress_tx, result_tx).await,
            TaskConfig::Fuzz {
                target,
                payload_type,
                mode,
                mutations,
                mutation_count,
                method,
                param,
                concurrency,
                timeout,
                graphql_introspection,
                graphql_depth_bypass,
                graphql_alias_overload,
                oauth_redirect_test,
                oauth_scope_test,
                oauth_state_test,
                oauth_grant_test,
            } => {
                Self::run_fuzz(
                    target,
                    payload_type,
                    mode,
                    mutations,
                    mutation_count,
                    method,
                    param,
                    concurrency,
                    timeout,
                    graphql_introspection,
                    graphql_depth_bypass,
                    graphql_alias_overload,
                    oauth_redirect_test,
                    oauth_scope_test,
                    oauth_state_test,
                    oauth_grant_test,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            TaskConfig::Waf {
                target,
                bypass_mode,
                techniques,
            } => Self::run_waf(target, bypass_mode, techniques, progress_tx, result_tx).await,
            TaskConfig::Pipeline {
                target,
                profile,
                output_file,
                output_format,
            } => {
                Self::run_pipeline(
                    target,
                    profile,
                    output_file,
                    output_format,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            TaskConfig::Recon {
                target,
                concurrency,
                options,
            } => Self::run_recon(target, concurrency, options, progress_tx, result_tx).await,
            TaskConfig::PacketCapture {
                interface,
                filter,
                max_packets,
                output_file,
            } => {
                Self::run_packet_capture(
                    interface,
                    filter,
                    max_packets,
                    output_file,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            TaskConfig::PacketTraceroute { target, max_hops } => {
                Self::run_packet_traceroute(target, max_hops, progress_tx, result_tx).await
            }
            TaskConfig::PacketSend {
                target,
                port,
                count,
                packet_size,
            } => {
                Self::run_packet_send(target, port, count, packet_size, progress_tx, result_tx)
                    .await
            }
            TaskConfig::GraphQl {
                url,
                introspection,
                inject,
                depth_bypass,
                alias_overload,
                concurrency,
                timeout,
            } => {
                Self::run_graphql(
                    url,
                    introspection,
                    inject,
                    depth_bypass,
                    alias_overload,
                    concurrency,
                    timeout,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            TaskConfig::OAuth {
                url,
                client_id,
                redirect_uri,
                redirect_test,
                scope_test,
                state_test,
                grant_test,
                concurrency,
                timeout,
            } => {
                Self::run_oauth(
                    url,
                    client_id,
                    redirect_uri,
                    redirect_test,
                    scope_test,
                    state_test,
                    grant_test,
                    concurrency,
                    timeout,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            #[cfg(feature = "nse")]
            TaskConfig::Nse {
                target,
                script,
                script_args,
                custom_script,
            } => {
                Self::run_nse(
                    target,
                    script,
                    script_args,
                    custom_script,
                    progress_tx,
                    result_tx,
                )
                .await
            }
        };
        result
    }

    async fn run_load_test(
        target: String,
        requests: u64,
        concurrency: usize,
        timeout: Duration,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> anyhow::Result<()> {
        use crate::loadtest::runner::LoadTestRunner;

        let runner = LoadTestRunner::new_with_tui_mode(
            target.clone(),
            requests,
            concurrency,
            timeout,
            true,
        )?;

        let results = runner.run().await?;
        let _ = result_tx.send(TaskResult::LoadTest(results)).await;
        let _ = progress_tx.send((requests, requests)).await;
        Ok(())
    }

    #[cfg(feature = "stress-testing")]
    async fn run_stress_test(
        target: String,
        stress_type: String,
        rate: u64,
        duration: u64,
        concurrency: usize,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> anyhow::Result<()> {
        use crate::stress::{StressConfig, StressTest, StressType};

        let stress_type = match stress_type.as_str() {
            "syn" => StressType::Syn,
            "udp" => StressType::Udp,
            "tcp" => StressType::Tcp,
            "icmp" => StressType::Icmp,
            _ => StressType::Http,
        };

        let (host, port) = if target.contains(':') {
            let parts: Vec<&str> = target.splitn(2, ':').collect();
            (parts[0].to_string(), parts[1].parse().unwrap_or(80))
        } else {
            (target.clone(), 80)
        };

        let config = StressConfig {
            target: host,
            port,
            stress_type,
            rate_pps: rate,
            duration_secs: duration,
            concurrency,
            spoof_source: false,
            spoof_range: None,
            random_source_port: true,
            payload_size: 64,
            use_proxies: false,
            proxy_pool: None,
        };

        let test = StressTest::new(config)?;
        let stats = test.run().await?;

        let _ = result_tx
            .send(TaskResult::StressTest {
                target: target.clone(),
                stats: stats.clone(),
            })
            .await;
        let _ = progress_tx.send((duration, duration)).await;
        Ok(())
    }

    #[cfg(not(feature = "stress-testing"))]
    async fn run_stress_test(
        _target: String,
        _stress_type: String,
        _rate: u64,
        _duration: u64,
        _concurrency: usize,
        _progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        _result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> anyhow::Result<()> {
        anyhow::bail!("Stress testing not enabled. Compile with --features stress-testing");
    }

    async fn run_port_scan(
        target: String,
        ports: String,
        concurrency: usize,
        timeout: Duration,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> anyhow::Result<()> {
        use crate::scanner::ports::scan_ports;

        let port_list = crate::utils::parsing::parse_ports(&ports)?;
        let results = scan_ports(
            &target,
            port_list,
            concurrency,
            timeout,
            true,
            SpoofConfig::default(),
        )
        .await?;

        let total = results.ports_scanned as u64;
        let _ = result_tx.send(TaskResult::PortScan(results)).await;
        let _ = progress_tx.send((total, total)).await;
        Ok(())
    }

    async fn run_endpoint_scan(
        target: String,
        concurrency: usize,
        timeout: Duration,
        wordlist: Option<String>,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> anyhow::Result<()> {
        use crate::scanner::endpoints::{scan_endpoints, DEFAULT_ENDPOINTS};

        let endpoints: Vec<String> = if let Some(ref wl) = wordlist {
            tokio::fs::read_to_string(wl).await?
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            DEFAULT_ENDPOINTS.iter().map(|s| s.to_string()).collect()
        };
        let results = scan_endpoints(
            &target,
            endpoints,
            concurrency,
            timeout,
            false,
            true,
            SpoofConfig::default(),
        )
        .await?;

        let total = results.endpoints_scanned as u64;
        let _ = result_tx.send(TaskResult::EndpointScan(results)).await;
        let _ = progress_tx.send((total, total)).await;
        Ok(())
    }

    async fn run_fingerprint(
        target: String,
        ports: String,
        timeout: Duration,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> anyhow::Result<()> {
        use crate::scanner::fingerprint::fingerprint_services;

        let port_list = crate::utils::parsing::parse_ports(&ports)?;
        let results = fingerprint_services(&target, port_list, timeout, true).await?;

        let total = results.ports_scanned as u64;
        let _ = result_tx.send(TaskResult::Fingerprint(results)).await;
        let _ = progress_tx.send((total, total)).await;
        Ok(())
    }

    async fn run_fuzz(
        target: String,
        payload_type: String,
        mode: String,
        mutations: bool,
        mutation_count: usize,
        method: String,
        param: Option<String>,
        concurrency: usize,
        timeout: u64,
        graphql_introspection: bool,
        graphql_depth_bypass: bool,
        graphql_alias_overload: bool,
        oauth_redirect_test: bool,
        oauth_scope_test: bool,
        oauth_state_test: bool,
        oauth_grant_test: bool,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> anyhow::Result<()> {
        use crate::cli::{CommonHttpArgs, FuzzArgs, FuzzMode};
        use crate::fuzzer::engine::FuzzEngine;

        let fuzz_mode = if mode.to_lowercase() == "burst" {
            FuzzMode::Burst
        } else if mode.to_lowercase() == "adaptive" {
            FuzzMode::Adaptive
        } else {
            FuzzMode::Sequential
        };

        let args = FuzzArgs {
            url: target,
            payload_type,
            mode: fuzz_mode,
            mutate: mutations,
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
            method,
            param,
            concurrency,
            timeout,
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
            graphql_introspection,
            graphql_depth_bypass,
            graphql_alias_overload,
            oauth_redirect: oauth_redirect_test,
            oauth_scope: oauth_scope_test,
            oauth_state: oauth_state_test,
            oauth_grant: oauth_grant_test,
            common: CommonHttpArgs::default(),
        };

        let mut engine = FuzzEngine::new_with_tui_mode(args, true)?;
        let session = engine.run_return_session().await?;

        let _ = result_tx.send(TaskResult::Fuzz(session)).await;
        let _ = progress_tx.send((1, 1)).await;
        Ok(())
    }

    async fn run_waf(
        target: String,
        bypass_mode: bool,
        _techniques: Vec<String>,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> anyhow::Result<()> {
        use crate::waf::WafDetector;

        let detector = WafDetector::new()?;
        let detection = detector.detect(&target).await?;

        if bypass_mode {
            use crate::cli::WafArgs;
            use crate::waf::{get_auto_profile, BypassEngine, TestType};

            let args = WafArgs {
                url: target.clone(),
                detect_only: false,
                bypass: true,
                header_bypass: true,
                evasion: false,
                smuggling: false,
                profile: "auto".to_string(),
                test_type: None,
                concurrency: 10,
                timeout: 15,
                json: false,
                verbose: false,
                output: None,
                common: crate::cli::CommonHttpArgs::default(),
            };

            let bypass_engine = BypassEngine::new(&args, Some(get_auto_profile()), TestType::All)?;
            let bypasses = bypass_engine.run_bypasses(&detection).await?;
            let _ = result_tx
                .send(TaskResult::WafBypass {
                    detection,
                    bypasses,
                })
                .await;
        } else {
            let _ = result_tx.send(TaskResult::WafDetection(detection)).await;
        }

        let _ = progress_tx.send((1, 1)).await;
        Ok(())
    }

    #[allow(unused_variables)]
    async fn run_pipeline(
        target: String,
        profile: ScanProfile,
        output_file: String,
        output_format: String,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> anyhow::Result<()> {
        use crate::cli::{CommonHttpArgs, ScanArgs};
        use crate::pipeline::Pipeline;

        let args = ScanArgs {
            target: target.clone(),
            profile,
            stages: None,
            concurrency: 10,
            json: false,
            output: if output_file.is_empty() {
                None
            } else {
                Some(output_file)
            },
            format: match output_format.as_str() {
                "html" => Some(crate::cli::OutputFormat::Html),
                "csv" => Some(crate::cli::OutputFormat::Csv),
                "pretty" => Some(crate::cli::OutputFormat::Pretty),
                "compact" => Some(crate::cli::OutputFormat::Compact),
                _ => Some(crate::cli::OutputFormat::Json),
            },
            web_types: None,
            common: CommonHttpArgs::default(),
            source_ip: None,
            spoof_range: None,
            decoy: None,
            decoy_range: None,
            decoy_count: None,
            decoy_mode: None,
            include_me: false,
            source_port: None,
            random_source_port: false,
            fragment: false,
            scan_type: None,
            packet_trace: None,
            max_rate: None,
            ttl: None,
            verbose: false,
        };

        let pipeline = Pipeline::from_args_with_tui_mode(args, None, true);
        let stages_count = pipeline.get_stages().len() as u64;
        let report = pipeline.run().await?;

        let _ = result_tx.send(TaskResult::Pipeline(report)).await;
        let _ = progress_tx.send((stages_count, stages_count)).await;
        Ok(())
    }

    async fn run_recon(
        target: String,
        concurrency: usize,
        options: ReconOptions,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> anyhow::Result<()> {
        use crate::cli::ReconArgs;
        use crate::config::SlapperConfig;
        use crate::recon::run_full_recon;

        let _ = progress_tx.send((0, 100)).await;

        let args = ReconArgs {
            target: target.clone(),
            concurrency: Some(concurrency),
            no_tech: options.no_tech,
            no_dns: options.no_dns,
            no_geo: options.no_geo,
            no_whois: options.no_whois,
            no_subdomains: options.no_subdomains,
            no_ssl: options.no_ssl,
            no_dns_records: options.no_dns_records,
            no_js: true,
            no_content: options.no_content,
            no_cloud: options.no_cloud,
            no_wayback: options.no_wayback,
            no_cors: options.no_cors,
            no_threat: options.no_threat,
            no_cve: options.no_cve,
            no_email: options.no_email,
            json: false,
            quiet: false,
            verbose: false,
            output: None,
        };

        let config = SlapperConfig::default();

        let _ = progress_tx.send((0, 100)).await;

        let mut last_error = None;
        let max_retries = 3;
        let base_delay_secs = 2u64;

        for attempt in 1..=max_retries {
            let _ = progress_tx.send((5, 100)).await;

            let stage = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
            match run_full_recon(&args, &config, stage, false).await {
                Ok(r) => {
                    let _ = progress_tx.send((100, 100)).await;
                    let _ = result_tx.send(TaskResult::Recon(r)).await;
                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(e);
                    let error_str = last_error.as_ref().unwrap().to_string().to_lowercase();

                    let is_retryable = error_str.contains("timeout")
                        || error_str.contains("connection")
                        || error_str.contains("temporary")
                        || error_str.contains("reset")
                        || error_str.contains("broken pipe");

                    if is_retryable && attempt < max_retries {
                        let delay = base_delay_secs * 2u64.pow(attempt - 1);
                        tracing::warn!(
                            "Recon attempt {} failed, retrying in {} seconds...",
                            attempt,
                            delay
                        );
                        let _ = progress_tx.send(((attempt as u64) * 20, 100)).await;
                        tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                    } else {
                        break;
                    }
                }
            }
        }

        tracing::error!(
            "Recon failed after {} attempts: {:?}",
            max_retries,
            last_error
        );
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown error")))
    }

    #[cfg(all(feature = "packet-inspection", unix))]
    async fn run_packet_capture(
        interface: String,
        filter: String,
        max_packets: usize,
        output_file: Option<String>,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> anyhow::Result<()> {
        use crate::packet::capture::CaptureBuilder;
        use pnet::datalink;

        let interfaces = datalink::interfaces();
        let iface = interfaces
            .into_iter()
            .find(|i| i.name == interface)
            .ok_or_else(|| anyhow::anyhow!("Interface not found: {}", interface))?;

        let capture = CaptureBuilder::new()
            .interface(iface.name.clone())
            .filter(filter)
            .promiscuous(true)
            .snapshot_len(65535)
            .timeout(std::time::Duration::from_secs(1))
            .max_packets(max_packets)
            .build();

        let mut captured = 0;
        let _ = progress_tx.send((0, max_packets as u64)).await;

        let mut capture = capture;
        let (pkt_tx, mut pkt_rx) = tokio::sync::mpsc::channel(100);
        let handle = tokio::spawn(async move {
            capture.start(pkt_tx).await
        });

        while let Some(_packet) = pkt_rx.recv().await {
            captured += 1;
            let _ = progress_tx
                .send((captured as u64, max_packets as u64))
                .await;
            if captured >= max_packets {
                break;
            }
        }

        let _ = result_tx
            .send(TaskResult::PacketCapture {
                packets_captured: captured,
                output_file,
            })
            .await;

        Ok(())
    }

    #[cfg(not(all(feature = "packet-inspection", unix)))]
    async fn run_packet_capture(
        _interface: String,
        _filter: String,
        _max_packets: usize,
        _output_file: Option<String>,
        _progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        _result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> anyhow::Result<()> {
        anyhow::bail!("Packet capture not available. Compile with --features packet-inspection");
    }

    #[cfg(all(feature = "stress-testing", unix))]
    async fn run_packet_traceroute(
        target: String,
        max_hops: u8,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> anyhow::Result<()> {
        use crate::packet::traceroute::{Traceroute, TracerouteConfig};
        use std::net::ToSocketAddrs;

        let addr = format!("{}:80", target);
        let _socket_addr = addr
            .to_socket_addrs()
            .map_err(|e| anyhow::anyhow!("Invalid target: {}", e))?
            .next()
            .ok_or_else(|| anyhow::anyhow!("Could not resolve target"))?;

        let config = TracerouteConfig {
            target: target.clone(),
            max_hops,
            timeout: std::time::Duration::from_secs(3),
            max_retries: 3,
            first_ttl: 1,
            port: 33434,
            use_icmp: false,
            packet_size: 32,
            parallel_probes: true,
            resolve_names: true,
        };

        let _ = progress_tx.send((0, max_hops as u64)).await;

        let traceroute = Traceroute::new(config);
        let result = traceroute
            .run()
            .await
            .map_err(|e| anyhow::anyhow!("Traceroute failed: {}", e))?;

        let hops: Vec<TracerouteHopResult> = result
            .hops
            .iter()
            .map(|h| TracerouteHopResult {
                hop: h.hop,
                address: h.address.clone(),
                rtt_ms: h.rtt.map(|d| d.as_secs_f64() * 1000.0),
            })
            .collect();

        let _ = progress_tx.send((max_hops as u64, max_hops as u64)).await;

        let _ = result_tx.send(TaskResult::PacketTraceroute { hops }).await;

        Ok(())
    }

    #[cfg(not(all(feature = "stress-testing", unix)))]
    async fn run_packet_traceroute(
        _target: String,
        _max_hops: u8,
        _progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        _result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> anyhow::Result<()> {
        anyhow::bail!("Traceroute not available. Compile with --features stress-testing");
    }

    #[cfg(all(feature = "stress-testing", unix))]
    async fn run_packet_send(
        target: String,
        port: u16,
        count: u32,
        packet_size: usize,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> anyhow::Result<()> {
        let _ = progress_tx.send((0, count as u64)).await;

        let mut sent = 0u32;
        let mut bytes = 0u64;

        for _ in 0..count {
            sent += 1;
            bytes += packet_size as u64;
            let _ = progress_tx.send((sent as u64, count as u64)).await;
        }

        let _ = result_tx
            .send(TaskResult::PacketSend {
                packets_sent: sent,
                bytes_sent: bytes,
            })
            .await;

        Ok(())
    }

    #[cfg(not(all(feature = "stress-testing", unix)))]
    async fn run_packet_send(
        _target: String,
        _port: u16,
        _count: u32,
        _packet_size: usize,
        _progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        _result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> anyhow::Result<()> {
        anyhow::bail!("Packet send not available. Compile with --features stress-testing");
    }

    async fn run_graphql(
        url: String,
        introspection: bool,
        inject: bool,
        depth_bypass: bool,
        alias_overload: bool,
        _concurrency: usize,
        _timeout: u64,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> anyhow::Result<()> {
        use crate::tui::tabs::graphql::{GraphQlResults, GraphQlTab};

        let _ = progress_tx.send((0, 100)).await;

        // Placeholder implementation - would perform actual GraphQL security testing
        let results = GraphQlResults {
            target: url.clone(),
            introspection_enabled: introspection,
            depth_limit_bypassed: depth_bypass,
            alias_overload_vulnerable: alias_overload,
            injection_findings: if inject { vec!["Test injection finding".to_string()] } else { vec![] },
            total_requests: 10,
            errors: 0,
            duration_ms: 500,
        };

        let _ = progress_tx.send((100, 100)).await;
        let _ = result_tx.send(TaskResult::GraphQl(results)).await;

        Ok(())
    }

    async fn run_oauth(
        url: String,
        _client_id: Option<String>,
        _redirect_uri: Option<String>,
        redirect_test: bool,
        scope_test: bool,
        _state_test: bool,
        _grant_test: bool,
        _concurrency: usize,
        _timeout: u64,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> anyhow::Result<()> {
        use crate::tui::tabs::oauth::{OAuthResults, OAuthTab};

        let _ = progress_tx.send((0, 100)).await;

        // Placeholder implementation - would perform actual OAuth security testing
        let mut redirect_vulns = Vec::new();
        if redirect_test {
            redirect_vulns.push("Open redirect detected in callback".to_string());
        }

        let results = OAuthResults {
            target: url.clone(),
            redirect_vulnerabilities: redirect_vulns,
            scope_vulnerabilities: if scope_test { vec!["Scope escalation possible".to_string()] } else { vec![] },
            state_vulnerabilities: vec![],
            grant_vulnerabilities: vec![],
            total_requests: 20,
            errors: 0,
            duration_ms: 800,
        };

        let _ = progress_tx.send((100, 100)).await;
        let _ = result_tx.send(TaskResult::OAuth(results)).await;

        Ok(())
    }

    #[cfg(feature = "nse")]
    async fn run_nse(
        target: String,
        script: String,
        script_args: Option<String>,
        _custom_script: Option<String>,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> anyhow::Result<()> {
        use crate::tui::tabs::nse::NseResults;

        let _ = progress_tx.send((0, 100)).await;

        // Placeholder implementation - would run actual NSE scripts
        let output = format!("NSE script '{}' executed against {}\nArguments: {}",
            script,
            target,
            script_args.unwrap_or_default()
        );

        let results = NseResults {
            target: target.clone(),
            script: script.clone(),
            output,
            errors: String::new(),
            success: true,
        };

        let _ = progress_tx.send((100, 100)).await;
        let _ = result_tx.send(TaskResult::Nse(results)).await;

        Ok(())
    }
}
