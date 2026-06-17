#![allow(dead_code)]

use crate::app::tab_error::TabError;
use crate::components::{empty_state_paragraph, InputField, InputGroup, ScrollableText, Selector};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::tc;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Clone, Copy, PartialEq)]
pub enum PacketView {
    Capture,
    Send,
    Dump,
    Icmp,
    Traceroute,
    Interfaces,
}

pub struct PacketTab {
    pub view_selector: Selector,
    pub inputs: InputGroup,
    pub results: Vec<String>,
    pub current_view: PacketView,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub is_root: bool,
    pub privileges_required: bool,
    pub error: Option<TabError>,
}

impl PacketTab {
    pub fn new() -> Self {
        let view_selector = Selector::new("Tool").simple_items(vec![
            "Capture",
            "Send",
            "Dump",
            "ICMP Echo",
            "Traceroute",
            "Interfaces",
        ]);

        let inputs = InputGroup::new()
            .add(InputField::new("Target / Interface"))
            .add(InputField::new("Filter / Port"))
            .add(InputField::new("Max Packets (default: 100)").with_value("100"))
            .add(InputField::new("Output File (optional)"));

        #[cfg(feature = "stress-testing")]
        let is_root = eggsec::utils::privilege::is_root();
        #[cfg(not(feature = "stress-testing"))]
        let is_root = false;
        let privileges_required = true;

        Self {
            view_selector,
            inputs,
            results: Vec::new(),
            current_view: PacketView::Capture,
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            is_root,
            privileges_required,
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

    pub fn filter(&self) -> &str {
        self.inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn max_packets(&self) -> usize {
        self.inputs
            .fields
            .get(2)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(100)
    }

    pub fn output_file(&self) -> Option<&str> {
        let v = self
            .inputs
            .fields
            .get(3)
            .map(|f| f.value.as_str())
            .unwrap_or("");
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    }

    pub fn run_interfaces(&mut self) {
        self.results_view.clear();

        #[cfg(all(feature = "packet-inspection", unix))]
        {
            use pnet::datalink;
            let interfaces = datalink::interfaces();

            self.results_view.add_line(Line::from(vec![Span::styled(
                "Available Network Interfaces",
                Style::default().fg(tc!(accent)),
            )]));
            self.results_view.add_line(Line::from(""));

            for iface in interfaces {
                let ips: Vec<String> = iface.ips.iter().map(|ip| format!("{}", ip)).collect();
                let name = iface.name.clone();
                self.results_view.add_line(Line::from(vec![
                    Span::styled(name, Style::default().fg(tc!(info))),
                    Span::raw(format!(" - {}", ips.join(", "))),
                ]));
            }
        }

        #[cfg(not(all(feature = "packet-inspection", unix)))]
        {
            self.results_view.add_line(Line::from(vec![
                Span::styled("Error: ", Style::default().fg(tc!(error))),
                Span::raw("Packet inspection not available on this platform."),
            ]));
        }

        self.state = AppState::Completed;
    }

    pub fn set_send_results(&mut self, packets_sent: u32, bytes_sent: u64) {
        use ratatui::style::Style;
        use ratatui::text::{Line, Span};

        self.results_view.clear();

        self.results_view.add_line(Line::from(vec![Span::styled(
            "Packet Send Complete",
            Style::default().fg(tc!(accent)),
        )]));
        self.results_view.add_line(Line::from(""));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Packets sent: ", Style::default().fg(tc!(info))),
            Span::raw(packets_sent.to_string()),
        ]));
        self.results_view.add_line(Line::from(vec![
            Span::styled("Bytes sent: ", Style::default().fg(tc!(info))),
            Span::raw(bytes_sent.to_string()),
        ]));

        self.state = AppState::Completed;
    }

