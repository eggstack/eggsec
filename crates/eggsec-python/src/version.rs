use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde_json::json;

/// Schema version constant (exposed to Python).
pub const SCHEMA_VERSION: &str = "1.0";

/// Daemon/gRPC protocol version constant (exposed to Python).
pub const PROTOCOL_VERSION: &str = "1.0.0";

/// Native ABI version constant (exposed to Python).
pub const ABI_VERSION: &str = "1";

/// Returns the inferred wheel profile name based on compiled features.
///
/// - `"core"` — no feature-gated features are compiled in.
/// - `"full-no-system"` — websocket, git-secrets, sbom, and container are enabled,
///   with no other feature-gated features.
/// - `"custom"` — any other combination.
#[pyfunction]
pub fn wheel_profile() -> String {
    let feature_gated: Vec<(&str, bool)> = vec![
        ("websocket", cfg!(feature = "websocket")),
        ("git-secrets", cfg!(feature = "git-secrets")),
        ("sbom", cfg!(feature = "sbom")),
        ("db-pentest", cfg!(feature = "db-pentest")),
        ("db-pentest-mongodb", cfg!(feature = "db-pentest-mongodb")),
        ("db-pentest-redis", cfg!(feature = "db-pentest-redis")),
        ("web-proxy", cfg!(feature = "web-proxy")),
        ("mobile", cfg!(feature = "mobile")),
        ("mobile-dynamic", cfg!(feature = "mobile-dynamic")),
        ("packet-inspection", cfg!(feature = "packet-inspection")),
        ("stress-testing", cfg!(feature = "stress-testing")),
        ("nse", cfg!(feature = "nse")),
        ("container", cfg!(feature = "container")),
        ("daemon-client", cfg!(feature = "daemon-client")),
        ("headless-browser", cfg!(feature = "headless-browser")),
        ("advanced-hunting", cfg!(feature = "advanced-hunting")),
        ("compliance", cfg!(feature = "compliance")),
        ("wireless", cfg!(feature = "wireless")),
        ("evasion", cfg!(feature = "evasion")),
        ("postex", cfg!(feature = "postex")),
        ("c2", cfg!(feature = "c2")),
        ("ai-integration", cfg!(feature = "ai-integration")),
    ];

    let any_enabled = feature_gated.iter().any(|(_, enabled)| *enabled);
    if !any_enabled {
        return "core".to_string();
    }

    let full_no_system = vec!["websocket", "git-secrets", "sbom", "container"];
    let is_full_no_system = feature_gated.iter().all(|(name, enabled)| {
        if full_no_system.contains(name) {
            *enabled
        } else {
            !enabled
        }
    });
    if is_full_no_system {
        return "full-no-system".to_string();
    }

    "custom".to_string()
}

#[pyfunction]
pub fn build_info() -> PyObject {
    let compiled_features: Vec<String> = [
        ("websocket", cfg!(feature = "websocket")),
        ("git-secrets", cfg!(feature = "git-secrets")),
        ("sbom", cfg!(feature = "sbom")),
        ("db-pentest", cfg!(feature = "db-pentest")),
        ("db-pentest-mongodb", cfg!(feature = "db-pentest-mongodb")),
        ("db-pentest-redis", cfg!(feature = "db-pentest-redis")),
        ("web-proxy", cfg!(feature = "web-proxy")),
        ("mobile", cfg!(feature = "mobile")),
        ("mobile-dynamic", cfg!(feature = "mobile-dynamic")),
        ("packet-inspection", cfg!(feature = "packet-inspection")),
        ("stress-testing", cfg!(feature = "stress-testing")),
        ("nse", cfg!(feature = "nse")),
        ("container", cfg!(feature = "container")),
        ("daemon-client", cfg!(feature = "daemon-client")),
        ("headless-browser", cfg!(feature = "headless-browser")),
        ("advanced-hunting", cfg!(feature = "advanced-hunting")),
        ("compliance", cfg!(feature = "compliance")),
        ("wireless", cfg!(feature = "wireless")),
        ("evasion", cfg!(feature = "evasion")),
        ("postex", cfg!(feature = "postex")),
        ("c2", cfg!(feature = "c2")),
        ("ai-integration", cfg!(feature = "ai-integration")),
    ]
    .iter()
    .filter(|(_, enabled)| *enabled)
    .map(|(name, _)| name.to_string())
    .collect();

    Python::with_gil(|py| {
        let python_version = py
            .import_bound("sys")
            .and_then(|sys| sys.getattr("version"))
            .and_then(|v| v.extract::<String>())
            .unwrap_or_else(|_| "unknown".to_string());

        let info = json!({
            "version": env!("CARGO_PKG_VERSION"),
            "rust_crate_version": env!("CARGO_PKG_VERSION"),
            "package_name": env!("CARGO_PKG_NAME"),
            "target_triple": std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string()),
            "binding_version": "0.1.0",
            "schema_version": SCHEMA_VERSION,
            "protocol_version": PROTOCOL_VERSION,
            "abi_version": ABI_VERSION,
            "python_version": python_version,
            "compiled_features": compiled_features,
            "wheel_profile": wheel_profile(),
        });

        let json_str = info.to_string();
        py.import_bound("json")
            .expect("json module not available")
            .call_method1("loads", (json_str,))
            .expect("json.loads failed")
            .into()
    })
}

/// Returns a machine-readable dict with package, schema, protocol, and ABI versions,
/// plus the list of available feature names.
#[pyfunction]
pub fn api_surface_version() -> PyObject {
    Python::with_gil(|py| {
        let dict = PyDict::new_bound(py);

        dict.set_item("package_version", env!("CARGO_PKG_VERSION"))
            .expect("set_item failed");
        dict.set_item("schema_version", SCHEMA_VERSION)
            .expect("set_item failed");
        dict.set_item("protocol_version", PROTOCOL_VERSION)
            .expect("set_item failed");
        dict.set_item("abi_version", ABI_VERSION)
            .expect("set_item failed");

        let features = PyList::empty_bound(py);
        let feature_names: Vec<&str> = vec![
            "core",
            "scanner",
            "async-api",
            "endpoint-discovery",
            "service-fingerprinting",
            "waf-detection",
            "waf-validation",
            "http-fuzzing",
            "load-testing",
            "findings-reporting",
            "websocket",
            "git-secrets",
            "sbom",
            "db-pentest",
            "db-pentest-mongodb",
            "db-pentest-redis",
            "web-proxy",
            "mobile",
            "mobile-dynamic",
            "packet-inspection",
            "stress-testing",
            "nse",
            "container",
            "daemon-client",
            "headless-browser",
            "advanced-hunting",
            "compliance",
            "wireless",
            "evasion",
            "postex",
            "c2",
            "ai-integration",
        ];
        for name in feature_names {
            features.append(name).expect("append failed");
        }
        dict.set_item("features_list", features)
            .expect("set_item failed");

        dict.into()
    })
}
