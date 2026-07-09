use pyo3::prelude::*;
use serde_json::json;

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
