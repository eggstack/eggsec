/// Strip control chars (keep space) and either pad to `max_len` (for column alignment)
/// or truncate with "..." suffix if input exceeds `max_len`.
///
/// **Note**: Short inputs are right-padded with spaces to exactly `max_len` characters.
/// This is intentional for column-aligned terminal output (see callers in scanner/fuzzer
/// Display impls). Use `truncate_only` if you want pure truncation without padding.
pub fn strip_controls(s: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }
    let cleaned: String = s.chars().filter(|c| !c.is_control() || *c == ' ').collect();
    let char_count = cleaned.chars().count();
    if char_count > max_len {
        if max_len < 3 {
            cleaned.chars().take(max_len).collect()
        } else {
            let truncated: String = cleaned.chars().take(max_len.saturating_sub(3)).collect();
            format!("{}...", truncated)
        }
    } else {
        format!("{:<width$}", cleaned, width = max_len)
    }
}

/// Preserve all characters (no stripping) and either pad to `max_len` (for column alignment)
/// or truncate with "..." suffix if input exceeds `max_len`.
///
/// **Note**: Short inputs are right-padded with spaces to exactly `max_len` characters.
/// This is intentional for column-aligned terminal output.
pub fn preserve_all(s: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }
    let char_count = s.chars().count();
    if char_count > max_len {
        if max_len < 3 {
            s.chars().take(max_len).collect()
        } else {
            let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
            format!("{}...", truncated)
        }
    } else {
        format!("{:<width$}", s, width = max_len)
    }
}

pub fn truncate_only(s: &str, max_len: usize) -> String {
    s.chars().take(max_len).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_strip_controls_short_string() {
        let result = strip_controls("hello", 10);
        assert_eq!(result, "hello     ");
    }

    #[test]
    fn test_strip_controls_exact_length() {
        let result = strip_controls("hello", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_strip_controls_long_string() {
        let result = strip_controls("hello world this is a test", 10);
        assert_eq!(result, "hello w...");
    }

    #[test]
    fn test_strip_controls_removes_control_chars() {
        let result = strip_controls("hello\x00world", 20);
        assert_eq!(result, "helloworld          ");
    }

    #[test]
    fn test_strip_controls_preserves_unicode() {
        let result = strip_controls("héllo wörld", 15);
        assert!(result.contains("héllo"));
        assert!(result.contains("wörld"));
    }

    #[test]
    fn test_strip_controls_max_len_1() {
        let result = strip_controls("hello", 1);
        assert_eq!(result.len(), 1);
        assert!(!result.ends_with("..."));
    }

    #[test]
    fn test_strip_controls_max_len_2() {
        let result = strip_controls("hello", 2);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_preserve_all_max_len_1() {
        let result = preserve_all("hello", 1);
        assert_eq!(result.len(), 1);
        assert!(!result.ends_with("..."));
    }

    #[test]
    fn test_preserve_all_max_len_2() {
        let result = preserve_all("hello", 2);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_preserve_all_short() {
        let result = preserve_all("hello", 10);
        assert_eq!(result, "hello     ");
    }

    #[test]
    fn test_preserve_all_long() {
        let result = preserve_all("hello world this is a test", 10);
        assert_eq!(result, "hello w...");
    }

    proptest! {
        #[test]
        fn test_strip_controls_never_exceeds_max_len(s in "\\PC{0,100}", max_len in 5usize..50) {
            let result = strip_controls(&s, max_len);
            prop_assert!(result.chars().count() <= max_len);
        }

        #[test]
        fn test_preserve_all_never_exceeds_max_len(s in "[ -~]{0,100}", max_len in 5usize..50) {
            let result = preserve_all(&s, max_len);
            prop_assert!(result.chars().count() <= max_len);
        }
    }
}
