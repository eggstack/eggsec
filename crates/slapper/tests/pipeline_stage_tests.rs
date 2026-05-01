use slapper::cli::ScanProfile;
use slapper::pipeline::stage::{parse_stages, Stage};

#[test]
fn test_stage_display() {
    assert_eq!(format!("{}", Stage::PortScan), "Port Scan");
    assert_eq!(format!("{}", Stage::Fingerprint), "Fingerprint");
    assert_eq!(format!("{}", Stage::EndpointScan), "Endpoint Scan");
    assert_eq!(format!("{}", Stage::Fuzz), "Fuzzing");
    assert_eq!(format!("{}", Stage::LoadTest), "Load Test");
    assert_eq!(format!("{}", Stage::Waf), "WAF Test");
    assert_eq!(format!("{}", Stage::Recon), "Recon");
}

#[test]
fn test_stage_from_string_aliases() {
    assert_eq!(Stage::from_string("port"), Some(Stage::PortScan));
    assert_eq!(Stage::from_string("portscan"), Some(Stage::PortScan));
    assert_eq!(Stage::from_string("port-scan"), Some(Stage::PortScan));
    assert_eq!(Stage::from_string("fingerprint"), Some(Stage::Fingerprint));
    assert_eq!(Stage::from_string("fp"), Some(Stage::Fingerprint));
    assert_eq!(Stage::from_string("endpoint"), Some(Stage::EndpointScan));
    assert_eq!(Stage::from_string("endpoints"), Some(Stage::EndpointScan));
    assert_eq!(
        Stage::from_string("endpoint-scan"),
        Some(Stage::EndpointScan)
    );
    assert_eq!(Stage::from_string("fuzz"), Some(Stage::Fuzz));
    assert_eq!(Stage::from_string("fuzzer"), Some(Stage::Fuzz));
    assert_eq!(Stage::from_string("fuzzing"), Some(Stage::Fuzz));
    assert_eq!(Stage::from_string("load"), Some(Stage::LoadTest));
    assert_eq!(Stage::from_string("loadtest"), Some(Stage::LoadTest));
    assert_eq!(Stage::from_string("load-test"), Some(Stage::LoadTest));
    assert_eq!(Stage::from_string("waf"), Some(Stage::Waf));
    assert_eq!(Stage::from_string("recon"), Some(Stage::Recon));
}

#[test]
fn test_stage_from_string_case_insensitive() {
    assert_eq!(Stage::from_string("PORT"), Some(Stage::PortScan));
    assert_eq!(Stage::from_string("Fingerprint"), Some(Stage::Fingerprint));
    assert_eq!(Stage::from_string("FUZZ"), Some(Stage::Fuzz));
}

#[test]
fn test_stage_from_string_unknown() {
    assert_eq!(Stage::from_string("unknown"), None);
    assert_eq!(Stage::from_string(""), None);
}

#[test]
fn test_stage_from_string_special_aliases() {
    assert_eq!(Stage::from_string("graphql"), Some(Stage::Fuzz));
    assert_eq!(Stage::from_string("oauth"), Some(Stage::Fuzz));
    assert_eq!(Stage::from_string("jwt"), Some(Stage::Fuzz));
}

#[test]
fn test_parse_stages_mixed() {
    let stages = parse_stages("port, fingerprint, fuzz, waf");
    assert_eq!(stages.len(), 4);
    assert_eq!(stages[0], Stage::PortScan);
    assert_eq!(stages[1], Stage::Fingerprint);
    assert_eq!(stages[2], Stage::Fuzz);
    assert_eq!(stages[3], Stage::Waf);
}

#[test]
fn test_parse_stages_with_unknown() {
    let stages = parse_stages("port,invalid,fuzz");
    assert_eq!(stages.len(), 2);
}

#[test]
fn test_parse_stages_empty() {
    let stages = parse_stages("");
    assert!(stages.is_empty());
}

#[test]
fn test_parse_stages_single() {
    let stages = parse_stages("recon");
    assert_eq!(stages.len(), 1);
    assert_eq!(stages[0], Stage::Recon);
}

#[test]
fn test_all_profiles_have_stages() {
    let profiles = [
        ScanProfile::Quick,
        ScanProfile::Endpoint,
        ScanProfile::Web,
        ScanProfile::Waf,
        ScanProfile::Full,
        ScanProfile::Api,
        ScanProfile::Recon,
        ScanProfile::Stealth,
        ScanProfile::Deep,
        ScanProfile::Vuln,
        ScanProfile::Auth,
    ];

    for profile in &profiles {
        let stages = Stage::from_profile(*profile);
        assert!(
            !stages.is_empty(),
            "Profile {:?} should have at least one stage",
            profile
        );
        assert!(
            stages.contains(&Stage::PortScan),
            "Profile {:?} should include PortScan",
            profile
        );
        assert!(
            stages.contains(&Stage::Fingerprint),
            "Profile {:?} should include Fingerprint",
            profile
        );
    }
}

#[test]
fn test_profiles_have_expected_end_stages() {
    let quick = Stage::from_profile(ScanProfile::Quick);
    assert_eq!(quick.last(), Some(&Stage::Fingerprint));

    let full = Stage::from_profile(ScanProfile::Full);
    assert_eq!(full.last(), Some(&Stage::LoadTest));

    let waf = Stage::from_profile(ScanProfile::Waf);
    assert!(waf.contains(&Stage::Waf));

    let recon = Stage::from_profile(ScanProfile::Recon);
    assert!(recon.contains(&Stage::Recon));

    let auth = Stage::from_profile(ScanProfile::Auth);
    let last_auth_stage = auth.last().unwrap();
    assert_eq!(*last_auth_stage, Stage::Fuzz);
}

#[test]
fn test_stage_equality() {
    assert_eq!(Stage::PortScan, Stage::PortScan);
    assert_ne!(Stage::PortScan, Stage::Fingerprint);
}

#[test]
fn test_stage_clone_copy() {
    let stage = Stage::Fuzz;
    let copied = stage;
    assert_eq!(stage, copied);
}
