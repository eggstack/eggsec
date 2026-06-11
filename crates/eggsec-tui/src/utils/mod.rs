pub mod clipboard;
pub mod fuzzy;

pub use clipboard::Clipboard;

/// Minimal POSIX shell single-quote escape for safe CLI command generation.
/// Alphanumeric + safe chars (-_. /:@=&?+~) are passed unquoted when possible;
/// everything else (incl. spaces, quotes, $, `, etc.) is wrapped in single quotes
/// with internal ' turned into '\'' .
pub fn shell_escape(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }
    let safe = s
        .chars()
        .all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '/' | ':' | '@' | '=' | '?' | '&' | '+' | '~'));
    if safe {
        s.to_string()
    } else {
        let mut out = String::with_capacity(s.len() + 2);
        out.push('\'');
        for c in s.chars() {
            if c == '\'' {
                out.push_str("'\\''");
            } else {
                out.push(c);
            }
        }
        out.push('\'');
        out
    }
}
