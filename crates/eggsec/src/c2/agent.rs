//! Agent lifecycle management for C2 framework.
//!
//! Handles agent registration, check-in, task dispatch, and self-destruct.
//! Agents represent simulated implant instances that communicate via beacons.

use super::{TaskResult, TaskStatus, TaskType};
use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub beacon_interval_ms: u64,
    pub jitter_percent: u32,
    pub max_checkins: u32,
    pub auto_self_destruct: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "default-agent".to_string(),
            beacon_interval_ms: 60000,
            jitter_percent: 25,
            max_checkins: 100,
            auto_self_destruct: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    Registered,
    Active,
    Idle,
    Compromised,
    SelfDestructed,
    Terminated,
}

impl AgentState {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentState::Registered => "registered",
            AgentState::Active => "active",
            AgentState::Idle => "idle",
            AgentState::Compromised => "compromised",
            AgentState::SelfDestructed => "self-destructed",
            AgentState::Terminated => "terminated",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInstance {
    pub config: AgentConfig,
    pub state: AgentState,
    pub checkin_count: u32,
    pub registered_at: String,
    pub last_checkin: Option<String>,
    pub tasks_completed: u32,
    pub tasks_failed: u32,
}

impl AgentInstance {
    pub fn new(config: AgentConfig) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            config,
            state: AgentState::Registered,
            checkin_count: 0,
            registered_at: now.clone(),
            last_checkin: None,
            tasks_completed: 0,
            tasks_failed: 0,
        }
    }

    pub fn check_in(&mut self) -> Result<AgentCheckinResult> {
        if self.state == AgentState::SelfDestructed || self.state == AgentState::Terminated {
            return Ok(AgentCheckinResult {
                accepted: false,
                reason: Some(format!(
                    "Agent is in terminal state: {}",
                    self.state.as_str()
                )),
                tasks: Vec::new(),
                should_self_destruct: false,
            });
        }

        self.checkin_count += 1;
        self.last_checkin = Some(chrono::Utc::now().to_rfc3339());
        self.state = AgentState::Active;

        let should_self_destruct =
            self.config.auto_self_destruct && self.checkin_count >= self.config.max_checkins;

        if should_self_destruct {
            self.state = AgentState::SelfDestructed;
        }

        Ok(AgentCheckinResult {
            accepted: true,
            reason: None,
            tasks: Vec::new(),
            should_self_destruct,
        })
    }

    pub fn complete_task(&mut self) {
        self.tasks_completed += 1;
    }

    pub fn fail_task(&mut self) {
        self.tasks_failed += 1;
    }

    pub fn terminate(&mut self) {
        self.state = AgentState::Terminated;
    }

    pub fn mark_compromised(&mut self) {
        self.state = AgentState::Compromised;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCheckinResult {
    pub accepted: bool,
    pub reason: Option<String>,
    pub tasks: Vec<super::TaskType>,
    pub should_self_destruct: bool,
}

/// Simulate agent lifecycle for dry-run campaigns.
pub fn simulate_agent_lifecycle(
    config: &AgentConfig,
    campaign_phases: &[super::CampaignPhase],
) -> AgentLifecycleResult {
    let mut agent = AgentInstance::new(config.clone());
    let mut events = Vec::new();
    let mut all_tasks = Vec::new();

    // Phase 1: Registration
    events.push(AgentEvent {
        timestamp: chrono::Utc::now().to_rfc3339(),
        event_type: AgentEventType::Registered,
        detail: format!("Agent '{}' registered with beacon interval {}ms", config.name, config.beacon_interval_ms),
    });

    // Phase 2: Check-ins across campaign phases
    for phase in campaign_phases {
        let checkin_result = agent.check_in().unwrap_or_else(|_| AgentCheckinResult {
            accepted: false,
            reason: Some("check-in failed".to_string()),
            tasks: Vec::new(),
            should_self_destruct: false,
        });

        if checkin_result.accepted {
            events.push(AgentEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: AgentEventType::CheckIn,
                detail: format!("Check-in #{} during phase '{}'", agent.checkin_count, phase.name),
            });

            // Generate tasks for this phase
            for technique in &phase.mitre_techniques {
                let task_type = task_type_for_technique(technique);
                let result = TaskResult {
                    task_type,
                    status: TaskStatus::Simulated,
                    output: Some(format!(
                        "dry-run: {} task simulated in phase '{}' (technique: {})",
                        task_type.as_str(), phase.name, technique
                    )),
                    mitre_technique: Some(technique.clone()),
                };
                all_tasks.push(result);
                agent.complete_task();
            }
        } else {
            events.push(AgentEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: AgentEventType::CheckInFailed,
                detail: checkin_result
                    .reason
                    .unwrap_or_else(|| "unknown reason".to_string()),
            });
        }

        if checkin_result.should_self_destruct {
            events.push(AgentEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: AgentEventType::SelfDestructed,
                detail: format!(
                    "Agent self-destructed after {} check-ins (max: {})",
                    agent.checkin_count, config.max_checkins
                ),
            });
            break;
        }
    }

    // Phase 3: Termination
    if agent.state != AgentState::SelfDestructed && agent.state != AgentState::Terminated {
        agent.terminate();
        events.push(AgentEvent {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event_type: AgentEventType::Terminated,
            detail: "Agent terminated after campaign completion".to_string(),
        });
    }

    AgentLifecycleResult {
        agent,
        events,
        tasks: all_tasks,
    }
}

