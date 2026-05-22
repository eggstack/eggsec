use crate::error::Result;
use reqwest::Client;

use super::{BypassResult, BypassTechnique, TestType, WafProfile};
use crate::waf::detector::WafDetectionResult;
use crate::waf::payloads::encoding::{
    get_command_injection_payloads, get_sqli_payloads, get_ssrf_payloads, get_traversal_payloads,
    get_waf_test_payloads, get_xss_payloads, BypassType,
};

pub struct EvasionBypass {
    profile: Option<WafProfile>,
}

impl EvasionBypass {
    pub fn new(profile: Option<WafProfile>) -> Self {
        Self { profile }
    }

    pub async fn run(
        &self,
        client: &Client,
        url: &str,
        detection: &WafDetectionResult,
        test_type: TestType,
    ) -> Result<Vec<BypassResult>> {
        let mut results = Vec::new();
        let normalized_url = crate::waf::WafDetector::normalize_url_static(url);

        if let Some(ref profile) = self.profile {
            for bypass in &profile.bypasses {
                let technique = bypass.technique;
                if matches!(
                    technique,
                    BypassTechnique::EncodingBypass
                        | BypassTechnique::Homoglyph
                        | BypassTechnique::ZeroWidthInjection
                        | BypassTechnique::CaseRotation
                        | BypassTechnique::UnicodeEncoding
                        | BypassTechnique::CommentObfuscation
                        | BypassTechnique::WhitespaceVariation
                        | BypassTechnique::DoubleEncoding
                ) {
                    for payload in &bypass.payloads {
                        let result = self
                            .test_payload(
                                client,
                                &normalized_url,
                                payload,
                                technique,
                                bypass.description.clone(),
                                detection,
                            )
                            .await?;
                        results.push(result);
                    }
                }
            }
        } else {
            let test_payloads = self.generate_evasion_payloads(test_type);

            for (technique, payload, description) in test_payloads {
                let result = self
                    .test_payload(
                        client,
                        &normalized_url,
                        &payload,
                        technique,
                        description,
                        detection,
                    )
                    .await?;
                results.push(result);
            }

            let structured_payloads = self.generate_structured_payloads(test_type);
            for (technique, payload, description) in structured_payloads {
                let result = self
                    .test_payload(
                        client,
                        &normalized_url,
                        &payload,
                        technique,
                        description,
                        detection,
                    )
                    .await?;
                results.push(result);
            }
        }

        Ok(results)
    }

    fn generate_evasion_payloads(
        &self,
        test_type: TestType,
    ) -> Vec<(BypassTechnique, String, String)> {
        let mut payloads = Vec::new();

        if test_type == TestType::All || test_type == TestType::Sql {
            for sqli in get_sqli_payloads() {
                payloads.push((
                    BypassTechnique::CaseRotation,
                    apply_case_rotation(sqli),
                    format!("Case rotation: {}", &sqli[..30.min(sqli.len())]),
                ));
            }

            for sqli in get_sqli_payloads() {
                payloads.push((
                    BypassTechnique::Homoglyph,
                    apply_homoglyphs(sqli),
                    format!("Homoglyph: {}", &sqli[..30.min(sqli.len())]),
                ));
            }

            for sqli in get_sqli_payloads().iter().take(3) {
                payloads.push((
                    BypassTechnique::ZeroWidthInjection,
                    apply_zero_width(sqli),
                    format!("Zero-width: {}", &sqli[..30.min(sqli.len())]),
                ));
            }

            for sqli in get_sqli_payloads().iter().take(3) {
                payloads.push((
                    BypassTechnique::CommentObfuscation,
                    apply_comment_obfuscation(sqli),
                    format!("Comment obfuscation: {}", &sqli[..30.min(sqli.len())]),
                ));
            }

            for sqli in get_sqli_payloads().iter().take(3) {
                payloads.push((
                    BypassTechnique::WhitespaceVariation,
                    apply_whitespace_variation(sqli),
                    format!("Whitespace variation: {}", &sqli[..30.min(sqli.len())]),
                ));
            }

            for sqli in get_sqli_payloads().iter().take(2) {
                payloads.push((
                    BypassTechnique::UnicodeEncoding,
                    apply_unicode_encoding(sqli),
                    format!("Unicode encoding: {}", &sqli[..30.min(sqli.len())]),
                ));
            }

            for sqli in get_sqli_payloads().iter().take(2) {
                payloads.push((
                    BypassTechnique::DoubleEncoding,
                    apply_double_encoding(sqli),
                    format!("Double encoding: {}", &sqli[..30.min(sqli.len())]),
                ));
            }
        }

        if test_type == TestType::All || test_type == TestType::Xss {
            for xss in get_xss_payloads().iter().take(3) {
                payloads.push((
                    BypassTechnique::Homoglyph,
                    apply_homoglyphs(xss),
                    format!("XSS Homoglyph: {}", &xss[..30.min(xss.len())]),
                ));
            }
        }

        if test_type == TestType::All || test_type == TestType::Ssrf {
            for ssrf in get_ssrf_payloads().iter().take(3) {
                payloads.push((
                    BypassTechnique::DoubleEncoding,
                    apply_double_encoding(ssrf),
                    format!("SSRF Double Encoding: {}", &ssrf[..30.min(ssrf.len())]),
                ));
            }
        }

        if test_type == TestType::All || test_type == TestType::Traversal {
            for traversal in get_traversal_payloads().iter().take(5) {
                payloads.push((
                    BypassTechnique::EncodingBypass,
                    apply_double_encoding(traversal),
                    format!(
                        "Traversal Double Encoding: {}",
                        &traversal[..30.min(traversal.len())]
                    ),
                ));
            }
        }

        if test_type == TestType::All || test_type == TestType::Cmd {
            for cmd in get_command_injection_payloads().iter().take(5) {
                payloads.push((
                    BypassTechnique::EncodingBypass,
                    apply_case_rotation(cmd),
                    format!("Cmd Case Rotation: {}", &cmd[..30.min(cmd.len())]),
                ));
            }
        }

        payloads
    }

