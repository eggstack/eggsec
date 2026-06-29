//! Lab manifest parser + validator for db-pentest (TOML-based allowlist).
//! Mirrors the advisory LabManifest pattern from mobile-dynamic (errors become action notes).

pub use crate::types::LabDbManifest;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lab_manifest_parse_and_match() {
        let toml = r#"
description = "lab postgres only"
allowed_hosts = ["127.0.0.1", "localhost", "lab-pg.internal"]
allowed_ports = [5432]
allowed_databases = ["lab", "test"]
max_queries_default = 150
"#;
        let m: LabDbManifest = toml::from_str(toml).unwrap();
        assert!(m.allows("127.0.0.1", 5432, "lab"));
        assert!(m.allows("localhost", 5432, "testdb")); // substring match on db
        assert!(!m.allows("evil.com", 5432, "lab"));
        assert!(!m.allows("127.0.0.1", 5433, "lab"));
    }

    #[test]
    fn empty_manifest_is_permissive() {
        let m = LabDbManifest::default();
        assert!(m.allows("any.host", 9999, "anydb"));
    }
}