fn task_type_for_technique(technique: &str) -> TaskType {
    match technique {
        "T1071" | "T1071.001" | "T1071.004" => TaskType::Recon,
        "T1059" | "T1053" => TaskType::Execute,
        "T1003" | "T1555" => TaskType::Recon,
        "T1021.002" | "T1021.001" | "T1021.006" => TaskType::Lateral,
        "T1041" | "T1570" => TaskType::Exfil,
        "T1070.006" => TaskType::Evade,
        "T1547.001" => TaskType::Persist,
        "T1573" | "T1573.002" => TaskType::Recon,
        "T1001" => TaskType::Evade,
        "T1565.001" => TaskType::Execute,
        _ => TaskType::Execute,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLifecycleResult {
    pub agent: AgentInstance,
    pub events: Vec<AgentEvent>,
    pub tasks: Vec<TaskResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub timestamp: String,
    pub event_type: AgentEventType,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentEventType {
    Registered,
    CheckIn,
    CheckInFailed,
    SelfDestructed,
    Terminated,
    Compromised,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert!(!config.id.is_empty());
        assert_eq!(config.beacon_interval_ms, 60000);
        assert_eq!(config.jitter_percent, 25);
        assert!(!config.auto_self_destruct);
    }

    #[test]
    fn test_agent_instance_creation() {
        let config = AgentConfig::default();
        let agent = AgentInstance::new(config);
        assert_eq!(agent.state, AgentState::Registered);
        assert_eq!(agent.checkin_count, 0);
        assert_eq!(agent.tasks_completed, 0);
    }

    #[test]
    fn test_agent_check_in() {
        let config = AgentConfig::default();
        let mut agent = AgentInstance::new(config);
        let result = agent.check_in().unwrap();
        assert!(result.accepted);
        assert_eq!(agent.state, AgentState::Active);
        assert_eq!(agent.checkin_count, 1);
    }

    #[test]
    fn test_agent_self_destruct_after_max_checkins() {
        let config = AgentConfig {
            max_checkins: 2,
            auto_self_destruct: true,
            ..Default::default()
        };
        let mut agent = AgentInstance::new(config);
        agent.check_in().unwrap();
        assert_eq!(agent.state, AgentState::Active);
        let result = agent.check_in().unwrap();
        assert!(result.should_self_destruct);
        assert_eq!(agent.state, AgentState::SelfDestructed);
    }

    #[test]
    fn test_agent_terminal_state_rejects_checkin() {
        let config = AgentConfig::default();
        let mut agent = AgentInstance::new(config);
        agent.terminate();
        let result = agent.check_in().unwrap();
        assert!(!result.accepted);
    }

    #[test]
    fn test_agent_task_tracking() {
        let config = AgentConfig::default();
        let mut agent = AgentInstance::new(config);
        agent.complete_task();
        agent.complete_task();
        agent.fail_task();
        assert_eq!(agent.tasks_completed, 2);
        assert_eq!(agent.tasks_failed, 1);
    }

    #[test]
    fn test_agent_state_as_str() {
        assert_eq!(AgentState::Registered.as_str(), "registered");
        assert_eq!(AgentState::Active.as_str(), "active");
        assert_eq!(AgentState::SelfDestructed.as_str(), "self-destructed");
    }

    #[test]
    fn test_simulate_agent_lifecycle() {
        let config = AgentConfig::default();
        let phases = vec![
            super::super::CampaignPhase {
                id: "p1".to_string(),
                name: "Phase 1".to_string(),
                description: "Test".to_string(),
                mitre_techniques: vec!["T1071.001".to_string(), "T1547.001".to_string()],
                order: 1,
            },
            super::super::CampaignPhase {
                id: "p2".to_string(),
                name: "Phase 2".to_string(),
                description: "Test".to_string(),
                mitre_techniques: vec!["T1041".to_string()],
                order: 2,
            },
        ];
        let result = simulate_agent_lifecycle(&config, &phases);
        assert!(!result.events.is_empty());
        assert!(!result.tasks.is_empty());
        assert_eq!(result.agent.state, AgentState::Terminated);
    }

    #[test]
    fn test_task_type_for_technique() {
        assert_eq!(task_type_for_technique("T1071.001"), TaskType::Recon);
        assert_eq!(task_type_for_technique("T1021.002"), TaskType::Lateral);
        assert_eq!(task_type_for_technique("T1041"), TaskType::Exfil);
        assert_eq!(task_type_for_technique("T1547.001"), TaskType::Persist);
        assert_eq!(task_type_for_technique("T1070.006"), TaskType::Evade);
    }
}
