//! Tests for WAF detection and pattern matching.
//!
//! Tests the WAF detector's ability to identify WAF products based on
//! response headers, cookies, and body patterns.

mod common;

use common::*;

#[tokio::test]
async fn test_waf_detector_cloudflare_header() {
    let server = create_test_server().await;
    mock_cloudflare_response("/test").mount(&server).await;

    let detector = slapper::waf::detector::WafDetector::new().unwrap();
    let result = detector.detect(&format!("{}/test", server.uri())).await.unwrap();

    assert_eq!(result.status_code, 403, "Should get 403 status");
    // Cloudflare mock has cf-ray header and 403 status - detector should flag it
    assert!(result.waf_name.is_some(), "Should detect some WAF with cf-ray header");
    assert!(result.confidence > 0, "Should have positive confidence");
}

#[tokio::test]
async fn test_waf_detector_aws_waf() {
    let server = create_test_server().await;
    mock_aws_waf_response("/test").mount(&server).await;

    let detector = slapper::waf::detector::WafDetector::new().unwrap();
    let result = detector.detect(&format!("{}/test", server.uri())).await.unwrap();

    assert!(result.waf_name.is_some(), "Should detect some WAF");
    assert!(result.confidence > 0, "Confidence should be > 0");
}

#[tokio::test]
async fn test_waf_detector_no_waf() {
    let server = create_test_server().await;
    mock_ok("/test").mount(&server).await;

    let detector = slapper::waf::detector::WafDetector::new().unwrap();
    let result = detector.detect(&format!("{}/test", server.uri())).await.unwrap();

    assert!(result.waf_name.is_none(), "Should not detect WAF on normal response");
    assert_eq!(result.confidence, 0);
}

#[tokio::test]
async fn test_waf_detector_status_code_block() {
    let server = create_test_server().await;
    // 403 is a common WAF block status
    mock_status("/test", 403).mount(&server).await;

    let detector = slapper::waf::detector::WafDetector::new().unwrap();
    let result = detector.detect(&format!("{}/test", server.uri())).await.unwrap();

    // 403 alone doesn't confirm a WAF, but the detector should handle it
    assert_eq!(result.status_code, 403);
}

#[tokio::test]
async fn test_waf_detector_unreachable_url() {
    let detector = slapper::waf::detector::WafDetector::new().unwrap();
    // Use a non-routable IP with a short timeout to avoid slow CI runs
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        detector.detect("http://192.0.2.1:1/test"),
    ).await;

    // Should either return a result or timeout - both are acceptable
    match result {
        Ok(Ok(r)) => {
            assert!(r.waf_name.is_none());
            assert_eq!(r.status_code, 0);
        }
        Ok(Err(_)) => {} // Request error is fine
        Err(_) => {} // Timeout is fine for unreachable hosts
    }
}

// Unit tests for waf_patterns module
#[test]
fn test_waf_signatures_not_empty() {
    let signatures = slapper::waf::waf_patterns::get_waf_signatures();
    assert!(!signatures.is_empty(), "Should have WAF signatures");
    assert!(signatures.len() >= 10, "Should have at least 10 WAF signatures");
}

#[test]
fn test_cloudflare_signature_exists() {
    let signatures = slapper::waf::waf_patterns::get_waf_signatures();
    assert!(signatures.contains_key("cloudflare"), "Should have Cloudflare signature");
    
    let cf = &signatures["cloudflare"];
    assert!(!cf.headers.is_empty(), "Cloudflare should have header patterns");
    assert!(!cf.cookies.is_empty(), "Cloudflare should have cookie patterns");
    assert!(!cf.body_patterns.is_empty(), "Cloudflare should have body patterns");
}

#[test]
fn test_aws_waf_signature_exists() {
    let signatures = slapper::waf::waf_patterns::get_waf_signatures();
    assert!(signatures.contains_key("aws_waf"), "Should have AWS WAF signature");
}

#[test]
fn test_common_response_patterns() {
    let patterns = slapper::waf::waf_patterns::get_common_waf_response_patterns();
    assert!(!patterns.is_empty(), "Should have common response patterns");
}

// Test WAF bypass payload generation
#[test]
fn test_bypass_header_generation() {
    let user_agents = slapper::waf::bypass::headers::get_user_agents();
    assert!(!user_agents.is_empty(), "Should have user agents");
    assert!(user_agents.len() >= 5, "Should have multiple user agents");
}

#[test]
fn test_xff_ip_generation() {
    let ips = slapper::waf::bypass::headers::generate_xff_ips();
    assert!(!ips.is_empty(), "Should generate XFF IPs");
}

#[test]
fn test_evasion_case_rotation() {
    let result = slapper::waf::bypass::evasion::apply_case_rotation("SELECT * FROM users");
    assert!(!result.is_empty(), "Should produce case-rotated output");
    assert_ne!(result, "SELECT * FROM users", "Should modify the input");
}

#[test]
fn test_evasion_comment_obfuscation() {
    let result = slapper::waf::bypass::evasion::apply_comment_obfuscation("SELECT");
    assert!(!result.is_empty(), "Should produce obfuscated output");
}

#[test]
fn test_smuggling_payloads() {
    let cl_te = slapper::waf::bypass::smuggling::generate_cl_te_payloads();
    let te_cl = slapper::waf::bypass::smuggling::generate_te_cl_payloads();
    assert!(!cl_te.is_empty(), "Should generate CL-TE payloads");
    assert!(!te_cl.is_empty(), "Should generate TE-CL payloads");
}

#[test]
fn test_waf_profiles() {
    let profiles = slapper::waf::bypass::profiles::get_waf_profiles();
    assert!(!profiles.is_empty(), "Should have WAF profiles");
    
    let cloudflare = slapper::waf::bypass::profiles::get_profile_by_name("cloudflare");
    assert!(cloudflare.is_some(), "Should find Cloudflare profile");
}

#[test]
fn test_auto_profile() {
    let profile = slapper::waf::bypass::profiles::get_auto_profile();
    assert!(!profile.name.is_empty(), "Auto profile should have a name");
}