    pub fn set_capture_results(&mut self, packets_captured: usize, output_file: Option<String>) {
        use ratatui::style::Style;
        use ratatui::text::{Line, Span};

        self.results_view.clear();

        self.results_view.add_line(Line::from(vec![Span::styled(
            "Packet Capture Complete",
            Style::default().fg(tc!(accent)),
        )]));
        self.results_view.add_line(Line::from(""));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Packets captured: ", Style::default().fg(tc!(info))),
            Span::raw(packets_captured.to_string()),
        ]));

        if let Some(file) = output_file {
            self.results_view.add_line(Line::from(vec![
                Span::styled("Output file: ", Style::default().fg(tc!(info))),
                Span::raw(file),
            ]));
        }

        self.state = AppState::Completed;
    }

    pub fn set_traceroute_results(
        &mut self,
        hops: Vec<super::super::workers::TracerouteHopResult>,
    ) {
        use ratatui::style::Style;
        use ratatui::text::{Line, Span};

        self.results_view.clear();

        self.results_view.add_line(Line::from(vec![Span::styled(
            "Traceroute Results",
            Style::default().fg(tc!(accent)),
        )]));
        self.results_view.add_line(Line::from(""));

        for hop in hops {
            let addr = hop.address.unwrap_or_else(|| "*".to_string());
            let rtt = hop
                .rtt_ms
                .map(|ms| format!("{:.2} ms", ms))
                .unwrap_or_else(|| "timeout".to_string());
            self.results_view
                .add_line(Line::from(format!("{:2}  {}  {}", hop.hop, addr, rtt)));
        }

        self.state = AppState::Completed;
    }

    pub fn run_traceroute(&mut self) {
        self.results_view.clear();

        let target = self.target().to_string();
        if target.is_empty() {
            self.results_view.add_line(Line::from(vec![
                Span::styled("Error: ", Style::default().fg(tc!(error))),
                Span::raw("Target is required"),
            ]));
            self.state = AppState::Idle;
            return;
        }

        self.results_view.add_line(Line::from(vec![
            Span::styled("Traceroute to ", Style::default().fg(tc!(accent))),
            Span::raw(target.clone()),
        ]));
        self.results_view.add_line(Line::from(
            "Note: Traceroute requires root and running in async context",
        ));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(format!(
            "Use CLI for traceroute: eggsec packet traceroute {}",
            target
        )));

        self.state = AppState::Completed;
    }

    pub fn run_icmp(&mut self) {
        self.results_view.clear();

        let target = self.target().to_string();
        if target.is_empty() {
            self.results_view.add_line(Line::from(vec![
                Span::styled("Error: ", Style::default().fg(tc!(error))),
                Span::raw("Target host is required"),
            ]));
            self.state = AppState::Idle;
            return;
        }

        self.results_view.add_line(Line::from(vec![
            Span::styled("ICMP Echo (Ping) to ", Style::default().fg(tc!(accent))),
            Span::raw(target.clone()),
        ]));
        self.results_view
            .add_line(Line::from("Note: ICMP requires root/admin privileges"));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(format!(
            "Use CLI for ICMP: eggsec icmp {}",
            target
        )));

        self.state = AppState::Completed;
    }

    pub fn run_capture(&mut self) {
        self.results_view.clear();

        #[cfg(all(feature = "packet-inspection", unix))]
        {
            let target = self.target();
            if target.is_empty() {
                self.results_view.add_line(Line::from(vec![
                    Span::styled("Error: ", Style::default().fg(tc!(error))),
                    Span::raw("Interface name is required"),
                ]));
                self.state = AppState::Idle;
                return;
            }

            self.results_view.add_line(Line::from(format!(
                "Starting capture on interface: {}",
                target
            )));
            self.results_view
                .add_line(Line::from("Note: Live capture requires root privileges"));
            self.results_view.add_line(Line::from(""));
            self.results_view
                .add_line(Line::from(format!("Max packets: {}", self.max_packets())));
        }

        #[cfg(not(all(feature = "packet-inspection", unix)))]
        {
            self.results_view.add_line(Line::from(vec![
                Span::styled("Error: ", Style::default().fg(tc!(error))),
                Span::raw("Packet inspection not available on this platform."),
            ]));
        }

        self.state = AppState::Completed;
    }

    pub fn run_send(&mut self) {
        self.results_view.clear();

        let target = self.target().to_string();
        if target.is_empty() {
            self.results_view.add_line(Line::from(vec![
                Span::styled("Error: ", Style::default().fg(tc!(error))),
                Span::raw("Target is required"),
            ]));
            self.state = AppState::Idle;
            return;
        }

        self.results_view.add_line(Line::from(vec![
            Span::styled("Send packets to ", Style::default().fg(tc!(accent))),
            Span::raw(target.clone()),
        ]));
        self.results_view.add_line(Line::from(
            "Note: Packet sending requires root and running in async context",
        ));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(format!(
            "Use CLI for packet sending: eggsec packet send {} --dst-port {}",
            target,
            self.filter()
        )));

        self.state = AppState::Completed;
    }

    pub fn run_dump(&mut self) {
        self.results_view.clear();

        let target = self.target().to_string();
        if target.is_empty() {
            self.results_view.add_line(Line::from(vec![
                Span::styled("Error: ", Style::default().fg(tc!(error))),
                Span::raw("Input file is required"),
            ]));
            self.state = AppState::Idle;
            return;
        }

        #[cfg(feature = "packet-inspection")]
        {
            use eggsec::packet::hexdump;
            use std::fs::File;
            use std::io::Read;

            match File::open(&target) {
                Ok(mut file) => {
                    let mut buffer = Vec::new();
                    if file.read_to_end(&mut buffer).is_ok() {
                        let dump = hexdump(&buffer);
                        self.results_view.add_line(Line::from(vec![
                            Span::styled("Hexdump: ", Style::default().fg(tc!(accent))),
                            Span::raw(target),
                        ]));
                        self.results_view.add_line(Line::from(""));
                        for line in dump.lines().take(100) {
                            self.results_view.add_line(Line::from(line.to_string()));
                        }
                    } else {
                        self.results_view.add_line(Line::from(vec![
                            Span::styled("Error: ", Style::default().fg(tc!(error))),
                            Span::raw("Failed to read file"),
                        ]));
                    }
                }
                Err(e) => {
                    self.results_view.add_line(Line::from(vec![
                        Span::styled("Error: ", Style::default().fg(tc!(error))),
                        Span::raw(format!("Failed to open file: {}", e)),
                    ]));
                }
            }
        }

        #[cfg(not(feature = "packet-inspection"))]
        {
            self.results_view.add_line(Line::from(vec![
                Span::styled("Error: ", Style::default().fg(tc!(error))),
                Span::raw("Packet inspection not available on this platform."),
            ]));
        }

        self.state = AppState::Completed;
    }

    pub fn execute(&mut self) {
        if !self.can_run() {
            self.state = AppState::Error("Root privileges required".to_string());
            return;
        }

        match self.current_view {
            PacketView::Capture => {
                self.state = AppState::Running;
                self.run_capture();
            }
            PacketView::Send => {
                self.state = AppState::Running;
                self.run_send();
            }
            PacketView::Dump => {
                self.state = AppState::Running;
                self.run_dump();
            }
            PacketView::Icmp => {
                self.state = AppState::Running;
                self.run_icmp();
            }
            PacketView::Traceroute => {
                self.state = AppState::Running;
                self.run_traceroute();
            }
            PacketView::Interfaces => {
                self.state = AppState::Running;
                self.run_interfaces();
            }
        }
    }

    fn update_results_view(&mut self) {
        self.results_view.clear();
        let lines: Vec<Line> = self.results.iter().map(|s| Line::from(s.clone())).collect();
        for line in lines {
            self.results_view.add_line(line);
        }
    }

    pub fn can_run(&mut self) -> bool {
        if self.privileges_required && !self.is_root {
            self.results_view.clear();
            self.results_view.add_line(Line::from(vec![
                Span::styled("Error: ", Style::default().fg(tc!(error))),
                Span::raw("Root privileges required for packet operations."),
            ]));
            self.results_view.add_line(Line::from(""));
            self.results_view
                .add_line(Line::from("Run with sudo or as root."));
            return false;
        }
        true
    }
}

