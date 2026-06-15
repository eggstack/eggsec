use super::{C2Campaign, TaskResult, TaskStatus, TaskType};

pub fn simulate_tasks(campaign: &C2Campaign) -> Vec<TaskResult> {
    let mut results = Vec::new();

    for phase in &campaign.phases {
        for technique in &phase.mitre_techniques {
            let (task_type, status, output) = match technique.as_str() {
                "T1071" | "T1071.001" | "T1071.004" => (
                    TaskType::Recon,
                    TaskStatus::Completed,
                    Some(format!(
                        "dry-run: network recon completed (phase: {})",
                        phase.name
                    )),
                ),
                "T1059" => (
                    TaskType::Execute,
                    TaskStatus::Simulated,
                    Some(format!(
                        "dry-run: command execution simulated (phase: {})",
                        phase.name
                    )),
                ),
                "T1053" => (
                    TaskType::Persist,
                    TaskStatus::Simulated,
                    Some(format!(
                        "dry-run: scheduled task persistence simulated (phase: {})",
                        phase.name
                    )),
                ),
                "T1003" | "T1555" => (
                    TaskType::Recon,
                    TaskStatus::Completed,
                    Some(format!(
                        "dry-run: credential harvesting simulated (phase: {})",
                        phase.name
                    )),
                ),
                "T1021.002" => (
                    TaskType::Lateral,
                    TaskStatus::Simulated,
                    Some(format!(
                        "dry-run: lateral movement simulated (phase: {})",
                        phase.name
                    )),
                ),
                "T1570" | "T1041" => (
                    TaskType::Exfil,
                    TaskStatus::Simulated,
                    Some(format!(
                        "dry-run: data exfiltration simulated (phase: {})",
                        phase.name
                    )),
                ),
                "T1070.006" => (
                    TaskType::Evade,
                    TaskStatus::Simulated,
                    Some(format!(
                        "dry-run: log evasion simulated (phase: {})",
                        phase.name
                    )),
                ),
                "T1547.001" => (
                    TaskType::Persist,
                    TaskStatus::Simulated,
                    Some(format!(
                        "dry-run: registry persistence simulated (phase: {})",
                        phase.name
                    )),
                ),
                "T1565.001" => (
                    TaskType::Execute,
                    TaskStatus::Simulated,
                    Some(format!(
                        "dry-run: data manipulation simulated (phase: {})",
                        phase.name
                    )),
                ),
                "T1001" => (
                    TaskType::Evade,
                    TaskStatus::Simulated,
                    Some(format!(
                        "dry-run: traffic obfuscation simulated (phase: {})",
                        phase.name
                    )),
                ),
                _ => (
                    TaskType::Execute,
                    TaskStatus::Simulated,
                    Some(format!(
                        "dry-run: generic task simulated (phase: {})",
                        phase.name
                    )),
                ),
            };

            results.push(TaskResult {
                task_type,
                status,
                output,
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
}
