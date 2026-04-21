//! Skill system for the security agent.
//!
//! Loads and manages YAML+Markdown skill files that define agent capabilities.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub triggers: Vec<String>,
    pub metadata: SkillMetadata,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub category: String,
    pub tools: Vec<String>,
    pub scope: String,
    pub requires: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: Option<String>,
    pub triggers: Option<Vec<String>>,
    pub metadata: SkillMetadata,
}

impl Skill {
    pub fn parse(content: &str) -> Result<Self> {
        let parts: Vec<&str> = content.split("\n---\n").collect();

        if parts.len() != 2 {
            anyhow::bail!(
                "Skill file must have YAML frontmatter and Markdown body separated by '---'"
            );
        }

        let frontmatter: SkillFrontmatter = serde_yaml_neo::from_str(parts[0])?;

        let name = frontmatter.name.clone();
        let description = frontmatter
            .description
            .clone()
            .unwrap_or_else(|| extract_description(parts[1]));
        let triggers = frontmatter
            .triggers
            .clone()
            .unwrap_or_else(|| extract_triggers(parts[1]));

        Ok(Self {
            name,
            description,
            triggers,
            metadata: frontmatter.metadata,
            content: parts[1].to_string(),
        })
    }

    pub fn matches_trigger(&self, input: &str) -> bool {
        let input_lower = input.to_lowercase();
        self.triggers
            .iter()
            .any(|t| input_lower.contains(&t.to_lowercase()))
    }

    pub fn to_prompt(&self) -> String {
        format!(
            "# Skill: {}\n\n{}\n\n---\n\n{}\n\n## Tool Usage\nTools: {}",
            self.name,
            self.description,
            self.content.lines().take(50).collect::<Vec<_>>().join("\n"),
            self.metadata.tools.join(", ")
        )
    }
}

fn extract_description(markdown: &str) -> String {
    markdown
        .lines()
        .skip_while(|l| !l.starts_with("##"))
        .skip(1)
        .take_while(|l| !l.starts_with("##"))
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_triggers(markdown: &str) -> Vec<String> {
    let mut triggers = Vec::new();

    for line in markdown.lines() {
        let line_lower = line.to_lowercase();
        if line_lower.contains("trigger")
            || line_lower.contains("keyword")
            || line_lower.contains("example")
        {
            let cleaned: String = line
                .chars()
                .skip_while(|c| !c.is_alphanumeric())
                .take_while(|c| c.is_alphanumeric() || *c == ',' || *c == ' ' || *c == '-')
                .collect();

            for part in cleaned.split(&[',', ' ', '-'][..]) {
                let part = part.trim();
                if part.len() > 2 && !part.starts_with('#') {
                    triggers.push(part.to_lowercase());
                }
            }
        }
    }

    if triggers.is_empty() {
        triggers.push("scan".to_string());
        triggers.push("security".to_string());
        triggers.push("test".to_string());
    }

    triggers
}

pub struct SkillLoader {
    skill_dirs: Vec<PathBuf>,
}

impl SkillLoader {
    pub fn new(skill_dirs: Vec<PathBuf>) -> Self {
        Self { skill_dirs }
    }

    pub fn load_skills(&self) -> Result<Vec<Skill>> {
        let mut skills = Vec::new();

        for dir in &self.skill_dirs {
            let canonical_dir = dir.canonicalize().map_err(|e| {
                anyhow::anyhow!(
                    "Failed to canonicalize skill directory {}: {}",
                    dir.display(),
                    e
                )
            })?;

            if !canonical_dir.exists() {
                continue;
            }

            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().map(|e| e == "md").unwrap_or(false) {
                    if let Ok(canonical_path) = path.canonicalize() {
                        if canonical_path.starts_with(&canonical_dir) {
                            if let Ok(skill) = self.load_skill(&path) {
                                skills.push(skill);
                            }
                        }
                    }
                }
            }
        }

        Ok(skills)
    }

    pub fn load_skill(&self, path: &PathBuf) -> Result<Skill> {
        let content = fs::read_to_string(path)?;
        Skill::parse(&content)
    }
}

