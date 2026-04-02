use crate::tui::components::{InputField, InputGroup, ScrollableText, Selector, SelectorItem};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReportFocusArea {
    ViewSelector,
    Inputs,
    Results,
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
            ("Conversion Complete", Color::Green)
        } else {
            ("Conversion Failed", Color::Red)
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
            Style::default().fg(Color::Green),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view
            .add_line(Line::from(format!("Before: {}", before_file)));
        self.results_view
            .add_line(Line::from(format!("After: {}", after_file)));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(Span::styled(
            "Summary:",
            Style::default().fg(Color::Yellow),
        )));
        self.results_view.add_line(Line::from(summary));
    }

    pub fn set_schedule_added(&mut self, cron: &str, target: &str) {
        self.state = AppState::Completed;
        self.results_view.clear();

        self.results_view.add_line(Line::from(Span::styled(
            "Schedule Added",
            Style::default().fg(Color::Green),
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
            Style::default().fg(Color::Yellow),
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
    }

    fn set_error(&mut self, msg: String) {
        self.state = AppState::Error(msg.clone());
        self.results_view.add_line(Line::from(Span::styled(
            format!("Error: {}", msg),
            Style::default().fg(Color::Red),
        )));
    }
}

impl TabRender for ReportTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(14),
                Constraint::Min(5),
            ])
            .split(area);

        // View selector
        let mut selector = self.view_selector.clone();
        selector.focused = self.focus_area == ReportFocusArea::ViewSelector;
        selector.render(f, chunks[0]);

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
                    Color::Yellow
                } else {
                    Color::Gray
                }),
            );

        let current_inputs = match self.current_view {
            ReportView::Convert => &self.convert_inputs,
            ReportView::Trend => &self.trend_inputs,
            ReportView::Schedule => &self.schedule_inputs,
        };

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(inputs_block.inner(chunks[1]));

        f.render_widget(inputs_block, chunks[1]);

        for (i, field) in current_inputs.fields.iter().enumerate() {
            if i < input_chunks.len() {
                field.render(f, input_chunks[i], insert_mode);
            }
        }

        // Format selector for Convert view
        if self.current_view == ReportView::Convert {
            let format_area = Rect {
                x: chunks[1].x + chunks[1].width - 25,
                y: chunks[1].y + 1,
                width: 23,
                height: 3,
            };
            let mut fmt_sel = self.format_selector.clone();
            fmt_sel.focused = self.focus_area == ReportFocusArea::Inputs;
            fmt_sel.render(f, format_area);
        }

        // Results
        self.results_view.render(f, chunks[2]);
    }
}

impl TabInput for ReportTab {
    fn handle_focus_next(&mut self) {
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
        if self.focus_area == ReportFocusArea::Inputs {
            let current_inputs = match self.current_view {
                ReportView::Convert => &mut self.convert_inputs,
                ReportView::Trend => &mut self.trend_inputs,
                ReportView::Schedule => &mut self.schedule_inputs,
            };
            current_inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if self.focus_area == ReportFocusArea::Inputs {
            let current_inputs = match self.current_view {
                ReportView::Convert => &mut self.convert_inputs,
                ReportView::Trend => &mut self.trend_inputs,
                ReportView::Schedule => &mut self.schedule_inputs,
            };
            current_inputs.backspace();
        }
    }

    fn handle_enter(&mut self) {
        match self.focus_area {
            ReportFocusArea::ViewSelector => {
                self.view_selector.handle_enter();
                self.current_view = match self.view_selector.selected {
                    0 => ReportView::Convert,
                    1 => ReportView::Trend,
                    2 => ReportView::Schedule,
                    _ => ReportView::Convert,
                };
            }
            ReportFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ReportView::Convert => &mut self.convert_inputs,
                    ReportView::Trend => &mut self.trend_inputs,
                    ReportView::Schedule => &mut self.schedule_inputs,
                };
                current_inputs.blur();
            }
            ReportFocusArea::Results => {}
        }
    }

    fn handle_escape(&mut self) {
        self.view_selector.blur();
        let current_inputs = match self.current_view {
            ReportView::Convert => &mut self.convert_inputs,
            ReportView::Trend => &mut self.trend_inputs,
            ReportView::Schedule => &mut self.schedule_inputs,
        };
        current_inputs.blur();
    }

    fn handle_up(&mut self) {
        match self.focus_area {
            ReportFocusArea::ViewSelector => {
                self.view_selector.handle_up();
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
        match self.focus_area {
            ReportFocusArea::ViewSelector => {
                self.view_selector.handle_down();
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
        match self.focus_area {
            ReportFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ReportView::Convert => &mut self.convert_inputs,
                    ReportView::Trend => &mut self.trend_inputs,
                    ReportView::Schedule => &mut self.schedule_inputs,
                };
                current_inputs.move_left()
            }
            _ => false,
        }
    }

    fn handle_right(&mut self) -> bool {
        match self.focus_area {
            ReportFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ReportView::Convert => &mut self.convert_inputs,
                    ReportView::Trend => &mut self.trend_inputs,
                    ReportView::Schedule => &mut self.schedule_inputs,
                };
                current_inputs.move_right()
            }
            _ => false,
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == ReportFocusArea::Inputs
    }

    fn is_at_left_edge(&self) -> bool {
        match self.focus_area {
            ReportFocusArea::ViewSelector => self.view_selector.selected == 0,
            ReportFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ReportView::Convert => &self.convert_inputs,
                    ReportView::Trend => &self.trend_inputs,
                    ReportView::Schedule => &self.schedule_inputs,
                };
                !current_inputs.can_move_left()
            }
            _ => true,
        }
    }

    fn is_at_right_edge(&self) -> bool {
        match self.focus_area {
            ReportFocusArea::ViewSelector => {
                self.view_selector.selected >= self.view_selector.items.len().saturating_sub(1)
            }
            ReportFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ReportView::Convert => &self.convert_inputs,
                    ReportView::Trend => &self.trend_inputs,
                    ReportView::Schedule => &self.schedule_inputs,
                };
                !current_inputs.can_move_right()
            }
            _ => true,
        }
    }
}

impl ReportTab {
    pub fn stop(&mut self) {
        if self.state == AppState::Running {
            self.state = AppState::Idle;
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.results_view.scroll_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.results_view.scroll_down(page_size);
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
