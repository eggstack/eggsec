pub fn escape_html(s: &str) -> String {
    let mut buf = String::with_capacity(s.len() * 6);
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
    let formula_chars = ['=', '+', '-', '@', '\t', '\r'];
    let starts_with_formula = s
        .chars()
        .next()
        .map(|c| c.is_ascii() && formula_chars.contains(&c))
        .unwrap_or(false);

    let first_char_is_control = s
        .chars()
        .next()
        .map(|c| !c.is_ascii())
        .unwrap_or(false);

    if first_char_is_control || s.contains(',') || s.contains('"') || s.contains('\n') || starts_with_formula {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[allow(dead_code)]
pub fn escape_xml(s: &str) -> String {
    let mut buf = String::with_capacity(s.len() * 6);
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
