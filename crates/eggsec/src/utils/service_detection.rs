use rustc_hash::FxHashMap;
use std::sync::LazyLock;

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

pub static PORT_SERVICE_MAP: LazyLock<FxHashMap<u16, &'static str>> =
    LazyLock::new(|| COMMON_PORTS.iter().cloned().collect());

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_service_name_known_port() {
        assert_eq!(get_service_name(80), "HTTP");
        assert_eq!(get_service_name(443), "HTTPS");
        assert_eq!(get_service_name(22), "SSH");
        assert_eq!(get_service_name(3306), "MySQL");
    }

    #[test]
    fn test_get_service_name_unknown_port() {
        assert_eq!(get_service_name(12345), "unknown");
        assert_eq!(get_service_name(0), "unknown");
    }

    #[test]
    fn test_get_service_by_port_some() {
        assert_eq!(get_service_by_port(21), Some("FTP"));
        assert_eq!(get_service_by_port(53), Some("DNS"));
    }

    #[test]
    fn test_get_service_by_port_none() {
        assert_eq!(get_service_by_port(12345), None);
    }

    #[test]
    fn test_guess_service_from_banner_ssh() {
        assert_eq!(
            guess_service_from_banner("SSH-2.0-OpenSSH_8.9 version 8.9"),
            Some("SSH")
        );
    }

    #[test]
    fn test_guess_service_from_banner_mysql() {
        assert_eq!(
            guess_service_from_banner("5.7.35 MySQL Community Server"),
            Some("MySQL")
        );
        assert_eq!(guess_service_from_banner("MariaDB-10.5"), Some("MySQL"));
    }

    #[test]
    fn test_guess_service_from_banner_redis() {
        assert_eq!(
            guess_service_from_banner("Redis server v=6.2.6"),
            Some("Redis")
        );
    }

    #[test]
    fn test_guess_service_from_banner_postgresql() {
        assert_eq!(
            guess_service_from_banner("PostgreSQL 14.1"),
            Some("PostgreSQL")
        );
    }

    #[test]
    fn test_guess_service_from_banner_mongodb() {
        assert_eq!(guess_service_from_banner("MongoDB 5.0"), Some("MongoDB"));
    }

    #[test]
    fn test_guess_service_from_banner_http() {
        assert_eq!(guess_service_from_banner("HTTP/1.1 200 OK"), Some("HTTP"));
        assert_eq!(guess_service_from_banner("nginx/1.21.0"), Some("HTTP"));
        assert_eq!(guess_service_from_banner("Apache/2.4.51"), Some("HTTP"));
        assert_eq!(
            guess_service_from_banner("Microsoft-IIS/10.0"),
            Some("HTTP")
        );
    }

    #[test]
    fn test_guess_service_from_banner_smb() {
        assert_eq!(guess_service_from_banner("Samba 4.15"), Some("SMB"));
        assert_eq!(guess_service_from_banner("microsoft-ds"), Some("SMB"));
    }

    #[test]
    fn test_guess_service_from_banner_none() {
        assert_eq!(guess_service_from_banner("random garbage xyz"), None);
        assert_eq!(guess_service_from_banner(""), None);
    }

    #[test]
    fn test_guess_service_from_banner_case_insensitive() {
        assert_eq!(
            guess_service_from_banner("ssh-2.0-openssh version 8.9"),
            Some("SSH")
        );
        assert_eq!(guess_service_from_banner("REDIS SERVER"), Some("Redis"));
    }

    #[test]
    fn test_guess_service_with_banner_match() {
        assert_eq!(guess_service(22, Some("SSH-2.0-OpenSSH")), "SSH");
    }

    #[test]
    fn test_guess_service_with_banner_no_match_fallback_to_port() {
        assert_eq!(guess_service(80, Some("unknown banner")), "HTTP");
    }

    #[test]
    fn test_guess_service_no_banner() {
        assert_eq!(guess_service(443, None), "HTTPS");
        assert_eq!(guess_service(9999, None), "unknown");
    }

    #[test]
    fn test_is_web_service() {
        assert!(is_web_service(80));
        assert!(is_web_service(443));
        assert!(is_web_service(8080));
        assert!(is_web_service(3000));
        assert!(!is_web_service(22));
        assert!(!is_web_service(3306));
    }

    #[test]
    fn test_is_database() {
        assert!(is_database(3306));
        assert!(is_database(5432));
        assert!(is_database(6379));
        assert!(is_database(27017));
        assert!(!is_database(80));
        assert!(!is_database(22));
    }

    #[test]
    fn test_is_mail_service() {
        assert!(is_mail_service(25));
        assert!(is_mail_service(587));
        assert!(is_mail_service(993));
        assert!(!is_mail_service(80));
        assert!(!is_mail_service(443));
    }

    #[test]
    fn test_common_ports_not_empty() {
        assert!(!COMMON_PORTS.is_empty());
    }

    #[test]
    fn test_port_service_map_populated() {
        assert!(!PORT_SERVICE_MAP.is_empty());
        assert_eq!(PORT_SERVICE_MAP.get(&80), Some(&"HTTP"));
    }
}
