use crate::app::tab_error::TabError;
use crate::components::InputField;
use crate::tabs::core::{render_results_area, TabCore};
use crate::tabs::{AppState, TabInput, TabRender, TabState};

use crate::{tab_input_indexed, tab_state_boilerplate, tc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthFocusArea {
    Target,
    Username,
    PasswordList,
    CredentialFile,
    MaxAttempts,
    Concurrency,
    Timeout,
    Results,
}

const AUTH_INPUT_AREAS: &[AuthFocusArea] = &[
    AuthFocusArea::Target,
    AuthFocusArea::Username,
    AuthFocusArea::PasswordList,
    AuthFocusArea::CredentialFile,
    AuthFocusArea::MaxAttempts,
    AuthFocusArea::Concurrency,
    AuthFocusArea::Timeout,
];

pub struct AuthTab {
    pub core: TabCore,
    pub focus_area: AuthFocusArea,
}

impl AuthTab {
    pub fn new() -> Self {
        let inputs = crate::components::InputGroup::new()
            .add(
                InputField::new("Target URL")
                    .with_width(50)
                    .with_value("https://target.lab"),
            )
            .add(
                InputField::new("Username / Userlist")
                    .with_width(40)
                    .with_value("admin or users.txt"),
            )
            .add(
                InputField::new("Password List / Wordlist")
                    .with_width(45)
                    .with_value("passwords.txt or rockyou.txt"),
            )
            .add(
                InputField::new("Credential File (optional)")
                    .with_width(45)
                    .with_value("user:pass file"),
            )
            .add(
                InputField::new("Max Attempts")
                    .with_width(12)
                    .with_value("50"),
            )
            .add(
                InputField::new("Concurrency")
                    .with_width(12)
                    .with_value("5"),
            )
            .add(
                InputField::new("Timeout (sec)")
                    .with_width(12)
                    .with_value("30"),
            );

        Self {
            core: TabCore::new("Running auth tests...", "Auth Results").with_inputs(inputs),
            focus_area: AuthFocusArea::Target,
        }
    }

    fn sync_input_focus(&mut self) {
        crate::tabs::core::sync_indexed_input_focus(
            &mut self.core,
            self.focus_area,
            AUTH_INPUT_AREAS,
        );
    }

    pub fn target(&self) -> Option<&str> {
        self.core
            .inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .filter(|v| !v.is_empty())
    }

    pub fn username(&self) -> Option<&str> {
        self.core
            .inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .filter(|v| !v.is_empty())
    }

    pub fn password_list(&self) -> Option<&str> {
        self.core
            .inputs
            .fields
            .get(2)
            .map(|f| f.value.as_str())
            .filter(|v| !v.is_empty())
    }

    pub fn credential_file(&self) -> Option<&str> {
        self.core
            .inputs
            .fields
            .get(3)
            .map(|f| f.value.as_str())
            .filter(|v| !v.is_empty())
    }

    pub fn max_attempts(&self) -> usize {
        self.core
            .inputs
            .fields
            .get(4)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(50)
    }

    pub fn concurrency(&self) -> usize {
        self.core
            .inputs
            .fields
            .get(5)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(5)
    }

    pub fn timeout(&self) -> u64 {
        self.core
            .inputs
            .fields
            .get(6)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(30)
    }

    pub fn primary_target(&self) -> Option<String> {
        self.target().map(|s| s.to_string())
    }

    pub fn build_cli_equivalent(&self) -> Option<String> {
        let target = self.target()?;
        let mut cmd = format!("eggsec auth-test {}", target);
        if let Some(u) = self.username() {
            cmd.push_str(&format!(" --username {}", u));
        }
        if let Some(p) = self.password_list() {
            cmd.push_str(&format!(" --wordlist {}", p));
        }
        if let Some(c) = self.credential_file() {
            cmd.push_str(&format!(" --credential-file {}", c));
        }
        let ma = self.max_attempts();
        if ma != 50 {
            cmd.push_str(&format!(" --max-attempts {}", ma));
        }
        let conc = self.concurrency();
        if conc != 5 {
            cmd.push_str(&format!(" --concurrency {}", conc));
        }
        let to = self.timeout();
        if to != 30 {
            cmd.push_str(&format!(" --timeout {}", to));
        }
        Some(cmd)
    }

    pub fn set_results_from_report(&mut self, report: &eggsec::auth::AuthTestReport) {
        let mut out = format!(
            "Target: {}\nTests run: {}\nTotal attempts: {}\n\n",
            report.target,
            report.tests_run.len(),
            report.total_attempts
        );

        if !report.findings.is_empty() {
            out.push_str("Findings:\n");
            for f in &report.findings {
                out.push_str(&format!("  [{}] {}\n", f.severity, f.description));
            }
        } else {
            out.push_str("No significant findings.\n");
        }
        let view = self.core.prepare_results();
        for line in out.lines() {
            view.add_line(ratatui::text::Line::from(line.to_string()));
        }
    }
}

impl Default for AuthTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for AuthTab {
    tab_state_boilerplate!(AuthTab, core: core);

    fn reset(&mut self) {
        self.core.reset_all();
        self.core.inputs.clear_all_fields();
        self.focus_area = AuthFocusArea::Target;
    }
}

impl TabRender for AuthTab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let focus = match self.focus_area {
            AuthFocusArea::Target => "Target",
            AuthFocusArea::Username => "Username",
            AuthFocusArea::PasswordList => "Password List",
            AuthFocusArea::CredentialFile => "Cred File",
            AuthFocusArea::MaxAttempts => "Max Attempts",
            AuthFocusArea::Concurrency => "Concurrency",
            AuthFocusArea::Timeout => "Timeout",
            AuthFocusArea::Results => "Results",
        };
        Some(vec!["Auth", focus])
    }

    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        use crate::components::FormBuilder;

        if let Some(ref err) = self.core.error {
            crate::tabs::core::render_error_block(f, area, "Auth - Error", err);
            return;
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(22),
                Constraint::Min(8),
            ])
            .split(area);

        let Some(title_area) = layout.first() else {
            return;
        };
        let Some(inputs_area) = layout.get(1) else {
            return;
        };
        let Some(results_area) = layout.get(2) else {
            return;
        };

        let title = Paragraph::new("Authentication Testing — Defense-lab only | Brute-force, Credential Stuffing, Lockout, Rate-limit, MFA, Timing, Session, Policy")
            .block(Block::default().borders(Borders::ALL).title("⚠ Defense Lab").border_style(Style::default().fg(tc!(border))))
            .style(Style::default().fg(tc!(warning)));
        f.render_widget(title, *title_area);

        let mut builder = FormBuilder::new("Inputs").row_height(3);
        for field in &self.core.inputs.fields {
            builder = builder.add_input(field.clone());
        }
        builder.render(f, *inputs_area, insert_mode);

        render_results_area(
            f,
            *results_area,
            &self.core.state,
            &self.core.error,
            &self.core.results_view,
            &self.core.progress,
            "Auth Test Results",
            "Ready for authentication testing. Enter a target and press Enter.",
        );
    }
}

