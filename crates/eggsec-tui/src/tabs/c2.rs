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
pub enum C2FocusArea {
    Target,
    Campaign,
    Results,
}

pub struct C2Tab {
    pub inputs: InputGroup,
    pub results: String,
    pub state: AppState,
    pub focus_area: C2FocusArea,
    pub error: Option<TabError>,
    progress: f64,
}

impl C2Tab {
    pub fn new() -> Self {
        Self {
            inputs: InputGroup::new()
                .add(InputField::new("Target").with_width(50).with_value("localhost"))
                .add(InputField::new("Campaign Profile").with_width(30).with_value("apt29")),
            results: "Ready for C2 simulation. Select a campaign profile and press Enter.\n\nAvailable profiles: apt29, carbanak, default".to_string(),
            state: AppState::Idle,
            focus_area: C2FocusArea::Target,
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
        self.focus_area = C2FocusArea::Target;
        self.progress = 0.0;
        self.results = "Ready for C2 simulation. Select a campaign profile and press Enter.\n\nAvailable profiles: apt29, carbanak, default".to_string();
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

impl TabState for C2Tab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        self.progress
    }

    fn reset(&mut self) {
        C2Tab::reset(self);
    }

    fn set_error(&mut self, error: TabError) {
        C2Tab::set_error_state(self, error);
    }
}

impl TabRender for C2Tab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let focus = match self.focus_area {
            C2FocusArea::Target => "Target",
            C2FocusArea::Campaign => "Campaign",
            C2FocusArea::Results => "Results",
        };
        Some(vec!["C2", focus])
    }

    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        use ratatui::style::Style;
        use crate::components::FormBuilder;

        if let Some(ref err) = self.error {
            let error_text = Paragraph::new(format!("Error: {}", err.message()))
                .block(Block::default().borders(Borders::ALL).title("C2 - Error"))
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, area);
            return;
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(9),
                Constraint::Min(8),
            ])
            .split(area);

        let Some(title_area) = layout.get(0) else { return; };
        let Some(inputs_area) = layout.get(1) else { return; };
        let Some(results_area) = layout.get(2) else { return; };

        let title = Paragraph::new("C2 Campaign Simulation — Defense-lab only | Beacons, Tasking, OPSEC, Attack Graph")
            .block(Block::default().borders(Borders::ALL).title("\u{26a0} Defense Lab"))
            .style(Style::default().fg(tc!(warning)));
        f.render_widget(title, *title_area);

        let mut builder = FormBuilder::new("Inputs").row_height(3);
        for field in &self.inputs.fields {
            builder = builder.add_input(field.clone());
        }
        builder.render(f, *inputs_area, insert_mode);

        let results_content = if self.results.is_empty() || self.results.starts_with("Ready") {
            crate::components::empty_state_paragraph("Results", "No results yet. Run a simulation to see campaign results.")
        } else {
            Paragraph::new(self.results.as_str())
                .block(Block::default().borders(Borders::ALL).title("C2 Campaign Results"))
                .style(Style::default().fg(tc!(text)))
        };
        f.render_widget(results_content, *results_area);
    }
}

impl TabInput for C2Tab {
    fn stop(&mut self) {
        C2Tab::stop(self);
    }

    fn handle_focus_next(&mut self) {
        if !self.is_running() {
            self.focus_area = match self.focus_area {
                C2FocusArea::Target => C2FocusArea::Campaign,
                C2FocusArea::Campaign => C2FocusArea::Results,
                C2FocusArea::Results => C2FocusArea::Target,
            };
            self.sync_input_focus();
        }
    }

    fn handle_focus_prev(&mut self) {
        if !self.is_running() {
            self.focus_area = match self.focus_area {
                C2FocusArea::Target => C2FocusArea::Results,
                C2FocusArea::Campaign => C2FocusArea::Target,
                C2FocusArea::Results => C2FocusArea::Campaign,
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
            self.focus_area = C2FocusArea::Target;
            self.sync_input_focus();
        }
    }

    fn handle_bottom(&mut self) {
        if !self.is_running() {
            self.focus_area = C2FocusArea::Results;
            self.sync_input_focus();
        }
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }

        if self.focus_area == C2FocusArea::Results {
            return;
        }

        if self.target().map_or(true, |t| t.is_empty()) {
            self.set_error_state(TabError::Target("Target is required for C2 simulation".to_string()));
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
        self.focus_area = C2FocusArea::Results;
        self.sync_input_focus();
    }

    fn handle_up(&mut self) {
        if !self.is_running() {
            match self.focus_area {
                C2FocusArea::Results => {
                    self.focus_area = C2FocusArea::Campaign;
                    if self.inputs.fields.len() > 1 { self.inputs.focus(1); }
                }
                C2FocusArea::Campaign => {
                    self.focus_area = C2FocusArea::Target;
                    if self.inputs.fields.len() > 0 { self.inputs.focus(0); }
                }
                C2FocusArea::Target => {
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
                C2FocusArea::Target => {
                    self.focus_area = C2FocusArea::Campaign;
                    if self.inputs.fields.len() > 1 { self.inputs.focus(1); }
                }
                C2FocusArea::Campaign => {
                    self.focus_area = C2FocusArea::Results;
                    self.inputs.blur();
                }
                C2FocusArea::Results => {}
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
        !matches!(self.focus_area, C2FocusArea::Results)
    }
}

impl C2Tab {
    fn current_input_index(&self) -> Option<usize> {
        match self.focus_area {
            C2FocusArea::Target if self.inputs.fields.len() > 0 => Some(0),
            C2FocusArea::Campaign if self.inputs.fields.len() > 1 => Some(1),
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

    pub fn campaign(&self) -> Option<&str> {
        self.inputs.fields.get(1).map(|f| f.value.as_str()).filter(|v| !v.is_empty())
    }

    pub fn primary_target(&self) -> Option<String> {
        self.target().map(|s| s.to_string())
    }

    pub fn build_task_config(&self) -> Option<TaskConfig> {
        let target = self.target()?.to_string();
        if target.is_empty() {
            return None;
        }

        let campaign = self.campaign().unwrap_or("default").to_string();

        Some(TaskConfig::C2 {
            target,
            campaign,
            dry_run: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_returns_none_when_empty() {
        let mut tab = C2Tab::new();
        tab.inputs.fields[0].value.clear();
        assert!(tab.target().is_none());
    }

    #[test]
    fn test_target_returns_value_when_set() {
        let mut tab = C2Tab::new();
        tab.inputs.fields[0].value = "10.0.0.1".to_string();
        assert_eq!(tab.target(), Some("10.0.0.1"));
    }

    #[test]
    fn test_campaign_returns_default_when_empty() {
        let tab = C2Tab::new();
        // Default value is "apt29"
        assert_eq!(tab.campaign(), Some("apt29"));
    }

    #[test]
    fn test_build_task_config_returns_none_without_target() {
        let mut tab = C2Tab::new();
        tab.inputs.fields[0].value.clear();
        assert!(tab.build_task_config().is_none());
    }

    #[test]
    fn test_build_task_config_uses_ui_values() {
        let mut tab = C2Tab::new();
        tab.inputs.fields[0].value = "10.0.0.1".to_string();
        tab.inputs.fields[1].value = "carbanak".to_string();

        let config = tab.build_task_config().unwrap();
        match config {
            TaskConfig::C2 { target, campaign, dry_run } => {
                assert_eq!(target, "10.0.0.1");
                assert_eq!(campaign, "carbanak");
                assert!(dry_run);
            }
            _ => panic!("Expected TaskConfig::C2"),
        }
    }
}
