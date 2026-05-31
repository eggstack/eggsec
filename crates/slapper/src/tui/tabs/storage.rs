use crate::storage::{models::StoredFinding, models::StoredScan, StorageConfig};
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

pub struct StorageTab {
    pub config_inputs: InputGroup,
    pub query_inputs: InputGroup,
    pub mode_selector: Selector,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub connected: bool,
    pub scans: Vec<StoredScan>,
    pub findings: Vec<StoredFinding>,
    pub focus_area: StorageFocusArea,
    pub current_mode: StorageMode,
    pub error: Option<TabError>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageFocusArea {
    Config,
    Mode,
    Query,
    Results,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageMode {
    Connect,
    ListScans,
    ListFindings,
    SearchByCve,
}

impl StorageTab {
    pub fn new() -> Self {
        let config_inputs = InputGroup::new()
            .add(InputField::new("Host").with_value("localhost"))
            .add(InputField::new("Port").with_value("5432"))
            .add(InputField::new("Database"))
            .add(InputField::new("Username"))
            .add(InputField::new("Password"));

        let query_inputs = InputGroup::new()
            .add(InputField::new("Scan ID / CVE ID"))
            .add(InputField::new("Severity Filter (optional)"));

        let mode_selector = Selector::new("Mode").items(vec![
            SelectorItem::new("Connect", "connect"),
            SelectorItem::new("List Scans", "list_scans"),
            SelectorItem::new("List Findings", "list_findings"),
            SelectorItem::new("Search by CVE", "search_cve"),
        ]);

        Self {
            config_inputs,
            query_inputs,
            mode_selector,
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            connected: false,
            scans: Vec::new(),
            findings: Vec::new(),
            focus_area: StorageFocusArea::Config,
            current_mode: StorageMode::Connect,
            error: None,
        }
    }

    pub fn host(&self) -> &str {
        self.config_inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn port(&self) -> u16 {
        self.config_inputs
            .fields
            .get(1)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(5432)
    }

    pub fn database(&self) -> &str {
        self.config_inputs
            .fields
            .get(2)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn username(&self) -> &str {
        self.config_inputs
            .fields
            .get(3)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn password(&self) -> &str {
        self.config_inputs
            .fields
            .get(4)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn query_id(&self) -> &str {
        self.query_inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn severity_filter(&self) -> Option<&str> {
        self.query_inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .filter(|v| !v.is_empty())
    }

    pub fn get_config(&self) -> StorageConfig {
        StorageConfig {
            host: self.host().to_string(),
            port: self.port(),
            database: self.database().to_string(),
            username: self.username().to_string(),
            password: self.password().to_string().into(),
            max_connections: 5,
        }
    }

    pub fn get_mode(&self) -> &str {
        match self.current_mode {
            StorageMode::Connect => "connect",
            StorageMode::ListScans => "list_scans",
            StorageMode::ListFindings => "list_findings",
            StorageMode::SearchByCve => "search_cve",
        }
    }

    pub fn start(&mut self) {
        self.state = AppState::Running;
        self.results_view.clear();
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
        self.state = if connected {
            AppState::Completed
        } else {
            AppState::Error("Connection failed".to_string())
        };
        self.results_view.clear();
        if connected {
            self.results_view.add_line(Line::from(Span::styled(
                "Connected to database",
                Style::default().fg(tc!(success)),
            )));
        } else {
            self.results_view.add_line(Line::from(Span::styled(
                "Failed to connect to database",
                Style::default().fg(tc!(error)),
            )));
        }
    }

    pub fn set_scans(&mut self, scans: Vec<StoredScan>) {
        self.scans = scans.clone();
        self.state = AppState::Completed;
        self.results_view.clear();
        self.results_view.add_line(Line::from(Span::styled(
            format!("Recent Scans ({}):", scans.len()),
            Style::default().fg(tc!(warning)),
        )));
        self.results_view.add_line(Line::from(""));
        for scan in &scans {
            self.results_view.add_line(Line::from(format!(
                "  {} - {} - {:?} ({} findings)",
                scan.id, scan.target, scan.status, scan.findings_count
            )));
        }
    }

    pub fn set_findings(&mut self, findings: Vec<StoredFinding>) {
        self.findings = findings.clone();
        self.state = AppState::Completed;
        self.results_view.clear();
        self.results_view.add_line(Line::from(Span::styled(
            format!("Findings ({}):", findings.len()),
            Style::default().fg(tc!(warning)),
        )));
        self.results_view.add_line(Line::from(""));
        for finding in &findings {
            self.results_view.add_line(Line::from(format!(
                "  [{}] {} - {:?} ({})",
                finding.severity, finding.title, finding.status, finding.id
            )));
        }
    }

}

impl Default for StorageTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for StorageTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        0.0
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.connected = false;
        self.scans.clear();
        self.findings.clear();
        self.results_view.clear();
        self.error = None;
        self.config_inputs.blur();
        for field in &mut self.config_inputs.fields {
            field.clear();
        }
        if let Some(f) = self.config_inputs.fields.get_mut(0) {
            f.value = "localhost".to_string();
        }
        if let Some(f) = self.config_inputs.fields.get_mut(1) {
            f.value = "5432".to_string();
        }
        self.query_inputs.blur();
        for field in &mut self.query_inputs.fields {
            field.clear();
        }
        self.mode_selector.select(0);
        self.current_mode = StorageMode::Connect;
        self.focus_area = StorageFocusArea::Config;
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
    }
}

impl TabRender for StorageTab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let focus = match self.focus_area {
            StorageFocusArea::Config => "Config",
            StorageFocusArea::Mode => "Mode",
            StorageFocusArea::Query => "Query",
            StorageFocusArea::Results => "Results",
        };
        Some(vec!["Storage", focus])
    }

    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(if self.current_mode == StorageMode::Connect {
                    18
                } else {
                    9
                }),
                Constraint::Min(0),
            ])
            .split(area);

        let Some(input_area) = chunks.get(0) else { return; };
        let Some(results_area) = chunks.get(1) else { return; };

        let config_block = Block::default()
            .borders(Borders::ALL)
            .title(" Configuration ")
            .border_style(Style::default().fg(
                if self.focus_area != StorageFocusArea::Results {
                    tc!(border_focused)
                } else {
                    tc!(border)
                },
            ));
        f.render_widget(config_block, input_area);

        let input_area = config_block.inner(input_area);

        let status_color = if self.connected {
            tc!(success)
        } else {
            tc!(error)
        };
        let status_text = if self.connected {
            "Connected"
        } else {
            "Disconnected"
        };

        if self.current_mode == StorageMode::Connect {
            let config_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                ])
                .split(input_area);

