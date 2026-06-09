use crate::ai::client::AiClient;
use crate::ai::types::ScanFinding;
use crate::error::Result;
use crate::types::Severity;

pub struct AdaptiveScanEngine {
    client: Option<AiClient>,
    strategy: String,
    ai_suggested_strategy: Option<String>,
}

impl AdaptiveScanEngine {
    pub fn new(client: Option<AiClient>) -> Self {
        Self {
            client,
            strategy: "standard".to_string(),
            ai_suggested_strategy: None,
        }
    }

    pub async fn adjust_strategy(&mut self, findings: &[ScanFinding]) -> Result<&str> {
        if let Some(ref client) = self.client {
            let findings_json: Vec<serde_json::Value> = findings
                .iter()
                .filter_map(|f| serde_json::to_value(f).ok())
                .collect();

            if !findings_json.is_empty() {
                match client.analyze_findings(&findings_json).await {
                    Ok(response) => {
                        if let Some(content) = response
                            .get("choices")
                            .and_then(|c| c.get(0))
                            .and_then(|c| c.get("message"))
                            .and_then(|m| m.get("content"))
                            .and_then(|c| c.as_str())
                        {
                            let strategy = Self::extract_strategy_from_ai_response(content);
                            self.ai_suggested_strategy = Some(strategy.clone());
                            self.strategy = strategy;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("AI strategy suggestion failed, using fallback: {}", e);
                        self.strategy = Self::fallback_strategy(findings);
                    }
                }
            } else {
                self.strategy = Self::fallback_strategy(findings);
            }
        } else {
            self.strategy = Self::fallback_strategy(findings);
        }

        Ok(&self.strategy)
    }

    fn extract_strategy_from_ai_response(content: &str) -> String {
        let lower = content.to_lowercase();
        if lower.contains("deep") || lower.contains("thorough") {
            "deep".to_string()
        } else if lower.contains("aggressive") || lower.contains("comprehensive") {
            "thorough".to_string()
        } else if lower.contains("quick") || lower.contains("light") {
            "quick".to_string()
        } else if lower.contains("stealth") {
            "stealth".to_string()
        } else {
            "standard".to_string()
        }
    }

    fn fallback_strategy(findings: &[ScanFinding]) -> String {
        let critical_count = findings
            .iter()
            .filter(|f| f.severity == Severity::Critical)
            .count();
        let high_count = findings
            .iter()
            .filter(|f| f.severity == Severity::High)
            .count();

        if critical_count > 0 {
            "deep".to_string()
        } else if high_count > 3 {
            "thorough".to_string()
        } else {
            "quick".to_string()
        }
    }

    pub fn get_strategy(&self) -> &str {
        &self.strategy
    }

    pub fn get_ai_suggestion(&self) -> Option<&str> {
        self.ai_suggested_strategy.as_deref()
    }

    pub fn fallback_to_standard(&mut self) {
        self.strategy = "standard".to_string();
        self.ai_suggested_strategy = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_strategy_deep() {
        assert_eq!(
            AdaptiveScanEngine::extract_strategy_from_ai_response(
                "I recommend a deep scan approach."
            ),
            "deep"
        );
    }

    #[test]
    fn test_extract_strategy_quick() {
        assert_eq!(
            AdaptiveScanEngine::extract_strategy_from_ai_response(
                "Quick scan should be sufficient."
            ),
            "quick"
        );
    }

    #[test]
    fn test_extract_strategy_stealth() {
        assert_eq!(
            AdaptiveScanEngine::extract_strategy_from_ai_response(
                "Use stealth mode to avoid detection."
            ),
            "stealth"
        );
    }

    #[test]
    fn test_extract_strategy_default() {
        assert_eq!(
            AdaptiveScanEngine::extract_strategy_from_ai_response("No specific recommendation."),
            "standard"
        );
    }

    #[test]
    fn test_fallback_strategy_critical() {
        let findings = vec![ScanFinding {
            id: "1".to_string(),
            title: "Critical vuln".to_string(),
            severity: crate::types::Severity::Critical,
            description: "Test".to_string(),
        }];
        assert_eq!(AdaptiveScanEngine::fallback_strategy(&findings), "deep");
    }

    #[test]
    fn test_fallback_strategy_high() {
        let findings = vec![
            ScanFinding {
                id: "1".to_string(),
                title: "High".to_string(),
                severity: crate::types::Severity::High,
                description: "Test".to_string(),
            },
            ScanFinding {
                id: "2".to_string(),
                title: "High".to_string(),
                severity: crate::types::Severity::High,
                description: "Test".to_string(),
            },
            ScanFinding {
                id: "3".to_string(),
                title: "High".to_string(),
                severity: crate::types::Severity::High,
                description: "Test".to_string(),
            },
            ScanFinding {
                id: "4".to_string(),
                title: "High".to_string(),
                severity: crate::types::Severity::High,
                description: "Test".to_string(),
            },
        ];
        assert_eq!(AdaptiveScanEngine::fallback_strategy(&findings), "thorough");
    }

    #[test]
    fn test_fallback_strategy_quick() {
        let findings = vec![ScanFinding {
            id: "1".to_string(),
            title: "Low".to_string(),
            severity: crate::types::Severity::Low,
            description: "Test".to_string(),
        }];
        assert_eq!(AdaptiveScanEngine::fallback_strategy(&findings), "quick");
    }

    #[test]
    fn test_engine_creation() {
        let engine = AdaptiveScanEngine::new(None);
        assert_eq!(engine.get_strategy(), "standard");
        assert!(engine.get_ai_suggestion().is_none());
    }
}
