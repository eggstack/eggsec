use crate::waf::{BypassResult, WafDetectionResult};

pub fn format_detection(detection: &WafDetectionResult) -> String {
    let mut output = String::new();
    output.push_str("WAF Detection Results\n");

    if let Some(ref err) = detection.request_error {
        output.push_str(&format!("request status: failed ({})\n", err));
    }

    if let Some(ref waf_name) = detection.waf_name {
        output.push_str(&format!(
            "waf: {} ({}% confidence)\n",
            waf_name, detection.confidence
        ));
        if !detection.matched_headers.is_empty() {
            output.push_str(&format!(
                "matched headers: {}\n",
                detection.matched_headers.join(", ")
            ));
        }
        if !detection.matched_cookies.is_empty() {
            output.push_str(&format!(
                "matched cookies: {}\n",
                detection.matched_cookies.join(", ")
            ));
        }
        if !detection.matched_patterns.is_empty() {
            output.push_str(&format!(
                "matched patterns: {}\n",
                detection.matched_patterns.join(", ")
            ));
        }
    } else {
        output.push_str("waf: none detected\n");
        if !detection.matched_patterns.is_empty() {
            output.push_str(&format!(
                "matched patterns: {}\n",
                detection.matched_patterns.join(", ")
            ));
        }
    }

    output.trim_end().to_string()
}

pub fn print_detection(detection: &WafDetectionResult) {
    println!("{}", format_detection(detection));
}

pub fn print_detection_json(detection: &WafDetectionResult) {
    match serde_json::to_string_pretty(detection) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Failed to serialize detection result: {}", e),
    }
}

pub fn format_results(
    detection: &WafDetectionResult,
    bypass_results: &[BypassResult],
    _selected_profile: Option<&String>,
) -> String {
    let mut output = format_detection(detection);
    output.push_str("\n\n");

    let successful: Vec<_> = bypass_results.iter().filter(|r| r.success).collect();
    let failed: Vec<_> = bypass_results.iter().filter(|r| !r.success).collect();

    for result in &successful {
        output.push_str(&format!(
            "[+] {:?}: {}\n",
            result.technique, result.description
        ));
    }

    for result in &failed {
        output.push_str(&format!(
            "[-] {:?}: {}\n",
            result.technique, result.description
        ));
    }

    output.push_str(&format!(
        "\nbypasses: {} / {} successful",
        successful.len(),
        bypass_results.len()
    ));

    output
}

pub fn print_results(
    detection: &WafDetectionResult,
    bypass_results: &[BypassResult],
    selected_profile: Option<&String>,
) {
    println!(
        "{}",
        format_results(detection, bypass_results, selected_profile)
    );
}

pub fn print_results_json(detection: &WafDetectionResult, bypass_results: &[BypassResult]) {
    let output = serde_json::json!({
        "detection": detection,
        "bypass_results": bypass_results,
    });
    match serde_json::to_string_pretty(&output) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Failed to serialize results: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_detection() -> WafDetectionResult {
        WafDetectionResult {
            waf_name: Some("Cloudflare".to_string()),
            confidence: 90,
            request_error: None,
            matched_headers: vec!["cf-ray: abc".to_string()],
            matched_cookies: vec!["__cfduid".to_string()],
            matched_patterns: vec!["access denied".to_string()],
            server_header: Some("cloudflare".to_string()),
            status_code: 403,
        }
    }

    #[test]
    fn format_detection_separates_lines() {
        let out = format_detection(&base_detection());
        assert!(out.contains("\nwaf: Cloudflare (90% confidence)\n"));
        assert!(out.contains("matched headers: cf-ray: abc\n"));
        assert!(out.contains("matched cookies: __cfduid\n"));
    }

    #[test]
    fn format_detection_includes_request_error() {
        let mut detection = base_detection();
        detection.waf_name = None;
        detection.request_error = Some("dns failed".to_string());
        let out = format_detection(&detection);
        assert!(out.contains("request status: failed (dns failed)"));
    }
}
