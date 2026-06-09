//! Feature surface audit test.
//!
//! Ensures every `#[cfg(feature = "...")]` in the source tree references a
//! feature declared in `Cargo.toml`. Catches accidentally undeclared features
//! early.

use std::collections::HashSet;
use std::path::Path;

#[test]
fn all_cfg_features_are_declared_in_manifest() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let manifest_path = format!("{}/Cargo.toml", manifest_dir);
    let manifest = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", manifest_path, e));

    let declared = extract_declared_features(&manifest);

    let src_dir = format!("{}/src", manifest_dir);
    let used = extract_used_features(&src_dir);

    let mut undeclared = Vec::new();
    for feature in &used {
        if !declared.contains(feature.as_str()) {
            undeclared.push(feature.clone());
        }
    }

    undeclared.sort();
    assert!(
        undeclared.is_empty(),
        "Features used in #[cfg(feature = \"...\")] but not declared in Cargo.toml: {:?}",
        undeclared
    );
}

/// Parse `[features]` section of a Cargo.toml and return the declared feature names.
fn extract_declared_features(toml: &str) -> HashSet<String> {
    let mut features = HashSet::new();
    let mut in_features = false;

    for line in toml.lines() {
        let trimmed = line.trim();

        if trimmed == "[features]" || trimmed == "[features ]" {
            in_features = true;
            continue;
        }

        if in_features && trimmed.starts_with('[') && !trimmed.starts_with("[features") {
            in_features = false;
            continue;
        }

        if in_features && !trimmed.is_empty() && !trimmed.starts_with('#') {
            if let Some(name) = trimmed.split('=').next() {
                let name = name.trim().to_string();
                if !name.is_empty() {
                    features.insert(name);
                }
            }
        }
    }

    features
}

/// Walk `src_dir` and extract every feature name referenced in `cfg(feature = "...")`.
fn extract_used_features(src_dir: &str) -> HashSet<String> {
    let mut features = HashSet::new();
    let pattern = regex::Regex::new(r#"cfg\(feature\s*=\s*"([^"]+)""#).unwrap();

    walk_dir(Path::new(src_dir), &pattern, &mut features);
    features
}

fn walk_dir(dir: &Path, pattern: &regex::Regex, features: &mut HashSet<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, pattern, features);
            continue;
        }

        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for cap in pattern.captures_iter(&content) {
            if let Some(m) = cap.get(1) {
                features.insert(m.as_str().to_string());
            }
        }
    }
}
