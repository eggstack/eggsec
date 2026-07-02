use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::protocol::ClientCommand;
use eggsec_runtime::{ClientId, RuntimeSurface};

/// The kind of client connecting to the daemon.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientKind {
    Cli,
    Tui,
    DaemonInternal,
    Mcp,
    Rest,
    Agent,
    Unknown,
}

impl Default for ClientKind {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Role a client has for a specific session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientRole {
    Owner,
    Controller,
    Observer,
    Approver,
}

/// Permission level required for a daemon command.
///
/// Every `ClientCommand` variant maps to exactly one permission.
/// This enum is the single source of truth for the RBAC matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandPermission {
    /// No authorization required (health, capabilities).
    Public,
    /// Requires declared client identity but no session role.
    DeclaredClient,
    /// Read-only session access (snapshot, subscribe).
    Observer,
    /// Mutating session access (submit, cancel).
    Controller,
    /// Full session lifecycle (close).
    Owner,
    /// Policy approval (manual-surface restricted).
    Approver,
}

/// Map a `ClientCommand` to its required permission level.
///
/// This is a total function — every `ClientCommand` variant must be covered.
/// Adding a new variant to `ClientCommand` without updating this function
/// will cause a compile error.
pub fn command_permission(cmd: &ClientCommand) -> CommandPermission {
    match cmd {
        ClientCommand::Health { .. } | ClientCommand::Capabilities { .. } => {
            CommandPermission::Public
        }
        ClientCommand::DeclareClient { .. } => CommandPermission::DeclaredClient,
        ClientCommand::CreateSession { .. } => CommandPermission::DeclaredClient,
        ClientCommand::ListSessions { .. } => CommandPermission::DeclaredClient,
        ClientCommand::GetSnapshot { .. } | ClientCommand::Subscribe { .. } => {
            CommandPermission::Observer
        }
        ClientCommand::SubmitTask { .. }
        | ClientCommand::CancelTask { .. }
        | ClientCommand::CancelActive { .. } => CommandPermission::Controller,
        ClientCommand::CloseSession { .. } => CommandPermission::Owner,
        ClientCommand::ApprovePolicy { .. } => CommandPermission::Approver,
    }
}

/// Information about a connected client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub client_id: ClientId,
    pub kind: ClientKind,
    pub surface: RuntimeSurface,
    pub connected_at_secs: u64,
    pub label: Option<String>,
}

/// Access rule for a client on a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientAccessRule {
    pub client_id: ClientId,
    pub role: ClientRole,
}

/// Session-level access control metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAccess {
    pub owner_client_id: Option<ClientId>,
    pub allowed_clients: Vec<ClientAccessRule>,
    /// The actual runtime surface bound at session creation.
    pub surface: RuntimeSurface,
    /// The client kind of the session creator (for audit).
    pub owner_client_kind: ClientKind,
    pub default_observer_allowed: bool,
    pub default_controller_allowed: bool,
}

impl Default for SessionAccess {
    fn default() -> Self {
        Self {
            owner_client_id: None,
            allowed_clients: Vec::new(),
            surface: RuntimeSurface::Unknown,
            owner_client_kind: ClientKind::Unknown,
            default_observer_allowed: true,
            default_controller_allowed: false,
        }
    }
}

/// Registry tracking connected clients.
pub struct ClientRegistry {
    clients: HashMap<ClientId, ClientInfo>,
}

impl ClientRegistry {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub fn register(&mut self, info: ClientInfo) {
        self.clients.insert(info.client_id, info);
    }

    pub fn unregister(&mut self, client_id: &ClientId) {
        self.clients.remove(client_id);
    }

    pub fn get(&self, client_id: &ClientId) -> Option<&ClientInfo> {
        self.clients.get(client_id)
    }

    pub fn clients(&self) -> Vec<&ClientInfo> {
        self.clients.values().collect()
    }
}

