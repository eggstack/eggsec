use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafFingerprint {
    pub name: String,
    pub vendor: String,
    pub confidence: f64,
    pub detection_headers: Vec<WafHeader>,
    pub detection_cookies: Vec<String>,
    pub detection_status_codes: Vec<u16>,
    pub detection_body_patterns: Vec<String>,
    pub bypass_techniques: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafHeader {
    pub header: String,
    pub patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafDetectionResult {
    pub waf_name: Option<String>,
    pub vendor: Option<String>,
    pub confidence: f64,
    pub detected: bool,
    pub matched_rules: Vec<String>,
    pub bypass_suggestions: Vec<String>,
}

pub struct WafFingerprinter {
    fingerprints: Vec<WafFingerprint>,
}

impl Default for WafFingerprinter {
    fn default() -> Self {
        Self::new()
    }
}

impl WafFingerprinter {
    pub fn new() -> Self {
        Self {
            fingerprints: Self::default_fingerprints(),
        }
    }

    pub fn with_custom_fingerprints(mut self, fps: Vec<WafFingerprint>) -> Self {
        self.fingerprints.extend(fps);
        self
    }

    fn default_fingerprints() -> Vec<WafFingerprint> {
        vec![
            WafFingerprint {
                name: "Cloudflare".to_string(),
                vendor: "Cloudflare".to_string(),
                confidence: 0.9,
                detection_headers: vec![
                    WafHeader {
                        header: "server".to_string(),
                        patterns: vec!["cloudflare".to_lowercase()],
                    },
                    WafHeader {
                        header: "cf-ray".to_string(),
                        patterns: vec![],
                    },
                    WafHeader {
                        header: "cf-cache-status".to_string(),
                        patterns: vec![],
                    },
                    WafHeader {
                        header: "cf-edge-server".to_string(),
                        patterns: vec![],
                    },
                ],
                detection_cookies: vec!["__cf_bm".to_string(), "__cfduid".to_string()],
                detection_status_codes: vec![],
                detection_body_patterns: vec![
                    "Attention required".to_lowercase(),
                    "cloudflare".to_lowercase(),
                    "cf-error-details".to_lowercase(),
                ],
                bypass_techniques: vec![
                    "Use HTTP/2 request smuggling".to_string(),
                    "Cloudflare IP ranges bypass".to_string(),
                    "Original IP header X-Real-IP".to_string(),
                ],
            },
            WafFingerprinter::akamai(),
            WafFingerprinter::aws_waf(),
            WafFingerprinter::imperva(),
            WafFingerprinter::f5_asm(),
            WafFingerprinter::azure_waf(),
            WafFingerprinter::fortiweb(),
            WafFingerprinter::modsecurity(),
            WafFingerprinter::sucuri(),
            WafFingerprinter::incapsula(),
            WafFingerprinter::barracuda(),
            WafFingerprinter::denyall(),
            WafFingerprinter::radware(),
            WafFingerprinter::safe3(),
            WafFingerprinter::dotdefender(),
            WafFingerprinter::stackpath(),
            WafFingerprinter::fastly(),
            WafFingerprinter::cloudfront(),
        ]
    }

    fn akamai() -> WafFingerprint {
        WafFingerprint {
            name: "Akamai".to_string(),
            vendor: "Akamai".to_string(),
            confidence: 0.85,
            detection_headers: vec![
                WafHeader {
                    header: "server".to_string(),
                    patterns: vec!["akamai".to_lowercase()],
                },
                WafHeader {
                    header: "x-cdn".to_string(),
                    patterns: vec!["akamai".to_lowercase()],
                },
                WafHeader {
                    header: "akamai-origin-hop".to_string(),
                    patterns: vec![],
                },
            ],
            detection_cookies: vec!["ak_bmsc".to_string()],
            detection_status_codes: vec![],
            detection_body_patterns: vec![
                "Reference #".to_lowercase(),
                "Access Denied".to_lowercase(),
            ],
            bypass_techniques: vec![
                "Akamai JSON body parsing issues".to_string(),
                "Case sensitivity in headers".to_string(),
            ],
        }
    }

    fn aws_waf() -> WafFingerprint {
        WafFingerprint {
            name: "AWS WAF".to_string(),
            vendor: "Amazon".to_string(),
            confidence: 0.7,
            detection_headers: vec![
                WafHeader {
                    header: "server".to_string(),
                    patterns: vec!["awselb".to_lowercase()],
                },
                WafHeader {
                    header: "x-amzn-requestid".to_string(),
                    patterns: vec![],
                },
            ],
            detection_cookies: vec![],
            detection_status_codes: vec![],
            detection_body_patterns: vec!["AWS WAF".to_lowercase(), "403 Forbidden".to_lowercase()],
            bypass_techniques: vec![
                "AWS WAF token challenge bypass".to_string(),
                "IP rotation with different subnets".to_string(),
            ],
        }
    }

    fn imperva() -> WafFingerprint {
        WafFingerprint {
            name: "Imperva Incapsula".to_string(),
            vendor: "Imperva".to_string(),
            confidence: 0.9,
            detection_headers: vec![
                WafHeader {
                    header: "server".to_string(),
                    patterns: vec!["incapsula".to_lowercase(), "imperva".to_lowercase()],
                },
                WafHeader {
                    header: "x-cdn".to_string(),
                    patterns: vec!["incapsula".to_lowercase()],
                },
                WafHeader {
                    header: "x-iinfo".to_string(),
                    patterns: vec![],
                },
            ],
            detection_cookies: vec!["incap_ses".to_string(), "visid_incap".to_string()],
            detection_status_codes: vec![],
            detection_body_patterns: vec!["incapsula".to_lowercase(), "incident ID".to_lowercase()],
            bypass_techniques: vec![
                "HTTP/0.9 request".to_string(),
                "Invalid URL encoding".to_string(),
            ],
        }
    }

    fn f5_asm() -> WafFingerprint {
        WafFingerprint {
            name: "F5 ASM".to_string(),
            vendor: "F5 Networks".to_string(),
            confidence: 0.8,
            detection_headers: vec![
                WafHeader {
                    header: "server".to_string(),
                    patterns: vec!["bigip".to_lowercase(), "bigip".to_lowercase()],
                },
                WafHeader {
                    header: "x-cnection".to_string(),
                    patterns: vec![],
                },
            ],
            detection_cookies: vec!["TS".to_lowercase()],
            detection_status_codes: vec![],
            detection_body_patterns: vec!["The requested URL was rejected".to_lowercase()],
            bypass_techniques: vec!["Chunked encoding manipulation".to_string()],
        }
    }

    fn azure_waf() -> WafFingerprint {
        WafFingerprint {
            name: "Azure WAF".to_string(),
            vendor: "Microsoft".to_string(),
            confidence: 0.75,
            detection_headers: vec![
                WafHeader {
                    header: "server".to_string(),
                    patterns: vec!["azure".to_lowercase()],
                },
                WafHeader {
                    header: "x-azure-ref".to_string(),
                    patterns: vec![],
                },
            ],
            detection_cookies: vec![],
            detection_status_codes: vec![],
            detection_body_patterns: vec!["Azure WAF".to_lowercase(), "waf".to_lowercase()],
            bypass_techniques: vec!["Double URL encoding".to_string()],
        }
    }

    fn fortiweb() -> WafFingerprint {
        WafFingerprint {
            name: "FortiWeb".to_string(),
            vendor: "Fortinet".to_string(),
            confidence: 0.8,
            detection_headers: vec![WafHeader {
                header: "server".to_string(),
                patterns: vec!["fortiweb".to_lowercase()],
            }],
            detection_cookies: vec!["FORTIWAFSID".to_string()],
            detection_status_codes: vec![],
            detection_body_patterns: vec!["fortiweb".to_lowercase()],
            bypass_techniques: vec!["HTTP parameter pollution".to_string()],
        }
    }

    fn modsecurity() -> WafFingerprint {
        WafFingerprint {
            name: "ModSecurity".to_string(),
            vendor: "Trustwave".to_string(),
            confidence: 0.7,
            detection_headers: vec![
                WafHeader {
                    header: "server".to_string(),
                    patterns: vec!["mod_security".to_lowercase(), "modsecurity".to_lowercase()],
                },
                WafHeader {
                    header: "x-mod-security".to_string(),
                    patterns: vec![],
                },
            ],
            detection_cookies: vec![],
            detection_status_codes: vec![406, 501],
            detection_body_patterns: vec![
                "mod_security".to_lowercase(),
                "modsecurity".to_lowercase(),
            ],
            bypass_techniques: vec![
                "UTF-7 encoding".to_string(),
                "Null byte injection".to_string(),
            ],
        }
    }

    fn sucuri() -> WafFingerprint {
        WafFingerprint {
            name: "Sucuri".to_string(),
            vendor: "Sucuri".to_string(),
            confidence: 0.85,
            detection_headers: vec![
                WafHeader {
                    header: "server".to_string(),
                    patterns: vec!["sucuri".to_lowercase()],
                },
                WafHeader {
                    header: "x-sucuri".to_string(),
                    patterns: vec![],
                },
                WafHeader {
                    header: "x-sucuri-id".to_string(),
                    patterns: vec![],
                },
            ],
            detection_cookies: vec!["sucuri".to_lowercase()],
            detection_status_codes: vec![],
            detection_body_patterns: vec!["sucuri".to_lowercase(), "cloudproxy".to_lowercase()],
            bypass_techniques: vec!["Host header manipulation".to_string()],
        }
    }

    fn incapsula() -> WafFingerprint {
        WafFingerprinter::imperva()
    }

    fn barracuda() -> WafFingerprint {
        WafFingerprint {
            name: "Barracuda WAF".to_string(),
            vendor: "Barracuda".to_string(),
            confidence: 0.8,
            detection_headers: vec![
                WafHeader {
                    header: "server".to_string(),
                    patterns: vec!["barracuda".to_lowercase()],
                },
                WafHeader {
                    header: "barra".to_string(),
                    patterns: vec![],
                },
            ],
            detection_cookies: vec!["barra".to_string()],
            detection_status_codes: vec![],
            detection_body_patterns: vec!["barracuda".to_lowercase()],
            bypass_techniques: vec!["HTTP verb tampering".to_string()],
        }
    }

    fn denyall() -> WafFingerprint {
        WafFingerprint {
            name: "Deny All".to_string(),
            vendor: "DenyAll".to_string(),
            confidence: 0.75,
            detection_headers: vec![WafHeader {
                header: "server".to_string(),
                patterns: vec!["denyall".to_lowercase()],
            }],
            detection_cookies: vec!["session".to_string()],
            detection_status_codes: vec![],
            detection_body_patterns: vec!["denyall".to_lowercase()],
            bypass_techniques: vec!["Content-Type manipulation".to_string()],
        }
    }

    fn radware() -> WafFingerprint {
        WafFingerprint {
            name: "Radware".to_string(),
            vendor: "Radware".to_string(),
            confidence: 0.7,
            detection_headers: vec![WafHeader {
                header: "server".to_string(),
                patterns: vec!["radware".to_lowercase()],
            }],
            detection_cookies: vec![],
            detection_status_codes: vec![],
            detection_body_patterns: vec!["radware".to_lowercase()],
            bypass_techniques: vec!["Header case variation".to_string()],
        }
    }

    fn safe3() -> WafFingerprint {
        WafFingerprint {
            name: "Safe3".to_string(),
            vendor: "Safe3".to_string(),
            confidence: 0.65,
            detection_headers: vec![WafHeader {
                header: "server".to_string(),
                patterns: vec!["safe3".to_lowercase()],
            }],
            detection_cookies: vec![],
            detection_status_codes: vec![],
            detection_body_patterns: vec!["safe3".to_lowercase()],
            bypass_techniques: vec!["Encoding variations".to_string()],
        }
    }

    fn dotdefender() -> WafFingerprint {
        WafFingerprint {
            name: "dotDefender".to_string(),
            vendor: "Applicure".to_string(),
            confidence: 0.7,
            detection_headers: vec![WafHeader {
                header: "x-dotdefender".to_string(),
                patterns: vec![],
            }],
            detection_cookies: vec![],
            detection_status_codes: vec![],
            detection_body_patterns: vec!["dotdefender".to_lowercase()],
            bypass_techniques: vec!["Unicode encoding".to_string()],
        }
    }

    fn stackpath() -> WafFingerprint {
        WafFingerprint {
            name: "StackPath".to_string(),
            vendor: "StackPath".to_string(),
            confidence: 0.8,
            detection_headers: vec![WafHeader {
                header: "server".to_string(),
                patterns: vec!["stackpath".to_lowercase()],
            }],
            detection_cookies: vec!["stackpath".to_lowercase()],
            detection_status_codes: vec![],
            detection_body_patterns: vec!["stackpath".to_lowercase()],
            bypass_techniques: vec!["Cache poisoning".to_string()],
        }
    }

    fn fastly() -> WafFingerprint {
        WafFingerprint {
            name: "Fastly".to_string(),
            vendor: "Fastly".to_string(),
            confidence: 0.75,
            detection_headers: vec![
                WafHeader {
                    header: "server".to_string(),
                    patterns: vec!["fastly".to_lowercase()],
                },
                WafHeader {
                    header: "x-served-by".to_string(),
                    patterns: vec!["fastly".to_lowercase()],
                },
            ],
            detection_cookies: vec![],
            detection_status_codes: vec![],
            detection_body_patterns: vec!["fastly".to_lowercase()],
            bypass_techniques: vec!["Fastly request pooling bypass".to_string()],
        }
    }

    fn cloudfront() -> WafFingerprint {
        WafFingerprint {
            name: "CloudFront".to_string(),
            vendor: "Amazon".to_string(),
            confidence: 0.65,
            detection_headers: vec![
                WafHeader {
                    header: "server".to_string(),
                    patterns: vec!["cloudfront".to_lowercase()],
                },
                WafHeader {
                    header: "x-amz-cf-id".to_string(),
                    patterns: vec![],
                },
            ],
            detection_cookies: vec![],
            detection_status_codes: vec![],
            detection_body_patterns: vec!["cloudfront".to_lowercase()],
            bypass_techniques: vec!["X-Origin header injection".to_string()],
        }
    }

    pub fn detect(&self, headers: &HeaderMap, status_code: u16, body: &str) -> WafDetectionResult {
        let mut matches: Vec<(WafFingerprint, f64, Vec<String>)> = Vec::new();

        for fp in &self.fingerprints {
            let mut confidence: f64 = 0.0;
            let mut matched_rules = Vec::new();

            for waf_header in &fp.detection_headers {
                if let Some(header_value) = headers.get(&waf_header.header) {
                    if let Ok(value_str) = header_value.to_str() {
                        let value_lower = value_str.to_lowercase();
                        for pattern in &waf_header.patterns {
                            if value_lower.contains(&pattern.to_lowercase()) {
                                confidence += 0.3;
                                matched_rules
                                    .push(format!("header:{}:{}", waf_header.header, pattern));
                            }
                        }
                        if waf_header.patterns.is_empty() {
                            confidence += 0.2;
                            matched_rules.push(format!("header:{}", waf_header.header));
                        }
                    }
                }
            }

            for cookie_name in &fp.detection_cookies {
                if let Some(cookie_header) = headers.get("set-cookie") {
                    if let Ok(cookie_str) = cookie_header.to_str() {
                        if cookie_str
                            .to_lowercase()
                            .contains(&cookie_name.to_lowercase())
                        {
                            confidence += 0.25;
                            matched_rules.push(format!("cookie:{}", cookie_name));
                        }
                    }
                }
            }

            if fp.detection_status_codes.contains(&status_code) {
                confidence += 0.2;
                matched_rules.push(format!("status:{}", status_code));
            }

            let body_lower = body.to_lowercase();
            for pattern in &fp.detection_body_patterns {
                if body_lower.contains(&pattern.to_lowercase()) {
                    confidence += 0.15;
                    matched_rules.push(format!("body:{}", pattern));
                }
            }

            confidence = confidence.min(fp.confidence);

            if confidence > 0.2 {
                matches.push((fp.clone(), confidence, matched_rules));
            }
        }

        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((best, conf, rules)) = matches.first() {
            WafDetectionResult {
                waf_name: Some(best.name.clone()),
                vendor: Some(best.vendor.clone()),
                confidence: *conf,
                detected: true,
                matched_rules: rules.clone(),
                bypass_suggestions: best.bypass_techniques.clone(),
            }
        } else {
            WafDetectionResult {
                waf_name: None,
                vendor: None,
                confidence: 0.0,
                detected: false,
                matched_rules: vec![],
                bypass_suggestions: vec![],
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    #[test]
    fn test_waf_fingerprinter() {
        let fingerprinter = WafFingerprinter::new();

        let mut headers = HeaderMap::new();
        headers.insert("server", HeaderValue::from_static("cloudflare"));

        let result = fingerprinter.detect(&headers, 200, "");

        assert!(result.detected);
    }
}
