use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde_json::json;

/// Schema version constant (exposed to Python).
pub const SCHEMA_VERSION: &str = "1.0";

/// Daemon/gRPC protocol version constant (exposed to Python).
pub const PROTOCOL_VERSION: &str = "1.0.0";

/// Native ABI version constant (exposed to Python).
pub const ABI_VERSION: &str = "1";

#[pyfunction]
pub fn build_info() -> PyObject {
    let info = json!({
        "version": env!("CARGO_PKG_VERSION"),
        "rust_crate_version": env!("CARGO_PKG_VERSION"),
        "package_name": env!("CARGO_PKG_NAME"),
        "target_triple": std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string()),
        "binding_version": "0.1.0",
    });
    Python::with_gil(|py| {
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
