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
