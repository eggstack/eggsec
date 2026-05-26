use crate::tc;
use crate::tui::app::tab_error::TabError;
use crate::tui::components::{InputField, InputGroup, ScrollableText, Selector, SelectorItem};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

pub struct PluginTab {
    pub inputs: InputGroup,
    pub plugin_selector: Selector,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub focus_area: PluginFocusArea,
    pub plugins_loaded: bool,
    pub plugin_list: Vec<PluginInfo>,
    pub error: Option<TabError>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PluginFocusArea {
    Inputs,
    PluginSelector,
    Results,
}

#[derive(Clone, Debug)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub tags: Vec<String>,
    pub language: String,
}

impl From<slapper_plugin::PluginInfo> for PluginInfo {
    fn from(info: slapper_plugin::PluginInfo) -> Self {
        Self {
            name: info.name,
            version: info.version,
            description: info.description,
            author: info.author,
            tags: info.tags,
            language: format!("{:?}", info.language),
        }
    }
}

impl PluginTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new().add(InputField::new("Target URL or Host"));

        let plugin_selector = Selector::new("Plugin").items(vec![]);

        Self {
            inputs,
            plugin_selector,
            state: AppState::Idle,
            results_view: ScrollableText::new("Plugin Results"),
            focus_area: PluginFocusArea::Inputs,
            plugins_loaded: false,
            plugin_list: Vec::new(),
            error: None,
        }
    }

    pub fn target(&self) -> &str {
        self.inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn plugin_name(&self) -> Option<&str> {
        self.plugin_selector.selected_value()
    }

    pub fn load_plugins(&mut self, plugins: Vec<PluginInfo>) {
        self.plugin_list = plugins.clone();
        let items: Vec<SelectorItem> = plugins
            .iter()
            .map(|p| SelectorItem::new(&p.name, &p.name))
            .collect();
        self.plugin_selector = Selector::new("Plugin").items(items);
        self.plugins_loaded = true;
    }

    pub fn set_results(&mut self, results: PluginResults) {
        self.state = AppState::Completed;
        self.results_view.clear();

        self.results_view.add_line(Line::from(Span::styled(
            format!("Plugin: {}", results.plugin_name),
            Style::default().fg(tc!(success)),
        )));
        self.results_view.add_line(Line::from(Span::styled(
            format!("Target: {}", results.target),
            Style::default().fg(tc!(accent)),
        )));
        self.results_view.add_line(Line::from(Span::styled(
            format!("Execution Time: {}ms", results.execution_time_ms),
            Style::default().fg(tc!(info)),
        )));
        self.results_view.add_line(Line::from(""));

        if results.success {
            self.results_view.add_line(Line::from(Span::styled(
                "Status: Success",
                Style::default().fg(tc!(success)),
            )));
        } else {
            self.results_view.add_line(Line::from(Span::styled(
                "Status: Failed",
                Style::default().fg(tc!(error)),
            )));
        }

        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(Span::styled(
            format!("Findings ({}):", results.findings.len()),
            Style::default().fg(tc!(accent)),
        )));

        for finding in &results.findings {
            let severity_color = match finding.severity.as_str() {
                "critical" => tc!(error),
                "high" => tc!(error),
                "medium" => tc!(warning),
                "low" => tc!(secondary),
                _ => tc!(text_dim),
            };
            self.results_view.add_line(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    finding.severity.clone(),
                    Style::default().fg(severity_color),
                ),
                Span::raw(": "),
                Span::raw(finding.title.clone()),
            ]));
            if let Some(ref evidence) = finding.evidence {
                self.results_view.add_line(Line::from(vec![
                    Span::raw("    Evidence: "),
                    Span::raw(evidence.clone()),
                ]));
            }
        }

        if !results.errors.is_empty() {
            self.results_view.add_line(Line::from(""));
            self.results_view.add_line(Line::from(Span::styled(
                "Errors:",
                Style::default().fg(tc!(error)),
            )));
            for err in &results.errors {
                self.results_view
                    .add_line(Line::from(format!("  - {}", err)));
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct PluginResults {
    pub plugin_name: String,
    pub target: String,
    pub success: bool,
    pub findings: Vec<Finding>,
    pub errors: Vec<String>,
    pub execution_time_ms: u64,
}

#[derive(Clone, Debug)]
pub struct Finding {
    pub title: String,
    pub severity: String,
    pub description: String,
    pub evidence: Option<String>,
}

impl TabState for PluginTab {
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
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
    }
}

impl TabRender for PluginTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        if let Some(ref error) = self.error {
            use ratatui::widgets::Paragraph;
            let error_text = Paragraph::new(format!("Error: {}", error.message()))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Plugin - Error"),
                )
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Min(5),
            ])
            .split(area);

        // Target input
        let input_block = Block::default()
            .title(" Target ")
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if self.focus_area == PluginFocusArea::Inputs {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3)])
            .split(input_block.inner(chunks[0]));

        f.render_widget(input_block, chunks[0]);
        if let Some(field) = self.inputs.fields.first() {
            if let Some(chunk) = input_chunks.first() {
                field.render(f, *chunk, insert_mode);
            }
        }

        // Plugin selector
        let mut selector = self.plugin_selector.clone();
        selector.focused = self.focus_area == PluginFocusArea::PluginSelector;
        selector.render(f, chunks[1]);

        // Results
        self.results_view.render(f, chunks[2], None);
    }
}

