use crate::tc;
use crate::tui::app::tab_error::TabError;
use crate::tui::components::{empty_state_paragraph, InputField, InputGroup, ScrollableText, Selector, SelectorItem};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

#[derive(Clone, Copy, PartialEq)]
pub enum ReportView {
    Convert,
    Trend,
    Schedule,
}

pub struct ReportTab {
    pub view_selector: Selector,
    pub convert_inputs: InputGroup,
    pub trend_inputs: InputGroup,
    pub schedule_inputs: InputGroup,
    pub format_selector: Selector,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub current_view: ReportView,
    pub focus_area: ReportFocusArea,
    pub error: Option<TabError>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReportFocusArea {
    ViewSelector,
    Inputs,
    Results,
}

impl Default for ReportTab {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportTab {
    pub fn new() -> Self {
        let view_selector =
            Selector::new("Mode").simple_items(vec!["Convert", "Trend Analysis", "Schedule"]);

        let convert_inputs = InputGroup::new()
            .add(InputField::new("Input File (JSON)"))
            .add(InputField::new("Output File (optional)"));

        let trend_inputs = InputGroup::new()
            .add(InputField::new("Before Scan File"))
            .add(InputField::new("After Scan File"))
            .add(InputField::new("Output File (optional)"));

        let schedule_inputs = InputGroup::new()
            .add(InputField::new("Cron Expression"))
            .add(InputField::new("Target URL"))
            .add(InputField::new("Scan Type").with_value("scan"))
            .add(InputField::new("Output File (optional)"));

        let format_selector = Selector::new("Output Format").items(vec![
            SelectorItem::new("JSON", "json"),
            SelectorItem::new("CSV", "csv"),
            SelectorItem::new("HTML", "html"),
            SelectorItem::new("Markdown", "markdown"),
            SelectorItem::new("SARIF", "sarif"),
            SelectorItem::new("JUnit", "junit"),
        ]);

        Self {
            view_selector,
            convert_inputs,
            trend_inputs,
            schedule_inputs,
            format_selector,
            state: AppState::Idle,
            results_view: ScrollableText::new("Report Results"),
            current_view: ReportView::Convert,
            focus_area: ReportFocusArea::ViewSelector,
            error: None,
        }
    }

