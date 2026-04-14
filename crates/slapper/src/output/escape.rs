pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn escape_csv(s: &str) -> String {
    let formula_chars = ['=', '+', '-', '@', '\t', '\r'];
    let starts_with_formula = s
        .chars()
        .next()
        .map(|c| formula_chars.contains(&c))
        .unwrap_or(false);
    if s.contains(',') || s.contains('"') || s.contains('\n') || starts_with_formula {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[allow(dead_code)]
pub fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