impl Default for PacketTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for PacketTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        0.0
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.results.clear();
        self.results_view.clear();
        self.error = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
        if self.inputs.fields.len() > 2 {
            self.inputs.fields[2].value = "100".to_string();
        }
        self.inputs.blur();
        self.current_view = PacketView::Capture;
        self.view_selector.select(0);
        self.view_selector.cancel();
        self.view_selector.blur();
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
    }
}

impl TabRender for PacketTab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let view_name = match self.current_view {
            PacketView::Capture => "Capture",
            PacketView::Send => "Send",
            PacketView::Dump => "Dump",
            PacketView::Icmp => "ICMP Echo",
            PacketView::Traceroute => "Traceroute",
            PacketView::Interfaces => "Interfaces",
        };
        Some(vec!["Packet", view_name])
    }

    fn render(&self, f: &mut Frame, area: Rect, _insert_mode: bool) {
        if let Some(ref err) = self.error {
            use ratatui::widgets::{Block, Borders, Paragraph};
            let error_text = Paragraph::new(format!("Error: {}", err.message()))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Packet - Error"),
                )
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(15),
                Constraint::Min(0),
            ])
            .split(area);

        let selector_area = chunks.first().copied().unwrap_or(area);
        let input_area = chunks.get(1).copied().unwrap_or(area);
        let results_area = chunks.get(2).copied().unwrap_or(area);

        self.view_selector.render(f, selector_area);

        let config_block = Block::default()
            .borders(Borders::ALL)
            .title(" Configuration ")
            .border_style(Style::default().fg(
                if self.view_selector.is_focused() || self.inputs.is_focused() {
                    tc!(border_focused)
                } else {
                    tc!(border)
                },
            ));
        let config_inner = config_block.inner(input_area);
        f.render_widget(config_block, input_area);

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(config_inner);

        for (i, field) in self.inputs.fields.iter().enumerate() {
            if let Some(chunk) = input_chunks.get(i) {
                field.render(f, *chunk, false);
            }
        }

        if !self.is_root {
            let warning = Paragraph::new("Warning: Root privileges required for packet operations")
                .style(Style::default().fg(tc!(warning)));
            f.render_widget(
                warning,
                Rect {
                    x: config_inner.x,
                    y: config_inner.y + 12,
                    width: config_inner.width,
                    height: 1,
                },
            );
        }

        if !self.results_view.is_empty() {
            self.results_view.render(f, results_area, None);
        } else {
            let placeholder = empty_state_paragraph(
                "Results",
                "Select a tool, enter parameters, and press Enter to run",
            );
            f.render_widget(placeholder, results_area);
        }
    }

    fn render_overlays(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(15),
                Constraint::Min(0),
            ])
            .split(area);
        let selector_area = chunks.first().copied().unwrap_or(area);
        if let Some(dropdown) = self.view_selector.dropdown_info(selector_area) {
            dropdown.render(f);
        }
    }
}

