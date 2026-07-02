use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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
    pub default_observer_allowed: bool,
    pub default_controller_allowed: bool,
}

impl Default for SessionAccess {
    fn default() -> Self {
        Self {
            owner_client_id: None,
            allowed_clients: Vec::new(),
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
    client_kind: &ClientKind,
    client_role: &ClientRole,
    session_surface: &RuntimeSurface,
    command: &str,
) -> Result<(), String> {
    match command {
        "health" | "capabilities" | "list-sessions" => Ok(()),
        "get-snapshot" | "subscribe" => match client_role {
            ClientRole::Owner | ClientRole::Controller | ClientRole::Observer => Ok(()),
            ClientRole::Approver => Ok(()),
        },
        "submit-task" | "cancel-task" | "cancel-active" => match client_role {
            ClientRole::Owner | ClientRole::Controller => Ok(()),
            ClientRole::Observer => Err(
                "permission-denied: observers cannot submit or cancel tasks".into(),
            ),
            ClientRole::Approver => Err(
                "permission-denied: approvers cannot submit or cancel tasks".into(),
            ),
        },
        "approve-policy" => match session_surface {
            RuntimeSurface::TuiManual | RuntimeSurface::CliManual => match client_role {
                ClientRole::Owner | ClientRole::Controller | ClientRole::Approver => Ok(()),
                ClientRole::Observer => {
                    Err("permission-denied: observers cannot approve policies".into())
                }
            },
            _ => match client_role {
                ClientRole::Owner | ClientRole::Controller => Ok(()),
                ClientRole::Approver | ClientRole::Observer => Err(
                    "policy-approval-not-allowed: strict sessions do not accept manual approvals from unrelated clients"
                        .into(),
                ),
            },
        },
        "close-session" => match client_role {
            ClientRole::Owner | ClientRole::Controller => Ok(()),
            _ => Err(
                "permission-denied: only owner or controller can close session".into(),
            ),
        },
        _ => Err(format!("permission-denied: unknown command '{}'", command)),
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
            "health"
        )
        .is_ok());
    }

    #[test]
    fn permission_observer_cannot_submit() {
        let result = check_permission(
            &ClientKind::Cli,
            &ClientRole::Observer,
            &RuntimeSurface::TuiManual,
            "submit-task",
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
            "submit-task",
        )
        .is_ok());
    }

    #[test]
    fn permission_owner_can_submit() {
        assert!(check_permission(
            &ClientKind::Tui,
            &ClientRole::Owner,
            &RuntimeSurface::TuiManual,
            "submit-task",
        )
        .is_ok());
    }

    #[test]
    fn permission_approver_can_approve_on_manual() {
        assert!(check_permission(
            &ClientKind::Tui,
            &ClientRole::Approver,
            &RuntimeSurface::TuiManual,
            "approve-policy",
        )
        .is_ok());
    }

    #[test]
    fn permission_approver_cannot_approve_on_strict() {
        let result = check_permission(
            &ClientKind::Tui,
            &ClientRole::Approver,
            &RuntimeSurface::McpServer,
            "approve-policy",
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
            "get-snapshot",
        )
        .is_ok());
    }

    #[test]
    fn permission_owner_can_close_session() {
        assert!(check_permission(
            &ClientKind::Tui,
            &ClientRole::Owner,
            &RuntimeSurface::TuiManual,
            "close-session",
        )
        .is_ok());
    }

    #[test]
    fn permission_observer_cannot_close_session() {
        let result = check_permission(
            &ClientKind::Cli,
            &ClientRole::Observer,
            &RuntimeSurface::TuiManual,
            "close-session",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("permission-denied"));
    }

    #[test]
    fn permission_unknown_command_denied() {
        let result = check_permission(
            &ClientKind::Tui,
            &ClientRole::Owner,
            &RuntimeSurface::TuiManual,
            "nonexistent-command",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown command"));
    }
}
