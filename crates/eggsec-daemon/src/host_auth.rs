//! Daemon client authorization / RBAC helpers (Phase D WS7).
//!
//! Cohesive module extracted from `host.rs`: session ownership and role
//! resolution only. No persistence, request handling, recovery, or runtime
//! lifecycle here.
//!
//! These are pure functions over the session-access map so they remain
//! testable without a full `DaemonHost`. `host.rs` delegates its
//! `client_role_for_session` method here and stays the stable facade.
//!
//! Session ownership behavior is unchanged: no owner info means legacy /
//! recovered sessions stay visible; an explicit owner restricts to the
//! owner plus allowed clients; everyone else is `Observer`.

use std::collections::HashMap;

use eggsec_runtime::SessionId;

use crate::client_registry::{ClientRole, SessionAccess};

/// Resolve the [`ClientRole`] for `client_id` on `session_id`.
///
/// Pure function over the access map; see `DaemonHost::client_role_for_session`.
pub fn role_for_session(
    access: &HashMap<SessionId, SessionAccess>,
    client_id: &eggsec_runtime::ClientId,
    session_id: &SessionId,
) -> ClientRole {
    if let Some(session_access) = access.get(session_id) {
        if session_access.owner_client_id == Some(*client_id) {
            return ClientRole::Owner;
        }
        for rule in &session_access.allowed_clients {
            if rule.client_id == *client_id {
                return rule.role.clone();
            }
        }
    }
    ClientRole::Observer
}

/// Returns `true` when `client_id` may observe `session_id` given the access
/// map. Legacy/recovered sessions without owner info stay visible; otherwise
/// the caller must be the owner, an allowed client, or (when no client id is
/// present) the session must be ownerless.
pub fn may_observe_session(
    access: &HashMap<SessionId, SessionAccess>,
    client_id: Option<eggsec_runtime::ClientId>,
    session_id: &SessionId,
) -> bool {
    match access.get(session_id) {
        None => true,
        Some(session_access) => match (client_id, session_access.owner_client_id) {
            // No owner info → include (legacy/recovered sessions).
            (_, None) => true,
            (Some(cid), Some(owner)) => {
                cid == owner
                    || session_access
                        .allowed_clients
                        .iter()
                        .any(|rule| rule.client_id == cid)
            }
            // No client ID but session has owner → exclude.
            (None, Some(_)) => false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_registry::{ClientAccessRule, ClientKind};

    fn owner_access(owner: eggsec_runtime::ClientId) -> SessionAccess {
        SessionAccess {
            owner_client_id: Some(owner),
            owner_client_kind: ClientKind::Cli,
            ..Default::default()
        }
    }

    #[test]
    fn owner_resolves_to_owner_role() {
        let owner = eggsec_runtime::ClientId::new();
        let session = SessionId::new();
        let mut access = HashMap::new();
        access.insert(session, owner_access(owner));
        assert!(matches!(
            role_for_session(&access, &owner, &session),
            ClientRole::Owner
        ));
        assert!(may_observe_session(&access, Some(owner), &session));
    }

    #[test]
    fn stranger_is_observer_and_may_not_observe_owned_session() {
        let owner = eggsec_runtime::ClientId::new();
        let stranger = eggsec_runtime::ClientId::new();
        let session = SessionId::new();
        let mut access = HashMap::new();
        access.insert(session, owner_access(owner));
        assert!(matches!(
            role_for_session(&access, &stranger, &session),
            ClientRole::Observer
        ));
        assert!(!may_observe_session(&access, Some(stranger), &session));
    }

    #[test]
    fn allowed_client_keeps_explicit_role() {
        let owner = eggsec_runtime::ClientId::new();
        let observer = eggsec_runtime::ClientId::new();
        let session = SessionId::new();
        let mut access = HashMap::new();
        access.insert(
            session,
            SessionAccess {
                owner_client_id: Some(owner),
                owner_client_kind: ClientKind::Cli,
                allowed_clients: vec![ClientAccessRule {
                    client_id: observer,
                    role: ClientRole::Observer,
                }],
                ..Default::default()
            },
        );
        assert!(matches!(
            role_for_session(&access, &observer, &session),
            ClientRole::Observer
        ));
        assert!(may_observe_session(&access, Some(observer), &session));
    }

    #[test]
    fn legacy_session_without_owner_stays_visible() {
        let session = SessionId::new();
        let access: HashMap<SessionId, SessionAccess> = HashMap::new();
        let stranger = eggsec_runtime::ClientId::new();
        assert!(may_observe_session(&access, Some(stranger), &session));
    }
}
