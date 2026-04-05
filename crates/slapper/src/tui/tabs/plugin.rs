use crate::tui::components::{InputField, InputGroup, ScrollableText, Selector, SelectorItem};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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
    pub language: String,
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
            Style::default().fg(Color::Green),
        )));
        self.results_view.add_line(Line::from(Span::styled(
            format!("Target: {}", results.target),
            Style::default().fg(Color::Yellow),
        )));
        self.results_view.add_line(Line::from(Span::styled(
            format!("Execution Time: {}ms", results.execution_time_ms),
            Style::default().fg(Color::Cyan),
        )));
        self.results_view.add_line(Line::from(""));

        if results.success {
            self.results_view.add_line(Line::from(Span::styled(
                "Status: Success",
                Style::default().fg(Color::Green),
            )));
        } else {
            self.results_view.add_line(Line::from(Span::styled(
                "Status: Failed",
                Style::default().fg(Color::Red),
            )));
        }

        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(Span::styled(
            format!("Findings ({}):", results.findings.len()),
            Style::default().fg(Color::Yellow),
        )));

        for finding in &results.findings {
            let severity_color = match finding.severity.as_str() {
                "critical" => Color::Red,
                "high" => Color::LightRed,
                "medium" => Color::Yellow,
                "low" => Color::Blue,
                _ => Color::Gray,
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
                Style::default().fg(Color::Red),
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
    }

    fn set_error(&mut self, msg: String) {
        self.state = AppState::Error(msg.clone());
        self.results_view.add_line(Line::from(Span::styled(
            format!("Error: {}", msg),
            Style::default().fg(Color::Red),
        )));
    }
}

impl TabRender for PluginTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
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
                    Color::Yellow
                } else {
                    Color::Gray
                }),
            );

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3)])
            .split(input_block.inner(chunks[0]));

        f.render_widget(input_block, chunks[0]);
        if let Some(field) = self.inputs.fields.first() {
            field.render(f, input_chunks[0], insert_mode);
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
        if self.focus_area == PluginFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if self.focus_area == PluginFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_enter(&mut self) {
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
            PluginFocusArea::Inputs => !self.inputs.can_move_left(),
            PluginFocusArea::PluginSelector => self.plugin_selector.selected == 0,
            _ => true,
        }
    }

    fn is_at_right_edge(&self) -> bool {
        match self.focus_area {
            PluginFocusArea::Inputs => !self.inputs.can_move_right(),
            PluginFocusArea::PluginSelector => {
                self.plugin_selector.selected >= self.plugin_selector.items.len().saturating_sub(1)
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
