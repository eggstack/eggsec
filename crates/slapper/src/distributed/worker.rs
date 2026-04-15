use crate::distributed::{Heartbeat, Task, TaskResult, TaskType, WorkerRegistration, WorkerStatus};
use crate::error::Result;
use crate::scanner::endpoints::EndpointScanConfig;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub worker_id: String,
    pub coordinator_url: String,
    pub max_concurrency: usize,
    pub heartbeat_interval_secs: u64,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            worker_id: uuid::Uuid::new_v4().to_string(),
            coordinator_url: "http://localhost:8080".to_string(),
            max_concurrency: 10,
            heartbeat_interval_secs: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStats {
    pub worker_id: String,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub tasks_in_progress: usize,
    pub last_heartbeat_secs: i64,
}

pub struct Worker {
    config: WorkerConfig,
    stats: WorkerStats,
    receiver: Option<mpsc::Receiver<Task>>,
    client: reqwest::Client,
    heartbeat_handle: Option<JoinHandle<()>>,
    task_processor_handle: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn new(config: WorkerConfig) -> Self {
        Self {
            config: config.clone(),
            stats: WorkerStats {
                worker_id: config.worker_id.clone(),
                tasks_completed: 0,
                tasks_failed: 0,
                tasks_in_progress: 0,
                last_heartbeat_secs: chrono::Utc::now().timestamp(),
            },
            receiver: None,
            client: reqwest::Client::new(),
            heartbeat_handle: None,
            task_processor_handle: None,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        self.register_with_coordinator().await?;

        let (_tx, rx) = mpsc::channel::<Task>(100);
        self.receiver = Some(rx);

        self.start_heartbeat_loop().await;
        self.start_task_processing_loop().await;

        Ok(())
    }

    async fn register_with_coordinator(&self) -> Result<()> {
        let registration = WorkerRegistration {
            worker_id: self.config.worker_id.clone(),
            hostname: hostname::get()?.to_string_lossy().to_string(),
            capabilities: vec![
                TaskType::PortScan,
                TaskType::ServiceFingerprint,
                TaskType::EndpointDiscovery,
                TaskType::Fuzz,
                TaskType::WafTest,
                TaskType::LoadTest,
                TaskType::Recon,
            ],
            max_concurrency: self.config.max_concurrency,
            status: WorkerStatus::Idle,
        };

        let url = format!("{}/api/workers/register", self.config.coordinator_url);
        self.client.post(&url).json(&registration).send().await?;

        Ok(())
    }

    async fn start_heartbeat_loop(&mut self) {
        let worker_id = self.config.worker_id.clone();
        let coordinator_url = self.config.coordinator_url.clone();
        let interval = self.config.heartbeat_interval_secs;
        let client = self.client.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval));

            loop {
                interval.tick().await;

                let heartbeat = Heartbeat {
                    worker_id: worker_id.clone(),
                    status: WorkerStatus::Idle,
                    current_jobs: 0,
                    completed_jobs: 0,
                    failed_jobs: 0,
                    cpu_usage: 0.0,
                    memory_usage: 0.0,
                };

                let url = format!("{}/api/workers/heartbeat", coordinator_url);
                if let Err(e) = client.post(&url).json(&heartbeat).send().await {
                    tracing::warn!("Heartbeat failed: {}", e);
                }
            }
        });
        self.heartbeat_handle = Some(handle);
    }

    async fn start_task_processing_loop(&mut self) {
        if let Some(receiver) = self.receiver.take() {
            let handle = tokio::spawn(async move {
                let mut receiver = receiver;

                while let Some(task) = receiver.recv().await {
                    tokio::spawn(async move {
                        let result = process_task(task).await;
                        if let Err(e) = result {
                            tracing::error!("Task processing error: {}", e);
                        }
                    });
                }
            });
            self.task_processor_handle = Some(handle);
        }
    }

    pub fn get_stats(&self) -> &WorkerStats {
        &self.stats
    }
}

async fn process_task(task: Task) -> Result<TaskResult> {
    let start_time = std::time::Instant::now();
    let task_id = task.id.clone();
    let task_type = task.task_type;

    let result = match task_type {
        TaskType::PortScan => process_port_scan(task).await,
        TaskType::ServiceFingerprint => process_fingerprint(task).await,
        TaskType::EndpointDiscovery => process_endpoints(task).await,
        TaskType::Fuzz => process_fuzz(task).await,
        TaskType::WafTest => process_waf(task).await,
        TaskType::LoadTest => process_load_test(task).await,
        TaskType::Recon => process_recon(task).await,
    };

    let duration = start_time.elapsed();
    let success = result.is_ok();
    let output = result
        .as_ref()
        .ok()
        .map(|o| serde_json::to_string(o).unwrap_or_default())
        .unwrap_or_default();
    let error = result.err().map(|e| e.to_string());

    Ok(TaskResult {
        task_id,
        success,
        output,
        error,
        duration_millis: duration.as_millis() as u64,
    })
}

async fn process_port_scan(task: Task) -> Result<serde_json::Value> {
    let target = &task.target;
    let ports = task
        .payload
        .get("ports")
        .and_then(|v| v.as_str())
        .unwrap_or("1-1000");
    let concurrency: usize = task
        .payload
        .get("concurrency")
        .and_then(|v| v.as_u64())
        .unwrap_or(100) as usize;
    let timeout: u64 = task
        .payload
        .get("timeout")
        .and_then(|v| v.as_u64())
        .unwrap_or(5);

    let parsed_ports = crate::utils::parsing::parse_ports(ports)?;

    let results = crate::scanner::ports::scan_ports(
        target,
        parsed_ports,
        concurrency,
        std::time::Duration::from_secs(timeout),
        false,
        crate::scanner::spoof::SpoofConfig::default(),
        None,
    )
    .await?;

    Ok(serde_json::json!({
        "target": target,
        "open_ports": results.open_ports,
        "scan_duration_ms": results.duration_ms,
    }))
}