#[derive(Clone)]
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
    skills_by_trigger: HashMap<String, Vec<String>>,
    skills_by_tool: HashMap<String, Vec<String>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            skills_by_trigger: HashMap::new(),
            skills_by_tool: HashMap::new(),
        }
    }

    pub fn register(&mut self, skill: Skill) -> Result<()> {
        let id = skill.name.clone();

        self.skills.insert(id.clone(), skill.clone());

        for trigger in &skill.triggers {
            self.skills_by_trigger
                .entry(trigger.clone())
                .or_default()
                .push(id.clone());
        }

        for tool in &skill.metadata.tools {
            self.skills_by_tool
                .entry(tool.clone())
                .or_default()
                .push(id.clone());
        }

        Ok(())
    }

    pub fn find_by_trigger(&self, trigger: &str) -> Vec<&Skill> {
        self.skills_by_trigger
            .get(trigger)
            .map(|ids| ids.iter().filter_map(|id| self.skills.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn find_by_tool(&self, tool_id: &str) -> Vec<&Skill> {
        self.skills_by_tool
            .get(tool_id)
            .map(|ids| ids.iter().filter_map(|id| self.skills.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn get_prompts_for_context(&self, context: &str) -> Vec<String> {
        let mut prompts = Vec::new();

        for skill in self.skills.values() {
            if skill.matches_trigger(context) {
                prompts.push(skill.to_prompt());
            }
        }

        prompts
    }

    pub fn skill_count(&self) -> usize {
        self.skills.len()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_parse() {
        let content = r#"---
name: test-skill
description: "Test skill for unit testing"
metadata:
  category: testing
  tools: [recon, scan]
  scope: targets
---

## Overview
This is a test skill.

## Triggers
- test
- scan
- security
"#;

        let skill = Skill::parse(content).unwrap();
        assert_eq!(skill.name, "test-skill");
        assert!(!skill.triggers.is_empty());
    }

    #[test]
    fn test_trigger_matching() {
        let content = r#"---
name: recon-skill
triggers:
  - recon
  - reconnaissance
  - dns
metadata:
  category: recon
  tools: [recon]
  scope: targets
---

## Overview
Reconnaissance skill.
"#;

        let skill = Skill::parse(content).unwrap();
        assert!(skill.matches_trigger("recon"));
        assert!(skill.matches_trigger("run recon on example.com"));
    }

    const VALID_SKILL_CONTENT: &str = r#"---
name: sql-injection-scanner
description: "SQL injection vulnerability scanner"
triggers:
  - sql injection
  - sqli
  - database
metadata:
  category: vulnerability
  tools: [fuzzer, scanner]
  scope: targets
---

## Overview
Scans for SQL injection vulnerabilities.

## Usage
Use the fuzzer with SQL injection payloads.

## Keywords
SQL injection, SQLi, database vulnerability
"#;

    const VALID_SKILL_CONTENT_2: &str = r#"---
name: xss-scanner
description: "Cross-site scripting scanner"
triggers:
  - xss
  - cross-site scripting
  - javascript
metadata:
  category: vulnerability
  tools: [fuzzer]
  scope: targets
---

## Overview
Scans for XSS vulnerabilities.
"#;

    #[test]
    fn test_skill_parse_valid() {
        let skill = Skill::parse(VALID_SKILL_CONTENT).unwrap();
        assert_eq!(skill.name, "sql-injection-scanner");
        assert!(skill.description.contains("SQL injection"));
        assert!(skill.triggers.contains(&"sql injection".to_string()));
        assert_eq!(skill.metadata.category, "vulnerability");
        assert!(skill.metadata.tools.contains(&"fuzzer".to_string()));
    }

    #[test]
    fn test_skill_parse_without_frontmatter_triggers() {
        let content = r#"---
name: test-skill
description: "Test skill"
metadata:
  category: testing
  tools: [scan]
  scope: targets
---

## Overview
This is a test skill.

## Keywords
scan, test, security
"#;

        let skill = Skill::parse(content).unwrap();
        assert!(!skill.triggers.is_empty());
    }

    #[test]
    fn test_skill_parse_invalid_no_separator() {
        let content = r#"name: test-skill
description: Test skill

This is just plain markdown without YAML frontmatter.
"#;

        let result = Skill::parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_skill_parse_invalid_yaml() {
        let content = r#"---
name: [invalid yaml
description: "Test"
---
Content
"#;

        let result = Skill::parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_skill_matches_trigger_case_insensitive() {
        let skill = Skill::parse(VALID_SKILL_CONTENT).unwrap();
        assert!(skill.matches_trigger("SQL INJECTION"));
        assert!(skill.matches_trigger("Sql Injection"));
        assert!(skill.matches_trigger("sqli testing"));
    }

    #[test]
    fn test_skill_matches_trigger_partial() {
        let skill = Skill::parse(VALID_SKILL_CONTENT).unwrap();
        assert!(skill.matches_trigger("test for sql injection on login"));
    }

    #[test]
    fn test_skill_matches_trigger_no_match() {
        let skill = Skill::parse(VALID_SKILL_CONTENT).unwrap();
        assert!(!skill.matches_trigger("xss attack"));
        assert!(!skill.matches_trigger(" Buffer Overflow"));
    }

    #[test]
    fn test_skill_to_prompt() {
        let skill = Skill::parse(VALID_SKILL_CONTENT).unwrap();
        let prompt = skill.to_prompt();
        assert!(prompt.contains("sql-injection-scanner"));
        assert!(prompt.contains("fuzzer"));
        assert!(prompt.contains("scanner"));
    }

    #[test]
    fn test_skill_to_prompt_truncates_content() {
        let long_content = format!("{}\n{}\n{}", VALID_SKILL_CONTENT, "# Section 2\n".repeat(100), "## End");
        let skill = Skill::parse(&long_content).unwrap();
        let prompt = skill.to_prompt();
        let lines: Vec<&str> = prompt.lines().collect();
        assert!(lines.len() < 100);
    }

    #[test]
    fn test_extract_description() {
        let content = r#"## Overview
This is the overview section.

## Usage
This describes usage.

## Another Section
More content here.
"#;

        let description = extract_description(content);
        assert!(description.contains("This is the overview section"));
        assert!(!description.contains("Usage"));
    }

    #[test]
    fn test_extract_triggers_default() {
        let content = "# Just a header\n\nSome content without triggers";
        let triggers = extract_triggers(content);
        assert!(triggers.contains(&"scan".to_string()));
        assert!(triggers.contains(&"security".to_string()));
    }

    #[test]
    fn test_extract_triggers_from_keyword_line() {
        let content = "## Keywords\nsql, injection, sqli, database";
        let triggers = extract_triggers(content);
        assert!(triggers.iter().any(|t| t.contains("sql")));
        assert!(triggers.iter().any(|t| t.contains("injection")));
    }

    #[test]
    fn test_skill_registry_new() {
        let registry = SkillRegistry::new();
        assert_eq!(registry.skill_count(), 0);
    }

    #[test]
    fn test_skill_registry_register() {
        let mut registry = SkillRegistry::new();
        let skill = Skill::parse(VALID_SKILL_CONTENT).unwrap();
        let result = registry.register(skill);
        assert!(result.is_ok());
        assert_eq!(registry.skill_count(), 1);
    }

    #[test]
    fn test_skill_registry_find_by_trigger() {
        let mut registry = SkillRegistry::new();
        let skill1 = Skill::parse(VALID_SKILL_CONTENT).unwrap();
        let skill2 = Skill::parse(VALID_SKILL_CONTENT_2).unwrap();
        registry.register(skill1).unwrap();
        registry.register(skill2).unwrap();

        let found = registry.find_by_trigger("sql injection");
        assert!(!found.is_empty());
        assert!(found.iter().any(|s| s.name == "sql-injection-scanner"));
    }

    #[test]
    fn test_skill_registry_find_by_trigger_no_match() {
        let mut registry = SkillRegistry::new();
        let skill = Skill::parse(VALID_SKILL_CONTENT).unwrap();
        registry.register(skill).unwrap();

        let found = registry.find_by_trigger("nonexistent");
        assert!(found.is_empty());
    }

    #[test]
    fn test_skill_registry_find_by_tool() {
        let mut registry = SkillRegistry::new();
        let skill1 = Skill::parse(VALID_SKILL_CONTENT).unwrap();
        let skill2 = Skill::parse(VALID_SKILL_CONTENT_2).unwrap();
        registry.register(skill1).unwrap();
        registry.register(skill2).unwrap();

        let found = registry.find_by_tool("fuzzer");
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn test_skill_registry_find_by_tool_no_match() {
        let mut registry = SkillRegistry::new();
        let skill = Skill::parse(VALID_SKILL_CONTENT).unwrap();
        registry.register(skill).unwrap();

        let found = registry.find_by_tool("nonexistent");
        assert!(found.is_empty());
    }

    #[test]
    fn test_skill_registry_get_prompts_for_context() {
        let mut registry = SkillRegistry::new();
        let skill1 = Skill::parse(VALID_SKILL_CONTENT).unwrap();
        let skill2 = Skill::parse(VALID_SKILL_CONTENT_2).unwrap();
        registry.register(skill1).unwrap();
        registry.register(skill2).unwrap();

        let prompts = registry.get_prompts_for_context("scan for sql injection");
        assert!(!prompts.is_empty());
    }

    #[test]
    fn test_skill_registry_get_prompts_for_context_no_match() {
        let mut registry = SkillRegistry::new();
        let skill = Skill::parse(VALID_SKILL_CONTENT).unwrap();
        registry.register(skill).unwrap();

        let prompts = registry.get_prompts_for_context("buffer overflow scan");
        assert!(prompts.is_empty());
    }

    #[test]
    fn test_skill_loader_new() {
        let loader = SkillLoader::new(vec![]);
        assert!(loader.load_skills().is_ok());
    }

    #[test]
    fn test_skill_metadata_default() {
        let metadata = SkillMetadata {
            category: "test".to_string(),
            tools: vec!["scan".to_string()],
            scope: "targets".to_string(),
            requires: None,
        };
        assert_eq!(metadata.category, "test");
        assert!(metadata.tools.contains(&"scan".to_string()));
    }
}
