use super::{C2Campaign, TaskResult, TaskStatus, TaskType};

/// Map a MITRE technique ID to a TaskType for static (non-phase) contexts.
pub fn task_type_for_technique_static(technique: &str) -> TaskType {
    match technique {
        "T1071" | "T1071.001" | "T1071.004" | "T1573" | "T1573.002" => TaskType::Recon,
        "T1003" | "T1555" => TaskType::Recon,
        "T1021.002" | "T1021.001" | "T1021.006" => TaskType::Lateral,
        "T1570" | "T1041" => TaskType::Exfil,
        "T1070.006" => TaskType::Evade,
        "T1547.001" | "T1547.002" | "T1547.003" => TaskType::Persist,
        "T1059" | "T1053" | "T1565.001" => TaskType::Execute,
        _ => TaskType::Execute,
    }
}

/// Postex categories and their associated MITRE techniques that map to C2 task types.
const POSTEX_LATERAL_TECHNIQUES: &[&str] = &["T1021.002", "T1021.001", "T1021.006", "T1090.002"];
const POSTEX_LOTL_TECHNIQUES: &[&str] = &[
    "T1059.001",
    "T1047",
    "T1105",
    "T1218.011",
    "T1218.007",
    "T1218.005",
    "T1218.010",
];
const POSTEX_CREDENTIAL_TECHNIQUES: &[&str] = &["T1003", "T1555"];
const POSTEX_PERSISTENCE_TECHNIQUES: &[&str] = &["T1547.001", "T1547.002", "T1547.003"];

/// Determine if a technique belongs to a postex category and map it to a C2 task output description.
fn postex_enrichment(technique: &str, phase_name: &str) -> Option<(TaskType, TaskStatus, String)> {
    if POSTEX_LATERAL_TECHNIQUES.contains(&technique) {
        Some((
            TaskType::Lateral,
            TaskStatus::Simulated,
            format!(
                "dry-run: postex lateral movement simulated (phase: {}, technique: {})",
                phase_name, technique
            ),
        ))
    } else if POSTEX_LOTL_TECHNIQUES.contains(&technique) {
        Some((
            TaskType::Execute,
            TaskStatus::Simulated,
            format!(
                "dry-run: postex LOTL technique simulated (phase: {}, technique: {})",
                phase_name, technique
            ),
        ))
    } else if POSTEX_CREDENTIAL_TECHNIQUES.contains(&technique) {
        Some((
            TaskType::Recon,
            TaskStatus::Completed,
            format!(
                "dry-run: postex credential access simulated (phase: {}, technique: {})",
                phase_name, technique
            ),
        ))
    } else if POSTEX_PERSISTENCE_TECHNIQUES.contains(&technique) {
        Some((
            TaskType::Persist,
            TaskStatus::Simulated,
            format!(
                "dry-run: postex persistence mechanism simulated (phase: {}, technique: {})",
                phase_name, technique
            ),
        ))
    } else {
        None
    }
}

