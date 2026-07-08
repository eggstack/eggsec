use eggsec_runtime::event::{
    ArtifactRef, RuntimeEvent, TaskOutcome, TaskProgress, TaskResultEnvelope, TaskStatus,
};
use eggsec_runtime::ids::{SessionId, TaskId};
use eggsec_runtime::request::{PortScanParams, RuntimeSurface, TaskKind};
use eggsec_runtime::session::{SessionScope, SessionSnapshot, SessionSummary, TaskSnapshot};
use eggsec_ui_model::*;

#[test]
fn session_summary_view_roundtrip() {
    let summary = SessionSummary {
        session_id: SessionId::new(),
        surface: RuntimeSurface::TuiManual,
        scope: Some(SessionScope {
            is_explicit: true,
            source: "config".into(),
            path: None,
        }),
        active_count: 2,
        completed_count: 5,
        created_at_epoch_secs: 100,
        owner_client_id: None,
    };
    let view = SessionSummaryView::from(&summary);
    assert_eq!(view.active_count, 2);
    assert_eq!(view.completed_count, 5);
    assert!(view.has_explicit_scope);
    assert_eq!(view.scope_source.as_deref(), Some("config"));
    let json = serde_json::to_string(&view).unwrap();
    let _: SessionSummaryView = serde_json::from_str(&json).unwrap();
}

#[test]
fn session_view_roundtrip() {
    let snapshot = SessionSnapshot {
        session_id: SessionId::new(),
        surface: RuntimeSurface::McpServer,
        scope: None,
        created_at_epoch_secs: 42,
        generation: 3,
        active_tasks: vec![],
        completed_tasks: vec![],
        capabilities: Default::default(),
        closed: false,
        closed_at: None,
        owner_client_id: None,
    };
    let view = SessionView::from(&snapshot);
    assert_eq!(view.surface_label, "mcp-server");
    assert_eq!(view.generation, 3);
    let json = serde_json::to_string(&view).unwrap();
    let _: SessionView = serde_json::from_str(&json).unwrap();
}

#[test]
fn task_view_roundtrip() {
    let task = TaskSnapshot {
        task_id: TaskId::new(),
        status: TaskStatus::Completed,
        task_kind: TaskKind::PortScan(PortScanParams {
            target: "10.0.0.1".into(),
            ports: None,
            scan_type: None,
            timeout_ms: None,
        }),
        request_summary: "port-scan: 10.0.0.1".into(),
        progress: None,
        last_error: None,
        outcome: None,
    };
    let view = TaskView::from(&task);
    assert_eq!(view.status_label, "Completed");
    assert_eq!(view.task_kind_label, "Port Scan");
    assert!(!view.has_outcome);
    let json = serde_json::to_string(&view).unwrap();
    let _: TaskView = serde_json::from_str(&json).unwrap();
}

#[test]
fn task_progress_view_percentage() {
    let progress = TaskProgress {
        completed: 50,
        total: Some(100),
        message: None,
    };
    let view = TaskProgressView::from(&progress);
    assert_eq!(view.percentage, Some(50.0));

    let progress_no_total = TaskProgress {
        completed: 50,
        total: None,
        message: None,
    };
    let view2 = TaskProgressView::from(&progress_no_total);
    assert_eq!(view2.percentage, None);
}

#[test]
fn result_envelope_view_roundtrip() {
    let env = TaskResultEnvelope {
        kind: "port-scan".into(),
        summary: Some("Found 3 open ports".into()),
        payload: serde_json::json!({"open_ports": [80, 443, 8080]}),
        artifacts: vec![ArtifactRef {
            id: "art1".into(),
            kind: "scan-report".into(),
            path: Some("/tmp/report.json".into()),
            mime_type: Some("application/json".into()),
            summary: None,
        }],
    };
    let view = ResultEnvelopeView::from(&env);
    assert_eq!(view.kind_label, "Port Scan");
    assert!(view.supports_rich_tui);
    assert!(view.supports_json_detail);
    assert_eq!(view.artifact_count, 1);
    let json = serde_json::to_string(&view).unwrap();
    let _: ResultEnvelopeView = serde_json::from_str(&json).unwrap();
}

#[test]
fn unknown_result_kind_degrades_gracefully() {
    let env = TaskResultEnvelope {
        kind: "unknown-kind".into(),
        summary: None,
        payload: serde_json::json!({"data": "test"}),
        artifacts: vec![],
    };
    let view = ResultEnvelopeView::from(&env);
    assert_eq!(view.kind_label, "Unknown");
    assert!(!view.supports_rich_tui);
    assert!(view.supports_json_detail);
}

