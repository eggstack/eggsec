use ratatui::style::Color;
use serde::Deserialize;
use thiserror::Error;

use super::builtin::{dark_theme, light_theme};
use super::canonical_theme_id;
use super::palette::{Theme, ThemeColors, ThemeMode};

#[derive(Debug, Error)]
pub enum ThemeLoadError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("no color definitions found in theme")]
    NoColors,
}

#[derive(Debug, Deserialize, Default)]
struct HalloyTheme {
    general: Option<HalloyGeneral>,
    text: Option<HalloyText>,
    buffer: Option<HalloyBuffer>,
    buttons: Option<HalloyButtons>,
}

#[derive(Debug, Deserialize, Default)]
struct HalloyGeneral {
    background: Option<String>,
    border: Option<String>,
    horizontal_rule: Option<String>,
    unread_indicator: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct HalloyText {
    primary: Option<String>,
    secondary: Option<String>,
    tertiary: Option<String>,
    success: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct HalloyBuffer {
    action: Option<String>,
    background: Option<String>,
    background_text_input: Option<String>,
    background_title_bar: Option<String>,
    border: Option<String>,
    border_selected: Option<String>,
    code: Option<String>,
    highlight: Option<String>,
    nickname: Option<String>,
    selection: Option<String>,
    timestamp: Option<String>,
    topic: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct HalloyButtons {
    primary: Option<HalloyButtonStyle>,
    secondary: Option<HalloyButtonStyle>,
}

#[derive(Debug, Deserialize, Default)]
struct HalloyButtonStyle {
    background: Option<String>,
    background_hover: Option<String>,
    background_selected: Option<String>,
    background_selected_hover: Option<String>,
}

fn parse_hex_color(s: &str) -> Option<Color> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Color::Rgb(r, g, b))
            }
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
                Some(Color::Rgb(r * 17, g * 17, b * 17))
            }
            _ => None,
        }
    } else {
        match s {
            "black" => Some(Color::Black),
            "red" => Some(Color::Red),
            "green" => Some(Color::Green),
            "yellow" => Some(Color::Yellow),
            "blue" => Some(Color::Blue),
            "magenta" | "purple" => Some(Color::Magenta),
            "cyan" => Some(Color::Cyan),
            "white" => Some(Color::White),
            "gray" | "grey" => Some(Color::Gray),
            "darkgray" | "darkgrey" => Some(Color::DarkGray),
            "lightgreen" => Some(Color::LightGreen),
            "lightblue" => Some(Color::LightBlue),
            _ => None,
        }
    }
}

fn parse_color_or(s: &Option<String>, default: Color) -> Color {
    s.as_deref().and_then(parse_hex_color).unwrap_or(default)
}

fn luminance(color: &str) -> f64 {
    if let Some(hex) = color.strip_prefix('#') {
        // Expand 3-char shorthand hex (#FFF -> #FFFFFF) before parsing
        let hex = if hex.len() == 3 {
            let mut expanded = String::with_capacity(6);
            for ch in hex.chars() {
                expanded.push(ch);
                expanded.push(ch);
            }
            expanded
        } else {
            hex.to_string()
        };
        if hex.len() < 6 {
            return 0.5;
        }
        let parse = |start: usize| -> f64 {
            u8::from_str_radix(&hex[start..start + 2], 16)
                .map(|v| v as f64 / 255.0)
                .unwrap_or(0.0)
        };
        let r = parse(0);
        let g = parse(2);
        let b = parse(4);
        return 0.2126 * r + 0.7152 * g + 0.0722 * b;
    }
    match color.to_ascii_lowercase().as_str() {
        "black" => 0.0,
        "white" => 1.0,
        "red" | "darkred" => 0.3,
        "lightred" => 0.7,
        "green" => 0.7,
        "darkgreen" => 0.2,
        "lightgreen" => 0.8,
        "blue" | "darkblue" => 0.3,
        "lightblue" => 0.8,
        "yellow" | "lightyellow" => 0.8,
        "orange" => 0.6,
        "darkorange" => 0.4,
        "gray" | "grey" | "darkgray" | "darkgrey" => 0.35,
        "lightgray" | "lightgrey" | "silver" => 0.75,
        "cyan" | "darkcyan" => 0.4,
        "lightcyan" => 0.8,
        "magenta" | "purple" => 0.4,
        _ => {
            // Unknown color name - warn so contributors notice non-standard names.
            tracing::warn!(
                "Theme color '{}' is not a recognized name; defaulting to neutral luminance",
                color
            );
            0.5
        }
    }
}