    fn generate_structured_payloads(
        &self,
        test_type: TestType,
    ) -> Vec<(BypassTechnique, String, String)> {
        let mut payloads = Vec::new();

        for waf_payload in get_waf_test_payloads() {
            let should_include = match test_type {
                TestType::All => true,
                TestType::Sql => waf_payload.bypass_types.contains(&BypassType::SqlInjection),
                TestType::Xss => waf_payload.bypass_types.contains(&BypassType::Xss),
                TestType::Ssrf => waf_payload.bypass_types.contains(&BypassType::Ssrf),
                TestType::Cmd => waf_payload
                    .bypass_types
                    .contains(&BypassType::CommandInjection),
                TestType::Traversal => waf_payload
                    .bypass_types
                    .contains(&BypassType::PathTraversal),
            };

            if should_include {
                payloads.push((
                    BypassTechnique::EncodingBypass,
                    apply_double_encoding(&waf_payload.payload),
                    format!("{}: {}", waf_payload.name, waf_payload.description),
                ));
            }
        }

        payloads
    }

    async fn test_payload(
        &self,
        client: &Client,
        url: &str,
        payload: &str,
        technique: BypassTechnique,
        description: String,
        detection: &WafDetectionResult,
    ) -> Result<BypassResult> {
        let test_url = format!("{}?q={}", url, urlencoding::encode(payload));

        let response = client
            .get(&test_url)
            .header("User-Agent", crate::waf::bypass::headers::get_random_ua())
            .send()
            .await?;

        let status = response.status().as_u16();
        let body = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                tracing::debug!("Failed to read response body in evasion bypass: {}", e);
                String::new()
            }
        };
        let success = self.is_bypass_successful(status, detection, payload, &body);

        Ok(BypassResult {
            technique,
            success,
            description,
            payload: Some(payload.to_string()),
            status_code: status,
            response_diff: None,
        })
    }

    fn is_bypass_successful(
        &self,
        status: u16,
        detection: &WafDetectionResult,
        payload: &str,
        response_body: &str,
    ) -> bool {
        super::is_bypass_successful(status, detection, payload, response_body)
    }
}

pub fn apply_case_rotation(input: &str) -> String {
    input
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if i % 2 == 0 {
                c.to_ascii_uppercase()
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect()
}

pub fn apply_homoglyphs(input: &str) -> String {
    let mut result = String::new();
    for c in input.chars() {
        match c {
            'a' => result.push('\u{0430}'),
            'c' => result.push('\u{0441}'),
            'e' => result.push('\u{0435}'),
            'o' => result.push('\u{043E}'),
            'p' => result.push('\u{0440}'),
            'x' => result.push('\u{0445}'),
            'y' => result.push('\u{0443}'),
            'A' => result.push('\u{0410}'),
            'B' => result.push('\u{0412}'),
            'C' => result.push('\u{0421}'),
            'E' => result.push('\u{0415}'),
            'H' => result.push('\u{041D}'),
            'K' => result.push('\u{041A}'),
            'M' => result.push('\u{041C}'),
            'O' => result.push('\u{041E}'),
            'P' => result.push('\u{0420}'),
            'T' => result.push('\u{0422}'),
            'X' => result.push('\u{0425}'),
            _ => result.push(c),
        }
    }
    result
}

pub fn apply_zero_width(input: &str) -> String {
    let zero_width_chars = ['\u{200B}', '\u{200C}', '\u{200D}', '\u{FEFF}'];
    let mut result = String::new();
    let mut rng = rand::thread_rng();

    for (i, c) in input.chars().enumerate() {
        result.push(c);
        if i < input.len() - 1 && rand::Rng::gen_ratio(&mut rng, 1, 3) {
            let zw = zero_width_chars[rand::Rng::gen_range(&mut rng, 0..zero_width_chars.len())];
            result.push(zw);
        }
    }
    result
}

pub fn apply_comment_obfuscation(input: &str) -> String {
    let mut result = String::new();
    let mut in_keyword = false;
    let keyword_chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

    for c in input.chars() {
        if keyword_chars.contains(c) {
            if in_keyword {
                result.push_str("/**/");
            }
            in_keyword = true;
            result.push(c);
        } else {
            in_keyword = false;
            result.push(c);
        }
    }
    result
}

pub fn apply_whitespace_variation(input: &str) -> String {
    let mut result = String::new();
    let whitespace_variants = [
        ' ', '\t', '\n', '\r', '\u{00A0}', '\u{1680}', '\u{2000}', '\u{2001}',
    ];
    let mut rng = rand::thread_rng();

    for c in input.chars() {
        if c == ' ' {
            let idx = rand::Rng::gen_range(&mut rng, 0..whitespace_variants.len());
            result.push(whitespace_variants[idx]);
        } else {
            result.push(c);
        }
    }
    result
}

pub fn apply_unicode_encoding(input: &str) -> String {
    let mut result = String::new();
    for c in input.chars() {
        if c.is_ascii_alphanumeric() {
            result.push_str(&format!("\\u{:04x}", c as u32));
        } else {
            result.push(c);
        }
    }
    result
}

pub fn apply_double_encoding(input: &str) -> String {
    let first_encode = urlencoding::encode(input);
    urlencoding::encode(&first_encode).to_string()
}
