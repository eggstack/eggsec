use serde::{Deserialize, Serialize};

/// Frontend-neutral client role view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRoleView {
    pub role: String,
    pub role_label: String,
    pub can_submit: bool,
    pub can_cancel: bool,
    pub can_close: bool,
    pub can_approve_policy: bool,
}

impl ClientRoleView {
    pub fn owner() -> Self {
        Self {
            role: "owner".into(),
            role_label: "Owner".into(),
            can_submit: true,
            can_cancel: true,
            can_close: true,
            can_approve_policy: true,
        }
    }
    pub fn controller() -> Self {
        Self {
            role: "controller".into(),
            role_label: "Controller".into(),
            can_submit: true,
            can_cancel: true,
            can_close: false,
            can_approve_policy: false,
        }
    }
    pub fn observer() -> Self {
        Self {
            role: "observer".into(),
            role_label: "Observer".into(),
            can_submit: false,
            can_cancel: false,
            can_close: false,
            can_approve_policy: false,
        }
    }
    pub fn approver() -> Self {
        Self {
            role: "approver".into(),
            role_label: "Approver".into(),
            can_submit: false,
            can_cancel: false,
            can_close: false,
            can_approve_policy: true,
        }
    }
}

/// Frontend-neutral permission status view for display in UIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionView {
    pub client_kind: String,
    pub client_kind_label: String,
    pub session_role: ClientRoleView,
    pub is_session_owner: bool,
    pub surface: String,
    pub surface_label: String,
}

impl PermissionView {
    pub fn new(
        client_kind: &str,
        role: ClientRoleView,
        is_owner: bool,
        surface: &str,
        surface_label: &str,
    ) -> Self {
        Self {
            client_kind: client_kind.into(),
            client_kind_label: client_kind_label(client_kind).into(),
            session_role: role,
            is_session_owner: is_owner,
            surface: surface.into(),
            surface_label: surface_label.into(),
        }
    }
}

fn client_kind_label(kind: &str) -> &'static str {
    match kind {
        "Cli" => "CLI",
        "Tui" => "TUI",
        "DaemonInternal" => "Daemon",
        "Mcp" => "MCP",
        "Rest" => "REST",
        "Agent" => "Agent",
        _ => "Unknown",
    }
}