fn halloy_to_theme(halloy: &HalloyTheme, file_stem: &str) -> Result<Theme, ThemeLoadError> {
    let bg_hex = halloy
        .general
        .as_ref()
        .and_then(|g| g.background.clone())
        .or_else(|| halloy.buffer.as_ref().and_then(|b| b.background.clone()));

    let has_any_color = bg_hex.is_some()
        || halloy.text.is_some()
        || halloy.buffer.is_some()
        || halloy.buttons.is_some()
        || halloy.general.as_ref().is_some_and(|g| {
            g.border.is_some() || g.horizontal_rule.is_some() || g.unread_indicator.is_some()
        });

    if !has_any_color {
        return Err(ThemeLoadError::NoColors);
    }

    let defaults = match bg_hex.as_deref().map(luminance) {
        Some(lum) if lum < 0.5 => dark_theme(),
        Some(_) => light_theme(),
        None => dark_theme(),
    };

    let mode = match bg_hex.as_deref().map(luminance) {
        Some(lum) if lum < 0.5 => ThemeMode::Dark,
        Some(_) => ThemeMode::Light,
        None => ThemeMode::Dark,
    };

    let general = halloy.general.as_ref();
    let text = halloy.text.as_ref();
    let buffer = halloy.buffer.as_ref();
    let buttons = halloy.buttons.as_ref();

    let colors = ThemeColors {
        primary: parse_color_or(
            &text.and_then(|t| t.primary.clone()),
            defaults.colors.primary,
        ),
        secondary: parse_color_or(
            &text.and_then(|t| t.secondary.clone()),
            defaults.colors.secondary,
        ),
        accent: parse_color_or(
            &buffer
                .and_then(|b| b.highlight.clone())
                .or_else(|| text.and_then(|t| t.tertiary.clone())),
            defaults.colors.accent,
        ),
        background: parse_color_or(&bg_hex, defaults.colors.background),
        foreground: parse_color_or(
            &text.and_then(|t| t.primary.clone()),
            defaults.colors.foreground,
        ),
        surface: parse_color_or(
            &buffer.and_then(|b| b.background.clone()),
            defaults.colors.surface,
        ),
        border: parse_color_or(
            &general
                .and_then(|g| g.border.clone())
                .or_else(|| buffer.and_then(|b| b.border.clone())),
            defaults.colors.border,
        ),
        border_focused: parse_color_or(
            &buffer.and_then(|b| b.border_selected.clone()),
            defaults.colors.border_focused,
        ),
        text: parse_color_or(&text.and_then(|t| t.primary.clone()), defaults.colors.text),
        text_dim: parse_color_or(
            &text.and_then(|t| t.secondary.clone()),
            defaults.colors.text_dim,
        ),
        text_bright: parse_color_or(
            &text.and_then(|t| t.primary.clone()),
            defaults.colors.text_bright,
        ),
        success: parse_color_or(
            &text.and_then(|t| t.success.clone()),
            defaults.colors.success,
        ),
        warning: parse_color_or(
            &general.and_then(|g| g.unread_indicator.clone()),
            defaults.colors.warning,
        ),
        error: parse_color_or(&text.and_then(|t| t.error.clone()), defaults.colors.error),
        info: parse_color_or(&text.and_then(|t| t.tertiary.clone()), defaults.colors.info),
        selected: parse_color_or(
            &buffer.and_then(|b| b.selection.clone()),
            defaults.colors.selected,
        ),
        selected_text: parse_color_or(
            &buffer.and_then(|b| b.nickname.clone()),
            defaults.colors.selected_text,
        ),
        highlight: parse_color_or(
            &buffer.and_then(|b| b.highlight.clone()),
            defaults.colors.highlight,
        ),
        mode_normal: parse_color_or(
            &buttons
                .and_then(|b| b.primary.as_ref())
                .and_then(|p| p.background.clone()),
            defaults.colors.mode_normal,
        ),
        mode_insert: parse_color_or(
            &buttons
                .and_then(|b| b.secondary.as_ref())
                .and_then(|s| s.background.clone()),
            defaults.colors.mode_insert,
        ),
        tab_active: parse_color_or(
            &buffer.and_then(|b| b.border_selected.clone()),
            defaults.colors.tab_active,
        ),
        tab_inactive: parse_color_or(
            &general.and_then(|g| g.border.clone()),
            defaults.colors.tab_inactive,
        ),
        status_running: defaults.colors.status_running,
        status_idle: parse_color_or(
            &general.and_then(|g| g.border.clone()),
            defaults.colors.status_idle,
        ),
        status_error: parse_color_or(
            &text.and_then(|t| t.error.clone()),
            defaults.colors.status_error,
        ),
        focus_normal: parse_color_or(
            &buffer.and_then(|b| b.highlight.clone()),
            defaults.colors.focus_normal,
        ),
        focus_input: parse_color_or(
            &buffer.and_then(|b| b.border_selected.clone()),
            defaults.colors.focus_input,
        ),
        focus_results: parse_color_or(
            &text.and_then(|t| t.success.clone()),
            defaults.colors.focus_results,
        ),
        safe: parse_color_or(&text.and_then(|t| t.success.clone()), defaults.colors.safe),
        danger: parse_color_or(&text.and_then(|t| t.error.clone()), defaults.colors.danger),
        muted: parse_color_or(
            &text.and_then(|t| t.secondary.clone()),
            defaults.colors.muted,
        ),
        active_task: parse_color_or(
            &text.and_then(|t| t.success.clone()),
            defaults.colors.active_task,
        ),
        paused_task: parse_color_or(
            &general.and_then(|g| g.unread_indicator.clone()),
            defaults.colors.paused_task,
        ),
        scope_match: parse_color_or(
            &text.and_then(|t| t.success.clone()),
            defaults.colors.scope_match,
        ),
        scope_miss: parse_color_or(
            &general.and_then(|g| g.unread_indicator.clone()),
            defaults.colors.scope_miss,
        ),
        policy_required: parse_color_or(
            &general.and_then(|g| g.unread_indicator.clone()),
            defaults.colors.policy_required,
        ),
        policy_denied: parse_color_or(
            &text.and_then(|t| t.error.clone()),
            defaults.colors.policy_denied,
        ),
    };

    Ok(Theme {
        mode,
        name: canonical_theme_id(file_stem),
        colors,
    })
}

