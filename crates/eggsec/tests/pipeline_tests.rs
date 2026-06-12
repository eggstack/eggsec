#[test]
fn test_mutator_basic() {
    let payload = "SELECT * FROM users";
    let mutations = eggsec::fuzzer::generate_mutations(payload, 3);

    assert!(mutations.len() >= 1);
    assert!(mutations.contains(&payload.to_string()));
}

#[test]
fn test_pipeline_context_new() {
    let ctx = eggsec::pipeline::PipelineContext::new("example.com");
    assert_eq!(ctx.target, "example.com");
    assert!(ctx.open_ports.is_empty());
}

#[test]
fn test_stage_from_profile() {
    use eggsec::cli::ScanProfile;
    use eggsec::pipeline::Stage;

    let quick = Stage::from_profile(ScanProfile::Quick);
    assert_eq!(quick.len(), 2);
    assert!(quick.contains(&Stage::PortScan));
    assert!(quick.contains(&Stage::Fingerprint));

    let deep = Stage::from_profile(ScanProfile::Deep);
    assert_eq!(deep.len(), 4);
    assert!(deep.contains(&Stage::PortScan));
    assert!(deep.contains(&Stage::Fingerprint));
    assert!(deep.contains(&Stage::EndpointScan));
    assert!(deep.contains(&Stage::Fuzz));

    let dbreg = Stage::from_profile(ScanProfile::DbRegression);
    assert!(!dbreg.is_empty());
}

#[test]
fn test_stage_from_string() {
    use eggsec::pipeline::Stage;

    assert!(matches!(Stage::from_string("port"), Some(Stage::PortScan)));
    assert!(matches!(
        Stage::from_string("fingerprint"),
        Some(Stage::Fingerprint)
    ));
    assert!(matches!(
        Stage::from_string("endpoint"),
        Some(Stage::EndpointScan)
    ));
    assert!(matches!(Stage::from_string("fuzz"), Some(Stage::Fuzz)));
    assert!(matches!(Stage::from_string("load"), Some(Stage::LoadTest)));
    assert!(Stage::from_string("invalid").is_none());
}

#[test]
fn test_pipeline_builder() {
    use eggsec::pipeline::{Pipeline, Stage};

    let pipeline = Pipeline::new("example.com")
        .add_stage(Stage::PortScan)
        .add_stage(Stage::Fingerprint);

    assert!(pipeline.has_stages());
}

#[test]
fn test_parse_stages() {
    use eggsec::pipeline::stage::parse_stages;
    use eggsec::pipeline::Stage;

    let stages = parse_stages("port,fingerprint,endpoint");
    assert_eq!(stages.len(), 3);
    assert_eq!(stages[0], Stage::PortScan);
    assert_eq!(stages[1], Stage::Fingerprint);
    assert_eq!(stages[2], Stage::EndpointScan);
}

#[test]
fn test_pipeline_report_failure_helpers() {
    use eggsec::pipeline::executor::StageResult;
    use eggsec::pipeline::{PipelineReport, Stage};

    let report = PipelineReport {
        target: "example.com".to_string(),
        total_duration_ms: 100,
        stage_results: vec![
            StageResult {
                stage: Stage::PortScan,
                duration_ms: 10,
                success: true,
                error: None,
            },
            StageResult {
                stage: Stage::Fuzz,
                duration_ms: 20,
                success: false,
                error: Some("fuzz failed".to_string()),
            },
        ],
        open_ports: vec![],
        services: vec![],
        endpoints: vec![],
        checkpoint_error: None,
        manifest: None,
        vuln_assessment: None,
        load_test_results: None,
    };

    assert!(report.has_failures());
    let failed = report.first_failed_stage().expect("missing failed stage");
    assert_eq!(failed.stage, Stage::Fuzz);
    assert_eq!(failed.error.as_deref(), Some("fuzz failed"));
}
