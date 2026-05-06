use crate::tc;
use crate::tui::app::tab_error::TabError;
use crate::tui::components::{InputField, InputGroup, ScrollableText, Selector};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

#[derive(Clone, Copy, PartialEq)]
pub enum ClusterView {
    Worker,
    Coordinator,
    Status,
}

pub struct ClusterTab {
    pub view_selector: Selector,
    pub worker_inputs: InputGroup,
    pub coordinator_inputs: InputGroup,
    pub status_inputs: InputGroup,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub current_view: ClusterView,
    pub focus_area: ClusterFocusArea,
    pub error: Option<TabError>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClusterFocusArea {
    ViewSelector,
    Inputs,
    Results,
}

impl Default for ClusterTab {
    fn default() -> Self {
        Self::new()
    }
}

impl ClusterTab {
    pub fn new() -> Self {
        let view_selector =
            Selector::new("Mode").simple_items(vec!["Worker", "Coordinator", "Status"]);

        let worker_inputs = InputGroup::new()
            .add(InputField::new("Coordinator Address").with_value("localhost:9000"))
            .add(InputField::new("Worker Threads").with_value("4"))
            .add(InputField::new("Worker ID (optional)"))
            .add(InputField::new("Pre-Shared Key (optional)"));

        let coordinator_inputs = InputGroup::new()
            .add(InputField::new("Port").with_value("9000"))
            .add(InputField::new("Bind Address (optional)"))
            .add(InputField::new("Max Workers (optional)"))
            .add(InputField::new("Pre-Shared Key (optional)"));

        let status_inputs =
            InputGroup::new().add(InputField::new("Coordinator Address (optional)"));

        Self {
            view_selector,
            worker_inputs,
            coordinator_inputs,
            status_inputs,
            state: AppState::Idle,
            results_view: ScrollableText::new("Cluster Status"),
            current_view: ClusterView::Worker,
            focus_area: ClusterFocusArea::ViewSelector,
            error: None,
        }
    }

    pub fn coordinator(&self) -> &str {
        self.worker_inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("localhost:9000")
    }

    pub fn worker_threads(&self) -> usize {
        self.worker_inputs
            .fields
            .get(1)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(4)
    }

    pub fn coordinator_port(&self) -> u16 {
        self.coordinator_inputs
            .fields
            .first()
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(9000)
    }

    pub fn max_workers(&self) -> Option<usize> {
        self.coordinator_inputs
            .fields
            .get(2)
            .filter(|f| !f.value.is_empty())
            .and_then(|f| f.value.parse().ok())
    }

    pub fn psk(&self) -> Option<&str> {
        let inputs = match self.current_view {
            ClusterView::Worker => &self.worker_inputs,
            ClusterView::Coordinator => &self.coordinator_inputs,
            _ => return None,
        };
        inputs
            .fields
            .last()
            .map(|f| f.value.as_str())
            .filter(|v| !v.is_empty())
    }

