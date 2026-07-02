use serde::{Deserialize, Serialize};

use crate::client_registry::ClientKind;
use eggsec_runtime::{
    ClientId, RunRequest, RuntimeCapabilities, RuntimeEvent, RuntimeSurface, SessionId,
    SessionSnapshot, SessionSummary, TaskId,
};

/// Error codes returned by the daemon in response to client commands.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ErrorCode {
    InvalidRequest,
    SessionNotFound,
    TaskNotFound,
    TaskAlreadyCompleted,
    UnsupportedCommand,
    Internal,
    PermissionDenied,
    InvalidSurface,
}

/// A command sent from a client to the daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientCommand {
    Health {
        request_id: String,
    },
    Capabilities {
        request_id: String,
    },
    DeclareClient {
        request_id: String,
        kind: ClientKind,
        label: Option<String>,
    },
    CreateSession {
        request_id: String,
        surface: RuntimeSurface,
        scope: Option<eggsec_runtime::SessionScope>,
        labels: Vec<String>,
    },
    ListSessions {
        request_id: String,
    },
    GetSnapshot {
        request_id: String,
        session_id: SessionId,
    },
    SubmitTask {
        request_id: String,
        session_id: SessionId,
        request: RunRequest,
    },
    CancelTask {
        request_id: String,
        session_id: SessionId,
        task_id: TaskId,
    },
    CancelActive {
        request_id: String,
        session_id: SessionId,
    },
    Subscribe {
        request_id: String,
        session_id: SessionId,
    },
    CloseSession {
        request_id: String,
        session_id: SessionId,
    },
    ApprovePolicy {
        request_id: String,
        session_id: SessionId,
        task_id: TaskId,
        approved: bool,
        reason: Option<String>,
    },
}

impl ClientCommand {
    /// Borrow the request_id without destructuring.
    pub fn request_id(&self) -> &str {
        match self {
            Self::Health { request_id }
            | Self::Capabilities { request_id }
            | Self::DeclareClient { request_id, .. }
            | Self::CreateSession { request_id, .. }
            | Self::ListSessions { request_id }
            | Self::GetSnapshot { request_id, .. }
            | Self::SubmitTask { request_id, .. }
            | Self::CancelTask { request_id, .. }
            | Self::CancelActive { request_id, .. }
            | Self::Subscribe { request_id, .. }
            | Self::CloseSession { request_id, .. }
            | Self::ApprovePolicy { request_id, .. } => request_id,
        }
    }

    /// Borrow the session_id if this command targets one.
    pub fn session_id(&self) -> Option<&SessionId> {
        match self {
            Self::GetSnapshot { session_id, .. }
            | Self::SubmitTask { session_id, .. }
            | Self::CancelTask { session_id, .. }
            | Self::CancelActive { session_id, .. }
            | Self::Subscribe { session_id, .. }
            | Self::CloseSession { session_id, .. }
            | Self::ApprovePolicy { session_id, .. } => Some(session_id),
            _ => None,
        }
    }
}

