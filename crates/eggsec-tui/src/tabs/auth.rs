use crate::tc;
use crate::app::tab_error::TabError;
use crate::components::{InputField, InputGroup};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::workers::TaskConfig;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
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

pub struct AuthTab {
    pub inputs: InputGroup,
    pub results: String,
    pub state: AppState,
    pub focus_area: AuthFocusArea,
    pub error: Option<TabError>,
    /// Simple local progress (0.0 - 1.0). Can be wired to real worker progress later.
    progress: f64,
}

impl AuthTab {
    pub fn new() -> Self {
        Self {
            inputs: InputGroup::new()
                .add(InputField::new("Target URL").with_width(50).with_value("https://target.lab"))
                .add(InputField::new("Username / Userlist").with_width(40).with_value("admin or users.txt"))
                .add(InputField::new("Password List / Wordlist").with_width(45).with_value("passwords.txt or rockyou.txt"))
                .add(InputField::new("Credential File (optional)").with_width(45).with_value("user:pass file"))
                .add(InputField::new("Max Attempts").with_width(12).with_value("50"))
                .add(InputField::new("Concurrency").with_width(12).with_value("5"))
                .add(InputField::new("Timeout (sec)").with_width(12).with_value("30")),
            results: "Ready for authentication testing. Enter a target and press Enter.".to_string(),
            state: AppState::Idle,
            focus_area: AuthFocusArea::Target,
            error: None,
            progress: 0.0,
        }
    }

    pub fn start(&mut self) {
        self.state = AppState::Running;
        self.error = None;
        self.progress = 0.0;
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

    pub fn reset(&mut self) {
        self.state = AppState::Idle;
        self.error = None;
        self.focus_area = AuthFocusArea::Target;
        self.progress = 0.0;
        self.results = "Ready for authentication testing. Enter a target and press Enter.".to_string();
        for field in &mut self.inputs.fields {
            field.clear();
        }
    }

    fn set_error_state(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
    }

    pub fn set_progress(&mut self, progress: f64) {
        self.progress = progress.clamp(0.0, 1.0);
    }
}

impl TabState for AuthTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        self.progress
    }

    fn reset(&mut self) {
        AuthTab::reset(self);
    }

    fn set_error(&mut self, error: TabError) {
        AuthTab::set_error_state(self, error);
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
        use ratatui::style::Style;
        use crate::components::FormBuilder;

        if let Some(ref err) = self.error {
            let error_text = Paragraph::new(format!("Error: {}", err.message()))
                .block(Block::default().borders(Borders::ALL).title("Auth - Error"))
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, area);
            return;
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(22), // 7 inputs * ~3 lines
                Constraint::Min(8),
            ])
            .split(area);

        let Some(title_area) = layout.get(0) else { return; };
        let Some(inputs_area) = layout.get(1) else { return; };
        let Some(results_area) = layout.get(2) else { return; };

        // Safety banner
        let title = Paragraph::new("Authentication Testing — Defense-lab only | Brute-force, Credential Stuffing, Lockout, Rate-limit, MFA, Timing, Session, Policy")
            .block(Block::default().borders(Borders::ALL).title("⚠ Defense Lab"))
            .style(Style::default().fg(tc!(warning)));
        f.render_widget(title, *title_area);

        let mut builder = FormBuilder::new("Inputs").row_height(3);
        for field in &self.inputs.fields {
            builder = builder.add_input(field.clone());
        }
        builder.render(f, *inputs_area, insert_mode);

        // Results area with better structure
        let results_content = if self.results.is_empty() || self.results.starts_with("Ready") {
            crate::components::empty_state_paragraph("Results", "No results yet. Run a test to see findings.")
        } else {
            Paragraph::new(self.results.as_str())
                .block(Block::default().borders(Borders::ALL).title("Auth Test Results"))
                .style(Style::default().fg(tc!(text)))
        };
        f.render_widget(results_content, *results_area);
    }
}

impl TabInput for AuthTab {
    fn stop(&mut self) {
        AuthTab::stop(self);
    }