impl TabInput for PluginTab {
    fn handle_focus_next(&mut self) {
        self.focus_area = match self.focus_area {
            PluginFocusArea::Inputs => {
                self.inputs.blur();
                PluginFocusArea::PluginSelector
            }
            PluginFocusArea::PluginSelector => {
                self.plugin_selector.blur();
                PluginFocusArea::Results
            }
            PluginFocusArea::Results => {
                self.inputs.focus(0);
                PluginFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = match self.focus_area {
            PluginFocusArea::Inputs => {
                self.inputs.blur();
                PluginFocusArea::Results
            }
            PluginFocusArea::PluginSelector => {
                self.inputs.focus(0);
                PluginFocusArea::Inputs
            }
            PluginFocusArea::Results => {
                self.plugin_selector.focus();
                PluginFocusArea::PluginSelector
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == PluginFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == PluginFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == PluginFocusArea::Inputs {
            self.inputs.paste(text);
        }
    }

    fn handle_word_forward(&mut self) {
        if self.focus_area == PluginFocusArea::Inputs {
            self.inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if self.focus_area == PluginFocusArea::Inputs {
            self.inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if self.focus_area == PluginFocusArea::Inputs {
            self.inputs.move_home();
        } else if self.focus_area == PluginFocusArea::Results {
            self.results_view.scroll_to_top();
        }
    }

    fn handle_end(&mut self) {
        if self.focus_area == PluginFocusArea::Inputs {
            self.inputs.move_end();
        } else if self.focus_area == PluginFocusArea::Results {
            self.results_view.scroll_to_bottom();
        }
    }

    fn handle_top(&mut self) {
        self.focus_area = PluginFocusArea::Inputs;
        self.inputs.focus(0);
    }

    fn handle_bottom(&mut self) {
        self.focus_area = PluginFocusArea::Results;
    }

    fn handle_enter(&mut self) {
        if !self.is_running() {
            return;
        }
        match self.focus_area {
            PluginFocusArea::Inputs => {
                self.inputs.blur();
            }
            PluginFocusArea::PluginSelector => {
                self.plugin_selector.handle_enter();
            }
            PluginFocusArea::Results => {}
        }
    }

    fn handle_escape(&mut self) {
        self.inputs.blur();
        self.plugin_selector.blur();
    }

    fn handle_up(&mut self) {
        match self.focus_area {
            PluginFocusArea::Inputs => {
                self.inputs.focus_prev();
            }
            PluginFocusArea::PluginSelector => {
                self.plugin_selector.handle_up();
            }
            PluginFocusArea::Results => {
                self.results_view.scroll_up(1);
            }
        }
    }

    fn handle_down(&mut self) {
        match self.focus_area {
            PluginFocusArea::Inputs => {
                self.inputs.focus_next();
            }
            PluginFocusArea::PluginSelector => {
                self.plugin_selector.handle_down();
            }
            PluginFocusArea::Results => {
                self.results_view.scroll_down(1);
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        match self.focus_area {
            PluginFocusArea::Inputs => self.inputs.move_left(),
            _ => false,
        }
    }

    fn handle_right(&mut self) -> bool {
        match self.focus_area {
            PluginFocusArea::Inputs => self.inputs.move_right(),
            _ => false,
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == PluginFocusArea::Inputs && self.inputs.is_focused()
    }

    fn is_at_left_edge(&self) -> bool {
        match self.focus_area {
            PluginFocusArea::Inputs => self.inputs.is_at_left_edge(),
            PluginFocusArea::PluginSelector => {
                self.plugin_selector.items.is_empty()
                    || self.plugin_selector.selected == 0
            }
            _ => true,
        }
    }

    fn is_at_right_edge(&self) -> bool {
        match self.focus_area {
            PluginFocusArea::Inputs => self.inputs.is_at_right_edge(),
            PluginFocusArea::PluginSelector => {
                self.plugin_selector.items.is_empty()
                    || self.plugin_selector.selected >= self.plugin_selector.items.len().saturating_sub(1)
            }
            _ => true,
        }
    }
}

impl PluginTab {
    pub fn stop(&mut self) {
        if self.state == AppState::Running {
            self.state = AppState::Idle;
        }
    }

    pub fn handle_word_forward(&mut self) {
        for _ in 0..5 {
            self.handle_right();
        }
    }

    pub fn handle_word_backward(&mut self) {
        for _ in 0..5 {
            self.handle_left();
        }
    }

    pub fn handle_home(&mut self) {
        for _ in 0..100 {
            self.handle_left();
        }
    }

    pub fn handle_end(&mut self) {
        for _ in 0..100 {
            self.handle_right();
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.results_view.page_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.results_view.page_down(page_size);
    }

    pub fn handle_top(&mut self) {
        self.results_view.scroll_to_top();
    }

    pub fn handle_bottom(&mut self) {
        self.results_view.scroll_to_bottom();
    }
}
