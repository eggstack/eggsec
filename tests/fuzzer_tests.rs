#[test]
fn test_fuzzer_mutations() {
    let payload = "test";
    let mutations = slapper::fuzzer::generate_mutations(payload, 1);
    assert!(!mutations.is_empty());
}

#[test]
fn test_fuzzer_redos_detector() {
    let detector = slapper::fuzzer::ReDosDetector::new();
    let result = detector.detect(r"(.+)+");
    assert!(result.is_vulnerable);
}

#[test]
fn test_fuzzer_redos_executor() {
    let executor = slapper::fuzzer::RegexExecutor::new();
    let result = executor.check_pattern(r"a+");
    assert!(!result.is_vulnerable);
    assert!(result.is_match);
}
