use crate::config::ProxyConfigEntry;
use crate::proxy::{HealthCheckConfig, HealthChecker, ProxyEntry, ProxyType};
use crate::tc;
use crate::tui::app::tab_error::TabError;
use crate::tui::components::{
    empty_state_paragraph, InputField, InputGroup, ScrollableText, Selector,
};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use crate::types::SensitiveString;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

#[derive(Clone, Copy, PartialEq)]
pub enum ProxyView {
    List,
    Add,
    HealthCheck,
    Test,
}

pub struct ProxyTab {
    pub view_selector: Selector,
    pub inputs: InputGroup,
    pub proxies: Vec<ProxyConfigEntry>,
    pub health_results: Vec<ProxyHealthResult>,
    pub test_result: Option<ProxyTestResult>,
    pub current_view: ProxyView,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub error: Option<TabError>,
}

#[derive(Clone)]
pub struct ProxyHealthResult {
    pub url: String,
    pub is_healthy: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct ProxyTestResult {
    pub url: String,
    pub is_healthy: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

impl ProxyTab {
    pub fn new() -> Self {
        let view_selector =
            Selector::new("View").simple_items(vec!["List", "Add", "Health Check", "Test"]);

        let inputs = InputGroup::new().add(InputField::new("Proxy File Path"));

        Self {
            view_selector,
            inputs,
            proxies: Vec::new(),
            health_results: Vec::new(),
            test_result: None,
            current_view: ProxyView::List,
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            error: None,
        }
    }

    pub fn load_proxies(&mut self, proxies: Vec<ProxyConfigEntry>) {
        self.proxies = proxies;
    }

    pub fn set_health_results(&mut self, results: Vec<ProxyHealthResult>) {
        self.health_results = results;
        self.update_health_view();
    }

    pub fn set_test_result(&mut self, result: ProxyTestResult) {
        self.test_result = Some(result);
        self.update_test_view();
    }

    fn update_health_view(&mut self) {
        self.results_view.clear();
        self.results_view.add_line(Line::from(vec![Span::styled(
            "Proxy Health Check Results",
            Style::default().fg(tc!(accent)),
        )]));
        self.results_view.add_line(Line::from(""));

        let total = self.health_results.len();
        let healthy = self.health_results.iter().filter(|r| r.is_healthy).count();
        let unhealthy = total - healthy;

        self.results_view
            .add_line(Line::from(vec![Span::raw(format!(
                "Total: {} | Healthy: {} | Unhealthy: {}",
                total, healthy, unhealthy
            ))]));
        self.results_view.add_line(Line::from(""));

        for result in &self.health_results {
            let status = if result.is_healthy { "✓" } else { "✗" };
            let latency = result
                .latency_ms
                .map(|ms| format!("{}ms", ms))
                .unwrap_or_else(|| "N/A".to_string());
            let error = result.error.as_deref().unwrap_or("OK");
            self.results_view.add_line(Line::from(vec![
                Span::styled(
                    format!("[{}] ", status),
                    if result.is_healthy {
                        Style::default().fg(tc!(success))
                    } else {
                        Style::default().fg(tc!(error))
                    },
                ),
                Span::raw(format!("{} - {} ({})", result.url, latency, error)),
            ]));
        }
    }

    fn update_test_view(&mut self) {
        self.results_view.clear();
        if let Some(ref result) = self.test_result {
            self.results_view.add_line(Line::from(vec![Span::styled(
                "Proxy Test Result",
                Style::default().fg(tc!(accent)),
            )]));
            self.results_view.add_line(Line::from(""));

            let status = if result.is_healthy {
                "✓ Healthy"
            } else {
                "✗ Failed"
            };
            let latency = result
                .latency_ms
                .map(|ms| format!("{}ms", ms))
                .unwrap_or_else(|| "N/A".to_string());
            let error = result.error.as_deref().unwrap_or("OK");

            self.results_view
                .add_line(Line::from(format!("Proxy: {}", result.url)));
            self.results_view.add_line(Line::from(vec![
                Span::styled(
                    status,
                    if result.is_healthy {
                        Style::default().fg(tc!(success))
                    } else {
                        Style::default().fg(tc!(error))
                    },
                ),
                Span::raw(format!(" (latency: {})", latency)),
            ]));
            if !result.is_healthy {
                let error_msg = error.to_string();
                self.results_view.add_line(Line::from(vec![
                    Span::styled("Error: ", Style::default().fg(tc!(error))),
                    Span::raw(error_msg),
                ]));
            }
        }
    }

    pub fn update_list_view(&mut self) {
        self.results_view.clear();
        if self.proxies.is_empty() {
            self.results_view.add_line(Line::from("No proxies loaded."));
            self.results_view.add_line(Line::from(""));
            self.results_view
                .add_line(Line::from("Run 'Add' to load proxies from file."));
        } else {
            self.results_view.add_line(Line::from(vec![Span::styled(
                format!("Proxy Pool ({} proxies)", self.proxies.len()),
                Style::default().fg(tc!(accent)),
            )]));
            self.results_view.add_line(Line::from(""));

            for (i, proxy) in self.proxies.iter().enumerate() {
                let status = if proxy.enabled { "enabled" } else { "disabled" };
                self.results_view.add_line(Line::from(vec![
                    Span::styled(format!("[{}] ", i + 1), Style::default().fg(tc!(info))),
                    Span::raw(format!(
                        "{}://{}:{} - {}",
                        proxy.proxy_type, proxy.address, proxy.port, status
                    )),
                ]));
            }
        }
    }

    pub fn proxy_file_path(&self) -> &str {
        self.inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn start_health_check(&mut self) {
        self.state = AppState::Running;
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

    pub fn load_proxies_from_file(&mut self, path: &str) -> Result<usize, String> {
        let entries = ProxyEntry::load_from_file(path)
            .map_err(|e| format!("Failed to load proxies: {}", e))?;

        self.proxies = entries
            .iter()
            .map(|p| ProxyConfigEntry {
                proxy_type: p.proxy_type,
                address: p.address.clone(),
                port: p.port,
                username: p.username.clone(),
                password: p.password.clone(),
                local_addr: None,
                weight: Some(p.weight),
                priority: Some(p.priority as u32),
                enabled: p.enabled,
            })
            .collect();

        self.update_list_view();
        Ok(self.proxies.len())
    }

    pub async fn run_health_check(&mut self, test_url: &str) {
        self.state = AppState::Running;
        self.results_view.clear();
        self.results_view
            .add_line(Line::from("Running health checks..."));

        let proxy_entries: Vec<ProxyEntry> = self
            .proxies
            .iter()
            .map(|p| ProxyEntry {
                name: None,
                proxy_type: p.proxy_type,
                address: p.address.clone(),
                port: p.port,
                username: p.username.clone(),
                password: p.password.clone(),
                weight: p.weight.unwrap_or(1),
                priority: p.priority.unwrap_or(1) as u8,
                timeout_ms: 10000,
                enabled: p.enabled,
                tags: Vec::new(),
            })
            .collect();

        let health_config = HealthCheckConfig {
            enabled: true,
            interval_secs: 60,
            timeout_ms: 10000,
            test_url: test_url.to_string(),
            max_failures: 3,
        };

        let checker = match HealthChecker::new(health_config) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to create health checker: {}", e);
                self.state = AppState::Idle;
                return;
            }
        };

        match checker.check_all(&proxy_entries).await {
            Ok(results) => {
                self.health_results = results
                    .results
                    .iter()
                    .map(|r| ProxyHealthResult {
                        url: r.proxy_url.clone(),
                        is_healthy: r.is_healthy,
                        latency_ms: r.latency_ms,
                        error: r.error.clone(),
                    })
                    .collect();
                self.update_health_view();
            }
            Err(e) => {
                self.results_view.add_line(Line::from(vec![
                    Span::styled("Error: ", Style::default().fg(tc!(error))),
                    Span::raw(e.to_string()),
                ]));
            }
        }

        self.state = AppState::Completed;
    }

    pub async fn run_test(&mut self, proxy_url: &str, test_url: &str) {
        self.state = AppState::Running;
        self.results_view.clear();
        self.results_view
            .add_line(Line::from(format!("Testing proxy: {}", proxy_url)));

        let (address, port, username, password) = match parse_proxy_url(proxy_url) {
            Ok((a, p, u, pw)) => (a, p, u, pw),
            Err(e) => {
                self.results_view.add_line(Line::from(vec![
                    Span::styled("Error: ", Style::default().fg(tc!(error))),
                    Span::raw(e.to_string()),
                ]));
                self.state = AppState::Error(e.to_string());
                return;
            }
        };

        let proxy_type = if proxy_url.starts_with("socks5://") || proxy_url.starts_with("socks://")
        {
            ProxyType::Socks5
        } else if proxy_url.starts_with("socks4://") {
            ProxyType::Socks4
        } else if proxy_url.starts_with("https://") {
            ProxyType::Https
        } else {
            ProxyType::Http
        };

        let proxy_entry = ProxyEntry {
            name: None,
            proxy_type,
            address,
            port,
            username,
            password: password.map(SensitiveString::new),
            weight: 1,
            priority: 0,
            timeout_ms: 10000,
            enabled: true,
            tags: Vec::new(),
        };

        let health_config = HealthCheckConfig {
            enabled: true,
            interval_secs: 60,
            timeout_ms: 10000,
            test_url: test_url.to_string(),
            max_failures: 3,
        };

        let checker = match HealthChecker::new(health_config) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to create health checker: {}", e);
                self.state = AppState::Idle;
                return;
            }
        };
        let result = checker.check(&proxy_entry).await;

        self.test_result = Some(ProxyTestResult {
            url: proxy_url.to_string(),
            is_healthy: result.is_healthy,
            latency_ms: result.latency_ms,
            error: result.error,
        });

        self.update_test_view();
        self.state = AppState::Completed;
    }
}

