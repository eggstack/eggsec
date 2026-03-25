use crate::tui::components::ScrollableText;
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::style::Color;
use ratatui::text::{Line, Span};
use ratatui::Frame;

pub struct DashboardTab {
    pub view: ScrollableText,
    pub state: AppState,
    pub total_scans: usize,
    pub successful_scans: usize,
    pub failed_scans: usize,
    pub last_scan_type: String,
    pub last_target: String,
}

impl DashboardTab {
    pub fn new() -> Self {
        let mut tab = Self {
            view: ScrollableText::new("Dashboard"),
            state: AppState::Idle,
            total_scans: 0,
            successful_scans: 0,
            failed_scans: 0,
            last_scan_type: String::new(),
            last_target: String::new(),
        };
        tab.render_welcome();
        tab
    }

    fn render_welcome(&mut self) {
        self.view.clear();
        self.view.add_line(Line::from(Span::styled(
            "Security Assessment Dashboard",
            ratatui::style::Style::default()
                .fg(Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )));
        self.view.add_line(Line::from(""));
        self.view
            .add_line(Line::from("Run scans in any tab to see results here."));
        self.view.add_line(Line::from(""));
        self.view.add_line(Line::from("Available Scans:"));
        self.view
            .add_line(Line::from("  [1] Recon      - Domain/IP reconnaissance"));
        self.view
            .add_line(Line::from("  [2] Load       - HTTP load testing"));
        self.view
            .add_line(Line::from("  [3] Scan Ports - Port scanning"));
        self.view
            .add_line(Line::from("  [4] Endpoints  - Endpoint discovery"));
        self.view
            .add_line(Line::from("  [5] Fingerprint- Service fingerprinting"));
        self.view
            .add_line(Line::from("  [6] Fuzz       - Security fuzzing"));
        self.view
            .add_line(Line::from("  [7] WAF        - WAF detection/bypass"));
        self.view
            .add_line(Line::from("  [8] WAF Stress - WAF stress testing"));
        self.view
            .add_line(Line::from("  [9] Scan       - Pipeline scan"));
        self.view.add_line(Line::from(""));
        self.view.add_line(Line::styled(
            "Additional Tabs:",
            ratatui::style::Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
        self.view
            .add_line(Line::from("  [10] Proxy      - Proxy management"));
        self.view.add_line(Line::from(
            "  [11] Packet     - Network tools (ICMP, Traceroute)",
        ));
        self.view
            .add_line(Line::from("  [12] GraphQL    - GraphQL security testing"));
        self.view.add_line(Line::from(
            "  [13] OAuth      - OAuth/OIDC security testing",
        ));
        self.view
            .add_line(Line::from("  [14] Cluster    - Distributed scanning"));
        self.view
            .add_line(Line::from("  [15] Stress     - Stress/load testing"));
        self.view
            .add_line(Line::from("  [16] Report     - Report conversion & trends"));
        self.view.add_line(Line::from(""));
        self.view.add_line(Line::styled(
            "Available Services:",
            ratatui::style::Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
        self.view.add_line(Line::from(
            "  REST API Server : Use CLI 'slapper serve' to start",
        ));
        self.view.add_line(Line::from(
            "  MCP Server      : Use CLI 'slapper mcp-serve' to start",
        ));
        self.view.add_line(Line::from(
            "  Cluster         : Use CLI 'slapper cluster' to manage",
        ));
        self.view.add_line(Line::from(""));
        self.view.add_line(Line::from("Keybindings:"));
        self.view
            .add_line(Line::from("  h/l or n/p   - Previous/Next tab"));
        self.view
            .add_line(Line::from("  j/k          - Scroll up/down"));
        self.view
            .add_line(Line::from("  Enter        - Start scan"));
        self.view
            .add_line(Line::from("  e            - Export results"));
        self.view
            .add_line(Line::from("  Space        - Toggle help"));
    }

    pub fn update_from_history(&mut self, history: &[crate::tui::tabs::history::HistoryEntry]) {
        self.total_scans = history.len();
        self.successful_scans = history
            .iter()
            .filter(|e| {
                e.summary.to_lowercase().contains("complete")
                    || e.summary.to_lowercase().contains("found")
            })
            .count();
        self.failed_scans = history
            .iter()
            .filter(|e| {
                e.summary.to_lowercase().contains("failed")
                    || e.summary.to_lowercase().contains("error")
            })
            .count();

        if let Some(last) = history.first() {
            self.last_scan_type = last.scan_type.clone();
            self.last_target = last.target.clone();
        }

        self.render_stats();
    }

    fn render_stats(&mut self) {
        self.view.clear();

        self.view.add_line(Line::from(Span::styled(
            "Security Assessment Dashboard",
            ratatui::style::Style::default()
                .fg(Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )));
        self.view.add_line(Line::from(""));

        self.view.add_line(Line::from(Span::styled(
            "Session Statistics",
            ratatui::style::Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )));
        self.view.add_line(Line::from(""));

        let total_str = format!("  Total Scans:      {}", self.total_scans);
        let success_str = format!("  Successful:       {}", self.successful_scans);
        let failed_str = format!("  Failed:           {}", self.failed_scans);

        self.view.add_line(Line::from(total_str));
        self.view.add_line(Line::from(success_str));
        self.view.add_line(Line::from(failed_str));

        if self.total_scans > 0 {
            let success_rate =
                (self.successful_scans as f64 / self.total_scans as f64 * 100.0) as usize;
            self.view
                .add_line(Line::from(format!("  Success Rate:    {}%", success_rate)));
        }

        self.view.add_line(Line::from(""));

        if !self.last_scan_type.is_empty() {
            self.view.add_line(Line::from(Span::styled(
                "Last Scan",
                ratatui::style::Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )));
            self.view.add_line(Line::from(""));
            self.view
                .add_line(Line::from(format!("  Type:   {}", self.last_scan_type)));
            self.view
                .add_line(Line::from(format!("  Target: {}", self.last_target)));
            self.view.add_line(Line::from(""));
        }

        self.view.add_line(Line::from(Span::styled(
            "Quick Actions",
            ratatui::style::Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )));
        self.view.add_line(Line::from(""));
        self.view.add_line(Line::from("  [h/l]     Navigate tabs"));
        self.view.add_line(Line::from("  [Enter]   Start new scan"));
        self.view.add_line(Line::from("  [e]       Export results"));
        self.view.add_line(Line::from("  [Space]   View help"));
    }
}

impl Default for DashboardTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for DashboardTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        0.0
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
    }
}

impl TabRender for DashboardTab {
    fn render(&self, f: &mut Frame, area: ratatui::layout::Rect, _insert_mode: bool) {
        self.view.render(f, area);
    }
}

impl TabInput for DashboardTab {
    fn handle_focus_next(&mut self) {}
    fn handle_focus_prev(&mut self) {}
    fn handle_char(&mut self, _c: char) {}
    fn handle_backspace(&mut self) {}
    fn handle_enter(&mut self) {}
    fn handle_escape(&mut self) {}
    fn handle_up(&mut self) {
        self.view.scroll_up(1);
    }
    fn handle_down(&mut self) {
        self.view.scroll_down(1);
    }
    fn handle_left(&mut self) -> bool {
        false
    }
    fn handle_right(&mut self) -> bool {
        false
    }
    fn handle_home(&mut self) {
        self.view.scroll_to_top();
    }
    fn handle_end(&mut self) {
        self.view.scroll_to_bottom();
    }
    fn handle_top(&mut self) {
        self.view.scroll_to_top();
    }
    fn handle_bottom(&mut self) {
        self.view.scroll_to_bottom();
    }
    fn handle_word_forward(&mut self) {
        for _ in 0..5 {
            self.view.scroll_right(1);
        }
    }
    fn handle_word_backward(&mut self) {
        for _ in 0..5 {
            self.view.scroll_left(1);
        }
    }
    fn is_input_focused(&self) -> bool {
        false
    }
}

impl DashboardTab {
    pub fn page_up(&mut self, count: usize) {
        self.view.scroll_up(count);
    }

    pub fn page_down(&mut self, count: usize) {
        self.view.scroll_down(count);
    }
}
