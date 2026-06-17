use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

use crate::app::NotificationSeverity;
use crate::theme::Theme;
use crate::App;
use crate::InputMode;

use crate::tabs::{spec_for, TabRiskGroup};
use eggsec::config::{EnforcementOutcome, ExecutionProfile, ScopeSource};

pub fn draw_tabs(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    use crate::tabs::{Tab, TabWindow};
    use ratatui::text::Line;

    let window = TabWindow::for_width(area.width, app.current_tab, app.tab_scroll_offset);

    // Phase 9: for very narrow tab bar (<60w or max_visible<=1), switch to compact breadcrumb mode
    // instead of full tab spans. Keeps navigation hint ^X for quick switch. Existing TabWindow
    // computation still used for offset/selected logic if needed later.
    if area.width < 60 || window.max_visible <= 1 {
        let idx = app.current_tab.visible_index().unwrap_or(0) + 1;
        let total = Tab::all().len();
        let title = app.current_tab.title();
        let text = format!("[{}/{}] {}  ^X quick", idx, total, title);
        let para = Paragraph::new(Line::from(text))
            .block(Block::default().borders(Borders::ALL).title("Eggsec"))
            .style(Style::default().fg(theme.colors.tab_active));
        f.render_widget(para, area);
        return;
    }

    let all_tabs: Vec<Line> = Tab::all().iter().map(|t| Line::from(t.title())).collect();
    let visible_titles: Vec<Line> = all_tabs[window.start..window.end].to_vec();

    let tabs = Tabs::new(visible_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Eggsec{}", window.range_text())),
        )
        .select(window.selected_visible)
        .style(Style::default().fg(theme.colors.tab_active))
        .highlight_style(
            Style::default()
                .fg(theme.colors.highlight)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}

pub fn draw_breadcrumb(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    use ratatui::text::{Line, Span};

    let parts = app
        .current_tab
        .as_tab_render(app)
        .breadcrumb()
        .unwrap_or_else(|| app.current_tab.default_breadcrumb());

    let mut spans = Vec::new();
    let total_parts = parts.len();

    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                " > ",
                Style::default().fg(theme.colors.text_dim),
            ));
        }

        let is_last = i == total_parts - 1;
        let style = if is_last {
            Style::default()
                .fg(theme.colors.accent)
                .add_modifier(Modifier::BOLD)
        } else if i == 0 {
            Style::default()
                .fg(theme.colors.text)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.colors.primary)
        };

        spans.push(Span::styled(*part, style));
    }

    let block = Block::default()
        .borders(Borders::NONE)
        .border_style(Style::default().fg(theme.colors.border));

    let paragraph = Paragraph::new(Line::from(spans))
        .block(block)
        .style(Style::default().fg(theme.colors.text));

    f.render_widget(paragraph, area);
}

pub fn draw_content(f: &mut Frame, app: &App, area: Rect) {
    use crate::tabs::TabRender;
    let insert_mode = app.mode == crate::InputMode::Insert;

    if app.current_tab == crate::tabs::Tab::History {
        let h = app.history.lock();
        h.render(f, area, insert_mode);
        h.render_overlays(f, area);
        return;
    }

    let tab_render = app.current_tab.as_tab_render(app);
    tab_render.render(f, area, insert_mode);
    tab_render.render_overlays(f, area);
}

