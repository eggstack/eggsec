use slapper::scanner::templates::verify::{SignedTemplate, SignerInfo, TemplateSigner, TemplateVerifier};
use slapper::scanner::templates::models::VulnerabilityTemplate;

fn create_test_template() -> VulnerabilityTemplate {
    VulnerabilityTemplate {
        id: "test-template".to_string(),
        info: crate::scanner::templates::models::TemplateInfo {
            name: "Test Template".to_string(),
            author: "test".to_string(),
            severity: "high".to_string(),
            description: "A test template".to_string(),
            tags: vec!["test".to_string()],
            references: vec![],
            remediation: "Fix the issue".to_string(),
        },
        matchers: vec![],
        requests: vec![],
    }
}

fn create_test_signer_info() -> SignerInfo {
    SignerInfo {
        name: "Test Signer".to_string(),
        email: Some("test@example.com".to_string()),
        organization: Some("Test Org".to_string()),
        timestamp: chrono::Utc::now(),
    }
}

#[test]
fn test_signer_creation() {
    let signer_info = create_test_signer_info();
    let signer = TemplateSigner::new(signer_info).unwrap();
    assert!(!signer.public_key_string().is_empty());
}

#[test]
fn test_sign_and_verify() {
    let signer_info = create_test_signer_info();
    let signer = TemplateSigner::new(signer_info).unwrap();
    let template = create_test_template();

    let signed = signer.sign(&template).unwrap();
    assert!(!signed.signature.is_empty());
    assert!(!signed.public_key.is_empty());

    let verifier = TemplateVerifier::with_public_key(&signed.public_key).unwrap();
    let result = verifier.verify(&signed).unwrap();
    assert!(result.valid);
    assert_eq!(result.template_id, "test-template");
}

#[test]
fn test_signature_verification_fails_on_tampered_template() {
    let signer_info = create_test_signer_info();
    let signer = TemplateSigner::new(signer_info).unwrap();
    let template = create_test_template();

    let mut signed = signer.sign(&template).unwrap();
    signed.template.info.severity = "low".to_string();

    let verifier = TemplateVerifier::with_public_key(&signed.public_key).unwrap();
    let result = verifier.verify(&signed).unwrap();
    assert!(!result.valid);
    assert!(result.error.is_some());
}

#[test]
fn test_verify_without_public_key() {
    let signer_info = create_test_signer_info();
    let signer = TemplateSigner::new(signer_info).unwrap();
    let template = create_test_template();

    let signed = signer.sign(&template).unwrap();

    let verifier = TemplateVerifier::new();
    let result = verifier.verify(&signed);
    assert!(result.is_err());
}

#[test]
fn test_sign_and_verify_raw() {
    let signer_info = create_test_signer_info();
    let signer = TemplateSigner::new(signer_info).unwrap();
    let template = create_test_template();

    let signed = signer.sign(&template).unwrap();

    let verifier = TemplateVerifier::with_public_key(&signed.public_key).unwrap();
    let is_valid = verifier.verify_raw(&signed.template, &signed.signature).unwrap();
    assert!(is_valid);
}

#[test]
fn test_public_key_string_format() {
    let signer_info = create_test_signer_info();
    let signer = TemplateSigner::new(signer_info).unwrap();
    let public_key_str = signer.public_key_string();

    let verifier = TemplateVerifier::with_public_key(&public_key_str).unwrap();
    assert!(verifier.verify_raw(&create_test_template(), "").is_err());
}

#[test]
fn test_signed_template_serialization() {
    let signer_info = create_test_signer_info();
    let signer = TemplateSigner::new(signer_info).unwrap();
    let template = create_test_template();
    let signed = signer.sign(&template).unwrap();

    let json = serde_json::to_string(&signed).unwrap();
    let deserialized: SignedTemplate = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.template.id, signed.template.id);
    assert_eq!(deserialized.signature, signed.signature);
}

#[test]
fn test_invalid_public_key() {
    let result = TemplateVerifier::with_public_key("invalid-base64!!!");
    assert!(result.is_err());
}

#[test]
fn test_wrong_length_public_key() {
    let result = TemplateVerifier::with_public_key("dGhpcyBpcyBub3QgcHJvcGVyIGJhc2U2NA=="); // 24 bytes, not 32
    assert!(result.is_err());
}

#[test]
fn test_invalid_signature_base64() {
    let signer_info = create_test_signer_info();
    let signer = TemplateSigner::new(signer_info).unwrap();
    let template = create_test_template();

    let signed = signer.sign(&template).unwrap();
    let verifier = TemplateVerifier::with_public_key(&signed.public_key).unwrap();
    
    let mut tampered = signed.clone();
    tampered.signature = "invalid-base64!!!".to_string();
    let result = verifier.verify(&tampered);
    assert!(result.is_err());
}

#[test]
fn test_signer_from_keypair() {
    let signer_info = create_test_signer_info();
    let signer = TemplateSigner::new(signer_info.clone()).unwrap();
    let keypair_bytes = signer.public_key_bytes();
    
    let signer2 = TemplateSigner::from_keypair(&keypair_bytes, signer_info).unwrap();
    assert!(!signer2.public_key_string().is_empty());
}

#[test]
fn test_invalid_keypair_length() {
    let signer_info = create_test_signer_info();
    let result = TemplateSigner::from_keypair(b"short", signer_info);
    assert!(result.is_err());
}