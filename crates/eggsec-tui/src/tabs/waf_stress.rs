use crate::components::InputField;
use crate::tabs::core::{
    render_config_block, render_input_fields, render_results_area, StandardFocusArea2, TabCore,
};
use crate::tabs::{TabInput, TabRender, TabState};
use crate::{tab_escape_2area, tab_input_boilerplate, tab_state_boilerplate};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

pub struct WafStressTab {
    pub core: TabCore,
    pub focus_area: StandardFocusArea2,
}

impl WafStressTab {
    pub fn new() -> Self {
        let inputs = crate::components::InputGroup::new()
            .add(InputField::new("Target URL"))
            .add(InputField::new("Concurrency").with_value("20"))
            .add(InputField::new("Timeout (s)").with_value("10"));

        Self {
            core: TabCore::new("WAF Stress Testing...", "Results").with_inputs(inputs),
            focus_area: StandardFocusArea2::Inputs,
        }
    }

    pub fn get_results(&self) -> Option<String> {
        if self.core.results_view.is_empty() {
            None
        } else {
            Some(self.core.results_view.get_content())
        }
    }

    pub fn target(&self) -> &str {
        self.core.target()
    }

    pub fn concurrency(&self) -> usize {
        self.core
            .inputs
            .fields
            .get(1)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(20)
    }

    pub fn timeout(&self) -> u64 {
        self.core
            .inputs
            .fields
            .get(2)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(10)
    }

    pub fn update_progress(&mut self, completed: u64, total: u64) {
        self.core.update_progress(completed, total);
    }
}

impl Default for WafStressTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for WafStressTab {
    tab_state_boilerplate!(WafStressTab, core: core);

    fn reset(&mut self) {
        self.core.reset_all();
        if let Some(field) = self.core.inputs.fields.get_mut(1) {
            field.value = "20".to_string();
            field.cursor_pos = 2;
        }
        if let Some(field) = self.core.inputs.fields.get_mut(2) {
            field.value = "10".to_string();
            field.cursor_pos = 2;
        }
        self.focus_area = StandardFocusArea2::Inputs;
    }
}

impl TabRender for WafStressTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(12), Constraint::Min(0)])
            .split(area);

        let input_area = match chunks.first() {
            Some(area) => *area,
            None => return,
        };
        let results_area = match chunks.get(1) {
            Some(area) => *area,
            None => return,
        };

        let input_inner = render_config_block(
            f,
            input_area,
            "WAF Stress Configuration",
            self.focus_area == StandardFocusArea2::Inputs,
        );

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(input_inner);

        render_input_fields(f, &input_chunks, &self.core.inputs, insert_mode);

        render_results_area(
            f,
            results_area,
            &self.core.state,
            &self.core.error,
            &self.core.results_view,
            &self.core.progress,
            "Results",
            "Results will appear here after running",
        );
    }
}

impl TabInput for WafStressTab {
    tab_input_boilerplate!(
        WafStressTab,
        core: core,
        focus: focus_area,
        Inputs: StandardFocusArea2::Inputs,
        Results: StandardFocusArea2::Results
    );
    tab_escape_2area!(WafStressTab, core: core, focus: focus_area, Inputs: StandardFocusArea2::Inputs);

    fn handle_focus_next(&mut self) {
        self.focus_area = crate::tabs::core::focus_next_2area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea2::Inputs,
            StandardFocusArea2::Results,
        );
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = crate::tabs::core::focus_prev_2area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea2::Inputs,
            StandardFocusArea2::Results,
        );
    }

    fn handle_char(&mut self, c: char) {
        let running = self.is_running();
        let inputs = self.focus_area == StandardFocusArea2::Inputs;
        crate::tabs::core::tab_input_char(&mut self.core, c, running, inputs);
    }

    fn handle_backspace(&mut self) {
        let running = self.is_running();
        let inputs = self.focus_area == StandardFocusArea2::Inputs;
        crate::tabs::core::tab_input_backspace(&mut self.core, running, inputs);
    }

    fn handle_paste(&mut self, text: &str) {
        let running = self.is_running();
        let inputs = self.focus_area == StandardFocusArea2::Inputs;
        crate::tabs::core::tab_input_paste(&mut self.core, text, running, inputs);
    }

    fn handle_enter(&mut self) {
        let running = self.is_running();
        let inputs_focused = self.core.inputs.is_focused();
        crate::tabs::core::handle_enter_2area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea2::Inputs,
            StandardFocusArea2::Results,
            running,
            inputs_focused,
        );
    }

    fn handle_up(&mut self) {
        if self.focus_area == StandardFocusArea2::Results {
            self.core.scroll_results_up();
        } else if self.focus_area == StandardFocusArea2::Inputs {
            self.core.inputs.focus_prev();
        }
    }

    fn handle_down(&mut self) {
        if self.focus_area == StandardFocusArea2::Results {
            self.core.scroll_results_down();
        } else if self.focus_area == StandardFocusArea2::Inputs {
            self.core.inputs.focus_next();
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == StandardFocusArea2::Inputs {
            self.core.inputs.move_left()
        } else {
            self.core.results_view.scroll_left(5);
            true
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == StandardFocusArea2::Inputs {
            self.core.inputs.move_right()
        } else {
            self.core.results_view.scroll_right(5);
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == StandardFocusArea2::Inputs && self.core.inputs.is_focused()
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == StandardFocusArea2::Inputs {
            self.core.inputs.is_at_left_edge()
        } else {
            self.core.results_view.is_at_left_edge()
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == StandardFocusArea2::Inputs {
            self.core.inputs.is_at_right_edge()
        } else {
            self.core.results_view.is_at_right_edge()
        }
    }
}
