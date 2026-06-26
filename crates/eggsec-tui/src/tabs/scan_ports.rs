use crate::app::tab_error::TabError;
use crate::components::{Checkbox, InputField, InputGroup, ValidationResult};
use crate::tabs::core::{
    field_as, field_str, render_config_block, render_input_fields, render_results_area,
    StandardFocusArea, TabCore,
};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::{tab_escape, tab_input_boilerplate, tab_state_boilerplate, tc};
use eggsec::scanner::ports::PortScanResults;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    Frame,
};

pub struct ScanPortsTab {
    pub core: TabCore,
    pub results: Option<PortScanResults>,
    pub udp_checkbox: Checkbox,
    pub focus_area: StandardFocusArea,
}

impl ScanPortsTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("Target Host"))
            .add(InputField::new("Ports (e.g., 1-1024 or 22,80,443)").with_value("1-1024"))
            .add(InputField::new("Concurrency").with_value("100"))
            .add(InputField::new("Timeout (s)").with_value("2"));

        Self {
            core: TabCore::new("Scanning ports...", "Results").with_inputs(inputs),
            results: None,
            udp_checkbox: Checkbox::new("Enable UDP (requires root/sudo)").checked(false),
            focus_area: StandardFocusArea::Inputs,
        }
    }

    pub fn get_results(&self) -> Option<&PortScanResults> {
        self.results.as_ref()
    }

    pub fn target(&self) -> &str {
        self.core.target()
    }

    pub fn targets(&self) -> Vec<String> {
        self.core.targets()
    }

    pub fn is_multi_target(&self) -> bool {
        self.core.is_multi_target()
    }

    pub fn ports(&self) -> &str {
        field_str(&self.core, 1)
    }

    pub fn concurrency(&self) -> usize {
        field_as(&self.core, 2, 100)
    }

    pub fn timeout(&self) -> u64 {
        field_as(&self.core, 3, 2)
    }

    pub fn udp(&self) -> bool {
        self.udp_checkbox.checked
    }

    pub fn set_results(&mut self, results: PortScanResults) {
        let _view = self.core.prepare_results();
        self.update_results_view(&results);
        self.results = Some(results);
    }

    fn update_results_view(&mut self, results: &PortScanResults) {
        use ratatui::text::{Line, Span};

        self.core.results_view.clear();

        let host = results.host.clone();
        let ports_scanned = results.ports_scanned;
        let open_ports: Vec<_> = results
            .open_ports
            .iter()
            .map(|p| (p.port, p.service.clone()))
            .collect();

        self.core.results_view.add_line(Line::from(vec![
            Span::styled("Host: ", Style::default().fg(tc!(warning))),
            Span::raw(host),
        ]));

        self.core.results_view.add_line(Line::from(vec![
            Span::styled("Ports scanned: ", Style::default().fg(tc!(info))),
            Span::raw(ports_scanned.to_string()),
            Span::raw(" | "),
            Span::styled("Open: ", Style::default().fg(tc!(success))),
            Span::raw(open_ports.len().to_string()),
        ]));

        self.core.results_view.add_line(Line::from(""));
        self.core.results_view.add_line(Line::from(vec![
            Span::styled(format!("{:<8}", "PORT"), Style::default().fg(tc!(accent))),
            Span::styled(
                format!("{:<15}", "SERVICE"),
                Style::default().fg(tc!(accent)),
            ),
        ]));

        for (port, service) in open_ports {
            self.core.results_view.add_line(Line::from(vec![
                Span::styled(format!("{:<8}", port), Style::default().fg(tc!(success))),
                Span::raw(format!("{:<15}", service)),
            ]));
        }
    }

    pub fn start(&mut self) {
        let target = self.target();
        if target.is_empty() {
            self.core.state = AppState::Error("Target cannot be empty".to_string());
            self.core.error = Some(TabError::Target("Target cannot be empty".to_string()));
            return;
        }

        if self.core.inputs.fields.len() < 2 {
            self.core.state = AppState::Error("Input fields not initialized".to_string());
            self.core.error = Some(TabError::Config("Input fields not initialized".to_string()));
            return;
        }

        if let Some(port_field) = self.core.inputs.fields.get(1) {
            let port_value = port_field.value.clone();
            for t in self.targets() {
                if let Some(target_field) = self.core.inputs.fields.get_mut(0) {
                    let old_target = std::mem::take(&mut target_field.value);
                    target_field.value = t.clone();
                    let target_validation = target_field.validate_ip();
                    target_field.value = old_target;

                    if !target_validation.valid && !t.contains('.') && !t.contains(':') {
                        self.core.state = AppState::Error(format!(
                            "Invalid target: {} - {}",
                            t, target_validation.message
                        ));
                        self.core.error = Some(TabError::Target(format!(
                            "Invalid target: {} - {}",
                            t, target_validation.message
                        )));
                        return;
                    }
                }

                if let Some(port_field) = self.core.inputs.fields.get_mut(1) {
                    let old_port = std::mem::take(&mut port_field.value);
                    port_field.value = port_value.clone();
                    let port_validation = port_field.validate_port_range();
                    port_field.value = old_port;

                    if !port_validation.valid {
                        self.core.state = AppState::Error(format!(
                            "Invalid port range: {}",
                            port_validation.message
                        ));
                        self.core.error = Some(TabError::Config(format!(
                            "Invalid port range: {}",
                            port_validation.message
                        )));
                        return;
                    }
                }
            }
        }

        self.core.state = AppState::Running;
        self.core.progress.current = 0;
        self.results = None;
        self.core.results_view.clear();
        self.core.error = None;
    }

    pub fn update_progress(&mut self, completed: u64, total: u64) {
        self.core.update_progress(completed, total);
    }

    fn update_field_validation(&mut self) {
        if let Some(ref mut target_field) = self.core.inputs.fields.get_mut(0) {
            let validation = if target_field.value.contains('.') || target_field.value.contains(':')
            {
                target_field.validate_ip()
            } else {
                ValidationResult {
                    valid: !target_field.value.is_empty(),
                    message: if target_field.value.is_empty() {
                        "Target cannot be empty".to_string()
                    } else {
                        String::new()
                    },
                }
            };
            target_field.validation = Some(validation);
        }
        if let Some(ref mut port_field) = self.core.inputs.fields.get_mut(1) {
            port_field.validation = Some(port_field.validate_port_range());
        }
    }
}