    pub fn input_file(&self) -> &str {
        self.convert_inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn output_file(&self) -> Option<&str> {
        self.convert_inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .filter(|v| !v.is_empty())
    }

    pub fn before_file(&self) -> &str {
        self.trend_inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn after_file(&self) -> &str {
        self.trend_inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn schedule_cron(&self) -> &str {
        self.schedule_inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn schedule_target(&self) -> &str {
        self.schedule_inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn format(&self) -> &str {
        self.format_selector.selected_value().unwrap_or("html")
    }

    pub fn set_convert_results(&mut self, success: bool, message: String) {
        self.state = AppState::Completed;
        self.results_view.clear();

        let (title, color) = if success {
            ("Conversion Complete", tc!(success))
        } else {
            ("Conversion Failed", tc!(error))
        };

        self.results_view
            .add_line(Line::from(Span::styled(title, Style::default().fg(color))));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(message));
    }

    pub fn set_trend_results(&mut self, before_file: &str, after_file: &str, summary: String) {
        self.state = AppState::Completed;
        self.results_view.clear();

        self.results_view.add_line(Line::from(Span::styled(
            "Trend Analysis Complete",
            Style::default().fg(tc!(success)),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view
            .add_line(Line::from(format!("Before: {}", before_file)));
        self.results_view
            .add_line(Line::from(format!("After: {}", after_file)));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(Span::styled(
            "Summary:",
            Style::default().fg(tc!(warning)),
        )));
        self.results_view.add_line(Line::from(summary));
    }

    pub fn set_schedule_added(&mut self, cron: &str, target: &str) {
        self.state = AppState::Completed;
        self.results_view.clear();

        self.results_view.add_line(Line::from(Span::styled(
            "Schedule Added",
            Style::default().fg(tc!(success)),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view
            .add_line(Line::from(format!("Schedule: {}", cron)));
        self.results_view
            .add_line(Line::from(format!("Target: {}", target)));
    }

    pub fn list_schedules(&mut self, schedules: Vec<String>) {
        self.state = AppState::Completed;
        self.results_view.clear();

        self.results_view.add_line(Line::from(Span::styled(
            "Scheduled Scans",
            Style::default().fg(tc!(warning)),
        )));
        self.results_view.add_line(Line::from(""));

        if schedules.is_empty() {
            self.results_view
                .add_line(Line::from("No scheduled scans configured."));
        } else {
            for (i, schedule) in schedules.iter().enumerate() {
                self.results_view
                    .add_line(Line::from(format!("  {}. {}", i + 1, schedule)));
            }
        }
    }
}

impl TabState for ReportTab {
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
        self.view_selector.select(0);
        self.format_selector.select(0);
        self.current_view = ReportView::Convert;
        for field in &mut self.convert_inputs.fields {
            field.clear();
        }
        for field in &mut self.trend_inputs.fields {
            field.clear();
        }
        for field in &mut self.schedule_inputs.fields {
            field.clear();
        }
        self.focus_area = ReportFocusArea::ViewSelector;
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
    }
}

impl TabRender for ReportTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        if let Some(ref err) = self.error {
            use ratatui::widgets::Paragraph;
            let error_text = Paragraph::new(format!("Error: {}", err.message()))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Report - Error"),
                )
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(14),
                Constraint::Min(5),
            ])
            .split(area);

        // View selector
        let Some(view_area) = chunks.get(0) else {
            return;
        };
        let mut selector = self.view_selector.clone();
        selector.focused = self.focus_area == ReportFocusArea::ViewSelector;
        selector.render(f, *view_area);

        // Inputs based on current view
        let inputs_block = Block::default()
            .title(match self.current_view {
                ReportView::Convert => " Convert Report ",
                ReportView::Trend => " Trend Analysis ",
                ReportView::Schedule => " Schedule Scan ",
            })
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if self.focus_area == ReportFocusArea::Inputs {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );

        let current_inputs = match self.current_view {
            ReportView::Convert => &self.convert_inputs,
            ReportView::Trend => &self.trend_inputs,
            ReportView::Schedule => &self.schedule_inputs,
        };

        let Some(inputs_area) = chunks.get(1) else {
            return;
        };
        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(inputs_block.inner(*inputs_area));

        f.render_widget(inputs_block, *inputs_area);

        for (i, field) in current_inputs.fields.iter().enumerate() {
            if let Some(chunk) = input_chunks.get(i) {
                field.render(f, *chunk, insert_mode);
            }
        }

        // Format selector for Convert view
        if self.current_view == ReportView::Convert {
            let format_area = Rect {
                x: inputs_area.x + inputs_area.width - 25,
                y: inputs_area.y + 1,
                width: 23,
                height: 3,
            };
            let mut fmt_sel = self.format_selector.clone();
            fmt_sel.focused = self.focus_area == ReportFocusArea::Inputs;
            fmt_sel.render(f, format_area);
        }

        // Results
        if let Some(results_area) = chunks.get(2) {
            if self.results_view.is_empty() {
                let placeholder =
                    empty_state_paragraph("Results", "Results will appear here after running");
                f.render_widget(placeholder, *results_area);
            } else {
                self.results_view.render(f, *results_area, None);
            }
        }
    }
}

impl TabInput for ReportTab {
    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            ReportFocusArea::ViewSelector => {
                self.view_selector.blur();
                ReportFocusArea::Inputs
            }
            ReportFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ReportView::Convert => &mut self.convert_inputs,
                    ReportView::Trend => &mut self.trend_inputs,
                    ReportView::Schedule => &mut self.schedule_inputs,
                };
                current_inputs.blur();
                ReportFocusArea::Results
            }
            ReportFocusArea::Results => {
                self.view_selector.focus();
                ReportFocusArea::ViewSelector
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            ReportFocusArea::ViewSelector => {
                self.view_selector.blur();
                ReportFocusArea::Results
            }
            ReportFocusArea::Inputs => {
                self.view_selector.focus();
                ReportFocusArea::ViewSelector
            }
            ReportFocusArea::Results => {
                let current_inputs = match self.current_view {
                    ReportView::Convert => &mut self.convert_inputs,
                    ReportView::Trend => &mut self.trend_inputs,
                    ReportView::Schedule => &mut self.schedule_inputs,
                };
                current_inputs.focus(0);
                ReportFocusArea::Inputs
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == ReportFocusArea::Inputs {
            let current_inputs = match self.current_view {
                ReportView::Convert => &mut self.convert_inputs,
                ReportView::Trend => &mut self.trend_inputs,
                ReportView::Schedule => &mut self.schedule_inputs,
            };
            current_inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == ReportFocusArea::Inputs {
            let current_inputs = match self.current_view {
                ReportView::Convert => &mut self.convert_inputs,
                ReportView::Trend => &mut self.trend_inputs,
                ReportView::Schedule => &mut self.schedule_inputs,
            };
            current_inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == ReportFocusArea::Inputs {
            let current_inputs = match self.current_view {
                ReportView::Convert => &mut self.convert_inputs,
                ReportView::Trend => &mut self.trend_inputs,
                ReportView::Schedule => &mut self.schedule_inputs,
            };
            current_inputs.paste(text);
        }
    }

    fn handle_word_forward(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ReportFocusArea::Inputs {
            let current_inputs = match self.current_view {
                ReportView::Convert => &mut self.convert_inputs,
                ReportView::Trend => &mut self.trend_inputs,
                ReportView::Schedule => &mut self.schedule_inputs,
            };
            current_inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ReportFocusArea::Inputs {
            let current_inputs = match self.current_view {
                ReportView::Convert => &mut self.convert_inputs,
                ReportView::Trend => &mut self.trend_inputs,
                ReportView::Schedule => &mut self.schedule_inputs,
            };
            current_inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ReportFocusArea::Inputs {
            let current_inputs = match self.current_view {
                ReportView::Convert => &mut self.convert_inputs,
                ReportView::Trend => &mut self.trend_inputs,
                ReportView::Schedule => &mut self.schedule_inputs,
            };
            current_inputs.move_home();
        } else if self.focus_area == ReportFocusArea::Results {
            self.results_view.scroll_to_top();
        }
    }

    fn handle_end(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ReportFocusArea::Inputs {
            let current_inputs = match self.current_view {
                ReportView::Convert => &mut self.convert_inputs,
                ReportView::Trend => &mut self.trend_inputs,
                ReportView::Schedule => &mut self.schedule_inputs,
            };
            current_inputs.move_end();
        } else if self.focus_area == ReportFocusArea::Results {
            self.results_view.scroll_to_bottom();
        }
    }

    fn handle_top(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = ReportFocusArea::ViewSelector;
        self.view_selector.focus();
    }

    fn handle_bottom(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = ReportFocusArea::Results;
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }

        match self.focus_area {
            ReportFocusArea::ViewSelector => {
                if self.view_selector.is_open() {
                    if self.view_selector.confirm().is_none() {
                        tracing::warn!("Failed to confirm view selector selection");
                    }
                    self.current_view = match self.view_selector.selected {
                        0 => ReportView::Convert,
                        1 => ReportView::Trend,
                        2 => ReportView::Schedule,
                        _ => ReportView::Convert,
                    };
                } else {
                    self.view_selector.open();
                    return;
                }
            }
            ReportFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ReportView::Convert => &mut self.convert_inputs,
                    ReportView::Trend => &mut self.trend_inputs,
                    ReportView::Schedule => &mut self.schedule_inputs,
                };
                current_inputs.blur();
            }
            ReportFocusArea::Results => {
                return;
            }
        }

        self.start();
    }

    fn handle_escape(&mut self) {
        if self.view_selector.is_open() {
            self.view_selector.cancel();
            return;
        }
        self.view_selector.blur();
        let current_inputs = match self.current_view {
            ReportView::Convert => &mut self.convert_inputs,
            ReportView::Trend => &mut self.trend_inputs,
            ReportView::Schedule => &mut self.schedule_inputs,
        };
        current_inputs.blur();
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            ReportFocusArea::ViewSelector => {
                if self.view_selector.is_open() {
                    self.view_selector.move_prev();
                }
            }
            ReportFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ReportView::Convert => &mut self.convert_inputs,
                    ReportView::Trend => &mut self.trend_inputs,
                    ReportView::Schedule => &mut self.schedule_inputs,
                };
                current_inputs.focus_prev();
            }
            ReportFocusArea::Results => {
                self.results_view.scroll_up(1);
            }
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            ReportFocusArea::ViewSelector => {
                if self.view_selector.is_open() {
                    self.view_selector.move_next();
                }
            }
            ReportFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ReportView::Convert => &mut self.convert_inputs,
                    ReportView::Trend => &mut self.trend_inputs,
                    ReportView::Schedule => &mut self.schedule_inputs,
                };
                current_inputs.focus_next();
            }
            ReportFocusArea::Results => {
                self.results_view.scroll_down(1);
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        match self.focus_area {
            ReportFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ReportView::Convert => &mut self.convert_inputs,
                    ReportView::Trend => &mut self.trend_inputs,
                    ReportView::Schedule => &mut self.schedule_inputs,
                };
                return current_inputs.move_left();
            }
            _ => return false,
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        match self.focus_area {
            ReportFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ReportView::Convert => &mut self.convert_inputs,
                    ReportView::Trend => &mut self.trend_inputs,
                    ReportView::Schedule => &mut self.schedule_inputs,
                };
                return current_inputs.move_right();
            }
            _ => return false,
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == ReportFocusArea::Inputs
    }

