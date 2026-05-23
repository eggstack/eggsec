pub mod evasion;
pub mod headers;
pub mod profiles;
pub mod smuggling;

use crate::error::Result;
use serde::{Deserialize, Serialize};

pub use evasion::EvasionBypass;
pub use headers::HeaderBypass;
pub use profiles::{
    get_auto_profile, get_profile_by_detection_sig, get_profile_by_name, get_waf_profiles,
    ProfileBypass, WafProfile,
};
pub use smuggling::SmugglingBypass;

use super::detector::WafDetectionResult;
use crate::cli::WafArgs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TestType {
    #[default]
    All,
    Sql,
    Xss,
    Ssrf,
    Cmd,
    Traversal,
}

impl TestType {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "sqli" | "sql" => TestType::Sql,
            "xss" => TestType::Xss,
            "ssrf" => TestType::Ssrf,
            "cmd" | "command" => TestType::Cmd,
            "traversal" | "lfi" => TestType::Traversal,
            _ => TestType::All,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BypassTechnique {
    HeaderManipulation,
    UserAgentRotation,
    XForwardedForSpoof,
    ContentTypeBypass,
    EncodingBypass,
    Homoglyph,
    ZeroWidthInjection,
    CaseRotation,
    UnicodeEncoding,
    CommentObfuscation,
    WhitespaceVariation,
    ChunkedEncoding,
    ContentLengthConflict,
    TransferEncodingConflict,
    DoubleEncoding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BypassResult {
    pub technique: BypassTechnique,
    pub success: bool,
    pub description: String,
    pub payload: Option<String>,
    pub status_code: u16,
    pub response_diff: Option<i64>,
}

pub struct BypassEngine {
    args: WafArgs,
    client: reqwest::Client,
    profile: Option<WafProfile>,
    test_type: TestType,
}

impl BypassEngine {
    pub fn new(args: &WafArgs, profile: Option<WafProfile>, test_type: TestType) -> Result<Self> {
        let client = crate::utils::create_insecure_client_with_options(args.timeout, |builder| {
            builder.redirect(reqwest::redirect::Policy::limited(5))
        })?;

        Ok(Self {
            args: args.clone(),
            client,
            profile,
            test_type,
        })
    }

    pub async fn run_bypasses(&self, detection: &WafDetectionResult) -> Result<Vec<BypassResult>> {
        let mut results = Vec::new();

        let profile = self.profile.as_ref();

        if self.args.header_bypass || self.args.bypass {
            let header_bypass = HeaderBypass::new(profile.cloned());
            results.extend(
                header_bypass
                    .run(&self.client, &self.args.url, detection, self.test_type)
                    .await?,
            );
        }

        if self.args.evasion || self.args.bypass {
            let evasion_bypass = EvasionBypass::new(profile.cloned());
            results.extend(
                evasion_bypass
                    .run(&self.client, &self.args.url, detection, self.test_type)
                    .await?,
            );
        }

        if self.args.smuggling || self.args.bypass {
            let smuggling_bypass = SmugglingBypass::new(profile.cloned());
            results.extend(
                smuggling_bypass
                    .run(&self.client, &self.args.url, detection)
                    .await?,
            );
        }

        Ok(results)
    }
}

pub fn is_bypass_successful(
    status: u16,
    detection: &WafDetectionResult,
    payload: &str,
    response_body: &str,
) -> bool {
    let blocked_codes = crate::constants::waf::BLOCKED_STATUS_CODES;
    let baseline_status = detection.status_code;
    let baseline_blocked = blocked_codes.contains(&baseline_status);
    let response_blocked = blocked_codes.contains(&status) || body_looks_blocked(response_body);
    if response_blocked {
        return false;
    }

    let reflected = payload_is_reflected(payload, response_body);
    let status_changed = status != baseline_status;
    let status_2xx = (200..300).contains(&status);
    if payload.is_empty() {
        return baseline_blocked && status_changed && status_2xx;
    }

    if baseline_blocked && status_changed && status_2xx {
        return reflected;
    }

    status_2xx && status_changed && reflected
}

fn payload_is_reflected(payload: &str, response_body: &str) -> bool {
    if payload.is_empty() {
        return true;
    }
    if response_body.is_empty() {
        return false;
    }
    let encoded_payload = urlencoding::encode(payload);
    response_body.contains(payload) || response_body.contains(encoded_payload.as_ref())
}

fn body_looks_blocked(response_body: &str) -> bool {
    let lower = response_body.to_lowercase();
    crate::constants::waf::BLOCKED_PATTERNS
        .iter()
        .any(|pattern| lower.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detection_with_status(status_code: u16) -> WafDetectionResult {
        WafDetectionResult {
            waf_name: Some("Test WAF".to_string()),
            confidence: 50,
            request_error: None,
            matched_headers: vec![],
            matched_cookies: vec![],
            matched_patterns: vec![],
            server_header: None,
            status_code,
        }
    }

    #[test]
    fn bypass_fails_for_blocked_status_codes() {
        let detection = detection_with_status(403);
        assert!(!is_bypass_successful(429, &detection, "", "ok"));
        assert!(!is_bypass_successful(503, &detection, "", "ok"));
    }

    #[test]
    fn bypass_fails_when_status_matches_baseline() {
        let detection = detection_with_status(200);
        assert!(!is_bypass_successful(200, &detection, "", "ok"));
    }

    #[test]
    fn bypass_requires_2xx_status_for_success() {
        let detection = detection_with_status(403);
        assert!(!is_bypass_successful(302, &detection, "", "ok"));
        assert!(is_bypass_successful(200, &detection, "", "ok"));
    }

    #[test]
    fn bypass_requires_payload_reflection_for_non_empty_payload() {
        let detection = detection_with_status(403);
        assert!(!is_bypass_successful(
            200, &detection, "admin'--", "welcome"
        ));
        assert!(is_bypass_successful(
            200,
            &detection,
            "admin'--",
            "admin'-- accepted"
        ));
    }

    #[test]
    fn empty_payload_needs_block_to_non_block_transition() {
        let detection = detection_with_status(200);
        assert!(!is_bypass_successful(200, &detection, "", "ok"));
        assert!(!is_bypass_successful(302, &detection, "", "ok"));
    }

    #[test]
    fn reflected_payload_fails_when_body_still_looks_blocked() {
        let detection = detection_with_status(403);
        assert!(!is_bypass_successful(
            200,
            &detection,
            "admin'--",
            "request blocked admin'--"
        ));
    }
}