impl Default for ProxyTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for ProxyTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        0.0
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.health_results.clear();
        self.proxies.clear();
        self.test_result = None;
        self.results_view.clear();
        self.error = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
        self.current_view = ProxyView::List;
        self.view_selector.select(0);
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
    }
}

impl TabRender for ProxyTab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let view_name = match self.current_view {
            ProxyView::List => "List",
            ProxyView::Add => "Add",
            ProxyView::HealthCheck => "Health Check",
            ProxyView::Test => "Test",
        };
        Some(vec!["Proxy", view_name])
    }

    fn render(&self, f: &mut Frame, area: Rect, _insert_mode: bool) {
        if let Some(ref err) = self.error {
            use ratatui::widgets::{Block, Borders, Paragraph};
            let error_text = Paragraph::new(format!("Error: {}", err.message()))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Proxy - Error"),
                )
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(area);

        let selector_area = chunks.get(0).copied().unwrap_or(area);
        let input_area = chunks.get(1).copied().unwrap_or(area);
        let results_area = chunks.get(2).copied().unwrap_or(area);

        self.view_selector.render(f, selector_area);
        if let Some(dropdown) = self.view_selector.dropdown_info(selector_area) {
            dropdown.render(f);
        }

        let config_block = Block::default()
            .borders(Borders::ALL)
            .title(" Configuration ")
            .border_style(
                Style::default().fg(
                    if self.view_selector.is_focused() || self.inputs.is_focused() {
                        tc!(border_focused)
                    } else {
                        tc!(border)
                    },
                ),
            );
        let config_inner = config_block.inner(input_area);
        f.render_widget(config_block, input_area);

        if matches!(
            self.current_view,
            ProxyView::Add | ProxyView::HealthCheck | ProxyView::Test
        ) {
            if let Some(field) = self.inputs.fields.first() {
                field.render(f, config_inner, false);
            }
        }

        if !self.results_view.is_empty() {
            self.results_view
                .render(f, results_area, None);
        } else {
            let placeholder = empty_state_paragraph("Results", "Results will appear here");
            f.render_widget(placeholder, results_area);
        }
    }

    fn render_overlays(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(area);
        let selector_area = chunks.get(0).copied().unwrap_or(area);
        if let Some(dropdown) = self.view_selector.dropdown_info(selector_area) {
            dropdown.render(f);
        }
    }
}

