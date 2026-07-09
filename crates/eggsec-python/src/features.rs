use pyo3::prelude::*;
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
        _ => false,
    }
}
