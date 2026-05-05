use crate::tc;
use crate::tui::components::ScrollableText;
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use chrono::{DateTime, Utc};
use ratatui::text::{Line, Span};
use ratatui::Frame;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize)]
struct PortfolioSnapshot {
    unique_targets: usize,
    total_scans: usize,
    #[allow(dead_code)]
    scans_today: usize,
    findings_by_severity: HashMap<String, usize>,
    findings_trend: Vec<(String, usize)>,
    critical_findings: usize,
    health_score: f64,
    #[allow(dead_code)]
    last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardFocusArea {
    Main,
}

pub struct DashboardTab {
    pub view: ScrollableText,
    pub state: AppState,
    pub focus_area: DashboardFocusArea,
    pub total_scans: usize,
    pub successful_scans: usize,
    pub failed_scans: usize,
    pub last_scan_type: String,
    pub last_target: String,
    pub sparkline_data: Vec<usize>,
    pub unique_targets: usize,
    pub critical_findings: usize,
    pub today_scans: usize,
    pub error_message: Option<String>,
}

impl DashboardTab {
    pub fn new() -> Self {
        let mut tab = Self {
            view: ScrollableText::new("Dashboard"),
            state: AppState::Idle,
            focus_area: DashboardFocusArea::Main,
            total_scans: 0,
            successful_scans: 0,
            failed_scans: 0,
            last_scan_type: String::new(),
            last_target: String::new(),
            sparkline_data: Vec::new(),
            unique_targets: 0,
            critical_findings: 0,
            today_scans: 0,
            error_message: None,
        };
        tab.render_welcome();
        tab
    }

    fn render_sparkline(data: &[usize]) -> String {
        if data.is_empty() {
            return String::from("[no data]");
        }

        let min_val = *data.iter().min().unwrap_or(&0);
        let max_val = *data.iter().max().unwrap_or(&0);
        let range = if max_val > min_val {
            max_val - min_val
        } else {
            1
        };

        let blocks = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        let bucket_count = blocks.len() - 1;

        let sparkline: String = data
            .iter()
            .map(|&v| {
                let normalized = ((v - min_val) * bucket_count) / range;
                let idx = normalized.min(bucket_count);
                blocks[idx]
            })
            .collect();

        format!(" {}", sparkline)
    }

