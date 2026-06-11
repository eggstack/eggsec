use super::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingAction {
    ResetTab,
    SaveSettings,
    DeleteHistoryEntry,
    ClearHistory,
}

impl PendingAction {
    pub fn message(&self) -> (String, Vec<String>) {
        match self {
            PendingAction::ResetTab => (
                "Confirm Reset".to_string(),
                vec![
                    "Are you sure you want to reset this tab?".to_string(),
                    "All current input will be lost.".to_string(),
                ],
            ),
            PendingAction::SaveSettings => (
                "Confirm Save Settings".to_string(),
                vec![
                    "Are you sure you want to save settings?".to_string(),
                    "This will overwrite your configuration file.".to_string(),
                ],
            ),
            PendingAction::DeleteHistoryEntry => (
                "Confirm Delete".to_string(),
                vec![
                    "Are you sure you want to delete this history entry?".to_string(),
                    "This action cannot be undone.".to_string(),
                ],
            ),
            PendingAction::ClearHistory => (
                "Confirm Clear History".to_string(),
                vec![
                    "Are you sure you want to clear all history?".to_string(),
                    "This action cannot be undone.".to_string(),
                ],
            ),
        }
    }

    pub fn execute(self, app: &mut App) {
        match self {
            PendingAction::ResetTab => app.reset_current_tab(),
            PendingAction::SaveSettings => app.save_settings(),
            PendingAction::DeleteHistoryEntry => app.delete_history_entry(),
            PendingAction::ClearHistory => app.clear_all_history(),
        }
    }
}

/// Captures a policy enforcement RequireConfirmation for interactive manual discretion in the TUI,
/// mirroring the CLI CommandContext + ManualOverride flow (narrow --yes semantics, dedicated
/// allow-* flags, stable kebab class strings for audit, etc.).
#[derive(Debug, Clone)]
pub struct PendingPolicyConfirmation {
    pub descriptor: eggsec::config::OperationDescriptor,
    pub decision: eggsec::config::PolicyDecision,
    pub required_classes: Vec<eggsec::config::ConfirmationClass>,
    pub reason_input: String,
    /// The TaskConfig that would have been spawned; replayed on successful manual override.
    pub captured_task_config: Option<crate::workers::TaskConfig>,
}

impl PendingPolicyConfirmation {
    pub fn message(&self) -> (String, Vec<String>) {
        let classes: Vec<String> = self
            .required_classes
            .iter()
            .map(|c| c.as_str().to_string())
            .collect();
        let mut lines = vec![
            format!("Operation: {}", self.descriptor.operation),
            format!("Risk: {:?}", self.descriptor.risk),
            format!(
                "Target: {}",
                self.descriptor.target.as_deref().unwrap_or("<unknown>")
            ),
            format!("Confirmation required for: {}", classes.join(", ")),
        ];
        if !self.decision.denied_reasons.is_empty() {
            lines.push("Reasons:".to_string());
            for r in &self.decision.denied_reasons {
                lines.push(format!("  - {}", r));
            }
        }
        if !self.decision.warnings.is_empty() {
            lines.push("Warnings:".to_string());
            for w in &self.decision.warnings {
                lines.push(format!("  - {}", w));
            }
        }
        lines.push(String::new());
        lines.push(format!(
            "Manual override reason (optional): {}",
            self.reason_input
        ));
        lines.push(String::new());
        lines.push("[Enter] Proceed with override   [Esc] Cancel".to_string());
        ("Policy Confirmation Required".to_string(), lines)
    }
}
