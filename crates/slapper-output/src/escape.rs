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

pub fn escape_csv(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    let normalized: String = s.nfkc().collect();
    let formula_chars = ['=', '+', '-', '@', '\t', '\r'];
    let starts_with_formula = normalized
        .chars()
        .next()
        .map(|c| c.is_ascii() && formula_chars.contains(&c))
        .unwrap_or(false);

    if normalized.contains(',')
        || normalized.contains('"')
        || normalized.contains('\n')
        || normalized.contains('\r')
        || normalized.contains('\t')
        || starts_with_formula
    {
        format!("\"{}\"", normalized.replace('"', "\"\""))
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fullwidth_equals_bypass() {
        assert!(escape_csv("\u{FF1D}1+1").starts_with('"'));
    }

    #[test]
    fn test_fullwidth_plus_bypass() {
        assert!(escape_csv("\u{FF0B}2+2").starts_with('"'));
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