impl TabInput for AuthTab {
    fn stop(&mut self) {
        self.core.stop();
    }

    tab_input_indexed!(
        AuthTab,
        core: core,
        focus: focus_area,
        InputAreas: AUTH_INPUT_AREAS,
        Results: AuthFocusArea::Results
    );

    fn handle_focus_next(&mut self) {
        self.focus_area = crate::tabs::core::focus_next_indexed(
            self.focus_area,
            AUTH_INPUT_AREAS,
            AuthFocusArea::Results,
        );
        self.sync_input_focus();
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = crate::tabs::core::focus_prev_indexed(
            self.focus_area,
            AUTH_INPUT_AREAS,
            AuthFocusArea::Results,
        );
        self.sync_input_focus();
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }

        if self.focus_area == AuthFocusArea::Results {
            return;
        }

        if self.target().map_or(true, |t| t.is_empty()) {
            let err =
                TabError::Target("Target URL is required for authentication testing".to_string());
            self.core.state = AppState::Error(err.message());
            self.core.error = Some(err);
            return;
        }

        if self.is_input_focused() {
            self.core.inputs.blur();
        }
        self.core.state = AppState::Running;
        self.core.error = None;
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        self.core.inputs.blur();
        self.focus_area = AuthFocusArea::Target;
        self.sync_input_focus();
    }

    fn handle_up(&mut self) {
        self.focus_area = crate::tabs::core::focus_up_indexed(
            self.focus_area,
            AUTH_INPUT_AREAS,
            AuthFocusArea::Results,
        );
        self.sync_input_focus();
    }

    fn handle_down(&mut self) {
        self.focus_area = crate::tabs::core::focus_down_indexed(
            self.focus_area,
            AUTH_INPUT_AREAS,
            AuthFocusArea::Results,
        );
        self.sync_input_focus();
    }

    fn handle_top(&mut self) {
        if !self.is_running() {
            self.focus_area = AuthFocusArea::Target;
            self.sync_input_focus();
        }
    }

    fn handle_bottom(&mut self) {
        if !self.is_running() {
            self.focus_area = AuthFocusArea::Results;
            self.sync_input_focus();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_returns_none_when_empty() {
        let mut tab = AuthTab::new();
        tab.core.inputs.fields[0].value.clear();
        assert!(tab.target().is_none());
    }

    #[test]
    fn test_target_returns_value_when_set() {
        let mut tab = AuthTab::new();
        tab.core.inputs.fields[0].value = "http://example.com".to_string();
        assert_eq!(tab.target(), Some("http://example.com"));
    }

    #[test]
    fn test_build_run_request_returns_none_without_target() {
        use crate::app::task_management::TaskBuilder;
        let mut tab = AuthTab::new();
        tab.core.inputs.fields[0].value.clear();
        assert!(tab.build_run_request().is_none());
    }

    #[test]
    fn test_build_run_request_uses_ui_values() {
        use crate::app::task_management::TaskBuilder;
        let mut tab = AuthTab::new();
        tab.core.inputs.fields[0].value = "https://target.lab".to_string();

        let req = tab.build_run_request().unwrap();
        match req.task_kind {
            eggsec_runtime::request::TaskKind::AuthTest(params) => {
                assert_eq!(params.target, "https://target.lab");
            }
            _ => panic!("Expected TaskKind::AuthTest"),
        }
    }

    #[test]
    fn non_first_input_fields_are_editable() {
        let mut tab = AuthTab::new();
        tab.handle_focus_next();
        assert_eq!(tab.focus_area, AuthFocusArea::Username);
        assert!(tab.is_input_focused());

        tab.core.inputs.fields[1].clear();
        tab.handle_char('a');

        assert_eq!(tab.core.inputs.fields[1].value, "a");
    }

    #[test]
    fn input_labels_are_unique() {
        let tab = AuthTab::new();
        assert_eq!(
            tab.core.inputs.duplicate_label_names(),
            Vec::<String>::new()
        );
    }
}