/// A message sent from the daemon to a client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    Ok {
        request_id: String,
    },
    Error {
        request_id: String,
        code: ErrorCode,
        message: String,
    },
    ClientDeclared {
        request_id: String,
        client_id: ClientId,
    },
    SessionCreated {
        request_id: String,
        session_id: SessionId,
    },
    Sessions {
        request_id: String,
        sessions: Vec<SessionSummary>,
    },
    Snapshot {
        request_id: String,
        snapshot: SessionSnapshot,
    },
    TaskSubmitted {
        request_id: String,
        task_id: TaskId,
    },
    Capabilities {
        request_id: String,
        capabilities: RuntimeCapabilities,
    },
    Health {
        request_id: String,
        status: String,
        version: String,
    },
    RuntimeEvent {
        session_id: SessionId,
        event: RuntimeEvent,
    },
    SessionClosed {
        request_id: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggsec_runtime::request::PortScanParams;
    use eggsec_runtime::{ClientId, SessionScope};

    fn rid() -> String {
        "req-001".into()
    }

    // ErrorCode round-trips

    #[test]
    fn error_code_roundtrip_invalid_request() {
        let code = ErrorCode::InvalidRequest;
        let json = serde_json::to_string(&code).unwrap();
        let back: ErrorCode = serde_json::from_str(&json).unwrap();
        assert_eq!(code, back);
    }

    #[test]
    fn error_code_roundtrip_session_not_found() {
        let code = ErrorCode::SessionNotFound;
        let json = serde_json::to_string(&code).unwrap();
        let back: ErrorCode = serde_json::from_str(&json).unwrap();
        assert_eq!(code, back);
    }

    #[test]
    fn error_code_roundtrip_task_not_found() {
        let code = ErrorCode::TaskNotFound;
        let json = serde_json::to_string(&code).unwrap();
        let back: ErrorCode = serde_json::from_str(&json).unwrap();
        assert_eq!(code, back);
    }

    #[test]
    fn error_code_roundtrip_task_already_completed() {
        let code = ErrorCode::TaskAlreadyCompleted;
        let json = serde_json::to_string(&code).unwrap();
        let back: ErrorCode = serde_json::from_str(&json).unwrap();
        assert_eq!(code, back);
    }

    #[test]
    fn error_code_roundtrip_unsupported_command() {
        let code = ErrorCode::UnsupportedCommand;
        let json = serde_json::to_string(&code).unwrap();
        let back: ErrorCode = serde_json::from_str(&json).unwrap();
        assert_eq!(code, back);
    }

    #[test]
    fn error_code_roundtrip_internal() {
        let code = ErrorCode::Internal;
        let json = serde_json::to_string(&code).unwrap();
        let back: ErrorCode = serde_json::from_str(&json).unwrap();
        assert_eq!(code, back);
    }

    #[test]
    fn error_code_produces_type_field() {
        let code = ErrorCode::Internal;
        let val = serde_json::to_value(&code).unwrap();
        assert_eq!(val["type"], "Internal");
    }

    // ClientCommand round-trips

    #[test]
    fn client_command_roundtrip_health() {
        let cmd = ClientCommand::Health { request_id: rid() };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: ClientCommand = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, ClientCommand::Health { request_id } if request_id == "req-001"));
    }

    #[test]
    fn client_command_roundtrip_capabilities() {
        let cmd = ClientCommand::Capabilities { request_id: rid() };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: ClientCommand = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, ClientCommand::Capabilities { .. }));
    }

    #[test]
    fn client_command_roundtrip_create_session() {
        let cmd = ClientCommand::CreateSession {
            request_id: rid(),
            surface: RuntimeSurface::McpServer,
            scope: Some(SessionScope {
                is_explicit: true,
                source: "config".into(),
                path: None,
            }),
            labels: vec!["test".into()],
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: ClientCommand = serde_json::from_str(&json).unwrap();
        if let ClientCommand::CreateSession {
            surface,
            scope,
            labels,
            ..
        } = back
        {
            assert_eq!(surface, RuntimeSurface::McpServer);
            let s = scope.unwrap();
            assert!(s.is_explicit);
            assert_eq!(s.source, "config");
            assert_eq!(labels, vec!["test".to_string()]);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn client_command_roundtrip_list_sessions() {
        let cmd = ClientCommand::ListSessions { request_id: rid() };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: ClientCommand = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, ClientCommand::ListSessions { .. }));
    }

    #[test]
    fn client_command_roundtrip_get_snapshot() {
        let sid = SessionId::new();
        let cmd = ClientCommand::GetSnapshot {
            request_id: rid(),
            session_id: sid,
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: ClientCommand = serde_json::from_str(&json).unwrap();
        if let ClientCommand::GetSnapshot { session_id, .. } = back {
            assert_eq!(session_id, sid);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn client_command_roundtrip_submit_task() {
        let sid = SessionId::new();
        let cmd = ClientCommand::SubmitTask {
            request_id: rid(),
            session_id: sid,
            request: RunRequest {
                task_kind: eggsec_runtime::TaskKind::PortScan(PortScanParams {
                    target: "10.0.0.1".into(),
                    ports: None,
                    scan_type: None,
                    timeout_ms: None,
                }),
                requested_by: Some(ClientId::new()),
                surface: RuntimeSurface::CliManual,
                labels: vec![],
            },
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: ClientCommand = serde_json::from_str(&json).unwrap();
        if let ClientCommand::SubmitTask { session_id, .. } = back {
            assert_eq!(session_id, sid);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn client_command_roundtrip_cancel_task() {
        let sid = SessionId::new();
        let tid = TaskId::new();
        let cmd = ClientCommand::CancelTask {
            request_id: rid(),
            session_id: sid,
            task_id: tid,
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: ClientCommand = serde_json::from_str(&json).unwrap();
        if let ClientCommand::CancelTask {
            session_id,
            task_id,
            ..
        } = back
        {
            assert_eq!(session_id, sid);
            assert_eq!(task_id, tid);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn client_command_roundtrip_cancel_active() {
        let sid = SessionId::new();
        let cmd = ClientCommand::CancelActive {
            request_id: rid(),
            session_id: sid,
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: ClientCommand = serde_json::from_str(&json).unwrap();
        if let ClientCommand::CancelActive { session_id, .. } = back {
            assert_eq!(session_id, sid);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn client_command_roundtrip_subscribe() {
        let sid = SessionId::new();
        let cmd = ClientCommand::Subscribe {
            request_id: rid(),
            session_id: sid,
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: ClientCommand = serde_json::from_str(&json).unwrap();
        if let ClientCommand::Subscribe { session_id, .. } = back {
            assert_eq!(session_id, sid);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn client_command_produces_type_field() {
        let cmd = ClientCommand::Health { request_id: rid() };
        let val = serde_json::to_value(&cmd).unwrap();
        assert_eq!(val["type"], "Health");
    }

    // ServerMessage round-trips

    #[test]
    fn server_message_roundtrip_ok() {
        let msg = ServerMessage::Ok { request_id: rid() };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, ServerMessage::Ok { request_id } if request_id == "req-001"));
    }

    #[test]
    fn server_message_roundtrip_error() {
        let msg = ServerMessage::Error {
            request_id: rid(),
            code: ErrorCode::SessionNotFound,
            message: "no such session".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        if let ServerMessage::Error { code, message, .. } = back {
            assert_eq!(code, ErrorCode::SessionNotFound);
            assert_eq!(message, "no such session");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn server_message_roundtrip_session_created() {
        let sid = SessionId::new();
        let msg = ServerMessage::SessionCreated {
            request_id: rid(),
            session_id: sid,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        if let ServerMessage::SessionCreated { session_id, .. } = back {
            assert_eq!(session_id, sid);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn server_message_roundtrip_sessions() {
        let msg = ServerMessage::Sessions {
            request_id: rid(),
            sessions: vec![SessionSummary {
                session_id: SessionId::new(),
                surface: RuntimeSurface::RestApi,
                scope: None,
                active_count: 1,
                completed_count: 0,
                created_at_secs: 100,
            }],
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        if let ServerMessage::Sessions { sessions, .. } = back {
            assert_eq!(sessions.len(), 1);
            assert_eq!(sessions[0].surface, RuntimeSurface::RestApi);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn server_message_roundtrip_snapshot() {
        let msg = ServerMessage::Snapshot {
            request_id: rid(),
            snapshot: SessionSnapshot {
                session_id: SessionId::new(),
                surface: RuntimeSurface::TuiManual,
                scope: None,
                created_at_secs: 42,
                generation: 0,
                active_tasks: vec![],
                completed_tasks: vec![],
                capabilities: RuntimeCapabilities::default(),
            },
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        if let ServerMessage::Snapshot { snapshot, .. } = back {
            assert_eq!(snapshot.created_at_secs, 42);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn server_message_roundtrip_task_submitted() {
        let tid = TaskId::new();
        let msg = ServerMessage::TaskSubmitted {
            request_id: rid(),
            task_id: tid,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        if let ServerMessage::TaskSubmitted { task_id, .. } = back {
            assert_eq!(task_id, tid);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn server_message_roundtrip_capabilities() {
        let msg = ServerMessage::Capabilities {
            request_id: rid(),
            capabilities: RuntimeCapabilities::default(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        if let ServerMessage::Capabilities { capabilities, .. } = back {
            assert!(capabilities.supports_cancellation);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn server_message_roundtrip_health() {
        let msg = ServerMessage::Health {
            request_id: rid(),
            status: "ok".into(),
            version: "0.1.0".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        if let ServerMessage::Health {
            status, version, ..
        } = back
        {
            assert_eq!(status, "ok");
            assert_eq!(version, "0.1.0");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn server_message_roundtrip_runtime_event() {
        let sid = SessionId::new();
        let msg = ServerMessage::RuntimeEvent {
            session_id: sid,
            event: RuntimeEvent::SessionCreated { session_id: sid },
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        if let ServerMessage::RuntimeEvent { session_id, event } = back {
            assert_eq!(session_id, sid);
            assert!(matches!(event, RuntimeEvent::SessionCreated { session_id: s } if s == sid));
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn server_message_produces_type_field() {
        let msg = ServerMessage::Ok { request_id: rid() };
        let val = serde_json::to_value(&msg).unwrap();
        assert_eq!(val["type"], "Ok");
    }

    // New variant round-trips

    #[test]
    fn error_code_roundtrip_permission_denied() {
        let code = ErrorCode::PermissionDenied;
        let json = serde_json::to_string(&code).unwrap();
        let back: ErrorCode = serde_json::from_str(&json).unwrap();
        assert_eq!(code, back);
    }

    #[test]
    fn error_code_roundtrip_invalid_surface() {
        let code = ErrorCode::InvalidSurface;
        let json = serde_json::to_string(&code).unwrap();
        let back: ErrorCode = serde_json::from_str(&json).unwrap();
        assert_eq!(code, back);
    }

    #[test]
    fn client_command_roundtrip_declare_client() {
        let cmd = ClientCommand::DeclareClient {
            request_id: rid(),
            kind: ClientKind::Tui,
            label: Some("my-tui".into()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: ClientCommand = serde_json::from_str(&json).unwrap();
        if let ClientCommand::DeclareClient { kind, label, .. } = back {
            assert_eq!(kind, ClientKind::Tui);
            assert_eq!(label.as_deref(), Some("my-tui"));
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn client_command_roundtrip_close_session() {
        let sid = SessionId::new();
        let cmd = ClientCommand::CloseSession {
            request_id: rid(),
            session_id: sid,
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: ClientCommand = serde_json::from_str(&json).unwrap();
        if let ClientCommand::CloseSession { session_id, .. } = back {
            assert_eq!(session_id, sid);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn client_command_roundtrip_approve_policy() {
        let sid = SessionId::new();
        let tid = TaskId::new();
        let cmd = ClientCommand::ApprovePolicy {
            request_id: rid(),
            session_id: sid,
            task_id: tid,
            approved: true,
            reason: Some("confirmed".into()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: ClientCommand = serde_json::from_str(&json).unwrap();
        if let ClientCommand::ApprovePolicy {
            session_id,
            task_id,
            approved,
            reason,
            ..
        } = back
        {
            assert_eq!(session_id, sid);
            assert_eq!(task_id, tid);
            assert!(approved);
            assert_eq!(reason.as_deref(), Some("confirmed"));
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn server_message_roundtrip_client_declared() {
        let cid = ClientId::new();
        let msg = ServerMessage::ClientDeclared {
            request_id: rid(),
            client_id: cid,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        if let ServerMessage::ClientDeclared { client_id, .. } = back {
            assert_eq!(client_id, cid);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn server_message_roundtrip_session_closed() {
        let msg = ServerMessage::SessionClosed {
            request_id: rid(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        if let ServerMessage::SessionClosed { request_id, .. } = back {
            assert_eq!(request_id, "req-001");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn client_command_type_field_declare_client() {
        let cmd = ClientCommand::DeclareClient {
            request_id: rid(),
            kind: ClientKind::Agent,
            label: None,
        };
        let val = serde_json::to_value(&cmd).unwrap();
        assert_eq!(val["type"], "DeclareClient");
    }

    #[test]
    fn client_command_type_field_close_session() {
        let cmd = ClientCommand::CloseSession {
            request_id: rid(),
            session_id: SessionId::new(),
        };
        let val = serde_json::to_value(&cmd).unwrap();
        assert_eq!(val["type"], "CloseSession");
    }

    #[test]
    fn server_message_type_field_client_declared() {
        let msg = ServerMessage::ClientDeclared {
            request_id: rid(),
            client_id: ClientId::new(),
        };
        let val = serde_json::to_value(&msg).unwrap();
        assert_eq!(val["type"], "ClientDeclared");
    }

    #[test]
    fn server_message_type_field_session_closed() {
        let msg = ServerMessage::SessionClosed {
            request_id: rid(),
        };
        let val = serde_json::to_value(&msg).unwrap();
        assert_eq!(val["type"], "SessionClosed");
    }

    // Cross-variant type field checks

    #[test]
    fn error_code_type_field_values() {
        let cases = vec![
            (ErrorCode::InvalidRequest, "InvalidRequest"),
            (ErrorCode::SessionNotFound, "SessionNotFound"),
            (ErrorCode::TaskNotFound, "TaskNotFound"),
            (ErrorCode::TaskAlreadyCompleted, "TaskAlreadyCompleted"),
            (ErrorCode::UnsupportedCommand, "UnsupportedCommand"),
            (ErrorCode::Internal, "Internal"),
            (ErrorCode::PermissionDenied, "PermissionDenied"),
            (ErrorCode::InvalidSurface, "InvalidSurface"),
        ];
        for (code, expected_type) in cases {
            let val = serde_json::to_value(&code).unwrap();
            assert_eq!(val["type"], expected_type);
        }
    }

    #[test]
    fn client_command_type_field_values() {
        let cases = vec![
            (
                serde_json::to_value(&ClientCommand::Health { request_id: rid() }).unwrap(),
                "Health",
            ),
            (
                serde_json::to_value(&ClientCommand::Capabilities { request_id: rid() }).unwrap(),
                "Capabilities",
            ),
            (
                serde_json::to_value(&ClientCommand::DeclareClient {
                    request_id: rid(),
                    kind: ClientKind::Unknown,
                    label: None,
                })
                .unwrap(),
                "DeclareClient",
            ),
            (
                serde_json::to_value(&ClientCommand::ListSessions { request_id: rid() }).unwrap(),
                "ListSessions",
            ),
            (
                serde_json::to_value(&ClientCommand::CloseSession {
                    request_id: rid(),
                    session_id: SessionId::new(),
                })
                .unwrap(),
                "CloseSession",
            ),
            (
                serde_json::to_value(&ClientCommand::ApprovePolicy {
                    request_id: rid(),
                    session_id: SessionId::new(),
                    task_id: TaskId::new(),
                    approved: true,
                    reason: None,
                })
                .unwrap(),
                "ApprovePolicy",
            ),
        ];
        for (val, expected_type) in cases {
            assert_eq!(val["type"], expected_type);
        }
    }

    #[test]
    fn server_message_type_field_values() {
        let cases = vec![
            (
                serde_json::to_value(&ServerMessage::Ok { request_id: rid() }).unwrap(),
                "Ok",
            ),
            (
                serde_json::to_value(&ServerMessage::ClientDeclared {
                    request_id: rid(),
                    client_id: ClientId::new(),
                })
                .unwrap(),
                "ClientDeclared",
            ),
            (
                serde_json::to_value(&ServerMessage::TaskSubmitted {
                    request_id: rid(),
                    task_id: TaskId::new(),
                })
                .unwrap(),
                "TaskSubmitted",
            ),
            (
                serde_json::to_value(&ServerMessage::SessionClosed {
                    request_id: rid(),
                })
                .unwrap(),
                "SessionClosed",
            ),
        ];
        for (val, expected_type) in cases {
            assert_eq!(val["type"], expected_type);
        }
    }
}
