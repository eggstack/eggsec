use once_cell::sync::Lazy;
use std::collections::HashMap;

pub static COMMON_PORTS: &[(u16, &str)] = &[
    (21, "FTP"),
    (22, "SSH"),
    (23, "Telnet"),
    (25, "SMTP"),
    (53, "DNS"),
    (67, "DHCP"),
    (68, "DHCP"),
    (69, "TFTP"),
    (80, "HTTP"),
    (110, "POP3"),
    (119, "NNTP"),
    (123, "NTP"),
    (135, "MSRPC"),
    (137, "NetBIOS"),
    (138, "NetBIOS"),
    (139, "NetBIOS"),
    (143, "IMAP"),
    (161, "SNMP"),
    (162, "SNMPTRAP"),
    (389, "LDAP"),
    (443, "HTTPS"),
    (445, "SMB"),
    (465, "SMTPS"),
    (514, "Syslog"),
    (515, "LPD"),
    (587, "SMTP"),
    (636, "LDAPS"),
    (993, "IMAPS"),
    (995, "POP3S"),
    (1080, "SOCKS"),
    (1433, "MSSQL"),
    (1521, "Oracle"),
    (1723, "PPTP"),
    (2049, "NFS"),
    (3306, "MySQL"),
    (3389, "RDP"),
    (5432, "PostgreSQL"),
    (5900, "VNC"),
    (5901, "VNC"),
    (6379, "Redis"),
    (8080, "HTTP-Alt"),
    (8443, "HTTPS-Alt"),
    (8888, "HTTP-Alt"),
    (9000, "SonarQube"),
    (9001, "Tor"),
    (9200, "Elasticsearch"),
    (27017, "MongoDB"),
];

pub static PORT_SERVICE_MAP: Lazy<HashMap<u16, &'static str>> =
    Lazy::new(|| COMMON_PORTS.iter().cloned().collect());

pub fn get_service_name(port: u16) -> &'static str {
    PORT_SERVICE_MAP.get(&port).copied().unwrap_or("unknown")
}

pub fn get_service_by_port(port: u16) -> Option<&'static str> {
    PORT_SERVICE_MAP.get(&port).copied()
}

pub fn guess_service_from_banner(banner: &str) -> Option<&'static str> {
    let lower = banner.to_lowercase();

    if lower.contains("ssh") && lower.contains("version") {
        return Some("SSH");
    }
    if lower.contains("mysql") || lower.contains("mariadb") {
        return Some("MySQL");
    }
    if lower.contains("redis") {
        return Some("Redis");
    }
    if lower.contains("postgresql") || lower.contains("postgres") {
        return Some("PostgreSQL");
    }
    if lower.contains("mongodb") {
        return Some("MongoDB");
    }
    if lower.contains("ftp") {
        return Some("FTP");
    }
    if lower.contains("smtp") || lower.contains("mail") {
        return Some("SMTP");
    }
    if lower.contains("imap") {
        return Some("IMAP");
    }
    if lower.contains("pop") {
        return Some("POP3");
    }
    if lower.contains("http")
        || lower.contains("nginx")
        || lower.contains("apache")
        || lower.contains("iis")
    {
        return Some("HTTP");
    }
    if lower.contains("ldap") {
        return Some("LDAP");
    }
    if lower.contains("smb") || lower.contains("samba") || lower.contains("microsoft-ds") {
        return Some("SMB");
    }
    if lower.contains("rdp") || lower.contains("terminal") {
        return Some("RDP");
    }
    if lower.contains("vnc") {
        return Some("VNC");
    }

    None
}

pub fn guess_service(port: u16, banner: Option<&str>) -> String {
    if let Some(banner) = banner {
        if let Some(service) = guess_service_from_banner(banner) {
            return service.to_string();
        }
    }
    get_service_name(port).to_string()
}

pub fn is_web_service(port: u16) -> bool {
    matches!(
        port,
        80 | 443 | 8080 | 8443 | 8888 | 8000 | 3000 | 5000 | 9000
    )
}

pub fn is_database(port: u16) -> bool {
    matches!(port, 1433 | 1521 | 3306 | 5432 | 6379 | 27017 | 9200)
}

pub fn is_mail_service(port: u16) -> bool {
    matches!(port, 25 | 110 | 143 | 465 | 587 | 993 | 995)
}