    fn load_portfolio_snapshot(path: &std::path::Path) -> Option<PortfolioSnapshot> {
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn render_welcome(&mut self) {
        self.view.clear();
        self.view.add_line(Line::from(Span::styled(
            "Security Assessment Dashboard",
            ratatui::style::Style::default()
                .fg(tc!(info))
                .add_modifier(ratatui::style::Modifier::BOLD),
        )));
        self.view.add_line(Line::from(""));
        self.view
            .add_line(Line::from("Run scans in any tab to see results here."));
        self.view.add_line(Line::from(""));
        self.view.add_line(Line::from("Available Scans:"));
        self.view
            .add_line(Line::from("  Recon      - Domain/IP reconnaissance"));
        self.view
            .add_line(Line::from("  Load       - HTTP load testing"));
        self.view
            .add_line(Line::from("  Scan Ports - Port scanning"));
        self.view
            .add_line(Line::from("  Endpoints  - Endpoint discovery"));
        self.view
            .add_line(Line::from("  Fingerprint- Service fingerprinting"));
        self.view
            .add_line(Line::from("  Fuzz       - Security fuzzing"));
        self.view
            .add_line(Line::from("  WAF        - WAF detection/bypass"));
        self.view
            .add_line(Line::from("  WAF Stress - WAF stress testing"));
        self.view
            .add_line(Line::from("  Scan       - Pipeline scan"));
        self.view.add_line(Line::from(""));
        self.view.add_line(Line::styled(
            "Additional Tabs:",
            ratatui::style::Style::default()
                .fg(tc!(accent))
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
        self.view
            .add_line(Line::from("  Proxy      - Proxy management"));
        self.view.add_line(Line::from(
            "  Packet     - Network tools (ICMP, Traceroute)",
        ));
        self.view
            .add_line(Line::from("  GraphQL    - GraphQL security testing"));
        self.view
            .add_line(Line::from("  OAuth      - OAuth/OIDC security testing"));
        self.view
            .add_line(Line::from("  Cluster    - Distributed scanning"));
        self.view
            .add_line(Line::from("  Stress     - Stress/load testing"));
        self.view
            .add_line(Line::from("  Report     - Report conversion & trends"));
        self.view.add_line(Line::from(""));
        self.view.add_line(Line::styled(
            "Available Services:",
            ratatui::style::Style::default()
                .fg(tc!(accent))
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
            .add_line(Line::from("  n/p or Shift+H/L - Previous/Next tab"));
        self.view
            .add_line(Line::from("  Ctrl+X       - Quick switch tab"));
        self.view
            .add_line(Line::from("  j/k          - Scroll up/down / Navigate"));
        self.view
            .add_line(Line::from("  Enter        - Select / Start scan"));
        self.view
            .add_line(Line::from("  e            - Export results"));
        self.view
            .add_line(Line::from("  Space        - Toggle help"));
    }

    pub fn update_from_history(&mut self, history: &[crate::tui::tabs::history::HistoryEntry]) {
        use std::collections::HashSet;

        // Reset counters before recalculating
        self.today_scans = 0;

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

        let last_n = 7.min(history.len());
        self.sparkline_data = history
            .iter()
            .take(last_n)
            .map(|e| Self::extract_activity_score(&e.summary))
            .collect();

        let mut targets: HashSet<String> = HashSet::new();
        let mut critical_count = 0;
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();

        for entry in history.iter() {
            targets.insert(entry.target.clone());

            if entry.timestamp.starts_with(&today) {
                self.today_scans += 1;
            }

            let summary_lower = entry.summary.to_lowercase();
            if summary_lower.contains("critical") || summary_lower.contains("critical findings") {
                critical_count += 1;
            }
        }

        self.unique_targets = targets.len();
        self.critical_findings = critical_count;

        self.render_stats();
    }

    fn extract_activity_score(summary: &str) -> usize {
        let numbers: Vec<usize> = summary
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();

        if numbers.is_empty() {
            return 1;
        }

        let sum: usize = numbers.iter().sum();
        let count = numbers.len();

        let base_score = if count > 0 { sum / count } else { 1 };

        base_score.clamp(1, 100)
    }

    fn render_stats(&mut self) {
        self.view.clear();

        self.view.add_line(Line::from(Span::styled(
            "Security Assessment Dashboard",
            ratatui::style::Style::default()
                .fg(tc!(info))
                .add_modifier(ratatui::style::Modifier::BOLD),
        )));
        self.view.add_line(Line::from(""));

        self.view.add_line(Line::from(Span::styled(
            "Session Statistics",
            ratatui::style::Style::default()
                .fg(tc!(accent))
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

        if !self.sparkline_data.is_empty() {
            self.view.add_line(Line::from(Span::styled(
                "Activity Trend (last 7 scans)",
                ratatui::style::Style::default()
                    .fg(tc!(success))
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )));
            self.view.add_line(Line::from(""));
            let sparkline = Self::render_sparkline(&self.sparkline_data);
            self.view.add_line(Line::from(sparkline));
            self.view.add_line(Line::from(""));
        }

        self.view.add_line(Line::from(Span::styled(
            "Asset Health Summary",
            ratatui::style::Style::default()
                .fg(tc!(success))
                .add_modifier(ratatui::style::Modifier::BOLD),
        )));
        self.view.add_line(Line::from(""));

        let snapshot = Self::load_portfolio_snapshot(
            &directories::ProjectDirs::from("com", "slapper", "slapper")
                .map(|d| d.config_dir().to_path_buf())
                .unwrap_or_default()
                .join("memory")
                .join("portfolio_snapshot.json"),
        );

        if let Some(snap) = snapshot {
            let health_pct = (snap.health_score * 100.0) as usize;
            self.view
                .add_line(Line::from(format!("  Portfolio Health: {}%", health_pct)));
            self.view.add_line(Line::from(format!(
                "  Total Scans:      {}",
                snap.total_scans
            )));
            self.view.add_line(Line::from(format!(
                "  Unique Targets:   {}",
                snap.unique_targets
            )));
            self.view.add_line(Line::from(format!(
                "  Critical Issues:  {}",
                snap.critical_findings
            )));

            let total_findings: usize = snap.findings_by_severity.values().sum();
            self.view.add_line(Line::from(format!(
                "  Total Findings:   {}",
                total_findings
            )));

            let health_status = if snap.critical_findings > 0 {
                "Needs Attention"
            } else if snap.unique_targets > 0 {
                "Healthy"
            } else {
                "No data"
            };
            self.view
                .add_line(Line::from(format!("  Status:           {}", health_status)));

            if !snap.findings_trend.is_empty() {
                self.view.add_line(Line::from(""));
                if let Some((_, last_count)) = snap.findings_trend.last() {
                    if let Some((_, prev_count)) = snap
                        .findings_trend
                        .get(snap.findings_trend.len().saturating_sub(2))
                    {
                        let diff = *last_count as i64 - *prev_count as i64;
                        let trend_icon = if diff > 0 {
                            "↑"
                        } else if diff < 0 {
                            "↓"
                        } else {
                            "→"
                        };
                        self.view.add_line(Line::from(format!(
                            "  Monthly Trend:    {} ({}{})",
                            trend_icon,
                            if diff > 0 { "+" } else { "" },
                            diff
                        )));
                    }
                }
            }
        } else {
            self.view.add_line(Line::from(format!(
                "  Unique Targets:  {}",
                self.unique_targets
            )));
            self.view.add_line(Line::from(format!(
                "  Scans Today:     {}",
                self.today_scans
            )));
            self.view.add_line(Line::from(format!(
                "  Critical Issues: {}",
                self.critical_findings
            )));

            let health_status = if self.critical_findings == 0 && self.unique_targets > 0 {
                "Healthy"
            } else if self.critical_findings > 0 {
                "Needs Attention"
            } else {
                "No data"
            };
            self.view
                .add_line(Line::from(format!("  Status:          {}", health_status)));
            self.view
                .add_line(Line::from("  (Session-only data - Agent not running)"));
        }
        self.view.add_line(Line::from(""));

        if !self.last_scan_type.is_empty() {
            self.view.add_line(Line::from(Span::styled(
                "Last Scan",
                ratatui::style::Style::default()
                    .fg(tc!(accent))
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
                .fg(tc!(accent))
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
        self.error_message = None;
        self.total_scans = 0;
        self.successful_scans = 0;
        self.failed_scans = 0;
        self.today_scans = 0;
        self.unique_targets = 0;
        self.critical_findings = 0;
        self.last_scan_type.clear();
        self.last_target.clear();
        self.sparkline_data.clear();
        self.render_welcome();
    }

    fn set_error(&mut self, msg: String) {
        self.state = AppState::Error(msg.clone());
        self.error_message = Some(msg);
    }
}

impl TabRender for DashboardTab {
    fn render(&self, f: &mut Frame, area: ratatui::layout::Rect, _insert_mode: bool) {
        if let Some(ref err_msg) = self.error_message {
            use ratatui::style::Style;
            use ratatui::widgets::{Block, Borders, Paragraph};
            let error_text = Paragraph::new(format!("Error: {}", err_msg))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Dashboard - Error"),
                )
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, area);
        } else {
            self.view.render(f, area, None);
        }
    }
}

impl TabInput for DashboardTab {
    fn handle_focus_next(&mut self) {}
    fn handle_focus_prev(&mut self) {}
    fn handle_char(&mut self, _c: char) {}
    fn handle_backspace(&mut self) {}

    fn handle_paste(&mut self, _text: &str) {}

    fn handle_copy(&mut self) -> Option<String> {
        Some(self.view.get_content())
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

    fn is_at_left_edge(&self) -> bool {
        self.view.is_at_left_edge()
    }

    fn is_at_right_edge(&self) -> bool {
        self.view.is_at_right_edge()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::tabs::history::HistoryEntry;

    fn create_test_entry(
        timestamp: &str,
        summary: &str,
        scan_type: &str,
        target: &str,
    ) -> HistoryEntry {
        static mut NEXT_ID: usize = 0;
        let id = unsafe {
            NEXT_ID += 1;
            NEXT_ID
        };
        HistoryEntry {
            id,
            timestamp: timestamp.to_string(),
            scan_type: scan_type.to_string(),
            target: target.to_string(),
            summary: summary.to_string(),
            details: Vec::new(),
        }
    }

    #[test]
    fn test_update_from_history_idempotent_today_scans() {
        let mut dashboard = DashboardTab::new();

        // Create entries with today's date
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let entries = vec![
            create_test_entry(&today, "Scan completed", "recon", "example.com"),
            create_test_entry(&today, "Scan completed", "fuzz", "example.com"),
        ];

        // Call update_from_history twice
        dashboard.update_from_history(&entries);
        let first_count = dashboard.today_scans;

        dashboard.update_from_history(&entries);
        let second_count = dashboard.today_scans;

        assert_eq!(
            first_count, second_count,
            "today_scans should be idempotent"
        );
        assert_eq!(second_count, 2, "Should have 2 today scans");
    }

    #[test]
    fn test_reset_clears_stats() {
        let mut dashboard = DashboardTab::new();

        // Simulate some state
        dashboard.total_scans = 10;
        dashboard.successful_scans = 8;
        dashboard.failed_scans = 2;
        dashboard.today_scans = 5;
        dashboard.unique_targets = 3;
        dashboard.critical_findings = 1;

        // Reset
        dashboard.reset();

        // Verify stats are reset
        assert_eq!(dashboard.total_scans, 0);
        assert_eq!(dashboard.successful_scans, 0);
        assert_eq!(dashboard.failed_scans, 0);
        assert_eq!(dashboard.today_scans, 0);
        assert_eq!(dashboard.unique_targets, 0);
        assert_eq!(dashboard.critical_findings, 0);

        // Verify state is Idle
        match dashboard.state {
            crate::tui::tabs::AppState::Idle => (),
            _ => panic!("State should be Idle after reset"),
        }
    }
}
