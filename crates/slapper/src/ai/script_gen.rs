use std::path::PathBuf;

use crate::ai::client::AiClient;
use crate::ai::errors::{AiError, Result};

/// Target language for generated security testing scripts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginLanguage {
    Python,
    Ruby,
    Rust,
}

#[derive(Debug, Clone)]
pub enum ScriptTarget {
    WafBypass {
        waf_name: String,
        blocked_payload: String,
    },
    PayloadGeneration {
        vuln_type: String,
        context: String,
    },
    AdaptiveScript {
        findings: Vec<serde_json::Value>,
    },
}

impl ScriptTarget {
    pub fn vuln_type(&self) -> Option<&str> {
        match self {
            ScriptTarget::WafBypass { .. } => Some("waf_bypass"),
            ScriptTarget::PayloadGeneration { vuln_type, .. } => Some(vuln_type),
            ScriptTarget::AdaptiveScript { .. } => Some("adaptive"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GeneratedScript {
    pub code: String,
    pub language: PluginLanguage,
    pub target: ScriptTarget,
    pub metadata: ScriptMetadata,
}

#[derive(Debug, Clone)]
pub struct ScriptMetadata {
    pub description: String,
    pub author: String,
    pub version: String,
    pub tags: Vec<String>,
    pub ai_generated: bool,
}

pub struct ScriptGenerator {
    client: Option<AiClient>,
    script_dir: PathBuf,
}

impl ScriptGenerator {
    pub fn new(client: Option<AiClient>) -> Self {
        let script_dir = directories::ProjectDirs::from("com", "slapper", "slapper")
            .map(|d| d.config_dir().join("generated_scripts"))
            .unwrap_or_else(|| PathBuf::from("generated_scripts"));

        Self {
            client: client.clone(),
            script_dir,
        }
    }

    pub async fn generate_waf_bypass_script(
        &self,
        waf: &str,
        blocked_payload: &str,
    ) -> Result<GeneratedScript> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| AiError::invalid_config("AI client required for script generation"))?;

        let prompt = Self::build_waf_bypass_prompt(waf, blocked_payload);

        let body = serde_json::json!({
            "model": client.model(),
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": 4096,
            "temperature": 0.7,
        });

        let response = client.chat_completion_from_messages(&body).await?;

        let content = response
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or(AiError::InvalidResponse)?;

        let code = Self::extract_code_block(content, "python")
            .unwrap_or_else(|| content.trim().to_string());
        if code.is_empty() {
            return Err(AiError::InvalidResponse);
        }

        Ok(GeneratedScript {
            code,
            language: PluginLanguage::Python,
            target: ScriptTarget::WafBypass {
                waf_name: waf.to_string(),
                blocked_payload: blocked_payload.to_string(),
            },
            metadata: ScriptMetadata {
                description: format!("WAF bypass script for {}", waf),
                author: "AI Generator".to_string(),
                version: "1.0.0".to_string(),
                tags: vec!["waf-bypass".to_string(), waf.to_lowercase()],
                ai_generated: true,
            },
        })
    }

    pub async fn generate_payload_script(
        &self,
        vuln_type: &str,
        context: &str,
    ) -> Result<GeneratedScript> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| AiError::invalid_config("AI client required for script generation"))?;

        let prompt = Self::build_payload_generation_prompt(vuln_type, context);

        let body = serde_json::json!({
            "model": client.model(),
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": 4096,
            "temperature": 0.8,
        });

        let response = client.chat_completion_from_messages(&body).await?;

        let content = response
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or(AiError::InvalidResponse)?;

        let code = Self::extract_code_block(content, "python")
            .unwrap_or_else(|| content.trim().to_string());
        if code.is_empty() {
            return Err(AiError::InvalidResponse);
        }

        Ok(GeneratedScript {
            code,
            language: PluginLanguage::Python,
            target: ScriptTarget::PayloadGeneration {
                vuln_type: vuln_type.to_string(),
                context: context.to_string(),
            },
            metadata: ScriptMetadata {
                description: format!("Payload generation script for {}", vuln_type),
                author: "AI Generator".to_string(),
                version: "1.0.0".to_string(),
                tags: vec!["payload-gen".to_string(), vuln_type.to_lowercase()],
                ai_generated: true,
            },
        })
    }

