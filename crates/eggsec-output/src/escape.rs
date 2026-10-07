use std::borrow::Cow;

pub fn escape_html(s: &str) -> String {
    let mut buf = String::with_capacity(s.len() + 64);
    for c in s.chars() {
        match c {
            '&' => buf.push_str("&amp;"),
            '<' => buf.push_str("&lt;"),
            '>' => buf.push_str("&gt;"),
            '"' => buf.push_str("&quot;"),
            '\'' => buf.push_str("&#39;"),
            _ => buf.push(c),
        }
    }
    buf
}

/// Escape a value for CSV output.
///
/// Two independent concerns:
/// 1. CSV *syntax* — a field containing a separator, quote or newline must be
///    quoted with embedded quotes doubled.
/// 2. CSV *formula injection* — Excel/LibreOffice/Sheets evaluate a cell whose
///    text begins with `=`, `+`, `-`, `@` (or a leading TAB/CR), so a banner or
///    path from a scanned host can execute a formula or DDE payload on the
///    analyst's workstation.
///
/// Quoting alone does **not** neutralise (2): `"=1+1"` still parses to the
/// value `=1+1`, which is still evaluated. A leading apostrophe is the standard
/// neutralisation (imported as text, not evaluated), and the apostrophe stays
/// inside the quotes so the CSV shape is unchanged.
pub fn escape_csv(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    let normalized: String = s.nfkc().collect();
    let formula_chars = ['=', '+', '-', '@', '\t', '\r'];
    let starts_with_formula = normalized
        .chars()
        .next()
        .map(|c| c.is_ascii() && formula_chars.contains(&c))
        .unwrap_or(false);

    // Neutralise the formula first so the leading character is no longer one of
    // the trigger characters.
    let neutralized: Cow<str> = if starts_with_formula {
        Cow::Owned(format!("'{normalized}"))
    } else {
        Cow::Borrowed(normalized.as_str())
    };

    if normalized.contains(',')
        || normalized.contains('"')
        || normalized.contains('\n')
        || normalized.contains('\r')
        || normalized.contains('\t')
        || starts_with_formula
    {
        format!("\"{}\"", neutralized.replace('"', "\"\""))
    } else {
        neutralized.into_owned()
    }
}

pub fn escape_xml(s: &str) -> String {
    let mut buf = String::with_capacity(s.len() + 64);
    for c in s.chars() {
        match c {
            '&' => buf.push_str("&amp;"),
            '<' => buf.push_str("&lt;"),
            '>' => buf.push_str("&gt;"),
            '"' => buf.push_str("&quot;"),
            '\'' => buf.push_str("&apos;"),
            _ => buf.push(c),
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    // Assert the *neutralised* form: quoting alone leaves the formula
    // evaluable, so the apostrophe prefix is the actual defence.
    #[test]
    fn test_fullwidth_equals_bypass() {
        assert_eq!(escape_csv("\u{FF1D}1+1"), "\"'=1+1\"");
    }

    #[test]
    fn test_fullwidth_plus_bypass() {
        assert_eq!(escape_csv("\u{FF0B}2+2"), "\"'+2+2\"");
    }

    #[test]
    fn test_csv_quotes_tab_mid_field() {
        let result = escape_csv("hello\tworld");
        assert!(result.starts_with('"'));
        assert!(result.contains('\t'));
    }

    #[test]
    fn test_csv_quotes_cr_mid_field() {
        let result = escape_csv("hello\rworld");
        assert!(result.starts_with('"'));
        assert!(result.contains('\r'));
    }

    #[test]
    fn test_csv_neutralises_leading_formula_chars() {
        for (input, expected) in [
            ("=1+1", "\"'=1+1\""),
            ("+1+1", "\"'+1+1\""),
            ("-2+3", "\"'-2+3\""),
            ("@SUM(A1)", "\"'@SUM(A1)\""),
            ("\t=cmd", "\"'\t=cmd\""),
        ] {
            let out = escape_csv(input);
            assert_eq!(out, expected, "input {input:?}");
            // The cell must not *parse* to a formula-leading value.
            let parsed = out.trim_matches('"');
            assert!(
                !matches!(parsed.chars().next(), Some('=' | '+' | '-' | '@')),
                "still formula-leading: {parsed:?}"
            );
        }
    }

    #[test]
    fn test_csv_leaves_ordinary_values_untouched() {
        assert_eq!(escape_csv("nginx/1.24.0"), "nginx/1.24.0");
        assert_eq!(escape_csv("/admin/login"), "/admin/login");
    }

    #[test]
    fn test_csv_formula_with_embedded_comma_is_quoted_and_neutralised() {
        let out = escape_csv("=cmd,'/C calc'!A0");
        assert!(out.starts_with('"'));
        assert!(out.starts_with("\"'"));
    }
}
