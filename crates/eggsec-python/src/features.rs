use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;

#[pyfunction]
pub fn features() -> HashMap<String, bool> {
    let mut map = HashMap::new();
    // Always available
    map.insert("core".to_string(), true);
    map.insert("scanner".to_string(), true);
    map.insert("async-api".to_string(), true);
    map.insert("endpoint-discovery".to_string(), true);
    map.insert("service-fingerprinting".to_string(), true);
    map.insert("waf-detection".to_string(), true);
    map.insert("waf-validation".to_string(), true);
    map.insert("http-fuzzing".to_string(), true);
    map.insert("load-testing".to_string(), true);
    map.insert("findings-reporting".to_string(), true);
    // Feature-gated
    map.insert("websocket".to_string(), cfg!(feature = "websocket"));
    map.insert("git-secrets".to_string(), cfg!(feature = "git-secrets"));
    map.insert("sbom".to_string(), cfg!(feature = "sbom"));
    map.insert("db-pentest".to_string(), cfg!(feature = "db-pentest"));
    map.insert(
        "db-pentest-mongodb".to_string(),
        cfg!(feature = "db-pentest-mongodb"),
    );
    map.insert(
        "db-pentest-redis".to_string(),
        cfg!(feature = "db-pentest-redis"),
    );
    map.insert("web-proxy".to_string(), cfg!(feature = "web-proxy"));
    map.insert("mobile".to_string(), cfg!(feature = "mobile"));
    map.insert(
        "mobile-dynamic".to_string(),
        cfg!(feature = "mobile-dynamic"),
    );
    map.insert(
        "packet-inspection".to_string(),
        cfg!(feature = "packet-inspection"),
    );
    map.insert(
        "stress-testing".to_string(),
        cfg!(feature = "stress-testing"),
    );
    map.insert("nse".to_string(), cfg!(feature = "nse"));
    map.insert("container".to_string(), cfg!(feature = "container"));
    map.insert("daemon-client".to_string(), cfg!(feature = "daemon-client"));
    map.insert(
        "headless-browser".to_string(),
        cfg!(feature = "headless-browser"),
    );
    map.insert(
        "advanced-hunting".to_string(),
        cfg!(feature = "advanced-hunting"),
    );
    map.insert("compliance".to_string(), cfg!(feature = "compliance"));
    map.insert("wireless".to_string(), cfg!(feature = "wireless"));
    map.insert("evasion".to_string(), cfg!(feature = "evasion"));
    map.insert("postex".to_string(), cfg!(feature = "postex"));
    map.insert("c2".to_string(), cfg!(feature = "c2"));
    map.insert(
        "ai-integration".to_string(),
        cfg!(feature = "ai-integration"),
    );
    map
}

#[pyfunction]
pub fn has_feature(name: &str) -> bool {
    match name {
        // Always available
        "core"
        | "scanner"
        | "async-api"
        | "endpoint-discovery"
        | "service-fingerprinting"
        | "waf-detection"
        | "waf-validation"
        | "http-fuzzing"
        | "load-testing"
        | "findings-reporting" => true,
        // Feature-gated
        "websocket" => cfg!(feature = "websocket"),
        "git-secrets" => cfg!(feature = "git-secrets"),
        "sbom" => cfg!(feature = "sbom"),
        "db-pentest" => cfg!(feature = "db-pentest"),
        "db-pentest-mongodb" => cfg!(feature = "db-pentest-mongodb"),
        "db-pentest-redis" => cfg!(feature = "db-pentest-redis"),
        "web-proxy" => cfg!(feature = "web-proxy"),
        "mobile" => cfg!(feature = "mobile"),
        "mobile-dynamic" => cfg!(feature = "mobile-dynamic"),
        "packet-inspection" => cfg!(feature = "packet-inspection"),
        "stress-testing" => cfg!(feature = "stress-testing"),
        "nse" => cfg!(feature = "nse"),
        "container" => cfg!(feature = "container"),
        "daemon-client" => cfg!(feature = "daemon-client"),
        "headless-browser" => cfg!(feature = "headless-browser"),
        "advanced-hunting" => cfg!(feature = "advanced-hunting"),
        "compliance" => cfg!(feature = "compliance"),
        "wireless" => cfg!(feature = "wireless"),
        "evasion" => cfg!(feature = "evasion"),
        "postex" => cfg!(feature = "postex"),
        "c2" => cfg!(feature = "c2"),
        "ai-integration" => cfg!(feature = "ai-integration"),
        _ => false,
    }
}