pub fn draw_status_bar(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let is_narrow = area.width < 100;
    let is_very_narrow = area.width < 60;
    // Phase 6: when task globally active, ensure status shows task strip (name/state/elapsed/hints)
    // even after navigating away. Prefer task summary unless an *error* notification is active
    // (keep error notif priority for visibility of failures). Non-error notifs yield to task.
    let (mut status_text, mut status_color) = if let Some(notif) = &app.overlay.notification {
        if !notif.is_expired() && notif.severity == NotificationSeverity::Error {
            let color = theme.colors.error;
            (notif.message.clone(), color)
        } else if app.has_active_task() {
            if let Some(summary) = app.task_status_summary() {
                let state = if app.is_paused() { "paused" } else { "running" };
                let col = theme
                    .style_for_task_state(state)
                    .fg
                    .unwrap_or(theme.colors.status_running);
                (summary, col)
            } else {
                get_normal_status(app, theme)
            }
        } else if !notif.is_expired() {
            let color = match notif.severity {
                NotificationSeverity::Info => theme.colors.status_idle,
                NotificationSeverity::Success => theme.colors.success,
                NotificationSeverity::Warning => theme.colors.warning,
                NotificationSeverity::Error => theme.colors.error,
            };
            (notif.message.clone(), color)
        } else {
            get_normal_status(app, theme)
        }
    } else if app.has_active_task() {
        if let Some(summary) = app.task_status_summary() {
            let state = if app.is_paused() { "paused" } else { "running" };
            let col = theme
                .style_for_task_state(state)
                .fg
                .unwrap_or(theme.colors.status_running);
            (summary, col)
        } else {
            get_normal_status(app, theme)
        }
    } else {
        get_normal_status(app, theme)
    };

    // Phase 5/6/9: compact handling on narrow (<100w). For active task, use shortened strip.
    // <60w: drop secs, keep only "Task: X [C]". Preflight compact for target tabs.
    // Low-priority drop first on <60w / <80w.
    let use_compact = is_narrow;
    let use_very_compact = is_very_narrow;
    if use_compact {
        if app.has_active_task() {
            if let Some(tab) = app.active_task_tab() {
                let name = tab.title();
                let state = if app.is_paused() { "P" } else { "R" };
                if use_very_compact {
                    // Phase 9: drop elapsed seconds on <60w, minimal hints.
                    status_text = format!("Task:{name} [{state}]");
                } else {
                    let secs = app.active_task_elapsed_secs().unwrap_or(0);
                    let short_hints = if app.is_paused() {
                        "[Y res]"
                    } else {
                        "[C stop Z pause]"
                    };
                    status_text = format!("Task:{name} {secs}s {state} {short_hints}");
                }
                let tstate = if app.is_paused() { "paused" } else { "running" };
                status_color = theme
                    .style_for_task_state(tstate)
                    .fg
                    .unwrap_or(theme.colors.status_running);
            }
        } else if let Some(spec) = spec_for(app.current_tab) {
            if spec.operation.is_some() {
                let (c_text, c_color) = get_preflight_status(app, theme, true, use_very_compact);
                status_text = c_text;
                status_color = c_color;
            } else if status_text.chars().count() > 45 {
                status_text = format!("{}…", status_text.chars().take(42).collect::<String>());
            }
        }
    }

    let help_text = get_help_text(app, area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(if is_narrow {
            [
                Constraint::Length(8),
                Constraint::Percentage(60),
                Constraint::Percentage(40),
            ]
        } else {
            [
                Constraint::Length(10),
                Constraint::Percentage(55),
                Constraint::Percentage(40),
            ]
        })
        .split(area);

    let mode_text = match app.mode {
        InputMode::Normal => "NORMAL",
        InputMode::Insert => "INSERT",
    };
    let mode_color = match app.mode {
        InputMode::Normal => theme.colors.mode_normal,
        InputMode::Insert => theme.colors.mode_insert,
    };
    let mode_indicator_widget = ratatui::widgets::Paragraph::new(format!(" {} ", mode_text)).style(
        Style::default()
            .fg(theme.colors.background)
            .bg(mode_color)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(
        mode_indicator_widget,
        chunks.first().copied().unwrap_or(area),
    );

    let status =
        ratatui::widgets::Paragraph::new(status_text).style(Style::default().fg(status_color));
    f.render_widget(status, chunks.get(1).copied().unwrap_or(area));

    let help = ratatui::widgets::Paragraph::new(help_text)
        .style(Style::default().fg(theme.colors.text_dim));
    f.render_widget(help, chunks.get(2).copied().unwrap_or(area));
}

pub fn get_tab_status(
    state: &crate::tabs::AppState,
    theme: &Theme,
) -> (String, ratatui::style::Color) {
    use crate::tabs::AppState;
    match state {
        AppState::Idle => (
            "Ready - Press Enter to start".to_string(),
            theme.colors.status_idle,
        ),
        AppState::Running => (
            "Running - Ctrl+C to stop".to_string(),
            theme.colors.status_running,
        ),
        AppState::Completed => ("Completed".to_string(), theme.colors.success),
        AppState::Error(e) => (e.to_string(), theme.colors.error),
    }
}

pub fn get_normal_status(app: &App, theme: &Theme) -> (String, ratatui::style::Color) {
    // Phase 6: if a task is globally active (even on another tab), surface the task strip
    // in the normal status path so UI indicates active task after nav away. Notif errors
    // still take precedence in draw_status_bar. Advisory only for visibility.
    if app.has_active_task() {
        if let Some(summary) = app.task_status_summary() {
            return (summary, theme.colors.status_running);
        }
    }
    // Phase 5: for tabs with declared operation (target-bearing), show persistent
    // manual-mode + scope provenance + risk + target preflight (advisory).
    // Uses TabSpec (risk_group/operation) + delegated descriptor via primary_target +
    // central EnforcementContext::evaluate for best-effort preview. Actual eval at launch.
    if let Some(spec) = spec_for(app.current_tab) {
        if spec.operation.is_some() {
            // Phase 9: pass very_narrow flag for further dropping on <60w
            let is_very = false; // caller context already handled in draw_status_bar path; normal path uses full
            return get_preflight_status(app, theme, false, is_very);
        }
    }
    match app.current_tab {
        crate::tabs::Tab::Settings => (
            "Press 's' to save settings, 'r' to reset".to_string(),
            theme.colors.status_idle,
        ),
        crate::tabs::Tab::History => (
            "↑↓ Navigate | 'd' Delete | 'r' Clear all".to_string(),
            theme.colors.status_idle,
        ),
        crate::tabs::Tab::Dashboard => (
            "Dashboard - View scan results overview".to_string(),
            theme.colors.status_idle,
        ),
        _ => {
            let state = app.current_tab.as_tab_state(app).state();
            get_tab_status(&state, theme)
        }
    }
}

/// Best-effort preflight indicator (Phase 5). Advisory only; never blocks edits or nav.
/// Computes using current TabSpec + tab.primary_target (delegated) + EnforcementContext evaluate.
/// Scope match uses LoadedScope provenance + cheap Scope::is_target_allowed when target present.
fn get_preflight_status(
    app: &App,
    theme: &Theme,
    compact: bool,
    very_compact: bool,
) -> (String, ratatui::style::Color) {
    // Descriptor via the post-Phase4 delegation (spec + primary_target path in App).
    let desc_opt = app.build_current_operation_descriptor();

    let spec = match spec_for(app.current_tab) {
        Some(s) if s.operation.is_some() => s,
        _ => {
            // fallback
            let state = app.current_tab.as_tab_state(app).state();
            return get_tab_status(&state, theme);
        }
    };

    // Target comes from the delegated path (build_current... already uses spec + tab.primary_target()).
    // We extract from the descriptor (populated by tabs via primary_target) to avoid calling TabInput
    // method on TabState dyn. This keeps preflight advisory and consistent with Phase 4 delegation.
    let target = desc_opt
        .as_ref()
        .and_then(|d| d.target.clone())
        .unwrap_or_default();
    let target_short = if target.chars().count() > 28 {
        format!("{}…", target.chars().take(25).collect::<String>())
    } else {
        target.clone()
    };

    let risk_str = match spec.risk_group {
        TabRiskGroup::Passive => "passive",
        TabRiskGroup::SafeActive => "safe",
        TabRiskGroup::Intrusive => "intrusive",
        TabRiskGroup::Administrative => "admin",
    };
    let mode_str = match app.enforcement.execution_profile {
        ExecutionProfile::ManualPermissive => {
            if compact {
                "manual"
            } else {
                "manual-permissive"
            }
        }
        ExecutionProfile::ManualGuarded => "manual-guarded",
        ExecutionProfile::CiStrict => "ci-strict",
        ExecutionProfile::McpStrict => "mcp-strict",
        ExecutionProfile::AgentStrict => "agent-strict",
    };

    let scope_str = match app.loaded_scope.source {
        ScopeSource::DefaultEmpty => {
            if compact {
                "default"
            } else {
                "default-empty"
            }
        }
        ScopeSource::ConfigFile => "config",
        ScopeSource::CliScopeFile => "--scope",
        ScopeSource::GeneratedPreset => "preset",
    };

    // Advisory only: run the real central evaluator (no side effects, same as launch path).
    let will = if let Some(ref desc) = desc_opt {
        match app.enforcement.evaluate(desc) {
            EnforcementOutcome::Allow(_) => "run",
            EnforcementOutcome::Warn(_) => "warn",
            EnforcementOutcome::RequireConfirmation(_) => "confirm",
            EnforcementOutcome::Deny(_) => "deny",
        }
    } else {
        "run"
    };

    // Phase 10: use semantic policy outcome helper instead of ad-hoc success/warning/error
    let will_color = theme
        .style_for_policy_outcome(will)
        .fg
        .unwrap_or(theme.colors.text_dim);

    // For the common startup/empty-target case on safe tabs (e.g. default Recon in tests),
    // keep the status bar color as idle (preserves existing test expectations for get_normal_status).
    // Intrusive/admin/risky cases and explicit targets use the semantic will/risk color.
    let status_color = if will == "run" && target.trim().is_empty() {
        theme.colors.status_idle
    } else {
        will_color
    };

    // Scope match: provenance + (when target present) cheap rule check from LoadedScope's inner Scope.
    // This avoids duplicating enforcement/scope logic for the UI preview label.
    let scope_match = if target.trim().is_empty() {
        "no-tgt"
    } else {
        match app.loaded_scope.scope.is_target_allowed(target.as_str()) {
            Ok(true) => {
                if compact {
                    "in"
                } else {
                    "in-scope"
                }
            }
            Ok(false) => {
                if compact {
                    "out"
                } else {
                    "out"
                }
            }
            Err(_) => "?",
        }
    };

    if compact {
        // Concise for <100w (or <80 in help paths): drop long labels, keep essentials.
        // e.g. "manual|default|intrus|tgt(in)|confirm?"
        // Phase 9: on very_compact (<60w) drop "Mode:" / "Scope:" labels, shorten further, keep only essentials.
        let tpart = if !target_short.is_empty() {
            format!("{}({})", target_short, scope_match)
        } else {
            scope_match.to_string()
        };
        if very_compact {
            // Drop Mode/Scope long labels; keep risk + target+scope + will (minimal for <60w preflight).
            let txt = format!("{}|{}|{}|{}", risk_str, tpart, will, mode_str);
            (txt, status_color)
        } else {
            let txt = format!("{}|{}|{}|{}|{}", mode_str, scope_str, risk_str, tpart, will);
            (txt, status_color)
        }
    } else {
        let will_hint = match will {
            "confirm" => "Enter: confirm required",
            "deny" => "deny (policy)",
            "warn" => "warn (proceed)",
            _ => "Enter: run",
        };
        let txt = if target_short.is_empty() {
            format!(
                "Mode: {} | Scope: {} | Risk: {} | {}",
                mode_str, scope_str, risk_str, will_hint
            )
        } else {
            format!(
                "Mode: {} | Scope: {} | Risk: {} | Target: {} ({}) | {}",
                mode_str, scope_str, risk_str, target_short, scope_match, will_hint
            )
        };
        (txt, status_color)
    }
}

pub fn get_help_text(app: &App, area: Rect) -> String {
    let is_narrow = area.width < 80;

    if app.overlay.pending_action.is_some() {
        return "[Enter] Confirm [Esc] Cancel".to_string();
    }

    if app
        .get_command_palette()
        .map(|p| p.visible)
        .unwrap_or(false)
    {
        return if is_narrow {
            "[Enter] Run [↑↓] Sel [Esc] Close".to_string()
        } else {
            "[Enter] Run [Up/Down] Select [Esc] Close".to_string()
        };
    }

    if app.overlay.show_search {
        return if is_narrow {
            "[Enter] Search [Bksp] Edit [Esc] Close".to_string()
        } else {
            "[Enter] Search [Backspace] Edit [Esc] Close".to_string()
        };
    }

    if app.overlay.show_help {
        return if is_narrow {
            "[Esc] Close | [h/l] Pane Nav".to_string()
        } else {
            "[Esc] Close Help | [h/l] Pane Navigation".to_string()
        };
    }

    match app.mode {
        InputMode::Normal => {
            // Phase 6: when global task active, make quit-block and task control hints prominent in help.
            // Status bar already carries the full task strip (tab/state/elapsed/hints); help reinforces
            // that q is blocked and surfaces the core task keys. Paused hints preserved/integrated.
            if app.has_active_task() {
                let pause_resume = if app.is_paused() {
                    if is_narrow {
                        " [Y res]"
                    } else {
                        " [Ctrl+Y] Resume"
                    }
                } else {
                    if is_narrow {
                        " [Z pause]"
                    } else {
                        " [Ctrl+Z] Pause"
                    }
                };
                if is_narrow {
                    format!("[C stop] Task active [q] blocked{pause_resume}")
                } else {
                    format!("[Ctrl+C] Stop task | q blocked while task active{pause_resume}")
                }
            } else if is_narrow {
                format!(
                    "[n/p] Tabs [hjkl] Move [/] Search [^X] Quick{} [q] Quit",
                    if app.is_paused() { " [P]" } else { "" }
                )
            } else {
                format!(
                    "[n/p] Tabs [hjkl] Move [/] Search [Ctrl+X] Quick Switch [Space] Help [q] Quit{}",
                    if app.is_paused() { " [Ctrl+Y] Resume" } else { "" }
                )
            }
        }
        InputMode::Insert => {
            if is_narrow {
                "[Esc] Normal [Tab] Next [Arw] Move [^V] Paste".to_string()
            } else {
                "[Esc] Normal Mode | [Tab/S-Tab] Focus | [Arrows] Move | [Ctrl+V] Paste".to_string()
            }
        }
    }
}

/// Phase 9: very small terminal guard. Below this we render a clear fallback message
/// instead of attempting normal layout (prevents garbled UI). 80x24 good, 60x20 usable,
/// terminals <45 cols or <12 rows trigger fallback (tuned to cover "very small" per plan e.g. ~40x10
/// and the dedicated render test at 40x12; 60x20 must remain usable and not hit this).
/// Policy confirms are still rendered (clamped) even in this path for readability.
pub fn is_terminal_too_small(area: Rect) -> bool {
    area.width < 45 || area.height < 12
}
