pub fn truncate(s: &str, max_len: usize) -> String {
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

pub fn truncate_simple(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    } else {
        format!("{:<width$}", s, width = max_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_truncate_short_string() {
        let result = truncate("hello", 10);
        assert_eq!(result, "hello     ");
    }

    #[test]
    fn test_truncate_exact_length() {
        let result = truncate("hello", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        let result = truncate("hello world this is a test", 10);
        assert_eq!(result, "hello w...");
    }

    #[test]
    fn test_truncate_removes_control_chars() {
        let result = truncate("hello\x00world", 20);
        assert_eq!(result, "helloworld          ");
    }

    #[test]
    fn test_truncate_simple_short() {
        let result = truncate_simple("hello", 10);
        assert_eq!(result, "hello     ");
    }

    #[test]
    fn test_truncate_simple_long() {
        let result = truncate_simple("hello world this is a test", 10);
        assert_eq!(result, "hello w...");
    }

    proptest! {
        #[test]
        fn test_truncate_never_exceeds_max_len(s in "\\PC{0,100}", max_len in 5usize..50) {
            let result = truncate(&s, max_len);
            prop_assert!(result.len() <= max_len);
        }

        #[test]
        fn test_truncate_simple_never_exceeds_max_len(s in "[ -~]{0,100}", max_len in 5usize..50) {
            let result = truncate_simple(&s, max_len);
            prop_assert!(result.len() <= max_len);
        }
    }
}