async fn process_fingerprint(task: Task) -> Result<serde_json::Value> {
    let target = &task.target;
    let ports: Vec<u16> = task
        .payload
        .get("ports")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_u64())
                .map(|p| p as u16)
                .collect()
        })
        .unwrap_or_else(|| {
            vec![
                21, 22, 23, 25, 53, 80, 110, 143, 443, 445, 993, 995, 1433, 1521, 3306, 3389, 5432,
                5900, 6379, 8080, 8443, 27017,
            ]
        });
    let timeout: u64 = task
        .payload
        .get("timeout")
        .and_then(|v| v.as_u64())
        .unwrap_or(5);

    let results = crate::scanner::fingerprint::fingerprint_services(
        target,
        ports,
        std::time::Duration::from_secs(timeout),
        false,
        20,
        None,
    )
    .await?;

    Ok(serde_json::json!({
        "target": target,
        "services": results.results,
    }))
}

async fn process_endpoints(task: Task) -> Result<serde_json::Value> {
    let target = &task.target;
    let wordlist = if let Some(w) = task.payload.get("wordlist").and_then(|v| v.as_str()) {
        tokio::fs::read_to_string(w)
            .await
            .map(|content| content.lines().map(|s| s.to_string()).collect::<Vec<_>>())
            .unwrap_or_default()
    } else {
        vec![
            "/admin".to_string(),
            "/api".to_string(),
            "/login".to_string(),
            "/config".to_string(),
            "/.env".to_string(),
            "/status".to_string(),
            "/health".to_string(),
        ]
    };
    let concurrency: usize = task
        .payload
        .get("concurrency")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;

    let results = crate::scanner::endpoints::scan_endpoints(EndpointScanConfig {
        base_url: target.to_string(),
        endpoints: wordlist,
        concurrency,
        timeout_duration: std::time::Duration::from_secs(10),
        include_404: false,
        tui_mode: false,
        spoof_config: crate::scanner::spoof::SpoofConfig::default(),
        verify_tls: true,
        progress_tx: None,
    })
    .await?;

    Ok(serde_json::json!({
        "target": target,
        "endpoints": results.results,
    }))
}

async fn process_fuzz(task: Task) -> Result<serde_json::Value> {
    let target = &task.target;
    let payload_type = task
        .payload
        .get("payload_type")
        .and_then(|v| v.as_str())
        .unwrap_or("all");
    let concurrency: usize = task
        .payload
        .get("concurrency")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;

    let args = crate::cli::FuzzArgs {
        url: target.to_string(),
        payload_type: payload_type.to_string(),
        mode: crate::cli::FuzzMode::Sequential,
        mutate: false,
        mutation_count: 3,
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
        concurrency,
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
        common: crate::cli::CommonHttpArgs::default(),
    };

    let mut engine = crate::fuzzer::engine::FuzzEngine::new(args)?;
    let session = engine.run_return_session().await?;

    let findings: Vec<_> = session
        .results
        .iter()
        .take(50)
        .map(|f| {
            serde_json::json!({
                "payload": f.payload,
                "severity": f.detected_severity,
                "description": f.payload.description,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "target": target,
        "total_requests": session.successful_requests + session.failed_requests,
        "findings": findings,
    }))
}

async fn process_waf(task: Task) -> Result<serde_json::Value> {
    let target = &task.target;
    let detect_only = task
        .payload
        .get("detect_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let args = crate::cli::WafArgs {
        url: target.to_string(),
        detect_only,
        bypass: !detect_only,
        header_bypass: true,
        smuggling: true,
        evasion: true,
        profile: "auto".to_string(),
        test_type: None,
        concurrency: 10,
        timeout: 15,
        json: false,
        verbose: false,
        quiet: false,
        output: None,
        common: crate::cli::CommonHttpArgs::default(),
    };

    crate::waf::run_cli(args).await?;

    Ok(serde_json::json!({
        "target": target,
        "status": "completed",
    }))
}

async fn process_load_test(task: Task) -> Result<serde_json::Value> {
    let target = &task.target;
    let requests: u64 = task
        .payload
        .get("requests")
        .and_then(|v| v.as_u64())
        .unwrap_or(100);
    let concurrency: usize = task
        .payload
        .get("concurrency")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;
    let method = task
        .payload
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET");

    let args = crate::cli::LoadArgs {
        url: target.to_string(),
        requests,
        concurrency,
        method: method.to_string(),
        body: None,
        headers: Vec::new(),
        timeout: 30,
        json: false,
        verbose: false,
        quiet: false,
        output: None,
        common: crate::cli::CommonHttpArgs::default(),
    };

    let config = crate::config::SlapperConfig::default();
    crate::loadtest::run_cli(args, &config).await?;

    Ok(serde_json::json!({
        "target": target,
        "status": "completed",
    }))
}

async fn process_recon(task: Task) -> Result<serde_json::Value> {
    let target = &task.target;
    let no_tech = task
        .payload
        .get("no_tech")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let no_dns = task
        .payload
        .get("no_dns")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let args = crate::cli::ReconArgs {
        target: target.to_string(),
        no_tech,
        no_dns,
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
        concurrency: Some(10),
        json: false,
        quiet: false,
        verbose: false,
        output: None,
    };

    let config = crate::config::SlapperConfig::default();
    crate::recon::run_cli(args, &config).await?;

    Ok(serde_json::json!({
        "target": target,
        "status": "completed",
    }))
}