#[test]
fn artifact_view_roundtrip() {
    let artifact = ArtifactRef {
        id: "a1".into(),
        kind: "pcap".into(),
        path: Some("/tmp/capture.pcap".into()),
        mime_type: Some("application/vnd.tcpdump.pcap".into()),
        summary: Some("100 packets".into()),
    };
    let view = ArtifactView::from(&artifact);
    assert_eq!(view.id, "a1");
    assert_eq!(view.kind, "pcap");
    let json = serde_json::to_string(&view).unwrap();
    let _: ArtifactView = serde_json::from_str(&json).unwrap();
}

#[test]
fn permission_view_roles() {
    let owner = ClientRoleView::owner();
    assert!(owner.can_submit);
    assert!(owner.can_cancel);
    assert!(owner.can_close);
    assert!(owner.can_approve_policy);

    let controller = ClientRoleView::controller();
    assert!(controller.can_submit);
    assert!(controller.can_cancel);
    assert!(!controller.can_close);
    assert!(!controller.can_approve_policy);

    let observer = ClientRoleView::observer();
    assert!(!observer.can_submit);
    assert!(!observer.can_cancel);
    assert!(!observer.can_close);
    assert!(!observer.can_approve_policy);

    let approver = ClientRoleView::approver();
    assert!(!approver.can_submit);
    assert!(!approver.can_cancel);
    assert!(!approver.can_close);
    assert!(approver.can_approve_policy);
}

#[test]
fn permission_view_construction() {
    let view = PermissionView::new("Tui", ClientRoleView::owner(), true, "tui-manual", "TUI");
    assert_eq!(view.client_kind_label, "TUI");
    assert!(view.is_session_owner);
    assert_eq!(view.surface_label, "TUI");
    let json = serde_json::to_string(&view).unwrap();
    let _: PermissionView = serde_json::from_str(&json).unwrap();
}

#[test]
fn policy_prompt_view_roundtrip() {
    let prompt = eggsec_runtime::event::PolicyPrompt {
        message: "Allow scan?".into(),
        confirmation_class: Some("target-scope".into()),
        requires_explicit_approval: true,
    };
    let view = PolicyPromptView::from(&prompt);
    assert!(!view.can_auto_approve);
    assert_eq!(view.confirmation_class.as_deref(), Some("target-scope"));

    let prompt_auto = eggsec_runtime::event::PolicyPrompt {
        message: "Auto-approved".into(),
        confirmation_class: None,
        requires_explicit_approval: false,
    };
    let view_auto = PolicyPromptView::from(&prompt_auto);
    assert!(view_auto.can_auto_approve);
    let json = serde_json::to_string(&view).unwrap();
    let _: PolicyPromptView = serde_json::from_str(&json).unwrap();
}

#[test]
fn dashboard_summary_view() {
    let summaries = vec![
        SessionSummary {
            session_id: SessionId::new(),
            surface: RuntimeSurface::TuiManual,
            scope: None,
            active_count: 1,
            completed_count: 3,
            created_at_epoch_secs: 100,
            owner_client_id: None,
        },
        SessionSummary {
            session_id: SessionId::new(),
            surface: RuntimeSurface::McpServer,
            scope: None,
            active_count: 0,
            completed_count: 2,
            created_at_epoch_secs: 200,
            owner_client_id: None,
        },
    ];
    let view = DashboardSummaryView::from_summaries(&summaries);
    assert_eq!(view.total_sessions, 2);
    assert_eq!(view.active_sessions, 1);
    assert_eq!(view.total_active_tasks, 1);
    assert_eq!(view.total_completed_tasks, 5);
    let json = serde_json::to_string(&view).unwrap();
    let _: DashboardSummaryView = serde_json::from_str(&json).unwrap();
}

#[test]
fn event_view_roundtrip() {
    let event = eggsec_runtime::event::RuntimeEvent::TaskStarted {
        session_id: SessionId::new(),
        task_id: TaskId::new(),
    };
    let view = EventView::from(&event);
    assert_eq!(view.event_type, "task-started");
    assert!(view.task_id.is_some());
    let json = serde_json::to_string(&view).unwrap();
    let _: EventView = serde_json::from_str(&json).unwrap();
}

