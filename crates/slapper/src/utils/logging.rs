fn is_ansi_terminator(b: u8) -> bool {
    matches!(b, 0x40..=0x7E)
}

fn sanitize_bytes(input: &str, max_len: usize) -> String {
    let mut result = String::with_capacity(input.len().min(max_len));
    let mut char_count = 0;

    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];

        if b == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == 0x5B {
            let mut j = i + 2;
            while j < bytes.len() && !is_ansi_terminator(bytes[j]) {
                j += 1;
            }
            if j < bytes.len() {
                j += 1;
            }
            i = j;
            continue;
        }

        if (0x00..=0x1F).contains(&b) && b != 0x09 {
            i += 1;
            continue;
        }

        if char_count >= max_len {
            break;
        }

        let char_len = if b < 0x80 {
            1
        } else if b < 0xE0 {
            2
        } else if b < 0xF0 {
            3
        } else {
            4
        };

        if i + char_len <= bytes.len() {
            if let Ok(s) = std::str::from_utf8(&bytes[i..i + char_len]) {
                result.push_str(s);
                char_count += 1;
            }
        }

        i += char_len;
    }

    result
}

pub fn sanitize_for_logging(input: &str) -> String {
    sanitize_bytes(input, 500)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strips_ansi_escape_sequences() {
        let input = "\x1B[31mRed Text\x1B[0m Normal";
        let result = sanitize_for_logging(input);
        assert_eq!(result, "Red Text Normal");
    }

    #[test]
    fn test_strips_csi_sequences() {
        let input = "\x1B[1;2;3mBold\x1B[0m";
        let result = sanitize_for_logging(input);
        assert_eq!(result, "Bold");
    }

    #[test]
    fn test_preserves_tab_only() {
        let input = "Line1\nLine2\rLine3\tTab";
        let result = sanitize_for_logging(input);
        assert_eq!(result, "Line1Line2Line3\tTab");
    }

    #[test]
    fn test_strips_control_characters() {
        let input = "Start\x00\x01\x02\x03End";
        let result = sanitize_for_logging(input);
        assert_eq!(result, "StartEnd");
    }

    #[test]
    fn test_preserves_printable_chars() {
        let input = "Hello, World! 日本語 🎯";
        let result = sanitize_for_logging(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_truncates_long_input() {
        let input = "a".repeat(1000);
        let result = sanitize_for_logging(&input);
        assert_eq!(result.len(), 500);
    }

    #[test]
    fn test_default_max_length() {
        let input = "x".repeat(1000);
        let result = sanitize_for_logging(&input);
        assert_eq!(result.len(), 500);
    }

    #[test]
    fn test_empty_string() {
        let input = "";
        let result = sanitize_for_logging(input);
        assert_eq!(result, "");
    }

    #[test]
    fn test_mixed_ansi_and_controls() {
        let input = "\x1B[7mInverse\x1B[0m \x00\x07\x1B[2J Normal";
        let result = sanitize_for_logging(input);
        assert_eq!(result, "Inverse  Normal");
    }
}