/// Produce dry-run synthetic task results (no I/O).
fn dry_run_tasks(campaign: &C2Campaign) -> Vec<TaskResult> {
    let mut results = Vec::new();

    for phase in &campaign.phases {
        for technique in &phase.mitre_techniques {
            let (task_type, status, output) =
                if let Some(enriched) = postex_enrichment(technique, &phase.name) {
                    enriched
                } else {
                    match technique.as_str() {
                        "T1071" | "T1071.001" | "T1071.004" => (
                            TaskType::Recon,
                            TaskStatus::Completed,
                            format!("dry-run: network recon completed (phase: {})", phase.name),
                        ),
                        "T1059" => (
                            TaskType::Execute,
                            TaskStatus::Simulated,
                            format!(
                                "dry-run: command execution simulated (phase: {})",
                                phase.name
                            ),
                        ),
                        "T1053" => (
                            TaskType::Persist,
                            TaskStatus::Simulated,
                            format!(
                                "dry-run: scheduled task persistence simulated (phase: {})",
                                phase.name
                            ),
                        ),
                        "T1570" | "T1041" => (
                            TaskType::Exfil,
                            TaskStatus::Simulated,
                            format!(
                                "dry-run: data exfiltration simulated (phase: {})",
                                phase.name
                            ),
                        ),
                        "T1070.006" => (
                            TaskType::Evade,
                            TaskStatus::Simulated,
                            format!("dry-run: log evasion simulated (phase: {})", phase.name),
                        ),
                        "T1547.001" => (
                            TaskType::Persist,
                            TaskStatus::Simulated,
                            format!(
                                "dry-run: registry persistence simulated (phase: {})",
                                phase.name
                            ),
                        ),
                        "T1565.001" => (
                            TaskType::Execute,
                            TaskStatus::Simulated,
                            format!(
                                "dry-run: data manipulation simulated (phase: {})",
                                phase.name
                            ),
                        ),
                        "T1001" => (
                            TaskType::Evade,
                            TaskStatus::Simulated,
                            format!(
                                "dry-run: traffic obfuscation simulated (phase: {})",
                                phase.name
                            ),
                        ),
                        "T1573" | "T1573.002" => (
                            TaskType::Recon,
                            TaskStatus::Simulated,
                            format!(
                                "dry-run: encrypted channel simulation (phase: {})",
                                phase.name
                            ),
                        ),
                        _ => (
                            TaskType::Execute,
                            TaskStatus::Simulated,
                            format!("dry-run: generic task simulated (phase: {})", phase.name),
                        ),
                    }
                };

            results.push(TaskResult {
                task_type,
                status,
                output: Some(output),
                mitre_technique: Some(technique.clone()),
            });
        }
    }

    if results.is_empty() {
        results.push(TaskResult {
            task_type: TaskType::Recon,
            status: TaskStatus::Simulated,
            output: Some("dry-run: default recon task simulated".to_string()),
            mitre_technique: Some("T1071".to_string()),
        });
    }

    results
}

/// TCP connect scan a single port with timeout.
async fn tcp_connect_scan(host: &str, port: u16) -> (bool, u64) {
    let start = std::time::Instant::now();
    let addr = format!("{}:{}", host, port);
    match tokio::time::timeout(
        std::time::Duration::from_secs(3),
        tokio::net::TcpStream::connect(&addr),
    )
    .await
    {
        Ok(Ok(_)) => (true, start.elapsed().as_millis() as u64),
        _ => (false, start.elapsed().as_millis() as u64),
    }
}

/// Perform a real recon task: TCP connect scan of common C2-related ports.
async fn real_recon_task(target: &str, technique: &str, _phase_name: &str) -> TaskResult {
    let ports = [80, 443, 8443, 8080, 4443];
    let mut open_ports = Vec::new();

    for &port in &ports {
        let (open, _latency) = tcp_connect_scan(target, port).await;
        if open {
            open_ports.push(port);
        }
    }

    let output = if open_ports.is_empty() {
        format!(
            "real: recon against {} — no open ports found in [{}]",
            target,
            ports
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        format!(
            "real: recon against {} — open ports: {}",
            target,
            open_ports
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    TaskResult {
        task_type: TaskType::Recon,
        status: if open_ports.is_empty() {
            TaskStatus::Failed
        } else {
            TaskStatus::Completed
        },
        output: Some(output),
        mitre_technique: Some(technique.to_string()),
    }
}

/// Perform a real execute task: send HTTP POST to target simulating task delivery.
async fn real_execute_task(target: &str, technique: &str, phase_name: &str) -> TaskResult {
    let url = format!("http://{}/task/execute", target);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(e) => {
            return TaskResult {
                task_type: TaskType::Execute,
                status: TaskStatus::Failed,
                output: Some(format!("real: failed to build HTTP client: {}", e)),
                mitre_technique: Some(technique.to_string()),
            };
        }
    };

    match client
        .post(&url)
        .body(format!("phase={},technique={}", phase_name, technique))
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status().as_u16();
            TaskResult {
                task_type: TaskType::Execute,
                status: if resp.status().is_success() || resp.status().is_client_error() {
                    TaskStatus::Completed
                } else {
                    TaskStatus::Failed
                },
                output: Some(format!(
                    "real: execute task to {} returned HTTP {}",
                    url, status
                )),
                mitre_technique: Some(technique.to_string()),
            }
        }
        Err(e) => TaskResult {
            task_type: TaskType::Execute,
            status: TaskStatus::Failed,
            output: Some(format!("real: execute task to {} failed: {}", url, e)),
            mitre_technique: Some(technique.to_string()),
        },
    }
}