/// Returns a machine-readable dict of all features with availability, description,
/// and whether system dependencies are required.
#[pyfunction]
pub fn feature_matrix() -> PyObject {
    Python::with_gil(|py| {
        let dict = PyDict::new_bound(py);

        macro_rules! add_feature {
            ($name:expr, $available:expr, $desc:expr, $sys_deps:expr) => {
                let entry = PyDict::new_bound(py);
                entry.set_item("available", $available).unwrap();
                entry.set_item("description", $desc).unwrap();
                entry.set_item("requires_system_deps", $sys_deps).unwrap();
                dict.set_item($name, entry).unwrap();
            };
        }

        // Always available
        add_feature!("core", true, "Core engine types and configuration", false);
        add_feature!(
            "scanner",
            true,
            "Port scanning and service fingerprinting",
            false
        );
        add_feature!(
            "async-api",
            true,
            "Async wrappers for all operations",
            false
        );
        add_feature!(
            "endpoint-discovery",
            true,
            "HTTP endpoint discovery and probing",
            false
        );
        add_feature!(
            "service-fingerprinting",
            true,
            "Service version and technology fingerprinting",
            false
        );
        add_feature!(
            "waf-detection",
            true,
            "Web Application Firewall detection",
            false
        );
        add_feature!(
            "waf-validation",
            true,
            "WAF bypass validation and testing",
            false
        );
        add_feature!(
            "http-fuzzing",
            true,
            "HTTP parameter and header fuzzing",
            false
        );
        add_feature!(
            "load-testing",
            true,
            "HTTP load testing and benchmarking",
            false
        );
        add_feature!(
            "findings-reporting",
            true,
            "Finding storage, correlation, and report generation",
            false
        );

        // Feature-gated
        add_feature!(
            "websocket",
            cfg!(feature = "websocket"),
            "WebSocket security testing",
            false
        );
        add_feature!(
            "git-secrets",
            cfg!(feature = "git-secrets"),
            "Git repository secret detection",
            false
        );
        add_feature!(
            "sbom",
            cfg!(feature = "sbom"),
            "Software Bill of Materials generation",
            false
        );
        add_feature!(
            "db-pentest",
            cfg!(feature = "db-pentest"),
            "Database penetration testing (Postgres/MySQL/MSSQL)",
            false
        );
        add_feature!(
            "db-pentest-mongodb",
            cfg!(feature = "db-pentest-mongodb"),
            "MongoDB penetration testing",
            false
        );
        add_feature!(
            "db-pentest-redis",
            cfg!(feature = "db-pentest-redis"),
            "Redis penetration testing",
            false
        );
        add_feature!(
            "web-proxy",
            cfg!(feature = "web-proxy"),
            "HTTP/HTTPS interception proxy",
            false
        );
        add_feature!(
            "mobile",
            cfg!(feature = "mobile"),
            "Mobile app static analysis (APK/IPA)",
            false
        );
        add_feature!(
            "mobile-dynamic",
            cfg!(feature = "mobile-dynamic"),
            "Android dynamic analysis via ADB",
            true
        );
        add_feature!(
            "packet-inspection",
            cfg!(feature = "packet-inspection"),
            "Packet capture and network analysis",
            true
        );
        add_feature!(
            "stress-testing",
            cfg!(feature = "stress-testing"),
            "Network stress testing and DoS simulation",
            false
        );
        add_feature!(
            "nse",
            cfg!(feature = "nse"),
            "Nmap NSE script execution",
            true
        );
        add_feature!(
            "container",
            cfg!(feature = "container"),
            "Docker and Kubernetes security scanning",
            false
        );
        add_feature!(
            "daemon-client",
            cfg!(feature = "daemon-client"),
            "Daemon session management client",
            false
        );
        add_feature!(
            "headless-browser",
            cfg!(feature = "headless-browser"),
            "Headless browser security testing",
            true
        );
        add_feature!(
            "advanced-hunting",
            cfg!(feature = "advanced-hunting"),
            "Advanced vulnerability hunting (chains, race conditions)",
            false
        );
        add_feature!(
            "compliance",
            cfg!(feature = "compliance"),
            "Compliance framework mapping (OWASP, NIST, etc.)",
            false
        );
        add_feature!(
            "wireless",
            cfg!(feature = "wireless"),
            "WiFi network scanning and analysis",
            true
        );
        add_feature!(
            "evasion",
            cfg!(feature = "evasion"),
            "Evasion technique detection and validation",
            false
        );
        add_feature!(
            "postex",
            cfg!(feature = "postex"),
            "Post-exploitation simulation",
            false
        );
        add_feature!("c2", cfg!(feature = "c2"), "C2 framework simulation", false);
        add_feature!(
            "ai-integration",
            cfg!(feature = "ai-integration"),
            "AI-assisted finding analysis and payload generation",
            false
        );

        dict.into()
    })
}
