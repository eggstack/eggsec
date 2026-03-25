use crate::tui::components::{InputField, InputGroup, ScrollableText, Selector, SelectorItem};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct NseTab {
    pub inputs: InputGroup,
    pub script_selector: Selector,
    pub progress: f64,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub focus_area: NseFocusArea,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NseFocusArea {
    Inputs,
    ScriptSelector,
    Results,
}

impl NseTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("Target Host / URL"))
            .add(InputField::new("Script Arguments (key=value,comma-sep)"))
            .add(InputField::new("Custom Script Path (optional)"));

        let script_selector = Selector::new("NSE Script").items(vec![
            SelectorItem::new("Default Scripts", "default"),
            SelectorItem::new("Discovery", "discovery"),
            SelectorItem::new("Banner Grab", "banner"),
            SelectorItem::new("HTTP Headers", "http-headers"),
            SelectorItem::new("DNS Check", "dns-check"),
            SelectorItem::new("SSL Certificate", "ssl-cert"),
            SelectorItem::new("Custom Script", "custom"),
        ]);

        Self {
            inputs,
            script_selector,
            progress: 0.0,
            state: AppState::Idle,
            results_view: ScrollableText::new("NSE Results"),
            focus_area: NseFocusArea::Inputs,
        }
    }

    pub fn target(&self) -> &str {
        self.inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn script_args(&self) -> Option<&str> {
        self.inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .filter(|v| !v.is_empty())
    }

    pub fn custom_script(&self) -> Option<&str> {
        self.inputs
            .fields
            .get(2)
            .map(|f| f.value.as_str())
            .filter(|v| !v.is_empty())
    }

    pub fn script(&self) -> &str {
        self.script_selector.selected_value().unwrap_or("default")
    }

    pub fn set_results(&mut self, results: NseResults) {
        self.state = AppState::Completed;
        self.results_view.clear();

        self.results_view.add_line(Line::from(Span::styled(
            format!("NSE Script Results: {}", results.script),
            Style::default().fg(Color::Green),
        )));
        self.results_view.add_line(Line::from(Span::styled(
            format!("Target: {}", results.target),
            Style::default().fg(Color::Yellow),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(Span::styled(
            "Output:",
            Style::default().fg(Color::Cyan),
        )));
        self.results_view.add_line(Line::from(""));

        for line in results.output.lines() {
            self.results_view.add_line(Line::from(line.to_string()));
        }

        if !results.errors.is_empty() {
            self.results_view.add_line(Line::from(""));
            self.results_view.add_line(Line::from(Span::styled(
                "Errors:",
                Style::default().fg(Color::Red),
            )));
            for err in results.errors.lines() {
                self.results_view.add_line(Line::from(err.to_string()));
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct NseResults {
    pub target: String,
    pub script: String,
    pub output: String,
    pub errors: String,
    pub success: bool,
}

impl TabState for NseTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        self.progress
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.results_view.clear();
        self.progress = 0.0;
    }

    fn set_error(&mut self, msg: String) {
        self.state = AppState::Error(msg.clone());
        self.results_view.add_line(Line::from(Span::styled(
            format!("Error: {}", msg),
            Style::default().fg(Color::Red),
        )));
    }
}

impl TabRender for NseTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(12),
                Constraint::Length(4),
                Constraint::Min(5),
            ])
            .split(area);

        // Input fields
        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(chunks[0]);

        let input_block = Block::default()
            .title(" NSE Configuration ")
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if self.focus_area == NseFocusArea::Inputs {
                    Color::Yellow
                } else {
                    Color::Gray
                }),
            );
        f.render_widget(input_block, chunks[0]);

        for (i, field) in self.inputs.fields.iter().enumerate() {
            if i < input_chunks.len() {
                field.render(f, input_chunks[i], insert_mode);
            }
        }

        // Script selector
        let mut selector = self.script_selector.clone();
        selector.focused = self.focus_area == NseFocusArea::ScriptSelector;
        selector.render(f, chunks[1]);

        // Results
        self.results_view.render(f, chunks[2]);
    }
}

impl TabInput for NseTab {
    fn handle_focus_next(&mut self) {
        self.focus_area = match self.focus_area {
            NseFocusArea::Inputs => {
                self.inputs.blur();
                NseFocusArea::ScriptSelector
            }
            NseFocusArea::ScriptSelector => {
                self.script_selector.blur();
                NseFocusArea::Results
            }
            NseFocusArea::Results => {
                self.inputs.focus(0);
                NseFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = match self.focus_area {
            NseFocusArea::Inputs => {
                self.inputs.blur();
                NseFocusArea::Results
            }
            NseFocusArea::ScriptSelector => {
                self.inputs.focus(0);
                NseFocusArea::Inputs
            }
            NseFocusArea::Results => {
                self.script_selector.focus();
                NseFocusArea::ScriptSelector
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if self.focus_area == NseFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if self.focus_area == NseFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_enter(&mut self) {
        match self.focus_area {
            NseFocusArea::Inputs => {
                self.inputs.blur();
            }
            NseFocusArea::ScriptSelector => {
                self.script_selector.handle_enter();
            }
            NseFocusArea::Results => {}
        }
    }

    fn handle_escape(&mut self) {
        self.inputs.blur();
        self.script_selector.blur();
    }

    fn handle_up(&mut self) {
        match self.focus_area {
            NseFocusArea::Inputs => {
                self.inputs.focus_prev();
            }
            NseFocusArea::ScriptSelector => {
                self.script_selector.handle_up();
            }
            NseFocusArea::Results => {
                self.results_view.scroll_up(1);
            }
        }
    }

    fn handle_down(&mut self) {
        match self.focus_area {
            NseFocusArea::Inputs => {
                self.inputs.focus_next();
            }
            NseFocusArea::ScriptSelector => {
                self.script_selector.handle_down();
            }
            NseFocusArea::Results => {
                self.results_view.scroll_down(1);
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        match self.focus_area {
            NseFocusArea::Inputs => self.inputs.move_left(),
            _ => false,
        }
    }

    fn handle_right(&mut self) -> bool {
        match self.focus_area {
            NseFocusArea::Inputs => self.inputs.move_right(),
            _ => false,
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == NseFocusArea::Inputs && self.inputs.is_focused()
    }

    fn is_at_left_edge(&self) -> bool {
        match self.focus_area {
            NseFocusArea::Inputs => !self.inputs.can_move_left(),
            NseFocusArea::ScriptSelector => self.script_selector.selected == 0,
            _ => true,
        }
    }

    fn is_at_right_edge(&self) -> bool {
        match self.focus_area {
            NseFocusArea::Inputs => !self.inputs.can_move_right(),
            NseFocusArea::ScriptSelector => {
                self.script_selector.selected >= self.script_selector.items.len().saturating_sub(1)
            }
            _ => true,
        }
    }
}

impl NseTab {
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

    pub fn handle_top(&mut self) {
        for _ in 0..100 {
            self.results_view.scroll_up(1);
        }
    }

    pub fn handle_bottom(&mut self) {
        for _ in 0..100 {
            self.results_view.scroll_down(1);
        }
    }
}