    fn is_at_left_edge(&self) -> bool {
        match self.focus_area {
            ReportFocusArea::ViewSelector if self.view_selector.is_open() => {
                self.view_selector.items.is_empty() || self.view_selector.selected == 0
            }
            ReportFocusArea::ViewSelector => true,
            ReportFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ReportView::Convert => &self.convert_inputs,
                    ReportView::Trend => &self.trend_inputs,
                    ReportView::Schedule => &self.schedule_inputs,
                };
                current_inputs.is_at_left_edge()
            }
            _ => true,
        }
    }

    fn is_at_right_edge(&self) -> bool {
        match self.focus_area {
            ReportFocusArea::ViewSelector if self.view_selector.is_open() => {
                self.view_selector.items.is_empty()
                    || self.view_selector.selected
                        >= self.view_selector.items.len().saturating_sub(1)
            }
            ReportFocusArea::ViewSelector => true,
            ReportFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ReportView::Convert => &self.convert_inputs,
                    ReportView::Trend => &self.trend_inputs,
                    ReportView::Schedule => &self.schedule_inputs,
                };
                current_inputs.is_at_right_edge()
            }
            _ => true,
        }
    }
}

impl ReportTab {
    pub fn start(&mut self) {
        self.state = AppState::Running;
        self.results_view.clear();
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.results_view.scroll_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.results_view.scroll_down(page_size);
    }
}
