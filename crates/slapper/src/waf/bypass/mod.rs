
pub mod evasion;
pub mod headers;
pub mod profiles;
pub mod smuggling;

use crate::error::Result;
use serde::{Deserialize, Serialize};

pub use evasion::EvasionBypass;
pub use headers::HeaderBypass;
pub use profiles::{
    get_auto_profile, get_profile_by_name, get_waf_profiles, ProfileBypass, WafProfile,
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

pub fn is_bypass_successful(status: u16, detection: &WafDetectionResult) -> bool {
    !crate::constants::waf::BLOCKED_STATUS_CODES.contains(&status) 
        && status != detection.status_code 
        && (200..400).contains(&status)
}
