use crate::integrations::{IntegrationConfig, Issue};
use crate::tc;
use crate::tui::app::tab_error::TabError;
use crate::tui::components::{
    empty_state_paragraph, InputField, InputGroup, ScrollableText, Selector, SelectorItem,
};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

pub struct IntegrationsTab {
    pub config_inputs: InputGroup,
    pub issue_inputs: InputGroup,
    pub tracker_selector: Selector,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub focus_area: IntegrationsFocusArea,
    pub current_mode: IntegrationsMode,
    pub error: Option<TabError>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntegrationsFocusArea {
    Tracker,
    Config,
    Issue,
    Results,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntegrationsMode {
    Configure,
    CreateIssue,
    SearchIssues,
}

impl IntegrationsTab {
    pub fn new() -> Self {
        let config_inputs = InputGroup::new()
            .add(InputField::new("URL"))
            .add(InputField::new("Username / Token"))
            .add(InputField::new("Project Key / Owner / Repo"))
            .add(InputField::new("Password / API Token"));

        let issue_inputs = InputGroup::new()
            .add(InputField::new("Issue Title"))
            .add(InputField::new("Description"))
            .add(InputField::new("Labels (comma-separated)"))
            .add(InputField::new("Assignees (comma-separated)"))
            .add(InputField::new("Search Query"));

        let tracker_selector = Selector::new("Tracker").items(vec![
            SelectorItem::new("Jira", "jira"),
            SelectorItem::new("GitHub", "github"),
            SelectorItem::new("GitLab", "gitlab"),
        ]);

        Self {
            config_inputs,
            issue_inputs,
            tracker_selector,
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            focus_area: IntegrationsFocusArea::Tracker,
            current_mode: IntegrationsMode::Configure,
            error: None,
        }
    }

    pub fn tracker_url(&self) -> &str {
        self.config_inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn tracker_token(&self) -> &str {
        self.config_inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn tracker_project(&self) -> &str {
        self.config_inputs
            .fields
            .get(2)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn tracker_password(&self) -> &str {
        self.config_inputs
            .fields
            .get(3)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn issue_title(&self) -> &str {
        self.issue_inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn issue_description(&self) -> &str {
        self.issue_inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn issue_labels(&self) -> Vec<String> {
        self.issue_inputs
            .fields
            .get(2)
            .map(|f| {
                f.value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn issue_assignees(&self) -> Vec<String> {
        self.issue_inputs
            .fields
            .get(3)
            .map(|f| {
                f.value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn search_query(&self) -> &str {
        self.issue_inputs
            .fields
            .get(4)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn get_config(&self) -> IntegrationConfig {
        use crate::integrations::{github::GitHubConfig, gitlab::GitLabConfig, jira::JiraConfig};
        use crate::types::SensitiveString;

        let mut config = IntegrationConfig::default();

        match self.tracker_selector.selected_value().unwrap_or("") {
            "jira" => {
                config.jira = Some(JiraConfig {
                    url: self.tracker_url().to_string(),
                    username: self.tracker_token().to_string(),
                    api_token: SensitiveString::new(self.tracker_password().to_string()),
                    project_key: self.tracker_project().to_string(),
                });
            }
            "github" => {
                config.github = Some(GitHubConfig {
                    owner: self.tracker_token().to_string(),
                    repo: self.tracker_project().to_string(),
                    api_token: SensitiveString::new(self.tracker_password().to_string()),
                });
            }
            "gitlab" => {
                config.gitlab = Some(GitLabConfig {
                    url: self.tracker_url().to_string(),
                    project_id: self.tracker_project().to_string(),
                    api_token: SensitiveString::new(self.tracker_password().to_string()),
                });
            }
            _ => {}
        }

        config
    }

    pub fn build_issue(&self) -> Issue {
        Issue {
            id: None,
            title: self.issue_title().to_string(),
            description: self.issue_description().to_string(),
            labels: self.issue_labels(),
            severity: None,
            assignees: self.issue_assignees(),
            status: None,
            url: None,
            created_at: None,
        }
    }

    pub fn get_mode(&self) -> &str {
        match self.current_mode {
            IntegrationsMode::Configure => "configure",
            IntegrationsMode::CreateIssue => "create_issue",
            IntegrationsMode::SearchIssues => "search_issues",
        }
    }

    pub fn get_issue_params(&self) -> (String, String) {
        (
            self.issue_title().to_string(),
            self.issue_description().to_string(),
        )
    }

    pub fn start(&mut self) {
        self.state = AppState::Running;
        self.results_view.clear();
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

    pub fn set_result(&mut self, success: bool, message: &str) {
        self.state = if success {
            AppState::Completed
        } else {
            AppState::Error(message.to_string())
        };
        self.results_view.clear();
        let color = if success { tc!(success) } else { tc!(error) };
        self.results_view.add_line(Line::from(Span::styled(
            message.to_string(),
            Style::default().fg(color),
        )));
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.results_view.page_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.results_view.page_down(page_size);
    }
}

impl Default for IntegrationsTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for IntegrationsTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        0.0
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.results_view.clear();
        self.error = None;
        self.focus_area = IntegrationsFocusArea::Tracker;
        self.current_mode = IntegrationsMode::Configure;
        self.tracker_selector.selected = 0;
        for field in &mut self.config_inputs.fields {
            field.clear();
        }
        for field in &mut self.issue_inputs.fields {
            field.clear();
        }
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error.clone());
        self.results_view.add_line(Line::from(Span::styled(
            format!("Error: {}", error.message()),
            Style::default().fg(tc!(error)),
        )));
    }
}

impl TabRender for IntegrationsTab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let focus = match self.focus_area {
            IntegrationsFocusArea::Tracker => "Tracker",
            IntegrationsFocusArea::Config => "Config",
            IntegrationsFocusArea::Issue => "Issue",
            IntegrationsFocusArea::Results => "Results",
        };
        Some(vec!["Integrations", focus])
    }

    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let input_height = match self.current_mode {
            IntegrationsMode::Configure => 15,
            IntegrationsMode::CreateIssue => 18,
            IntegrationsMode::SearchIssues => 9,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(input_height), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        let mut sel = self.tracker_selector.clone();
        sel.focused = self.focus_area == IntegrationsFocusArea::Tracker;
        sel.render(f, input_area);

        let fields_area = Rect {
            y: input_area.y + 3,
            height: input_area.height - 3,
            ..input_area
        };

        let fields: &[InputField] = match self.current_mode {
            IntegrationsMode::Configure => &self.config_inputs.fields,
            IntegrationsMode::CreateIssue => {
                self.issue_inputs.fields.get(..4).unwrap_or(&self.issue_inputs.fields)
            }
            IntegrationsMode::SearchIssues => {
                self.issue_inputs.fields.get(4..).unwrap_or(&self.issue_inputs.fields)
            }
        };

        let field_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(3); fields.len()])
            .split(fields_area);

        for (i, field) in fields.iter().enumerate() {
            if let Some(chunk) = field_chunks.get(i) {
                field.render(f, *chunk, insert_mode);
            }
        }

        if self.state == AppState::Running {
            let gauge = ratatui::widgets::Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Processing..."),
                )
                .gauge_style(Style::default().fg(tc!(warning)))
                .ratio(0.5);
            f.render_widget(gauge, results_area);
        } else if !self.results_view.is_empty() {
            self.results_view
                .render(f, results_area, Some(tc!(success)));
        } else {
            let placeholder = empty_state_paragraph(
                "Issue Tracker Integration",
                "Select tracker, configure, and press Enter",
            );
            f.render_widget(placeholder, results_area);
        }
    }
}

impl TabInput for IntegrationsTab {
    fn handle_focus_next(&mut self) {
        self.focus_area = match self.focus_area {
            IntegrationsFocusArea::Tracker => {
                self.tracker_selector.blur();
                self.config_inputs.focus(0);
                IntegrationsFocusArea::Config
            }
            IntegrationsFocusArea::Config => IntegrationsFocusArea::Issue,
            IntegrationsFocusArea::Issue => IntegrationsFocusArea::Results,
            IntegrationsFocusArea::Results => {
                self.tracker_selector.focus();
                IntegrationsFocusArea::Tracker
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = match self.focus_area {
            IntegrationsFocusArea::Tracker => IntegrationsFocusArea::Results,
            IntegrationsFocusArea::Config => {
                self.tracker_selector.focus();
                IntegrationsFocusArea::Tracker
            }
            IntegrationsFocusArea::Issue => IntegrationsFocusArea::Config,
            IntegrationsFocusArea::Results => IntegrationsFocusArea::Issue,
        };
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() {
            #[allow(clippy::single_char_pattern)]
            if self.focus_area == IntegrationsFocusArea::Config {
                self.config_inputs.insert(c);
            } else if self.focus_area == IntegrationsFocusArea::Issue {
                self.issue_inputs.insert(c);
            }
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() {
            if self.focus_area == IntegrationsFocusArea::Config {
                self.config_inputs.backspace();
            } else if self.focus_area == IntegrationsFocusArea::Issue {
                self.issue_inputs.backspace();
            }
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() {
            if self.focus_area == IntegrationsFocusArea::Config {
                self.config_inputs.paste(text);
            } else if self.focus_area == IntegrationsFocusArea::Issue {
                self.issue_inputs.paste(text);
            }
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if !self.is_running() {
            if self.focus_area == IntegrationsFocusArea::Config {
                self.config_inputs.get_focused_value()
            } else if self.focus_area == IntegrationsFocusArea::Issue {
                self.issue_inputs.get_focused_value()
            } else if self.focus_area == IntegrationsFocusArea::Results {
                Some(self.results_view.get_content())
            } else {
                None
            }
        } else {
            None
        }
    }

    fn handle_word_forward(&mut self) {
        if !self.is_running() {
            if self.focus_area == IntegrationsFocusArea::Config {
                self.config_inputs.move_word_forward();
            } else if self.focus_area == IntegrationsFocusArea::Issue {
                self.issue_inputs.move_word_forward();
            }
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() {
            if self.focus_area == IntegrationsFocusArea::Config {
                self.config_inputs.move_word_backward();
            } else if self.focus_area == IntegrationsFocusArea::Issue {
                self.issue_inputs.move_word_backward();
            }
        }
    }

    fn handle_home(&mut self) {
        if !self.is_running() {
            if self.focus_area == IntegrationsFocusArea::Config {
                self.config_inputs.move_home();
            } else if self.focus_area == IntegrationsFocusArea::Issue {
                self.issue_inputs.move_home();
            } else if self.focus_area == IntegrationsFocusArea::Results {
                self.results_view.scroll_to_top();
            }
        }
    }

    fn handle_end(&mut self) {
        if !self.is_running() {
            if self.focus_area == IntegrationsFocusArea::Config {
                self.config_inputs.move_end();
            } else if self.focus_area == IntegrationsFocusArea::Issue {
                self.issue_inputs.move_end();
            } else if self.focus_area == IntegrationsFocusArea::Results {
                self.results_view.scroll_to_bottom();
            }
        }
    }

    fn handle_top(&mut self) {
        self.focus_area = IntegrationsFocusArea::Tracker;
        self.tracker_selector.focus();
    }

    fn handle_bottom(&mut self) {
        self.focus_area = IntegrationsFocusArea::Results;
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        match self.focus_area {
            IntegrationsFocusArea::Tracker => {
                self.tracker_selector.handle_enter();
            }
            IntegrationsFocusArea::Config => {
                self.config_inputs.blur();
            }
            IntegrationsFocusArea::Issue => {
                self.issue_inputs.blur();
            }
            IntegrationsFocusArea::Results => {}
        }

        self.start();
    }

    fn handle_escape(&mut self) {
        self.tracker_selector.blur();
        self.config_inputs.blur();
        self.issue_inputs.blur();
    }

    fn handle_up(&mut self) {
        if !self.is_running() {
            match self.focus_area {
                IntegrationsFocusArea::Tracker => self.tracker_selector.handle_up(),
                IntegrationsFocusArea::Config => self.config_inputs.focus_prev(),
                IntegrationsFocusArea::Issue => self.issue_inputs.focus_prev(),
                IntegrationsFocusArea::Results => self.results_view.scroll_up(1),
            }
        }
    }

    fn handle_down(&mut self) {
        if !self.is_running() {
            match self.focus_area {
                IntegrationsFocusArea::Tracker => self.tracker_selector.handle_down(),
                IntegrationsFocusArea::Config => self.config_inputs.focus_next(),
                IntegrationsFocusArea::Issue => self.issue_inputs.focus_next(),
                IntegrationsFocusArea::Results => self.results_view.scroll_down(1),
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        if !self.is_running() {
            match self.focus_area {
                IntegrationsFocusArea::Config => self.config_inputs.move_left(),
                IntegrationsFocusArea::Issue => self.issue_inputs.move_left(),
                _ => false,
            }
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if !self.is_running() {
            match self.focus_area {
                IntegrationsFocusArea::Config => self.config_inputs.move_right(),
                IntegrationsFocusArea::Issue => self.issue_inputs.move_right(),
                _ => false,
            }
        } else {
            false
        }
    }

    fn is_at_left_edge(&self) -> bool {
        match self.focus_area {
            IntegrationsFocusArea::Tracker => {
                self.tracker_selector.items.is_empty()
                    || self.tracker_selector.selected == 0
            }
            IntegrationsFocusArea::Config => self.config_inputs.is_at_left_edge(),
            IntegrationsFocusArea::Issue => self.issue_inputs.is_at_left_edge(),
            _ => true,
        }
    }

    fn is_at_right_edge(&self) -> bool {
        match self.focus_area {
            IntegrationsFocusArea::Tracker => {
                self.tracker_selector.items.is_empty()
                    || self.tracker_selector.selected
                        >= self.tracker_selector.items.len().saturating_sub(1)
            }
            IntegrationsFocusArea::Config => self.config_inputs.is_at_right_edge(),
            IntegrationsFocusArea::Issue => self.issue_inputs.is_at_right_edge(),
            _ => true,
        }
    }

    fn is_input_focused(&self) -> bool {
        (self.focus_area == IntegrationsFocusArea::Tracker && self.tracker_selector.is_focused())
            || (self.focus_area == IntegrationsFocusArea::Config && self.config_inputs.is_focused())
            || (self.focus_area == IntegrationsFocusArea::Issue && self.issue_inputs.is_focused())
    }
}