/// Perform a real exfil task: send small test payload via HTTP POST.
async fn real_exfil_task(target: &str, technique: &str, _phase_name: &str) -> TaskResult {
    let url = format!("http://{}/data/exfil", target);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(e) => {
            return TaskResult {
                task_type: TaskType::Exfil,
                status: TaskStatus::Failed,
                output: Some(format!("real: failed to build HTTP client: {}", e)),
                mitre_technique: Some(technique.to_string()),
            };
        }
    };

    // 1KB test payload
    let payload = vec![0xABu8; 1024];

    match client.post(&url).body(payload).send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            TaskResult {
                task_type: TaskType::Exfil,
                status: if resp.status().is_success() || resp.status().is_client_error() {
                    TaskStatus::Completed
                } else {
                    TaskStatus::Failed
                },
                output: Some(format!(
                    "real: exfil task to {} returned HTTP {} (1KB payload)",
                    url, status
                )),
                mitre_technique: Some(technique.to_string()),
            }
        }
        Err(e) => TaskResult {
            task_type: TaskType::Exfil,
            status: TaskStatus::Failed,
            output: Some(format!("real: exfil task to {} failed: {}", url, e)),
            mitre_technique: Some(technique.to_string()),
        },
    }
}

/// Perform a real evade task: send decoy HTTP traffic.
async fn real_evade_task(target: &str, technique: &str) -> TaskResult {
    let url = format!(
        "http://{}/decoy/{}",
        target,
        technique.to_lowercase().replace('.', "-")
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(e) => {
            return TaskResult {
                task_type: TaskType::Evade,
                status: TaskStatus::Failed,
                output: Some(format!("real: failed to build HTTP client: {}", e)),
                mitre_technique: Some(technique.to_string()),
            };
        }
    };

    match client.get(&url).send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            TaskResult {
                task_type: TaskType::Evade,
                status: TaskStatus::Completed,
                output: Some(format!(
                    "real: decoy traffic to {} returned HTTP {}",
                    url, status
                )),
                mitre_technique: Some(technique.to_string()),
            }
        }
        Err(e) => TaskResult {
            task_type: TaskType::Evade,
            status: TaskStatus::Failed,
            output: Some(format!("real: decoy traffic to {} failed: {}", url, e)),
            mitre_technique: Some(technique.to_string()),
        },
    }
}

/// Perform a real persist task: evidence-only (persistence mechanisms are out of scope for network C2).
fn real_persist_task(technique: &str, phase_name: &str) -> TaskResult {
    TaskResult {
        task_type: TaskType::Persist,
        status: TaskStatus::Simulated,
        output: Some(format!(
            "real: persistence mechanism {} noted (phase: {}; actual implementation out of scope for network C2)",
            technique, phase_name
        )),
        mitre_technique: Some(technique.to_string()),
    }
}

/// Produce real task results with actual network I/O against the target.
///
/// All operations have timeouts and never panic. Connection failures produce
/// `TaskStatus::Failed` with descriptive error evidence.
async fn real_tasks(campaign: &C2Campaign, target: &str) -> Vec<TaskResult> {
    let mut results = Vec::new();

    for phase in &campaign.phases {
        for technique in &phase.mitre_techniques {
            // Postex enrichment still uses dry-run messaging (real postex primitives are out of scope)
            if let Some(enriched) = postex_enrichment(technique, &phase.name) {
                results.push(TaskResult {
                    task_type: enriched.0,
                    status: enriched.1,
                    output: Some(enriched.2),
                    mitre_technique: Some(technique.clone()),
                });
                continue;
            }

            let task = match technique.as_str() {
                "T1071" | "T1071.001" | "T1071.004" => {
                    real_recon_task(target, technique, &phase.name).await
                }
                "T1059" => real_execute_task(target, technique, &phase.name).await,
                "T1053" => real_persist_task(technique, &phase.name),
                "T1570" | "T1041" => real_exfil_task(target, technique, &phase.name).await,
                "T1070.006" => real_evade_task(target, technique).await,
                "T1547.001" => real_persist_task(technique, &phase.name),
                "T1565.001" => real_execute_task(target, technique, &phase.name).await,
                "T1001" => real_evade_task(target, technique).await,
                "T1573" | "T1573.002" => real_recon_task(target, technique, &phase.name).await,
                _ => real_execute_task(target, technique, &phase.name).await,
            };

            results.push(task);
        }
    }

    if results.is_empty() {
        results.push(TaskResult {
            task_type: TaskType::Recon,
            status: TaskStatus::Simulated,
            output: Some("real: default recon task (no campaign phases defined)".to_string()),
            mitre_technique: Some("T1071".to_string()),
        });
    }

    results
}

