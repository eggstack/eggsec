use slapper::waf::{OwaspCategory, TestType};

#[test]
fn test_waf_category() {
    let category = OwaspCategory::A01_2021_BrokenAccessControl;
    let display = format!("{}", category);
    assert!(display.contains("Broken Access Control"));
}

#[test]
fn test_waf_category_2023() {
    use slapper::waf::OwaspCategory;

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
    assert_eq!(TestType::from_string("sqli"), TestType::Sql);
    assert_eq!(TestType::from_string("SQL"), TestType::Sql);
    assert_eq!(TestType::from_string("xss"), TestType::Xss);
    assert_eq!(TestType::from_string("XSS"), TestType::Xss);
    assert_eq!(TestType::from_string("ssrf"), TestType::Ssrf);
    assert_eq!(TestType::from_string("cmd"), TestType::Cmd);
    assert_eq!(TestType::from_string("command"), TestType::Cmd);
    assert_eq!(TestType::from_string("traversal"), TestType::Traversal);
    assert_eq!(TestType::from_string("lfi"), TestType::Traversal);
    assert_eq!(TestType::from_string("unknown"), TestType::All);
    assert_eq!(TestType::from_string("all"), TestType::All);
}

#[test]
fn test_test_type_default() {
    let test_type = TestType::default();
    assert_eq!(test_type, TestType::All);
}
