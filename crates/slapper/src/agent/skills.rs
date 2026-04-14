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

        let frontmatter: SkillFrontmatter = serde_yaml::from_str(parts[0])?;

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
}