/// Simulate C2 tasks. Produces dry-run synthetic results or real network I/O
/// depending on the `dry_run` flag.
pub async fn simulate_tasks(campaign: &C2Campaign, target: &str, dry_run: bool) -> Vec<TaskResult> {
    if dry_run {
        dry_run_tasks(campaign)
    } else {
        real_tasks(campaign, target).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c2::CampaignPhase;

    fn test_campaign() -> C2Campaign {
        C2Campaign {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            mitre_profile: "Test".to_string(),
            phases: vec![CampaignPhase {
                id: "p1".to_string(),
                name: "Phase 1".to_string(),
                description: "Test phase".to_string(),
                mitre_techniques: vec!["T1071.001".to_string(), "T1570".to_string()],
                order: 1,
            }],
        }
    }

    #[tokio::test]
    async fn test_dry_run_tasks_produces_results() {
        let campaign = test_campaign();
        let tasks = simulate_tasks(&campaign, "localhost", true).await;
        assert!(!tasks.is_empty());
    }

    #[tokio::test]
    async fn test_dry_run_tasks_empty_campaign() {
        let campaign = C2Campaign {
            id: "empty".to_string(),
            name: "Empty".to_string(),
            description: "Empty".to_string(),
            mitre_profile: "None".to_string(),
            phases: Vec::new(),
        };
        let tasks = simulate_tasks(&campaign, "localhost", true).await;
        assert_eq!(tasks.len(), 1);
    }

    #[tokio::test]
    async fn test_dry_run_task_types_match_techniques() {
        let campaign = test_campaign();
        let tasks = simulate_tasks(&campaign, "localhost", true).await;
        let types: Vec<_> = tasks.iter().map(|t| t.task_type).collect();
        assert!(types.contains(&TaskType::Recon));
        assert!(types.contains(&TaskType::Exfil));
    }

    #[tokio::test]
    async fn test_postex_lateral_enrichment() {
        let campaign = C2Campaign {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            mitre_profile: "Test".to_string(),
            phases: vec![CampaignPhase {
                id: "p1".to_string(),
                name: "Lateral Phase".to_string(),
                description: "Test".to_string(),
                mitre_techniques: vec!["T1021.002".to_string(), "T1021.006".to_string()],
                order: 1,
            }],
        };
        let tasks = simulate_tasks(&campaign, "localhost", true).await;
        assert!(tasks.iter().all(|t| t.task_type == TaskType::Lateral));
        assert!(tasks.iter().all(|t| t.status == TaskStatus::Simulated));
        assert!(tasks.iter().any(|t| {
            t.output
                .as_ref()
                .map_or(false, |o| o.contains("postex lateral"))
        }));
    }

    #[tokio::test]
    async fn test_postex_lotl_enrichment() {
        let campaign = C2Campaign {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            mitre_profile: "Test".to_string(),
            phases: vec![CampaignPhase {
                id: "p1".to_string(),
                name: "LOTL Phase".to_string(),
                description: "Test".to_string(),
                mitre_techniques: vec!["T1218.011".to_string(), "T1105".to_string()],
                order: 1,
            }],
        };
        let tasks = simulate_tasks(&campaign, "localhost", true).await;
        assert!(tasks.iter().all(|t| t.task_type == TaskType::Execute));
        assert!(tasks.iter().any(|t| {
            t.output
                .as_ref()
                .map_or(false, |o| o.contains("postex LOTL"))
        }));
    }

    #[tokio::test]
    async fn test_postex_credential_enrichment() {
        let campaign = C2Campaign {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            mitre_profile: "Test".to_string(),
            phases: vec![CampaignPhase {
                id: "p1".to_string(),
                name: "Cred Phase".to_string(),
                description: "Test".to_string(),
                mitre_techniques: vec!["T1003".to_string(), "T1555".to_string()],
                order: 1,
            }],
        };
        let tasks = simulate_tasks(&campaign, "localhost", true).await;
        assert!(tasks.iter().all(|t| t.task_type == TaskType::Recon));
        assert!(tasks.iter().all(|t| t.status == TaskStatus::Completed));
        assert!(tasks.iter().any(|t| {
            t.output
                .as_ref()
                .map_or(false, |o| o.contains("postex credential"))
        }));
    }

    #[tokio::test]
    async fn test_c2_native_vs_postex_fallback() {
        let campaign = C2Campaign {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            mitre_profile: "Test".to_string(),
            phases: vec![CampaignPhase {
                id: "p1".to_string(),
                name: "Mixed".to_string(),
                description: "Test".to_string(),
                mitre_techniques: vec!["T1041".to_string(), "T1021.002".to_string()],
                order: 1,
            }],
        };
        let tasks = simulate_tasks(&campaign, "localhost", true).await;
        let exfil: Vec<_> = tasks
            .iter()
            .filter(|t| t.task_type == TaskType::Exfil)
            .collect();
        let lateral: Vec<_> = tasks
            .iter()
            .filter(|t| t.task_type == TaskType::Lateral)
            .collect();
        assert_eq!(exfil.len(), 1);
        assert_eq!(lateral.len(), 1);
    }

    #[tokio::test]
    async fn test_real_recon_task_closed_port() {
        // Port 1 on loopback is almost always filtered/closed
        let task = real_recon_task("127.0.0.1:1", "T1071.001", "Test Phase").await;
        assert_eq!(task.task_type, TaskType::Recon);
        assert_eq!(task.status, TaskStatus::Failed);
        let output = task.output.unwrap();
        assert!(output.starts_with("real:"));
        assert!(!output.contains("dry-run"));
    }

    #[tokio::test]
    async fn test_real_execute_task_unreachable() {
        let task = real_execute_task("127.0.0.1:1", "T1059", "Test Phase").await;
        assert_eq!(task.task_type, TaskType::Execute);
        assert_eq!(task.status, TaskStatus::Failed);
        let output = task.output.unwrap();
        assert!(output.starts_with("real:"));
    }

    #[tokio::test]
    async fn test_real_exfil_task_unreachable() {
        let task = real_exfil_task("127.0.0.1:1", "T1041", "Test Phase").await;
        assert_eq!(task.task_type, TaskType::Exfil);
        assert_eq!(task.status, TaskStatus::Failed);
    }

    #[tokio::test]
    async fn test_real_evade_task_unreachable() {
        let task = real_evade_task("127.0.0.1:1", "T1070.006").await;
        assert_eq!(task.task_type, TaskType::Evade);
        assert_eq!(task.status, TaskStatus::Failed);
    }

    #[test]
    fn test_real_persist_task_always_simulated() {
        let task = real_persist_task("T1547.001", "Test");
        assert_eq!(task.task_type, TaskType::Persist);
        assert_eq!(task.status, TaskStatus::Simulated);
        assert!(task.output.unwrap().contains("real:"));
    }

    #[test]
    fn test_task_type_for_technique_static() {
        assert_eq!(task_type_for_technique_static("T1071.001"), TaskType::Recon);
        assert_eq!(
            task_type_for_technique_static("T1021.002"),
            TaskType::Lateral
        );
        assert_eq!(task_type_for_technique_static("T1041"), TaskType::Exfil);
        assert_eq!(
            task_type_for_technique_static("T1547.001"),
            TaskType::Persist
        );
        assert_eq!(task_type_for_technique_static("T1070.006"), TaskType::Evade);
    }
}