pub fn load_halloy_theme(content: &str, file_stem: &str) -> Result<Theme, ThemeLoadError> {
    let halloy: HalloyTheme = toml::from_str(content)?;
    halloy_to_theme(&halloy, file_stem)
}

pub fn load_halloy_theme_bytes(content: &[u8], file_stem: &str) -> Result<Theme, ThemeLoadError> {
    let s = std::str::from_utf8(content).map_err(|e| {
        use serde::de::Error;
        ThemeLoadError::ParseError(toml::de::Error::custom(e.to_string()))
    })?;
    load_halloy_theme(s, file_stem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_valid_theme() {
        let toml_content = r##"
[general]
background = "#2E3440"

[text]
primary = "#D8DEE9"
"##;
        let theme = load_halloy_theme(toml_content, "test-theme").unwrap();
        assert_eq!(theme.name, "test-theme");
        assert_eq!(theme.mode, ThemeMode::Dark);
        assert_eq!(theme.colors.background, Color::Rgb(0x2E, 0x34, 0x40));
        assert_eq!(theme.colors.text, Color::Rgb(0xD8, 0xDE, 0xE9));
    }

    #[test]
    fn canonicalizes_theme_names_from_file_stems() {
        let toml_content = r##"
[general]
background = "#2E3440"
"##;
        let theme = load_halloy_theme(toml_content, "Cyber Red").unwrap();
        assert_eq!(theme.name, "cyber-red");
    }

    #[test]
    fn malformed_toml_returns_error_no_panic() {
        let result = load_halloy_theme("{{{{not valid toml", "bad");
        assert!(result.is_err());
        assert!(matches!(result, Err(ThemeLoadError::ParseError(_))));
    }

    #[test]
    fn empty_theme_returns_no_colors() {
        let result = load_halloy_theme("", "empty");
        assert!(matches!(result, Err(ThemeLoadError::NoColors)));
    }

    #[test]
    fn parse_rrggbb_hex() {
        let c = parse_hex_color("#FF8800").unwrap();
        assert_eq!(c, Color::Rgb(0xFF, 0x88, 0x00));
    }

    #[test]
    fn parse_rgb_shorthand() {
        let c = parse_hex_color("#F80").unwrap();
        assert_eq!(c, Color::Rgb(0xFF, 0x88, 0x00));
    }

    #[test]
    fn parse_named_color() {
        assert_eq!(parse_hex_color("red"), Some(Color::Red));
        assert_eq!(parse_hex_color("cyan"), Some(Color::Cyan));
        assert_eq!(parse_hex_color("darkgray"), Some(Color::DarkGray));
    }

    #[test]
    fn invalid_hex_returns_none() {
        assert!(parse_hex_color("#ZZZZZZ").is_none());
        assert!(parse_hex_color("notacolor").is_none());
    }

    #[test]
    fn mode_dark_for_dark_background() {
        let toml_content = r##"
[general]
background = "#000000"
"##;
        let theme = load_halloy_theme(toml_content, "dark-test").unwrap();
        assert_eq!(theme.mode, ThemeMode::Dark);
    }

    #[test]
    fn mode_light_for_light_background() {
        let toml_content = r##"
[general]
background = "#FFFFFF"
"##;
        let theme = load_halloy_theme(toml_content, "light-test").unwrap();
        assert_eq!(theme.mode, ThemeMode::Light);
    }

    #[test]
    fn full_halloy_theme_mapping() {
        let toml_content = r##"
[general]
background = "#2E3440"
border = "#4C566A"
horizontal_rule = "#3B4252"
unread_indicator = "#81A1C1"

[text]
primary = "#D8DEE9"
secondary = "#616E88"
tertiary = "#D08770"
success = "#A3BE8C"
error = "#BF616A"

[buffer]
action = "#B48EAD"
background = "#3B4252"
background_text_input = "#2E3440"
background_title_bar = "#232831"
border = "#4C566A"
border_selected = "#81A1C1"
code = "#8FBCBB"
highlight = "#434C5E"
nickname = "#88C0D0"
selection = "#4C566A"
timestamp = "#616E88"
topic = "#D8DEE9"
url = "#88C0D0"

[buttons.primary]
background = "#3B4252"
background_hover = "#333D4D"
background_selected = "#232831"
background_selected_hover = "#1E222A"

[buttons.secondary]
background = "#4C566A"
background_hover = "#5A657D"
background_selected = "#81A1C1"
background_selected_hover = "#88C0D0"
"##;
        let theme = load_halloy_theme(toml_content, "nord").unwrap();
        assert_eq!(theme.name, "nord");
        assert_eq!(theme.mode, ThemeMode::Dark);
        assert_eq!(theme.colors.background, Color::Rgb(0x2E, 0x34, 0x40));
        assert_eq!(theme.colors.text, Color::Rgb(0xD8, 0xDE, 0xE9));
        assert_eq!(theme.colors.border, Color::Rgb(0x4C, 0x56, 0x6A));
        assert_eq!(theme.colors.border_focused, Color::Rgb(0x81, 0xA1, 0xC1));
        assert_eq!(theme.colors.success, Color::Rgb(0xA3, 0xBE, 0x8C));
        assert_eq!(theme.colors.error, Color::Rgb(0xBF, 0x61, 0x6A));
        assert_eq!(theme.colors.warning, Color::Rgb(0x81, 0xA1, 0xC1));
        assert_eq!(theme.colors.info, Color::Rgb(0xD0, 0x87, 0x70));
        assert_eq!(theme.colors.selected, Color::Rgb(0x4C, 0x56, 0x6A));
        assert_eq!(theme.colors.selected_text, Color::Rgb(0x88, 0xC0, 0xD0));
        assert_eq!(theme.colors.highlight, Color::Rgb(0x43, 0x4C, 0x5E));
        assert_eq!(theme.colors.mode_normal, Color::Rgb(0x3B, 0x42, 0x52));
        assert_eq!(theme.colors.mode_insert, Color::Rgb(0x4C, 0x56, 0x6A));
        assert_eq!(theme.colors.tab_active, Color::Rgb(0x81, 0xA1, 0xC1));
        assert_eq!(theme.colors.tab_inactive, Color::Rgb(0x4C, 0x56, 0x6A));
    }

    #[test]
    fn missing_fields_use_defaults() {
        let toml_content = r##"
[general]
background = "#1A1A2E"
"##;
        let theme = load_halloy_theme(toml_content, "minimal").unwrap();
        let defaults = dark_theme();
        assert_eq!(theme.colors.success, defaults.colors.success);
        assert_eq!(theme.colors.error, defaults.colors.error);
        assert_eq!(theme.colors.status_running, defaults.colors.status_running);
    }

    #[test]
    fn bytes_wrapper_works() {
        let content = b"[general]\nbackground = \"#000000\"";
        let theme = load_halloy_theme_bytes(content, "bytes-test").unwrap();
        assert_eq!(theme.name, "bytes-test");
        assert_eq!(theme.mode, ThemeMode::Dark);
    }

    #[test]
    fn invalid_utf8_bytes_returns_error() {
        let content = b"[general]\nbackground = \"#\xFF\xFE\"";
        let result = load_halloy_theme_bytes(content, "bad-utf8");
        assert!(result.is_err());
    }
}
