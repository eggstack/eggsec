use slapper::output::{JUnitBuilder, JUnitTestResult, SarifBuilder};

#[test]
fn test_sarif_output() {
    let report = SarifBuilder::new()
        .add_rule(
            "SQLI001",
            "SQL Injection",
            "error",
            "SQL injection vulnerability detected",
        )
        .add_result(
            "SQLI001",
            "error",
            "Potential SQL injection in parameter 'id'",
            "https://example.com/api/users?id=1",
        )
        .build();

    assert_eq!(report.version, "2.1.0");
    assert_eq!(report.runs.len(), 1);
    assert_eq!(report.runs[0].results.len(), 1);

    let json = report.to_json().unwrap();
    assert!(json.contains("SQLI001"));
    assert!(json.contains("SQL Injection"));
}

#[test]
fn test_sarif_multiple_findings() {
    let report = SarifBuilder::new()
        .add_rule("SQLI001", "SQL Injection", "error", "SQL injection")
        .add_rule(
            "XSS001",
            "Cross-Site Scripting",
            "warning",
            "XSS vulnerability",
        )
        .add_result("SQLI001", "error", "SQLi found", "https://example.com?id=1")
        .add_result(
            "XSS001",
            "warning",
            "XSS found",
            "https://example.com?search=test",
        )
        .build();

    assert_eq!(report.runs[0].results.len(), 2);
}

#[test]
fn test_junit_output() {
    let report = JUnitBuilder::new("Security Tests")
        .add_test_case(
            "SQL Injection",
            "test_sqli_param_id",
            "SQLInjection",
            0.5,
            JUnitTestResult::Failed {
                message: "SQL injection vulnerability".to_string(),
                failure_type: "SQLI".to_string(),
                text: Some("Payload: ' OR 1=1--".to_string()),
            },
        )
        .add_test_case(
            "XSS",
            "test_xss_param_search",
            "XSS",
            0.3,
            JUnitTestResult::Passed,
        )
        .build();

    assert_eq!(report.name, "Security Tests");
    assert_eq!(report.tests, 2);
    assert_eq!(report.failures, 1);

    let xml = report.to_xml().unwrap();
    assert!(xml.contains("<?xml"));
    assert!(xml.contains("<testsuites"));
    assert!(xml.contains("test_sqli_param_id"));
}

#[test]
fn test_junit_findings() {
    let report = JUnitBuilder::new("Security Scan")
        .add_finding(
            "Vulnerabilities",
            "sqli",
            "high",
            "https://example.com/api?id=1",
            "SQL injection in id parameter",
        )
        .add_finding(
            "Vulnerabilities",
            "xss",
            "medium",
            "https://example.com/search?q=test",
            "Reflected XSS in search parameter",
        )
        .build();

    assert_eq!(report.failures, 2);
    assert_eq!(report.test_suites.len(), 1);
}

#[test]
fn test_junit_xml_structure() {
    let report = JUnitBuilder::new("Test Suite")
        .add_test_case("Suite", "test1", "Class", 1.0, JUnitTestResult::Passed)
        .add_test_case(
            "Suite",
            "test2",
            "Class",
            2.0,
            JUnitTestResult::Error {
                message: "Connection failed".to_string(),
                error_type: "NetworkError".to_string(),
                text: None,
            },
        )
        .add_test_case(
            "Suite",
            "test3",
            "Class",
            0.5,
            JUnitTestResult::Skipped {
                message: "Test skipped".to_string(),
            },
        )
        .build();

    let xml = report.to_xml().unwrap();

    assert!(xml.contains("tests=\"3\""));
    assert!(xml.contains("errors=\"1\""));
    assert!(xml.contains("skipped=\"1\""));
    assert!(xml.contains("<error"));
    assert!(xml.contains("<skipped"));
}
