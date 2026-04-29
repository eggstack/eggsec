//! Skill system for the security agent.
//!
//! Loads and manages YAML+Markdown skill files that define agent capabilities.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ai-integration")]
use semver::{Version, VersionReq};

#[derive(Debug, Deserialize, Serialize)]
struct SkillFrontmatter {
    name: String,
    description: String,
    triggers: Vec<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    metadata: Option<SkillMetadata>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkillMetadata {
    pub category: Option<String>,
    pub tools: Vec<String>,
    pub scope: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub triggers: Vec<String>,
    pub content: String,
    pub version: Option<String>,
    pub metadata: Option<SkillMetadata>,
}

impl Skill {
    pub fn parse(path: PathBuf) -> Result<Self> {
        let content = fs::read_to_string(&path)?;
        let (frontmatter, content) = extract_frontmatter(&content)?;

        let fm: SkillFrontmatter = serde_yaml_neo::from_str(&frontmatter)
            .map_err(|e| anyhow::anyhow!("Failed to parse frontmatter: {}", e))?;

        let version = fm.version.or(fm.metadata.as_ref().and_then(|m| m.version.clone()));

        let skill = Skill {
            name: fm.name,
            description: fm.description,
            triggers: fm.triggers,
            content,
            version,
            metadata: fm.metadata,
        };

        Ok(skill)
    }

    fn validate_triggers(&self) -> Result<()> {
        if self.triggers.is_empty() {
            return Err(anyhow::anyhow!("Skill '{}' has no triggers", self.name));
        }
        for trigger in &self.triggers {
            if trigger.len() < 2 {
                return Err(anyhow::anyhow!(
                    "Skill '{}' trigger '{}' is too short (min 2 chars)",
                    self.name,
                    trigger
                ));
            }
        }
        Ok(())
    }

    fn validate_version(&self) -> Result<()> {
        if let Some(ref version) = self.version {
            if !Self::is_valid_version(version) {
                return Err(anyhow::anyhow!(
                    "Skill '{}' has invalid version format: {}",
                    self.name,
                    version
                ));
            }
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_triggers()?;
        self.validate_version()?;
        Ok(())
    }

    pub fn is_valid_version(version: &str) -> bool {
        #[cfg(feature = "ai-integration")]
        {
            if Version::parse(version).is_ok() {
                return true;
            }
        }
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() < 2 || parts.len() > 3 {
            return false;
        }
        parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()))
    }

    pub fn is_compatible_with(&self, required_version: &str) -> bool {
        #[cfg(feature = "ai-integration")]
        {
            if let Some(ref self_version) = self.version {
                if let (Ok(sv), Ok(rv)) = (Version::parse(self_version), VersionReq::parse(required_version)) {
                    return rv.matches(&sv);
                }
            }
        }
        self.version.as_deref() == Some(required_version)
    }
}

fn extract_frontmatter(content: &str) -> Result<(String, String)> {
    let mut lines = content.lines();
    let mut frontmatter = String::new();
    let mut content = String::new();
    let mut in_frontmatter = false;
    let mut frontmatter_ended = false;

    while let Some(line) = lines.next() {
        if line == "---" {
            if in_frontmatter {
                frontmatter_ended = true;
                continue;
            } else {
                in_frontmatter = true;
                continue;
            }
        }
        if in_frontmatter && !frontmatter_ended {
            frontmatter.push_str(line);
            frontmatter.push('\n');
        } else if frontmatter_ended {
            content.push_str(line);
            content.push('\n');
        }
    }

    Ok((frontmatter, content))
}

pub struct SkillLoader {
    dirs: Vec<PathBuf>,
}

impl SkillLoader {
    pub fn new(dirs: Vec<PathBuf>) -> Self {
        SkillLoader { dirs }
    }

    pub fn load_skills(&self) -> Result<Vec<Skill>> {
        let mut skills = Vec::new();
        for dir in &self.dirs {
            if !dir.exists() {
                continue;
            }
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "md") {
                    match Skill::parse(path.clone()) {
                        Ok(skill) => {
                            if let Err(e) = skill.validate() {
                                tracing::warn!("Skipping invalid skill {}: {}", path.display(), e);
                                continue;
                            }
                            skills.push(skill);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse skill {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
        Ok(skills)
    }
}

pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
    trigger_index: HashMap<String, Vec<String>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        SkillRegistry {
            skills: HashMap::new(),
            trigger_index: HashMap::new(),
        }
    }

    pub fn register(&mut self, skill: Skill) -> Result<()> {
        skill.validate()?;
        let name = skill.name.clone();
        for trigger in &skill.triggers {
            self.trigger_index
                .entry(trigger.clone())
                .or_insert_with(Vec::new)
                .push(name.clone());
        }
        self.skills.insert(name, skill);
        Ok(())
    }

    pub fn find_by_trigger(&self, trigger: &str) -> Vec<&Skill> {
        let trigger_lower = trigger.to_lowercase();
        self.skills
            .values()
            .filter(|s| s.triggers.iter().any(|t| t.to_lowercase() == trigger_lower))
            .collect()
    }

    pub fn find_compatible_with_version(&self, required_version: &str) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|s| s.is_compatible_with(required_version))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_validate_triggers_empty() {
        let skill = Skill {
            name: "test".to_string(),
            description: "test".to_string(),
            triggers: vec![],
            content: "".to_string(),
            version: None,
            metadata: None,
        };
        assert!(skill.validate_triggers().is_err());
    }

    #[test]
    fn test_validate_triggers_too_short() {
        let skill = Skill {
            name: "test".to_string(),
            description: "test".to_string(),
            triggers: vec!["a".to_string()],
            content: "".to_string(),
            version: None,
            metadata: None,
        };
        assert!(skill.validate_triggers().is_err());
    }

    #[test]
    fn test_validate_version_invalid() {
        let skill = Skill {
            name: "test".to_string(),
            description: "test".to_string(),
            triggers: vec!["test".to_string()],
            content: "".to_string(),
            version: Some("invalid".to_string()),
            metadata: None,
        };
        assert!(skill.validate_version().is_err());
    }

    #[test]
    fn test_is_valid_version_valid() {
        assert!(Skill::is_valid_version("1.0.0"));
        assert!(Skill::is_valid_version("1.0"));
        #[cfg(feature = "ai-integration")]
        {
            assert!(Skill::is_valid_version("1.0.0-alpha+build"));
        }
        assert!(!Skill::is_valid_version("invalid"));
        assert!(!Skill::is_valid_version("1"));
    }

    #[test]
    fn test_skill_parse_and_validate() {
        let dir = TempDir::new().unwrap();
        let skill_path = dir.path().join("test.md");
        let content_str = "---\nname: test_skill\ndescription: Test skill\ntriggers:\n  - test\nversion: \"1.0.0\"\n---\n\n## Test Skill\nContent here\n";
        fs::write(&skill_path, content_str).unwrap();

        let skill = Skill::parse(skill_path).unwrap();
        assert_eq!(skill.name, "test_skill");
        assert_eq!(skill.version, Some("1.0.0".to_string()));
        assert!(skill.validate().is_ok());
    }

    #[test]
    fn test_skill_compatibility() {
        let skill = Skill {
            name: "test".to_string(),
            description: "test".to_string(),
            triggers: vec!["test".to_string()],
            content: "".to_string(),
            version: Some("1.0.0".to_string()),
            metadata: None,
        };
        #[cfg(feature = "ai-integration")]
        {
            assert!(skill.is_compatible_with("^1.0.0"));
        }
        assert!(skill.is_compatible_with("1.0.0"));
        assert!(!skill.is_compatible_with("2.0.0"));
    }
}
