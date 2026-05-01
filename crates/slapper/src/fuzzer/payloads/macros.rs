//! Macros for reducing payload generation boilerplate.

/// Build a `Vec<Payload>` from inline data tuples grouped by tag.
///
/// Each group section specifies a tag and a list of `(payload_str, description, severity)` tuples.
///
/// # Example
///
/// ```ignore
/// use slapper::payload_vec;
///
/// let payloads = payload_vec!(PayloadType::Sqli,
///     "basic", [
///         ("' OR 1=1--", "Classic OR", Severity::Critical),
///         ("\" OR \"\"=\"", "Double quote", Severity::High),
///     ];
///     "error", [
///         ("' AND 1=CONVERT(int,...)--", "MSSQL CONVERT", Severity::High),
///     ];
/// );
/// ```
macro_rules! payload_vec {
    ($pt:expr, $($tag:expr, [ $( ($payload:expr, $desc:expr, $sev:expr) ),* $(,)? ]);+ $(;)?) => {{
        #[allow(clippy::vec_init_then_push)]
        {
            let mut v: Vec<$crate::fuzzer::payloads::Payload> = Vec::with_capacity(64);
            $(
                $(
                    v.push($crate::fuzzer::payloads::Payload {
                        payload_type: $pt,
                        payload: $payload.to_string(),
                        description: $desc.to_string(),
                        severity: $sev,
                        tags: vec![$tag.to_string()],
                    });
                )*
            )+
            v
        }
    }};
}

#[cfg(test)]
mod tests {
    use crate::fuzzer::payloads::{PayloadType, Severity};

    #[test]
    fn test_payload_vec_macro() {
        let payloads = payload_vec!(PayloadType::Xss,
            "test", [
                ("<script>alert(1)</script>", "Basic XSS", Severity::High),
                ("\" onmouseover=\"alert(1)", "Attribute XSS", Severity::High),
            ];
        );
        assert_eq!(payloads.len(), 2);
        assert!(payloads[0].payload.contains("<script>"));
        assert!(payloads[1].payload.contains("onmouseover"));
        assert_eq!(payloads[0].payload_type, PayloadType::Xss);
        assert_eq!(payloads[1].payload_type, PayloadType::Xss);
    }

    #[test]
    fn test_payload_vec_macro_single_group() {
        let payloads = payload_vec!(PayloadType::Sqli,
            "basic", [
                ("' OR 1=1--", "Classic OR", Severity::Critical),
            ];
        );
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].payload, "' OR 1=1--");
    }

    #[test]
    fn test_payload_vec_macro_multiple_groups() {
        let payloads = payload_vec!(PayloadType::Headers,
            "group1", [
                ("header1", "First", Severity::Low),
            ];
            "group2", [
                ("header2", "Second", Severity::Medium),
                ("header3", "Third", Severity::High),
            ];
        );
        assert_eq!(payloads.len(), 3);
        assert_eq!(payloads[0].tags, vec!["group1"]);
        assert_eq!(payloads[1].tags, vec!["group2"]);
        assert_eq!(payloads[2].tags, vec!["group2"]);
    }
}
