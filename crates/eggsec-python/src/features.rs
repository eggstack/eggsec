use pyo3::prelude::*;
use std::collections::HashMap;

#[pyfunction]
pub fn features() -> HashMap<String, bool> {
    let mut map = HashMap::new();
    map.insert("core".to_string(), true);
    map.insert("scanner".to_string(), true);
    map.insert("async-api".to_string(), true);
    map.insert("endpoint-discovery".to_string(), true);
    map.insert("service-fingerprinting".to_string(), true);
    map.insert("nse".to_string(), false);
    map.insert("stress-testing".to_string(), false);
    map.insert("packet-inspection".to_string(), false);
    map.insert("headless-browser".to_string(), false);
    map.insert("database".to_string(), false);
    map.insert("cloud".to_string(), false);
    map.insert("sbom".to_string(), false);
    map.insert("websocket".to_string(), false);
    map
}

#[pyfunction]
pub fn has_feature(name: &str) -> bool {
    match name {
        "core" | "scanner" | "async-api" | "endpoint-discovery" | "service-fingerprinting" => true,
        _ => false,
    }
}
