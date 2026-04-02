use crate::ai::client::AiClient;
use crate::ai::types::ScanFinding;

pub struct AdaptiveScanEngine {
    client: Option<AiClient>,
    strategy: String,
}

impl AdaptiveScanEngine {
    pub fn new(client: Option<AiClient>) -> Self {
        Self {
            client,
            strategy: "standard".to_string(),
        }
    }

    pub fn adjust_strategy(&mut self, findings: &[ScanFinding]) -> &str {
        if self.client.is_some() {
            let critical_count = findings.iter().filter(|f| f.severity.as_int() >= 4).count();
            let high_count = findings.iter().filter(|f| f.severity.as_int() >= 3).count();

            if critical_count > 0 {
                self.strategy = "deep".to_string();
            } else if high_count > 3 {
                self.strategy = "thorough".to_string();
            } else {
                self.strategy = "quick".to_string();
            }
        }
        &self.strategy
    }

    pub fn get_strategy(&self) -> &str {
        &self.strategy
    }

    pub fn fallback_to_standard(&mut self) {
        self.strategy = "standard".to_string();
    }
}
