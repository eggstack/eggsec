use serde::{Deserialize, Serialize};

use crate::request::TaskKind;

/// Capability descriptor for a single task kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCapability {
    pub name: String,
    pub description: String,
}

impl TaskCapability {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

/// Runtime capabilities describing what the runtime supports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeCapabilities {
    pub task_kinds: Vec<TaskCapability>,
    pub transports: Vec<String>,
    pub supports_cancellation: bool,
    pub supports_multiple_sessions: bool,
    pub supports_multiple_active_tasks: bool,
}

/// The default capability set is the **canonical** one: every capability name
/// the task-kind set defines, derived rather than transcribed.
///
/// The capability set is an *advertisement and admission* concern, not an
/// authorisation one. Whether a given operation may run is decided by the
/// enforcement layer from the operation's risk, the operator's execution
/// policy, the loaded scope, and the enabled features. Publishing a hazard
/// family here therefore does not permit it — it only stops the runtime from
/// refusing the submission as an unknown kind.
impl Default for RuntimeCapabilities {
    fn default() -> Self {
        Self::canonical()
    }
}

impl RuntimeCapabilities {
    /// The canonical capability list, derived from the task-kind set.
    ///
    /// Every canonical capability name appears exactly once — `resume`
    /// included — because the list is generated from
    /// [`TaskKind::all_variants`] rather than transcribed. The published
    /// descriptions come from the canonical description table, which is keyed
    /// by capability name and covers exactly this name set.
    pub fn canonical() -> Self {
        let task_kinds = TaskKind::all_variants()
            .iter()
            .map(|kind| TaskCapability::new(kind.capability_name(), kind.capability_description()))
            .collect();
        Self::with_task_kinds(task_kinds)
    }

    /// A capability set carrying exactly the given task kinds.
    ///
    /// The transports and the session/task flags are the standard ones; only
    /// the task-kind list varies. This is how a host makes its **published**
    /// set and the runtime's **enforced** set the same object: the published
    /// list is computed first, then handed to the runtime as its enforced
    /// set.
    pub fn with_task_kinds(task_kinds: Vec<TaskCapability>) -> Self {
        Self {
            task_kinds,
            transports: vec!["in-process".into()],
            supports_cancellation: true,
            supports_multiple_sessions: true,
            supports_multiple_active_tasks: false,
        }
    }

    /// Create capabilities for a no-op executor (no task kinds).
    pub fn noop() -> Self {
        Self::with_task_kinds(Vec::new())
    }

    /// Check if a task kind is supported by these capabilities.
    ///
    /// The name lookup delegates to [`TaskKind::capability_name`], the single
    /// canonical name table — there is no second copy to drift.
    pub fn supports_task_kind(&self, kind: &TaskKind) -> bool {
        let name = kind.capability_name();
        self.task_kinds.iter().any(|c| c.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::{CAPABILITY_DESCRIPTIONS, *};

    fn canonical_names() -> Vec<String> {
        RuntimeCapabilities::canonical()
            .task_kinds
            .iter()
            .map(|c| c.name.clone())
            .collect()
    }

    #[test]
    fn capabilities_roundtrip() {
        let caps = RuntimeCapabilities::canonical();
        let json = serde_json::to_string(&caps).unwrap();
        let deserialized: RuntimeCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(caps.task_kinds.len(), deserialized.task_kinds.len());
        assert!(deserialized.supports_cancellation);
    }

    #[test]
    fn default_capabilities_are_the_canonical_set() {
        assert_eq!(
            RuntimeCapabilities::default().task_kinds.len(),
            RuntimeCapabilities::canonical().task_kinds.len()
        );
    }

    #[test]
    fn default_capabilities_have_task_kinds() {
        let caps = RuntimeCapabilities::default();
        assert!(!caps.task_kinds.is_empty());
        let names: Vec<_> = caps.task_kinds.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"port-scan"));
        assert!(names.contains(&"load-test"));
    }

    #[test]
    fn default_capabilities_single_active_task() {
        let caps = RuntimeCapabilities::default();
        assert!(!caps.supports_multiple_active_tasks);
    }

    #[test]
    fn default_capabilities_no_unimplemented_transports() {
        let caps = RuntimeCapabilities::default();
        assert_eq!(caps.transports, vec!["in-process"]);
    }

    #[test]
    fn default_capabilities_sessions_supported() {
        let caps = RuntimeCapabilities::default();
        assert!(caps.supports_multiple_sessions);
    }

    /// The derived list must cover every canonical capability name. The
    /// enumeration is generated from the canonical task-kind table, so this
    /// holds by construction — the test is the machine-checked backstop for
    /// the guard that pins the derivation.
    #[test]
    fn canonical_covers_every_task_kind() {
        let caps = RuntimeCapabilities::canonical();
        let variants = TaskKind::all_variants();
        assert_eq!(
            caps.task_kinds.len(),
            variants.len(),
            "the derived list must have one entry per task kind"
        );
        for kind in &variants {
            assert!(
                caps.supports_task_kind(kind),
                "canonical list is missing {:?}",
                kind.capability_name()
            );
        }
    }