    fn handle_focus_next(&mut self) {
        if !self.is_running() {
            self.focus_area = match self.focus_area {
                AuthFocusArea::Target => AuthFocusArea::Username,
                AuthFocusArea::Username => AuthFocusArea::PasswordList,
                AuthFocusArea::PasswordList => AuthFocusArea::CredentialFile,
                AuthFocusArea::CredentialFile => AuthFocusArea::MaxAttempts,
                AuthFocusArea::MaxAttempts => AuthFocusArea::Concurrency,
                AuthFocusArea::Concurrency => AuthFocusArea::Timeout,
                AuthFocusArea::Timeout => AuthFocusArea::Results,
                AuthFocusArea::Results => AuthFocusArea::Target,
            };
            self.sync_input_focus();
        }
    }

    fn handle_focus_prev(&mut self) {
        if !self.is_running() {
            self.focus_area = match self.focus_area {
                AuthFocusArea::Target => AuthFocusArea::Results,
                AuthFocusArea::Username => AuthFocusArea::Target,
                AuthFocusArea::PasswordList => AuthFocusArea::Username,
                AuthFocusArea::CredentialFile => AuthFocusArea::PasswordList,
                AuthFocusArea::MaxAttempts => AuthFocusArea::CredentialFile,
                AuthFocusArea::Concurrency => AuthFocusArea::MaxAttempts,
                AuthFocusArea::Timeout => AuthFocusArea::Concurrency,
                AuthFocusArea::Results => AuthFocusArea::Timeout,
            };
            self.sync_input_focus();
        }
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() {
            if let Some(idx) = self.current_input_index() {
                if let Some(field) = self.inputs.fields.get_mut(idx) {
                    field.insert(c);
                }
            }
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() {
            if let Some(idx) = self.current_input_index() {
                if let Some(field) = self.inputs.fields.get_mut(idx) {
                    field.backspace();
                }
            }
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() {
            if let Some(idx) = self.current_input_index() {
                if let Some(field) = self.inputs.fields.get_mut(idx) {
                    field.paste(text);
                }
            }
        }
    }

    fn handle_word_forward(&mut self) {
        if !self.is_running() {
            if let Some(idx) = self.current_input_index() {
                if let Some(field) = self.inputs.fields.get_mut(idx) {
                    field.move_word_forward();
                }
            }
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() {
            if let Some(idx) = self.current_input_index() {
                if let Some(field) = self.inputs.fields.get_mut(idx) {
                    field.move_word_backward();
                }
            }
        }
    }

    fn handle_home(&mut self) {
        if !self.is_running() {
            if let Some(idx) = self.current_input_index() {
                if let Some(field) = self.inputs.fields.get_mut(idx) {
                    field.move_home();
                }
            }
        }
    }

    fn handle_end(&mut self) {
        if !self.is_running() {
            if let Some(idx) = self.current_input_index() {
                if let Some(field) = self.inputs.fields.get_mut(idx) {
                    field.move_end();
                }
            }
        }
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

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }

        if self.focus_area == AuthFocusArea::Results {
            return;
        }

        if self.target().map_or(true, |t| t.is_empty()) {
            self.set_error_state(TabError::Target("Target URL is required for authentication testing".to_string()));
            return;
        }

        if self.is_input_focused() {
            self.inputs.blur();
        }
        self.start();
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        self.inputs.blur();
        self.focus_area = AuthFocusArea::Results;
        self.sync_input_focus();
    }

    fn handle_up(&mut self) {
        if !self.is_running() {
            match self.focus_area {
                AuthFocusArea::Results => {
                    self.focus_area = AuthFocusArea::Timeout;
                    if self.inputs.fields.len() > 6 { self.inputs.focus(6); }
                }
                AuthFocusArea::Timeout => {
                    self.focus_area = AuthFocusArea::Concurrency;
                    if self.inputs.fields.len() > 5 { self.inputs.focus(5); }
                }
                AuthFocusArea::Concurrency => {
                    self.focus_area = AuthFocusArea::MaxAttempts;
                    if self.inputs.fields.len() > 4 { self.inputs.focus(4); }
                }
                AuthFocusArea::MaxAttempts => {
                    self.focus_area = AuthFocusArea::CredentialFile;
                    if self.inputs.fields.len() > 3 { self.inputs.focus(3); }
                }
                AuthFocusArea::CredentialFile => {
                    self.focus_area = AuthFocusArea::PasswordList;
                    if self.inputs.fields.len() > 2 { self.inputs.focus(2); }
                }
                AuthFocusArea::PasswordList => {
                    self.focus_area = AuthFocusArea::Username;
                    if self.inputs.fields.len() > 1 { self.inputs.focus(1); }
                }
                AuthFocusArea::Username => {
                    self.focus_area = AuthFocusArea::Target;
                    if self.inputs.fields.len() > 0 { self.inputs.focus(0); }
                }
                AuthFocusArea::Target => {
                    self.inputs.focus_prev();
                    if !self.inputs.is_focused() {
                        if self.inputs.fields.is_empty() { return; }
                        self.inputs.focus(self.inputs.fields.len() - 1);
                    }
                }
            }
            self.sync_input_focus();
        }
    }

    fn handle_down(&mut self) {
        if !self.is_running() {
            match self.focus_area {
                AuthFocusArea::Target => {
                    self.focus_area = AuthFocusArea::Username;
                    if self.inputs.fields.len() > 1 { self.inputs.focus(1); }
                }
                AuthFocusArea::Username => {
                    self.focus_area = AuthFocusArea::PasswordList;
                    if self.inputs.fields.len() > 2 { self.inputs.focus(2); }
                }
                AuthFocusArea::PasswordList => {
                    self.focus_area = AuthFocusArea::CredentialFile;
                    if self.inputs.fields.len() > 3 { self.inputs.focus(3); }
                }
                AuthFocusArea::CredentialFile => {
                    self.focus_area = AuthFocusArea::MaxAttempts;
                    if self.inputs.fields.len() > 4 { self.inputs.focus(4); }
                }
                AuthFocusArea::MaxAttempts => {
                    self.focus_area = AuthFocusArea::Concurrency;
                    if self.inputs.fields.len() > 5 { self.inputs.focus(5); }
                }
                AuthFocusArea::Concurrency => {
                    self.focus_area = AuthFocusArea::Timeout;
                    if self.inputs.fields.len() > 6 { self.inputs.focus(6); }
                }
                AuthFocusArea::Timeout => {
                    self.focus_area = AuthFocusArea::Results;
                    self.inputs.blur();
                }
                AuthFocusArea::Results => {}
            }
            self.sync_input_focus();
        }
    }

    fn handle_left(&mut self) -> bool {
        if !self.is_running() {
            self.inputs.move_left()
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if !self.is_running() {
            self.inputs.move_right()
        } else {
            false
        }
    }

    fn is_at_left_edge(&self) -> bool {
        if self.is_input_focused() {
            self.inputs.is_at_left_edge()
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.is_input_focused() {
            self.inputs.is_at_right_edge()
        } else {
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        !matches!(self.focus_area, AuthFocusArea::Results)
    }
}

impl AuthTab {
    fn current_input_index(&self) -> Option<usize> {
        match self.focus_area {
            AuthFocusArea::Target if self.inputs.fields.len() > 0 => Some(0),
            AuthFocusArea::Username if self.inputs.fields.len() > 1 => Some(1),
            AuthFocusArea::PasswordList if self.inputs.fields.len() > 2 => Some(2),
            AuthFocusArea::CredentialFile if self.inputs.fields.len() > 3 => Some(3),
            AuthFocusArea::MaxAttempts if self.inputs.fields.len() > 4 => Some(4),
            AuthFocusArea::Concurrency if self.inputs.fields.len() > 5 => Some(5),
            AuthFocusArea::Timeout if self.inputs.fields.len() > 6 => Some(6),
            _ => None,
        }
    }

    fn sync_input_focus(&mut self) {
        let idx = self.current_input_index();
        for (i, field) in self.inputs.fields.iter_mut().enumerate() {
            field.focused = Some(i) == idx;
        }
    }

    pub fn target(&self) -> Option<&str> {
        self.inputs.fields.first().map(|f| f.value.as_str()).filter(|v| !v.is_empty())
    }

    pub fn username(&self) -> Option<&str> {
        self.inputs.fields.get(1).map(|f| f.value.as_str()).filter(|v| !v.is_empty())
    }

    pub fn password_list(&self) -> Option<&str> {
        self.inputs.fields.get(2).map(|f| f.value.as_str()).filter(|v| !v.is_empty())
    }

    pub fn credential_file(&self) -> Option<&str> {
        self.inputs.fields.get(3).map(|f| f.value.as_str()).filter(|v| !v.is_empty())
    }

    pub fn max_attempts(&self) -> usize {
        self.inputs.fields.get(4)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(50)
    }

    pub fn concurrency(&self) -> usize {
        self.inputs.fields.get(5)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(5)
    }

    pub fn timeout(&self) -> u64 {
        self.inputs.fields.get(6)
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
        if ma != 50 { cmd.push_str(&format!(" --max-attempts {}", ma)); }
        let conc = self.concurrency();
        if conc != 5 { cmd.push_str(&format!(" --concurrency {}", conc)); }
        let to = self.timeout();
        if to != 30 { cmd.push_str(&format!(" --timeout {}", to)); }
        Some(cmd)
    }

    /// Produces TaskConfig using values from the UI fields.
    pub fn build_task_config(&self) -> Option<TaskConfig> {
        let target = self.target()?.to_string();
        if target.is_empty() {
            return None;
        }

        Some(TaskConfig::Auth {
            target,
            username: self.username().map(|s| s.to_string()),
            password_list: self.password_list().map(|s| s.to_string()),
            credential_file: self.credential_file().map(|s| s.to_string()),
            max_attempts: self.max_attempts(),
            concurrency: self.concurrency(),
            timeout: self.timeout(),
        })
    }

    /// Update results from a completed AuthTestReport (called externally when task finishes).
    pub fn set_results_from_report(&mut self, report: &eggsec::auth::AuthTestReport) {
        let mut out = format!("Target: {}\nTests run: {}\nTotal attempts: {}\n\n", 
            report.target, report.tests_run.len(), report.total_attempts);

        if !report.findings.is_empty() {
            out.push_str("Findings:\n");
            for f in &report.findings {
                out.push_str(&format!("  [{}] {}\n", f.severity, f.description));
            }
        } else {
            out.push_str("No significant findings.\n");
        }
        self.results = out;
        self.progress = 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_returns_none_when_empty() {
        let mut tab = AuthTab::new();
        tab.inputs.fields[0].value.clear();
        assert!(tab.target().is_none());
    }

    #[test]
    fn test_target_returns_value_when_set() {
        let mut tab = AuthTab::new();
        tab.inputs.fields[0].value = "http://example.com".to_string();
        assert_eq!(tab.target(), Some("http://example.com"));
    }

    #[test]
    fn test_build_task_config_returns_none_without_target() {
        let mut tab = AuthTab::new();
        tab.inputs.fields[0].value.clear();
        assert!(tab.build_task_config().is_none());
    }

    #[test]
    fn test_build_task_config_uses_ui_values() {
        let mut tab = AuthTab::new();
        tab.inputs.fields[0].value = "https://target.lab".to_string();
        tab.inputs.fields[4].value = "100".to_string(); // max_attempts
        tab.inputs.fields[5].value = "10".to_string();   // concurrency

        let config = tab.build_task_config().unwrap();
        match config {
            TaskConfig::Auth { target, max_attempts, concurrency, .. } => {
                assert_eq!(target, "https://target.lab");
                assert_eq!(max_attempts, 100);
                assert_eq!(concurrency, 10);
            }
            _ => panic!("Expected TaskConfig::Auth"),
        }
    }
}