    pub async fn generate_adaptive_script(
        &self,
        findings: Vec<serde_json::Value>,
    ) -> Result<GeneratedScript> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| AiError::invalid_config("AI client required for script generation"))?;

        let prompt = Self::build_adaptive_script_prompt(&findings);

        let body = serde_json::json!({
            "model": client.model(),
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": 4096,
            "temperature": 0.7,
        });

        let response = client.chat_completion_from_messages(&body).await?;

        let content = response
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or(AiError::InvalidResponse)?;

        let code = Self::extract_code_block(content, "python")
            .unwrap_or_else(|| content.trim().to_string());
        if code.is_empty() {
            return Err(AiError::InvalidResponse);
        }

        Ok(GeneratedScript {
            code,
            language: PluginLanguage::Python,
            target: ScriptTarget::AdaptiveScript { findings },
            metadata: ScriptMetadata {
                description: "Adaptive security testing script generated from findings".to_string(),
                author: "AI Generator".to_string(),
                version: "1.0.0".to_string(),
                tags: vec!["adaptive".to_string(), "ai-generated".to_string()],
                ai_generated: true,
            },
        })
    }

    /// Save a generated script to disk.
    ///
    /// Files are written to `{config_dir}/generated_scripts/` with the naming
    /// convention `script_{vuln_type}_{timestamp}.py`. The file includes a
    /// metadata header with description, version, author, and tags.
    ///
    /// Returns the path to the saved file.
    pub fn save_script(&self, script: &GeneratedScript) -> Result<PathBuf> {
        let filename = format!(
            "script_{}_{}.py",
            script.target.vuln_type().unwrap_or("unknown"),
            chrono::Utc::now().timestamp()
        );

        let file_path = self.script_dir.join(&filename);

        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let header = format!(
            "# Name: {}\n# Version: {}\n# Description: {}\n# Author: {}\n# Tags: {}\n\n",
            script.target.vuln_type().unwrap_or("script"),
            script.metadata.version,
            script.metadata.description,
            script.metadata.author,
            script.metadata.tags.join(", ")
        );

        let full_content = format!("{}{}", header, script.code);
        std::fs::write(&file_path, full_content)?;

        tracing::info!("Saved generated script to {}", file_path.display());

        Ok(file_path)
    }

    pub fn script_dir(&self) -> &PathBuf {
        &self.script_dir
    }

    fn build_waf_bypass_prompt(waf: &str, blocked_payload: &str) -> String {
        format!(
            r#"Generate a Python security testing script that bypasses the {} WAF.

The following payload was blocked: {}

Write a complete Python script that:
1. Implements the same security test using an alternative approach
2. Uses encoding, obfuscation, or other bypass techniques specific to {}
3. Includes proper error handling and target validation
4. Has clear comments explaining the bypass technique
5. Is safe and only tests for vulnerabilities

Return ONLY the Python code, no markdown formatting or explanation."#,
            waf, blocked_payload, waf
        )
    }

    fn build_payload_generation_prompt(vuln_type: &str, context: &str) -> String {
        format!(
            r#"Generate a Python security testing script for {} vulnerability testing.

Context: {}

Write a complete Python script that:
1. Generates and sends {} test payloads
2. Properly handles response analysis
3. Includes appropriate error handling
4. Uses realistic security testing patterns
5. Has clear documentation

Return ONLY the Python code, no markdown formatting or explanation."#,
            vuln_type, context, vuln_type
        )
    }

    fn build_adaptive_script_prompt(findings: &[serde_json::Value]) -> String {
        let findings_str = match serde_json::to_string_pretty(findings) {
            Ok(s) => s,
            Err(e) => {
                tracing::debug!("Failed to serialize findings for prompt: {}", e);
                String::new()
            }
        };
        format!(
            r#"Based on these security findings:
{}

Generate a Python adaptive security testing script that:
1. Focuses on the identified vulnerability types
2. Uses targeted payloads based on the findings
3. Adapts the testing approach based on responses
4. Includes proper rate limiting and error handling
5. Is safe and ethical

Return ONLY the Python code, no markdown formatting or explanation."#,
            findings_str
        )
    }

    fn extract_code_block(content: &str, language: &str) -> Option<String> {
        let lang_marker = format!("```{}", language);
        let alt_marker = "```";

        if let Some(start) = content.find(&lang_marker) {
            let after_start = &content[start + lang_marker.len()..];
            if let Some(end) = after_start.find("```") {
                return Some(after_start[..end].trim().to_string());
            }
        }

        if let Some(start) = content.find(alt_marker) {
            let after_start = &content[start + 3..];
            if let Some(end) = after_start.find("```") {
                return Some(after_start[..end].trim().to_string());
            } else if after_start.len() > 10 {
                return Some(after_start.trim().to_string());
            }
        }

        None
    }
}

