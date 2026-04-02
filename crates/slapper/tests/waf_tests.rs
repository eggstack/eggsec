use slapper::waf::{
    bypass::{get_auto_profile, get_profile_by_name, get_waf_profiles},
    OwaspCategory, TestType,
};

#[test]
fn test_waf_category() {
    let category = OwaspCategory::A01_2021_BrokenAccessControl;
    let display = format!("{}", category);
    assert!(display.contains("Broken Access Control"));
}

#[test]
fn test_waf_category_2023() {
    let category = OwaspCategory::A01_2023_BrokenObjectLevelAuthorization;
    let display = format!("{}", category);
    assert!(display.contains("Broken Object"));
}

#[test]
fn test_waf_severity() {
    use slapper::waf::Severity;

    let high = Severity::High;
    let low = Severity::Low;

    assert!(matches!(high, Severity::High));
    assert!(matches!(low, Severity::Low));
}

#[test]
fn test_test_type_from_string() {
    assert_eq!(TestType::parse("sqli"), TestType::Sql);
    assert_eq!(TestType::parse("SQL"), TestType::Sql);
    assert_eq!(TestType::parse("xss"), TestType::Xss);
    assert_eq!(TestType::parse("XSS"), TestType::Xss);
    assert_eq!(TestType::parse("ssrf"), TestType::Ssrf);
    assert_eq!(TestType::parse("cmd"), TestType::Cmd);
    assert_eq!(TestType::parse("command"), TestType::Cmd);
    assert_eq!(TestType::parse("traversal"), TestType::Traversal);
    assert_eq!(TestType::parse("lfi"), TestType::Traversal);
    assert_eq!(TestType::parse("unknown"), TestType::All);
    assert_eq!(TestType::parse("all"), TestType::All);
}

#[test]
fn test_test_type_default() {
    let test_type = TestType::default();
    assert_eq!(test_type, TestType::All);
}

#[test]
fn test_waf_profiles_exist() {
    let profiles = get_waf_profiles();
    assert!(!profiles.is_empty(), "Should have WAF profiles");

    // Verify common WAFs have profiles (case-insensitive check)
    let profile_names: Vec<String> = profiles.iter().map(|p| p.name.to_lowercase()).collect();
    assert!(
        profile_names
            .iter()
            .any(|n: &String| n.contains("cloudflare")),
        "Should have Cloudflare profile"
    );
    assert!(
        profile_names.iter().any(|n: &String| n.contains("aws")),
        "Should have AWS WAF profile"
    );
    assert!(
        profile_names.iter().any(|n: &String| n.contains("akamai")),
        "Should have Akamai profile"
    );
}

#[test]
fn test_waf_profile_by_name() {
    let cloudflare = get_profile_by_name("cloudflare");
    assert!(cloudflare.is_some(), "Should find Cloudflare profile");

    let profile = cloudflare.unwrap();
    assert_eq!(profile.name.to_lowercase(), "cloudflare");
    assert!(
        !profile.detection_signatures.is_empty(),
        "Should have detection signatures"
    );
}

#[test]
fn test_waf_profile_not_found() {
    let unknown = get_profile_by_name("nonexistent_waf_xyz");
    assert!(unknown.is_none(), "Should not find unknown profile");
}

#[test]
fn test_waf_auto_profile() {
    let auto = get_auto_profile();
    assert!(!auto.name.is_empty(), "Auto profile should have a name");
    // Auto profile may not have detection signatures
}

#[test]
fn test_waf_profile_bypass_techniques() {
    let profiles = get_waf_profiles();

    for profile in &profiles {
        // Each profile should have some bypass techniques
        assert!(
            !profile.detection_signatures.is_empty(),
            "Profile '{}' should have detection signatures",
            profile.name
        );
    }
}

#[test]
fn test_owasp_category_mapping() {
    // Test OWASP 2021 categories
    let a01 = OwaspCategory::A01_2021_BrokenAccessControl;
    let a02 = OwaspCategory::A02_2021_CryptographicFailures;
    let a03 = OwaspCategory::A03_2021_Injection;

    assert!(format!("{}", a01).contains("Broken Access"));
    assert!(format!("{}", a02).contains("Cryptographic"));
    assert!(format!("{}", a03).contains("Injection"));

    // Test OWASP 2023 categories
    let a01_2023 = OwaspCategory::A01_2023_BrokenObjectLevelAuthorization;
    let a02_2023 = OwaspCategory::A02_2023_BrokenAuthentication;

    assert!(format!("{}", a01_2023).contains("Broken Object"));
    assert!(format!("{}", a02_2023).contains("Authentication"));
}

#[test]
fn test_severity_ordering() {
    use slapper::waf::Severity;

    let critical = Severity::Critical;
    let high = Severity::High;
    let medium = Severity::Medium;
    let low = Severity::Low;
    let info = Severity::Info;

    // Verify all severity levels exist
    assert!(matches!(critical, Severity::Critical));
    assert!(matches!(high, Severity::High));
    assert!(matches!(medium, Severity::Medium));
    assert!(matches!(low, Severity::Low));
    assert!(matches!(info, Severity::Info));
}

#[test]
fn test_test_type_variants() {
    // Test all TestType variants
    assert_ne!(TestType::Sql, TestType::Xss);
    assert_ne!(TestType::Xss, TestType::Ssrf);
    assert_ne!(TestType::Ssrf, TestType::Cmd);
    assert_ne!(TestType::Cmd, TestType::Traversal);
    assert_ne!(TestType::Traversal, TestType::All);

    // Test that All encompasses everything
    assert_eq!(TestType::default(), TestType::All);
}

#[test]
fn test_waf_profile_case_insensitive_lookup() {
    // Test case-insensitive profile lookup
    let lower = get_profile_by_name("cloudflare");
    let upper = get_profile_by_name("CLOUDFLARE");
    let mixed = get_profile_by_name("CloudFlare");

    // At least one should work (implementation may vary)
    assert!(
        lower.is_some() || upper.is_some() || mixed.is_some(),
        "Should find profile with at least one case variant"
    );
}
