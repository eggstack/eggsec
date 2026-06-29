//! Campaign orchestration module.
//!
//! Supports MITRE ATT&CK profiles, automated campaign runners, and
//! attack graph / timeline generation for purple team exercises.

use super::CampaignPhase;
use serde::{Deserialize, Serialize};

/// List available campaign profiles.
pub fn available_profiles() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "apt29",
            "APT29 (Cozy Bear) simulation with HTTP/S beacons and LOTL techniques",
        ),
        (
            "carbanak",
            "Carbanak/FIN7 simulation with DNS beacons and financial targeting",
        ),
        (
            "default",
            "Generic purple team campaign with mixed C2 protocols",
        ),
    ]
}

/// Get a campaign description by profile name.
pub fn profile_description(profile: &str) -> Option<&'static str> {
    available_profiles()
        .into_iter()
        .find(|(name, _)| *name == profile)
        .map(|(_, desc)| desc)
}

/// Attack graph node representing a single technique in the campaign timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackGraphNode {
    pub technique_id: String,
    pub phase_name: String,
    pub phase_order: u32,
    pub task_type: String,
    pub depends_on: Vec<String>,
}

/// Attack graph representing the full campaign flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackGraph {
    pub campaign_id: String,
    pub campaign_name: String,
    pub nodes: Vec<AttackGraphNode>,
    pub critical_path: Vec<String>,
}

/// Timeline entry representing a single event in the campaign.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub timestamp: String,
    pub phase_order: u32,
    pub phase_name: String,
    pub technique_id: String,
    pub description: String,
    pub depends_on: Vec<String>,
}

/// Full timeline for a campaign.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignTimeline {
    pub campaign_id: String,
    pub campaign_name: String,
    pub entries: Vec<TimelineEntry>,
    pub total_phases: usize,
    pub total_techniques: usize,
}

/// Build an attack graph from campaign phases.
///
/// The graph connects each technique node to the previous phase's techniques,
/// creating a dependency chain that reflects the campaign progression.
pub fn build_attack_graph(
    campaign_id: &str,
    campaign_name: &str,
    phases: &[CampaignPhase],
) -> AttackGraph {
    let mut nodes = Vec::new();
    let mut prev_phase_technique_ids: Vec<String> = Vec::new();

    for phase in phases {
        let mut current_phase_ids = Vec::new();
        for technique in &phase.mitre_technique_ids() {
            let depends_on = prev_phase_technique_ids.clone();
            let node = AttackGraphNode {
                technique_id: technique.to_string(),
                phase_name: phase.name.clone(),
                phase_order: phase.order,
                task_type: super::tasking::task_type_for_technique_static(technique)
                    .as_str()
                    .to_string(),
                depends_on,
            };
            current_phase_ids.push(node.technique_id.clone());
            nodes.push(node);
        }
        prev_phase_technique_ids = current_phase_ids;
    }

    // Critical path: one representative technique per phase (first technique)
    let mut critical_path = Vec::new();
    let mut seen_phases = Vec::new();
    for node in &nodes {
        if !seen_phases.contains(&node.phase_name) {
            seen_phases.push(node.phase_name.clone());
            critical_path.push(node.technique_id.clone());
        }
    }

    AttackGraph {
        campaign_id: campaign_id.to_string(),
        campaign_name: campaign_name.to_string(),
        nodes,
        critical_path,
    }
}