impl Clone for ScriptGenerator {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            script_dir: self.script_dir.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_client() -> Option<AiClient> {
        Some(
            AiClient::new(crate::config::AiConfig {
                provider: "openai".to_string(),
                model: Some("gpt-4".to_string()),
                api_key: None,
                base_url: None,
                max_tokens: Some(2048),
                temperature: Some(0.7),
                max_payloads: 50,
                max_bypasses: 10,
            })
            .expect("test client should be valid"),
        )
    }

    #[test]
    fn test_script_target_vuln_type() {
        assert_eq!(
            ScriptTarget::WafBypass {
                waf_name: "cloudflare".to_string(),
                blocked_payload: "test".to_string(),
            }
            .vuln_type(),
            Some("waf_bypass")
        );

        assert_eq!(
            ScriptTarget::PayloadGeneration {
                vuln_type: "sqli".to_string(),
                context: "test".to_string(),
            }
            .vuln_type(),
            Some("sqli")
        );

        assert_eq!(
            ScriptTarget::AdaptiveScript { findings: vec![] }.vuln_type(),
            Some("adaptive")
        );
    }

    #[test]
    fn test_script_generator_creation() {
        let generator = ScriptGenerator::new(None);
        assert!(generator.client.is_none());
    }

    #[test]
    fn test_script_generator_with_client() {
        let generator = ScriptGenerator::new(create_test_client());
        assert!(generator.client.is_some());
    }

    #[test]
    fn test_extract_code_block_with_language() {
        let content = "Here is the code:\n```python\nprint('hello')\n```\nDone";
        let extracted = ScriptGenerator::extract_code_block(content, "python");
        assert_eq!(extracted, Some("print('hello')".to_string()));
    }

    #[test]
    fn test_extract_code_block_without_language() {
        let content = "```\nprint('hello')\n```";
        let extracted = ScriptGenerator::extract_code_block(content, "python");
        assert_eq!(extracted, Some("print('hello')".to_string()));
    }

    #[test]
    fn test_extract_code_block_no_block() {
        let content = "Just some text without code blocks";
        let extracted = ScriptGenerator::extract_code_block(content, "python");
        assert!(extracted.is_none());
    }

    #[test]
    fn test_generate_waf_bypass_script_requires_client() {
        let generator = ScriptGenerator::new(None);
        let result = futures::executor::block_on(
            generator.generate_waf_bypass_script("cloudflare", "payload"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_payload_script_requires_client() {
        let generator = ScriptGenerator::new(None);
        let result =
            futures::executor::block_on(generator.generate_payload_script("xss", "reflected"));
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_adaptive_script_requires_client() {
        let generator = ScriptGenerator::new(None);
        let result = futures::executor::block_on(generator.generate_adaptive_script(vec![]));
        assert!(result.is_err());
    }

    #[test]
    fn test_script_dir_default() {
        let generator = ScriptGenerator::new(None);
        let dir = generator.script_dir();
        assert!(dir.to_string_lossy().contains("generated_scripts"));
    }

    #[test]
    fn test_clone_preserves_client() {
        let generator = ScriptGenerator::new(create_test_client());
        let cloned = generator.clone();
        assert!(cloned.client.is_some());
    }

    #[test]
    fn test_generated_script_metadata() {
        let script = GeneratedScript {
            code: "print('test')".to_string(),
            language: PluginLanguage::Python,
            target: ScriptTarget::PayloadGeneration {
                vuln_type: "xss".to_string(),
                context: "reflected".to_string(),
            },
            metadata: ScriptMetadata {
                description: "Test script".to_string(),
                author: "Test".to_string(),
                version: "1.0.0".to_string(),
                tags: vec!["test".to_string()],
                ai_generated: true,
            },
        };

        assert_eq!(script.metadata.author, "Test");
        assert!(script.metadata.ai_generated);
    }
}
