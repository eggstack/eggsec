#[test]
fn test_mutator_basic() {
    let payload = "SELECT * FROM users";
    let mutations = slapper::fuzzer::generate_mutations(payload, 3);

    assert!(mutations.len() >= 1);
    assert!(mutations.contains(&payload.to_string()));
}

#[test]
fn test_pipeline_context_new() {
    let ctx = slapper::pipeline::PipelineContext::new("example.com");
    assert_eq!(ctx.target, "example.com");
    assert!(ctx.open_ports.is_empty());
}

#[test]
fn test_stage_from_profile() {
    use slapper::cli::ScanProfile;
    use slapper::pipeline::Stage;

    let quick = Stage::from_profile(ScanProfile::Quick);
    assert_eq!(quick.len(), 2);
    assert!(quick.contains(&Stage::PortScan));
    assert!(quick.contains(&Stage::Fingerprint));

    let deep = Stage::from_profile(ScanProfile::Deep);
    assert_eq!(deep.len(), 4);
}

#[test]
fn test_stage_from_string() {
    use slapper::pipeline::Stage;

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
    use slapper::pipeline::{Pipeline, Stage};

    let pipeline = Pipeline::new("example.com")
        .add_stage(Stage::PortScan)
        .add_stage(Stage::Fingerprint);

    assert!(pipeline.has_stages());
}

#[test]
fn test_parse_stages() {
    use slapper::pipeline::stage::parse_stages;
    use slapper::pipeline::Stage;

    let stages = parse_stages("port,fingerprint,endpoint");
    assert_eq!(stages.len(), 3);
    assert_eq!(stages[0], Stage::PortScan);
    assert_eq!(stages[1], Stage::Fingerprint);
    assert_eq!(stages[2], Stage::EndpointScan);
}

#[test]
fn test_pipeline_report_failure_helpers() {
    use slapper::pipeline::executor::StageResult;
    use slapper::pipeline::{PipelineReport, Stage};

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
    };

    assert!(report.has_failures());
    let failed = report.first_failed_stage().expect("missing failed stage");
    assert_eq!(failed.stage, Stage::Fuzz);
    assert_eq!(failed.error.as_deref(), Some("fuzz failed"));
}
