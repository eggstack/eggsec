use crate::error::Result;
use crate::recon::dependency_scan::{DependencyEcosystem, DependencyInfo};
use std::collections::HashMap;
use std::path::Path;

pub struct JavaScanner;

impl Default for JavaScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaScanner {
    pub fn new() -> Self {
        Self
    }

    pub fn scan_pom_xml(path: &Path) -> Result<DependencyEcosystem> {
        let content = std::fs::read_to_string(path)?;
        let mut dependencies = Vec::new();
        let mut property_versions: HashMap<String, String> = HashMap::new();

        let mut in_properties = false;
        let mut in_dependency = false;
        let mut current_group_id: Option<String> = None;
        let mut current_artifact_id: Option<String> = None;
        let mut current_version: Option<String> = None;
        let mut in_tag = false;
        let mut current_tag = String::new();
        let mut tag_content = String::new();

        for ch in content.chars() {
            if ch == '<' {
                in_tag = true;
                current_tag.clear();
                tag_content.clear();
            } else if ch == '>' && in_tag {
                in_tag = false;
                let tag_lower = current_tag.to_lowercase();

                match tag_lower.as_str() {
                    "properties" => in_properties = true,
                    "/properties" => in_properties = false,
                    "dependency" => {
                        in_dependency = true;
                        current_group_id = None;
                        current_artifact_id = None;
                        current_version = None;
                    }
                    "/dependency" => {
                        if let (Some(g), Some(a)) = (&current_group_id, &current_artifact_id) {
                            let name = format!("{}:{}", g, a);
                            let resolved_version = current_version
                                .as_ref()
                                .and_then(|v| {
                                    if v.starts_with("${") && v.ends_with("}") {
                                        let prop_name = &v[2..v.len() - 1];
                                        property_versions.get(prop_name).cloned()
                                    } else {
                                        Some(v.clone())
                                    }
                                })
                                .unwrap_or_else(|| "*".to_string());

                            dependencies.push(DependencyInfo {
                                name,
                                version: resolved_version,
                                is_direct: true,
                            });
                        }
                        in_dependency = false;
                    }
                    "groupId" | "artifactId" | "version" => {
                        if in_properties {
                            if !current_tag.starts_with("project.") && !current_tag.starts_with("maven.") {
                                let trimmed = tag_content.trim();
                                if !trimmed.is_empty() {
                                    property_versions.insert(current_tag.clone(), trimmed.to_string());
                                }
                            }
                        } else if in_dependency {
                            match current_tag.as_str() {
                                "groupId" => current_group_id = Some(tag_content.trim().to_string()),
                                "artifactId" => current_artifact_id = Some(tag_content.trim().to_string()),
                                "version" => current_version = Some(tag_content.trim().to_string()),
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            } else if in_tag {
                current_tag.push(ch);
            } else {
                tag_content.push(ch);
            }
        }

        Ok(DependencyEcosystem {
            name: "Java (Maven)".to_string(),
            manifest_file: path.to_string_lossy().to_string(),
            lock_file: None,
            dependencies,
            vulnerabilities: Vec::new(),
        })
    }
}