#[test]
fn event_view_all_variants() {
    let sid = SessionId::new();
    let tid = TaskId::new();

    let cases: Vec<RuntimeEvent> = vec![
        RuntimeEvent::SessionCreated { session_id: sid },
        RuntimeEvent::Snapshot {
            session_id: sid,
            snapshot: SessionSnapshot {
                session_id: sid,
                surface: RuntimeSurface::Ci,
                scope: None,
                created_at_epoch_secs: 0,
                generation: 0,
                active_tasks: vec![],
                completed_tasks: vec![],
                capabilities: Default::default(),
                closed: false,
                closed_at: None,
                owner_client_id: None,
            },
        },
        RuntimeEvent::TaskQueued {
            session_id: sid,
            task_id: tid,
            request: eggsec_runtime::request::RunRequest {
                task_kind: TaskKind::PortScan(PortScanParams {
                    target: "x".into(),
                    ports: None,
                    scan_type: None,
                    timeout_ms: None,
                }),
                requested_by: None,
                surface: RuntimeSurface::Ci,
                labels: vec![],
            },
        },
        RuntimeEvent::TaskStarted {
            session_id: sid,
            task_id: tid,
        },
        RuntimeEvent::TaskProgress {
            session_id: sid,
            task_id: tid,
            progress: TaskProgress {
                completed: 1,
                total: Some(10),
                message: Some("test".into()),
            },
        },
        RuntimeEvent::TaskLog {
            session_id: sid,
            task_id: Some(tid),
            level: eggsec_runtime::event::LogLevel::Info,
            message: "log msg".into(),
        },
        RuntimeEvent::PolicyDecisionRequired {
            session_id: sid,
            task_id: Some(tid),
            prompt: eggsec_runtime::event::PolicyPrompt {
                message: "yes?".into(),
                confirmation_class: None,
                requires_explicit_approval: false,
            },
        },
        RuntimeEvent::TaskCompleted {
            session_id: sid,
            task_id: tid,
            outcome: TaskOutcome::Empty,
        },
        RuntimeEvent::TaskFailed {
            session_id: sid,
            task_id: tid,
            error: eggsec_runtime::event::RuntimeErrorInfo {
                message: "err".into(),
                code: None,
                details: None,
            },
        },
        RuntimeEvent::TaskCancelled {
            session_id: sid,
            task_id: tid,
            reason: Some("user".into()),
        },
        RuntimeEvent::Audit {
            session_id: sid,
            event: eggsec_runtime::event::RuntimeAuditEvent {
                event_type: "test".into(),
                surface: "test".into(),
                outcome: "allow".into(),
                details: None,
            },
        },
        RuntimeEvent::SessionClosed { session_id: sid },
    ];

    for event in &cases {
        let view = EventView::from(event);
        let json = serde_json::to_string(&view).unwrap();
        let _: EventView = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn outcome_view_text() {
    let outcome = TaskOutcome::Text("done".into());
    let view = OutcomeView::from(&outcome);
    assert_eq!(view.outcome_type, "text");
    assert_eq!(view.text_content.as_deref(), Some("done"));
    assert!(view.envelope.is_none());
}

#[test]
fn outcome_view_result() {
    let env = TaskResultEnvelope {
        kind: "recon".into(),
        summary: Some("done".into()),
        payload: serde_json::json!({}),
        artifacts: vec![],
    };
    let outcome = TaskOutcome::Result(env);
    let view = OutcomeView::from(&outcome);
    assert_eq!(view.outcome_type, "result");
    assert!(view.envelope.is_some());
}

#[test]
fn outcome_view_empty() {
    let outcome = TaskOutcome::Empty;
    let view = OutcomeView::from(&outcome);
    assert_eq!(view.outcome_type, "empty");
    assert!(view.summary.is_none());
}

#[test]
fn renderer_for_port_scan() {
    let r = renderer_for_kind("port-scan").unwrap();
    assert_eq!(r.title, "Port Scan");
    assert!(r.supports_rich_tui);
    assert!(r.supports_json_detail);
    assert!(r.summary_fields.contains(&"open_ports"));
}

#[test]
fn renderer_registry_all_entries_have_nonempty_fields() {
    for r in RENDERER_REGISTRY {
        assert!(!r.kind.is_empty());
        assert!(!r.title.is_empty());
        assert!(!r.summary_fields.is_empty());
    }
}