    #[test]
    fn canonical_names_are_unique() {
        let names = canonical_names();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), names.len(), "duplicate capability names");
    }

    #[test]
    fn canonical_includes_resume() {
        let names = canonical_names();
        assert!(
            names.contains(&"resume".to_string()),
            "resume must be advertised like every other canonical kind"
        );
    }

    /// The hazard families are *offered*, not *permitted*: publishing them
    /// stops the runtime refusing the submission as an unknown kind. The
    /// operator's execution policy still decides whether they may run.
    #[test]
    fn canonical_includes_the_hazard_families() {
        let names = canonical_names();
        for expected in &[
            "stress-test",
            "packet-send",
            "wireless",
            "wireless-active",
            "db-pentest",
            "intercept",
            "c2",
            "packet-capture",
            "traceroute",
        ] {
            assert!(
                names.contains(&expected.to_string()),
                "hazard family should be advertised: {expected}"
            );
        }
    }

    /// The description table is data, not identity: it is keyed by capability
    /// name and must cover exactly the derived name set — no more, no fewer.
    #[test]
    fn description_table_covers_exactly_the_canonical_names() {
        let derived: Vec<String> = canonical_names();
        let derived: Vec<&str> = derived.iter().map(|n| n.as_str()).collect();
        let described: Vec<&str> = CAPABILITY_DESCRIPTIONS.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            derived.len(),
            described.len(),
            "description table must cover exactly the derived name set"
        );
        for name in &derived {
            assert!(
                described.contains(name),
                "description table is missing {name}"
            );
        }
        for name in &described {
            assert!(
                derived.contains(name),
                "description table has an extra name {name}"
            );
        }
    }

    #[test]
    fn every_canonical_entry_has_a_description() {
        for capability in &RuntimeCapabilities::canonical().task_kinds {
            assert!(
                !capability.description.trim().is_empty(),
                "capability {} has no description",
                capability.name
            );
        }
    }

    /// The name lookup and the canonical name table must not diverge.
    #[test]
    fn supports_task_kind_agrees_with_capability_name() {
        let caps = RuntimeCapabilities::canonical();
        for kind in TaskKind::all_variants() {
            let name = kind.capability_name();
            assert!(
                caps.task_kinds.iter().any(|c| c.name == name),
                "{name} is not in the canonical list"
            );
        }
    }

    #[test]
    fn supports_task_kind_positive() {
        let caps = RuntimeCapabilities::canonical();
        assert!(caps.supports_task_kind(&TaskKind::PortScan(PortScanParams {
            target: "10.0.0.1".into(),
            ports: None,
            scan_type: None,
            timeout_ms: None,
            concurrency: None,
            udp: None,
        })));
        assert!(caps.supports_task_kind(&TaskKind::Recon(ReconParams {
            target: "example.com".into(),
            modules: None,
        })));
    }

    /// A runtime configured with a narrower set still refuses the kinds
    /// outside it. The canonical list is the default, not the only option.
    #[test]
    fn a_narrow_set_rejects_the_kinds_outside_it() {
        let narrow = RuntimeCapabilities {
            task_kinds: vec![
                TaskCapability::new("port-scan", "Port scanning"),
                TaskCapability::new("load-test", "HTTP load testing"),
            ],
            transports: vec!["in-process".into()],
            supports_cancellation: true,
            supports_multiple_sessions: true,
            supports_multiple_active_tasks: false,
        };
        assert!(
            narrow.supports_task_kind(&TaskKind::PortScan(PortScanParams {
                target: "10.0.0.1".into(),
                ports: None,
                scan_type: None,
                timeout_ms: None,
                concurrency: None,
                udp: None,
            }))
        );
        assert!(
            !narrow.supports_task_kind(&TaskKind::StressTest(StressTestParams {
                target: "10.0.0.1".into(),
                flood_type: "syn".into(),
                duration_secs: None,
                threads: None,
                rate_pps: None,
            }))
        );
        assert!(
            !narrow.supports_task_kind(&TaskKind::Wireless(WirelessParams {
                interface: None,
                duration_secs: None,
            }))
        );
    }

    #[test]
    fn noop_capabilities_are_empty() {
        let caps = RuntimeCapabilities::noop();
        assert!(caps.task_kinds.is_empty());
        assert!(
            !caps.supports_task_kind(&TaskKind::PortScan(PortScanParams {
                target: "10.0.0.1".into(),
                ports: None,
                scan_type: None,
                timeout_ms: None,
                concurrency: None,
                udp: None,
            }))
        );
    }
}