impl Default for ClientRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a client has the required role for a command on a session.
pub fn check_permission(
    _client_kind: &ClientKind,
    client_role: &ClientRole,
    session_surface: &RuntimeSurface,
    permission: CommandPermission,
) -> Result<(), String> {
    match permission {
        CommandPermission::Public => Ok(()),
        CommandPermission::DeclaredClient => Ok(()),
        CommandPermission::Observer => match client_role {
            ClientRole::Owner | ClientRole::Controller | ClientRole::Observer => Ok(()),
            ClientRole::Approver => Ok(()),
        },
        CommandPermission::Controller => match client_role {
            ClientRole::Owner | ClientRole::Controller => Ok(()),
            ClientRole::Observer => Err(
                "permission-denied: observers cannot submit or cancel tasks".into(),
            ),
            ClientRole::Approver => Err(
                "permission-denied: approvers cannot submit or cancel tasks".into(),
            ),
        },
        CommandPermission::Approver => match session_surface {
            RuntimeSurface::TuiManual | RuntimeSurface::CliManual => match client_role {
                ClientRole::Owner | ClientRole::Controller | ClientRole::Approver => Ok(()),
                ClientRole::Observer => {
                    Err("permission-denied: observers cannot approve policies".into())
                }
            },
            _ => match client_role {
                // Strict sessions: only the session owner can approve policies.
                // Controllers, approvers, and observers from unrelated clients are denied.
                ClientRole::Owner => Ok(()),
                ClientRole::Controller | ClientRole::Approver | ClientRole::Observer => Err(
                    "policy-approval-not-allowed: strict sessions do not accept manual approvals from unrelated clients"
                        .into(),
                ),
            },
        },
        CommandPermission::Owner => match client_role {
            ClientRole::Owner | ClientRole::Controller => Ok(()),
            _ => Err(
                "permission-denied: only owner or controller can close session".into(),
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_client_kind_is_unknown() {
        assert_eq!(ClientKind::default(), ClientKind::Unknown);
    }

    #[test]
    fn session_access_defaults() {
        let access = SessionAccess::default();
        assert!(access.owner_client_id.is_none());
        assert!(access.default_observer_allowed);
        assert!(!access.default_controller_allowed);
        assert_eq!(access.surface, RuntimeSurface::Unknown);
    }

    #[test]
    fn client_registry_register_and_get() {
        let mut registry = ClientRegistry::new();
        let cid = ClientId::new();
        let info = ClientInfo {
            client_id: cid,
            kind: ClientKind::Tui,
            surface: RuntimeSurface::TuiManual,
            connected_at_secs: 100,
            label: None,
        };
        registry.register(info);
        assert!(registry.get(&cid).is_some());
        assert_eq!(registry.get(&cid).unwrap().kind, ClientKind::Tui);
    }

    #[test]
    fn client_registry_unregister() {
        let mut registry = ClientRegistry::new();
        let cid = ClientId::new();
        let info = ClientInfo {
            client_id: cid,
            kind: ClientKind::Cli,
            surface: RuntimeSurface::CliManual,
            connected_at_secs: 100,
            label: None,
        };
        registry.register(info);
        registry.unregister(&cid);
        assert!(registry.get(&cid).is_none());
    }

    #[test]
    fn permission_health_always_allowed() {
        assert!(check_permission(
            &ClientKind::Unknown,
            &ClientRole::Observer,
            &RuntimeSurface::McpServer,
            CommandPermission::Public,
        )
        .is_ok());
    }

    #[test]
    fn permission_observer_cannot_submit() {
        let result = check_permission(
            &ClientKind::Cli,
            &ClientRole::Observer,
            &RuntimeSurface::TuiManual,
            CommandPermission::Controller,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("permission-denied"));
    }

    #[test]
    fn permission_controller_can_submit() {
        assert!(check_permission(
            &ClientKind::Cli,
            &ClientRole::Controller,
            &RuntimeSurface::TuiManual,
            CommandPermission::Controller,
        )
        .is_ok());
    }

    #[test]
    fn permission_owner_can_submit() {
        assert!(check_permission(
            &ClientKind::Tui,
            &ClientRole::Owner,
            &RuntimeSurface::TuiManual,
            CommandPermission::Controller,
        )
        .is_ok());
    }

    #[test]
    fn permission_approver_can_approve_on_manual() {
        assert!(check_permission(
            &ClientKind::Tui,
            &ClientRole::Approver,
            &RuntimeSurface::TuiManual,
            CommandPermission::Approver,
        )
        .is_ok());
    }

    #[test]
    fn permission_approver_cannot_approve_on_strict() {
        let result = check_permission(
            &ClientKind::Tui,
            &ClientRole::Approver,
            &RuntimeSurface::McpServer,
            CommandPermission::Approver,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("policy-approval-not-allowed"));
    }

    #[test]
    fn permission_observer_can_snapshot() {
        assert!(check_permission(
            &ClientKind::Cli,
            &ClientRole::Observer,
            &RuntimeSurface::TuiManual,
            CommandPermission::Observer,
        )
        .is_ok());
    }

    #[test]
    fn permission_owner_can_close_session() {
        assert!(check_permission(
            &ClientKind::Tui,
            &ClientRole::Owner,
            &RuntimeSurface::TuiManual,
            CommandPermission::Owner,
        )
        .is_ok());
    }

    #[test]
    fn permission_observer_cannot_close_session() {
        let result = check_permission(
            &ClientKind::Cli,
            &ClientRole::Observer,
            &RuntimeSurface::TuiManual,
            CommandPermission::Owner,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("permission-denied"));
    }

    #[test]
    fn command_permission_mapping_covers_all_variants() {
        let commands = vec![
            ClientCommand::Health {
                request_id: "t".into(),
            },
            ClientCommand::Capabilities {
                request_id: "t".into(),
            },
            ClientCommand::DeclareClient {
                request_id: "t".into(),
                kind: ClientKind::Tui,
                label: None,
            },
            ClientCommand::CreateSession {
                request_id: "t".into(),
                surface: RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            },
            ClientCommand::ListSessions {
                request_id: "t".into(),
            },
            ClientCommand::GetSnapshot {
                request_id: "t".into(),
                session_id: eggsec_runtime::SessionId::new(),
            },
            ClientCommand::SubmitTask {
                request_id: "t".into(),
                session_id: eggsec_runtime::SessionId::new(),
                request: eggsec_runtime::RunRequest {
                    task_kind: eggsec_runtime::TaskKind::PortScan(
                        eggsec_runtime::request::PortScanParams {
                            target: "10.0.0.1".into(),
                            ports: None,
                            scan_type: None,
                            timeout_ms: None,
                        },
                    ),
                    requested_by: None,
                    surface: RuntimeSurface::CliManual,
                    labels: vec![],
                },
            },
            ClientCommand::CancelTask {
                request_id: "t".into(),
                session_id: eggsec_runtime::SessionId::new(),
                task_id: eggsec_runtime::TaskId::new(),
            },
            ClientCommand::CancelActive {
                request_id: "t".into(),
                session_id: eggsec_runtime::SessionId::new(),
            },
            ClientCommand::Subscribe {
                request_id: "t".into(),
                session_id: eggsec_runtime::SessionId::new(),
            },
            ClientCommand::CloseSession {
                request_id: "t".into(),
                session_id: eggsec_runtime::SessionId::new(),
            },
            ClientCommand::ApprovePolicy {
                request_id: "t".into(),
                session_id: eggsec_runtime::SessionId::new(),
                task_id: eggsec_runtime::TaskId::new(),
                approved: true,
                reason: None,
            },
        ];
        for cmd in &commands {
            let perm = command_permission(cmd);
            // Every command must map to a permission — no panics or undefined behavior.
            let _ = check_permission(&ClientKind::Tui, &ClientRole::Owner, &RuntimeSurface::TuiManual, perm);
        }
    }

    #[test]
    fn strict_surface_approver_denied() {
        let surfaces = vec![
            RuntimeSurface::McpServer,
            RuntimeSurface::RestApi,
            RuntimeSurface::GrpcApi,
            RuntimeSurface::SecurityAgent,
            RuntimeSurface::Ci,
        ];
        for surface in &surfaces {
            let result = check_permission(
                &ClientKind::Agent,
                &ClientRole::Approver,
                surface,
                CommandPermission::Approver,
            );
            assert!(result.is_err(), "Approver should be denied on strict surface {:?}", surface);
        }
    }

    #[test]
    fn manual_surface_approver_allowed() {
        let surfaces = vec![RuntimeSurface::TuiManual, RuntimeSurface::CliManual];
        for surface in &surfaces {
            assert!(
                check_permission(
                    &ClientKind::Tui,
                    &ClientRole::Approver,
                    surface,
                    CommandPermission::Approver,
                )
                .is_ok(),
                "Approver should be allowed on manual surface {:?}",
                surface
            );
        }
    }

    #[test]
    fn observer_denied_on_all_mutation_permissions() {
        let mutation_perms = vec![CommandPermission::Controller, CommandPermission::Owner];
        for perm in &mutation_perms {
            let result = check_permission(
                &ClientKind::Cli,
                &ClientRole::Observer,
                &RuntimeSurface::TuiManual,
                *perm,
            );
            assert!(result.is_err(), "Observer should be denied for {:?}", perm);
        }
    }

    #[test]
    fn approver_denied_on_mutation_permissions() {
        let mutation_perms = vec![CommandPermission::Controller, CommandPermission::Owner];
        for perm in &mutation_perms {
            let result = check_permission(
                &ClientKind::Tui,
                &ClientRole::Approver,
                &RuntimeSurface::TuiManual,
                *perm,
            );
            assert!(result.is_err(), "Approver should be denied for {:?}", perm);
        }
    }

    #[test]
    fn unrelated_tui_cannot_approve_strict_session() {
        let result = check_permission(
            &ClientKind::Tui,
            &ClientRole::Controller,
            &RuntimeSurface::McpServer,
            CommandPermission::Approver,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("policy-approval-not-allowed"));
    }
}