impl TabInput for ProxyTab {
    fn handle_focus_next(&mut self) {
        if !self.is_running() {
            if self.view_selector.is_focused() {
                self.view_selector.blur();
                if matches!(
                    self.current_view,
                    ProxyView::Add | ProxyView::HealthCheck | ProxyView::Test
                ) {
                    self.inputs.focus_next();
                }
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
                if matches!(
                    self.current_view,
                    ProxyView::Add | ProxyView::HealthCheck | ProxyView::Test
                ) {
                    self.inputs.focus_prev();
                }
            } else if self.inputs.is_focused() {
                self.inputs.focus_prev();
                if !self.inputs.is_focused() {
                    self.inputs.blur();
                    self.view_selector.focus();
                }
            } else {
                self.view_selector.focus();
            }
        }
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() {
            if self.view_selector.is_focused() {
                self.view_selector.handle_char(c);
            } else if self.inputs.is_focused() {
                self.inputs.insert(c);
            }
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() {
            if self.view_selector.is_focused() {
                self.view_selector.handle_backspace();
            } else if self.inputs.is_focused() {
                self.inputs.backspace();
            }
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && !self.view_selector.is_focused() && self.inputs.is_focused() {
            self.inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if !self.is_running() {
            if self.view_selector.is_focused() {
                None
            } else if self.inputs.is_focused() {
                self.inputs.get_focused_value()
            } else {
                Some(self.results_view.get_content())
            }
        } else {
            None
        }
    }

    fn handle_word_forward(&mut self) {
        if !self.is_running() && self.inputs.is_focused() {
            self.inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() && self.inputs.is_focused() {
            self.inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if !self.is_running() {
            if self.inputs.is_focused() {
                self.inputs.move_home();
            } else if !self.results_view.is_empty() {
                self.results_view.scroll_to_top();
            }
        }
    }

    fn handle_end(&mut self) {
        if !self.is_running() {
            if self.inputs.is_focused() {
                self.inputs.move_end();
            } else if !self.results_view.is_empty() {
                self.results_view.scroll_to_bottom();
            }
        }
    }

    fn handle_top(&mut self) {
        if !self.is_running() {
            self.view_selector.focus();
        }
    }

    fn handle_bottom(&mut self) {
        if !self.is_running() {
            self.view_selector.blur();
            self.inputs.blur();
        }
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            return;
        }
        if self.view_selector.is_focused() {
            self.view_selector.handle_enter();
            self.current_view = match self.view_selector.selected {
                0 => ProxyView::List,
                1 => ProxyView::Add,
                2 => ProxyView::HealthCheck,
                3 => ProxyView::Test,
                _ => ProxyView::List,
            };
            if matches!(self.current_view, ProxyView::List) {
                self.update_list_view();
            }
        } else if self.inputs.is_focused() {
            self.inputs.blur();
        }
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.stop();
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
        if self.view_selector.is_focused() && self.view_selector.is_open() {
            self.view_selector.handle_up();
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
        if self.view_selector.is_focused() && self.view_selector.is_open() {
            self.view_selector.handle_down();
        } else if !self.inputs.is_focused() {
            self.results_view.scroll_down(1);
        } else {
            self.inputs.focus_next();
        }
    }

    fn handle_left(&mut self) -> bool {
        if !self.is_running() {
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
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if !self.is_running() {
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
        } else {
            false
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
}

fn parse_proxy_url(
    proxy_url: &str,
) -> Result<(String, u16, Option<String>, Option<String>), String> {
    let url = url::Url::parse(proxy_url).map_err(|e| format!("Invalid proxy URL: {}", e))?;

    let host = url
        .host_str()
        .ok_or_else(|| "Proxy URL missing host".to_string())?
        .to_string();

    let port = url
        .port()
        .ok_or_else(|| "Proxy URL missing port".to_string())?;

    let (username, password) = if let Some(pwd) = url.password() {
        (Some(url.username().to_string()), Some(pwd.to_string()))
    } else if !url.username().is_empty() {
        (Some(url.username().to_string()), None)
    } else {
        (None, None)
    };

    Ok((host, port, username, password))
}
