use serde::{Deserialize, Serialize};

use eggsec_runtime::event::PolicyPrompt;

/// Frontend-neutral policy prompt view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyPromptView {
    pub message: String,
    pub confirmation_class: Option<String>,
    pub requires_explicit_approval: bool,
    pub can_auto_approve: bool,
}

impl From<&PolicyPrompt> for PolicyPromptView {
    fn from(p: &PolicyPrompt) -> Self {
        Self {
            message: p.message.clone(),
            confirmation_class: p.confirmation_class.clone(),
            requires_explicit_approval: p.requires_explicit_approval,
            can_auto_approve: !p.requires_explicit_approval,
        }
    }
}
