use subtle::ConstantTimeEq;

pub fn constant_time_eq(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_time_eq_same() {
        assert!(constant_time_eq("test", "test"));
        assert!(constant_time_eq("", ""));
        assert!(constant_time_eq("hello world", "hello world"));
    }

    #[test]
    fn test_constant_time_eq_different() {
        assert!(!constant_time_eq("test", "other"));
        assert!(!constant_time_eq("test", "TEST"));
        assert!(!constant_time_eq("hello", "world"));
    }

    #[test]
    fn test_constant_time_eq_different_lengths() {
        assert!(!constant_time_eq("short", "longer string"));
        assert!(!constant_time_eq("longer string", "short"));
    }
}