impl Default for ScanPortsTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for ScanPortsTab {
    tab_state_boilerplate!(ScanPortsTab, core: core);

    fn reset(&mut self) {
        self.core.reset_all();
        self.core.inputs.blur();
        if let Some(field) = self.core.inputs.fields.get_mut(1) {
            field.value = "1-1024".to_string();
            field.cursor_pos = 6;
        }
        if let Some(field) = self.core.inputs.fields.get_mut(2) {
            field.value = "100".to_string();
            field.cursor_pos = 3;
        }
        if let Some(field) = self.core.inputs.fields.get_mut(3) {
            field.value = "2".to_string();
            field.cursor_pos = 1;
        }
        self.focus_area = StandardFocusArea::Inputs;
        self.udp_checkbox.checked = false;
        self.udp_checkbox.focused = false;
    }
}

impl TabRender for ScanPortsTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(17), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        let input_inner = render_config_block(
            f,
            input_area,
            "Port Scan Configuration",
            self.focus_area == StandardFocusArea::Inputs,
        );

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(input_inner);

        render_input_fields(f, &input_chunks, &self.core.inputs, insert_mode);

        if let Some(chunk) = input_chunks.get(4) {
            self.udp_checkbox.render_with_focus(
                self.focus_area == StandardFocusArea::Options,
                f,
                *chunk,
            );
        }

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

impl TabInput for ScanPortsTab {
    tab_input_boilerplate!(
        ScanPortsTab,
        core: core,
        focus: focus_area,
        Inputs: StandardFocusArea::Inputs,
        Results: StandardFocusArea::Results
    );

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.insert(c);
            self.update_field_validation();
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.backspace();
            self.update_field_validation();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.paste(text);
            self.update_field_validation();
        }
    }

    fn handle_focus_next(&mut self) {
        self.focus_area = crate::tabs::core::focus_next_3area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea::Inputs,
            StandardFocusArea::Options,
            StandardFocusArea::Results,
        );
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = crate::tabs::core::focus_prev_3area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea::Inputs,
            StandardFocusArea::Options,
            StandardFocusArea::Results,
        );
    }

    fn handle_up(&mut self) {
        crate::tabs::core::handle_up_3area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea::Inputs,
            StandardFocusArea::Results,
        );
    }

    fn handle_down(&mut self) {
        crate::tabs::core::handle_down_3area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea::Inputs,
            StandardFocusArea::Results,
        );
    }

    fn handle_enter(&mut self) {
        let running = self.is_running();
        let inputs_focused = self.core.inputs.is_focused();
        crate::tabs::core::handle_enter_3area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea::Inputs,
            StandardFocusArea::Options,
            StandardFocusArea::Results,
            running,
            inputs_focused,
            |_core| false,
        );
        if self.focus_area == StandardFocusArea::Options && !self.is_running() {
            self.udp_checkbox.checked = !self.udp_checkbox.checked;
        }
    }

    tab_escape!(ScanPortsTab, core: core, focus: focus_area, strategy: three_area, Inputs: StandardFocusArea::Inputs, Options: StandardFocusArea::Options, Results: StandardFocusArea::Results);

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.move_left()
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.move_right()
        } else {
            false
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == StandardFocusArea::Inputs && self.core.inputs.is_focused()
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.fields.is_empty() || self.core.inputs.is_at_left_edge()
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.fields.is_empty() || self.core.inputs.is_at_right_edge()
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tab() -> ScanPortsTab {
        ScanPortsTab::new()
    }

    #[test]
    fn test_enter_in_inputs_focused_blurs_does_not_start() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Inputs;
        tab.core.inputs.focus(0);
        assert!(tab.core.inputs.is_focused());
        tab.handle_enter();
        assert!(!tab.core.inputs.is_focused());
        assert!(!tab.is_running());
    }

    #[test]
    fn test_enter_in_options_toggles_does_not_start() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Options;
        let before = tab.udp_checkbox.checked;
        tab.handle_enter();
        assert_eq!(tab.udp_checkbox.checked, !before);
        assert!(!tab.is_running());
    }

    #[test]
    fn test_enter_in_results_no_op() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Results;
        tab.handle_enter();
        assert!(!tab.is_running());
    }
}