    pub fn set_status_results(&mut self, results: ClusterStatusResults) {
        self.state = AppState::Completed;
        self.results_view.clear();

        self.results_view.add_line(Line::from(Span::styled(
            "Cluster Status",
            Style::default().fg(tc!(success)),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(format!(
            "Coordinator: {}",
            results.coordinator_address
        )));
        self.results_view.add_line(Line::from(format!(
            "Total Workers: {}",
            results.total_workers
        )));
        self.results_view.add_line(Line::from(format!(
            "Active Workers: {}",
            results.active_workers
        )));
        self.results_view.add_line(Line::from(format!(
            "Pending Tasks: {}",
            results.pending_tasks
        )));
        self.results_view.add_line(Line::from(format!(
            "Completed Tasks: {}",
            results.completed_tasks
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(Span::styled(
            "Workers:",
            Style::default().fg(tc!(warning)),
        )));
        for worker in &results.workers {
            let status_color = if worker.is_active {
                tc!(success)
            } else {
                tc!(error)
            };
            let status_text = if worker.is_active {
                "Active"
            } else {
                "Inactive"
            };
            self.results_view.add_line(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::raw(worker.id.clone()),
                Span::raw(" - "),
                Span::styled(status_text, Style::default().fg(status_color)),
                Span::raw(format!(" (Tasks: {})", worker.tasks_completed)),
            ]));
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClusterStatusResults {
    pub coordinator_address: String,
    pub total_workers: usize,
    pub active_workers: usize,
    pub pending_tasks: usize,
    pub completed_tasks: usize,
    pub workers: Vec<WorkerInfo>,
}

#[derive(Clone, Debug)]
pub struct WorkerInfo {
    pub id: String,
    pub is_active: bool,
    pub tasks_completed: usize,
}

impl TabState for ClusterTab {
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
        self.error = Some(error.clone());
        self.results_view.add_line(Line::from(Span::styled(
            format!("Error: {}", error.message()),
            Style::default().fg(tc!(error)),
        )));
    }
}

impl TabRender for ClusterTab {
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
        selector.focused = self.focus_area == ClusterFocusArea::ViewSelector;
        selector.render(f, chunks[0]);

        // Inputs based on current view
        let inputs_block = Block::default()
            .title(match self.current_view {
                ClusterView::Worker => " Worker Configuration ",
                ClusterView::Coordinator => " Coordinator Configuration ",
                ClusterView::Status => " Status Query ",
            })
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if self.focus_area == ClusterFocusArea::Inputs {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );

        let current_inputs = match self.current_view {
            ClusterView::Worker => &self.worker_inputs,
            ClusterView::Coordinator => &self.coordinator_inputs,
            ClusterView::Status => &self.status_inputs,
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

        // Results
        self.results_view.render(f, chunks[2], None);
    }
}

impl TabInput for ClusterTab {
    fn handle_focus_next(&mut self) {
        self.focus_area = match self.focus_area {
            ClusterFocusArea::ViewSelector => {
                self.view_selector.blur();
                ClusterFocusArea::Inputs
            }
            ClusterFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ClusterView::Worker => &mut self.worker_inputs,
                    ClusterView::Coordinator => &mut self.coordinator_inputs,
                    ClusterView::Status => &mut self.status_inputs,
                };
                current_inputs.blur();
                ClusterFocusArea::Results
            }
            ClusterFocusArea::Results => {
                self.view_selector.focus();
                ClusterFocusArea::ViewSelector
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = match self.focus_area {
            ClusterFocusArea::ViewSelector => {
                self.view_selector.blur();
                ClusterFocusArea::Results
            }
            ClusterFocusArea::Inputs => {
                self.view_selector.focus();
                ClusterFocusArea::ViewSelector
            }
            ClusterFocusArea::Results => {
                let current_inputs = match self.current_view {
                    ClusterView::Worker => &mut self.worker_inputs,
                    ClusterView::Coordinator => &mut self.coordinator_inputs,
                    ClusterView::Status => &mut self.status_inputs,
                };
                current_inputs.focus(0);
                ClusterFocusArea::Inputs
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if self.focus_area == ClusterFocusArea::Inputs {
            let current_inputs = match self.current_view {
                ClusterView::Worker => &mut self.worker_inputs,
                ClusterView::Coordinator => &mut self.coordinator_inputs,
                ClusterView::Status => &mut self.status_inputs,
            };
            current_inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if self.focus_area == ClusterFocusArea::Inputs {
            let current_inputs = match self.current_view {
                ClusterView::Worker => &mut self.worker_inputs,
                ClusterView::Coordinator => &mut self.coordinator_inputs,
                ClusterView::Status => &mut self.status_inputs,
            };
            current_inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if self.focus_area == ClusterFocusArea::Inputs {
            let current_inputs = match self.current_view {
                ClusterView::Worker => &mut self.worker_inputs,
                ClusterView::Coordinator => &mut self.coordinator_inputs,
                ClusterView::Status => &mut self.status_inputs,
            };
            current_inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.focus_area == ClusterFocusArea::Inputs {
            let current_inputs = match self.current_view {
                ClusterView::Worker => &self.worker_inputs,
                ClusterView::Coordinator => &self.coordinator_inputs,
                ClusterView::Status => &self.status_inputs,
            };
            current_inputs.get_focused_value()
        } else if self.focus_area == ClusterFocusArea::Results {
            Some(self.results_view.get_content())
        } else {
            None
        }
    }

    fn handle_word_forward(&mut self) {
        if self.focus_area == ClusterFocusArea::Inputs {
            let current_inputs = match self.current_view {
                ClusterView::Worker => &mut self.worker_inputs,
                ClusterView::Coordinator => &mut self.coordinator_inputs,
                ClusterView::Status => &mut self.status_inputs,
            };
            current_inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if self.focus_area == ClusterFocusArea::Inputs {
            let current_inputs = match self.current_view {
                ClusterView::Worker => &mut self.worker_inputs,
                ClusterView::Coordinator => &mut self.coordinator_inputs,
                ClusterView::Status => &mut self.status_inputs,
            };
            current_inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if self.focus_area == ClusterFocusArea::Inputs {
            let current_inputs = match self.current_view {
                ClusterView::Worker => &mut self.worker_inputs,
                ClusterView::Coordinator => &mut self.coordinator_inputs,
                ClusterView::Status => &mut self.status_inputs,
            };
            current_inputs.move_home();
        } else if self.focus_area == ClusterFocusArea::Results {
            self.results_view.scroll_to_top();
        }
    }

    fn handle_end(&mut self) {
        if self.focus_area == ClusterFocusArea::Inputs {
            let current_inputs = match self.current_view {
                ClusterView::Worker => &mut self.worker_inputs,
                ClusterView::Coordinator => &mut self.coordinator_inputs,
                ClusterView::Status => &mut self.status_inputs,
            };
            current_inputs.move_end();
        } else if self.focus_area == ClusterFocusArea::Results {
            self.results_view.scroll_to_bottom();
        }
    }

    fn handle_top(&mut self) {
        self.focus_area = ClusterFocusArea::ViewSelector;
        self.view_selector.focus();
    }

    fn handle_bottom(&mut self) {
        self.focus_area = ClusterFocusArea::Results;
    }

    fn handle_enter(&mut self) {
        match self.focus_area {
            ClusterFocusArea::ViewSelector => {
                if self.view_selector.is_open() {
                    let _ = self.view_selector.confirm();
                    self.current_view = match self.view_selector.selected {
                        0 => ClusterView::Worker,
                        1 => ClusterView::Coordinator,
                        2 => ClusterView::Status,
                        _ => ClusterView::Worker,
                    };
                } else {
                    self.view_selector.open();
                }
            }
            ClusterFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ClusterView::Worker => &mut self.worker_inputs,
                    ClusterView::Coordinator => &mut self.coordinator_inputs,
                    ClusterView::Status => &mut self.status_inputs,
                };
                current_inputs.blur();
            }
            ClusterFocusArea::Results => {}
        }
    }

    fn handle_escape(&mut self) {
        if self.view_selector.is_open() {
            self.view_selector.cancel();
            return;
        }
        self.view_selector.blur();
        let current_inputs = match self.current_view {
            ClusterView::Worker => &mut self.worker_inputs,
            ClusterView::Coordinator => &mut self.coordinator_inputs,
            ClusterView::Status => &mut self.status_inputs,
        };
        current_inputs.blur();
    }

    fn handle_up(&mut self) {
        match self.focus_area {
            ClusterFocusArea::ViewSelector => {
                if self.view_selector.is_open() {
                    self.view_selector.move_prev();
                }
            }
            ClusterFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ClusterView::Worker => &mut self.worker_inputs,
                    ClusterView::Coordinator => &mut self.coordinator_inputs,
                    ClusterView::Status => &mut self.status_inputs,
                };
                current_inputs.focus_prev();
            }
            ClusterFocusArea::Results => {
                self.results_view.scroll_up(1);
            }
        }
    }

    fn handle_down(&mut self) {
        match self.focus_area {
            ClusterFocusArea::ViewSelector => {
                if self.view_selector.is_open() {
                    self.view_selector.move_next();
                }
            }
            ClusterFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ClusterView::Worker => &mut self.worker_inputs,
                    ClusterView::Coordinator => &mut self.coordinator_inputs,
                    ClusterView::Status => &mut self.status_inputs,
                };
                current_inputs.focus_next();
            }
            ClusterFocusArea::Results => {
                self.results_view.scroll_down(1);
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        match self.focus_area {
            ClusterFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ClusterView::Worker => &mut self.worker_inputs,
                    ClusterView::Coordinator => &mut self.coordinator_inputs,
                    ClusterView::Status => &mut self.status_inputs,
                };
                current_inputs.move_left()
            }
            _ => false,
        }
    }

    fn handle_right(&mut self) -> bool {
        match self.focus_area {
            ClusterFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ClusterView::Worker => &mut self.worker_inputs,
                    ClusterView::Coordinator => &mut self.coordinator_inputs,
                    ClusterView::Status => &mut self.status_inputs,
                };
                current_inputs.move_right()
            }
            _ => false,
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == ClusterFocusArea::Inputs
    }

    fn is_at_left_edge(&self) -> bool {
        match self.focus_area {
            ClusterFocusArea::ViewSelector => {
                if self.view_selector.is_open() {
                    self.view_selector.selected == 0
                } else {
                    true
                }
            }
            ClusterFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ClusterView::Worker => &self.worker_inputs,
                    ClusterView::Coordinator => &self.coordinator_inputs,
                    ClusterView::Status => &self.status_inputs,
                };
                !current_inputs.can_move_left()
            }
            _ => true,
        }
    }

    fn is_at_right_edge(&self) -> bool {
        match self.focus_area {
            ClusterFocusArea::ViewSelector => {
                if self.view_selector.is_open() {
                    self.view_selector.selected
                        >= self.view_selector.items.len().saturating_sub(1)
                } else {
                    true
                }
            }
            ClusterFocusArea::Inputs => {
                let current_inputs = match self.current_view {
                    ClusterView::Worker => &self.worker_inputs,
                    ClusterView::Coordinator => &self.coordinator_inputs,
                    ClusterView::Status => &self.status_inputs,
                };
                !current_inputs.can_move_right()
            }
            _ => true,
        }
    }
}

impl ClusterTab {
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

    pub fn scroll_results_page_up(&mut self) {
        self.results_view.scroll_up(1);
    }

    pub fn scroll_results_page_down(&mut self) {
        self.results_view.scroll_down(1);
    }

    pub fn scroll_results_to_top(&mut self) {
        self.results_view.scroll_to_top();
    }

    pub fn scroll_results_to_bottom(&mut self) {
        self.results_view.scroll_to_bottom();
    }
}