impl TabInput for PacketTab {
    fn stop(&mut self) {
        if self.state == AppState::Running {
            self.state = AppState::Idle;
        }
    }

    fn handle_focus_next(&mut self) {
        if !self.is_running() {
            if self.view_selector.is_focused() {
                self.view_selector.blur();
                self.inputs.focus_next();
            } else if self.inputs.is_focused() {
                self.inputs.focus_next();
                if self.inputs.is_focused() {
                    self.inputs.blur();
                    self.view_selector.focus();
                }
            } else {
                self.view_selector.focus();
            }
        }
    }

    fn handle_focus_prev(&mut self) {
        if !self.is_running() {
            if self.view_selector.is_focused() {
                self.view_selector.blur();
                self.inputs.focus_prev();
            } else if self.inputs.is_focused() {
                self.inputs.blur();
                self.view_selector.focus();
            } else {
                self.view_selector.focus();
            }
        }
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() {
            if self.view_selector.is_focused() {
                self.view_selector.handle_char(c);
            } else {
                self.inputs.insert(c);
            }
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() {
            if self.view_selector.is_focused() {
                self.view_selector.handle_backspace();
            } else {
                self.inputs.backspace();
            }
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && !self.view_selector.is_focused() {
            self.inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.is_running() {
            return None;
        }
        if !self.view_selector.is_focused() && self.inputs.is_focused() {
            self.inputs.get_focused_value()
        } else {
            Some(self.results_view.get_content())
        }
    }

    fn handle_word_forward(&mut self) {
        if self.is_running() {
            return;
        }
        if self.inputs.is_focused() {
            self.inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if self.is_running() {
            return;
        }
        if self.inputs.is_focused() {
            self.inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if self.is_running() {
            return;
        }
        if self.inputs.is_focused() {
            self.inputs.move_home();
        } else if !self.results_view.is_empty() {
            self.results_view.scroll_to_top();
        }
    }

    fn handle_end(&mut self) {
        if self.is_running() {
            return;
        }
        if self.inputs.is_focused() {
            self.inputs.move_end();
        } else if !self.results_view.is_empty() {
            self.results_view.scroll_to_bottom();
        }
    }

    fn handle_top(&mut self) {
        if self.is_running() {
            return;
        }
        self.inputs.blur();
        self.view_selector.focus();
    }

    fn handle_bottom(&mut self) {
        if self.is_running() {
            return;
        }
        self.view_selector.blur();
        self.inputs.blur();
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            return;
        }
        if self.view_selector.is_focused() {
            if self.view_selector.is_open() {
                if self.view_selector.confirm().is_none() {
                    tracing::warn!("Failed to confirm packet view selector");
                }
                self.current_view = match self.view_selector.selected {
                    0 => PacketView::Capture,
                    1 => PacketView::Send,
                    2 => PacketView::Dump,
                    3 => PacketView::Icmp,
                    4 => PacketView::Traceroute,
                    5 => PacketView::Interfaces,
                    _ => PacketView::Capture,
                };
            } else {
                self.view_selector.open();
            }
        } else if self.inputs.is_focused() {
            self.inputs.blur();
        } else {
            self.execute();
        }
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        if self.view_selector.is_open() {
            self.view_selector.cancel();
            return;
        }
        if self.view_selector.is_focused() {
            self.view_selector.blur();
        }
        self.inputs.blur();
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        if self.view_selector.is_focused() {
            if self.view_selector.is_open() {
                self.view_selector.move_prev();
            }
        } else if !self.inputs.is_focused() {
            self.results_view.scroll_up(1);
        } else {
            self.inputs.focus_prev();
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        if self.view_selector.is_focused() {
            if self.view_selector.is_open() {
                self.view_selector.move_next();
            }
        } else if !self.inputs.is_focused() {
            self.results_view.scroll_down(1);
        } else {
            self.inputs.focus_next();
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.view_selector.is_focused() {
            if self.view_selector.is_open() {
                self.view_selector.move_prev();
                true
            } else {
                false
            }
        } else {
            self.inputs.move_left()
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.view_selector.is_focused() {
            if self.view_selector.is_open() {
                self.view_selector.move_next();
                true
            } else {
                false
            }
        } else {
            self.inputs.move_right()
        }
    }

    fn is_input_focused(&self) -> bool {
        self.view_selector.is_focused() || self.inputs.is_focused()
    }

    fn is_at_left_edge(&self) -> bool {
        if self.view_selector.is_focused() {
            if self.view_selector.is_open() {
                self.view_selector.items.is_empty() || self.view_selector.selected == 0
            } else {
                true
            }
        } else if self.inputs.is_focused() {
            self.inputs.is_at_left_edge()
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.view_selector.is_focused() {
            if self.view_selector.is_open() {
                self.view_selector.items.is_empty()
                    || self.view_selector.selected
                        >= self.view_selector.items.len().saturating_sub(1)
            } else {
                true
            }
        } else if self.inputs.is_focused() {
            self.inputs.is_at_right_edge()
        } else {
            true
        }
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

    fn primary_target(&self) -> Option<String> {
        Some(self.target().to_string())
    }
}
