use ratatui::style::Color;

/// Compute the linearized value of an sRGB channel (0.0–1.0).
fn linearize(channel: f64) -> f64 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// Compute relative luminance for a `Color` using the WCAG 2.x formula.
///
/// For `Color::Rgb(r, g, b)` the standard formula is applied:
/// `0.2126*R_lin + 0.7152*G_lin + 0.0722*B_lin`
///
/// Non-RGB colors (e.g. `Color::Reset`, named terminal colors) return a
/// neutral default of 0.5.
pub fn relative_luminance(color: Color) -> f64 {
    match color {
        Color::Rgb(r, g, b) => {
            let r = linearize(r as f64 / 255.0);
            let g = linearize(g as f64 / 255.0);
            let b = linearize(b as f64 / 255.0);
            0.2126 * r + 0.7152 * g + 0.0722 * b
        }
        _ => 0.5,
    }
}

/// Compute the WCAG contrast ratio between two colors.
///
/// Returns a value in `[1.0, 21.0]`. A ratio of 1.0 means identical
/// luminance; 21.0 means pure black vs pure white.
pub fn contrast_ratio(c1: Color, c2: Color) -> f64 {
    let l1 = relative_luminance(c1);
    let l2 = relative_luminance(c2);
    let (lighter, darker) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

/// Check whether `fg` on `bg` meets the minimum contrast ratio `min_ratio`.
pub fn check_contrast(fg: Color, bg: Color, min_ratio: f64) -> bool {
    contrast_ratio(fg, bg) >= min_ratio
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luminance_black_is_zero() {
        let lum = relative_luminance(Color::Rgb(0, 0, 0));
        assert!((lum - 0.0).abs() < 1e-10, "black luminance should be 0.0, got {lum}");
    }

    #[test]
    fn luminance_white_is_one() {
        let lum = relative_luminance(Color::Rgb(255, 255, 255));
        assert!((lum - 1.0).abs() < 1e-10, "white luminance should be 1.0, got {lum}");
    }

    #[test]
    fn contrast_ratio_black_vs_white() {
        let ratio = contrast_ratio(Color::Rgb(0, 0, 0), Color::Rgb(255, 255, 255));
        assert!((ratio - 21.0).abs() < 1e-10, "black/white ratio should be 21.0, got {ratio}");
    }

    #[test]
    fn contrast_ratio_same_color_is_one() {
        let c = Color::Rgb(128, 64, 32);
        let ratio = contrast_ratio(c, c);
        assert!((ratio - 1.0).abs() < 1e-10, "same-color ratio should be 1.0, got {ratio}");
    }

    #[test]
    fn check_contrast_passes_for_high_contrast() {
        // White text on black background — ratio ≈ 21.0, well above 4.5
        assert!(check_contrast(
            Color::Rgb(255, 255, 255),
            Color::Rgb(0, 0, 0),
            4.5
        ));
    }

    #[test]
    fn check_contrast_fails_for_low_contrast() {
        // Dark gray on slightly darker gray — ratio well below 4.5
        assert!(!check_contrast(
            Color::Rgb(80, 80, 80),
            Color::Rgb(70, 70, 70),
            4.5
        ));
    }

    #[test]
    fn non_rgb_colors_default_to_neutral() {
        let lum = relative_luminance(Color::Reset);
        assert!((lum - 0.5).abs() < 1e-10, "non-RGB should default to 0.5, got {lum}");
    }
}
