pub fn strip_controls(s: &str, max_len: usize) -> String {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .collect();
    if cleaned.len() > max_len {
        format!("{}...", &cleaned[..max_len.saturating_sub(3)])
    } else {
        format!("{:<width$}", cleaned, width = max_len)
    }
}

pub fn preserve_all(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    } else {
        format!("{:<width$}", s, width = max_len)
    }
}

#[deprecated(since = "0.1.0", note = "Use strip_controls instead")]
pub fn truncate(s: &str, max_len: usize) -> String {
    strip_controls(s, max_len)
}

#[deprecated(since = "0.1.0", note = "Use preserve_all instead")]
pub fn truncate_simple(s: &str, max_len: usize) -> String {
    preserve_all(s, max_len)
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
    fn test_preserve_all_short() {
        let result = preserve_all("hello", 10);
        assert_eq!(result, "hello     ");
    }

    #[test]
    fn test_preserve_all_long() {
        let result = preserve_all("hello world this is a test", 10);
        assert_eq!(result, "hello w...");
    }

    #[test]
    fn test_deprecated_truncate() {
        #[allow(deprecated)]
        let result = truncate("hello world", 10);
        assert_eq!(result, "hello w...");
    }

    #[test]
    fn test_deprecated_truncate_simple() {
        #[allow(deprecated)]
        let result = truncate_simple("hello world", 10);
        assert_eq!(result, "hello w...");
    }

    proptest! {
        #[test]
        fn test_strip_controls_never_exceeds_max_len(s in "\\PC{0,100}", max_len in 5usize..50) {
            let result = strip_controls(&s, max_len);
            prop_assert!(result.len() <= max_len);
        }

        #[test]
        fn test_preserve_all_never_exceeds_max_len(s in "[ -~]{0,100}", max_len in 5usize..50) {
            let result = preserve_all(&s, max_len);
            prop_assert!(result.len() <= max_len);
        }
    }
}
