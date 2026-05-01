use crate::waf::{BypassResult, WafDetectionResult};

pub fn format_detection(detection: &WafDetectionResult) -> String {
    let mut output = String::new();
    output.push_str("WAF Detection Results\n");

    if let Some(ref waf_name) = detection.waf_name {
        output.push_str(&format!(
            "waf: {} ({}% confidence)",
            waf_name, detection.confidence
        ));
        if !detection.matched_headers.is_empty() {
            output.push_str(&format!(
                "matched headers: {}",
                detection.matched_headers.join(", ")
            ));
        }
        if !detection.matched_cookies.is_empty() {
            output.push_str(&format!(
                "matched cookies: {}",
                detection.matched_cookies.join(", ")
            ));
        }
    } else {
        output.push_str("waf: none detected");
    }

    output
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
