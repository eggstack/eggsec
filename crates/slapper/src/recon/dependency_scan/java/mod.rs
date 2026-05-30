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
        let mut depth = 0;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("<!--") {
                continue;
            }

            if trimmed.starts_with("<properties>") {
                in_properties = true;
                continue;
            }
            if trimmed.starts_with("</properties>") {
                in_properties = false;
                continue;
            }

            if in_properties {
                if let Some((key, value)) = Self::parse_property(trimmed) {
                    property_versions.insert(key, value);
                }
            }

            if trimmed.starts_with("<dependency>") {
                depth += 1;
            }
            if trimmed.starts_with("</dependency>") {
                depth -= 1;
            }

            if depth > 0 && trimmed.starts_with("<groupId>") {
                if let Some(dep) = Self::extract_dep_block(&content, &mut dependencies, &property_versions) {
                    dependencies.push(dep);
                }
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

    fn parse_property(line: &str) -> Option<(String, String)> {
        if line.starts_with('<') && line.contains("</") {
            let rest = &line[1..];
            if let Some(end_pos) = rest.find("</") {
                let tag_and_value = &rest[..end_pos];
                if let Some(eq_pos) = tag_and_value.find('>') {
                    let key = tag_and_value[..eq_pos].to_string();
                    let value = tag_and_value[eq_pos + 1..].trim().to_string();
                    if !key.is_empty() && !value.is_empty() {
                        return Some((key, value));
                    }
                }
            }
        }
        None
    }

    fn extract_dep_block(content: &str, deps: &mut Vec<DependencyInfo>, props: &HashMap<String, String>) -> Option<DependencyInfo> {
        let start = content.find("<groupId>")?;
        let end = content.find("</dependency>")?;
        let block = &content[start..end.min(start + 2000)];

        let group_id = Self::extract_xml_value(block, "groupId")?;
        let artifact_id = Self::extract_xml_value(block, "artifactId")?;
        let version = Self::extract_xml_value(block, "version");

        let name = format!("{}:{}", group_id, artifact_id);

        let resolved_version = version.and_then(|v| {
            if v.starts_with("${") && v.ends_with("}") {
                let prop_name = &v[2..v.len() - 1];
                props.get(prop_name).cloned()
            } else {
                Some(v)
            }
        }).unwrap_or_else(|| "*".to_string());

        Some(DependencyInfo {
            name,
            version: resolved_version,
            is_direct: true,
        })
    }

    fn extract_xml_value(block: &str, tag: &str) -> Option<String> {
        let open_tag = format!("<{}>", tag);
        let close_tag = format!("</{}>", tag);

        let start = block.find(&open_tag)? + open_tag.len();
        let end = block.find(&close_tag)?;

        let value = block[start..end].trim().to_string();
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    }
}