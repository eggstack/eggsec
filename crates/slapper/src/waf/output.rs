use crate::waf::{BypassResult, WafDetectionResult};

pub fn print_detection(detection: &WafDetectionResult) {
    println!("WAF Detection Results");

    if let Some(ref waf_name) = detection.waf_name {
        println!("waf: {} ({}% confidence)", waf_name, detection.confidence);
        if !detection.matched_headers.is_empty() {
            println!("matched headers: {}", detection.matched_headers.join(", "));
        }
        if !detection.matched_cookies.is_empty() {
            println!("matched cookies: {}", detection.matched_cookies.join(", "));
        }
    } else {
        println!("waf: none detected");
    }
}

pub fn print_detection_json(detection: &WafDetectionResult) {
    match serde_json::to_string_pretty(detection) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Failed to serialize detection result: {}", e),
    }
}

pub fn print_results(
    detection: &WafDetectionResult,
    bypass_results: &[BypassResult],
    _selected_profile: Option<&String>,
) {
    print_detection(detection);

    println!();

    let successful: Vec<_> = bypass_results.iter().filter(|r| r.success).collect();
    let failed: Vec<_> = bypass_results.iter().filter(|r| !r.success).collect();

    for result in &successful {
        println!("[+] {:?}: {}", result.technique, result.description);
    }

    for result in &failed {
        println!("[-] {:?}: {}", result.technique, result.description);
    }

    println!(
        "\nbypasses: {} / {} successful",
        successful.len(),
        bypass_results.len()
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
