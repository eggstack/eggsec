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
/// Used when campaigns reference postex techniques (LOTL, lateral movement, etc.).
const POSTEX_LATERAL_TECHNIQUES: &[&str] = &["T1021.002", "T1021.001", "T1021.006", "T1090.002"];
const POSTEX_LOTL_TECHNIQUES: &[&str] = &[
    "T1059.001", "T1047", "T1105", "T1218.011", "T1218.007", "T1218.005", "T1218.010",
];
const POSTEX_CREDENTIAL_TECHNIQUES: &[&str] = &["T1003", "T1555"];
const POSTEX_PERSISTENCE_TECHNIQUES: &[&str] = &["T1547.001", "T1547.002", "T1547.003"];

/// Determine if a technique belongs to a postex category and map it to a C2 task output description.
fn postex_enrichment(technique: &str, phase_name: &str) -> Option<(TaskType, TaskStatus, String)> {
    if POSTEX_LATERAL_TECHNIQUES.contains(&technique) {
        Some((
            TaskType::Lateral,
            TaskStatus::Simulated,
            format!("dry-run: postex lateral movement simulated (phase: {}, technique: {})", phase_name, technique),
        ))
    } else if POSTEX_LOTL_TECHNIQUES.contains(&technique) {
        Some((
            TaskType::Execute,
            TaskStatus::Simulated,
            format!("dry-run: postex LOTL technique simulated (phase: {}, technique: {})", phase_name, technique),
        ))
    } else if POSTEX_CREDENTIAL_TECHNIQUES.contains(&technique) {
        Some((
            TaskType::Recon,
            TaskStatus::Completed,
            format!("dry-run: postex credential access simulated (phase: {}, technique: {})", phase_name, technique),
        ))
    } else if POSTEX_PERSISTENCE_TECHNIQUES.contains(&technique) {
        Some((
            TaskType::Persist,
            TaskStatus::Simulated,
            format!("dry-run: postex persistence mechanism simulated (phase: {}, technique: {})", phase_name, technique),
        ))
    } else {
        None
    }
}

pub fn simulate_tasks(campaign: &C2Campaign) -> Vec<TaskResult> {
    let mut results = Vec::new();

    for phase in &campaign.phases {
        for technique in &phase.mitre_techniques {
            // Try postex enrichment first, then fall back to C2-native mapping
            let (task_type, status, output) = if let Some(enriched) =
                postex_enrichment(technique, &phase.name)
            {
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
                        format!("dry-run: command execution simulated (phase: {})", phase.name),
                    ),
                    "T1053" => (
                        TaskType::Persist,
                        TaskStatus::Simulated,
                        format!("dry-run: scheduled task persistence simulated (phase: {})", phase.name),
                    ),
                    "T1570" | "T1041" => (
                        TaskType::Exfil,
                        TaskStatus::Simulated,
                        format!("dry-run: data exfiltration simulated (phase: {})", phase.name),
                    ),
                    "T1070.006" => (
                        TaskType::Evade,
                        TaskStatus::Simulated,
                        format!("dry-run: log evasion simulated (phase: {})", phase.name),
                    ),
                    "T1547.001" => (
                        TaskType::Persist,
                        TaskStatus::Simulated,
                        format!("dry-run: registry persistence simulated (phase: {})", phase.name),
                    ),
                    "T1565.001" => (
                        TaskType::Execute,
                        TaskStatus::Simulated,
                        format!("dry-run: data manipulation simulated (phase: {})", phase.name),
                    ),
                    "T1001" => (
                        TaskType::Evade,
                        TaskStatus::Simulated,
                        format!("dry-run: traffic obfuscation simulated (phase: {})", phase.name),
                    ),
                    "T1573" | "T1573.002" => (
                        TaskType::Recon,
                        TaskStatus::Simulated,
                        format!("dry-run: encrypted channel simulation (phase: {})", phase.name),
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

    // Always include at least one task
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

    #[test]
    fn test_simulate_tasks_produces_results() {
        let campaign = test_campaign();
        let tasks = simulate_tasks(&campaign);
        assert!(!tasks.is_empty());
    }

    #[test]
    fn test_simulate_tasks_empty_campaign() {
        let campaign = C2Campaign {
            id: "empty".to_string(),
            name: "Empty".to_string(),
            description: "Empty".to_string(),
            mitre_profile: "None".to_string(),
            phases: Vec::new(),
        };
        let tasks = simulate_tasks(&campaign);
        assert_eq!(tasks.len(), 1);
    }

    #[test]
    fn test_task_types_match_techniques() {
        let campaign = test_campaign();
        let tasks = simulate_tasks(&campaign);
        let types: Vec<_> = tasks.iter().map(|t| t.task_type).collect();
        assert!(types.contains(&TaskType::Recon));
        assert!(types.contains(&TaskType::Exfil));
    }

    #[test]
    fn test_postex_lateral_enrichment() {
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
        let tasks = simulate_tasks(&campaign);
        assert!(tasks.iter().all(|t| t.task_type == TaskType::Lateral));
        assert!(tasks.iter().all(|t| t.status == TaskStatus::Simulated));
        assert!(tasks.iter().any(|t| {
            t.output
                .as_ref()
                .map_or(false, |o| o.contains("postex lateral"))
        }));
    }

    #[test]
    fn test_postex_lotl_enrichment() {
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
        let tasks = simulate_tasks(&campaign);
        assert!(tasks.iter().all(|t| t.task_type == TaskType::Execute));
        assert!(tasks.iter().any(|t| {
            t.output
                .as_ref()
                .map_or(false, |o| o.contains("postex LOTL"))
        }));
    }

    #[test]
    fn test_postex_credential_enrichment() {
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
        let tasks = simulate_tasks(&campaign);
        assert!(tasks.iter().all(|t| t.task_type == TaskType::Recon));
        assert!(tasks.iter().all(|t| t.status == TaskStatus::Completed));
        assert!(tasks.iter().any(|t| {
            t.output
                .as_ref()
                .map_or(false, |o| o.contains("postex credential"))
        }));
    }

    #[test]
    fn test_c2_native_vs_postex_fallback() {
        // T1041 is exfil (C2-native), T1021.002 is lateral (postex)
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
        let tasks = simulate_tasks(&campaign);
        let exfil: Vec<_> = tasks.iter().filter(|t| t.task_type == TaskType::Exfil).collect();
        let lateral: Vec<_> = tasks.iter().filter(|t| t.task_type == TaskType::Lateral).collect();
        assert_eq!(exfil.len(), 1);
        assert_eq!(lateral.len(), 1);
    }
}