            let status_line = Line::from(Span::styled(
                format!("Status: {}", status_text),
                Style::default().fg(status_color),
            ));
            if let Some(chunk) = config_chunks.first() {
                f.render_widget(ratatui::widgets::Paragraph::new(status_line), *chunk);
            }

            for (i, field) in self.config_inputs.fields.iter().enumerate() {
                if let Some(chunk) = config_chunks.get(i + 1) {
                    field.render(f, *chunk, insert_mode);
                }
            }
        } else {
            let query_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                ])
                .split(input_area);

            let mut sel = self.mode_selector.clone();
            sel.focused = self.focus_area == StorageFocusArea::Mode;
            if let Some(chunk) = query_chunks.get(0) {
                sel.render(f, *chunk);
            }

            for (i, field) in self.query_inputs.fields.iter().enumerate() {
                if let Some(chunk) = query_chunks.get(i + 1) {
                    field.render(f, *chunk, insert_mode);
                }
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
                .render(f, results_area, None);
        } else {
            let placeholder = empty_state_paragraph(
                "Database Storage",
                "Configure database connection and press Enter",
            );
            f.render_widget(placeholder, results_area);
        }
    }
}

impl TabInput for StorageTab {
    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            StorageFocusArea::Config => {
                self.config_inputs.blur();
                if self.current_mode == StorageMode::Connect {
                    StorageFocusArea::Results
                } else {
                    self.mode_selector.focus();
                    StorageFocusArea::Mode
                }
            }
            StorageFocusArea::Mode => {
                self.mode_selector.blur();
                self.query_inputs.focus(0);
                StorageFocusArea::Query
            }
            StorageFocusArea::Query => {
                self.query_inputs.blur();
                StorageFocusArea::Results
            }
            StorageFocusArea::Results => {
                if self.current_mode == StorageMode::Connect {
                    self.config_inputs.focus(0);
                    StorageFocusArea::Config
                } else {
                    self.mode_selector.focus();
                    StorageFocusArea::Mode
                }
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            StorageFocusArea::Config => {
                self.config_inputs.blur();
                StorageFocusArea::Results
            }
            StorageFocusArea::Mode => {
                if self.current_mode == StorageMode::Connect {
                    self.config_inputs.focus(0);
                    StorageFocusArea::Config
                } else {
                    self.mode_selector.blur();
                    StorageFocusArea::Results
                }
            }
            StorageFocusArea::Query => {
                self.query_inputs.blur();
                if self.current_mode == StorageMode::Connect {
                    self.config_inputs.focus(0);
                    StorageFocusArea::Config
                } else {
                    self.mode_selector.focus();
                    StorageFocusArea::Mode
                }
            }
            StorageFocusArea::Results => {
                if self.current_mode == StorageMode::Connect {
                    self.config_inputs.focus(0);
                    StorageFocusArea::Config
                } else {
                    self.mode_selector.focus();
                    StorageFocusArea::Mode
                }
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() {
            if self.focus_area == StorageFocusArea::Config
                && self.current_mode == StorageMode::Connect
            {
                self.config_inputs.insert(c);
            } else if self.focus_area == StorageFocusArea::Query {
                self.query_inputs.insert(c);
            }
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() {
            if self.focus_area == StorageFocusArea::Config
                && self.current_mode == StorageMode::Connect
            {
                self.config_inputs.backspace();
            } else if self.focus_area == StorageFocusArea::Query {
                self.query_inputs.backspace();
            }
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() {
            if self.focus_area == StorageFocusArea::Config
                && self.current_mode == StorageMode::Connect
            {
                self.config_inputs.paste(text);
            } else if self.focus_area == StorageFocusArea::Query {
                self.query_inputs.paste(text);
            }
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if !self.is_running() {
            if self.focus_area == StorageFocusArea::Config
                && self.current_mode == StorageMode::Connect
            {
                return self.config_inputs.get_focused_value();
            } else if self.focus_area == StorageFocusArea::Query {
                return self.query_inputs.get_focused_value();
            } else if self.focus_area == StorageFocusArea::Results {
                return Some(self.results_view.get_content());
            }
        }
        None
    }

    fn handle_word_forward(&mut self) {
        if !self.is_running() {
            if self.focus_area == StorageFocusArea::Config {
                self.config_inputs.move_word_forward();
            } else if self.focus_area == StorageFocusArea::Query {
                self.query_inputs.move_word_forward();
            }
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() {
            if self.focus_area == StorageFocusArea::Config {
                self.config_inputs.move_word_backward();
            } else if self.focus_area == StorageFocusArea::Query {
                self.query_inputs.move_word_backward();
            }
        }
    }

    fn handle_home(&mut self) {
        if !self.is_running() {
            if self.focus_area == StorageFocusArea::Config {
                self.config_inputs.move_home();
            } else if self.focus_area == StorageFocusArea::Query {
                self.query_inputs.move_home();
            } else if self.focus_area == StorageFocusArea::Results {
                self.results_view.scroll_to_top();
            }
        }
    }

    fn handle_end(&mut self) {
        if !self.is_running() {
            if self.focus_area == StorageFocusArea::Config {
                self.config_inputs.move_end();
            } else if self.focus_area == StorageFocusArea::Query {
                self.query_inputs.move_end();
            } else if self.focus_area == StorageFocusArea::Results {
                self.results_view.scroll_to_bottom();
            }
        }
    }

    fn handle_top(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            StorageFocusArea::Config => self.config_inputs.blur(),
            StorageFocusArea::Query => self.query_inputs.blur(),
            StorageFocusArea::Mode => self.mode_selector.blur(),
            StorageFocusArea::Results => {}
        }
        if self.current_mode == StorageMode::Connect {
            self.focus_area = StorageFocusArea::Config;
            self.config_inputs.focus(0);
        } else {
            self.focus_area = StorageFocusArea::Mode;
            self.mode_selector.focus();
        }
    }

    fn handle_bottom(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            StorageFocusArea::Config => self.config_inputs.blur(),
            StorageFocusArea::Query => self.query_inputs.blur(),
            StorageFocusArea::Mode => self.mode_selector.blur(),
            StorageFocusArea::Results => {}
        }
        self.focus_area = StorageFocusArea::Results;
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }

        match self.focus_area {
            StorageFocusArea::Config => {
                self.config_inputs.blur();
            }
            StorageFocusArea::Mode => {
                let was_open = self.mode_selector.is_open();
                self.mode_selector.handle_enter();
                if !was_open {
                    return;
                }
                self.current_mode = match self.mode_selector.selected {
                    0 => StorageMode::Connect,
                    1 => StorageMode::ListScans,
                    2 => StorageMode::ListFindings,
                    _ => StorageMode::SearchByCve,
                };
            }
            StorageFocusArea::Query => {
                self.query_inputs.blur();
            }
            StorageFocusArea::Results => {
                return;
            }
        }

        self.start();
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        self.config_inputs.blur();
        self.mode_selector.blur();
        self.query_inputs.blur();
    }

    fn handle_up(&mut self) {
        if !self.is_running() {
            match self.focus_area {
                StorageFocusArea::Config => self.config_inputs.focus_prev(),
                StorageFocusArea::Mode => self.mode_selector.handle_up(),
                StorageFocusArea::Query => self.query_inputs.focus_prev(),
                StorageFocusArea::Results => self.results_view.scroll_up(1),
            }
        }
    }

    fn handle_down(&mut self) {
        if !self.is_running() {
            match self.focus_area {
                StorageFocusArea::Config => self.config_inputs.focus_next(),
                StorageFocusArea::Mode => self.mode_selector.handle_down(),
                StorageFocusArea::Query => self.query_inputs.focus_next(),
                StorageFocusArea::Results => self.results_view.scroll_down(1),
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        if !self.is_running() {
            match self.focus_area {
                StorageFocusArea::Config => self.config_inputs.move_left(),
                StorageFocusArea::Query => self.query_inputs.move_left(),
                _ => false,
            }
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if !self.is_running() {
            match self.focus_area {
                StorageFocusArea::Config => self.config_inputs.move_right(),
                StorageFocusArea::Query => self.query_inputs.move_right(),
                _ => false,
            }
        } else {
            false
        }
    }

    fn is_at_left_edge(&self) -> bool {
        match self.focus_area {
            StorageFocusArea::Config => self.config_inputs.is_at_left_edge(),
            StorageFocusArea::Mode => {
                self.mode_selector.items.is_empty() || self.mode_selector.selected == 0
            }
            StorageFocusArea::Query => self.query_inputs.is_at_left_edge(),
            _ => true,
        }
    }

    fn is_at_right_edge(&self) -> bool {
        match self.focus_area {
            StorageFocusArea::Config => self.config_inputs.is_at_right_edge(),
            StorageFocusArea::Mode => {
                self.mode_selector.items.is_empty()
                    || self.mode_selector.selected
                        >= self.mode_selector.items.len().saturating_sub(1)
            }
            StorageFocusArea::Query => self.query_inputs.is_at_right_edge(),
            _ => true,
        }
    }

    fn is_input_focused(&self) -> bool {
        (self.focus_area == StorageFocusArea::Config && self.config_inputs.is_focused())
            || (self.focus_area == StorageFocusArea::Query && self.query_inputs.is_focused())
    }

    fn page_up(&mut self, page_size: usize) {
        if self.is_running() {
            return;
        }
        self.results_view.page_up(page_size);
    }

    fn page_down(&mut self, page_size: usize) {
        if self.is_running() {
            return;
        }
        self.results_view.page_down(page_size);
    }
}
