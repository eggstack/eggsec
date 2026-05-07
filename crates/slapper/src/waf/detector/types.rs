use crate::utils::contains_ignore_case;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafDetectionResult {
    pub waf_name: Option<String>,
    pub confidence: u8,
    #[serde(default)]
    pub request_error: Option<String>,
    pub matched_headers: Vec<String>,
    pub matched_cookies: Vec<String>,
    pub matched_patterns: Vec<String>,
    pub server_header: Option<String>,
    pub status_code: u16,
}

pub(crate) struct WafSignatureLower {
    pub(crate) headers: Vec<String>,
    pub(crate) cookies: Vec<String>,
    pub(crate) body_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseDiff {
    pub normal_status: u16,
    pub normal_length: usize,
    pub malicious_status: u16,
    pub malicious_length: usize,
    pub normal_headers: Option<std::collections::HashMap<String, String>>,
    pub malicious_headers: Option<std::collections::HashMap<String, String>>,
    pub header_diffs: Vec<String>,
    pub body_diffs: Option<bool>,
}

impl ResponseDiff {
    pub fn is_waf_blocked(&self) -> bool {
        let status_blocked = self.malicious_status != self.normal_status
            && (self.malicious_status == 403
                || self.malicious_status == 406
                || self.malicious_status == 405);

        let length_blocked = self.normal_length.saturating_sub(self.malicious_length)
            > crate::constants::waf::LENGTH_DIFF_THRESHOLD;

        let header_blocked = self.header_diffs.iter().any(|h| {
            contains_ignore_case(h, "waf")
                || contains_ignore_case(h, "firewall")
                || contains_ignore_case(h, "blocked")
                || contains_ignore_case(h, "attack")
        });

        status_blocked || length_blocked || header_blocked
    }
}