/// Build a campaign timeline from phases with sequential timestamps.
pub fn build_timeline(
    campaign_id: &str,
    campaign_name: &str,
    phases: &[CampaignPhase],
) -> CampaignTimeline {
    let mut entries = Vec::new();
    let mut total_techniques = 0;
    let base_time = chrono::Utc::now();

    for (phase_idx, phase) in phases.iter().enumerate() {
        let techniques = phase.mitre_technique_ids();
        total_techniques += techniques.len();

        for (tech_idx, technique) in techniques.iter().enumerate() {
            let offset_secs = (phase_idx * 60 + tech_idx * 10) as i64;
            let timestamp = base_time
                .checked_add_signed(chrono::Duration::seconds(offset_secs))
                .unwrap_or(base_time)
                .to_rfc3339();

            let depends_on = if tech_idx > 0 {
                vec![techniques[tech_idx - 1].clone()]
            } else if phase_idx > 0 {
                let prev_techniques = phases[phase_idx - 1].mitre_technique_ids();
                if !prev_techniques.is_empty() {
                    vec![prev_techniques.last().unwrap().clone()]
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

            entries.push(TimelineEntry {
                timestamp,
                phase_order: phase.order,
                phase_name: phase.name.clone(),
                technique_id: technique.to_string(),
                description: format!("{}: {}", phase.name, technique),
                depends_on,
            });
        }
    }

    CampaignTimeline {
        campaign_id: campaign_id.to_string(),
        campaign_name: campaign_name.to_string(),
        entries,
        total_phases: phases.len(),
        total_techniques,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_available_profiles() {
        let profiles = available_profiles();
        assert!(profiles.len() >= 3);
        let names: Vec<_> = profiles.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"apt29"));
        assert!(names.contains(&"carbanak"));
        assert!(names.contains(&"default"));
    }

    #[test]
    fn test_profile_description() {
        assert!(profile_description("apt29").is_some());
        assert!(profile_description("carbanak").is_some());
        assert!(profile_description("unknown").is_none());
    }

    #[test]
    fn test_build_attack_graph() {
        let phases = vec![
            CampaignPhase {
                id: "p1".to_string(),
                name: "Recon".to_string(),
                description: "Phase 1".to_string(),
                mitre_techniques: vec!["T1071.001".to_string(), "T1573".to_string()],
                order: 1,
            },
            CampaignPhase {
                id: "p2".to_string(),
                name: "Exploit".to_string(),
                description: "Phase 2".to_string(),
                mitre_techniques: vec!["T1021.002".to_string()],
                order: 2,
            },
        ];
        let graph = build_attack_graph("c1", "Test", &phases);
        assert_eq!(graph.campaign_id, "c1");
        assert_eq!(graph.nodes.len(), 3);
        // Phase 1 nodes have no dependencies
        assert!(graph.nodes[0].depends_on.is_empty());
        assert!(graph.nodes[1].depends_on.is_empty());
        // Phase 2 node depends on both phase 1 techniques
        assert_eq!(graph.nodes[2].depends_on.len(), 2);
        // Critical path has one entry per phase
        assert_eq!(graph.critical_path.len(), 2);
    }

    #[test]
    fn test_build_timeline() {
        let phases = vec![
            CampaignPhase {
                id: "p1".to_string(),
                name: "Phase 1".to_string(),
                description: "Test".to_string(),
                mitre_techniques: vec!["T1071.001".to_string()],
                order: 1,
            },
            CampaignPhase {
                id: "p2".to_string(),
                name: "Phase 2".to_string(),
                description: "Test".to_string(),
                mitre_techniques: vec!["T1041".to_string(), "T1021.002".to_string()],
                order: 2,
            },
        ];
        let timeline = build_timeline("c1", "Test", &phases);
        assert_eq!(timeline.total_phases, 2);
        assert_eq!(timeline.total_techniques, 3);
        assert_eq!(timeline.entries.len(), 3);
        // Phase 2 entries depend on phase 1
        assert_eq!(timeline.entries[1].depends_on, vec!["T1071.001"]);
        assert_eq!(timeline.entries[2].depends_on, vec!["T1041"]);
    }

    #[test]
    fn test_empty_phases() {
        let graph = build_attack_graph("c1", "Empty", &[]);
        assert!(graph.nodes.is_empty());
        assert!(graph.critical_path.is_empty());

        let timeline = build_timeline("c1", "Empty", &[]);
        assert_eq!(timeline.total_phases, 0);
        assert_eq!(timeline.total_techniques, 0);
    }
}
